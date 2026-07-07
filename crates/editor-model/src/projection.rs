use std::collections::{BTreeMap, HashMap};

use editor_crdt::Dot;
use editor_crdt::OpLog;
use editor_crdt::sequence::{SeqCheckout, SeqResolve, checkout_with_resolver};

use crate::{
    BlockNode, BlockTree, Child, ChildList, Modifier, ModifierAttrLog, ModifierType, NodeType,
    OwnModifier, ProjectError, SchemaError, anchor_dot,
};
use crate::{Node, NodeAttrLog, SeqItem, SpanLog, normalize, project_blocks, validate_block_tree};

#[derive(Debug)]
pub enum ProjectionError {
    Project(ProjectError),
    LeafTypedBlock { dot: Dot, node_type: NodeType },
    SchemaInvalid(SchemaError),
}

#[derive(Clone, Debug)]
pub struct DocLogs {
    pub seq: OpLog<SeqItem>,
    pub spans: SpanLog,
    pub block_modifiers: ModifierAttrLog,
    pub node_attrs: NodeAttrLog,
    pub node_carries: ModifierAttrLog,
}

/// A leaf's effective-modifier map, shared by reference: every leaf of a
/// uniform run (and the run segment itself) points at one allocation, so a
/// whole-range styling clones/compares Arcs instead of BTreeMaps and dropping
/// a superseded projection frees O(runs) maps, not O(leaves).
pub type LeafEff = std::sync::Arc<BTreeMap<ModifierType, Modifier>>;
/// A leaf's own-modifier map, shared like [`LeafEff`].
pub type LeafOwn = std::sync::Arc<BTreeMap<ModifierType, OwnModifier>>;

#[derive(Clone, Debug)]
pub struct ProjectedDoc {
    pub tree: BlockTree,
    pub block_effective: imbl::HashMap<Dot, BTreeMap<ModifierType, Modifier>>,
    /// Run segments per block: the sole store of per-leaf derived
    /// `effective` / `own` modifier state, coalesced by `(leaf_type, covering)`.
    pub seg_index: crate::span::BlockSegs,
    pub block_modifiers: imbl::HashMap<Dot, BTreeMap<ModifierType, Modifier>>,
    pub node_attrs: imbl::HashMap<Dot, Node>,
    pub node_carries: imbl::HashMap<Dot, BTreeMap<ModifierType, Modifier>>,
}

impl ProjectedDoc {
    /// Insert `leaf` into `block` at leaf `offset`. Only the tree is updated here;
    /// the segment index is maintained by the caller (editor-state).
    pub fn splice_char(&mut self, block: Dot, offset: usize, leaf: Dot, item: SeqItem) {
        insert_leaf_at(&mut self.tree, block, offset, leaf, &item);
    }

    pub fn splice_delete_leaf(&mut self, block: Dot, leaf: Dot) -> bool {
        if remove_leaf_from_block(&mut self.tree, block, leaf).is_none() {
            return false;
        }
        self.node_attrs.remove(&leaf);
        self.node_carries.remove(&leaf);
        true
    }

