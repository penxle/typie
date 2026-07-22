use std::collections::VecDeque;

use editor_crdt::Dot;
use hashbrown::HashSet;

use super::project::{RawChild, RawNode, RawTree, synthetic_id};
use crate::nodes::NodeType;
use crate::schema::{ContentExpr, context_allows, wrap_chain};

/// Counters for the deterministic repair pass. `drops` and `totality_violations`
/// record real (authored) content loss — `drops` should always be zero under the
/// total-projection algebra (WRAP/SPLIT-HOIST move, never discard), and
/// `totality_violations` is the measured set-difference deficit an external
/// oracle records. `repairs` counts every WRAP / SPLIT-HOIST / revival applied.
/// `projection_degraded` is set only when the repair-pass cap is reached, in
/// which case the projection's schema validation is downgraded to telemetry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RepairStats {
    pub drops: u64,
    pub repairs: u64,
    pub totality_violations: u64,
    pub projection_degraded: bool,
}

impl RepairStats {
    /// Fold `other`'s counts into `self` (counts add, degradation is sticky).
    pub fn accumulate(&mut self, other: &RepairStats) {
        self.drops += other.drops;
        self.repairs += other.repairs;
        self.totality_violations += other.totality_violations;
        self.projection_degraded |= other.projection_degraded;
    }
}

/// Upper bound on the number of WRAP / SPLIT-HOIST operations a single normalize
/// invocation may apply before falling back to the current (partially repaired)
/// tree. The frontier-lexicographic convergence argument bounds the true count
/// far below this, so reaching it signals an unproven cycle — the fail-safe keeps
/// totality and order intact and downgrades schema validation to telemetry.
const DEFAULT_REPAIR_BUDGET: usize = 1 << 20;

#[cfg(any(test, feature = "test-utils"))]
thread_local! {
    static REPAIR_BUDGET_OVERRIDE: std::cell::Cell<Option<usize>> =
        const { std::cell::Cell::new(None) };
}

/// Force the full-tree repair pass to run with `budget` for as long as the returned
/// guard is alive, so a test can exercise the degraded (cap-exhausted) fallback that
/// the production [`DEFAULT_REPAIR_BUDGET`] never reaches on realistic input. The
/// override is per-thread (parallel tests never see each other's cap) and is restored
/// when the guard drops.
#[cfg(any(test, feature = "test-utils"))]
pub fn override_repair_budget(budget: usize) -> RepairBudgetGuard {
    let prev = REPAIR_BUDGET_OVERRIDE.with(|c| c.replace(Some(budget)));
    RepairBudgetGuard { prev }
}

#[cfg(any(test, feature = "test-utils"))]
#[must_use]
pub struct RepairBudgetGuard {
    prev: Option<usize>,
}

#[cfg(any(test, feature = "test-utils"))]
impl Drop for RepairBudgetGuard {
    fn drop(&mut self) {
        REPAIR_BUDGET_OVERRIDE.with(|c| c.set(self.prev));
    }
}

fn effective_repair_budget() -> usize {
    #[cfg(any(test, feature = "test-utils"))]
    {
        REPAIR_BUDGET_OVERRIDE
            .with(|c| c.get())
            .unwrap_or(DEFAULT_REPAIR_BUDGET)
    }
    #[cfg(not(any(test, feature = "test-utils")))]
    {
        DEFAULT_REPAIR_BUDGET
    }
}

/// Mutable state threaded through one normalize invocation.
struct RepairCtx<'a> {
    stats: &'a mut RepairStats,
    /// `(scaffold id, level depth)` of every WRAP-created scaffold, so a scaffold
    /// that becomes a residue again at the SAME level is HOISTed rather than
    /// re-wrapped (breaking the wrap cycle); a HOIST to a different level re-keys
    /// on the new depth and may legitimately re-wrap.
    wrap_created: HashSet<(Dot, usize)>,
    budget: usize,
    capped: bool,
}

impl<'a> RepairCtx<'a> {
    fn new(stats: &'a mut RepairStats, budget: usize) -> Self {
        Self {
            stats,
            wrap_created: HashSet::new(),
            budget,
            capped: false,
        }
    }

    /// Charge one repair operation. Returns `false` (and latches `capped`) once
    /// the budget is exhausted, so callers no-op and leave the tree as-is.
    fn spend(&mut self) -> bool {
        if self.capped {
            return false;
        }
        if self.budget == 0 {
            self.capped = true;
            return false;
        }
        self.budget -= 1;
        self.stats.repairs += 1;
        true
    }
}

fn fixed_slot_roles(content: &ContentExpr) -> Option<Vec<NodeType>> {
    match content {
        ContentExpr::Seq(es) if es.iter().all(|e| matches!(e, ContentExpr::Single(_))) => Some(
            es.iter()
                .map(|e| match e {
                    ContentExpr::Single(t) => *t,
                    _ => unreachable!(),
                })
                .collect(),
        ),
        _ => None,
    }
}

/// Whether `t`'s children occupy fixed, role-keyed slots (e.g. `Fold`'s
/// `FoldTitle`/`FoldContent`) — a schema `Seq` of `Single`s. Order-preserving
/// repair keeps such nodes in sequence order, but sequence-position math still
/// uses this to fall back to an exhaustive subtree scan for a damaged (reversed)
/// container that has not yet been re-projected.
pub fn is_fixed_slot(t: NodeType) -> bool {
    fixed_slot_roles(&t.spec().content).is_some()
}

/// Move any leading `Unknown` children (opaque preserved ops) verbatim from
/// `kids` to `out` without consuming a content slot — they belong with the
/// preceding real sibling and are invisible to content matching.
fn skip_unknowns(kids: &mut VecDeque<RawChild>, out: &mut Vec<RawChild>) {
    while kids
        .front()
        .and_then(|c| c.as_child_type())
        .is_some_and(|t| t == NodeType::Unknown)
    {
        out.push(kids.pop_front().unwrap());
    }
}

/// Greedily consume `kids` against `expr`, appending matched real children and
/// deterministic fillers for missing required slots to `out`, leaving any residue
/// in `kids`. `Unknown` children pass through transparently. Returns the number
/// of real (non-`Unknown`) children consumed. This is the container-completion
/// engine: it never removes or reorders existing real children.
fn match_content(
    expr: &ContentExpr,
    kids: &mut VecDeque<RawChild>,
    parent: Dot,
    out: &mut Vec<RawChild>,
) -> usize {
    let front = |k: &VecDeque<RawChild>, e: &ContentExpr| {
        k.front()
            .and_then(|c| c.as_child_type())
            .is_some_and(|t| t != NodeType::Unknown && e.matches(t))
    };
    match expr {
        ContentExpr::Empty | ContentExpr::Any => 0,
        ContentExpr::Single(t) => {
            skip_unknowns(kids, out);
            if kids
                .front()
                .and_then(|c| c.as_child_type())
                .is_some_and(|ct| ct == *t)
            {
                out.push(kids.pop_front().unwrap());
                1
            } else {
                out.push(RawChild::Block(scaffold_block(*t, 0, parent)));
                0
            }
        }
        ContentExpr::Optional(inner) => {
            skip_unknowns(kids, out);
            if front(kids, inner) {
                out.push(kids.pop_front().unwrap());
                1
            } else {
                0
            }
        }
        ContentExpr::ZeroOrMore(inner) => {
            let mut n = 0;
            loop {
                skip_unknowns(kids, out);
                if front(kids, inner) {
                    out.push(kids.pop_front().unwrap());
                    n += 1;
                } else {
                    break;
                }
            }
            n
        }
        ContentExpr::OneOrMore(inner) => {
            let mut n = 0;
            loop {
                skip_unknowns(kids, out);
                if front(kids, inner) {
                    out.push(kids.pop_front().unwrap());
                    n += 1;
                } else {
                    break;
                }
            }
            if n == 0 {
                out.push(RawChild::Block(scaffold_block(
                    first_type(inner),
                    0,
                    parent,
                )));
            }
            n
        }
        ContentExpr::Choice(cs) => {
            skip_unknowns(kids, out);
            match cs.iter().find(|c| front(kids, c)) {
                Some(c) => match_content(c, kids, parent, out),
                None => {
                    out.push(RawChild::Block(scaffold_block(
                        first_type(&cs[0]),
                        0,
                        parent,
                    )));
                    0
                }
            }
        }
        ContentExpr::Seq(exprs) => {
            let mut n = 0;
            for e in exprs {
                n += match_content(e, kids, parent, out);
            }
            n
        }
    }
}

/// Greedily consume `types` (real child types only) against `expr` and return how
/// many were consumed — the mirror of [`match_content`] without scaffolding. A
/// residue exists exactly when the returned count is short of `types.len()`.
fn greedy_consume(expr: &ContentExpr, types: &[NodeType]) -> usize {
    fn matches_at(e: &ContentExpr, types: &[NodeType], idx: usize) -> bool {
        types.get(idx).is_some_and(|t| e.matches(*t))
    }
    fn walk(expr: &ContentExpr, types: &[NodeType], idx: &mut usize) {
        match expr {
            ContentExpr::Empty => {}
            ContentExpr::Any => *idx = types.len(),
            ContentExpr::Single(t) => {
                if types.get(*idx) == Some(t) {
                    *idx += 1;
                }
            }
            ContentExpr::Optional(inner) => {
                if matches_at(inner, types, *idx) {
                    walk(inner, types, idx);
                }
            }
            ContentExpr::ZeroOrMore(inner) | ContentExpr::OneOrMore(inner) => {
                while matches_at(inner, types, *idx) {
                    walk(inner, types, idx);
                }
            }
            ContentExpr::Choice(cs) => {
                if let Some(c) = cs.iter().find(|c| matches_at(c, types, *idx)) {
                    walk(c, types, idx);
                }
            }
            ContentExpr::Seq(es) => {
                for e in es {
                    walk(e, types, idx);
                }
            }
        }
    }
    let mut idx = 0;
    walk(expr, types, &mut idx);
    idx
}

/// The index of the first content residue among `node`'s children (the first real
/// child greedy matching cannot consume), treating `Unknown` as transparent.
/// `None` when the content is a completable prefix (missing required slots are a
/// completion concern, not a residue).
fn content_residue_index(node: &RawNode) -> Option<usize> {
    if node.node_type == NodeType::Unknown {
        return None;
    }
    let reals: Vec<(usize, NodeType)> = node
        .children
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            c.as_child_type()
                .filter(|t| *t != NodeType::Unknown)
                .map(|t| (i, t))
        })
        .collect();
    let types: Vec<NodeType> = reals.iter().map(|(_, t)| *t).collect();
    let consumed = greedy_consume(&node.node_type.spec().content, &types);
    (consumed < reals.len()).then(|| reals[consumed].0)
}

fn min_opt(a: Option<usize>, b: Option<usize>) -> Option<usize> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    }
}

/// The index of the first child that cannot legally stay a direct child here — a
/// content residue or an actual-path context violation — in document order.
fn first_misfit(node: &RawNode, path: &[NodeType]) -> Option<usize> {
    let ctx_viol = (node.node_type != NodeType::Unknown)
        .then(|| {
            node.children.iter().position(|c| {
                c.as_child_type()
                    .is_some_and(|ct| ct != NodeType::Unknown && !context_allows(path, ct))
            })
        })
        .flatten();
    min_opt(ctx_viol, content_residue_index(node))
}

/// Whether the child at `i` is a misfit here (context violation or the first
/// content residue). Correct when every child before `i` already fits and
/// `residue` equals `content_residue_index(node)` for the node's current
/// children — callers cache it across loop iterations and must recompute it
/// after any mutation of `node.children`.
fn is_misfit_at(node: &RawNode, i: usize, path: &[NodeType], residue: Option<usize>) -> bool {
    debug_assert_eq!(residue, content_residue_index(node));
    let Some(ct) = node.children[i].as_child_type() else {
        return false;
    };
    if ct == NodeType::Unknown {
        return false;
    }
    if node.node_type != NodeType::Unknown && !context_allows(path, ct) {
        return true;
    }
    residue == Some(i)
}

fn child_own_dot(c: &RawChild) -> Dot {
    match c {
        RawChild::Leaf { id, .. } => *id,
        RawChild::Block(b) => b.id,
    }
}

fn first_real_dot_child(c: &RawChild) -> Option<Dot> {
    match c {
        RawChild::Leaf { id, .. } => (!id.is_synthetic()).then_some(*id),
        RawChild::Block(b) => first_real_dot_node(b),
    }
}

fn first_real_dot_node(n: &RawNode) -> Option<Dot> {
    if !n.id.is_synthetic() {
        return Some(n.id);
    }
    n.children.iter().find_map(first_real_dot_child)
}

/// The `Dot` that keys a content-owning scaffold: the first real dot it wraps, so
/// the id is replica-, window-, and reprojection-invariant. Falls back to the
/// child's own dot only when it wraps purely synthetic content.
fn wrap_cause(c: &RawChild) -> Dot {
    first_real_dot_child(c).unwrap_or_else(|| child_own_dot(c))
}

/// Build a content-owning scaffold of `role` holding `children`, keyed by
/// `(cause real dot, depth, role, kind)`, then fill any missing required slots
/// AFTER inserting the real children (so they are never displaced into a re-wrap
/// cycle). WRAP scaffolds register `(id, depth)` for the wrap-once rule; split
/// tail containers do not. The `split_tail` kind bit keeps a WRAP scaffold and a
/// SPLIT-tail scaffold distinct even when the same real content reaches both at
/// the same `(cause, depth, role)` — otherwise they collide to one id and the
/// same subtree is grafted twice.
fn empty_scaffold(
    role: NodeType,
    children: Vec<RawChild>,
    cause: Dot,
    depth: usize,
    register: bool,
    split_tail: bool,
    ctx: &mut RepairCtx,
) -> RawNode {
    let id = synthetic_id(cause, depth * 2 + split_tail as usize, role);
    let mut node = RawNode {
        id,
        node_type: role,
        attrs: vec![],
        children,
    };
    if register {
        ctx.wrap_created.insert((id, depth));
    }
    complete_required(&mut node);
    node
}

/// Insert deterministic fillers for `node`'s missing required content slots,
/// preserving every existing child (including `Unknown`s) in place. Assumes the
/// node is residue-free (misfits already resolved).
fn complete_required(node: &mut RawNode) {
    if node.node_type == NodeType::Unknown {
        return;
    }
    let content = node.node_type.spec().content.clone();
    // Already valid — do not run the greedy filler, whose repeatable groups would
    // absorb an existing trailing required slot and re-scaffold it (non-idempotent).
    let real_types: Vec<NodeType> = node
        .children
        .iter()
        .filter_map(|c| c.as_child_type())
        .filter(|t| *t != NodeType::Unknown)
        .collect();
    if content.matches_sequence(&real_types) {
        return;
    }
    let mut kids: VecDeque<RawChild> = std::mem::take(&mut node.children).into_iter().collect();
    let mut out = Vec::new();
    match_content(&content, &mut kids, node.id, &mut out);
    skip_unknowns(&mut kids, &mut out);
    out.extend(kids);
    node.children = out;
}

/// The WRAP plan for the misfit child at `i`: the scaffold chain to wrap it in, or
/// `None` when it must SPLIT-HOIST instead — the type fits directly (an
/// empty chain, i.e. a count/position overflow), no chain exists, the context is
/// unsatisfiable, or the child is a wrap scaffold already created at this level.
fn wrap_plan(
    node: &RawNode,
    i: usize,
    path: &[NodeType],
    ctx: &RepairCtx,
) -> Option<Vec<NodeType>> {
    let ct = node.children[i].as_child_type()?;
    if ct == NodeType::Unknown {
        return None;
    }
    let chain = wrap_chain(path, ct)?;
    if chain.is_empty() {
        return None;
    }
    if let RawChild::Block(b) = &node.children[i]
        && ctx.wrap_created.contains(&(b.id, path.len()))
    {
        return None;
    }
    Some(chain)
}

/// Replace the child at `i` with a scaffold chain (`chain[0]` outermost) that
/// makes its type context-legal here. No-op when the budget is exhausted.
fn wrap_child(
    node: &mut RawNode,
    i: usize,
    chain: &[NodeType],
    path: &[NodeType],
    ctx: &mut RepairCtx,
) {
    if !ctx.spend() {
        return;
    }
    let child = node.children.remove(i);
    let cause = wrap_cause(&child);
    let base_depth = path.len();
    let mut current = child;
    for (pos, &role) in chain.iter().enumerate().rev() {
        let scaffold = empty_scaffold(
            role,
            vec![current],
            cause,
            base_depth + pos,
            pos == 0,
            false,
            ctx,
        );
        current = RawChild::Block(scaffold);
    }
    node.children.insert(i, current);
}

/// Promote the child at slot `k` to the parent level, splitting `node` at its
/// sequence position: `node` keeps `[0, k)` (completed), and the returned forest —
/// `[promoted, tail-scaffold?]` — is inserted right after `node` by the caller.
/// The tail (slots after `k`) is wrapped in a scaffold of `node`'s own type so it
/// trails the promotion in sequence order. The sole order-preserving move. No-op
/// (empty return) when the budget is exhausted.
fn split_out(
    node: &mut RawNode,
    k: usize,
    path: &[NodeType],
    ctx: &mut RepairCtx,
) -> Vec<RawChild> {
    if !ctx.spend() {
        return Vec::new();
    }
    let tail: Vec<RawChild> = node.children.split_off(k + 1);
    let promoted = node.children.pop().expect("k is a valid child index");
    complete_required(node);
    let mut out = vec![promoted];
    if !tail.is_empty() {
        let cause = tail
            .iter()
            .find_map(first_real_dot_child)
            .unwrap_or_else(|| child_own_dot(&tail[0]));
        let scaffold = empty_scaffold(
            node.node_type,
            tail,
            cause,
            path.len().saturating_sub(1),
            false,
            true,
            ctx,
        );
        out.push(RawChild::Block(scaffold));
    }
    out
}

/// Resolve every own-misfit of `node` (WRAP in place or SPLIT-HOIST) without
/// recursing into children. Returns the hoist forest a terminating SPLIT produces
/// (empty otherwise). Used by the shallow Root-content path.
fn resolve_own_misfits(
    node: &mut RawNode,
    path: &[NodeType],
    ctx: &mut RepairCtx,
) -> Vec<RawChild> {
    loop {
        if ctx.capped {
            return Vec::new();
        }
        let Some(k) = first_misfit(node, path) else {
            return Vec::new();
        };
        match wrap_plan(node, k, path, ctx) {
            Some(chain) => wrap_child(node, k, &chain, path, ctx),
            None => return split_out(node, k, path, ctx),
        }
    }
}