    /// Carry modifiers of `block` a caret/insert/aggregate consumer may read:
    /// per-type LWW winners restricted to the carryable kinds and valid values.
    /// The sole sanctioned read of `node_carries` for interpretation; structural
    /// reproduction (revert/undo) reads the log directly.
    pub fn carry_modifiers(&self, block: Dot) -> BTreeMap<ModifierType, Modifier> {
        self.node_carries
            .get(&block)
            .map(|m| {
                m.iter()
                    .filter(|(ty, v)| ty.is_carry_kind() && v.is_valid())
                    .map(|(ty, v)| (*ty, v.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    // `derive_block_effective` emits an entry for every block (empty or not), so
    // always insert; `collect_block_modifiers` only keeps non-empty blocks.
    pub fn set_block_effective(&mut self, block: Dot, be: BTreeMap<ModifierType, Modifier>) {
        self.block_effective.insert(block, be);
    }

    pub fn set_block_own_modifiers(&mut self, block: Dot, m: BTreeMap<ModifierType, Modifier>) {
        if m.is_empty() {
            self.block_modifiers.remove(&block);
        } else {
            self.block_modifiers.insert(block, m);
        }
    }
}

impl PartialEq for ProjectedDoc {
    /// Compare the tree, block-level maps, and the authoritative `seg_index` by value.
    fn eq(&self, o: &Self) -> bool {
        self.tree == o.tree
            && self.block_effective == o.block_effective
            && self.block_modifiers == o.block_modifiers
            && self.node_attrs == o.node_attrs
            && self.node_carries == o.node_carries
            && self.seg_index == o.seg_index
    }
}

fn collect_real_nodes(tree: &BlockTree) -> HashMap<Dot, Node> {
    fn walk(tree: &BlockTree, node: &BlockNode, out: &mut HashMap<Dot, Node>) {
        if let Some(d) = anchor_dot(node.id) {
            out.insert(d, node.node_type.into_node());
        }
        for c in &node.children {
            match c {
                Child::Leaf { id, item } => {
                    let node = match item {
                        SeqItem::Atom(atom) => atom.clone().into_node(),
                        _ => item.as_child_type().into_node(),
                    };
                    out.insert(*id, node);
                }
                Child::Block(id) => {
                    if let Some(b) = tree.get(*id) {
                        walk(tree, b, out);
                    }
                }
            }
        }
    }
    let mut out = HashMap::new();
    if let Some(r) = tree.root_node() {
        walk(tree, r, &mut out);
    }
    out
}

fn collect_block_modifiers(
    tree: &BlockTree,
    log: &ModifierAttrLog,
) -> imbl::HashMap<Dot, BTreeMap<ModifierType, Modifier>> {
    fn walk(
        tree: &BlockTree,
        node: &BlockNode,
        log: &ModifierAttrLog,
        out: &mut imbl::HashMap<Dot, BTreeMap<ModifierType, Modifier>>,
    ) {
        if let Some(d) = anchor_dot(node.id) {
            let m = log.modifiers_of(d);
            if !m.is_empty() {
                out.insert(d, m);
            }
        }
        for c in &node.children {
            if let Child::Block(id) = c
                && let Some(b) = tree.get(*id)
            {
                walk(tree, b, log, out);
            }
        }
    }
    let mut out = imbl::HashMap::new();
    if let Some(r) = tree.root_node() {
        walk(tree, r, log, &mut out);
    }
    out
}

/// Reflect live carry records onto their target blocks. Two kinds are silently
/// dropped here — the projection is the final guard against carry inflow — so
/// they never reach interpretation regardless of how the log op arrived: records
/// of a non-carry kind, and records whose target is not a text block.
fn collect_node_carries(
    tree: &BlockTree,
    log: &ModifierAttrLog,
) -> imbl::HashMap<Dot, BTreeMap<ModifierType, Modifier>> {
    fn walk(
        tree: &BlockTree,
        node: &BlockNode,
        log: &ModifierAttrLog,
        out: &mut imbl::HashMap<Dot, BTreeMap<ModifierType, Modifier>>,
    ) {
        if let Some(d) = anchor_dot(node.id)
            && node.node_type.spec().is_textblock()
        {
            let m: BTreeMap<ModifierType, Modifier> = log
                .modifiers_of(d)
                .into_iter()
                .filter(|(ty, _)| ty.is_carry_kind())
                .collect();
            if !m.is_empty() {
                out.insert(d, m);
            }
        }
        for c in &node.children {
            if let Child::Block(id) = c
                && let Some(b) = tree.get(*id)
            {
                walk(tree, b, log, out);
            }
        }
    }
    let mut out = imbl::HashMap::new();
    if let Some(r) = tree.root_node() {
        walk(tree, r, log, &mut out);
    }
    out
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockPaths {
    parent: imbl::HashMap<Dot, Dot>,
    block_of_leaf: imbl::HashMap<Dot, Dot>,
    node_type: imbl::HashMap<Dot, NodeType>,
}

impl BlockPaths {
    pub fn from_tree(tree: &BlockTree) -> Self {
        fn walk(
            tree: &BlockTree,
            node: &BlockNode,
            parent: &mut imbl::HashMap<Dot, Dot>,
            bol: &mut imbl::HashMap<Dot, Dot>,
            nt: &mut imbl::HashMap<Dot, NodeType>,
        ) {
            nt.insert(node.id, node.node_type);
            for c in &node.children {
                match c {
                    Child::Block(id) => {
                        parent.insert(*id, node.id);
                        if let Some(b) = tree.get(*id) {
                            walk(tree, b, parent, bol, nt);
                        }
                    }
                    Child::Leaf { id, .. } => {
                        bol.insert(*id, node.id);
                    }
                }
            }
        }
        let mut parent = imbl::HashMap::new();
        let mut block_of_leaf = imbl::HashMap::new();
        let mut node_type = imbl::HashMap::new();
        if let Some(root) = tree.root_node() {
            walk(tree, root, &mut parent, &mut block_of_leaf, &mut node_type);
        }
        Self {
            parent,
            block_of_leaf,
            node_type,
        }
    }

    /// The ancestor block chain (root-first, inclusive of `block`) as
    /// `(NodeType, anchor_dot)` pairs — the `block_path` shape `resolve_effective`
    /// expects for a leaf whose parent block is `block`.
    pub fn block_path_of(&self, block: Dot) -> Vec<(NodeType, Option<Dot>)> {
        let mut chain = self.path_of(block);
        chain.reverse();
        chain
            .into_iter()
            .filter_map(|d| self.node_type.get(&d).map(|t| (*t, anchor_dot(d))))
            .collect()
    }

    pub fn block_of(&self, leaf: Dot) -> Option<Dot> {
        self.block_of_leaf.get(&leaf).copied()
    }

    /// Whether `dot` is a live node (leaf or block) in the current tree.
    pub fn contains(&self, dot: Dot) -> bool {
        self.block_of_leaf.contains_key(&dot) || self.node_type.contains_key(&dot)
    }

    pub fn node_type_of(&self, node: Dot) -> Option<NodeType> {
        self.node_type.get(&node).copied()
    }

    pub fn parent_of(&self, node: Dot) -> Option<Dot> {
        self.parent.get(&node).copied()
    }

    pub fn add_block(&mut self, block: Dot, parent: Dot, node_type: NodeType) {
        self.parent.insert(block, parent);
        self.node_type.insert(block, node_type);
    }

    pub fn remove_block(&mut self, block: Dot) {
        self.parent.remove(&block);
        self.node_type.remove(&block);
    }

    pub fn set_block_of_leaf(&mut self, leaf: Dot, block: Dot) {
        self.block_of_leaf.insert(leaf, block);
    }

    pub fn remove_leaf(&mut self, leaf: Dot) {
        self.block_of_leaf.remove(&leaf);
    }

    pub fn path_of(&self, node: Dot) -> Vec<Dot> {
        let mut out = vec![node];
        let mut cur = node;
        loop {
            let next = self
                .parent
                .get(&cur)
                .copied()
                .or_else(|| self.block_of_leaf.get(&cur).copied());
            match next {
                Some(p) if p != cur => {
                    out.push(p);
                    cur = p;
                }
                _ => break,
            }
        }
        out
    }

    pub fn descendants_of(&self, block: Dot) -> Vec<Dot> {
        let mut children: HashMap<Dot, Vec<Dot>> = HashMap::new();
        for (&child, &par) in &self.parent {
            children.entry(par).or_default().push(child);
        }
        for (&leaf, &par) in &self.block_of_leaf {
            children.entry(par).or_default().push(leaf);
        }
        let mut out = Vec::new();
        let mut stack = vec![block];
        while let Some(n) = stack.pop() {
            if let Some(cs) = children.get(&n) {
                for &c in cs {
                    out.push(c);
                    stack.push(c);
                }
            }
        }
        out
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProjectionIndexes {
    pub paths: BlockPaths,
    pub spans: crate::span::SpanAnchorIndex,
}

impl ProjectionIndexes {
    pub fn rebuild_from(projected: &ProjectedDoc, spans: &SpanLog) -> Self {
        Self {
            paths: BlockPaths::from_tree(&projected.tree),
            spans: crate::span::SpanAnchorIndex::build(spans),
        }
    }
}

/// Derive a segment's `(effective, own)` from its key directly. The canonical
/// `covering` (per-type LWW winner) resolves the same explicit effects a full covering
/// set would. `attr_leaf`
/// is `Some(dot)` only for attrs-singleton segments, threading the real leaf dot so
/// per-leaf `node_attrs` (and block modifiers on block-level atoms) resolve naturally;
/// otherwise a sentinel dot keyed to this segment's synthetic inputs stands in.
pub fn derive_seg_state(
    paths: &BlockPaths,
    logs: &DocLogs,
    projected: &ProjectedDoc,
    block: Dot,
    leaf_type: NodeType,
    covering: Option<&crate::span::Covering>,
    attr_leaf: Option<Dot>,
) -> (LeafEff, LeafOwn) {
    let block_path = paths.block_path_of(block);
    let mut leaf_path: Vec<NodeType> = block_path.iter().map(|(t, _)| *t).collect();
    leaf_path.push(leaf_type);
    let explicit = covering
        .map(|c| crate::span::explicit_from_winners(c, &logs.spans, &leaf_path))
        .unwrap_or_default();
    let sentinel = attr_leaf.unwrap_or(Dot::new(u64::MAX, u64::MAX));
    let mut ex_map: HashMap<Dot, BTreeMap<ModifierType, Modifier>> = HashMap::new();
    ex_map.insert(sentinel, explicit);
    let src = crate::span::EffectiveSources {
        block_modifiers: &logs.block_modifiers,
        explicit_spans: &ex_map,
        node_attrs: &projected.node_attrs,
    };
    let eff = crate::span::resolve_effective(&block_path, Some(sentinel), leaf_type, true, &src);
    let own = crate::span::own_modifiers_for_leaf(sentinel, &src);
    (LeafEff::new(eff), LeafOwn::new(own))
}

/// Fold a position's stabbing span op dots into the canonical per-type LWW winner.
/// `None` when nothing covers the position.
fn canonical_covering(dots: &[Dot], spans: &SpanLog) -> Option<crate::span::SegCovering> {
    let mut cov: Option<crate::span::SegCovering> = None;
    for &d in dots {
        if let Some(op) = spans.get(d)
            && let Some(next) =
                crate::span::covering_absorb(cov.as_ref(), crate::span::covering_of_op(op), d)
        {
            cov = Some(next);
        }
    }
    cov
}

/// Cold segmentation of one block: walk its child leaves in document order and
/// coalesce consecutive leaves that agree on `(leaf_type, covering)` and are
/// not attrs-singletons. `covering_for` yields the span op dots stabbing a leaf's
/// visible position (from `ResolvedSpans`). `derive_seg_state` runs once per
/// distinct key via an LRU-1 memo.
pub fn segment_block(
    paths: &BlockPaths,
    logs: &DocLogs,
    projected: &ProjectedDoc,
    block: &BlockNode,
    covering_for: impl Fn(Dot) -> Vec<Dot>,
) -> Vec<crate::span::Seg> {
    struct SegCache {
        leaf_type: NodeType,
        covering: Option<crate::span::SegCovering>,
        eff: LeafEff,
        own: LeafOwn,
    }
    let mut out: Vec<crate::span::Seg> = Vec::new();
    let mut cache: Option<SegCache> = None;
    for c in &block.children {
        let Child::Leaf { id, item } = c else {
            continue;
        };
        let dot = *id;
        let leaf_type = item.as_child_type();
        let covering = canonical_covering(&covering_for(dot), &logs.spans);
        // Per-leaf-input singleton: attrs carriers, plus every non-inline leaf —
        // `own_effect` reads `block_modifiers` through the REAL dot for those, which
        // the sentinel can't stand in for.
        let attrs_singleton =
            projected.node_attrs.contains_key(&dot) || !crate::Schema::node_spec(leaf_type).inline;

        let (eff, own) = if attrs_singleton {
            derive_seg_state(
                paths,
                logs,
                projected,
                block.id,
                leaf_type,
                covering.as_deref(),
                Some(dot),
            )
        } else {
            let reuse = match &cache {
                Some(k) if k.leaf_type == leaf_type && k.covering == covering => {
                    Some((k.eff.clone(), k.own.clone()))
                }
                _ => None,
            };
            match reuse {
                Some(v) => v,
                None => {
                    let d = derive_seg_state(
                        paths,
                        logs,
                        projected,
                        block.id,
                        leaf_type,
                        covering.as_deref(),
                        None,
                    );
                    cache = Some(SegCache {
                        leaf_type,
                        covering: covering.clone(),
                        eff: d.0.clone(),
                        own: d.1.clone(),
                    });
                    d
                }
            }
        };

        let seg = crate::span::Seg {
            count: 1,
            leaf_type,
            covering,
            attrs_singleton,
            eff,
            own,
        };
        match out.last_mut() {
            Some(last) if last.key_eq(&seg) => last.count += 1,
            _ => out.push(seg),
        }
    }
    out
}

/// Segment every block of a freshly projected tree, given a per-leaf covering
/// source. The cold companion to the old per-leaf maps (both built during a
/// projection; the maps stay authoritative until later tasks).
fn build_seg_index(
    pd: &ProjectedDoc,
    paths: &BlockPaths,
    logs: &DocLogs,
    covering_for: impl Fn(Dot) -> Vec<Dot>,
) -> crate::span::BlockSegs {
    let mut segs = crate::span::BlockSegs::new();
    let mut stack: Vec<Dot> = pd.tree.root_node().map(|r| r.id).into_iter().collect();
    while let Some(bid) = stack.pop() {
        let Some(node) = pd.tree.get(bid) else {
            continue;
        };
        let block_segs = segment_block(paths, logs, pd, node, &covering_for);
        segs.set_block(bid, block_segs);
        for c in &node.children {
            if let Child::Block(id) = c {
                stack.push(*id);
            }
        }
    }
    segs
}

/// Resolve a single block's `block_effective` (inheritance + its own modifiers),
/// matching `derive_block_effective` for that block. O(depth).
pub fn block_effective_one(
    paths: &BlockPaths,
    logs: &DocLogs,
    projected: &ProjectedDoc,
    block: Dot,
) -> BTreeMap<ModifierType, Modifier> {
    let full_path = paths.block_path_of(block);
    let Some((self_type, self_dot)) = full_path.last().copied() else {
        return BTreeMap::new();
    };
    let ancestors = &full_path[..full_path.len() - 1];
    let empty: HashMap<Dot, BTreeMap<ModifierType, Modifier>> = HashMap::new();
    let src = crate::span::EffectiveSources {
        block_modifiers: &logs.block_modifiers,
        explicit_spans: &empty,
        node_attrs: &projected.node_attrs,
    };
    crate::span::resolve_effective(ancestors, self_dot, self_type, false, &src)
}

/// Find `block` and run `f` on its children, returning whether it was found.
/// `O(log N)` — a single `Dot`-keyed lookup, no descent.
fn with_block_children(tree: &mut BlockTree, block: Dot, f: impl FnOnce(&mut ChildList)) -> bool {
    tree.with_block_children(block, f)
}

/// Split a leaf-only block `p_block` (a child of `parent`) at the point just
/// after `split_after` (or at its start when `None`), moving the tail leaves
/// into a fresh `new_block` of `new_type` inserted right after `p_block`.
/// Returns the moved leaf dots, or `None` if the shape isn't a simple leaf-only
/// split (caller must then fall back). O(block size).
pub fn split_block_insert(
    tree: &mut BlockTree,
    parent: Dot,
    p_block: Dot,
    split_after: Option<Dot>,
    new_block: Dot,
    new_type: NodeType,
) -> Option<Vec<Dot>> {
    // Validate everything (block shapes, split point, that `p_block` is a child of
    // `parent`) BEFORE mutating, so any bail-out leaves the tree untouched — `p_block`
    // and `new_block` are addressed directly in `nodes`, no descent.
    let p_node = tree.get(p_block)?;
    if p_node.children.iter().any(|c| matches!(c, Child::Block(_))) {
        return None;
    }
    let split_at = match split_after {
        None => 0,
        Some(leaf) => {
            p_node
                .children
                .iter()
                .position(|c| matches!(c, Child::Leaf { id, .. } if *id == leaf))?
                + 1
        }
    };
    let p_idx = tree
        .get(parent)?
        .children
        .iter()
        .position(|c| matches!(c, Child::Block(b) if *b == p_block))?;

    // Move the tail leaves out of `p_block` into a fresh `new_block`.
    let mut p = tree.get(p_block).cloned().expect("validated above");
    let tail = p.children.split_off(split_at);
    let moved: Vec<Dot> = tail
        .iter()
        .filter_map(|c| match c {
            Child::Leaf { id, .. } => Some(*id),
            _ => None,
        })
        .collect();
    tree.nodes.insert(p_block, p);
    tree.nodes.insert(
        new_block,
        BlockNode {
            id: new_block,
            node_type: new_type,
            children: tail,
        },
    );
    tree.with_block_children(parent, |children| {
        children.insert(p_idx + 1, Child::Block(new_block));
    });
    Some(moved)
}

fn insert_leaf_at(
    tree: &mut BlockTree,
    block: Dot,
    offset: usize,
    leaf: Dot,
    item: &SeqItem,
) -> bool {
    // Leaf-bearing blocks never interleave block children, so the `offset`-th leaf
    // is the `offset`-th child; `leaf_slot` finds it in `O(log K)` via the children
    // tree's leaf-count sum.
    with_block_children(tree, block, |children| {
        let slot = children.leaf_slot(offset);
        children.insert(
            slot,
            Child::Leaf {
                id: leaf,
                item: item.clone(),
            },
        );
    })
}

fn remove_leaf_from_block(tree: &mut BlockTree, block: Dot, leaf: Dot) -> Option<usize> {
    let mut result: Option<usize> = None;
    with_block_children(tree, block, |children| {
        let mut seen = 0usize;
        let mut found = None;
        for (i, c) in children.iter().enumerate() {
            match c {
                Child::Leaf { id, .. } => {
                    if *id == leaf {
                        found = Some((i, seen));
                        break;
                    }
                    seen += 1;
                }
                Child::Block(_) => {}
            }
        }
        if let Some((idx, offset)) = found {
            children.remove(idx);
            result = Some(offset);
        }
    });
    result
}

pub fn project_document(logs: &DocLogs) -> Result<ProjectedDoc, ProjectionError> {
    let (elements, resolver) = checkout_with_resolver(&logs.seq);
    project_core(&elements, &resolver, logs)
}

pub fn project_from(logs: &DocLogs, seq: &SeqCheckout) -> Result<ProjectedDoc, ProjectionError> {
    let elements = seq.snapshot(&logs.seq);
    project_core(&elements, seq, logs)
}

pub fn project_core<R: SeqResolve>(
    elements: &[(Dot, SeqItem)],
    resolver: &R,
    logs: &DocLogs,
) -> Result<ProjectedDoc, ProjectionError> {
    for (d, item) in elements {
        if let SeqItem::Block { node_type, .. } = item
            && node_type.spec().is_leaf()
        {
            return Err(ProjectionError::LeafTypedBlock {
                dot: *d,
                node_type: *node_type,
            });
        }
    }

    let raw_tree = normalize(project_blocks(elements).map_err(ProjectionError::Project)?);
    let tree = BlockTree::from_raw(&raw_tree);
    validate_block_tree(&tree).map_err(ProjectionError::SchemaInvalid)?;

    Ok(project_from_tree(elements, tree, resolver, logs))
}

pub fn project_from_tree<R: SeqResolve>(
    elements: &[(Dot, SeqItem)],
    tree: BlockTree,
    resolver: &R,
    logs: &DocLogs,
) -> ProjectedDoc {
    let node_of = collect_real_nodes(&tree);
    let block_modifiers = collect_block_modifiers(&tree, &logs.block_modifiers);

    let node_attrs = logs.node_attrs.project(|d| node_of.get(&d).cloned());
    let node_carries = collect_node_carries(&tree, &logs.node_carries);

    let block_effective = block_effective_all(&tree, &logs.block_modifiers, &node_attrs);

    let mut pd = ProjectedDoc {
        tree,
        block_effective,
        seg_index: crate::span::BlockSegs::new(),
        block_modifiers,
        node_attrs,
        node_carries,
    };

    let paths = BlockPaths::from_tree(&pd.tree);
    let pos_of: HashMap<Dot, usize> = elements
        .iter()
        .enumerate()
        .map(|(i, (d, _))| (*d, i))
        .collect();
    let resolved = crate::span::ResolvedSpans::build(&logs.spans, resolver);
    pd.seg_index = build_seg_index(&pd, &paths, logs, |dot| {
        pos_of
            .get(&dot)
            .map(|&pos| resolved.covering(pos))
            .unwrap_or_default()
    });
    pd
}

/// Resolve `block_effective` for every block. Block-level effective resolution never
/// consults per-leaf explicit spans, so no span coverage is needed here.
fn block_effective_all(
    tree: &BlockTree,
    block_modifiers: &ModifierAttrLog,
    node_attrs: &imbl::HashMap<Dot, Node>,
) -> imbl::HashMap<Dot, BTreeMap<ModifierType, Modifier>> {
    let empty_ex: HashMap<Dot, BTreeMap<ModifierType, Modifier>> = HashMap::new();
    let src = crate::span::EffectiveSources {
        block_modifiers,
        explicit_spans: &empty_ex,
        node_attrs,
    };
    crate::span::derive_block_effective(tree, &src)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::{AtomLeaf, SeqItem, project_blocks};

    fn elems_nested() -> Vec<(Dot, SeqItem)> {
        let bq = Dot::new(1, 5);
        vec![
            (
                Dot::new(1, 1),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('H')),
            (Dot::new(1, 3), SeqItem::Char('i')),
            (Dot::new(1, 4), SeqItem::Atom(AtomLeaf::HardBreak)),
            (
                bq,
                SeqItem::Block {
                    node_type: NodeType::Blockquote,
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                Dot::new(1, 6),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, bq],
                },
            ),
            (Dot::new(1, 7), SeqItem::Char('y')),
            (Dot::new(1, 8), SeqItem::Char('o')),
        ]
    }

    #[test]
    fn block_paths_from_tree_matches_ancestry() {
        let root = Dot::new(1, 0);
        let bq = Dot::new(1, 1);
        let para = Dot::new(1, 2);
        let a = Dot::new(1, 3);
        let b = Dot::new(1, 4);
        let raw = crate::RawTree {
            roots: vec![crate::RawNode {
                id: root,
                node_type: NodeType::Root,
                children: vec![crate::RawChild::Block(crate::RawNode {
                    id: bq,
                    node_type: NodeType::Blockquote,
                    children: vec![crate::RawChild::Block(crate::RawNode {
                        id: para,
                        node_type: NodeType::Paragraph,
                        children: vec![
                            crate::RawChild::Leaf {
                                id: a,
                                item: SeqItem::Char('a'),
                            },
                            crate::RawChild::Leaf {
                                id: b,
                                item: SeqItem::Char('b'),
                            },
                        ],
                    })],
                })],
            }],
        };
        let tree = BlockTree::from_raw(&raw);
        let bp = BlockPaths::from_tree(&tree);
        assert_eq!(bp.block_of(a), Some(para));
        assert_eq!(bp.block_of(b), Some(para));
        assert_eq!(bp.path_of(para), vec![para, bq, root]);
        assert_eq!(bp.path_of(a), vec![a, para, bq, root]);
        let mut desc = bp.descendants_of(bq);
        desc.sort();
        let mut expect = vec![para, a, b];
        expect.sort();
        assert_eq!(desc, expect);
    }

    #[test]
    fn projection_indexes_rebuild_matches_derivations() {
        let tree = BlockTree::from_raw(&normalize(project_blocks(&elems_nested()).unwrap()));
        let projected = ProjectedDoc {
            tree: tree.clone(),
            block_effective: imbl::HashMap::new(),
            seg_index: crate::span::BlockSegs::new(),
            block_modifiers: imbl::HashMap::new(),
            node_attrs: imbl::HashMap::new(),
            node_carries: imbl::HashMap::new(),
        };
        let idx = ProjectionIndexes::rebuild_from(&projected, &SpanLog::new());
        assert_eq!(idx.paths, BlockPaths::from_tree(&tree));
    }

    #[test]
    fn collect_real_nodes_covers_blocks_chars_atoms() {
        let tree = BlockTree::from_raw(&project_blocks(&elems_nested()).unwrap());
        let nodes = collect_real_nodes(&tree);
        assert_eq!(
            nodes.get(&Dot::ROOT).map(Node::as_type),
            Some(NodeType::Root)
        );
        assert_eq!(
            nodes.get(&Dot::new(1, 1)).map(Node::as_type),
            Some(NodeType::Paragraph)
        );
        assert_eq!(
            nodes.get(&Dot::new(1, 2)).map(Node::as_type),
            Some(NodeType::Text)
        );
        assert_eq!(
            nodes.get(&Dot::new(1, 4)).map(Node::as_type),
            Some(NodeType::HardBreak)
        );
        assert_eq!(
            nodes.get(&Dot::new(1, 5)).map(Node::as_type),
            Some(NodeType::Blockquote)
        );
    }

    use crate::{
        Anchor, Bias, CalloutNodeAttr, CalloutVariant, ImageNodeAttr, Modifier, ModifierAttrOp,
        ModifierType, NodeAttr, NodeAttrOp, SpanOp,
    };
    use editor_crdt::{InputEvent, ListOp, build_oplog};

    fn events(items: &[(Dot, SeqItem)]) -> Vec<InputEvent<SeqItem>> {
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
        ev
    }

    fn logs_of(items: &[(Dot, SeqItem)]) -> DocLogs {
        DocLogs {
            seq: build_oplog(&events(items)),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_carries: ModifierAttrLog::new(),
        }
    }

    fn para_abc() -> (Vec<(Dot, SeqItem)>, Dot, Dot) {
        let para = Dot::new(1, 1);
        let elems = vec![
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('a')),
            (Dot::new(1, 3), SeqItem::Char('b')),
            (Dot::new(1, 4), SeqItem::Char('c')),
        ];
        (elems, Dot::ROOT, para)
    }

    /// A leaf's effective modifiers, read from the authoritative segment index.
    fn eff_of(pd: &ProjectedDoc, dot: Dot) -> BTreeMap<ModifierType, Modifier> {
        crate::DocView::new(pd)
            .leaf_state_by_dot_slow(dot)
            .map(|s| s.eff.clone())
            .unwrap_or_default()
    }

    /// A leaf's own modifiers, read from the authoritative segment index.
    fn own_of(pd: &ProjectedDoc, dot: Dot) -> BTreeMap<ModifierType, OwnModifier> {
        crate::DocView::new(pd)
            .leaf_state_by_dot_slow(dot)
            .map(|s| s.own.clone())
            .unwrap_or_default()
    }

    /// Total leaf count across the tree.
    fn leaf_count(pd: &ProjectedDoc) -> usize {
        fn count(tree: &BlockTree, n: &BlockNode) -> usize {
            n.children
                .iter()
                .map(|c| match c {
                    Child::Leaf { .. } => 1,
                    Child::Block(id) => tree.get(*id).map(|b| count(tree, b)).unwrap_or(0),
                })
                .sum()
        }
        pd.tree.root_node().map(|r| count(&pd.tree, r)).unwrap_or(0)
    }

    /// The number of coalesced segment groups in a block.
    fn seg_group_count(pd: &ProjectedDoc, block: Dot) -> usize {
        pd.seg_index.group_iter(block).count()
    }

    /// Every leaf dot in document order.
    fn all_leaf_dots(pd: &ProjectedDoc) -> Vec<Dot> {
        fn walk(tree: &BlockTree, n: &BlockNode, out: &mut Vec<Dot>) {
            for c in &n.children {
                match c {
                    Child::Leaf { id, .. } => out.push(*id),
                    Child::Block(id) => {
                        if let Some(b) = tree.get(*id) {
                            walk(tree, b, out);
                        }
                    }
                }
            }
        }
        let mut out = Vec::new();
        if let Some(r) = pd.tree.root_node() {
            walk(&pd.tree, r, &mut out);
        }
        out
    }

    /// Whether every block's segment leaf count equals its leaf-child count.
    fn seg_leaf_counts_match(pd: &ProjectedDoc) -> bool {
        fn walk(tree: &BlockTree, n: &BlockNode, pd: &ProjectedDoc) -> bool {
            let leaves = n
                .children
                .iter()
                .filter(|c| matches!(c, Child::Leaf { .. }))
                .count();
            pd.seg_index.leaf_count(n.id) == leaves
                && n.children.iter().all(|c| match c {
                    Child::Block(id) => tree.get(*id).map(|b| walk(tree, b, pd)).unwrap_or(true),
                    Child::Leaf { .. } => true,
                })
        }
        pd.tree
            .root_node()
            .map(|r| walk(&pd.tree, r, pd))
            .unwrap_or(true)
    }

    #[test]
    fn empty_document_projects_ok() {
        let pd = project_document(&logs_of(&[])).unwrap();
        assert_eq!(pd.tree.root_node().iter().count(), 1);
        assert_eq!(pd.tree.root_node().unwrap().node_type, NodeType::Root);
        assert_eq!(leaf_count(&pd), 0);
        assert!(pd.node_attrs.is_empty());
    }

    #[test]
    fn projects_nested_blocks() {
        let pd = project_document(&logs_of(&elems_nested())).unwrap();
        assert_eq!(pd.tree.root_node().iter().count(), 1);
        assert_eq!(pd.tree.root_node().unwrap().node_type, NodeType::Root);
        assert_eq!(leaf_count(&pd), 5);
    }

    #[test]
    fn bold_span_splits_runs() {
        let (elems, _root, para) = para_abc();
        let mut l = logs_of(&elems);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: Dot::new(1, 4),
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: Dot::new(1, 4),
                        bias: Bias::After,
                    },
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert_eq!(
            eff_of(&pd, Dot::new(1, 4)).get(&ModifierType::Bold),
            Some(&Modifier::Bold)
        );
        assert!(!eff_of(&pd, Dot::new(1, 2)).contains_key(&ModifierType::Bold));
        assert!(seg_group_count(&pd, para) >= 2);
    }

    #[test]
    fn overlays_attr_marker() {
        let callout = Dot::new(1, 1);
        let elems = vec![
            (
                callout,
                SeqItem::Block {
                    node_type: NodeType::Callout,
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                Dot::new(1, 2),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, callout],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        let mut l = logs_of(&elems);
        l.node_attrs = NodeAttrLog::new()
            .apply(
                Dot::new(2, 0),
                NodeAttrOp {
                    target: callout,
                    attr: NodeAttr::Callout {
                        attr: CalloutNodeAttr::Variant(CalloutVariant::Warning),
                    },
                },
            )
            .unwrap();
        l.node_carries = ModifierAttrLog::new()
            .apply(
                Dot::new(2, 2),
                ModifierAttrOp::SetModifier {
                    target: callout,
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert!(pd.node_attrs.contains_key(&callout));
        assert!(
            !pd.node_carries.contains_key(&callout),
            "a non-text block (callout) does not hold carry records"
        );
    }

    #[test]
    fn line_height_inheritable_double_source() {
        let (elems, _root, para) = para_abc();
        let mut l = logs_of(&elems);
        l.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::new(2, 0),
                ModifierAttrOp::SetModifier {
                    target: para,
                    modifier: Modifier::LineHeight { value: 200 },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert_eq!(
            pd.block_modifiers
                .get(&para)
                .and_then(|m| m.get(&ModifierType::LineHeight)),
            Some(&Modifier::LineHeight { value: 200 })
        );
        assert_eq!(
            pd.block_effective
                .get(&para)
                .and_then(|m| m.get(&ModifierType::LineHeight)),
            Some(&Modifier::LineHeight { value: 200 }),
            "LineHeight's consumer is the Paragraph itself, so it resolves on the paragraph block"
        );
        assert!(
            !eff_of(&pd, Dot::new(1, 2)).contains_key(&ModifierType::LineHeight),
            "the paragraph's LineHeight record is its own; it does not pass down to its text carriers"
        );
    }

    #[test]
    fn alignment_block_resolves_on_paragraph_not_descendant_text() {
        use crate::Alignment;
        let (elems, _root, para) = para_abc();
        let mut l = logs_of(&elems);
        l.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::new(2, 0),
                ModifierAttrOp::SetModifier {
                    target: para,
                    modifier: Modifier::Alignment {
                        value: Alignment::Center,
                    },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert!(
            pd.block_modifiers
                .get(&para)
                .is_some_and(|m| m.contains_key(&ModifierType::Alignment))
        );
        assert_eq!(
            pd.block_effective
                .get(&para)
                .and_then(|m| m.get(&ModifierType::Alignment)),
            Some(&Modifier::Alignment {
                value: Alignment::Center
            }),
            "the Paragraph consumes Alignment: it resolves on the paragraph block"
        );
        assert!(
            !eff_of(&pd, Dot::new(1, 2)).contains_key(&ModifierType::Alignment),
            "a Paragraph's Alignment record does not pass down to its text carriers"
        );
    }

    #[test]
    fn image_atom_attr_projected() {
        let image = Dot::new(1, 1);
        let mut img_node = match NodeType::Image.into_node() {
            Node::Image(n) => n,
            _ => unreachable!(),
        };
        img_node.id = editor_crdt::LwwReg::with_value(Some("asset-1".to_string()));
        img_node.proportion = editor_crdt::LwwReg::with_value(75);
        let elems = vec![
            (
                image,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node: img_node },
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                Dot::new(1, 2),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (Dot::new(1, 3), SeqItem::Char('x')),
        ];
        let mut l = logs_of(&elems);
        l.node_attrs = NodeAttrLog::new()
            .apply(
                Dot::new(2, 0),
                NodeAttrOp {
                    target: image,
                    attr: NodeAttr::Image {
                        attr: ImageNodeAttr::Proportion(150),
                    },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        match pd.node_attrs.get(&image) {
            Some(Node::Image(node)) => {
                assert_eq!(node.id.get(), &Some("asset-1".to_string()));
                assert_eq!(*node.proportion.get(), 150);
            }
            other => panic!("expected projected image attrs, got {other:?}"),
        }
    }

    #[test]
    fn structural_malformation_drops_and_repairs() {
        let elems = vec![(
            Dot::new(1, 1),
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![Dot::new(9, 9)],
            },
        )];
        let pd = project_document(&logs_of(&elems)).unwrap();
        let nodes = collect_real_nodes(&pd.tree);
        assert!(!nodes.contains_key(&Dot::new(1, 1)));
        assert!(validate_block_tree(&pd.tree).is_ok());
    }

    #[test]
    fn concurrent_container_delete_drops_subtree_no_crash() {
        let c = Dot::new(1, 0);
        let p1 = Dot::new(1, 1);
        let a = Dot::new(1, 2);
        let p2 = Dot::new(2, 0);
        let b = Dot::new(2, 1);
        let ev = vec![
            InputEvent {
                id: c,
                parents: vec![],
                op: ListOp::Ins {
                    pos: 0,
                    item: SeqItem::Block {
                        node_type: NodeType::Callout,
                        parents: vec![Dot::ROOT],
                    },
                },
            },
            InputEvent {
                id: p1,
                parents: vec![c],
                op: ListOp::Ins {
                    pos: 1,
                    item: SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![Dot::ROOT, c],
                    },
                },
            },
            InputEvent {
                id: a,
                parents: vec![p1],
                op: ListOp::Ins {
                    pos: 2,
                    item: SeqItem::Char('a'),
                },
            },
            InputEvent {
                id: Dot::new(1, 3),
                parents: vec![a],
                op: ListOp::Del { pos: 0, len: 3 },
            },
            InputEvent {
                id: p2,
                parents: vec![a],
                op: ListOp::Ins {
                    pos: 3,
                    item: SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![Dot::ROOT, c],
                    },
                },
            },
            InputEvent {
                id: b,
                parents: vec![p2],
                op: ListOp::Ins {
                    pos: 4,
                    item: SeqItem::Char('b'),
                },
            },
        ];
        let mut l = logs_of(&[]);
        l.seq = build_oplog(&ev);
        let pd = project_document(&l).unwrap();
        let nodes = collect_real_nodes(&pd.tree);
        assert!(!nodes.contains_key(&c));
        assert!(!nodes.contains_key(&p2));
        assert!(!nodes.contains_key(&b));
        assert!(validate_block_tree(&pd.tree).is_ok());
    }

    #[test]
    fn leaf_typed_block_errors() {
        for leaf_ty in [NodeType::Text, NodeType::Image] {
            let elems = vec![(
                Dot::new(1, 1),
                SeqItem::Block {
                    node_type: leaf_ty,
                    parents: vec![Dot::ROOT],
                },
            )];
            assert!(
                matches!(
                    project_document(&logs_of(&elems)),
                    Err(ProjectionError::LeafTypedBlock { .. })
                ),
                "leaf-typed block {leaf_ty:?} must fail-loud"
            );
        }
    }

    #[test]
    fn concurrent_block_delete_drops_orphan_no_crash() {
        let p = Dot::new(1, 0);
        let a = Dot::new(1, 1);
        let b = Dot::new(1, 2);
        let ev = vec![
            InputEvent {
                id: p,
                parents: vec![],
                op: ListOp::Ins {
                    pos: 0,
                    item: SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![Dot::ROOT],
                    },
                },
            },
            InputEvent {
                id: a,
                parents: vec![p],
                op: ListOp::Ins {
                    pos: 1,
                    item: SeqItem::Char('a'),
                },
            },
            InputEvent {
                id: b,
                parents: vec![a],
                op: ListOp::Ins {
                    pos: 2,
                    item: SeqItem::Char('b'),
                },
            },
            InputEvent {
                id: Dot::new(1, 3),
                parents: vec![b],
                op: ListOp::Del { pos: 0, len: 3 },
            },
            InputEvent {
                id: Dot::new(2, 0),
                parents: vec![a],
                op: ListOp::Ins {
                    pos: 2,
                    item: SeqItem::Char('X'),
                },
            },
        ];
        let mut l = logs_of(&[]);
        l.seq = build_oplog(&ev);
        let pd = project_document(&l).unwrap();
        let nodes = collect_real_nodes(&pd.tree);
        assert!(!nodes.contains_key(&Dot::new(2, 0)));
        assert!(validate_block_tree(&pd.tree).is_ok());
        assert_eq!(leaf_count(&pd), 0);
    }

    #[test]
    fn unknown_anchor_span_is_dropped() {
        let (elems, _root, _para) = para_abc();
        let mut l = logs_of(&elems);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: Dot::new(7, 7),
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: Dot::new(8, 8),
                        bias: Bias::After,
                    },
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert!(!eff_of(&pd, Dot::new(1, 2)).contains_key(&ModifierType::Bold));
    }

    #[test]
    fn deleted_anchor_span_no_panic() {
        let para = Dot::new(1, 1);
        let a = Dot::new(1, 2);
        let b = Dot::new(1, 3);
        let c = Dot::new(1, 4);
        let mut ev = events(&[
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            ),
            (a, SeqItem::Char('a')),
            (b, SeqItem::Char('b')),
            (c, SeqItem::Char('c')),
        ]);
        ev.push(InputEvent {
            id: Dot::new(1, 5),
            parents: vec![c],
            op: ListOp::Del { pos: 3, len: 1 },
        });
        let mut l = logs_of(&[]);
        l.seq = build_oplog(&ev);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: a,
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: b,
                        bias: Bias::After,
                    },
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        assert!(project_document(&l).is_ok());
    }

    #[test]
    fn stale_overlay_does_not_leak() {
        let (elems, _root, _para) = para_abc();
        let mut l = logs_of(&elems);
        l.node_carries = ModifierAttrLog::new()
            .apply(
                Dot::new(2, 0),
                ModifierAttrOp::SetModifier {
                    target: Dot::new(9, 9),
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert!(!pd.node_carries.contains_key(&Dot::new(9, 9)));
    }

    #[test]
    fn duplicate_fixed_slot_loser_overlay_no_leak() {
        let fold = Dot::new(1, 1);
        let title1 = Dot::new(1, 2);
        let loser = Dot::new(1, 3);
        let content = Dot::new(1, 4);
        let elems = vec![
            (
                fold,
                SeqItem::Block {
                    node_type: NodeType::Fold,
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                title1,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![Dot::ROOT, fold],
                },
            ),
            (
                loser,
                SeqItem::Block {
                    node_type: NodeType::FoldTitle,
                    parents: vec![Dot::ROOT, fold],
                },
            ),
            (
                content,
                SeqItem::Block {
                    node_type: NodeType::FoldContent,
                    parents: vec![Dot::ROOT, fold],
                },
            ),
        ];
        let mut l = logs_of(&elems);
        l.node_carries = ModifierAttrLog::new()
            .apply(
                Dot::new(2, 0),
                ModifierAttrOp::SetModifier {
                    target: loser,
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert!(!pd.node_carries.contains_key(&loser));
    }

    #[test]
    fn effective_matches_module_reference() {
        let (elems, _root, _para) = para_abc();
        let mut l = logs_of(&elems);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: Dot::new(1, 2),
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: Dot::new(1, 3),
                        bias: Bias::After,
                    },
                    modifier: Modifier::Italic,
                },
            )
            .unwrap();
        let (els, resolver) = checkout_with_resolver(&l.seq);
        let tree = BlockTree::from_raw(&normalize(project_blocks(&els).unwrap()));
        let node_of = collect_real_nodes(&tree);
        let node_attrs = l.node_attrs.project(|d| node_of.get(&d).cloned());
        let explicit: HashMap<Dot, _> =
            crate::span::derive_explicit_effect(&els, &tree, &resolver, &l.spans)
                .into_iter()
                .collect();
        let src = crate::span::EffectiveSources {
            block_modifiers: &l.block_modifiers,
            explicit_spans: &explicit,
            node_attrs: &node_attrs,
        };
        let pd = project_document(&l).unwrap();
        // Every leaf's segment-served effective matches the module reference resolve.
        crate::span::for_each_leaf(&tree, |path, leaf_type, leaf_dot| {
            let expected =
                crate::span::resolve_effective(path, Some(leaf_dot), leaf_type, true, &src);
            assert_eq!(
                eff_of(&pd, leaf_dot),
                expected,
                "eff mismatch at {leaf_dot:?}"
            );
        });
    }

    #[test]
    fn project_document_own_modifiers_present() {
        let (elems, _root, _para) = para_abc();
        let mut l = logs_of(&elems);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(2, 0),
                SpanOp::AddSpan {
                    start: Anchor {
                        id: Dot::new(1, 2),
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: Dot::new(1, 2),
                        bias: Bias::After,
                    },
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert_eq!(
            own_of(&pd, Dot::new(1, 2)).get(&ModifierType::Bold),
            Some(&crate::OwnModifier {
                value: Modifier::Bold
            })
        );
    }

    use proptest::prelude::*;

    fn arb_para_doc() -> impl Strategy<Value = Vec<(Dot, SeqItem)>> {
        "[a-c]{1,8}".prop_map(|s| {
            let para = Dot::new(1, 1);
            let mut v = vec![(
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            )];
            for (i, ch) in s.chars().enumerate() {
                v.push((Dot::new(1, 2 + i as u64), SeqItem::Char(ch)));
            }
            v
        })
    }

    proptest! {
        #[test]
        fn invariants_hold(items in arb_para_doc()) {
            let pd = project_document(&logs_of(&items)).unwrap();

            prop_assert!(validate_block_tree(&pd.tree).is_ok());

            let live = collect_real_nodes(&pd.tree);
            let live_leaves: HashSet<Dot> = live.iter()
                .filter(|(_, n)| n.as_type().spec().is_leaf())
                .map(|(d, _)| *d)
                .collect();

            // Segments cover exactly the live leaves, block by block.
            prop_assert!(seg_leaf_counts_match(&pd));
            let mut covered: Vec<Dot> = all_leaf_dots(&pd);
            covered.sort();
            let mut expect: Vec<Dot> = live_leaves.into_iter().collect();
            expect.sort();
            prop_assert_eq!(covered, expect);

            let all_ids: HashSet<Dot> = live.keys().copied().collect();
            for d in pd.node_attrs.keys() { prop_assert!(all_ids.contains(d)); }
            for d in pd.node_carries.keys() { prop_assert!(all_ids.contains(d)); }
            for d in pd.block_modifiers.keys() { prop_assert!(all_ids.contains(d)); }

            fn collect_block_ids(tree: &BlockTree, node: &BlockNode, out: &mut HashSet<Dot>) {
                out.insert(node.id);
                for c in &node.children {
                    if let Child::Block(id) = c
                        && let Some(b) = tree.get(*id)
                    {
                        collect_block_ids(tree, b, out);
                    }
                }
            }
            let mut block_ids: HashSet<Dot> = HashSet::new();
            if let Some(r) = pd.tree.root_node() {
                collect_block_ids(&pd.tree, r, &mut block_ids);
            }
            let be_keys: HashSet<Dot> = pd.block_effective.keys().copied().collect();
            prop_assert_eq!(be_keys, block_ids);
        }

        #[test]
        fn deterministic_under_shuffle(items in arb_para_doc()) {
            let ev = events(&items);
            let mut a = logs_of(&[]);
            a.seq = build_oplog(&ev);
            let mut rev = ev.clone();
            rev.reverse();
            let mut b = logs_of(&[]);
            b.seq = build_oplog(&rev);
            prop_assert_eq!(project_document(&a).unwrap(), project_document(&b).unwrap());
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 128, ..ProptestConfig::default() })]
        #[test]
        fn s4b_char_append_incremental_matches_full(s in "[a-c]{0,24}") {
            let para = Dot::new(1, 1);
            let base = vec![(
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                },
            )];
            let mut full = base.clone();
            for (i, ch) in s.chars().enumerate() {
                full.push((Dot::new(1, 2 + i as u64), SeqItem::Char(ch)));
            }

            let mut projected = project_document(&logs_of(&base)).unwrap();
            for (i, ch) in s.chars().enumerate() {
                let leaf = Dot::new(1, 2 + i as u64);
                projected.splice_char(para, i, leaf, SeqItem::Char(ch));
            }

            // `splice_char` maintains only the tree; segment maintenance is the caller's
            // job (editor-state). So the incremental tree must match the cold projection.
            let full_pd = project_document(&logs_of(&full)).unwrap();
            prop_assert_eq!(&projected.tree, &full_pd.tree);
        }
    }

    #[test]
    fn adjacent_block_atoms_do_not_share_block_modifier() {
        // Two adjacent block-level image atoms under Root, the FIRST carrying an
        // Alignment block modifier. Each non-inline leaf is its own segment singleton,
        // deriving block modifiers through its real dot — so the second image must NOT
        // cache-hit the first and inherit its Alignment.
        let img1 = Dot::new(1, 1);
        let img2 = Dot::new(1, 2);
        let img_node = || match NodeType::Image.into_node() {
            Node::Image(n) => n,
            _ => unreachable!(),
        };
        let elems = vec![
            (
                img1,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node: img_node() },
                    parents: vec![Dot::ROOT],
                },
            ),
            (
                img2,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node: img_node() },
                    parents: vec![Dot::ROOT],
                },
            ),
        ];
        let mut l = logs_of(&elems);
        l.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::new(2, 0),
                ModifierAttrOp::SetModifier {
                    target: img1,
                    modifier: Modifier::Alignment {
                        value: crate::Alignment::Center,
                    },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert!(
            eff_of(&pd, img1).contains_key(&ModifierType::Alignment),
            "first image keeps its own alignment"
        );
        assert!(
            !eff_of(&pd, img2).contains_key(&ModifierType::Alignment),
            "second image must NOT inherit the first's alignment"
        );
    }
}