fn normalize_grid(table: &mut RawNode) {
    let width = table
        .children
        .iter()
        .filter_map(|c| match c {
            RawChild::Block(r) if r.node_type == NodeType::TableRow => Some(
                r.children
                    .iter()
                    .filter(|cc| cc.as_child_type() == Some(NodeType::TableCell))
                    .count(),
            ),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    for slot in 0..table.children.len() {
        let Some(RawChild::Block(mut row)) = table.children.get(slot).cloned() else {
            continue;
        };
        if row.node_type != NodeType::TableRow {
            continue;
        }
        let mut count = row
            .children
            .iter()
            .filter(|cc| cc.as_child_type() == Some(NodeType::TableCell))
            .count();
        if count >= width {
            continue;
        }
        while count < width {
            let cell = scaffold_block(NodeType::TableCell, count, row.id);
            row.children.push(RawChild::Block(cell));
            count += 1;
        }
        table.children[slot] = RawChild::Block(row);
    }
}

fn scaffold_block(role: NodeType, slot: usize, parent: Dot) -> RawNode {
    let id = synthetic_id(parent, slot, role);
    let mut out = Vec::new();
    match_content(&role.spec().content, &mut VecDeque::new(), id, &mut out);
    RawNode {
        id,
        node_type: role,
        attrs: vec![],
        children: out,
    }
}

fn first_type(e: &ContentExpr) -> NodeType {
    match e {
        ContentExpr::Single(t) => *t,
        ContentExpr::Choice(cs) => first_type(&cs[0]),
        ContentExpr::OneOrMore(i) | ContentExpr::ZeroOrMore(i) | ContentExpr::Optional(i) => {
            first_type(i)
        }
        ContentExpr::Seq(es) => first_type(&es[0]),
        ContentExpr::Empty | ContentExpr::Any => unreachable!(),
    }
}

fn last_known_child(children: &[RawChild]) -> Option<&RawChild> {
    children.iter().rev().find(|child| {
        child
            .as_child_type()
            .is_some_and(|node_type| node_type != NodeType::Unknown)
    })
}

fn complete_root_trailing_editable_paragraph(node: &mut RawNode) {
    if node.node_type != NodeType::Root {
        return;
    }
    let Some(RawChild::Block(paragraph)) = last_known_child(&node.children) else {
        return;
    };
    if paragraph.node_type != NodeType::Paragraph
        || last_known_child(&paragraph.children).and_then(RawChild::as_child_type)
            != Some(NodeType::PageBreak)
    {
        return;
    }
    node.children.push(RawChild::Block(scaffold_block(
        NodeType::Paragraph,
        0,
        node.id,
    )));
}

fn fix_roots(tree: &mut RawTree) {
    let mut tops = std::mem::take(&mut tree.roots);
    match tops.iter().position(|r| r.node_type == NodeType::Root) {
        Some(i) => {
            let mut root = tops.remove(i);
            root.children.extend(tops.into_iter().map(RawChild::Block));
            tree.roots = vec![root];
        }
        None => {
            tree.roots = vec![RawNode {
                id: Dot::ROOT,
                node_type: NodeType::Root,
                attrs: vec![],
                children: tops.into_iter().map(RawChild::Block).collect(),
            }];
        }
    }
}

pub fn normalize(tree: RawTree) -> RawTree {
    let mut stats = RepairStats::default();
    normalize_with_stats(tree, &mut stats)
}

/// Repair `tree` into a schema-valid nested tree with a single deterministic
/// algebra: revival attachment (in the walk), then per-node WRAP / SPLIT-HOIST /
/// completion. Nothing is dropped — every real op is preserved and re-placed.
/// `stats` accumulates the repairs applied (and `projection_degraded` if the
/// pass cap is reached).
pub fn normalize_with_stats(tree: RawTree, stats: &mut RepairStats) -> RawTree {
    normalize_capped(tree, stats, effective_repair_budget())
}

fn normalize_capped(mut tree: RawTree, stats: &mut RepairStats, budget: usize) -> RawTree {
    fix_roots(&mut tree);
    let mut ctx = RepairCtx::new(stats, budget);
    for r in &mut tree.roots {
        let mut path = Vec::new();
        let hoist = normalize_node(r, &mut path, &mut ctx);
        debug_assert!(hoist.is_empty(), "Root cannot hoist to a parent level");
        // Defensive totality: a hoist here is unreachable (Root accepts every
        // block type), but keep any escapee rather than lose it.
        r.children.extend(hoist);
    }
    if ctx.capped {
        ctx.stats.projection_degraded = true;
    }
    tree
}

/// Normalize a window's Root-level forest with the SAME WRAP/SPLIT algebra the
/// full-document Root pass uses: each context-illegal top-level child is WRAPped
/// (e.g. a bare `ListItem` into a `BulletList`) BEFORE its content is recursed, so a
/// child's SPLIT-HOIST lands at its wrapping scaffold's level exactly as a cold
/// rebuild places it — not one level too high at Root. The Root *completion* rules
/// (required content, trailing editable paragraph) are deliberately omitted: they are
/// document-global and remain the incremental Root-content reconcile's job; applying
/// them to a window scaffolds spurious mid-document content. Returns the normalized
/// forest (WRAP scaffolds included) for the incremental caller to graft and index.
pub fn normalize_window_forest_with_stats(
    children: Vec<RawChild>,
    stats: &mut RepairStats,
) -> Vec<RawChild> {
    let mut root = RawNode {
        id: Dot::ROOT,
        node_type: NodeType::Root,
        attrs: Vec::new(),
        children,
    };
    let mut ctx = RepairCtx::new(stats, effective_repair_budget());
    // Mirror `process_children` for the Root node WITHOUT its terminal
    // `complete_required` (Root completion is applied document-globally elsewhere).
    let mut path = vec![NodeType::Root];
    let mut residue = content_residue_index(&root);
    let mut i = 0;
    while i < root.children.len() {
        if ctx.capped {
            break;
        }
        if is_misfit_at(&root, i, &path, residue) {
            match wrap_plan(&root, i, &path, &ctx) {
                Some(chain) => {
                    wrap_child(&mut root, i, &chain, &path, &mut ctx);
                    residue = content_residue_index(&root);
                    // The scaffold now fits here; re-examine slot i (recursed next pass).
                    continue;
                }
                None => {
                    // Root accepts every block type through WRAP, so a terminating
                    // SPLIT here is unreachable; keep the promoted forest as Root
                    // children rather than lose it (total).
                    let hoist = split_out(&mut root, i, &path, &mut ctx);
                    root.children.splice(i + 1..i + 1, hoist);
                    residue = content_residue_index(&root);
                    i += 1;
                    continue;
                }
            }
        }
        let child_hoist = if let RawChild::Block(b) = &mut root.children[i] {
            normalize_node(b, &mut path, &mut ctx)
        } else {
            Vec::new()
        };
        if !child_hoist.is_empty() {
            let husk_emptied =
                matches!(&root.children[i], RawChild::Block(b) if first_real_dot_node(b).is_none());
            if husk_emptied {
                root.children.splice(i..i + 1, child_hoist);
                residue = content_residue_index(&root);
                continue;
            }
            root.children.splice(i + 1..i + 1, child_hoist);
            residue = content_residue_index(&root);
        }
        i += 1;
    }
    if ctx.capped {
        ctx.stats.projection_degraded = true;
    }
    root.children
}

pub fn normalize_window_forest_for(
    container_id: Dot,
    container_type: NodeType,
    ancestors: &[NodeType],
    children: Vec<RawChild>,
    stats: &mut RepairStats,
) -> (Vec<RawChild>, Vec<RawChild>) {
    let mut root = RawNode {
        id: container_id,
        node_type: container_type,
        attrs: Vec::new(),
        children,
    };
    let mut ctx = RepairCtx::new(stats, effective_repair_budget());
    let mut path = ancestors.to_vec();
    path.push(container_type);
    let mut hoisted: Vec<RawChild> = Vec::new();
    let mut residue = content_residue_index(&root);
    let mut i = 0;
    while i < root.children.len() {
        if ctx.capped {
            break;
        }
        if is_misfit_at(&root, i, &path, residue) {
            match wrap_plan(&root, i, &path, &ctx) {
                Some(chain) => {
                    wrap_child(&mut root, i, &chain, &path, &mut ctx);
                    residue = content_residue_index(&root);
                    continue;
                }
                None => {
                    hoisted = split_out(&mut root, i, &path, &mut ctx);
                    break;
                }
            }
        }
        let child_hoist = if let RawChild::Block(b) = &mut root.children[i] {
            normalize_node(b, &mut path, &mut ctx)
        } else {
            Vec::new()
        };
        if !child_hoist.is_empty() {
            let husk_emptied =
                matches!(&root.children[i], RawChild::Block(b) if first_real_dot_node(b).is_none());
            if husk_emptied {
                root.children.splice(i..i + 1, child_hoist);
                residue = content_residue_index(&root);
                continue;
            }
            root.children.splice(i + 1..i + 1, child_hoist);
            residue = content_residue_index(&root);
        }
        i += 1;
    }
    if ctx.capped {
        ctx.stats.projection_degraded = true;
    }
    (root.children, hoisted)
}

/// Normalize a single block's subtree in place under the given ancestor types,
/// applying only that block's (and descendants') content rules — NOT the
/// document Root rules. For localized re-projection of one top-level block.
pub fn normalize_subtree(node: &mut RawNode, ancestors: &[NodeType]) {
    let mut stats = RepairStats::default();
    normalize_subtree_with_stats(node, ancestors, &mut stats);
}

/// As [`normalize_subtree`], accumulating repairs. Returns the hoist forest the
/// subtree promotes to its parent level (empty for a self-contained subtree);
/// the incremental caller splices it in as following top-level siblings.
pub fn normalize_subtree_with_stats(
    node: &mut RawNode,
    ancestors: &[NodeType],
    stats: &mut RepairStats,
) -> Vec<RawChild> {
    let mut path = ancestors.to_vec();
    let mut ctx = RepairCtx::new(stats, DEFAULT_REPAIR_BUDGET);
    let hoist = normalize_node(node, &mut path, &mut ctx);
    if ctx.capped {
        ctx.stats.projection_degraded = true;
    }
    hoist
}

/// Apply only `node`'s own schema content rule (repairing its direct children),
/// assuming its children are already individually normalized — no deep recursion.
/// Lets a caller re-establish a container's content invariant (e.g. the Root's
/// required trailing paragraph) by deferring to the schema rather than hardcoding
/// it. Newly scaffolded children are themselves shaped by `match_content`, so they
/// need no further normalization here.
pub fn normalize_content_shallow(node: &mut RawNode, ancestors: &[NodeType]) {
    let mut stats = RepairStats::default();
    normalize_content_shallow_with_stats(node, ancestors, &mut stats);
}

/// As [`normalize_content_shallow`], accumulating repairs and returning the hoist
/// forest the own-content repair promotes above `node`. Shares the SPLIT/WRAP
/// algebra with the full path so the incremental Root shallow reconcile stays
/// totality-consistent with a full reprojection.
pub fn normalize_content_shallow_with_stats(
    node: &mut RawNode,
    ancestors: &[NodeType],
    stats: &mut RepairStats,
) -> Vec<RawChild> {
    let mut path = ancestors.to_vec();
    path.push(node.node_type);
    let mut ctx = RepairCtx::new(stats, DEFAULT_REPAIR_BUDGET);
    let hoist = resolve_own_misfits(node, &path, &mut ctx);
    complete_required(node);
    complete_root_trailing_editable_paragraph(node);
    if ctx.capped {
        ctx.stats.projection_degraded = true;
    }
    hoist
}

/// Repair `node` in place and return the forest it hoists to its PARENT level
/// (inserted right after `node` by the caller, then re-processed there). Processes
/// children in document order: the first misfit is WRAPped in place or
/// SPLIT-HOISTed (ending this node's own processing), and every fitting child is
/// recursed, its hoist spliced in and re-examined.
fn normalize_node(
    node: &mut RawNode,
    path: &mut Vec<NodeType>,
    ctx: &mut RepairCtx,
) -> Vec<RawChild> {
    path.push(node.node_type);
    let hoist = process_children(node, path, ctx);
    if node.node_type == NodeType::Table {
        normalize_grid(node);
    }
    complete_root_trailing_editable_paragraph(node);
    path.pop();
    hoist
}

fn process_children(
    node: &mut RawNode,
    path: &mut Vec<NodeType>,
    ctx: &mut RepairCtx,
) -> Vec<RawChild> {
    let mut residue = content_residue_index(node);
    let mut i = 0;
    while i < node.children.len() {
        if ctx.capped {
            return Vec::new();
        }
        if is_misfit_at(node, i, path, residue) {
            match wrap_plan(node, i, path, ctx) {
                Some(chain) => {
                    wrap_child(node, i, &chain, path, ctx);
                    residue = content_residue_index(node);
                    // The scaffold now fits here; re-examine slot i (it will be
                    // recursed on the next pass).
                    continue;
                }
                None => return split_out(node, i, path, ctx),
            }
        }
        let child_hoist = match &mut node.children[i] {
            RawChild::Block(b) => normalize_node(b, path, ctx),
            RawChild::Leaf { .. } => Vec::new(),
        };
        if !child_hoist.is_empty() {
            let husk_emptied =
                matches!(&node.children[i], RawChild::Block(b) if first_real_dot_node(b).is_none());
            if husk_emptied {
                node.children.splice(i..i + 1, child_hoist);
                residue = content_residue_index(node);
                continue;
            }
            node.children.splice(i + 1..i + 1, child_hoist);
            residue = content_residue_index(node);
        }
        i += 1;
    }
    complete_required(node);
    Vec::new()
}

#[cfg(test)]
mod tests {
    use editor_crdt::Dot;

    use super::*;
    use crate::seq::{AtomLeaf, BlockTree, validate_block_tree as validate_flat};

    fn valid(t: &RawTree) -> Result<(), crate::SchemaError> {
        validate_flat(&BlockTree::from_raw(t))
    }

    fn find_leaf(tree: &RawTree, id: Dot) -> Option<&super::super::SeqItem> {
        fn in_node(node: &RawNode, id: Dot) -> Option<&super::super::SeqItem> {
            node.children.iter().find_map(|child| match child {
                RawChild::Leaf { id: leaf_id, item } if *leaf_id == id => Some(item),
                RawChild::Block(block) => in_node(block, id),
                RawChild::Leaf { .. } => None,
            })
        }

        tree.roots.iter().find_map(|root| in_node(root, id))
    }

    use crate::seq::Child;

    fn raw_char(clock: u64, ch: char) -> RawChild {
        RawChild::Leaf {
            id: Dot::new(1, clock),
            item: super::super::SeqItem::Char(ch),
        }
    }

    fn raw_block(clock: u64, t: NodeType, children: Vec<RawChild>) -> RawNode {
        RawNode {
            attrs: vec![],
            id: Dot::new(1, clock),
            node_type: t,
            children,
        }
    }

    fn raw_block_child(clock: u64, t: NodeType, children: Vec<RawChild>) -> RawChild {
        RawChild::Block(raw_block(clock, t, children))
    }

    /// A `RawTree` whose Root uses `Dot::ROOT` (matching the production projection
    /// path and `fix_roots`, which never rewrites an existing Root's id).
    fn raw_root(children: Vec<RawChild>) -> RawTree {
        RawTree {
            roots: vec![RawNode {
                attrs: vec![],
                id: Dot::ROOT,
                node_type: NodeType::Root,
                children,
            }],
        }
    }

    /// Preorder real (authored, non-synthetic) dots of the projected tree — the
    /// order invariant's witness. Excludes the implicit Root and every scaffold.
    fn preorder_real_dots(tree: &BlockTree) -> Vec<Dot> {
        fn visit(tree: &BlockTree, c: &Child, out: &mut Vec<Dot>) {
            match c {
                Child::Leaf { id, .. } => {
                    if !id.is_synthetic() {
                        out.push(*id);
                    }
                }
                Child::Block(id) => {
                    if !id.is_synthetic() {
                        out.push(*id);
                    }
                    if let Some(b) = tree.get(*id) {
                        for cc in &b.children {
                            visit(tree, cc, out);
                        }
                    }
                }
            }
        }
        let mut out = Vec::new();
        if let Some(root) = tree.root_node() {
            for c in &root.children {
                visit(tree, c, &mut out);
            }
        }
        out
    }

    fn index_of(order: &[Dot], id: Dot) -> usize {
        order
            .iter()
            .position(|d| *d == id)
            .unwrap_or_else(|| panic!("{id:?} missing from preorder {order:?}"))
    }

    /// The set of real (authored, non-synthetic) dots anywhere in a nested raw tree
    /// — the totality witness: repair must never add to or remove from this set.
    fn raw_real_dots(tree: &RawTree) -> std::collections::BTreeSet<Dot> {
        fn collect(c: &RawChild, out: &mut std::collections::BTreeSet<Dot>) {
            match c {
                RawChild::Leaf { id, .. } => {
                    if !id.is_synthetic() {
                        out.insert(*id);
                    }
                }
                RawChild::Block(b) => {
                    if !b.id.is_synthetic() {
                        out.insert(b.id);
                    }
                    for cc in &b.children {
                        collect(cc, out);
                    }
                }
            }
        }
        let mut out = std::collections::BTreeSet::new();
        for r in &tree.roots {
            if !r.id.is_synthetic() {
                out.insert(r.id);
            }
            for c in &r.children {
                collect(c, &mut out);
            }
        }
        out
    }

    #[test]
    fn normalize_wraps_root_list_item_instead_of_dropping() {
        // Root rejects a bare ListItem, but its type is placeable via a BulletList
        // scaffold — WRAP, not drop. No content is lost; one repair is counted.
        let list_item = Dot::new(1, 1);
        let promoted_para = Dot::new(1, 2);
        let tree = raw_root(vec![raw_block_child(
            1,
            NodeType::ListItem,
            vec![raw_block_child(2, NodeType::Paragraph, vec![])],
        )]);
        let mut stats = RepairStats::default();
        let out = normalize_with_stats(tree, &mut stats);
        assert_eq!(stats.drops, 0, "nothing is dropped");
        assert!(stats.repairs >= 1, "wrapping the ListItem is a repair");
        let flat = BlockTree::from_raw(&out);
        assert!(
            flat.get(list_item).is_some(),
            "the ListItem marker survives (wrapped in a BulletList)"
        );
        assert!(
            flat.get(promoted_para).is_some(),
            "its Paragraph survives inside the ListItem"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_fills_root_trailing_paragraph_and_empty_blockquote() {
        let tree = raw_root(vec![raw_block_child(1, NodeType::Blockquote, vec![])]);
        let out = normalize(tree);
        let root = &out.roots[0];
        // Blockquote stays first, a synthetic trailing Paragraph is appended, and the
        // empty Blockquote is completed with its required Paragraph.
        assert_eq!(root.children[0].as_child_type(), Some(NodeType::Blockquote));
        assert!(matches!(
            root.children.last(),
            Some(RawChild::Block(b)) if b.node_type == NodeType::Paragraph && b.id.is_synthetic()
        ));
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_root_adds_trailing_paragraph_after_page_break() {
        let paragraph_id = Dot::new(1, 1);
        let tree = RawTree {
            roots: vec![RawNode {
                attrs: vec![],
                id: Dot::ROOT,
                node_type: NodeType::Root,
                children: vec![RawChild::Block(RawNode {
                    attrs: vec![],
                    id: paragraph_id,
                    node_type: NodeType::Paragraph,
                    children: vec![RawChild::Leaf {
                        id: Dot::new(1, 2),
                        item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                    }],
                })],
            }],
        };

        let normalized = normalize(tree);
        let root = &normalized.roots[0];

        assert_eq!(root.children.len(), 2);
        assert!(matches!(
            &root.children[0],
            RawChild::Block(paragraph) if paragraph.id == paragraph_id
        ));
        assert!(matches!(
            &root.children[1],
            RawChild::Block(paragraph)
                if paragraph.node_type == NodeType::Paragraph && paragraph.id.is_synthetic()
        ));
    }

    #[test]
    fn normalize_content_shallow_adds_trailing_paragraph_after_page_break() {
        let mut root = RawNode {
            attrs: vec![],
            id: Dot::ROOT,
            node_type: NodeType::Root,
            children: vec![RawChild::Block(RawNode {
                attrs: vec![],
                id: Dot::new(1, 1),
                node_type: NodeType::Paragraph,
                children: vec![RawChild::Leaf {
                    id: Dot::new(1, 2),
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                }],
            })],
        };

        normalize_content_shallow(&mut root, &[]);

        assert!(matches!(
            &root.children[..],
            [RawChild::Block(_), RawChild::Block(paragraph)]
                if paragraph.node_type == NodeType::Paragraph && paragraph.id.is_synthetic()
        ));
    }

    #[test]
    fn normalize_root_ignores_unknown_after_terminal_page_break() {
        let tree = RawTree {
            roots: vec![RawNode {
                attrs: vec![],
                id: Dot::ROOT,
                node_type: NodeType::Root,
                children: vec![RawChild::Block(RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 1),
                    node_type: NodeType::Paragraph,
                    children: vec![
                        RawChild::Leaf {
                            id: Dot::new(1, 2),
                            item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                        },
                        RawChild::Leaf {
                            id: Dot::new(1, 3),
                            item: super::super::SeqItem::Unknown {
                                tag: 999,
                                bytes: vec![],
                            },
                        },
                    ],
                })],
            }],
        };

        let normalized = normalize(tree);

        assert!(matches!(
            &normalized.roots[0].children[..],
            [RawChild::Block(_), RawChild::Block(paragraph)]
                if paragraph.node_type == NodeType::Paragraph && paragraph.id.is_synthetic()
        ));
    }

    #[test]
    fn context_invalid_pagebreak_is_hoisted_to_root_paragraph_not_dropped() {
        // A PageBreak nested two levels deep (Blockquote > Paragraph) is
        // context-invalid there; it is SPLIT-HOISTed to Root and WRAPped in a
        // Paragraph (its only legal context) rather than dropped.
        let pagebreak = Dot::new(1, 2);
        let tree = raw_root(vec![raw_block_child(
            1,
            NodeType::Blockquote,
            vec![raw_block_child(
                3,
                NodeType::Paragraph,
                vec![RawChild::Leaf {
                    id: pagebreak,
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                }],
            )],
        )]);
        let out = normalize(tree);
        assert!(
            find_leaf(&out, pagebreak).is_some(),
            "the PageBreak survives (moved to a Root-level Paragraph)"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn extra_paragraph_in_list_item_becomes_sibling_item() {
        // <li><p>a</p><p>b</p></li> — the surplus Paragraph is SPLIT-HOISTed to the
        // list level and WRAPped into its own ListItem: siblings, not a merge.
        let a = Dot::new(1, 4);
        let b = Dot::new(1, 6);
        let tree = raw_root(vec![raw_block_child(
            1,
            NodeType::BulletList,
            vec![raw_block_child(
                2,
                NodeType::ListItem,
                vec![
                    raw_block_child(3, NodeType::Paragraph, vec![raw_char(4, 'a')]),
                    raw_block_child(5, NodeType::Paragraph, vec![raw_char(6, 'b')]),
                ],
            )],
        )]);
        let out = normalize(tree);
        let flat = BlockTree::from_raw(&out);
        let order = preorder_real_dots(&flat);
        assert!(index_of(&order, a) < index_of(&order, b), "a precedes b");
        let list = flat.get(Dot::new(1, 1)).expect("bullet list present");
        assert_eq!(list.children.len(), 2, "two sibling ListItems");
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn stray_char_under_list_scaffolds_item_and_tail_survives() {
        // [Item(a), stray char z, Item(b)] — z is WRAPped into a scaffold
        // ListItem>Paragraph in sequence position; Item(b) survives after it.
        let a = Dot::new(1, 4);
        let z = Dot::new(1, 5);
        let b = Dot::new(1, 8);
        let item_b = Dot::new(1, 6);
        let tree = raw_root(vec![raw_block_child(
            1,
            NodeType::BulletList,
            vec![
                raw_block_child(
                    2,
                    NodeType::ListItem,
                    vec![raw_block_child(
                        3,
                        NodeType::Paragraph,
                        vec![raw_char(4, 'a')],
                    )],
                ),
                raw_char(5, 'z'),
                raw_block_child(
                    6,
                    NodeType::ListItem,
                    vec![raw_block_child(
                        7,
                        NodeType::Paragraph,
                        vec![raw_char(8, 'b')],
                    )],
                ),
            ],
        )]);
        let out = normalize(tree);
        let flat = BlockTree::from_raw(&out);
        let order = preorder_real_dots(&flat);
        assert!(index_of(&order, a) < index_of(&order, z));
        assert!(index_of(&order, z) < index_of(&order, b));
        assert!(flat.get(item_b).is_some(), "Item(b) survives");
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn duplicate_fold_title_splits_in_sequence_order() {
        let title1 = Dot::new(1, 2);
        let title2 = Dot::new(1, 3);
        let content = Dot::new(1, 4);
        let tree = raw_root(vec![raw_block_child(
            1,
            NodeType::Fold,
            vec![
                raw_block_child(2, NodeType::FoldTitle, vec![]),
                raw_block_child(3, NodeType::FoldTitle, vec![]),
                raw_block_child(
                    4,
                    NodeType::FoldContent,
                    vec![raw_block_child(5, NodeType::Paragraph, vec![])],
                ),
            ],
        )]);
        let out = normalize(tree);
        let flat = BlockTree::from_raw(&out);
        let order = preorder_real_dots(&flat);
        assert!(index_of(&order, title1) < index_of(&order, title2));
        assert!(index_of(&order, title2) < index_of(&order, content));
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn reversed_fold_is_split_not_reordered() {
        // Fold[Content, Title] — no physical reorder: Fold[scaffold-Title, Content]
        // followed by Fold[Title, scaffold-Content]; preorder Content < Title.
        let content = Dot::new(1, 2);
        let title = Dot::new(1, 4);
        let tree = raw_root(vec![raw_block_child(
            1,
            NodeType::Fold,
            vec![
                raw_block_child(
                    2,
                    NodeType::FoldContent,
                    vec![raw_block_child(3, NodeType::Paragraph, vec![])],
                ),
                raw_block_child(4, NodeType::FoldTitle, vec![]),
            ],
        )]);
        let out = normalize(tree);
        let flat = BlockTree::from_raw(&out);
        let order = preorder_real_dots(&flat);
        assert!(
            index_of(&order, content) < index_of(&order, title),
            "Content stays before Title — split, not reorder"
        );
        // First Fold: scaffold FoldTitle, then the real Content.
        let first_fold = flat.get(Dot::new(1, 1)).expect("original fold");
        assert!(matches!(
            first_fold.children.first(),
            Some(Child::Block(id)) if flat.get(*id).map(|b| b.node_type) == Some(NodeType::FoldTitle) && id.is_synthetic()
        ));
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn leading_unknown_in_fold_is_attributed_to_the_fold_with_no_repair() {
        // Fold[Unknown, FoldTitle, FoldContent] — the transparent Unknown belongs
        // ahead of its real siblings; the roles are already in order, so nothing
        // repairs and preorder keeps Unknown < FoldTitle < FoldContent.
        let tree = raw_root(vec![raw_block_child(
            1,
            NodeType::Fold,
            vec![
                raw_block_child(2, NodeType::Unknown, vec![]),
                raw_block_child(3, NodeType::FoldTitle, vec![]),
                raw_block_child(
                    4,
                    NodeType::FoldContent,
                    vec![raw_block_child(5, NodeType::Paragraph, vec![])],
                ),
            ],
        )]);
        let mut stats = RepairStats::default();
        let out = normalize_with_stats(tree, &mut stats);
        assert_eq!(
            stats.repairs, 0,
            "an already-valid fold with a leading Unknown needs no repair"
        );
        let flat = BlockTree::from_raw(&out);
        let fold = flat.get(Dot::new(1, 1)).expect("fold");
        let child_types: Vec<NodeType> = fold
            .children
            .iter()
            .filter_map(|c| match c {
                Child::Block(id) => flat.get(*id).map(|b| b.node_type),
                Child::Leaf { .. } => None,
            })
            .collect();
        assert_eq!(
            child_types,
            vec![
                NodeType::Unknown,
                NodeType::FoldTitle,
                NodeType::FoldContent
            ],
            "the Unknown stays first and the typed roles follow in order"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn nested_table_split_preserves_order() {
        // TableCell = [P1, nested Table, P2] — the nested Table cannot live in a cell
        // (content + context), so it SPLIT-HOISTs out to Root, and P2 rides a scaffold
        // tail: preorder P1 < Table < P2, no P1/P2/Table flattening.
        let p1 = Dot::new(1, 4);
        let nested_table = Dot::new(1, 6);
        let p2 = Dot::new(1, 11);
        let tree = raw_root(vec![raw_block_child(
            1,
            NodeType::Table,
            vec![raw_block_child(
                2,
                NodeType::TableRow,
                vec![raw_block_child(
                    3,
                    NodeType::TableCell,
                    vec![
                        raw_block_child(4, NodeType::Paragraph, vec![raw_char(5, '1')]),
                        raw_block_child(
                            6,
                            NodeType::Table,
                            vec![raw_block_child(
                                7,
                                NodeType::TableRow,
                                vec![raw_block_child(
                                    8,
                                    NodeType::TableCell,
                                    vec![raw_block_child(
                                        9,
                                        NodeType::Paragraph,
                                        vec![raw_char(10, 'n')],
                                    )],
                                )],
                            )],
                        ),
                        raw_block_child(11, NodeType::Paragraph, vec![raw_char(12, '2')]),
                    ],
                )],
            )],
        )]);
        let out = normalize(tree);
        let flat = BlockTree::from_raw(&out);
        let order = preorder_real_dots(&flat);
        assert!(index_of(&order, p1) < index_of(&order, nested_table));
        assert!(index_of(&order, nested_table) < index_of(&order, p2));
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn block_atom_hoists_as_leaf_not_paragraph_carrier() {
        // A block-level atom (Image) directly under a BulletList promotes as a bare
        // leaf up to Root — never wrapped in a Paragraph carrier — and survives.
        let image = Dot::new(1, 2);
        let image_item = super::super::SeqItem::Atom(AtomLeaf::Image {
            node: crate::nodes::ImageNode::default(),
        });
        let tree = raw_root(vec![raw_block_child(
            1,
            NodeType::BulletList,
            vec![
                RawChild::Leaf {
                    id: image,
                    item: image_item,
                },
                raw_block_child(
                    3,
                    NodeType::ListItem,
                    vec![raw_block_child(
                        4,
                        NodeType::Paragraph,
                        vec![raw_char(5, 'x')],
                    )],
                ),
            ],
        )]);
        let out = normalize(tree);
        // The Image is a direct leaf child of Root, not inside a Paragraph.
        assert!(
            out.roots[0].children.iter().any(|c| matches!(
                c,
                RawChild::Leaf { id, item }
                    if *id == image && item.as_child_type() == Some(NodeType::Image)
            )),
            "Image survives as a Root-level leaf, not a Paragraph carrier"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn pass_cap_fallback_preserves_totality_and_order() {
        // Two stray Text leaves each need a WRAP; a budget of 1 forces the fallback
        // after the first. The partial tree keeps every real dot and sequence order,
        // reports projection_degraded with no totality_violations, and schema
        // validity is NOT guaranteed on this path.
        let tree = raw_root(vec![raw_block_child(
            1,
            NodeType::BulletList,
            vec![raw_char(2, 'a'), raw_char(3, 'b')],
        )]);
        let before: std::collections::BTreeSet<Dot> = {
            let mut set = std::collections::BTreeSet::new();
            fn collect(c: &RawChild, set: &mut std::collections::BTreeSet<Dot>) {
                match c {
                    RawChild::Leaf { id, .. } => {
                        if !id.is_synthetic() {
                            set.insert(*id);
                        }
                    }
                    RawChild::Block(b) => {
                        if !b.id.is_synthetic() {
                            set.insert(b.id);
                        }
                        for cc in &b.children {
                            collect(cc, set);
                        }
                    }
                }
            }
            for c in &tree.roots[0].children {
                collect(c, &mut set);
            }
            set
        };
        let mut stats = RepairStats::default();
        let out = normalize_capped(tree, &mut stats, 1);
        let flat = BlockTree::from_raw(&out);
        let order = preorder_real_dots(&flat);
        let after: std::collections::BTreeSet<Dot> = order.iter().copied().collect();
        assert_eq!(before, after, "every real dot survives the cap fallback");
        assert_eq!(
            order,
            vec![Dot::new(1, 1), Dot::new(1, 2), Dot::new(1, 3)],
            "preorder equals sequence order under the fallback"
        );
        assert!(
            stats.projection_degraded,
            "cap-hit sets projection_degraded"
        );
        assert_eq!(
            stats.totality_violations, 0,
            "cap-hit is not a measured totality deficit"
        );
    }

    #[test]
    fn fix_roots_wraps_non_root_top() {
        let tree = RawTree {
            roots: vec![RawNode {
                attrs: vec![],
                id: Dot::new(1, 0),
                node_type: NodeType::Paragraph,
                children: vec![],
            }],
        };
        let out = normalize(tree);
        assert_eq!(out.roots.len(), 1);
        assert_eq!(out.roots[0].node_type, NodeType::Root);
        assert!(
            out.roots[0]
                .children
                .iter()
                .any(|c| c.as_child_type() == Some(NodeType::Paragraph))
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_messy_sample_is_valid_and_idempotent() {
        let leaf = |i: u64, item: super::super::SeqItem| RawChild::Leaf {
            id: Dot::new(1, i),
            item,
        };
        let blk = |i: u64, t: NodeType, children: Vec<RawChild>| {
            RawChild::Block(RawNode {
                attrs: vec![],
                id: Dot::new(1, i),
                node_type: t,
                children,
            })
        };
        let tree = RawTree {
            roots: vec![
                RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 1),
                    node_type: NodeType::Paragraph,
                    children: vec![leaf(2, super::super::SeqItem::Char('a'))],
                },
                RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 3),
                    node_type: NodeType::Fold,
                    children: vec![],
                },
                RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 4),
                    node_type: NodeType::Fold,
                    children: vec![
                        blk(5, NodeType::FoldContent, vec![]),
                        blk(6, NodeType::FoldTitle, vec![]),
                    ],
                },
                RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 7),
                    node_type: NodeType::Blockquote,
                    children: vec![blk(
                        8,
                        NodeType::Paragraph,
                        vec![leaf(9, super::super::SeqItem::Atom(AtomLeaf::PageBreak))],
                    )],
                },
                RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 10),
                    node_type: NodeType::BulletList,
                    children: vec![leaf(11, super::super::SeqItem::Char('z'))],
                },
            ],
        };
        assert!(valid(&normalize(tree.clone())).is_ok());
        assert_eq!(normalize(normalize(tree.clone())), normalize(tree));
    }

    fn tcell(i: u64) -> RawChild {
        RawChild::Block(RawNode {
            attrs: vec![],
            id: Dot::new(2, i),
            node_type: NodeType::TableCell,
            children: vec![],
        })
    }

    fn trow(id: u64, cells: Vec<RawChild>) -> RawChild {
        RawChild::Block(RawNode {
            attrs: vec![],
            id: Dot::new(2, id),
            node_type: NodeType::TableRow,
            children: cells,
        })
    }

    fn ttable(rows: Vec<RawChild>) -> RawNode {
        RawNode {
            attrs: vec![],
            id: Dot::new(2, 0),
            node_type: NodeType::Table,
            children: rows,
        }
    }

    fn cell_count(row: &RawChild) -> usize {
        match row {
            RawChild::Block(b) => b
                .children
                .iter()
                .filter(|c| c.as_child_type() == Some(NodeType::TableCell))
                .count(),
            _ => 0,
        }
    }

    #[test]
    fn normalize_grid_pads_short_rows_to_max() {
        let mut t = ttable(vec![
            trow(10, vec![tcell(11), tcell(12)]),
            trow(20, vec![tcell(21), tcell(22), tcell(23)]),
        ]);
        normalize_grid(&mut t);
        assert_eq!(cell_count(&t.children[0]), 3);
        assert_eq!(cell_count(&t.children[1]), 3);
    }

    #[test]
    fn normalize_grid_rectangular_is_noop() {
        let mut t = ttable(vec![
            trow(10, vec![tcell(11), tcell(12)]),
            trow(20, vec![tcell(21), tcell(22)]),
        ]);
        let before = t.clone();
        normalize_grid(&mut t);
        assert_eq!(t, before);
    }

    #[test]
    fn normalize_grid_pad_cells_have_distinct_slots() {
        let mut t = ttable(vec![
            trow(10, vec![tcell(11)]),
            trow(20, vec![tcell(21), tcell(22), tcell(23)]),
        ]);
        normalize_grid(&mut t);
        let r0 = match &t.children[0] {
            RawChild::Block(b) => b,
            _ => panic!("row0 not block"),
        };
        assert_eq!(r0.children.len(), 3);
        let id1 = match &r0.children[1] {
            RawChild::Block(b) => b.id,
            _ => panic!(),
        };
        let id2 = match &r0.children[2] {
            RawChild::Block(b) => b.id,
            _ => panic!(),
        };
        assert_eq!(id1, synthetic_id(Dot::new(2, 10), 1, NodeType::TableCell));
        assert_eq!(id2, synthetic_id(Dot::new(2, 10), 2, NodeType::TableCell));
        assert!(id1.is_synthetic());
        assert!(id2.is_synthetic());
        assert_ne!(id1, id2);
    }

    #[test]
    fn normalize_grid_empty_table_noop() {
        let mut t = ttable(vec![]);
        normalize_grid(&mut t);
        assert!(t.children.is_empty());
    }

    fn root_with_table(rows: Vec<RawChild>) -> RawTree {
        RawTree {
            roots: vec![RawNode {
                attrs: vec![],
                id: Dot::new(2, 100),
                node_type: NodeType::Root,
                children: vec![RawChild::Block(RawNode {
                    attrs: vec![],
                    id: Dot::new(2, 0),
                    node_type: NodeType::Table,
                    children: rows,
                })],
            }],
        }
    }

    fn table_widths(tree: &RawTree) -> Vec<usize> {
        fn find_table(n: &RawNode) -> Option<&RawNode> {
            if n.node_type == NodeType::Table {
                return Some(n);
            }
            n.child_blocks().into_iter().find_map(find_table)
        }
        let table = tree.roots.iter().find_map(find_table).expect("table");
        table
            .children
            .iter()
            .filter_map(|c| match c {
                RawChild::Block(r) if r.node_type == NodeType::TableRow => Some(
                    r.children
                        .iter()
                        .filter(|cc| cc.as_child_type() == Some(NodeType::TableCell))
                        .count(),
                ),
                _ => None,
            })
            .collect()
    }

    fn grid_cell_ids(tree: &RawTree) -> Vec<Vec<Dot>> {
        fn find_table(n: &RawNode) -> Option<&RawNode> {
            if n.node_type == NodeType::Table {
                return Some(n);
            }
            n.child_blocks().into_iter().find_map(find_table)
        }
        let table = tree.roots.iter().find_map(find_table).expect("table");
        table
            .children
            .iter()
            .filter_map(|c| match c {
                RawChild::Block(r) if r.node_type == NodeType::TableRow => Some(
                    r.children
                        .iter()
                        .filter_map(|cc| match cc {
                            RawChild::Block(b) if b.node_type == NodeType::TableCell => Some(b.id),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn normalize_c2_column_plus_row_pads() {
        let t = root_with_table(vec![
            trow(10, vec![tcell(11), tcell(12), tcell(13)]),
            trow(20, vec![tcell(21), tcell(22), tcell(23)]),
            trow(30, vec![tcell(31), tcell(32)]),
        ]);
        let out = normalize(t);
        assert_eq!(table_widths(&out), vec![3, 3, 3]);
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_c4_irregular_paste_rectangular() {
        let t = root_with_table(vec![
            trow(10, vec![tcell(11)]),
            trow(20, vec![tcell(21), tcell(22), tcell(23), tcell(24)]),
            trow(30, vec![tcell(31), tcell(32)]),
        ]);
        let out = normalize(t);
        assert_eq!(table_widths(&out), vec![4, 4, 4]);
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_c6_empty_row_repaired_then_padded() {
        let t = root_with_table(vec![trow(10, vec![tcell(11), tcell(12)]), trow(20, vec![])]);
        let out = normalize(t);
        assert_eq!(table_widths(&out), vec![2, 2]);
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_c9_row_reorder_stays_rectangular() {
        let t = root_with_table(vec![
            trow(30, vec![tcell(31), tcell(32)]),
            trow(10, vec![tcell(11), tcell(12)]),
        ]);
        let out = normalize(t);
        assert_eq!(table_widths(&out), vec![2, 2]);
        assert_eq!(
            grid_cell_ids(&out),
            vec![
                vec![Dot::new(2, 31), Dot::new(2, 32)],
                vec![Dot::new(2, 11), Dot::new(2, 12)],
            ]
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_c7_empty_table_repaired() {
        let t = root_with_table(vec![]);
        let out = normalize(t);
        let widths = table_widths(&out);
        assert!(!widths.is_empty(), "repair 후 ≥1 행이어야");
        assert!(widths.iter().all(|&w| w >= 1), "각 행 ≥1 셀");
        let first = widths[0];
        assert!(widths.iter().all(|&w| w == first), "직사각형");
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_c3_misalign_limitation_grid_noop() {
        let t = root_with_table(vec![
            trow(10, vec![tcell(11), tcell(12), tcell(13), tcell(14)]),
            trow(20, vec![tcell(21), tcell(22), tcell(23), tcell(24)]),
        ]);
        let before = grid_cell_ids(&t);
        let out = normalize(t);
        assert_eq!(table_widths(&out), vec![4, 4]);
        assert_eq!(grid_cell_ids(&out), before);
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_preserves_unknown_leaf_in_paragraph() {
        let para = Dot::new(1, 1);
        let unknown = Dot::new(1, 2);
        let pb1 = Dot::new(1, 3);
        let pb2 = Dot::new(1, 4);
        let node = RawNode {
            attrs: vec![],
            id: para,
            node_type: NodeType::Paragraph,
            children: vec![
                RawChild::Leaf {
                    id: unknown,
                    item: super::super::SeqItem::Unknown {
                        tag: 999,
                        bytes: vec![0xAA],
                    },
                },
                RawChild::Leaf {
                    id: pb1,
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                },
                RawChild::Leaf {
                    id: pb2,
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                },
            ],
        };
        let out = normalize(RawTree { roots: vec![node] });
        assert!(
            matches!(
                find_leaf(&out, unknown),
                Some(super::super::SeqItem::Unknown { tag: 999, .. })
            ),
            "normalize가 unknown 리프를 드롭/변형하면 안 된다"
        );
        assert!(
            find_leaf(&out, pb1).is_some(),
            "매치되는 첫 PageBreak는 유지"
        );
        assert!(
            find_leaf(&out, pb2).is_some(),
            "잉여 PageBreak는 드롭이 아니라 SPLIT-HOIST로 Root 문단에 보존된다"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_preserves_unknown_leaf_in_table_row() {
        let unknown = Dot::new(2, 22);
        let t = root_with_table(vec![
            trow(10, vec![tcell(11), tcell(12), tcell(13)]),
            trow(
                20,
                vec![
                    tcell(21),
                    RawChild::Leaf {
                        id: unknown,
                        item: super::super::SeqItem::Unknown {
                            tag: 999,
                            bytes: vec![0xAA],
                        },
                    },
                    tcell(23),
                ],
            ),
        ]);
        let out = normalize(t);
        assert_eq!(
            table_widths(&out),
            vec![3, 3],
            "unknown은 셀로 집계되지 않는다"
        );
        fn find_table(n: &RawNode) -> Option<&RawNode> {
            if n.node_type == NodeType::Table {
                return Some(n);
            }
            n.child_blocks().into_iter().find_map(find_table)
        }
        let table = out.roots.iter().find_map(find_table).expect("table");
        let row20 = table
            .children
            .iter()
            .find_map(|c| match c {
                RawChild::Block(r) if r.id == Dot::new(2, 20) => Some(r),
                _ => None,
            })
            .expect("row 20");
        assert!(
            row20.children.iter().any(|c| matches!(
                c,
                RawChild::Leaf { id, item: super::super::SeqItem::Unknown { tag: 999, .. } } if *id == unknown
            )),
            "normalize_grid의 padding 경로가 unknown 리프를 건드리면 안 된다"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_preserves_unknown_leaf_through_promote() {
        let promoted_para = Dot::new(1, 2);
        let unknown = Dot::new(1, 9);
        let node = RawNode {
            attrs: vec![],
            id: Dot::ROOT,
            node_type: NodeType::Root,
            children: vec![
                RawChild::Block(RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 1),
                    node_type: NodeType::ListItem,
                    children: vec![RawChild::Block(RawNode {
                        attrs: vec![],
                        id: promoted_para,
                        node_type: NodeType::Paragraph,
                        children: vec![],
                    })],
                }),
                RawChild::Leaf {
                    id: unknown,
                    item: super::super::SeqItem::Unknown {
                        tag: 777,
                        bytes: vec![0xCC],
                    },
                },
            ],
        };
        let out = normalize(RawTree { roots: vec![node] });
        assert!(
            BlockTree::from_raw(&out).get(promoted_para).is_some(),
            "the ListItem's Paragraph survives (the ListItem is WRAPped in a BulletList)"
        );
        assert!(
            out.roots[0].children.iter().any(|c| matches!(
                c,
                RawChild::Leaf { id, item: super::super::SeqItem::Unknown { tag: 777, .. } } if *id == unknown
            )),
            "the stray Root Unknown leaf stays a direct Root child (transparent)"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_preserves_unknown_block_in_paragraph_unmatched_drop() {
        let para = Dot::new(1, 1);
        let unknown_block = Dot::new(1, 2);
        let unknown_child = Dot::new(1, 3);
        let pb1 = Dot::new(1, 4);
        let pb2 = Dot::new(1, 5);
        let node = RawNode {
            attrs: vec![],
            id: para,
            node_type: NodeType::Paragraph,
            children: vec![
                RawChild::Block(RawNode {
                    attrs: vec![],
                    id: unknown_block,
                    node_type: NodeType::Unknown,
                    children: vec![RawChild::Leaf {
                        id: unknown_child,
                        item: super::super::SeqItem::Char('x'),
                    }],
                }),
                RawChild::Leaf {
                    id: pb1,
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                },
                RawChild::Leaf {
                    id: pb2,
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                },
            ],
        };
        let out = normalize(RawTree { roots: vec![node] });
        // `fix_roots` wraps the bare Paragraph root under a synthesized Root.
        let para_out = out.roots[0]
            .children
            .iter()
            .find_map(|c| match c {
                RawChild::Block(b) if b.id == para => Some(b),
                _ => None,
            })
            .expect("paragraph must survive under the synthesized root");
        let unknown = para_out
            .children
            .iter()
            .find_map(|c| match c {
                RawChild::Block(b) if b.id == unknown_block => Some(b),
                _ => None,
            })
            .expect("unknown block must survive the unmatched-drop repair pass");
        assert_eq!(unknown.node_type, NodeType::Unknown);
        assert!(
            matches!(
                &unknown.children[0],
                RawChild::Leaf { id, item: super::super::SeqItem::Char('x') } if *id == unknown_child
            ),
            "unknown block's own child must attach normally, untouched"
        );
        assert!(
            para_out
                .children
                .iter()
                .any(|c| matches!(c, RawChild::Leaf { id, .. } if *id == pb1)),
            "matching first PageBreak kept"
        );
        assert!(
            find_leaf(&out, pb2).is_some(),
            "the surplus PageBreak is SPLIT-HOISTed to a Root Paragraph, not dropped"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_preserves_unknown_block_through_context_filter() {
        let unknown_block = Dot::new(1, 10);
        let unknown_child = Dot::new(1, 11);
        let pagebreak = Dot::new(1, 2);
        let tree = RawTree {
            roots: vec![RawNode {
                attrs: vec![],
                id: Dot::new(1, 0),
                node_type: NodeType::Blockquote,
                children: vec![
                    RawChild::Block(RawNode {
                        attrs: vec![],
                        id: Dot::new(1, 1),
                        node_type: NodeType::Paragraph,
                        children: vec![RawChild::Leaf {
                            id: pagebreak,
                            item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                        }],
                    }),
                    RawChild::Block(RawNode {
                        attrs: vec![],
                        id: unknown_block,
                        node_type: NodeType::Unknown,
                        children: vec![RawChild::Leaf {
                            id: unknown_child,
                            item: super::super::SeqItem::Char('u'),
                        }],
                    }),
                ],
            }],
        };
        let out = normalize(tree);

        fn find(n: &RawNode, id: Dot) -> Option<&RawNode> {
            if n.id == id {
                return Some(n);
            }
            n.child_blocks().into_iter().find_map(|c| find(c, id))
        }
        fn has_pagebreak(n: &RawNode) -> bool {
            n.children.iter().any(|c| match c {
                RawChild::Leaf { item, .. } => item.as_child_type() == Some(NodeType::PageBreak),
                RawChild::Block(b) => has_pagebreak(b),
            })
        }

        let root = &out.roots[0];
        let unknown =
            find(root, unknown_block).expect("unknown block must survive context filtering");
        assert!(
            matches!(
                &unknown.children[0],
                RawChild::Leaf { id, item: super::super::SeqItem::Char('u') } if *id == unknown_child
            ),
            "unknown block's own child must attach normally, untouched"
        );
        assert!(
            has_pagebreak(root),
            "the context-invalid PageBreak (Blockquote>Paragraph) is SPLIT-HOISTed to a \
             Root-level Paragraph and preserved, not dropped"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_preserves_unknown_block_through_promote() {
        let promoted_para = Dot::new(1, 2);
        let unknown_block = Dot::new(1, 9);
        let unknown_child = Dot::new(1, 10);
        let node = RawNode {
            attrs: vec![],
            id: Dot::ROOT,
            node_type: NodeType::Root,
            children: vec![
                RawChild::Block(RawNode {
                    attrs: vec![],
                    id: Dot::new(1, 1),
                    node_type: NodeType::ListItem,
                    children: vec![RawChild::Block(RawNode {
                        attrs: vec![],
                        id: promoted_para,
                        node_type: NodeType::Paragraph,
                        children: vec![],
                    })],
                }),
                RawChild::Block(RawNode {
                    attrs: vec![],
                    id: unknown_block,
                    node_type: NodeType::Unknown,
                    children: vec![RawChild::Leaf {
                        id: unknown_child,
                        item: super::super::SeqItem::Char('z'),
                    }],
                }),
            ],
        };
        let out = normalize(RawTree { roots: vec![node] });
        assert!(
            BlockTree::from_raw(&out).get(promoted_para).is_some(),
            "the ListItem's Paragraph survives (the ListItem is WRAPped in a BulletList)"
        );
        let unknown = out.roots[0]
            .children
            .iter()
            .find_map(|c| match c {
                RawChild::Block(b) if b.id == unknown_block => Some(b),
                _ => None,
            })
            .expect("unknown block must survive the promote cascade");
        assert!(
            matches!(
                &unknown.children[0],
                RawChild::Leaf { id, item: super::super::SeqItem::Char('z') } if *id == unknown_child
            ),
            "unknown block's own child must attach normally, untouched"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_preserves_atom_leaf_unknown_bearing_leaf() {
        let para = Dot::new(1, 1);
        let block_atom_unknown = Dot::new(1, 2);
        let pb1 = Dot::new(1, 3);
        let pb2 = Dot::new(1, 4);
        let node = RawNode {
            attrs: vec![],
            id: para,
            node_type: NodeType::Paragraph,
            children: vec![
                RawChild::Leaf {
                    id: block_atom_unknown,
                    item: super::super::SeqItem::BlockAtom {
                        leaf: AtomLeaf::Unknown(crate::nodes::UnknownNode),
                        parents: vec![para],
                    },
                },
                RawChild::Leaf {
                    id: pb1,
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                },
                RawChild::Leaf {
                    id: pb2,
                    item: super::super::SeqItem::Atom(AtomLeaf::PageBreak),
                },
            ],
        };
        let out = normalize(RawTree { roots: vec![node] });
        assert!(
            matches!(
                find_leaf(&out, block_atom_unknown),
                Some(super::super::SeqItem::BlockAtom {
                    leaf: AtomLeaf::Unknown(_),
                    ..
                })
            ),
            "an AtomLeaf::Unknown-bearing leaf must survive normalize unmodified"
        );
        assert!(
            find_leaf(&out, pb1).is_some(),
            "matching first PageBreak kept"
        );
        assert!(
            find_leaf(&out, pb2).is_some(),
            "the surplus PageBreak is SPLIT-HOISTed to a Root Paragraph, not dropped"
        );
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn normalize_preserves_unknown_wrapped_table_nested_in_table() {
        let unknown_block = Dot::new(1, 1);
        let inner_table = Dot::new(1, 2);
        let node = RawNode {
            attrs: vec![],
            id: Dot::new(1, 0),
            node_type: NodeType::Table,
            children: vec![RawChild::Block(RawNode {
                attrs: vec![],
                id: unknown_block,
                node_type: NodeType::Unknown,
                children: vec![RawChild::Block(RawNode {
                    attrs: vec![],
                    id: inner_table,
                    node_type: NodeType::Table,
                    children: vec![],
                })],
            })],
        };
        let out = normalize(RawTree { roots: vec![node] });

        fn find(n: &RawNode, id: Dot) -> Option<&RawNode> {
            if n.id == id {
                return Some(n);
            }
            n.child_blocks().into_iter().find_map(|c| find(c, id))
        }

        let root = &out.roots[0];
        let unknown = find(root, unknown_block).expect("unknown block must survive normalize");
        assert_eq!(unknown.node_type, NodeType::Unknown);
        let inner = find(unknown, inner_table).expect("nested table inside unknown must survive");
        assert_eq!(inner.node_type, NodeType::Table);
        assert!(valid(&out).is_ok());
    }

    #[test]
    fn project_document_keeps_unknown_leaf_as_one_slot() {
        use crate::projection::{DocLogs, project_document};
        use crate::{AliasLog, ModifierAttrLog, NodeAttrLog, SpanLog};
        use editor_crdt::{InputEvent, ListOp, build_oplog};

        let para = Dot::new(1, 1);
        let unknown = Dot::new(1, 2);
        let ch = Dot::new(1, 3);
        let items = [
            (
                para,
                super::super::SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            ),
            (
                unknown,
                super::super::SeqItem::Unknown {
                    tag: 999,
                    bytes: vec![0xAA],
                },
            ),
            (ch, super::super::SeqItem::Char('a')),
        ];
        let mut ev = Vec::new();
        let mut prev: Option<Dot> = None;
        for (i, (id, item)) in items.iter().enumerate() {
            ev.push(InputEvent {
                id: *id,
                parents: prev.into_iter().collect(),
                op: ListOp::Ins {
                    pos: i,
                    item: item.clone(),
                },
            });
            prev = Some(*id);
        }
        let logs = DocLogs {
            seq: build_oplog(&ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_carries: ModifierAttrLog::new(),
            aliases: AliasLog::new(),
        };
        let pd = project_document(&logs).unwrap();
        let p = pd.tree.get(para).expect("paragraph present");
        assert_eq!(
            p.children.len(),
            2,
            "unknown 리프가 투영 문서에서 1 슬롯을 점유해야 한다"
        );
        assert!(matches!(
            &p.children[0],
            crate::seq::Child::Leaf { id, item: super::super::SeqItem::Unknown { tag: 999, .. } } if *id == unknown
        ));
    }

    #[test]
    fn normalize_window_forest_for_root_equivalent_normal_paragraphs() {
        let children = vec![
            raw_block_child(1, NodeType::Paragraph, vec![raw_char(2, 'a')]),
            raw_block_child(3, NodeType::Paragraph, vec![raw_char(4, 'b')]),
        ];
        let mut stats_root = RepairStats::default();
        let forest_root = normalize_window_forest_with_stats(children.clone(), &mut stats_root);
        let mut stats_container = RepairStats::default();
        let (forest_container, hoisted) = normalize_window_forest_for(
            Dot::ROOT,
            NodeType::Root,
            &[],
            children,
            &mut stats_container,
        );
        assert_eq!(forest_root, forest_container);
        assert!(hoisted.is_empty());
        assert_eq!(stats_root, stats_container);
    }

    #[test]
    fn normalize_window_forest_for_root_equivalent_bare_list_item() {
        let children = vec![raw_block_child(
            1,
            NodeType::ListItem,
            vec![raw_block_child(2, NodeType::Paragraph, vec![])],
        )];
        let mut stats_root = RepairStats::default();
        let forest_root = normalize_window_forest_with_stats(children.clone(), &mut stats_root);
        let mut stats_container = RepairStats::default();
        let (forest_container, hoisted) = normalize_window_forest_for(
            Dot::ROOT,
            NodeType::Root,
            &[],
            children,
            &mut stats_container,
        );
        assert_eq!(forest_root, forest_container);
        assert!(hoisted.is_empty());
        assert_eq!(stats_root, stats_container);
    }

    #[test]
    fn normalize_window_forest_for_container_terminal_split_hoists_and_forest_truncates() {
        let container_id = Dot::new(1, 2);
        let first_para = Dot::new(1, 3);
        let second_para = Dot::new(1, 5);
        let children = vec![
            raw_block_child(3, NodeType::Paragraph, vec![raw_char(4, 'a')]),
            raw_block_child(5, NodeType::Paragraph, vec![raw_char(6, 'b')]),
        ];
        let mut stats = RepairStats::default();
        let (forest, hoisted) = normalize_window_forest_for(
            container_id,
            NodeType::ListItem,
            &[NodeType::Root, NodeType::BulletList],
            children,
            &mut stats,
        );
        assert_eq!(forest.len(), 1);
        assert!(matches!(&forest[0], RawChild::Block(b) if b.id == first_para));
        assert_eq!(hoisted.len(), 1);
        assert!(matches!(&hoisted[0], RawChild::Block(b) if b.id == second_para));
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;
        use strum::IntoEnumIterator;

        #[derive(Clone, Debug)]
        enum Shape {
            Leaf(super::super::super::SeqItem),
            Block {
                node_type: NodeType,
                children: Vec<Shape>,
            },
        }

        fn arb_leaf() -> impl Strategy<Value = Shape> {
            prop_oneof![
                any::<char>().prop_map(|c| Shape::Leaf(super::super::super::SeqItem::Char(c))),
                Just(Shape::Leaf(super::super::super::SeqItem::Atom(
                    AtomLeaf::HardBreak
                ))),
                Just(Shape::Leaf(super::super::super::SeqItem::Atom(
                    AtomLeaf::Tab
                ))),
                Just(Shape::Leaf(super::super::super::SeqItem::Atom(
                    AtomLeaf::PageBreak
                ))),
            ]
        }

        fn arb_block_type() -> impl Strategy<Value = NodeType> {
            // Root is an encoding error rejected at the projection boundary
            // (`RootTypedBlock`) and is deliberately unplaceable (excluded from the
            // completeness meta test), so normalize never receives a Root-typed
            // block; leaf atom types are never block markers either.
            let types: Vec<NodeType> = NodeType::iter()
                .filter(|t| {
                    !matches!(
                        t,
                        NodeType::HardBreak | NodeType::Tab | NodeType::PageBreak | NodeType::Root
                    )
                })
                .collect();
            prop::sample::select(types)
        }

        fn arb_block(depth: u32) -> impl Strategy<Value = Shape> {
            arb_leaf().prop_recursive(depth, 64, 4, move |inner| {
                (arb_block_type(), prop::collection::vec(inner, 0..4)).prop_map(
                    |(node_type, children)| Shape::Block {
                        node_type,
                        children,
                    },
                )
            })
        }

        fn arb_any_block_tree(depth: u32) -> impl Strategy<Value = RawTree> {
            let block = arb_block(depth)
                .prop_filter("roots는 블록만", |s| matches!(s, Shape::Block { .. }));
            prop::collection::vec(block, 0..4).prop_map(|roots| {
                let mut next = 0u64;
                RawTree {
                    roots: roots.iter().map(|s| build(s, &mut next)).collect(),
                }
            })
        }

        fn build(s: &Shape, next: &mut u64) -> RawNode {
            let id = Dot::new(1, *next);
            *next += 1;
            match s {
                Shape::Block {
                    node_type,
                    children,
                } => RawNode {
                    attrs: vec![],
                    id,
                    node_type: *node_type,
                    children: children.iter().map(|c| build_child(c, next)).collect(),
                },
                Shape::Leaf(_) => unreachable!("roots/children filtered/handled elsewhere"),
            }
        }

        fn build_child(s: &Shape, next: &mut u64) -> RawChild {
            match s {
                Shape::Leaf(item) => {
                    let id = Dot::new(1, *next);
                    *next += 1;
                    RawChild::Leaf {
                        id,
                        item: item.clone(),
                    }
                }
                Shape::Block { .. } => RawChild::Block(build(s, next)),
            }
        }

        proptest! {
            #[test]
            fn normalize_makes_any_tree_valid(tree in arb_any_block_tree(6)) {
                let before = raw_real_dots(&tree);
                let a = normalize(tree.clone());
                prop_assert!(valid(&a).is_ok());
                prop_assert_eq!(raw_real_dots(&a), before, "totality: real dot set preserved by repair");
                prop_assert_eq!(normalize(tree), a.clone());
                prop_assert_eq!(normalize(a.clone()), a);
            }
        }

        fn tcell(i: u64) -> RawChild {
            RawChild::Block(RawNode {
                attrs: vec![],
                id: Dot::new(2, i),
                node_type: NodeType::TableCell,
                children: vec![],
            })
        }
        fn trow(id: u64, cells: Vec<RawChild>) -> RawChild {
            RawChild::Block(RawNode {
                attrs: vec![],
                id: Dot::new(2, id),
                node_type: NodeType::TableRow,
                children: cells,
            })
        }
        fn root_with_table(rows: Vec<RawChild>) -> RawTree {
            RawTree {
                roots: vec![RawNode {
                    attrs: vec![],
                    id: Dot::new(2, 100),
                    node_type: NodeType::Root,
                    children: vec![RawChild::Block(RawNode {
                        attrs: vec![],
                        id: Dot::new(2, 0),
                        node_type: NodeType::Table,
                        children: rows,
                    })],
                }],
            }
        }
        fn table_widths(tree: &RawTree) -> Vec<usize> {
            fn find_table(n: &RawNode) -> Option<&RawNode> {
                if n.node_type == NodeType::Table {
                    return Some(n);
                }
                n.child_blocks().into_iter().find_map(find_table)
            }
            let table = tree.roots.iter().find_map(find_table).expect("table");
            table
                .children
                .iter()
                .filter_map(|c| match c {
                    RawChild::Block(r) if r.node_type == NodeType::TableRow => Some(
                        r.children
                            .iter()
                            .filter(|cc| cc.as_child_type() == Some(NodeType::TableCell))
                            .count(),
                    ),
                    _ => None,
                })
                .collect()
        }

        fn arb_table() -> impl Strategy<Value = RawTree> {
            prop::collection::vec(0usize..5, 1..5).prop_map(|row_sizes| {
                let mut next = 10u64;
                let rows: Vec<RawChild> = row_sizes
                    .into_iter()
                    .map(|n| {
                        let row_id = next;
                        next += 1;
                        let cells: Vec<RawChild> = (0..n)
                            .map(|_| {
                                let c = tcell(next);
                                next += 1;
                                c
                            })
                            .collect();
                        trow(row_id, cells)
                    })
                    .collect();
                root_with_table(rows)
            })
        }

        proptest! {
            #[test]
            fn normalize_table_is_rectangular(t in arb_table()) {
                let out = normalize(t);
                let widths = table_widths(&out);
                if let Some(&first) = widths.first() {
                    prop_assert!(widths.iter().all(|&w| w == first), "ragged: {widths:?}");
                }
                prop_assert!(valid(&out).is_ok());
            }

            #[test]
            fn normalize_table_idempotent(t in arb_table()) {
                let once = normalize(t);
                let twice = normalize(once.clone());
                prop_assert_eq!(once, twice);
            }

            #[test]
            fn normalize_width_matches_max_reference(row_sizes in prop::collection::vec(0usize..5, 1..5)) {
                let mut next = 10u64;
                let rows: Vec<RawChild> = row_sizes
                    .iter()
                    .map(|&n| {
                        let row_id = next;
                        next += 1;
                        let cells: Vec<RawChild> = (0..n).map(|_| { let c = tcell(next); next += 1; c }).collect();
                        trow(row_id, cells)
                    })
                    .collect();
                let out = normalize(root_with_table(rows));
                let widths = table_widths(&out);
                let reference = row_sizes.iter().map(|&n| n.max(1)).max().unwrap_or(1);
                prop_assert!(widths.iter().all(|&w| w == reference), "widths {widths:?} != ref {reference}");
            }
        }
    }
}
