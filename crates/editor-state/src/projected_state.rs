use editor_crdt::sequence::{Bias, SeqCheckout};
use editor_crdt::{Changeset, CrdtError, Dot, InputEvent, ListOp, Op, OpGraph, OpLog};
use editor_model::{
    Anchor, AtomLeaf, BlockNode, BlockPaths, BlockTree, Child, ChildList, DocLogs, DocView, EditOp,
    Modifier, ModifierAttrLog, ModifierType, Node, NodeAttrLog, NodeType, ProjectedDoc,
    ProjectionError, ProjectionIndexes, RawChild, RawNode, RepairStats, SeqItem, SpanAnchorIndex,
    SpanLog, SpanOp, SplitError, anchor_dot, block_effective_one, block_init_of,
    normalize_content_shallow_with_stats, normalize_window_forest_with_stats, project_blocks,
    project_from, project_from_tree, project_with_overlay, seq_parents, split_block_insert,
    split_logs,
};
use hashbrown::{HashMap, HashSet};

#[derive(Debug)]
pub enum SpineError {
    Crdt(CrdtError),
    Split(SplitError),
    Projection(ProjectionError),
    /// A locally-generated op failed admission validation (see
    /// `editor_model::alias_op_is_valid`) before it could reach `graph.add_mut`.
    InvalidOp,
}

impl From<CrdtError> for SpineError {
    fn from(e: CrdtError) -> Self {
        SpineError::Crdt(e)
    }
}
impl From<SplitError> for SpineError {
    fn from(e: SplitError) -> Self {
        SpineError::Split(e)
    }
}
impl From<ProjectionError> for SpineError {
    fn from(e: ProjectionError) -> Self {
        SpineError::Projection(e)
    }
}

// An inline leaf that lives inside a block like a character: a `Char`, or a
// non-block-level `Atom` (HardBreak/Tab/PageBreak). Block-level atoms project to
// `BlockAtom` structural nodes and must take the structural (fallback) path.
fn is_inline_leaf_item(item: &SeqItem) -> bool {
    match item {
        SeqItem::Char(_) => true,
        SeqItem::Atom(l) => !l.is_block_level(),
        _ => false,
    }
}

/// Fold a leaf's covering span-op dots into the canonical per-type LWW winner —
/// the segment-key `covering`. Mirrors `projection::canonical_covering` (private
/// there) so warm seg maintenance keys segments identically to cold segmentation.
fn canonical_covering_of(dots: &[Dot], spans: &SpanLog) -> Option<editor_model::SegCovering> {
    let mut cov: Option<editor_model::SegCovering> = None;
    for &d in dots {
        if let Some(op) = spans.get(d)
            && let Some(next) =
                editor_model::covering_absorb(cov.as_ref(), editor_model::covering_of_op(op), d)
        {
            cov = Some(next);
        }
    }
    cov
}

/// Where [`ProjectedState::resync_block_segs`] sources each leaf's segment covering.
enum CoveringSource<'a> {
    /// Resolve from the span log by visible position — structural rebuilds (window
    /// reproject, block split, undelete) where new/moved leaves have no prior segment
    /// to copy from. `O(leaves · spans)`, matching cold projection; the resolve is
    /// built once per operation and stabbed per position.
    Resolved(&'a editor_model::ResolvedSpans),
    /// Reuse each leaf's covering from its existing segment — a node/style/modifier op
    /// re-derives `(eff, own)` but never moves a leaf across a span anchor, so the
    /// covering is unchanged and re-resolving the log would be wasted work.
    Existing,
}

fn collect_subtree_nodes(tree: &BlockTree, node: &BlockNode, out: &mut Vec<Dot>) {
    out.push(node.id);
    for c in &node.children {
        match c {
            Child::Leaf { id, .. } => out.push(*id),
            Child::Block(id) => {
                if let Some(b) = tree.get(*id) {
                    collect_subtree_nodes(tree, b, out);
                }
            }
        }
    }
}

/// Graft a Root-repair WRAP scaffold subtree into the tree, ID-aware: a node that
/// already exists (a real block the scaffold reparented) keeps its live subtree —
/// the scaffold references it by id rather than clobbering it with the empty
/// shallow proxy (which would orphan the real block's real descendants). Preserved
/// real ids are collected so the caller keeps them out of its drop set. Newly
/// synthesized scaffold nodes are inserted whole.
fn graft_shallow_scaffold(tree: &mut BlockTree, raw: &RawNode, reparented: &mut HashSet<Dot>) {
    if tree.get(raw.id).is_some() {
        reparented.insert(raw.id);
        return;
    }
    let children: ChildList = raw
        .children
        .iter()
        .map(|c| match c {
            RawChild::Leaf { id, item } => Child::Leaf {
                id: *id,
                item: item.clone(),
            },
            RawChild::Block(b) => {
                graft_shallow_scaffold(tree, b, reparented);
                Child::Block(b.id)
            }
        })
        .collect();
    tree.nodes.insert(
        raw.id,
        BlockNode {
            id: raw.id,
            node_type: raw.node_type,
            attrs: raw.attrs.clone(),
            children,
        },
    );
}

fn last_known_child_for_shallow_root_repair(
    tree: &BlockTree,
    block: &BlockNode,
) -> Option<RawChild> {
    (0..block.children.len())
        .rev()
        .find_map(|index| match block.children.get(index)? {
            Child::Leaf { id, item }
                if item
                    .as_child_type()
                    .is_some_and(|ty| ty != NodeType::Unknown) =>
            {
                Some(RawChild::Leaf {
                    id: *id,
                    item: item.clone(),
                })
            }
            Child::Block(id) => tree
                .get(*id)
                .filter(|child| child.node_type != NodeType::Unknown)
                .map(|child| {
                    RawChild::Block(RawNode {
                        id: child.id,
                        node_type: child.node_type,
                        attrs: child.attrs.clone(),
                        children: Vec::new(),
                    })
                }),
            Child::Leaf { .. } => None,
        })
}

type BlockLeafPlan = Vec<(Dot, Vec<(Dot, Option<NodeType>)>)>;

/// Plan the index/derivation entries for a freshly re-projected (nested scratch)
/// subtree about to be grafted into the live tree.
fn plan_subtree(
    node: &RawNode,
    parent: Dot,
    blocks: &mut Vec<(Dot, Dot, NodeType)>,
    block_leaves: &mut BlockLeafPlan,
    nodes: &mut HashSet<Dot>,
) {
    nodes.insert(node.id);
    blocks.push((node.id, parent, node.node_type));
    let mut leaves = Vec::new();
    for c in &node.children {
        if let RawChild::Leaf { id, item } = c {
            nodes.insert(*id);
            leaves.push((*id, item.as_child_type()));
        }
    }
    block_leaves.push((node.id, leaves));
    for c in &node.children {
        if let RawChild::Block(b) = c {
            plan_subtree(b, node.id, blocks, block_leaves, nodes);
        }
    }
}

/// Remembers where the last incrementally-spliced leaf landed, so a sequential
/// insert run (paste, typing) can place each new leaf right after the previous one
/// without a positional search. See `leaf_insert_offset_cursored`.
#[derive(Clone, Copy, Debug)]
struct LeafCursor {
    block: Dot,
    leaf: Dot,
    offset: usize,
}

#[derive(Clone, Debug)]
pub struct ProjectedState {
    graph: OpGraph<EditOp>,
    logs: DocLogs,
    seq: SeqCheckout,
    projected: ProjectedDoc,
    indexes: ProjectionIndexes,
    leaf_cursor: Option<LeafCursor>,
    layout_dirty: crate::LayoutDirty,
    /// Running total of per-pass repair stats across every projection (full and
    /// incremental) since the last [`take_repair_stats`](Self::take_repair_stats).
    /// Repairs (revival / WRAP / SPLIT-HOIST) re-count on each pass over a persistent
    /// damaged state; `drops`/`totality_violations`/`projection_degraded` record
    /// actual content loss or degradation.
    repair_stats: RepairStats,
}

impl ProjectedState {
    fn build_warm(
        graph: &OpGraph<EditOp>,
    ) -> Result<(DocLogs, SeqCheckout, ProjectedDoc, ProjectionIndexes), SpineError> {
        let logs = split_logs(graph)?;
        let mut seq = SeqCheckout::new();
        seq.apply_tail(&logs.seq);
        let projected = project_from(&logs, &seq)?;
        let indexes = ProjectionIndexes::rebuild_from(&projected, &logs.spans);
        Ok((logs, seq, projected, indexes))
    }

    fn build_warm_with_overlay(
        graph: &OpGraph<EditOp>,
        overlay: &[Dot],
    ) -> Result<(DocLogs, SeqCheckout, ProjectedDoc, ProjectionIndexes), SpineError> {
        let logs = split_logs(graph)?;
        let mut seq = SeqCheckout::new();
        seq.apply_tail(&logs.seq);
        let projected = project_with_overlay(&logs, &seq, overlay)?;
        let indexes = ProjectionIndexes::rebuild_from(&projected, &logs.spans);
        Ok((logs, seq, projected, indexes))
    }

    fn rebuild_from_graph(&mut self) -> Result<(), SpineError> {
        let (logs, seq, projected, indexes) = Self::build_warm(&self.graph)?;
        self.repair_stats.accumulate(&projected.repair_stats);
        self.logs = logs;
        self.seq = seq;
        self.projected = projected;
        self.indexes = indexes;
        self.leaf_cursor = None;
        self.mark_dirty_full();
        Ok(())
    }

    fn warm_dispatch(&mut self, op: &Op<EditOp>) -> Result<(), SpineError> {
        match &op.payload {
            EditOp::Seq(list_op) => {
                let ev = InputEvent {
                    id: op.id,
                    parents: seq_parents(&self.graph, op.id),
                    op: list_op.clone(),
                };
                self.logs.seq.push(ev);
                self.seq.apply_tail(&self.logs.seq);
            }
            EditOp::Span(o) => {
                self.logs.spans = self
                    .logs
                    .spans
                    .apply(op.id, o.clone())
                    .map_err(SplitError::Crdt)?
            }
            EditOp::BlockModifier(o) => {
                self.logs.block_modifiers = self
                    .logs
                    .block_modifiers
                    .apply(op.id, o.clone())
                    .map_err(SplitError::Crdt)?
            }
            EditOp::NodeAttr(o) => {
                self.logs.node_attrs = self
                    .logs
                    .node_attrs
                    .apply(op.id, o.clone())
                    .map_err(SplitError::Crdt)?
            }
            EditOp::NodeCarry(o) => {
                self.logs.node_carries = self
                    .logs
                    .node_carries
                    .apply(op.id, o.clone())
                    .map_err(SplitError::Crdt)?
            }
            EditOp::Alias(o) => {
                self.logs.aliases.apply(o.clone());
                self.projected.alias_classes.apply(o);
            }
            EditOp::Unknown { .. } => {}
        }
        Ok(())
    }

    /// Admission gate for locally-generated ops only — must reject before any
    /// `&mut self` mutation; rehydrated ops enter via `ingest_op_warm` unvalidated.
    pub(crate) fn apply_op_warm(&mut self, payload: EditOp) -> Result<Op<EditOp>, SpineError> {
        if let EditOp::Alias(o) = &payload
            && !editor_model::alias_op_is_valid(o)
        {
            return Err(SpineError::InvalidOp);
        }
        let op = self.graph.add_mut(payload)?;
        self.warm_dispatch(&op)?;
        Ok(op)
    }

    pub(crate) fn ingest_op_warm(&mut self, op: &Op<EditOp>) -> Result<(), SpineError> {
        self.warm_dispatch(op)
    }

    pub fn from_graph(graph: OpGraph<EditOp>) -> Result<Self, SpineError> {
        Self::from_graph_with_overlay(graph, &[])
    }

    pub fn from_graph_with_overlay(
        graph: OpGraph<EditOp>,
        overlay: &[Dot],
    ) -> Result<Self, SpineError> {
        let (logs, seq, projected, indexes) = Self::build_warm_with_overlay(&graph, overlay)?;
        let repair_stats = projected.repair_stats;
        Ok(Self {
            graph,
            logs,
            seq,
            projected,
            indexes,
            leaf_cursor: None,
            layout_dirty: crate::LayoutDirty::Full,
            repair_stats,
        })
    }

    pub fn empty() -> Self {
        let mut graph = OpGraph::<EditOp>::with_actor(1);
        graph
            .add_mut(EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }))
            .expect("seed paragraph never conflicts");
        Self::from_graph(graph).expect("seed paragraph always projects")
    }

    fn mark_dirty_block(&mut self, dot: Dot) {
        self.layout_dirty.mark_content(dot);
    }

    fn mark_dirty_structural(&mut self, dot: Dot) {
        self.layout_dirty.mark_structural(dot);
    }

    fn mark_dirty_full(&mut self) {
        self.layout_dirty.mark_full();
    }

    pub fn take_layout_dirty(&mut self) -> crate::LayoutDirty {
        std::mem::replace(&mut self.layout_dirty, crate::LayoutDirty::empty())
    }

    fn reproject(&mut self) -> Result<(), SpineError> {
        let projected = project_from(&self.logs, &self.seq)?;
        let paths = BlockPaths::from_tree(&projected.tree);
        let spans = SpanAnchorIndex::build(&self.logs.spans);
        self.repair_stats.accumulate(&projected.repair_stats);
        self.projected = projected;
        self.indexes = ProjectionIndexes { paths, spans };
        self.leaf_cursor = None;
        self.mark_dirty_full();
        Ok(())
    }

    /// Whole-document reprojection for a bulk delete, reusing the span anchor index
    /// (a deletion never changes span anchors) but re-resolving the span log for the
    /// surviving leaves' coverings — `O(#spans · log)`. This is an accepted trade-off
    /// for a rare structural op now that the per-leaf coverage index (which let this
    /// path copy survivors' coverings in `O(survivors)`) is gone; the sole covering
    /// store is the segment index, which a full reprojection rebuilds from scratch.
    ///
    /// Only valid when every op since the last projection was a deletion (the carried
    /// anchor index describes the pre-delete spans) — the contract callers of
    /// [`apply_warm_only`](Self::apply_warm_only) with delete-only batches uphold.
    pub fn reproject_after_delete(&mut self) -> Result<(), SpineError> {
        let projected = project_from(&self.logs, &self.seq)?;
        let paths = BlockPaths::from_tree(&projected.tree);
        let spans = self.indexes.spans.clone();
        self.repair_stats.accumulate(&projected.repair_stats);
        self.projected = projected;
        self.indexes = ProjectionIndexes { paths, spans };
        self.leaf_cursor = None;
        self.mark_dirty_full();
        Ok(())
    }

    fn project_op(&mut self, op: &Op<EditOp>) -> Result<(), SpineError> {
        let ok = self.try_incremental(op);
        if !ok {
            match &op.payload {
                EditOp::Seq(_) => {
                    let ar = self.affected_range(op);
                    match ar {
                        Some((i, j)) => {
                            // An insert/undelete can introduce a block BEFORE the
                            // affected block's marker (lifting a paragraph to the front,
                            // restoring a leading list), so extend the window's left
                            // edge down to the earliest affected sequence position.
                            let floor = match &op.payload {
                                EditOp::Seq(ListOp::Ins { .. }) => self
                                    .seq
                                    .resolve_boundary(op.id, Bias::Before)
                                    .map(|b| b.position),
                                EditOp::Seq(ListOp::Undel { del }) => self
                                    .seq
                                    .del_target_dots(&self.logs.seq, *del)
                                    .iter()
                                    .filter_map(|t| {
                                        self.seq
                                            .resolve_boundary(*t, Bias::Before)
                                            .map(|b| b.position)
                                    })
                                    .min(),
                                _ => None,
                            };
                            self.reproject_window(i, j, floor)?;
                        }
                        None => self.reproject()?,
                    }
                }
                _ => self.reproject_from_tree()?,
            }
        }
        Ok(())
    }

    fn top_child_of(&self, dot: Dot) -> Option<Dot> {
        if !self.indexes.paths.contains(dot) {
            return None;
        }
        let mut cur = match self.indexes.paths.block_of(dot) {
            Some(Dot::ROOT) => return Some(dot),
            Some(block) => block,
            None => dot,
        };
        while let Some(p) = self.indexes.paths.parent_of(cur) {
            if p == Dot::ROOT {
                return Some(cur);
            }
            cur = p;
        }
        Some(cur)
    }

    fn top_child_index(&self, dot: Dot) -> Option<usize> {
        let top = self.top_child_of(dot)?;
        self.projected
            .tree
            .root_node()?
            .children
            .iter()
            .position(|child| match child {
                Child::Block(block) => *block == top,
                Child::Leaf { id, .. } => *id == top,
            })
    }

    fn top_child_near_pos(&self, pos: usize) -> Option<usize> {
        for cand in [pos.checked_sub(1), Some(pos), pos.checked_add(1)]
            .into_iter()
            .flatten()
        {
            if let Some(d) = self.seq.dot_at_visible(&self.logs.seq, cand)
                && let Some(idx) = self.top_child_index(d)
            {
                return Some(idx);
            }
        }
        // The ±1 neighbours are all ghosts (visible in the sequence but dropped from
        // the tree, e.g. content trailing a mid-text PageBreak). Fall back to the
        // enclosing top-level child so the op localizes to a window instead of a
        // whole-document reprojection. This branch is reached only when the cheap ±1
        // probe fails, so the common path is untouched.
        self.enclosing_top_child(pos)
    }

    /// The top-level child whose sequence range contains `pos`: the real child with the
    /// greatest marker/atom position `≤ pos`. Synthetic/scaffolded blocks (no resolvable
    /// position, e.g. a normalizer-inserted trailing paragraph) are skipped.
    fn enclosing_top_child(&self, pos: usize) -> Option<usize> {
        let mut best: Option<(usize, usize)> = None; // (marker_pos, child_index)
        let root = self.projected.tree.root_node()?;
        for (idx, c) in root.children.iter().enumerate() {
            let id = match c {
                Child::Block(block) => *block,
                Child::Leaf { id, .. } => *id,
            };
            if let Some(m) = self
                .seq
                .resolve_boundary(id, Bias::Before)
                .map(|x| x.position)
                && m <= pos
                && best.is_none_or(|(bm, _)| m >= bm)
            {
                best = Some((m, idx));
            }
        }
        best.map(|(_, idx)| idx)
    }

    /// The inclusive range of top-level child indices a structural op affects.
    /// Over-approximated (±1 sibling) so block merges/splits stay inside the
    /// window. `None` ⇒ caller can't localize (rare edge / Undel) → full path.
    fn affected_range(&self, op: &Op<EditOp>) -> Option<(usize, usize)> {
        let mut idxs: Vec<usize> = Vec::new();
        match &op.payload {
            EditOp::Seq(ListOp::Ins { .. }) => {
                let p = self
                    .seq
                    .resolve_boundary(op.id, Bias::Before)
                    .map(|b| b.position)?;
                idxs.push(self.top_child_near_pos(p)?);
            }
            EditOp::Seq(ListOp::Del { .. }) => {
                let targets = self.seq.del_target_dots(&self.logs.seq, op.id);
                // Collect the distinct top-level children via the O(depth) parent walk,
                // then resolve every child index in ONE pass over the root's children —
                // per-target `top_child_index` is an O(#children) scan, which makes a
                // multi-child delete `O(#targets · #children)`. Positions can't localize
                // a Del the way the Undel arm below does: the op is already applied, so
                // its targets are tombstoned and their boundary positions all collapse
                // onto the same surviving neighbour.
                let mut top_children: HashSet<Dot> = HashSet::new();
                for t in &targets {
                    if let Some(top_child) = self.top_child_of(*t) {
                        top_children.insert(top_child);
                    }
                }
                if !top_children.is_empty()
                    && let Some(root) = self.projected.tree.root_node()
                {
                    for (idx, c) in root.children.iter().enumerate() {
                        let id = match c {
                            Child::Block(block) => block,
                            Child::Leaf { id, .. } => id,
                        };
                        if top_children.contains(id) {
                            idxs.push(idx);
                        }
                    }
                }
                // Every target was a ghost (already absent from the tree). Localize to
                // the top-level child around where the deleted content lived — via the
                // in-tree neighbours of its position, then the enclosing child — rather
                // than reprojecting the whole document. `top_child_near_pos` falls back
                // through both, so even a ghost whose own child marker was already
                // deleted still localizes through a surviving sibling.
                if idxs.is_empty() {
                    for t in &targets {
                        if let Some(p) = self
                            .seq
                            .resolve_boundary(*t, Bias::Before)
                            .map(|b| b.position)
                            && let Some(idx) = self.top_child_near_pos(p)
                        {
                            idxs.push(idx);
                            break;
                        }
                    }
                }
            }
            EditOp::Seq(ListOp::Undel { del }) => {
                // Restored elements aren't in the (pre-op) tree; each is located by the
                // position it reappears at, via its in-tree neighbour. The window only
                // needs to span the restored range, so localize just its min/max
                // positions — not every restored element. Localizing all of them is
                // `O(#targets · #children)` (each `top_child_near_pos` scans the root's
                // children), which makes undoing a large delete quadratic. Resolving
                // positions is the cheap part; the block scan is what must stay bounded.
                let mut min_pos = usize::MAX;
                let mut max_pos = 0usize;
                for t in self.seq.del_target_dots(&self.logs.seq, *del) {
                    if let Some(p) = self
                        .seq
                        .resolve_boundary(t, Bias::Before)
                        .map(|b| b.position)
                    {
                        min_pos = min_pos.min(p);
                        max_pos = max_pos.max(p);
                    }
                }
                if min_pos != usize::MAX {
                    for p in [min_pos, max_pos] {
                        if let Some(idx) = self.top_child_near_pos(p) {
                            idxs.push(idx);
                        }
                    }
                }
            }
            _ => return None,
        }
        if idxs.is_empty() {
            return None;
        }
        let n = self
            .projected
            .tree
            .root_node()
            .map(|r| r.children.len())
            .unwrap_or(0);
        if n == 0 {
            return None;
        }
        let lo = idxs.iter().copied().min()?.saturating_sub(1);
        let hi = (idxs.iter().copied().max()? + 1).min(n - 1);
        Some((lo, hi))
    }

    /// Re-project only top-level children `[i, j]` from the live sequence and splice
    /// the result back, updating just those subtrees' indexes/derivations. Reuses
    /// `project_blocks` + `normalize` for correctness (all structural shapes,
    /// normalization) at O(window), never the whole document.
    fn reproject_window(
        &mut self,
        i: usize,
        j: usize,
        floor: Option<usize>,
    ) -> Result<(), SpineError> {
        let root_id = self.projected.tree.root;
        // Read-only snapshot of the root's children (O(1) persistent clone) for window
        // computation; the tree is mutated only after the window is fully planned.
        let Some(root_children) = self.projected.tree.root_node().map(|r| r.children.clone())
        else {
            return self.reproject();
        };
        if i > j || j >= root_children.len() {
            return self.reproject();
        }
        let child_id = |c: &Child| match c {
            Child::Block(block) => *block,
            Child::Leaf { id, .. } => *id,
        };
        // Real dots the window's blocks own — the [i..=j] subtrees, grown as
        // reparenting siblings are pulled in. A top-level sibling whose block-marker
        // parents chain roots inside this set reparents INTO the window after this op
        // (the order invariant makes it share the window's contiguous sequence range),
        // so it must be reprojected together or a cold rebuild diverges.
        let mut i = i;
        let mut j = j;
        let mut window_owned: HashSet<Dot> = HashSet::new();
        // Ancestors any window child's marker still nests under — drives left extension
        // to a preceding sibling a window element reparents onto.
        let mut referenced: HashSet<Dot> = HashSet::new();
        for slot in i..=j {
            if let Some(c) = root_children.get(slot) {
                let id = child_id(c);
                window_owned.extend(self.subtree_real_dots(id));
                referenced.extend(self.marker_parents(id));
            }
        }
        // Left extension runs BEFORE right extension so the right pass sees every block
        // the window came to own. Pull in a preceding sibling when either:
        //  (1) a window element still nests under it (its id is referenced by window
        //      content) — its marker must open the window so `project_blocks` rebuilds
        //      the nesting instead of reviving the content to Root; or
        //  (2) the current head is a synthetic SPLIT/WRAP scaffold — it owns no
        //      sequence marker, so its content sequence-belongs to an earlier real
        //      block. The window must open at that real block's marker or a mid-run
        //      snapshot regroups the split differently than a full reprojection. Walk
        //      back through the contiguous scaffold run to its real origin.
        // (A following sibling can never nest under a *right*-pulled block — ancestors
        // precede descendants in the sequence — so left-then-right needs no re-pass.)
        while i > 0 {
            let head_synthetic = root_children
                .get(i)
                .map(&child_id)
                .is_some_and(|d| d.is_synthetic());
            let Some(prev) = root_children.get(i - 1).map(&child_id) else {
                break;
            };
            if !head_synthetic && !referenced.contains(&prev) {
                break;
            }
            window_owned.extend(self.subtree_real_dots(prev));
            referenced.extend(self.marker_parents(prev));
            i -= 1;
        }
        // Right extension: subsume trailing synthetic scaffolds (own real dots, no
        // resolvable marker — e.g. a normalizer trailing paragraph) and following real
        // siblings that reparent into the window (including onto a block the left pass
        // just pulled in). Stop at the first genuine window-external child, whose marker
        // begins the excluded tail (`window_end`); once a non-descendant closes the run
        // the order invariant forbids any later sibling from reopening the window. If
        // only scaffolds trail, the window runs to the sequence end — never a full
        // reproject.
        let mut window_end = self.seq.visible_len();
        let mut k = j + 1;
        while let Some(c) = root_children.get(k) {
            let id = child_id(c);
            let parents = self.marker_parents(id);
            let resolved = self
                .seq
                .resolve_boundary(id, Bias::Before)
                .map(|b| b.position);
            match resolved {
                Some(p) if !parents.iter().any(|d| window_owned.contains(d)) => {
                    window_end = p;
                    break;
                }
                _ => {
                    window_owned.extend(self.subtree_real_dots(id));
                    referenced.extend(parents);
                    j = k;
                    k += 1;
                }
            }
        }
        let Some(first) = root_children.get(i).map(&child_id) else {
            return self.reproject();
        };
        // Window start: the first child's own marker, or — for a synthetic scaffold at
        // the head — the min position among the real dots it owns. Only a synthetic
        // filler owning no real dots (nothing to localize) falls back to full reproject.
        let Some(mut window_start) = self.seq_flat_pos(first).or_else(|| {
            self.subtree_real_dots(first)
                .iter()
                .filter_map(|&d| self.seq_flat_pos(d))
                .min()
        }) else {
            return self.reproject();
        };
        if let Some(f) = floor {
            window_start = window_start.min(f);
        }
        let mut old_nodes: Vec<Dot> = Vec::new();
        for slot in i..=j {
            match root_children.get(slot) {
                Some(Child::Block(block)) => {
                    if let Some(node) = self.projected.tree.get(*block) {
                        collect_subtree_nodes(&self.projected.tree, node, &mut old_nodes);
                    }
                }
                Some(Child::Leaf { id, .. }) => old_nodes.push(*id),
                None => {}
            }
        }

        let elements = self
            .seq
            .snapshot_range(&self.logs.seq, window_start..window_end);
        for (d, item) in &elements {
            if let SeqItem::Block { node_type, .. } = item {
                if *node_type == NodeType::Root {
                    return Err(ProjectionError::RootTypedBlock { dot: *d }.into());
                }
                if node_type.spec().is_leaf() {
                    return Err(ProjectionError::LeafTypedBlock {
                        dot: *d,
                        node_type: *node_type,
                    }
                    .into());
                }
            }
        }
        let mut window_stats = RepairStats::default();
        let raw = project_blocks(&elements).map_err(ProjectionError::Project)?;
        let raw_children = raw
            .roots
            .into_iter()
            .next()
            .map(|r| r.children)
            .unwrap_or_default();
        // Normalize the whole window forest with the full-document Root algebra —
        // WRAP a context-illegal top-level child (e.g. a bare `ListItem` whose
        // `BulletList` marker was deleted) BEFORE recursing its content, so a surplus
        // child SPLIT-HOISTs to the wrapping scaffold's level exactly as a cold rebuild
        // groups it (per-block `normalize_subtree` under `[Root]` hoisted it one level
        // too high, to Root). The Root *completion* rules stay document-global, applied
        // by `repair_root_content_shallow` below — never re-run per window.
        let forest = normalize_window_forest_with_stats(raw_children, &mut window_stats);
        // Plan each block's index entries and graft its subtree into the live `nodes`
        // map, building the flat child-reference list to splice.
        let mut plan_blocks: Vec<(Dot, Dot, NodeType)> = Vec::new();
        let mut plan_block_leaves: BlockLeafPlan = Vec::new();
        let mut new_nodes: HashSet<Dot> = HashSet::new();
        let mut new_children: Vec<Child> = Vec::new();
        // Block-level atoms (image / horizontal rule) project as a `Leaf` directly under
        // Root; they are leaves of Root, not of any window block, so they need their own
        // index/derivation maintenance (`plan_subtree` only covers block subtrees).
        let mut root_leaves: Vec<(Dot, Option<NodeType>)> = Vec::new();
        for c in forest {
            match c {
                RawChild::Block(b) => {
                    plan_subtree(
                        &b,
                        Dot::ROOT,
                        &mut plan_blocks,
                        &mut plan_block_leaves,
                        &mut new_nodes,
                    );
                    let id = self.projected.tree.insert_block_subtree(&b);
                    new_children.push(Child::Block(id));
                }
                RawChild::Leaf { id, item } => {
                    new_nodes.insert(id);
                    root_leaves.push((id, item.as_child_type()));
                    new_children.push(Child::Leaf { id, item });
                }
            }
        }

        // Snapshot the window's re-projected children before the splice consumes them;
        // the totality tripwire below walks them against the *post-repair* tree.
        let window_children: Vec<Child> = new_children.clone();

        self.projected
            .tree
            .with_block_children(root_id, |children| {
                children.splice(i..=j, new_children);
            });

        for &n in &old_nodes {
            if !new_nodes.contains(&n) {
                self.forget_node(n);
            }
        }

        for (block, parent, nt) in &plan_blocks {
            self.indexes.paths.add_block(*block, *parent, *nt);
            let m = self.logs.block_modifiers.modifiers_of(*block);
            self.projected.set_block_own_modifiers(*block, m);
            let init = block_init_of(&self.projected.tree, *block);
            let base = init.clone().unwrap_or_else(|| nt.into_node());
            match self.logs.node_attrs.project_target(*block, base) {
                Some(node) => {
                    self.projected.node_attrs.insert(*block, node);
                }
                None => {
                    if let Some(seeded) = init {
                        self.projected.node_attrs.insert(*block, seeded);
                    }
                }
            }
        }
        for (block, _, _) in &plan_blocks {
            self.recompute_block_effective(*block);
        }
        // Index every window leaf under its block. The segment index (the sole covering
        // store) is rebuilt below from a single span resolve — no per-leaf coverage to seed.
        for (block, leaves) in &plan_block_leaves {
            for (leaf, _lt) in leaves {
                self.indexes.paths.set_block_of_leaf(*leaf, *block);
            }
        }

        // Root-level block atoms: index them under Root.
        for (leaf, _lt) in &root_leaves {
            self.indexes.paths.set_block_of_leaf(*leaf, Dot::ROOT);
        }

        // Re-establish the Root node's OWN schema content rule (whatever it is —
        // e.g. a required trailing paragraph) by deferring to the normalizer, not
        // by hardcoding the rule here. Window blocks are already normalized, so
        // only the Root's direct content is repaired; returns any freshly scaffolded
        // blocks to index.
        let scaffolded = self.repair_root_content_shallow(&mut window_stats);
        let mut s_blocks: Vec<(Dot, Dot, NodeType)> = Vec::new();
        let mut s_block_leaves: BlockLeafPlan = Vec::new();
        let mut s_nodes: HashSet<Dot> = HashSet::new();
        for b in &scaffolded {
            plan_subtree(
                b,
                Dot::ROOT,
                &mut s_blocks,
                &mut s_block_leaves,
                &mut s_nodes,
            );
        }
        for (block, parent, nt) in &s_blocks {
            self.indexes.paths.add_block(*block, *parent, *nt);
            let m = self.logs.block_modifiers.modifiers_of(*block);
            self.projected.set_block_own_modifiers(*block, m);
        }
        for (block, _, _) in &s_blocks {
            self.recompute_block_effective(*block);
        }
        for (block, leaves) in &s_block_leaves {
            for (leaf, _lt) in leaves {
                self.indexes.paths.set_block_of_leaf(*leaf, *block);
            }
        }

        // Window-local totality tripwire: run the same predicate the full path uses,
        // but on a scratch Root over the window's re-projected children walked against
        // the *post-splice, post-shallow-repair* tree — so the check covers the Root
        // shallow reconcile (a rule-dropped window block leaves its id dangling and its
        // elements unreachable) while the DFS stays O(window), not O(document).
        {
            let mut window_tree = self.projected.tree.clone();
            let window_root = editor_model::synthetic_id(root_id, i, NodeType::Root);
            window_tree.nodes.insert(
                window_root,
                BlockNode {
                    id: window_root,
                    node_type: NodeType::Root,
                    attrs: Vec::new(),
                    children: ChildList::from_iter(window_children),
                },
            );
            window_tree.root = window_root;
            editor_model::totality_check(&elements, &window_tree, &mut window_stats);
        }

        // Rebuild segments for every block the window rewrote — the window
        // subtrees, any scaffolded blocks, and Root itself (its block-atom leaves were
        // re-indexed above). Each leaf's covering is resolved from the span log once here.
        let resolved = editor_model::ResolvedSpans::build(&self.logs.spans, &self.seq);
        let source = CoveringSource::Resolved(&resolved);
        for (block, _, _) in &plan_blocks {
            self.resync_block_segs(*block, &source);
        }
        for (block, _, _) in &s_blocks {
            self.resync_block_segs(*block, &source);
        }
        self.resync_block_segs(Dot::ROOT, &source);
        for (block, _, _) in &plan_blocks {
            self.mark_dirty_block(*block);
        }
        for (block, _, _) in &s_blocks {
            self.mark_dirty_block(*block);
        }
        self.mark_dirty_structural(Dot::ROOT);
        self.repair_stats.accumulate(&window_stats);
        Ok(())
    }

    /// Drop every projected derivation, index entry, and tree node for `n` (a block
    /// or leaf that left the tree). Removing it from `tree.nodes` keeps the flat tree
    /// free of unreachable blocks (a leaf id is simply absent there, a no-op).
    fn forget_node(&mut self, n: Dot) {
        self.projected.block_effective.remove(&n);
        self.projected.block_modifiers.remove(&n);
        self.projected.node_attrs.remove(&n);
        self.projected.node_carries.remove(&n);
        self.projected.seg_index.remove_block(n);
        self.indexes.paths.remove_block(n);
        self.indexes.paths.remove_leaf(n);
        self.projected.tree.nodes.remove(&n);
    }

    /// Re-establish the Root's own schema content rule on the flat tree by running the
    /// nested `normalize_content_shallow` over a shallow view of the root and reconciling
    /// the result back: existing blocks keep their real flat subtree, newly scaffolded
    /// blocks are grafted in (and returned for indexing), and rule-dropped blocks are
    /// forgotten. The last direct Paragraph carries its last schema-known child so
    /// Root-level completion can distinguish a terminal PageBreak from an editable
    /// trailing Paragraph without cloning the Paragraph's full content.
    fn repair_root_content_shallow(&mut self, stats: &mut RepairStats) -> Vec<RawNode> {
        let root_id = self.projected.tree.root;
        let Some(root) = self.projected.tree.get(root_id) else {
            return Vec::new();
        };
        let mut shallow = RawNode {
            id: root_id,
            node_type: root.node_type,
            attrs: root.attrs.clone(),
            children: Vec::new(),
        };
        let last_root_block = last_known_child_for_shallow_root_repair(&self.projected.tree, root)
            .and_then(|child| match child {
                RawChild::Block(block) => Some(block.id),
                RawChild::Leaf { .. } => None,
            });
        for c in &root.children {
            match c {
                Child::Leaf { id, item } => shallow.children.push(RawChild::Leaf {
                    id: *id,
                    item: item.clone(),
                }),
                Child::Block(id) => {
                    let block = self.projected.tree.get(*id);
                    let (nt, attrs) = block
                        .map(|b| (b.node_type, b.attrs.clone()))
                        .unwrap_or((NodeType::Paragraph, Vec::new()));
                    let children = if Some(*id) == last_root_block && nt == NodeType::Paragraph {
                        block
                            .and_then(|block| {
                                last_known_child_for_shallow_root_repair(
                                    &self.projected.tree,
                                    block,
                                )
                            })
                            .into_iter()
                            .collect()
                    } else {
                        Vec::new()
                    };
                    shallow.children.push(RawChild::Block(RawNode {
                        id: *id,
                        node_type: nt,
                        attrs,
                        children,
                    }));
                }
            }
        }
        let before: HashSet<Dot> = shallow
            .children
            .iter()
            .filter_map(|c| match c {
                RawChild::Block(b) => Some(b.id),
                _ => None,
            })
            .collect();

        // Share the SPLIT/WRAP algebra (and its `repairs` count) with the full path.
        // Root accepts every block type so nothing hoists above it, but keep any
        // escapee as a Root child rather than lose it — total, like `normalize_capped`.
        let hoist = normalize_content_shallow_with_stats(&mut shallow, &[], stats);
        shallow.children.extend(hoist);

        let mut new_children: Vec<Child> = Vec::new();
        let mut after: HashSet<Dot> = HashSet::new();
        let mut scaffolded: Vec<RawNode> = Vec::new();
        // Real blocks that a WRAP scaffold reparented (its proxy is an empty
        // placeholder): they moved, they were not dropped — preserved and kept out of
        // the forget set below.
        let mut reparented: HashSet<Dot> = HashSet::new();
        for c in shallow.children {
            match c {
                RawChild::Leaf { id, item } => new_children.push(Child::Leaf { id, item }),
                RawChild::Block(b) => {
                    after.insert(b.id);
                    if self.projected.tree.get(b.id).is_some() {
                        new_children.push(Child::Block(b.id));
                    } else {
                        graft_shallow_scaffold(&mut self.projected.tree, &b, &mut reparented);
                        new_children.push(Child::Block(b.id));
                        scaffolded.push(b);
                    }
                }
            }
        }
        // Forget any direct block the rule dropped (and its whole subtree). The
        // shallow proxy carries no descendants, so the normalize sink can't see this
        // loss; count the dropped subtree's real dots directly from the live tree. A
        // block a scaffold reparented is preserved, not dropped.
        after.extend(reparented.iter().copied());
        let dropped: Vec<Dot> = before.difference(&after).copied().collect();
        for d in dropped {
            stats.drops += self.subtree_real_dots(d).len() as u64;
            let mut sub = Vec::new();
            if let Some(b) = self.projected.tree.get(d) {
                collect_subtree_nodes(&self.projected.tree, b, &mut sub);
            }
            for n in sub {
                self.forget_node(n);
            }
        }
        self.projected
            .tree
            .with_block_children(root_id, |children| {
                *children = ChildList::from_iter(new_children);
            });
        scaffolded
    }

    fn reproject_from_tree(&mut self) -> Result<(), SpineError> {
        let elements = self.seq.snapshot(&self.logs.seq);
        let tree = self.projected.tree.clone();
        self.projected = project_from_tree(&elements, tree, &self.seq, &self.logs);
        self.indexes = ProjectionIndexes::rebuild_from(&self.projected, &self.logs.spans);
        self.mark_dirty_full();
        Ok(())
    }

    fn try_incremental(&mut self, op: &Op<EditOp>) -> bool {
        match &op.payload {
            EditOp::Seq(ListOp::Ins { item, .. }) if is_inline_leaf_item(item) => {
                self.try_insert_leaf(op.id, item)
            }
            EditOp::Seq(ListOp::Ins {
                item:
                    SeqItem::Block {
                        node_type,
                        parents,
                        attrs,
                    },
                ..
            }) => attrs.is_empty() && self.try_insert_block(op.id, *node_type, parents),
            EditOp::Seq(ListOp::Del { .. }) => self.try_delete_chars(op.id),
            EditOp::Seq(ListOp::Undel { del }) => self.try_undelete(*del),
            EditOp::Span(span_op) => self.try_apply_span(op.id, span_op),
            EditOp::BlockModifier(_) | EditOp::NodeAttr(_) | EditOp::NodeCarry(_) => {
                self.try_apply_node_op(op)
            }
            // `warm_dispatch` already folded this op into `logs.aliases` and
            // `projected.alias_classes` — no seq/tree change follows, so there is
            // nothing left to project.
            EditOp::Alias(_) => true,
            _ => false,
        }
    }

    fn node_base_of(&self, dot: Dot) -> Option<Node> {
        if self.indexes.paths.block_of(dot).is_some() {
            return self
                .atom_leaf(dot)
                .cloned()
                .map(AtomLeaf::into_node)
                .or_else(|| self.leaf_type_of(dot).map(NodeType::into_node));
        }
        self.indexes
            .paths
            .node_type_of(dot)
            .map(|nt| block_init_of(&self.projected.tree, dot).unwrap_or_else(|| nt.into_node()))
    }

    fn recompute_block_effective(&mut self, block: Dot) {
        let be = block_effective_one(&self.indexes.paths, &self.logs, &self.projected, block);
        self.projected.set_block_effective(block, be);
    }

    /// The leaf ordinal (count of leaf children before it) of `leaf` in `block`, or
    /// `None` if it isn't a leaf child there. `O(K)` walk over the block's children.
    fn leaf_ordinal_of(&self, block: Dot, leaf: Dot) -> Option<usize> {
        let node = self.projected.tree.get(block)?;
        let mut ord = 0usize;
        for c in node.children.iter() {
            match c {
                Child::Leaf { id, .. } if *id == leaf => return Some(ord),
                Child::Leaf { .. } => ord += 1,
                Child::Block(_) => {}
            }
        }
        None
    }

    /// Rebuild `block`'s segment index from its current leaf children, deriving
    /// `(eff, own)` self-sufficiently via `derive_seg_state` from each leaf's covering
    /// key (from `source`) plus its node style/attrs. Segments coalesce exactly as cold
    /// `segment_block` does (same `(leaf_type, style, covering)` key, singleton criterion,
    /// and LRU-1 derive memo). `O(K)` in the block's leaf count (plus, under
    /// [`CoveringSource::Resolved`], `O(spans)` per leaf to stab the resolve). Used by the
    /// structural rebuild paths (window reproject, subtree recompute, block split, undelete).
    fn resync_block_segs(&mut self, block: Dot, source: &CoveringSource) {
        struct Memo {
            leaf_type: NodeType,
            covering: Option<editor_model::SegCovering>,
            eff: editor_model::LeafEff,
            own: editor_model::LeafOwn,
        }
        let segs = {
            let Some(node) = self.projected.tree.get(block) else {
                self.projected.seg_index.remove_block(block);
                return;
            };
            let mut segs: Vec<editor_model::Seg> = Vec::new();
            let mut memo: Option<Memo> = None;
            let mut leaf_ord = 0usize;
            for c in node.children.iter() {
                let Child::Leaf { id, item } = c else {
                    continue;
                };
                let dot = *id;
                let leaf_type = item.as_child_type().unwrap_or(NodeType::Unknown);
                let covering = match source {
                    CoveringSource::Resolved(rs) => self
                        .seq
                        .resolve_boundary(dot, Bias::Before)
                        .map(|b| b.position)
                        .and_then(|p| canonical_covering_of(&rs.covering(p), &self.logs.spans)),
                    // The covering is unchanged by this op, so reuse the leaf's current
                    // segment covering (all leaves of a coalesced segment share it).
                    CoveringSource::Existing => self
                        .projected
                        .seg_index
                        .seg_at(block, leaf_ord)
                        .and_then(|(s, _)| s.covering.clone()),
                };
                leaf_ord += 1;
                let attrs_singleton =
                    self.projected.node_attrs.contains_key(&dot) || !leaf_type.spec().inline;
                let (eff, own) = if attrs_singleton {
                    editor_model::derive_seg_state(
                        &self.indexes.paths,
                        &self.logs,
                        &self.projected,
                        block,
                        leaf_type,
                        covering.as_deref(),
                        Some(dot),
                    )
                } else {
                    match &memo {
                        Some(m) if m.leaf_type == leaf_type && m.covering == covering => {
                            (m.eff.clone(), m.own.clone())
                        }
                        _ => {
                            let d = editor_model::derive_seg_state(
                                &self.indexes.paths,
                                &self.logs,
                                &self.projected,
                                block,
                                leaf_type,
                                covering.as_deref(),
                                None,
                            );
                            memo = Some(Memo {
                                leaf_type,
                                covering: covering.clone(),
                                eff: d.0.clone(),
                                own: d.1.clone(),
                            });
                            d
                        }
                    }
                };
                let seg = editor_model::Seg {
                    count: 1,
                    leaf_type,
                    covering,
                    attrs_singleton,
                    eff,
                    own,
                };
                match segs.last_mut() {
                    Some(last) if last.key_eq(&seg) => last.count += 1,
                    _ => segs.push(seg),
                }
            }
            segs
        };
        self.projected.seg_index.set_block(block, segs);
    }

    /// Recompute the projected derivations (block_effective and the segment index)
    /// for `target` and everything that inherits from it. A leaf target rebuilds its
    /// block's segments; a block target is its whole subtree (over-invalidation is
    /// safe — only derivations recompute, never structure). O(subtree).
    fn recompute_subtree(&mut self, target: Dot) {
        let tb = self.indexes.paths.block_of(target).unwrap_or(target);
        self.mark_dirty_block(tb);
        if let Some(block) = self.indexes.paths.block_of(target) {
            if self.leaf_type_of(target).is_some() {
                self.resync_block_segs(block, &CoveringSource::Existing);
            }
            return;
        }
        self.recompute_block_effective(target);
        let mut affected_blocks: HashSet<Dot> = HashSet::new();
        for d in self.indexes.paths.descendants_of(target) {
            if let Some(b) = self.indexes.paths.block_of(d) {
                if self.leaf_type_of(d).is_some() {
                    affected_blocks.insert(b);
                }
            } else {
                self.recompute_block_effective(d);
                self.mark_dirty_block(d);
            }
        }
        for b in affected_blocks {
            self.mark_dirty_block(b);
            self.resync_block_segs(b, &CoveringSource::Existing);
        }
    }

    fn try_apply_node_op(&mut self, op: &Op<EditOp>) -> bool {
        match &op.payload {
            EditOp::BlockModifier(o) => {
                let target = o.target_key().0;
                if !self.indexes.paths.contains(target) {
                    return true;
                }
                let m = self.logs.block_modifiers.modifiers_of(target);
                self.projected.set_block_own_modifiers(target, m);
                self.recompute_subtree(target);
            }
            EditOp::NodeAttr(o) => {
                let target = o.target;
                if !self.indexes.paths.contains(target) {
                    return true;
                }
                let Some(base) = self.node_base_of(target) else {
                    return true;
                };
                match self.logs.node_attrs.project_target(target, base) {
                    Some(node) => {
                        self.projected.node_attrs.insert(target, node);
                    }
                    None => match block_init_of(&self.projected.tree, target) {
                        Some(seeded) => {
                            self.projected.node_attrs.insert(target, seeded);
                        }
                        None => {
                            self.projected.node_attrs.remove(&target);
                        }
                    },
                }
                self.recompute_subtree(target);
            }
            EditOp::NodeCarry(o) => {
                let target = o.target_key().0;
                if !self.indexes.paths.contains(target) {
                    return true;
                }
                // Carries don't participate in effective/own derivation. The
                // projection is the final guard against inflow: drop non-carry
                // kinds and records whose target is not a text block.
                let is_textblock = self
                    .indexes
                    .paths
                    .node_type_of(target)
                    .map(|nt| nt.spec().is_textblock())
                    .unwrap_or(false);
                let carries: std::collections::BTreeMap<_, _> = self
                    .logs
                    .node_carries
                    .modifiers_of(target)
                    .into_iter()
                    .filter(|(ty, _)| ty.is_carry_kind())
                    .collect();
                if is_textblock && !carries.is_empty() {
                    self.projected.node_carries.insert(target, carries);
                } else {
                    self.projected.node_carries.remove(&target);
                }
                self.mark_dirty_block(target);
            }
            _ => return false,
        }
        true
    }

    fn leaf_type_of(&self, dot: Dot) -> Option<NodeType> {
        let lv = *self.logs.seq.lv_of.get(&dot)?;
        match &self.logs.seq.entries[lv].op {
            ListOp::Ins { item, .. } => item.as_child_type(),
            _ => None,
        }
    }

    fn span_covers(&self, span_dot: Dot, pos: usize) -> bool {
        let Some(op) = self.logs.spans.get(span_dot) else {
            return false;
        };
        let (sa, ea) = op.anchors();
        let (Some(s), Some(e)) = (
            self.seq
                .resolve_boundary(sa.id, sa.bias.into())
                .map(|b| b.position),
            self.seq
                .resolve_boundary(ea.id, ea.bias.into())
                .map(|b| b.position),
        ) else {
            return false;
        };
        s < e && s <= pos && pos < e
    }

    /// The segment covering (per-type LWW winners) for a leaf newly inserted at visible
    /// position `pos`, whose left neighbor is `neighbor`. Seeds from a nearby leaf's
    /// SEGMENT covering — for a mid-block insert that is `neighbor`'s own segment, passed
    /// in as `neighbor_seg_cov` so the hot path stays `O(neighbor covering + adjacent
    /// spans)` with no `O(K)` ordinal walk. Equals the seed except for spans anchored
    /// adjacent to the insertion gap: those covering `pos` are folded in; a seed winner
    /// EXCLUDED at `pos` forces a full resolve for that position, since winners-only
    /// seeding can't reveal a runner-up the excluded winner was hiding.
    fn covering_for_inserted(
        &self,
        neighbor: Dot,
        pos: usize,
        neighbor_seg_cov: Option<editor_model::SegCovering>,
    ) -> Option<editor_model::SegCovering> {
        if self.indexes.spans.is_empty() {
            return None;
        }
        let after = self.seq.dot_at_visible(&self.logs.seq, pos + 1);
        // Seed from a nearby leaf's segment covering: the left neighbour for a mid-block
        // insert, the right neighbour for a block-start insert into a non-empty block,
        // else the nearest leaf a bounded walk to the left (an empty block). A span
        // without an anchor at any element between the seed and `pos` covers the seed iff
        // it covers `pos` — every span boundary between them must be anchored to one of
        // the passed-over elements or gap tombstones (the new leaf cannot anchor
        // pre-existing spans), and those are collected into `near` and re-tested below.
        // Only when no seed is reachable does the whole-span-log resolve remain.
        let mut near = vec![neighbor];
        if let Some(m) = after {
            near.push(m);
        }
        let seed_cov = if self.indexes.paths.block_of(neighbor).is_some() {
            neighbor_seg_cov
        } else if let Some(a) = after.filter(|a| self.indexes.paths.block_of(*a).is_some()) {
            // Right-neighbour seed: a local insert lands before any tombstones at
            // its boundary, so ghosts pile up between the new leaf and `after`.
            near.extend(self.seq.invisible_dots_after_visible(pos));
            self.seg_covering_of_leaf(a)
        } else {
            match self.left_seed_across_markers(pos, &mut near) {
                Some(s) => self.seg_covering_of_leaf(s),
                None => return self.full_covering_at(pos),
            }
        };
        let boundary = self.indexes.spans.spans_near(near);
        if boundary.is_empty() {
            return seed_cov;
        }
        let mut cov = seed_cov.clone();
        for &s in &boundary {
            let Some(op) = self.logs.spans.get(s) else {
                continue;
            };
            let ty = editor_model::covering_of_op(op);
            if self.span_covers(s, pos) {
                if let Some(next) = editor_model::covering_absorb(cov.as_ref(), ty, s) {
                    cov = Some(next);
                }
            } else if seed_cov.as_ref().and_then(|c| c.get(&ty)) == Some(&s) {
                // The seed's winner for `ty` is excluded at `pos`. The winners-only seed
                // dropped any runner-up of `ty`, and a runner-up covering the seed with no
                // anchor between it and `pos` still covers `pos` — only a full resolve can
                // recover it. Rare (a boundary span must be a seed winner AND miss `pos`).
                return self.full_covering_at(pos);
            }
        }
        cov
    }

    /// The segment covering of a leaf, read from its block's segment index. `O(K)` in the
    /// block leaf count (the ordinal walk) — used only for non-neighbour insert seeds
    /// (block-start / empty-block), never the mid-block typing hot path.
    fn seg_covering_of_leaf(&self, leaf: Dot) -> Option<editor_model::SegCovering> {
        let block = self.indexes.paths.block_of(leaf)?;
        let ord = self.leaf_ordinal_of(block, leaf)?;
        self.projected
            .seg_index
            .seg_at(block, ord)
            .and_then(|(s, _)| s.covering.clone())
    }

    /// The authoritative segment covering at visible position `pos`: canonicalize the
    /// winners of every span stabbing `pos`. `O(spans · log)` — the fallback when a
    /// nearby seed can't be trusted.
    fn full_covering_at(&self, pos: usize) -> Option<editor_model::SegCovering> {
        canonical_covering_of(
            &editor_model::spans_covering(pos, &self.logs.spans, &self.seq),
            &self.logs.spans,
        )
    }

    fn left_seed_across_markers(&self, pos: usize, near: &mut Vec<Dot>) -> Option<Dot> {
        const MAX_WALK: usize = 32;
        let mut p = pos.checked_sub(1)?;
        for _ in 0..MAX_WALK {
            p = p.checked_sub(1)?;
            near.extend(self.seq.invisible_dots_after_visible(p));
            let d = self.seq.dot_at_visible(&self.logs.seq, p)?;
            near.push(d);
            if self.indexes.paths.block_of(d).is_some() {
                return Some(d);
            }
        }
        None
    }

    fn try_apply_span(&mut self, op_dot: Dot, span_op: &SpanOp) -> bool {
        self.indexes.spans.add(op_dot, span_op);
        let (sa, ea) = span_op.anchors();
        let (Some(start), Some(end)) = (
            self.seq
                .resolve_boundary(sa.id, sa.bias.into())
                .map(|b| b.position),
            self.seq
                .resolve_boundary(ea.id, ea.bias.into())
                .map(|b| b.position),
        ) else {
            return true;
        };
        if start >= end {
            return true;
        }
        let ty = editor_model::covering_of_op(span_op);
        let count = end - start;
        // Group the covered leaves by block in position order — the segment index is the
        // sole covering store, so nothing per-leaf is touched. Covered leaves of one block
        // are contiguous in leaf ordinal, so the group doubles as the block's `[lo, hi)`
        // apply range and its singleton dots.
        let mut groups: Vec<(Dot, Vec<Dot>)> = Vec::new();
        let mut group_of: HashMap<Dot, usize> = HashMap::new();
        let mut group_for = |block: Dot, groups: &mut Vec<(Dot, Vec<Dot>)>| -> usize {
            *group_of.entry(block).or_insert_with(|| {
                groups.push((block, Vec::new()));
                groups.len() - 1
            })
        };
        if count >= 64 {
            // Stream the visible sequence once, resolving each covered leaf to its block.
            // Under total projection an inline run can be SPLIT across synthetic scaffold
            // blocks (a mid-text PageBreak hoists its tail into new Paragraphs whose
            // markers are not in the sequence), so a leaf cannot inherit the block of the
            // last sequence marker — every covered leaf is attributed by its own
            // `block_of`. The order invariant keeps each block's leaves contiguous, so
            // `group_for` still yields one contiguous group per block.
            for (dot, item) in self
                .seq
                .iter_visible(&self.logs.seq)
                .skip(start)
                .take(count)
            {
                if matches!(item, SeqItem::Block { .. }) {
                    continue;
                }
                let Some(block) = self.indexes.paths.block_of(dot) else {
                    continue;
                };
                let gi = group_for(block, &mut groups);
                groups[gi].1.push(dot);
            }
        } else {
            // A short range (a styled keystroke's span touches one leaf) resolves each
            // position directly — `O(count · log)`, no full-sequence stream.
            for pos in start..end {
                let Some(leaf) = self.seq.dot_at_visible(&self.logs.seq, pos) else {
                    continue;
                };
                let Some(block) = self.indexes.paths.block_of(leaf) else {
                    continue;
                };
                let gi = group_for(block, &mut groups);
                groups[gi].1.push(leaf);
            }
        }
        // Fold `op_dot` into each covered block's segment coverings via a per-block
        // range apply, re-deriving `(eff, own)` from the new covering key. `apply_range`
        // splits `[lo, hi)` onto segment boundaries, so each covered segment already has
        // a distinct key — no derive memo can hit within one op. A covering-only change
        // (derived state unchanged, e.g. a neutral Remove) still rewrites the segment
        // but does NOT dirty the block. Take the index out so the derive closure can
        // borrow the rest of `self` immutably.
        let mut seg_index = std::mem::take(&mut self.projected.seg_index);
        let mut dirty_blocks: Vec<Dot> = Vec::new();
        for (block, leaves) in &groups {
            let lo = self
                .leaf_ordinal_of(*block, leaves[0])
                .expect("covered leaf has an ordinal in its block");
            let hi = lo + leaves.len();
            let mut block_dirty = false;
            seg_index.apply_range(*block, lo, hi, &mut |seg, start_ord| {
                let new_cov = editor_model::covering_absorb(seg.covering.as_ref(), ty, op_dot)?;
                // A singleton derives against its real leaf dot; the seg spans a single
                // covered ordinal, so `leaves[start_ord - lo]` is that leaf.
                let attr_leaf = seg
                    .attrs_singleton
                    .then(|| leaves.get(start_ord - lo).copied())
                    .flatten();
                let (eff, own) = editor_model::derive_seg_state(
                    &self.indexes.paths,
                    &self.logs,
                    &self.projected,
                    *block,
                    seg.leaf_type,
                    Some(&new_cov),
                    attr_leaf,
                );
                // Layout depends only on `effective`; an own-only change (e.g. an
                // inline modifier suppressed by the leaf's target, like italic in a
                // fold title) updates the segment but must not dirty layout.
                if *eff != *seg.eff {
                    block_dirty = true;
                }
                Some(editor_model::Seg {
                    covering: Some(new_cov),
                    eff,
                    own,
                    ..seg.clone()
                })
            });
            if block_dirty {
                dirty_blocks.push(*block);
            }
        }
        self.projected.seg_index = seg_index;
        for block in dirty_blocks {
            self.mark_dirty_block(block);
        }
        true
    }

    fn is_inline_leaf(&self, dot: Dot) -> bool {
        self.logs.seq.lv_of.get(&dot).is_some_and(|&lv| {
            matches!(&self.logs.seq.entries[lv].op, ListOp::Ins { item, .. } if is_inline_leaf_item(item))
        })
    }

    fn try_insert_leaf(&mut self, leaf: Dot, item: &SeqItem) -> bool {
        let Some(pos) = self
            .seq
            .resolve_boundary(leaf, Bias::Before)
            .map(|b| b.position)
        else {
            return false;
        };
        if pos == 0 {
            return false;
        }
        let Some(neighbor) = self.seq.dot_at_visible(&self.logs.seq, pos - 1) else {
            return false;
        };
        let leaf_type = item.as_child_type().unwrap_or(NodeType::Unknown);
        // The new leaf's parent block: the neighbor's block if the neighbor is a
        // leaf, or — when inserting at a block's start — the neighbor block itself.
        let (block, neighbor_is_leaf) = match self.indexes.paths.block_of(neighbor) {
            Some(b) => (b, true),
            None if self.indexes.paths.node_type_of(neighbor).is_some() => (neighbor, false),
            None => {
                return false;
            }
        };
        // A synthetic (hoist/scaffold-derived) block's sequence boundaries are set by
        // normalization's SPLIT, not by a contiguous authored run, so extending it with
        // an incremental splice can group content differently than a full reprojection
        // would. Reproject the window instead so the grouping stays consistent.
        if block.is_synthetic() {
            return false;
        }
        // Splice incrementally only when no normalization can be triggered: the new
        // leaf must be freely-repeatable content of its block, and (for a mid-block
        // insert) so must its left neighbor — otherwise the splice could place
        // content after a position-constrained element (e.g. a trailing PageBreak),
        // which projection would normalize away. Such inserts fall back.
        let Some(block_type) = self.indexes.paths.node_type_of(block) else {
            return false;
        };
        let content = &block_type.spec().content;
        if !content.is_repeatable(leaf_type) {
            return false;
        }
        if neighbor_is_leaf
            && !self
                .leaf_type_of(neighbor)
                .is_some_and(|lt| content.is_repeatable(lt))
        {
            return false;
        }
        // Place the new leaf right after its sequence neighbor among the block's
        // children. A sequential run hits the cursor (O(log K) identity check, no
        // `resolve_boundary`); anything else falls back to the binary search.
        let offset = self.leaf_insert_offset_cursored(block, neighbor, neighbor_is_leaf, pos);
        // The left neighbor's segment (the authoritative store): its effective map, whether
        // its own map is empty, and its covering — seed for `covering_for_inserted`. The
        // neighbor is the last leaf before `offset`, so its ordinal is
        // `leaf_ordinal_at(offset) - 1` — `O(log K)`, no full scan.
        let neighbor_seg: Option<(
            editor_model::LeafEff,
            bool,
            Option<editor_model::SegCovering>,
        )> = if neighbor_is_leaf {
            self.projected
                .tree
                .get(block)
                .map(|n| n.children.leaf_ordinal_at(offset))
                .and_then(|o| o.checked_sub(1))
                .and_then(|o| self.projected.seg_index.seg_at(block, o))
                .map(|(s, _)| (s.eff.clone(), s.own.is_empty(), s.covering.clone()))
        } else {
            None
        };
        let neighbor_plain = neighbor_seg
            .as_ref()
            .is_some_and(|(_, own_empty, _)| *own_empty);
        let neighbor_cov = neighbor_seg.as_ref().and_then(|(_, _, c)| c.clone());
        // Segment key inputs mirror cold `segment_block`: covering seeded from the
        // neighbour's segment, singleton per node_attrs/non-inline.
        let covering = self.covering_for_inserted(neighbor, pos, neighbor_cov);
        let attrs_singleton =
            self.projected.node_attrs.contains_key(&leaf) || !leaf_type.spec().inline;
        let (eff, own) = if covering.is_none() && neighbor_plain && !attrs_singleton {
            // L and its leaf neighbor are both plain leaves of the same block:
            // identical pure-inheritance effective and no own modifiers. Reusing the
            // neighbor's handle keeps the whole run on one shared map.
            (
                neighbor_seg.map(|(e, _, _)| e).unwrap_or_default(),
                editor_model::LeafOwn::default(),
            )
        } else {
            editor_model::derive_seg_state(
                &self.indexes.paths,
                &self.logs,
                &self.projected,
                block,
                leaf_type,
                covering.as_deref(),
                attrs_singleton.then_some(leaf),
            )
        };
        self.projected
            .splice_char(block, offset, leaf, item.clone());
        self.indexes.paths.set_block_of_leaf(leaf, block);
        // Splice the leaf's segment into `block` at its leaf ordinal, joining an
        // adjacent same-key segment.
        if let Some(ordinal) = self
            .projected
            .tree
            .get(block)
            .map(|n| n.children.leaf_ordinal_at(offset))
        {
            self.projected.seg_index.insert_leaf(
                block,
                ordinal,
                editor_model::Seg {
                    count: 1,
                    leaf_type,
                    covering,
                    attrs_singleton,
                    eff,
                    own,
                },
            );
        }
        self.leaf_cursor = Some(LeafCursor {
            block,
            leaf,
            offset,
        });
        self.mark_dirty_block(block);
        true
    }

    /// The child offset at which a leaf landing after `neighbor` (sequence position
    /// `pos`) belongs in `block`. Sequential inserts (paste, typing) place each leaf
    /// right after the previous one, so the cursor remembers the last
    /// `(block, leaf, offset)` and — when this leaf's neighbor is exactly that leaf and
    /// it still sits where we left it — returns `offset + 1` after a single `O(log K)`
    /// identity check, with no `resolve_boundary`. The validating `get` keeps it correct
    /// across any intervening edit (a stale cursor simply misses). A miss — block start,
    /// the first insert, or a non-sequential/remote edit — falls back to the binary
    /// search. Tree-identity based, so ghost-safe like the fallback.
    fn leaf_insert_offset_cursored(
        &self,
        block: Dot,
        neighbor: Dot,
        neighbor_is_leaf: bool,
        pos: usize,
    ) -> usize {
        if !neighbor_is_leaf {
            // The neighbor is the block marker itself: the new leaf is the first child.
            return 0;
        }
        if let Some(cur) = self.leaf_cursor.as_ref()
            && cur.block == block
            && cur.leaf == neighbor
            && self
                .projected
                .tree
                .get(block)
                .and_then(|n| n.children.get(cur.offset))
                .is_some_and(|c| matches!(c, Child::Leaf { id, .. } if *id == neighbor))
        {
            return cur.offset + 1;
        }
        self.leaf_insert_offset(block, pos)
    }

    /// The child offset at which a leaf landing at visible sequence position `pos`
    /// belongs in `block`. A block keeps its leaves in sequence order, so this binary-
    /// searches the children by their resolved position — `O(log² K)` and ghost-safe
    /// (only live tree leaves are weighed), independent of where in the block the insert
    /// lands (append, middle, or random).
    fn leaf_insert_offset(&self, block: Dot, pos: usize) -> usize {
        let Some(node) = self.projected.tree.get(block) else {
            return 0;
        };
        let (mut lo, mut hi) = (0usize, node.children.len());
        while lo < hi {
            let mid = (lo + hi) / 2;
            let child_dot = match node.children.get(mid) {
                Some(Child::Leaf { id, .. }) => *id,
                Some(Child::Block(d)) => *d,
                None => break,
            };
            let before = self
                .seq
                .resolve_boundary(child_dot, Bias::Before)
                .is_some_and(|b| b.position < pos);
            if before {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }

    // Incremental block insert for the common, normalization-free case: a new
    // top-level leaf-bearing block (e.g. pressing Enter to split a paragraph).
    // The split block and the new block must be the same simple inline-only block
    // type with no required content, so every prefix/suffix split stays valid.
    // Anything else (nested blocks, type changes, required content) falls back.
    fn try_insert_block(&mut self, block_dot: Dot, node_type: NodeType, parents: &[Dot]) -> bool {
        if parents != [Dot::ROOT] {
            return false;
        }
        let Some(pos) = self
            .seq
            .resolve_boundary(block_dot, Bias::Before)
            .map(|b| b.position)
        else {
            return false;
        };
        if pos == 0 {
            return false;
        }
        let Some(left) = self.seq.dot_at_visible(&self.logs.seq, pos - 1) else {
            return false;
        };
        let (p_block, split_after) = match self.indexes.paths.block_of(left) {
            Some(pb) => (pb, Some(left)),
            None if self.indexes.paths.node_type_of(left).is_some() => (left, None),
            None => return false,
        };
        if self.indexes.paths.parent_of(p_block) != Some(Dot::ROOT) {
            return false;
        }
        let Some(p_type) = self.indexes.paths.node_type_of(p_block) else {
            return false;
        };
        // Same simple inline-only block type, no required content → split is valid.
        let content = &p_type.spec().content;
        let simple = node_type == p_type
            && content.min_required() == 0
            && content.allowed_types().iter().all(|t| t.spec().inline);
        if !simple {
            return false;
        }
        let Some(moved) = split_block_insert(
            &mut self.projected.tree,
            Dot::ROOT,
            p_block,
            split_after,
            block_dot,
            node_type,
        ) else {
            return false;
        };
        self.indexes
            .paths
            .add_block(block_dot, Dot::ROOT, node_type);
        for &leaf in &moved {
            self.indexes.paths.set_block_of_leaf(leaf, block_dot);
        }
        self.recompute_block_effective(block_dot);
        // The split rewrote p_block's suffix and the new block from scratch — rebuild
        // both blocks' segments from their current leaves. p_block only needs a rebuild
        // when leaves actually moved out; the paste / Enter-at-end case (`moved` empty)
        // leaves its incrementally-built segments already correct. The moved leaves have no
        // segment in the new block yet, so their covering is resolved from the span log.
        let resolved = editor_model::ResolvedSpans::build(&self.logs.spans, &self.seq);
        let source = CoveringSource::Resolved(&resolved);
        if !moved.is_empty() {
            self.resync_block_segs(p_block, &source);
        }
        self.resync_block_segs(block_dot, &source);
        self.mark_dirty_block(p_block);
        self.mark_dirty_block(block_dot);
        self.mark_dirty_structural(Dot::ROOT);
        true
    }

    fn try_delete_chars(&mut self, del: Dot) -> bool {
        // A bulk delete that removes more than remains (select-all-delete: one range op
        // clears ~everything) is cheaper to rebuild wholesale — `O(visible-after)` — than
        // to splice each removed leaf out of the projection incrementally, `O(targets)`.
        // The seq op is already applied, so `visible_len()` is the post-delete count. The
        // `1024` floor keeps ordinary small deletes on the localized incremental path
        // (whose cost is below the reproject's fixed index-rebuild overhead). Decide on the
        // O(1) target count so the bulk path never materializes the O(len) target vector.
        let target_count = self.seq.del_target_count(del);
        if target_count == 0 {
            return false;
        }
        if target_count > self.seq.visible_len().max(1024) {
            return self.reproject_after_delete().is_ok();
        }
        let targets = self.seq.del_target_dots(&self.logs.seq, del);
        if targets.is_empty() {
            return false;
        }
        let mut block = None;
        for &t in &targets {
            let Some(b) = self.indexes.paths.block_of(t) else {
                return false;
            };
            // A synthetic (hoist/scaffold-derived) block's boundaries come from
            // normalization's SPLIT; removing a leaf from it can change how the
            // surrounding run regroups. Reproject the window so it matches cold.
            if b.is_synthetic() {
                return false;
            }
            if !self.is_inline_leaf(t) {
                return false;
            }
            // Deleting a position-constrained leaf (e.g. a trailing PageBreak) can
            // un-drop content that normalization previously removed — a non-local
            // effect a plain leaf removal can't reproduce. Fall back so the block is
            // re-projected from the sequence, restoring it locally.
            let constrained = self
                .leaf_type_of(t)
                .zip(self.indexes.paths.node_type_of(b))
                .is_some_and(|(lt, bt)| !bt.spec().content.is_repeatable(lt));
            if constrained {
                return false;
            }
            match block {
                None => block = Some(b),
                Some(bb) if bb == b => {}
                _ => return false,
            }
        }
        let block = block.expect("non-empty targets set block");
        self.mark_dirty_block(block);
        for &t in &targets {
            // Bridge: capture the leaf's ordinal before the splice removes it, then
            // drop that segment position after — keeping tree and segments in step.
            let ordinal = self.leaf_ordinal_of(block, t);
            if !self.projected.splice_delete_leaf(block, t) {
                return false;
            }
            self.indexes.paths.remove_leaf(t);
            if let Some(o) = ordinal {
                self.projected.seg_index.remove_leaf(block, o);
            }
        }
        true
    }

    /// Restore the leaves a prior delete removed (the inverse op an undo replays).
    /// Re-inserts each restored inline leaf incrementally — the same localized path a
    /// fresh insert takes — instead of a per-Undel reprojection (the `O(N²)` undo of a
    /// large delete). Falls back when a target isn't a plain inline leaf (block markers,
    /// position-constrained content) so the structural cases stay correct.
    fn try_undelete(&mut self, del: Dot) -> bool {
        let targets = self.seq.del_target_dots(&self.logs.seq, del);
        if targets.is_empty() {
            return false;
        }
        // Collect the targets this undel actually re-shows (a concurrently-deleted
        // target stays invisible — restoring it would diverge from a reproject), as
        // inline leaves, ordered by visible position so each insert sees its left
        // neighbour. Restore in ascending order.
        let mut ordered: Vec<(usize, Dot, SeqItem)> = Vec::with_capacity(targets.len());
        for &t in &targets {
            let Some(b) = self.seq.resolve_boundary(t, Bias::Before) else {
                return false;
            };
            if !b.visible {
                continue;
            }
            let Some(&lv) = self.logs.seq.lv_of.get(&t) else {
                return false;
            };
            let ListOp::Ins { item, .. } = &self.logs.seq.entries[lv].op else {
                return false;
            };
            if !is_inline_leaf_item(item) {
                return false;
            }
            ordered.push((b.position, t, item.clone()));
        }
        if ordered.is_empty() {
            return false;
        }
        ordered.sort_by_key(|(p, _, _)| *p);
        for (_, t, item) in &ordered {
            if !self.try_insert_leaf(*t, item) {
                return false;
            }
        }
        // The incremental insert seeds each restored leaf's covering from its neighbour,
        // which misses spans anchored to the restored leaf itself. Once all are back (final
        // positions), rebuild each affected block's segments from a single span resolve —
        // sourcing every leaf's covering authoritatively, fixing the self-anchored spans.
        let resolved = editor_model::ResolvedSpans::build(&self.logs.spans, &self.seq);
        let source = CoveringSource::Resolved(&resolved);
        let mut blocks: Vec<Dot> = Vec::new();
        for (_, t, _) in &ordered {
            if let Some(b) = self.indexes.paths.block_of(*t)
                && !blocks.contains(&b)
            {
                blocks.push(b);
            }
        }
        for b in blocks {
            self.resync_block_segs(b, &source);
        }
        true
    }

    pub fn apply(&mut self, payload: EditOp) -> Result<Op<EditOp>, SpineError> {
        let op = self.apply_op_warm(payload)?;
        self.project_op(&op)?;
        Ok(op)
    }

    /// Apply an op to the sequence/oplog WITHOUT projecting it. For callers that apply
    /// a run of ops back-to-back and can defer the projection to a single
    /// [`reproject_all`](Self::reproject_all) afterward — e.g. undoing a large delete,
    /// where projecting each re-inserted block as its own window reprojection is
    /// `O(ops · window)` but one final reproject is `O(document)`. Only safe when nothing
    /// between the ops reads the projection (seq inversion reads the checkout, not the
    /// projected tree).
    pub fn apply_warm_only(&mut self, payload: EditOp) -> Result<Op<EditOp>, SpineError> {
        self.apply_op_warm(payload)
    }

    /// Force a whole-document reprojection from the current sequence. Pairs with
    /// [`apply_warm_only`](Self::apply_warm_only) to collapse a batch of deferred ops
    /// into one projection.
    pub fn reproject_all(&mut self) -> Result<(), SpineError> {
        self.reproject()
    }

    pub fn apply_batch(&mut self, payloads: Vec<EditOp>) -> Result<Vec<Op<EditOp>>, SpineError> {
        let mut ops = Vec::with_capacity(payloads.len());
        for payload in payloads {
            match self.apply_op_warm(payload) {
                Ok(op) => {
                    self.project_op(&op)?;
                    ops.push(op);
                }
                Err(e) => {
                    self.rebuild_from_graph()?;
                    return Err(e);
                }
            }
        }
        Ok(ops)
    }

    pub fn commit(&mut self) {
        self.graph.commit_mut();
    }

    pub fn receive_changeset(&self, cs: Changeset<EditOp>) -> Result<Self, SpineError> {
        let (next, _applied) = self.receive_changesets(vec![cs])?;
        Ok(next)
    }

    /// Apply a batch of remote changesets against a single cloned state. A sync
    /// burst delivers many changesets at once (one payload, or many that pile up
    /// in the message queue before a tick drains them); receiving each one
    /// separately re-clones the whole `ProjectedState` — the `O(N)` `lv_of`
    /// HashMap and the seq `ContentTree` — per changeset, so `K` changesets cost
    /// `O(K·N)`. Cloning once and folding every changeset into that clone drops it
    /// to `O(N + novel)`.
    pub fn receive_changesets(
        &self,
        css: Vec<Changeset<EditOp>>,
    ) -> Result<(Self, Vec<Op<EditOp>>), SpineError> {
        let mut next = self.clone();
        // The novel ops are exactly the changesets' ops we don't already have,
        // accumulated in receive order. A Changeset is stored ancestry-first
        // (parents before children) and later changesets in the batch can only
        // depend on earlier ones (or the base graph), so the concatenation is
        // already a valid projection order. Checking `contains` against the
        // growing `next.graph` also dedupes ops shared across changesets.
        let mut all_novel: Vec<Dot> = Vec::new();
        for cs in css {
            let mut novel: Vec<Dot> = cs
                .ops
                .iter()
                .map(|o| o.id)
                .filter(|d| !next.graph.contains(d))
                .collect();
            next.graph.receive_changeset_mut(cs)?;
            all_novel.append(&mut novel);
        }
        let applied: Vec<Op<EditOp>> = all_novel
            .iter()
            .filter_map(|d| next.graph.get(d).cloned())
            .collect();
        // A batch large relative to the current document (a fresh-load pull, or another
        // peer's bulk paste) is cheaper to project in one pass than op-by-op: the
        // incremental path pays a fixed per-op cost, so `O(novel)` of it overtakes a
        // single `O(document)` reprojection once `novel` is more than a small fraction of
        // the doc. Small deltas (ordinary remote typing) stay incremental and localized.
        let bulk = all_novel.len().saturating_mul(9) > next.seq.visible_len();
        for op in &applied {
            next.ingest_op_warm(op)?;
            if !bulk {
                next.project_op(op)?;
            }
        }
        if bulk {
            next.reproject()?;
        }
        Ok((next, applied))
    }

    pub fn view(&self) -> DocView<'_> {
        // O(1): reuse the already-maintained `BlockPaths` index instead of rebuilding
        // a structural index on every view read.
        DocView::with_paths(&self.projected, &self.indexes.paths)
    }

    pub fn projected(&self) -> &ProjectedDoc {
        &self.projected
    }

    /// Repair stats accumulated since the last call, resetting the running total.
    pub fn take_repair_stats(&mut self) -> RepairStats {
        std::mem::take(&mut self.repair_stats)
    }

    /// Peeks the running repair stats without draining them. `take_repair_stats`
    /// requires `&mut self`, which forces an `Arc` copy-on-write clone whenever the
    /// projected state is shared (e.g. `View`'s layout-cache snapshot); callers on a
    /// hot, usually-empty path should check this first and skip the mutable call.
    pub fn repair_stats(&self) -> RepairStats {
        self.repair_stats
    }

    pub fn graph(&self) -> &OpGraph<EditOp> {
        &self.graph
    }

    pub fn block_modifiers(&self) -> &ModifierAttrLog {
        &self.logs.block_modifiers
    }

    pub fn seq(&self) -> &OpLog<SeqItem> {
        &self.logs.seq
    }

    /// The live sequence checkout, already materialized incrementally. Callers that
    /// only need to resolve a handful of anchors (selection restore) should build a
    /// `StableResolveCtx::from_live` over this instead of rebuilding a checkout from
    /// the oplog.
    pub fn seq_checkout(&self) -> &editor_crdt::sequence::SeqCheckout {
        &self.seq
    }

    pub fn node_attrs(&self) -> &NodeAttrLog {
        &self.logs.node_attrs
    }

    pub fn node_carries(&self) -> &ModifierAttrLog {
        &self.logs.node_carries
    }

    pub fn carry_modifiers(
        &self,
        block: Dot,
    ) -> std::collections::BTreeMap<ModifierType, editor_model::Modifier> {
        self.projected.carry_modifiers(block)
    }

    pub fn spans(&self) -> &SpanLog {
        &self.logs.spans
    }

    /// Whether any logged span op of `ty` overlaps the inclusive leaf range
    /// `[first, last]`. When none does, a whole-range cancel of that type is a
    /// provable no-op the command layer can skip emitting — sparing the
    /// O(covered leaves) projection walk the op would cost. Conservative: an
    /// unresolvable range reports `true` so the caller keeps the cancel.
    pub fn span_of_type_overlaps(&self, first: Dot, last: Dot, ty: ModifierType) -> bool {
        let (Some(s), Some(e)) = (
            self.seq
                .resolve_boundary(first, Bias::Before)
                .map(|b| b.position),
            self.seq
                .resolve_boundary(last, Bias::After)
                .map(|b| b.position),
        ) else {
            return true;
        };
        if s >= e {
            return false;
        }
        self.logs.spans.iter().any(|(_, op)| {
            let op_ty = match op {
                SpanOp::AddSpan { modifier, .. } => modifier.as_type(),
                SpanOp::RemoveSpan { modifier_type, .. } => *modifier_type,
            };
            if op_ty != ty {
                return false;
            }
            let (sa, ea) = op.anchors();
            let (Some(os), Some(oe)) = (
                self.seq
                    .resolve_boundary(sa.id, sa.bias.into())
                    .map(|b| b.position),
                self.seq
                    .resolve_boundary(ea.id, ea.bias.into())
                    .map(|b| b.position),
            ) else {
                return false;
            };
            os < e && s < oe
        })
    }

    pub fn span_covered_own(
        &self,
        start: Anchor,
        end: Anchor,
        ty: ModifierType,
    ) -> Vec<(Dot, Option<Modifier>)> {
        let (Some(s), Some(e)) = (
            self.seq
                .resolve_boundary(start.id, start.bias.into())
                .map(|b| b.position),
            self.seq
                .resolve_boundary(end.id, end.bias.into())
                .map(|b| b.position),
        ) else {
            return Vec::new();
        };
        if s >= e {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut cur_block: Option<Dot> = None;
        let mut ord = 0usize;
        for pos in s..e {
            let Some(leaf) = self.seq.dot_at_visible(&self.logs.seq, pos) else {
                continue;
            };
            let Some(block) = self.indexes.paths.block_of(leaf) else {
                cur_block = None;
                continue;
            };
            if cur_block != Some(block) {
                let Some(base) = self.leaf_ordinal_of(block, leaf) else {
                    cur_block = None;
                    continue;
                };
                cur_block = Some(block);
                ord = base;
            }
            let own = self
                .projected
                .seg_index
                .seg_at(block, ord)
                .and_then(|(seg, _)| seg.own.get(&ty).map(|o| o.value.clone()));
            out.push((leaf, own));
            ord += 1;
        }
        out
    }

    pub fn seq_flat_pos(&self, dot: Dot) -> Option<usize> {
        self.seq
            .resolve_boundary(dot, Bias::Before)
            .map(|b| b.position)
    }

    /// The tree-parent chain a block/block-atom marker was authored under, read from
    /// the sequence log (the authoritative projection source). Tombstone-safe — keyed
    /// by dot, not visible position. Empty for a synthetic dot, a non-structural item,
    /// or an unknown dot.
    fn marker_parents(&self, dot: Dot) -> Vec<Dot> {
        let Some(&lv) = self.logs.seq.lv_of.get(&dot) else {
            return Vec::new();
        };
        match &self.logs.seq.entries[lv].op {
            ListOp::Ins {
                item: SeqItem::Block { parents, .. } | SeqItem::BlockAtom { parents, .. },
                ..
            } => parents.clone(),
            _ => Vec::new(),
        }
    }

    pub fn seq_boundary_pos(&self, dot: Dot, bias: Bias) -> Option<usize> {
        self.seq.resolve_boundary(dot, bias).map(|b| b.position)
    }

    // === Structural navigation for the step layer ===
    // These read the flat tree (O(1) node access) and the incrementally-maintained
    // `BlockPaths` index directly, instead of building an `O(n)` `DocView`. The step
    // helpers call them per insert during a paste, so a `view()` rebuild here is the
    // paste's dominant non-projection cost.

    /// The block's node type, or `None` if it isn't a live block.
    pub fn block_node_type(&self, block: Dot) -> Option<NodeType> {
        self.projected.tree.get(block).map(|n| n.node_type)
    }

    /// Number of direct children (blocks + leaves) of `block`.
    pub fn child_count(&self, block: Dot) -> Option<usize> {
        self.projected.tree.get(block).map(|n| n.children.len())
    }

    /// Whether `block` is a live block node (not a leaf / absent).
    pub fn is_block(&self, block: Dot) -> bool {
        self.projected.tree.get(block).is_some()
    }

    /// Ordered child dots of `block` — leaves by id, child blocks by id (synthetic
    /// blocks included), matching the command layer's full child-slot addressing.
    pub fn child_elem_dots(&self, block: Dot) -> Vec<Dot> {
        match self.projected.tree.get(block) {
            Some(n) => n
                .children
                .iter()
                .map(|c| match c {
                    Child::Leaf { id, .. } => *id,
                    Child::Block(d) => *d,
                })
                .collect(),
            None => Vec::new(),
        }
    }

    /// Ordered child *block* dots of `block` (blocks only).
    pub fn child_block_dots(&self, block: Dot) -> Vec<Dot> {
        match self.projected.tree.get(block) {
            Some(n) => n
                .children
                .iter()
                .filter_map(|c| match c {
                    Child::Block(d) => Some(*d),
                    Child::Leaf { .. } => None,
                })
                .collect(),
            None => Vec::new(),
        }
    }

    /// The dot of `block`'s `index`-th child (leaf id or block id).
    pub fn child_dot_at(&self, block: Dot, index: usize) -> Option<Dot> {
        match self.projected.tree.get(block)?.children.get(index)? {
            Child::Leaf { id, .. } => Some(*id),
            Child::Block(d) => Some(*d),
        }
    }

    /// The maximum sequence position spanned by `block`'s subtree — i.e. the seq
    /// position of its last element, used to find where the next sibling inserts.
    /// A subtree is contiguous in the sequence and the flat tree mirrors sequence
    /// order (each block keeps its children in sequence order), so the deepest
    /// *last* child holds the max: walking the rightmost path is `O(depth)` and a
    /// single `resolve_boundary`, versus resolving every dot in the subtree. Falls
    /// back to the exhaustive `O(subtree)` max if the rightmost path can't resolve.
    pub fn subtree_max_seq_pos(&self, block: Dot) -> Option<usize> {
        let mut node = block;
        while let Some(n) = self.projected.tree.get(node) {
            match n.children.last() {
                Some(Child::Block(d)) => node = *d,
                Some(Child::Leaf { id, .. }) => match self.seq_flat_pos(*id) {
                    Some(p) => return Some(p),
                    None => break,
                },
                None => match anchor_dot(node).and_then(|d| self.seq_flat_pos(d)) {
                    Some(p) => return Some(p),
                    None => break,
                },
            }
        }
        self.subtree_real_dots(block)
            .iter()
            .filter_map(|&d| self.seq_flat_pos(d))
            .max()
    }

    /// `block` plus every descendant, real op dots only (synthetic scaffolds dropped).
    /// Order is unspecified — callers take the max sequence position. Walks the flat
    /// tree directly — NOT `BlockPaths::descendants_of`, which rebuilds a whole-document
    /// children index per call and turns a per-block caller (deleting N blocks) into
    /// O(N · document). O(subtree).
    pub fn subtree_real_dots(&self, block: Dot) -> Vec<Dot> {
        let mut out = Vec::new();
        if anchor_dot(block).is_some() {
            out.push(block);
        }
        let mut stack = vec![block];
        while let Some(b) = stack.pop() {
            if let Some(n) = self.projected.tree.get(b) {
                for c in &n.children {
                    match c {
                        Child::Block(d) => {
                            if anchor_dot(*d).is_some() {
                                out.push(*d);
                            }
                            stack.push(*d);
                        }
                        Child::Leaf { id, .. } => {
                            if anchor_dot(*id).is_some() {
                                out.push(*id);
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// `block`'s ancestor chain, root-first, real op dots only. Includes `block`
    /// itself when `inclusive`. O(depth).
    pub fn ancestor_real_dots(&self, block: Dot, inclusive: bool) -> Vec<Dot> {
        let mut chain = self.indexes.paths.path_of(block); // self → root
        if !inclusive && !chain.is_empty() {
            chain.remove(0);
        }
        chain.reverse(); // root → self
        chain
            .into_iter()
            .filter(|d| anchor_dot(*d).is_some())
            .collect()
    }

    /// The projected node value of `block` (its overlaid attrs, or the type default).
    pub fn block_node(&self, block: Dot) -> Option<Node> {
        let n = self.projected.tree.get(block)?;
        Some(
            anchor_dot(block)
                .and_then(|d| self.projected.node_attrs.get(&d).cloned())
                .or_else(|| block_init_of(&self.projected.tree, block))
                .unwrap_or_else(|| n.node_type.into_node()),
        )
    }

    pub fn atom_leaf_node(&self, leaf: Dot) -> Option<Node> {
        let base = self.atom_leaf(leaf)?.clone().into_node();
        Some(self.logs.node_attrs.attrs_of(leaf, base))
    }

    fn atom_leaf(&self, leaf: Dot) -> Option<&AtomLeaf> {
        let block = self.indexes.paths.block_of(leaf)?;
        self.projected
            .tree
            .get(block)?
            .children
            .iter()
            .find_map(|child| match child {
                Child::Leaf {
                    id,
                    item: SeqItem::Atom(atom),
                } if *id == leaf => Some(atom),
                _ => None,
            })
    }

    /// A clone of `block`'s direct children (leaves carry their item inline). O(children).
    pub fn block_children(&self, block: Dot) -> Option<Vec<Child>> {
        self.projected.tree.get(block).map(|n| n.children.to_vec())
    }

    /// The parent block of `node` (a block or leaf), or `None` for the root / absent.
    pub fn parent_of(&self, node: Dot) -> Option<Dot> {
        self.indexes
            .paths
            .parent_of(node)
            .or_else(|| self.indexes.paths.block_of(node))
    }

    /// `seq_flat_pos`, but `None` when `dot` is currently a tombstone (already
    /// deleted, e.g. by a concurrent op). Used to invert an `Ins`: re-deleting a
    /// char that is no longer visible would target a non-existent element and
    /// overrun the sequence, so undoing such an insertion must be a no-op.
    pub fn seq_visible_pos(&self, dot: Dot) -> Option<usize> {
        let boundary = self.seq.resolve_boundary(dot, Bias::Before)?;
        boundary.visible.then_some(boundary.position)
    }

    /// Descending current visible positions of the still-visible elements that
    /// deletion op `del` removed (used to invert an `Undel` for redo). Concurrently
    /// re-deleted targets are excluded, so each position can be re-deleted with a
    /// single-element `Del` applied in order.
    pub fn del_target_positions(&self, del: Dot) -> Vec<usize> {
        self.seq.del_target_positions(del)
    }
}

/// Storage-independent (log-derived) truth for a single leaf: the segment key and
/// derived state computed straight from the span log + node state, with NO read of
/// the old per-leaf `effective`/`own_modifiers` maps.
#[cfg(test)]
#[derive(Clone)]
pub(crate) struct LeafTruth {
    pub leaf_type: NodeType,
    pub covering: Option<editor_model::SegCovering>,
    pub eff: editor_model::LeafEff,
    pub own: editor_model::LeafOwn,
}

#[cfg(test)]
impl ProjectedState {
    /// Derive one leaf's truth from the logs: canonicalize the winners of the span
    /// ops stabbing its visible position, then run `derive_seg_state` with that
    /// covering plus the leaf's own node style/attrs.
    pub(crate) fn leaf_truth(
        &self,
        leaf: Dot,
        block: Dot,
        leaf_type: NodeType,
        rs: &editor_model::ResolvedSpans,
    ) -> LeafTruth {
        let covering = self
            .seq
            .resolve_boundary(leaf, Bias::Before)
            .map(|b| b.position)
            .and_then(|p| canonical_covering_of(&rs.covering(p), &self.logs.spans));
        let attrs_singleton =
            self.projected.node_attrs.contains_key(&leaf) || !leaf_type.spec().inline;
        let attr_leaf = attrs_singleton.then_some(leaf);
        let (eff, own) = editor_model::derive_seg_state(
            &self.indexes.paths,
            &self.logs,
            &self.projected,
            block,
            leaf_type,
            covering.as_deref(),
            attr_leaf,
        );
        LeafTruth {
            leaf_type,
            covering,
            eff,
            own,
        }
    }

    /// A log-derived truth for every block leaf in the document, keyed by leaf dot.
    pub(crate) fn log_derived_leaf_map(&self) -> HashMap<Dot, LeafTruth> {
        let rs = editor_model::ResolvedSpans::build(&self.logs.spans, &self.seq);
        let mut out: HashMap<Dot, LeafTruth> = HashMap::new();
        let mut stack: Vec<Dot> = self
            .projected
            .tree
            .root_node()
            .map(|r| r.id)
            .into_iter()
            .collect();
        while let Some(bid) = stack.pop() {
            let Some(node) = self.projected.tree.get(bid) else {
                continue;
            };
            for c in node.children.iter() {
                match c {
                    Child::Leaf { id, item } => {
                        let lt = item.as_child_type().unwrap_or(NodeType::Unknown);
                        out.insert(*id, self.leaf_truth(*id, bid, lt, &rs));
                    }
                    Child::Block(id) => stack.push(*id),
                }
            }
        }
        out
    }

    /// The segment index must expand, leaf-for-leaf, to the log-derived truth —
    /// segment key (`leaf_type`, `style`, `covering`) AND derived state
    /// (`eff`, `own`). Deriving `eff` FROM the covering makes a wrong LWW winner a
    /// visible mismatch, so this proves covering-key correctness, not just eff.
    pub(crate) fn assert_seg_index_matches_logs(&self) {
        let rs = editor_model::ResolvedSpans::build(&self.logs.spans, &self.seq);
        let mut stack: Vec<Dot> = self
            .projected
            .tree
            .root_node()
            .map(|r| r.id)
            .into_iter()
            .collect();
        while let Some(bid) = stack.pop() {
            let Some(node) = self.projected.tree.get(bid) else {
                continue;
            };
            let leaves: Vec<(Dot, NodeType)> = node
                .children
                .iter()
                .filter_map(|c| match c {
                    Child::Leaf { id, item } => {
                        Some((*id, item.as_child_type().unwrap_or(NodeType::Unknown)))
                    }
                    Child::Block(_) => None,
                })
                .collect();
            let expanded: Vec<&editor_model::Seg> = self
                .projected
                .seg_index
                .group_iter(bid)
                .flat_map(|s| std::iter::repeat_n(s, s.count))
                .collect();
            assert_eq!(
                expanded.len(),
                leaves.len(),
                "seg leaf count mismatch in {bid:?}"
            );
            for ((dot, ty), seg) in leaves.iter().zip(expanded) {
                let truth = self.leaf_truth(*dot, bid, *ty, &rs);
                assert_eq!(
                    seg.leaf_type, truth.leaf_type,
                    "leaf_type mismatch at {dot:?}"
                );
                assert_eq!(seg.covering, truth.covering, "covering mismatch at {dot:?}");
                assert_eq!(&*seg.eff, &*truth.eff, "eff mismatch at {dot:?}");
                assert_eq!(&*seg.own, &*truth.own, "own mismatch at {dot:?}");
            }
            for c in node.children.iter() {
                if let Child::Block(id) = c {
                    stack.push(*id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LayoutDirty;
    use editor_crdt::Dot;
    use editor_model::{
        AliasOp, AliasRun, Anchor, Bias, CalloutNodeAttr, CalloutVariant, ImageNodeAttr, Modifier,
        ModifierAttrOp, ModifierType, Node, NodeAttr, NodeAttrOp, SpanOp,
    };

    fn seq_block(pos: usize, node_type: NodeType, parents: Vec<Dot>) -> EditOp {
        EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Block {
                node_type,
                parents,
                attrs: vec![],
            },
        })
    }

    fn seq_char(pos: usize, c: char) -> EditOp {
        EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Char(c),
        })
    }

    /// Real (non-synthetic) dots reachable from Root by walking the projected tree's
    /// child references — the structural truth a totality check enforces. A block whose
    /// id is only referenced but whose node is absent (a dangling reference) is not
    /// counted, so an orphaned leaf under it is likewise absent here.
    fn reachable_real_dots(doc: &ProjectedDoc) -> HashSet<Dot> {
        let mut out = HashSet::new();
        let mut stack: Vec<Dot> = vec![doc.tree.root];
        while let Some(id) = stack.pop() {
            let Some(node) = doc.tree.get(id) else {
                continue;
            };
            for c in node.children.iter() {
                match c {
                    Child::Leaf { id, .. } => {
                        if !id.is_synthetic() {
                            out.insert(*id);
                        }
                    }
                    Child::Block(id) => {
                        if !id.is_synthetic() {
                            out.insert(*id);
                        }
                        stack.push(*id);
                    }
                }
            }
        }
        out
    }

    #[test]
    fn char_insert_marks_owning_block_content() {
        let mut ps = ProjectedState::empty();
        ps.commit();
        let para = ps
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        let _ = ps.take_layout_dirty();

        ps.apply(seq_char(1, 'a')).unwrap();

        match ps.take_layout_dirty() {
            LayoutDirty::Incremental { content, .. } => {
                assert!(
                    content.contains(&para),
                    "edited block must be marked content-dirty"
                );
            }
            LayoutDirty::Full => panic!("a single char insert must not force Full"),
        }
    }

    #[test]
    fn take_layout_dirty_resets_to_empty() {
        let mut ps = ProjectedState::empty();
        let _ = ps.take_layout_dirty();
        assert!(matches!(
            ps.take_layout_dirty(),
            LayoutDirty::Incremental { content, structural }
                if content.is_empty() && structural.is_empty()
        ));
    }

    #[test]
    fn from_graph_projects_a_paragraph() {
        let mut g = OpGraph::<EditOp>::with_actor(1);
        let para = g
            .add_mut(seq_block(0, NodeType::Paragraph, vec![Dot::ROOT]))
            .unwrap()
            .id;
        g.add_mut(seq_char(1, 'H')).unwrap();
        g.add_mut(seq_char(2, 'i')).unwrap();

        let state = ProjectedState::from_graph(g).expect("projects");
        let view = state.view();
        let p = view.node(para).expect("paragraph present");
        assert_eq!(p.node_type(), NodeType::Paragraph);
        assert_eq!(p.inline_text(), "Hi");
        assert!(state.block_modifiers().modifiers_of(Dot::ROOT).is_empty());
    }

    #[test]
    fn terminal_page_break_warm_projection_adds_synthetic_trailing_paragraph() {
        use editor_model::AtomLeaf;

        let mut warm = ProjectedState::empty();
        let _ = warm.take_layout_dirty();

        warm.apply(EditOp::Seq(ListOp::Ins {
            pos: 1,
            item: SeqItem::Atom(AtomLeaf::PageBreak),
        }))
        .unwrap();

        let children: Vec<(NodeType, Dot)> = warm
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .map(|block| (block.node_type(), block.id()))
            .collect();
        assert_eq!(children.len(), 2);
        assert_eq!(children[1].0, NodeType::Paragraph);
        assert!(children[1].1.is_synthetic());
        assert!(!matches!(warm.take_layout_dirty(), LayoutDirty::Full));

        let cold = ProjectedState::from_graph(warm.graph().clone()).unwrap();
        assert_eq!(warm.projected(), cold.projected());
    }

    #[test]
    fn unknown_root_block_after_page_break_does_not_hide_trailing_completion() {
        use editor_model::AtomLeaf;

        let mut warm = ProjectedState::empty();
        warm.apply(EditOp::Seq(ListOp::Ins {
            pos: 1,
            item: SeqItem::Atom(AtomLeaf::PageBreak),
        }))
        .unwrap();
        warm.apply(seq_block(2, NodeType::Unknown, vec![Dot::ROOT]))
            .unwrap();

        let cold = ProjectedState::from_graph(warm.graph().clone()).unwrap();
        assert_eq!(warm.projected(), cold.projected());
        assert!(
            warm.view()
                .root()
                .unwrap()
                .child_blocks()
                .last()
                .unwrap()
                .id()
                .is_synthetic()
        );
    }

    #[test]
    fn empty_seeds_implicit_root_and_paragraph() {
        let state = ProjectedState::empty();
        let view = state.view();
        let root = view.root().expect("root present");
        assert_eq!(root.node_type(), NodeType::Root);
        assert_eq!(root.id(), Dot::ROOT);
        let para = root.child_blocks().next().expect("seeded paragraph");
        assert_eq!(para.node_type(), NodeType::Paragraph);
        assert!(para.id().as_op_dot().is_some());
    }

    #[test]
    fn apply_builds_paragraph_and_returns_op_dots() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        let h = state.apply(seq_char(1, 'H')).unwrap();
        let _i = state.apply(seq_char(2, 'i')).unwrap();
        let view = state.view();
        assert_eq!(view.leaf(h.id).and_then(|l| l.as_char()), Some('H'));
        let p = view.node(para).unwrap();
        assert_eq!(p.inline_text(), "Hi");
    }

    #[test]
    fn apply_nested_blocks() {
        let mut state = ProjectedState::empty();
        let root = state.view().root().unwrap().dot().unwrap();
        let bq = state
            .apply(seq_block(1, NodeType::Blockquote, vec![root]))
            .unwrap()
            .id;
        let bqp = state
            .apply(seq_block(2, NodeType::Paragraph, vec![root, bq]))
            .unwrap()
            .id;
        let _x = state.apply(seq_char(3, 'x')).unwrap();
        let view = state.view();
        let bqp_view = view.node(bqp).unwrap();
        assert_eq!(bqp_view.inline_text(), "x");
        assert_eq!(bqp_view.parent().unwrap().node_type(), NodeType::Blockquote);
    }

    #[test]
    fn apply_span_enriches_effective() {
        let mut state = ProjectedState::empty();
        let x = state.apply(seq_char(1, 'x')).unwrap().id;
        state
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: x,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: x,
                    bias: Bias::After,
                },
                modifier: Modifier::Bold,
            }))
            .unwrap();
        let view = state.view();
        assert_eq!(
            view.leaf_state_by_dot_slow(x)
                .unwrap()
                .eff
                .get(&ModifierType::Bold),
            Some(&Modifier::Bold)
        );
    }

    #[test]
    fn from_graph_with_overlay_drops_dot_and_rebases_following_span_anchor() {
        let mut ps = ProjectedState::empty();
        let a = ps.apply(seq_char(1, 'a')).unwrap().id;
        let b = ps.apply(seq_char(2, 'b')).unwrap().id;
        let c = ps.apply(seq_char(3, 'c')).unwrap().id;
        ps.apply(EditOp::Span(SpanOp::AddSpan {
            start: Anchor {
                id: a,
                bias: Bias::Before,
            },
            end: Anchor {
                id: c,
                bias: Bias::After,
            },
            modifier: Modifier::Bold,
        }))
        .unwrap();
        ps.commit();
        let graph = ps.graph().clone();

        let full = ProjectedState::from_graph_with_overlay(graph.clone(), &[]).unwrap();
        assert_eq!(
            full.view()
                .root()
                .unwrap()
                .child_blocks()
                .next()
                .unwrap()
                .inline_text(),
            "abc"
        );

        let overlaid = ProjectedState::from_graph_with_overlay(graph, &[b]).unwrap();
        let view = overlaid.view();
        assert_eq!(
            view.root()
                .unwrap()
                .child_blocks()
                .next()
                .unwrap()
                .inline_text(),
            "ac",
            "the overlay dot is removed from the tree"
        );
        assert_eq!(
            view.leaf_state_by_dot_slow(a)
                .unwrap()
                .eff
                .get(&ModifierType::Bold),
            Some(&Modifier::Bold),
            "the anchor before the overlay keeps its span"
        );
        assert_eq!(
            view.leaf_state_by_dot_slow(c)
                .unwrap()
                .eff
                .get(&ModifierType::Bold),
            Some(&Modifier::Bold),
            "the anchor after the overlay is re-based, not drifted off the covered char"
        );
    }

    /// Winner-excluded-by-boundary: a leaf inserted just after `a`, whose seed
    /// neighbour `a` has bold-winner B (newer) that is anchored to `a` and so
    /// excludes the new position, while an older span A (covering the whole run,
    /// anchored away from the gap) still covers it. Winners-only seeding drops A
    /// (the runner-up B hid at `a`), so `covering_for_inserted` must fall back to a
    /// full resolve and recover A as the new leaf's bold winner. Skipping the
    /// fallback makes the new leaf lose bold — caught here and by the log oracle.
    #[test]
    fn insert_seeds_winner_excluded_by_boundary() {
        fn bold_winner(cov: &Option<editor_model::SegCovering>) -> Option<Dot> {
            cov.as_ref()
                .and_then(|c| c.get(&ModifierType::Bold))
                .copied()
        }
        let mut ps = ProjectedState::empty();
        let z = ps.apply(seq_char(1, 'z')).unwrap().id;
        let a = ps.apply(seq_char(2, 'a')).unwrap().id;
        ps.apply(seq_char(3, 'b')).unwrap();
        let c = ps.apply(seq_char(4, 'c')).unwrap().id;

        // Span A: bold over the whole run [z..c], anchored to z/c (away from the a|b
        // gap). Applied first → older op dot.
        let a_span = ps
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: z,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: c,
                    bias: Bias::After,
                },
                modifier: Modifier::Bold,
            }))
            .unwrap()
            .id;
        // Span B: bold anchored at `a` only (applied second → newer op dot). At `a` it
        // wins LWW, but end=(a, After) excludes anything inserted just after `a`.
        let b_span = ps
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: a,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: a,
                    bias: Bias::After,
                },
                modifier: Modifier::Bold,
            }))
            .unwrap()
            .id;
        assert!(a_span < b_span, "A must be the older op dot");

        // Neighbour `a`'s bold winner is B (the boundary span excluded at X).
        let a_block = ps.indexes.paths.block_of(a).unwrap();
        let a_ord = ps.leaf_ordinal_of(a_block, a).unwrap();
        let a_cov = ps
            .projected
            .seg_index
            .seg_at(a_block, a_ord)
            .unwrap()
            .0
            .covering
            .clone();
        assert_eq!(
            bold_winner(&a_cov),
            Some(b_span),
            "seed neighbour's bold winner must be B"
        );

        let _ = ps.take_layout_dirty();
        // Insert X between `a` and `b` — the incremental hot path exercising
        // `covering_for_inserted`.
        let x = ps.apply(seq_char(3, 'X')).unwrap().id;
        assert!(
            matches!(ps.take_layout_dirty(), LayoutDirty::Incremental { .. }),
            "mid-paragraph insert must stay on the incremental path"
        );

        // X must recover A's bold via the full-resolve fallback (B excludes X).
        let x_block = ps.indexes.paths.block_of(x).unwrap();
        let x_ord = ps.leaf_ordinal_of(x_block, x).unwrap();
        let x_cov = ps
            .projected
            .seg_index
            .seg_at(x_block, x_ord)
            .unwrap()
            .0
            .covering
            .clone();
        assert_eq!(
            bold_winner(&x_cov),
            Some(a_span),
            "new leaf's bold winner must be A (runner-up revealed by the fallback)"
        );
        assert_eq!(
            ps.view()
                .leaf_state_by_dot_slow(x)
                .unwrap()
                .eff
                .get(&ModifierType::Bold),
            Some(&Modifier::Bold),
            "new leaf must be bold"
        );

        ps.assert_seg_index_matches_logs();
    }

    #[test]
    fn apply_block_modifier_lands_in_log_and_projection() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        state
            .apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: para,
                modifier: Modifier::LineHeight { value: 200 },
            }))
            .unwrap();
        assert_eq!(
            state
                .block_modifiers()
                .modifiers_of(para)
                .get(&ModifierType::LineHeight),
            Some(&Modifier::LineHeight { value: 200 })
        );
        let _c = state.apply(seq_char(1, 'a')).unwrap().id;
        assert_eq!(
            state
                .view()
                .node(para)
                .unwrap()
                .effective()
                .get(&ModifierType::LineHeight),
            Some(&Modifier::LineHeight { value: 200 }),
            "the Paragraph consumes LineHeight: it resolves on the paragraph block (its record does not pass down to text carriers)"
        );
    }

    #[test]
    fn apply_node_carry_projects() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        state
            .apply(EditOp::NodeCarry(ModifierAttrOp::SetModifier {
                target: para,
                modifier: Modifier::Bold,
            }))
            .unwrap();
        assert_eq!(
            state
                .projected()
                .node_carries
                .get(&para)
                .and_then(|c| c.get(&ModifierType::Bold)),
            Some(&Modifier::Bold)
        );
    }

    #[test]
    fn apply_delete_and_undel() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        let a = state.apply(seq_char(1, 'a')).unwrap().id;
        let _b = state.apply(seq_char(2, 'b')).unwrap();
        assert_eq!(state.view().node(para).unwrap().inline_text(), "ab");
        let del = state
            .apply(EditOp::Seq(ListOp::Del { pos: 1, len: 1 }))
            .unwrap()
            .id;
        assert_eq!(state.view().node(para).unwrap().inline_text(), "b");
        let _ = a;
        state.apply(EditOp::Seq(ListOp::Undel { del })).unwrap();
        assert_eq!(state.view().node(para).unwrap().inline_text(), "ab");
    }

    #[test]
    fn apply_leaf_typed_block_errors() {
        let mut state = ProjectedState::empty();
        let root = state.view().root().unwrap().dot().unwrap();
        let err = state.apply(seq_block(1, NodeType::Text, vec![root]));
        assert!(matches!(
            err,
            Err(SpineError::Projection(
                ProjectionError::LeafTypedBlock { .. }
            ))
        ));
    }

    #[test]
    fn apply_node_attr_projects() {
        let mut state = ProjectedState::empty();
        let root = state.view().root().unwrap().dot().unwrap();
        let callout = state
            .apply(seq_block(1, NodeType::Callout, vec![root]))
            .unwrap()
            .id;
        let _cp = state
            .apply(seq_block(2, NodeType::Paragraph, vec![root, callout]))
            .unwrap();
        state
            .apply(EditOp::NodeAttr(NodeAttrOp {
                target: callout,
                attr: NodeAttr::Callout {
                    attr: CalloutNodeAttr::Variant(CalloutVariant::Warning),
                },
            }))
            .unwrap();
        assert!(state.projected().node_attrs.contains_key(&callout));
        assert!(matches!(
            state.view().node(callout).unwrap().node(),
            Node::Callout(_)
        ));
    }

    #[test]
    fn apply_node_attr_to_atom_leaf_preserves_payload_baseline() {
        let mut state = ProjectedState::empty();
        let mut image_node = match NodeType::Image.into_node() {
            Node::Image(n) => n,
            _ => unreachable!(),
        };
        image_node.id = editor_crdt::LwwReg::with_value(Some("asset-1".to_string()));
        image_node.proportion = editor_crdt::LwwReg::with_value(75);
        let image = state
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::BlockAtom {
                    leaf: AtomLeaf::Image { node: image_node },
                    parents: vec![Dot::ROOT],
                },
            }))
            .unwrap()
            .id;

        state
            .apply(EditOp::NodeAttr(NodeAttrOp {
                target: image,
                attr: NodeAttr::Image {
                    attr: ImageNodeAttr::Proportion(150),
                },
            }))
            .unwrap();

        match state.atom_leaf_node(image) {
            Some(Node::Image(node)) => {
                assert_eq!(node.id.get(), &Some("asset-1".to_string()));
                assert_eq!(*node.proportion.get(), 150);
            }
            other => panic!("expected projected image node, got {other:?}"),
        }
    }

    #[test]
    fn alias_warm_apply_matches_cold_projection() {
        let mut state = ProjectedState::empty();
        let a = state.apply(seq_char(1, 'a')).unwrap().id;
        let b = state.apply(seq_char(2, 'b')).unwrap().id;
        state
            .apply(EditOp::Alias(AliasOp {
                pairs: vec![AliasRun {
                    old_start: a,
                    len: 1,
                    new_start: b,
                }],
            }))
            .unwrap();
        let warm = state.projected().alias_classes.clone();
        let cold = {
            let logs = editor_model::split_logs(state.graph()).unwrap();
            editor_model::project_document(&logs).unwrap().alias_classes
        };
        assert_eq!(warm, cold);
    }

    #[test]
    fn alias_survives_forced_reproject() {
        let mut state = ProjectedState::empty();
        let a = state.apply(seq_char(1, 'a')).unwrap().id;
        let b = state.apply(seq_char(2, 'b')).unwrap().id;
        state
            .apply(EditOp::Alias(AliasOp {
                pairs: vec![AliasRun {
                    old_start: a,
                    len: 1,
                    new_start: b,
                }],
            }))
            .unwrap();
        let before = state.projected().alias_classes.clone();
        state.reproject_all().unwrap();
        assert_eq!(
            state.projected().alias_classes,
            before,
            "reproject가 logs.aliases에서 맵을 재구성"
        );
    }

    #[test]
    fn alias_op_admission_rejects_invalid_op() {
        let mut state = ProjectedState::empty();
        let a = state.apply(seq_char(1, 'a')).unwrap().id;
        let before = state.graph().clone();
        let err = state.apply(EditOp::Alias(AliasOp {
            pairs: vec![AliasRun {
                old_start: a,
                len: 1,
                new_start: a,
            }],
        }));
        assert!(matches!(err, Err(SpineError::InvalidOp)));
        assert_eq!(
            state.graph(),
            &before,
            "invalid alias op must not mutate the graph"
        );
    }

    #[test]
    fn alias_apply_does_not_force_full_reproject() {
        let mut state = ProjectedState::empty();
        let a = state.apply(seq_char(1, 'a')).unwrap().id;
        let b = state.apply(seq_char(2, 'b')).unwrap().id;
        let _ = state.take_layout_dirty();
        state
            .apply(EditOp::Alias(AliasOp {
                pairs: vec![AliasRun {
                    old_start: a,
                    len: 1,
                    new_start: b,
                }],
            }))
            .unwrap();
        match state.take_layout_dirty() {
            LayoutDirty::Full => panic!("alias op forced a full reproject"),
            LayoutDirty::Incremental {
                content,
                structural,
            } => {
                assert!(content.is_empty() && structural.is_empty());
            }
        }
    }

    fn arb_chars() -> impl proptest::strategy::Strategy<Value = Vec<char>> {
        use proptest::prelude::*;
        proptest::collection::vec(prop::sample::select(vec!['a', 'b', 'c']), 0..8)
    }

    proptest::proptest! {
        #[test]
        fn apply_char_sequence_never_panics_and_text_matches(chars in arb_chars()) {
            let mut state = ProjectedState::empty();
            let para = state
                .view()
                .root()
                .unwrap()
                .child_blocks()
                .next()
                .unwrap()
                .dot()
                .unwrap();
            for (i, c) in chars.iter().enumerate() {
                state.apply(seq_char(1 + i, *c)).expect("char applies");
            }
            let expected: String = chars.iter().collect();
            let got = state.view().node(para).unwrap().inline_text();
            proptest::prop_assert_eq!(got, expected);
        }
    }

    fn arb_seq_script() -> impl proptest::strategy::Strategy<Value = Vec<EditOp>> {
        use proptest::prelude::*;
        proptest::collection::vec(
            (
                any::<bool>(),
                any::<u8>(),
                any::<u8>(),
                prop::sample::select(vec!['a', 'b', 'c']),
            ),
            0..30,
        )
        .prop_map(|steps| {
            let mut count = 1usize;
            let mut out = Vec::new();
            for (is_del, raw, raw_len, ch) in steps {
                if is_del && count > 1 {
                    let pos = 1 + (raw as usize) % (count - 1);
                    let len = 1 + (raw_len as usize) % (count - pos);
                    out.push(EditOp::Seq(ListOp::Del { pos, len }));
                    count -= len;
                } else {
                    let pos = 1 + (raw as usize) % count;
                    out.push(seq_char(pos, ch));
                    count += 1;
                }
            }
            out
        })
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 192, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn warm_apply_matches_cold_from_graph(script in arb_seq_script()) {
            let mut warm = ProjectedState::empty();
            for payload in script {
                warm.apply(payload).expect("valid seq op applies");
            }
            let cold = ProjectedState::from_graph(warm.graph().clone())
                .expect("cold rebuild projects");
            proptest::prop_assert_eq!(warm.projected(), cold.projected());
        }
    }

    // Structural edits — paragraphs, blockquotes, blockquote-nested paragraphs, and
    // range deletes — that force window reprojection, hoist-join, and boundary
    // reparenting (a delete removing the top-level content between a blockquote and
    // its orphaned nested paragraphs re-opens the blockquote, pulling them back in).
    // The incrementally maintained projection must equal a cold rebuild over the same
    // op history. `arb_seq_script` above only chars/deletes, so this whole class went
    // unexercised until the reparent regression forced a hand-written script.
    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 192, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn warm_apply_nested_structural_matches_cold(
            script in proptest::collection::vec(
                (
                    proptest::prelude::any::<u8>(),
                    proptest::prelude::any::<u8>(),
                    proptest::prelude::any::<u8>(),
                ),
                0..24,
            ),
        ) {
            let root = Dot::ROOT;
            let mut warm = ProjectedState::empty();
            let mut count = 1usize;
            let mut bqs: Vec<Dot> = Vec::new();
            for (kind, a, b) in script {
                let a = a as usize;
                let b = b as usize;
                match kind % 12 {
                    0..=5 => {
                        let pos = 1 + a % count;
                        warm.apply(seq_char(pos, 'a')).unwrap();
                        count += 1;
                    }
                    6..=7 => {
                        let pos = a % (count + 1);
                        warm.apply(seq_block(pos, NodeType::Paragraph, vec![root])).unwrap();
                        count += 1;
                    }
                    8 => {
                        let pos = a % (count + 1);
                        let d = warm
                            .apply(seq_block(pos, NodeType::Blockquote, vec![root]))
                            .unwrap()
                            .id;
                        bqs.push(d);
                        count += 1;
                    }
                    9..=10 if !bqs.is_empty() => {
                        let bq = bqs[a % bqs.len()];
                        let pos = 1 + a % count;
                        warm.apply(seq_block(pos, NodeType::Paragraph, vec![root, bq])).unwrap();
                        count += 1;
                    }
                    _ if count > 1 => {
                        let pos = 1 + a % (count - 1);
                        let len = 1 + b % (count - pos);
                        warm.apply(EditOp::Seq(ListOp::Del { pos, len })).unwrap();
                        count -= len;
                    }
                    _ => {}
                }
            }
            let cold = ProjectedState::from_graph(warm.graph().clone())
                .expect("cold rebuild projects");
            proptest::prop_assert_eq!(warm.projected(), cold.projected());
        }
    }

    // Receiving a remote changeset projects each op incrementally (no whole-doc
    // reprojection per sync batch); the merged state must equal the cold rebuild.
    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 192, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn receive_changeset_matches_cold(script in arb_seq_script()) {
            let mut authored = ProjectedState::empty();
            for payload in script {
                authored.apply(payload).expect("valid seq op applies");
            }
            let receiver = ProjectedState::empty();
            let remote_heads: HashSet<Dot> = receiver.graph().current_heads().copied().collect();
            let missing = authored
                .graph()
                .missing_changesets_for(&remote_heads)
                .expect("changesets derivable");
            let mut merged = receiver;
            for cs in missing {
                merged = merged.receive_changeset(cs).expect("receive applies");
            }
            let cold = ProjectedState::from_graph(merged.graph().clone())
                .expect("cold rebuild projects");
            proptest::prop_assert_eq!(merged.projected(), cold.projected());
            merged.assert_seg_index_matches_logs();
        }
    }

    fn arb_span_action() -> impl proptest::strategy::Strategy<Value = (u8, u8, u8, u8, char)> {
        use proptest::prelude::*;
        (
            0u8..12,
            any::<u8>(),
            any::<u8>(),
            any::<u8>(),
            prop::sample::select(vec!['a', 'b', 'c']),
        )
    }

    // Drive the incremental spine with a mix of char ins/del and span add/remove
    // ops (anchored on live char dots), then assert it converges to the cold
    // full-projection. This exercises the lifted spans guard: insert-into/near
    // spans (degenerate→covering, bias edges), span apply over covered ranges,
    // and delete of anchor/covered leaves.
    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 256, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn warm_apply_with_spans_matches_cold(
            actions in proptest::collection::vec(arb_span_action(), 0..40),
        ) {
            let mut warm = ProjectedState::empty();
            let mut live: Vec<Dot> = Vec::new();
            let mut count = 1usize;
            for (kind, a, b, bias, ch) in actions {
                let pick = |i: u8, v: &[Dot]| v[(i as usize) % v.len()];
                let bias_s = if bias & 1 == 0 { Bias::Before } else { Bias::After };
                let bias_e = if bias & 2 == 0 { Bias::Before } else { Bias::After };
                match kind {
                    0..=5 => {
                        let pos = 1 + (a as usize) % count;
                        let d = warm.apply(seq_char(pos, ch)).unwrap().id;
                        live.push(d);
                        count += 1;
                    }
                    6..=7 if count > 1 => {
                        let pos = 1 + (a as usize) % (count - 1);
                        let len = 1 + (b as usize) % (count - pos);
                        warm.apply(EditOp::Seq(ListOp::Del { pos, len })).unwrap();
                        count -= len;
                    }
                    8..=10 if !live.is_empty() => {
                        let m = match bias % 3 {
                            0 => Modifier::Bold,
                            1 => Modifier::Italic,
                            _ => Modifier::FontSize { value: 1400 },
                        };
                        warm.apply(EditOp::Span(SpanOp::AddSpan {
                            start: Anchor { id: pick(a, &live), bias: bias_s },
                            end: Anchor { id: pick(b, &live), bias: bias_e },
                            modifier: m,
                        }))
                        .unwrap();
                    }
                    11 if !live.is_empty() => {
                        warm.apply(EditOp::Span(SpanOp::RemoveSpan {
                            start: Anchor { id: pick(a, &live), bias: bias_s },
                            end: Anchor { id: pick(b, &live), bias: bias_e },
                            modifier_type: ModifierType::Bold,
                        }))
                        .unwrap();
                    }
                    _ => {}
                }
            }
            let cold = ProjectedState::from_graph(warm.graph().clone())
                .expect("cold rebuild projects");
            proptest::prop_assert_eq!(warm.projected(), cold.projected());
            warm.assert_seg_index_matches_logs();
        }
    }

    // Whole-range span ops over a document mixing styled leaves, an inline atom,
    // a root-level block atom (Image), and two paragraphs must converge to the
    // cold rebuild after EVERY op. Pins the bulk span-apply path (shared
    // derivation per uniform leaf group, bulk re-segmentation) against per-leaf
    // divergence: styled leaves must not share a plain leaf's derivation, and an
    // op that changes nothing must not corrupt the projection. Sized past the
    // count>=64 threshold so every whole-range op takes `try_apply_span`'s
    // streaming grouping branch — including its block-level-atom `else` arm (the
    // Image, a leaf of Root arriving mid-stream with no preceding marker).
    #[test]
    fn whole_range_span_ops_styled_atoms_match_cold() {
        use editor_model::AtomLeaf;

        let mut warm = ProjectedState::empty();
        let mut leaves: Vec<Dot> = Vec::new();
        let mut pos = 1;
        for i in 0..40 {
            let ch = char::from(b'a' + (i % 26) as u8);
            leaves.push(warm.apply(seq_char(pos, ch)).unwrap().id);
            pos += 1;
        }
        warm.apply(EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Atom(AtomLeaf::HardBreak),
        }))
        .unwrap();
        pos += 1;
        let img_node = match NodeType::Image.into_node() {
            Node::Image(n) => n,
            _ => unreachable!(),
        };
        warm.apply(EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::BlockAtom {
                leaf: AtomLeaf::Image { node: img_node },
                parents: vec![Dot::ROOT],
            },
        }))
        .unwrap();
        pos += 1;
        warm.apply(seq_block(pos, NodeType::Paragraph, vec![Dot::ROOT]))
            .unwrap();
        pos += 1;
        for i in 0..40 {
            let ch = char::from(b'a' + (i % 26) as u8);
            leaves.push(warm.apply(seq_char(pos, ch)).unwrap().id);
            pos += 1;
        }

        warm.apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
            target: Dot::ROOT,
            modifier: Modifier::FontWeight { value: 400 },
        }))
        .unwrap();

        let first = *leaves.first().unwrap();
        let last = *leaves.last().unwrap();
        // The whole-range ops must exercise the streaming grouping branch
        // (`count >= 64` in `try_apply_span`), with the Image interleaved.
        let covered = warm
            .seq
            .resolve_boundary(last, editor_crdt::sequence::Bias::After)
            .unwrap()
            .position
            - warm
                .seq
                .resolve_boundary(first, editor_crdt::sequence::Bias::Before)
                .unwrap()
                .position;
        assert!(
            covered >= 64,
            "range must take the streaming path: {covered}"
        );
        let whole = |m: SpanOp| EditOp::Span(m);
        let anchors = (
            Anchor {
                id: first,
                bias: Bias::Before,
            },
            Anchor {
                id: last,
                bias: Bias::After,
            },
        );
        let ops = [
            whole(SpanOp::AddSpan {
                start: anchors.0,
                end: anchors.1,
                modifier: Modifier::Italic,
            }),
            // Effective-neutral: no leaf carries an own FontWeight span; the block
            // modifier keeps providing 400, so effectives must not change.
            whole(SpanOp::RemoveSpan {
                start: anchors.0,
                end: anchors.1,
                modifier_type: ModifierType::FontWeight,
            }),
            whole(SpanOp::AddSpan {
                start: anchors.0,
                end: anchors.1,
                modifier: Modifier::FontWeight { value: 700 },
            }),
            whole(SpanOp::RemoveSpan {
                start: anchors.0,
                end: anchors.1,
                modifier_type: ModifierType::Italic,
            }),
            whole(SpanOp::RemoveSpan {
                start: anchors.0,
                end: anchors.1,
                modifier_type: ModifierType::FontWeight,
            }),
        ];
        for (i, op) in ops.into_iter().enumerate() {
            warm.apply(op).unwrap();
            let cold =
                ProjectedState::from_graph(warm.graph().clone()).expect("cold rebuild projects");
            assert_eq!(
                warm.projected(),
                cold.projected(),
                "diverged from cold rebuild after whole-range op #{i}"
            );
            warm.assert_seg_index_matches_logs();
        }
    }

    // A whole-range span over a document whose first block ends in a GHOST region
    // (a mid-text PageBreak truncates the block, dropping its trailing run to
    // sequence ghosts) followed by a SECOND block, sized past the count>=64
    // streaming threshold. Pins the tail-drop invariant `try_apply_span`'s
    // streaming branch relies on: the ghost tail is attributed to the first
    // block's group and absorbed by `apply_range`'s clamping, and the branch MUST
    // reset on the second block's marker so its covered leaves land in their own
    // group. Dropping the marker reset mis-attributes the second block's leaves to
    // the first block and leaves the second block unspanned — caught here by both
    // the cold-equality check and `assert_seg_index_matches_logs`.
    #[test]
    fn whole_range_span_over_split_region_and_second_block_matches_cold() {
        use editor_model::AtomLeaf;

        let mut warm = ProjectedState::empty();
        // Block 1 head: 4 chars at 1..=4.
        let mut head: Vec<Dot> = Vec::new();
        for i in 0..4u8 {
            let d = warm
                .apply(seq_char(1 + i as usize, char::from(b'a' + i)))
                .unwrap()
                .id;
            head.push(d);
        }
        // Block 1 trailing run: 8 chars at 5..=12.
        for i in 0..8u8 {
            warm.apply(seq_char(5 + i as usize, char::from(b'a' + i)))
                .unwrap();
        }
        // Split block 1 with a mid-text PageBreak right after the head — under total
        // projection the 8 trailing chars are SPLIT-HOISTed into following Paragraphs
        // (not dropped), so block 1 keeps only its 4-char head.
        warm.apply(EditOp::Seq(ListOp::Ins {
            pos: 5,
            item: SeqItem::Atom(AtomLeaf::PageBreak),
        }))
        .unwrap();
        let p1_text = warm
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .inline_text();
        assert_eq!(
            p1_text, "abcd",
            "block 1 keeps only its head before the split"
        );

        // Second block appended AFTER the split region, then filled so a whole-range
        // span covers >= 64 visible positions.
        let p2_pos = warm.seq.visible_len();
        warm.apply(seq_block(p2_pos, NodeType::Paragraph, vec![Dot::ROOT]))
            .unwrap();
        let mut p2: Vec<Dot> = Vec::new();
        for i in 0..60u8 {
            let pos = warm.seq.visible_len();
            let d = warm
                .apply(seq_char(pos, char::from(b'a' + i % 26)))
                .unwrap()
                .id;
            p2.push(d);
        }
        assert!(
            warm.view().root().unwrap().child_blocks().count() >= 2,
            "the split region and the second block are multiple top-level blocks"
        );

        let first = *head.first().unwrap();
        let last = *p2.last().unwrap();
        let covered = warm
            .seq
            .resolve_boundary(last, editor_crdt::sequence::Bias::After)
            .unwrap()
            .position
            - warm
                .seq
                .resolve_boundary(first, editor_crdt::sequence::Bias::Before)
                .unwrap()
                .position;
        assert!(
            covered >= 64,
            "range must take the streaming path: {covered}"
        );

        warm.apply(EditOp::Span(SpanOp::AddSpan {
            start: Anchor {
                id: first,
                bias: Bias::Before,
            },
            end: Anchor {
                id: last,
                bias: Bias::After,
            },
            modifier: Modifier::Italic,
        }))
        .unwrap();

        let cold = ProjectedState::from_graph(warm.graph().clone()).expect("cold rebuild projects");
        assert_eq!(
            warm.projected(),
            cold.projected(),
            "diverged from cold rebuild over the split region"
        );
        warm.assert_seg_index_matches_logs();
    }

    // A span op that changes no leaf's effective (removing a FontWeight span no
    // leaf carries while the block modifier keeps providing the same value) must
    // not mark any block layout-dirty — nothing observable changed.
    #[test]
    fn effective_neutral_span_op_skips_layout_dirty() {
        let mut warm = ProjectedState::empty();
        let mut leaves: Vec<Dot> = Vec::new();
        for i in 0..20 {
            let ch = char::from(b'a' + (i % 26) as u8);
            leaves.push(warm.apply(seq_char(1 + i, ch)).unwrap().id);
        }
        warm.apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
            target: Dot::ROOT,
            modifier: Modifier::FontWeight { value: 400 },
        }))
        .unwrap();
        let _ = warm.take_layout_dirty();

        warm.apply(EditOp::Span(SpanOp::RemoveSpan {
            start: Anchor {
                id: *leaves.first().unwrap(),
                bias: Bias::Before,
            },
            end: Anchor {
                id: *leaves.last().unwrap(),
                bias: Bias::After,
            },
            modifier_type: ModifierType::FontWeight,
        }))
        .unwrap();

        match warm.take_layout_dirty() {
            LayoutDirty::Incremental {
                content,
                structural,
            } => {
                assert!(
                    content.is_empty() && structural.is_empty(),
                    "effective-neutral span op must not dirty layout, got content={content:?} structural={structural:?}"
                );
            }
            LayoutDirty::Full => panic!("effective-neutral span op must not force Full"),
        }
    }

    fn arb_bulk_span_action() -> impl proptest::strategy::Strategy<Value = (u8, u8, u8, u8)> {
        use proptest::prelude::*;
        (0u8..14, any::<u8>(), any::<u8>(), any::<u8>())
    }

    // Bulk-path variant of `warm_apply_with_spans_matches_cold`: larger leaf
    // counts (crossing the bulk re-segmentation threshold), per-leaf
    // node styles, inline atoms, and a second paragraph. The incremental spine
    // must converge to the cold rebuild.
    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 128, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn warm_apply_bulk_span_matches_cold(
            actions in proptest::collection::vec(arb_bulk_span_action(), 0..24),
        ) {
            use editor_model::AtomLeaf;

            let mut warm = ProjectedState::empty();
            let mut live: Vec<Dot> = Vec::new();
            // Seed enough chars that whole-range spans take the bulk path.
            let mut count = 1usize;
            for i in 0..40usize {
                let d = warm.apply(seq_char(count, char::from(b'a' + (i % 26) as u8))).unwrap().id;
                live.push(d);
                count += 1;
            }
            for (kind, a, b, bias) in actions {
                let pick = |i: u8, v: &[Dot]| v[(i as usize) % v.len()];
                let bias_s = if bias & 1 == 0 { Bias::Before } else { Bias::After };
                let bias_e = if bias & 2 == 0 { Bias::Before } else { Bias::After };
                let ty = match bias % 4 {
                    0 => ModifierType::Bold,
                    1 => ModifierType::Italic,
                    2 => ModifierType::FontWeight,
                    _ => ModifierType::FontSize,
                };
                let m = match bias % 4 {
                    0 => Modifier::Bold,
                    1 => Modifier::Italic,
                    2 => Modifier::FontWeight { value: 700 },
                    _ => Modifier::FontSize { value: 1400 },
                };
                match kind {
                    0..=2 => {
                        let pos = 1 + (a as usize) % count;
                        let d = warm.apply(seq_char(pos, 'z')).unwrap().id;
                        live.push(d);
                        count += 1;
                    }
                    3 => {
                        let pos = 1 + (a as usize) % count;
                        warm.apply(EditOp::Seq(ListOp::Ins {
                            pos,
                            item: SeqItem::Atom(AtomLeaf::HardBreak),
                        })).unwrap();
                        count += 1;
                    }
                    4 => {
                        let pos = 1 + (a as usize) % count;
                        warm.apply(seq_block(pos, NodeType::Paragraph, vec![Dot::ROOT])).unwrap();
                        count += 1;
                    }
                    6..=9 if !live.is_empty() => {
                        warm.apply(EditOp::Span(SpanOp::AddSpan {
                            start: Anchor { id: pick(a, &live), bias: bias_s },
                            end: Anchor { id: pick(b, &live), bias: bias_e },
                            modifier: m,
                        })).unwrap();
                    }
                    10..=13 if !live.is_empty() => {
                        warm.apply(EditOp::Span(SpanOp::RemoveSpan {
                            start: Anchor { id: pick(a, &live), bias: bias_s },
                            end: Anchor { id: pick(b, &live), bias: bias_e },
                            modifier_type: ty,
                        })).unwrap();
                    }
                    _ => {}
                }
            }
            let cold = ProjectedState::from_graph(warm.graph().clone())
                .expect("cold rebuild projects");
            proptest::prop_assert_eq!(warm.projected(), cold.projected());
            warm.assert_seg_index_matches_logs();
        }
    }

    // A bulk delete large enough to take the coverage-preserving reprojection path
    // (`reproject_after_delete`, threshold > 1024 targets) must land on the exact same
    // projection a cold rebuild produces — including surviving leaves that were, were
    // partially, or were never covered by a span. Guards the invariant that deletion
    // preserves each survivor's covering set and every span anchor.
    #[test]
    fn bulk_delete_reproject_matches_cold() {
        let mut warm = ProjectedState::empty();
        // Three paragraphs of ~600 chars each, so a middle-region delete crosses block
        // boundaries and leaves spanned survivors on both sides.
        let mut para_dots: Vec<Dot> = Vec::new();
        let mut char_dots: Vec<Dot> = Vec::new();
        let mut pos = 1usize;
        for p in 0..3 {
            if p > 0 {
                let d = warm
                    .apply(seq_block(pos, NodeType::Paragraph, vec![Dot::ROOT]))
                    .unwrap()
                    .id;
                para_dots.push(d);
                pos += 1;
            }
            for i in 0..600 {
                let ch = char::from(b'a' + (i % 26) as u8);
                let d = warm.apply(seq_char(pos, ch)).unwrap().id;
                char_dots.push(d);
                pos += 1;
            }
        }
        // Spans of varied kinds over varied ranges: fully-deleted, boundary-crossing,
        // and fully-surviving, plus overlaps that exercise last-writer-wins.
        let span = |warm: &mut ProjectedState, s: Dot, e: Dot, m: Modifier| {
            warm.apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: s,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: e,
                    bias: Bias::After,
                },
                modifier: m,
            }))
            .unwrap();
        };
        let n = char_dots.len();
        span(&mut warm, char_dots[0], char_dots[50], Modifier::Bold);
        span(&mut warm, char_dots[40], char_dots[900], Modifier::Italic);
        span(
            &mut warm,
            char_dots[800],
            char_dots[1200],
            Modifier::FontSize { value: 1400 },
        );
        span(
            &mut warm,
            char_dots[n - 100],
            char_dots[n - 1],
            Modifier::Bold,
        );
        span(
            &mut warm,
            char_dots[n - 60],
            char_dots[n - 5],
            Modifier::Italic,
        );
        span(
            &mut warm,
            char_dots[n - 100],
            char_dots[n - 1],
            Modifier::Italic,
        );

        let visible_before = warm.seq.visible_len();
        // Delete a large contiguous middle span: enough targets (> 1024) to force the
        // bulk coverage-preserving path, leaving spanned survivors at both ends.
        let del_len = 1400usize;
        let del_pos = 30usize;
        warm.apply(EditOp::Seq(ListOp::Del {
            pos: del_pos,
            len: del_len,
        }))
        .unwrap();
        assert!(
            warm.seq.visible_len() < visible_before,
            "delete shrank the doc"
        );

        let cold = ProjectedState::from_graph(warm.graph().clone()).expect("cold rebuild projects");
        assert_eq!(
            warm.projected(),
            cold.projected(),
            "bulk-delete reprojection diverged from cold rebuild"
        );
        warm.assert_seg_index_matches_logs();
    }

    // A block-start insert seeds the new leaf's span coverage from the right
    // neighbour's indexed coverage (mirroring the mid-block left-neighbour seed)
    // instead of re-resolving the whole span log per keystroke. The seeded
    // coverage must match the cold rebuild for spans that cross the block start
    // anchored far away, start exactly at the old first child, sit entirely in
    // another block, or anchor to a ghost at the seam — and the empty-block
    // fallback keeps deriving directly.
    #[test]
    fn block_start_insert_coverage_matches_cold() {
        let mut warm = ProjectedState::empty();
        let mut char_dots: Vec<Dot> = Vec::new();
        let mut pos = 1usize;
        for i in 0..40 {
            let d = warm
                .apply(seq_char(pos, char::from(b'a' + (i % 26) as u8)))
                .unwrap()
                .id;
            char_dots.push(d);
            pos += 1;
        }
        warm.apply(seq_block(pos, NodeType::Paragraph, vec![Dot::ROOT]))
            .unwrap();
        pos += 1;
        let p2_first = pos;
        for i in 0..40 {
            let d = warm
                .apply(seq_char(pos, char::from(b'a' + (i % 26) as u8)))
                .unwrap()
                .id;
            char_dots.push(d);
            pos += 1;
        }
        warm.apply(seq_block(pos, NodeType::Paragraph, vec![Dot::ROOT]))
            .unwrap();
        pos += 1;
        let p3_first = pos;
        warm.apply(seq_block(pos, NodeType::Paragraph, vec![Dot::ROOT]))
            .unwrap();
        pos += 1;
        for i in 0..20 {
            let d = warm
                .apply(seq_char(pos, char::from(b'a' + (i % 26) as u8)))
                .unwrap()
                .id;
            char_dots.push(d);
            pos += 1;
        }

        let span = |warm: &mut ProjectedState, s: Dot, e: Dot, m: Modifier| {
            warm.apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: s,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: e,
                    bias: Bias::After,
                },
                modifier: m,
            }))
            .unwrap();
        };
        // Crosses paragraph 2's start, anchored far from the seam.
        span(&mut warm, char_dots[10], char_dots[60], Modifier::Bold);
        // Starts exactly at paragraph 2's old first child.
        span(&mut warm, char_dots[40], char_dots[50], Modifier::Italic);
        // Entirely inside paragraph 1.
        span(
            &mut warm,
            char_dots[0],
            char_dots[5],
            Modifier::FontSize { value: 1400 },
        );
        // Crosses the empty paragraph 3.
        span(&mut warm, char_dots[70], char_dots[90], Modifier::Bold);
        // Ends exactly at paragraph 2's last char — deleted below, so its end
        // anchor becomes a ghost right before paragraph 3's marker.
        span(&mut warm, char_dots[45], char_dots[79], Modifier::Italic);

        // Ghost at the seam: delete paragraph 2's old first char (the Italic
        // span's start anchor), leaving a span anchored to a ghost right after
        // the block marker.
        warm.apply(EditOp::Seq(ListOp::Del {
            pos: p2_first,
            len: 1,
        }))
        .unwrap();
        // Ghost at the empty block's left seam: delete paragraph 2's last char.
        warm.apply(EditOp::Seq(ListOp::Del {
            pos: p3_first - 3,
            len: 1,
        }))
        .unwrap();

        // Block-start insert into non-empty paragraph 2 (right-neighbour seed).
        warm.apply(seq_char(p2_first, 'x')).unwrap();
        // First char of empty paragraph 3 (left-walk seed across the marker and
        // the seam ghost); the two deletes and the insert net to one removed
        // element, so the marker's gap sits one left of the build-time position.
        warm.apply(seq_char(p3_first - 1, 'y')).unwrap();

        let cold = ProjectedState::from_graph(warm.graph().clone()).expect("cold rebuild projects");
        assert_eq!(
            warm.projected(),
            cold.projected(),
            "block-start coverage seeding diverged from cold rebuild"
        );
    }

    // Deleting a whole middle paragraph takes the window-reprojection path (block-marker
    // removal), which now carries surviving leaves' span coverage forward instead of
    // re-resolving the span log. The survivors in the untouched paragraphs — some span-
    // covered, some not — must keep exactly the coverage a cold rebuild derives.
    #[test]
    fn window_reproject_preserves_survivor_span_coverage() {
        let mut warm = ProjectedState::empty();
        let mut para_char0: Vec<Dot> = Vec::new(); // first char dot of each paragraph
        let mut pos = 1usize;
        for p in 0..3usize {
            if p > 0 {
                warm.apply(seq_block(pos, NodeType::Paragraph, vec![Dot::ROOT]))
                    .unwrap();
                pos += 1;
            }
            let first = warm.apply(seq_char(pos, 'a')).unwrap().id;
            para_char0.push(first);
            pos += 1;
            for _ in 1..40 {
                warm.apply(seq_char(pos, 'b')).unwrap();
                pos += 1;
            }
        }
        // Span the whole first and third paragraphs (survivors), leaving the second
        // (about to be deleted) partly covered by a span that also reaches into a survivor.
        let span = |warm: &mut ProjectedState, s: Dot, e: Dot, m: Modifier| {
            warm.apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: s,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: e,
                    bias: Bias::After,
                },
                modifier: m,
            }))
            .unwrap();
        };
        span(&mut warm, para_char0[0], para_char0[1], Modifier::Bold);
        span(&mut warm, para_char0[2], para_char0[2], Modifier::Italic);

        // Delete the entire middle paragraph (its 40 chars + its block marker): a
        // window reprojection over survivors, not the bulk char path.
        let p2_start = 41usize; // 1 root para: 40 chars at pos 1..=40, para2 marker at 41
        warm.apply(EditOp::Seq(ListOp::Del {
            pos: p2_start,
            len: 41,
        }))
        .unwrap();

        let cold = ProjectedState::from_graph(warm.graph().clone()).expect("cold rebuild projects");
        assert_eq!(
            warm.projected(),
            cold.projected(),
            "window reprojection over survivors diverged from cold rebuild"
        );
    }

    // attr 보유 블록의 단일 op 삽입은 try_insert_block이 아니라 reproject_window로
    // 흐른다. window 경로가 init attrs를 node_attrs에 시딩하지 않으면 콜드 재구축과
    // 드리프트한다 — 이 테스트가 그 경로를 직접 운동시킨다.
    #[test]
    fn attr_block_single_op_insert_matches_cold() {
        let mut warm = ProjectedState::empty();
        for i in 0..3 {
            warm.apply(seq_char(i + 1, 'a')).unwrap();
        }
        let id = warm
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 4,
                item: SeqItem::Block {
                    node_type: NodeType::Callout,
                    parents: vec![Dot::ROOT],
                    attrs: vec![NodeAttr::Callout {
                        attr: CalloutNodeAttr::Variant(CalloutVariant::Warning),
                    }],
                },
            }))
            .unwrap()
            .id;
        let cold = ProjectedState::from_graph(warm.graph().clone()).unwrap();
        assert_eq!(
            warm.projected(),
            cold.projected(),
            "단일 op init 블록에서 warm이 콜드 재구축과 드리프트"
        );
        let node = warm
            .projected()
            .node_attrs
            .get(&id)
            .expect("init attrs가 node_attrs에 시딩돼야 한다");
        let Node::Callout(c) = node else {
            panic!("callout node expected");
        };
        assert_eq!(*c.variant.get(), CalloutVariant::Warning);
    }

    // Typing into a large, heavily-spanned document must stay incremental — no
    // full O(document) reprojection per keystroke even when many spans exist.
    #[test]
    fn large_spanned_doc_typing_is_subquadratic() {
        let n = 4000usize;
        let mut state = ProjectedState::empty();
        let mut dots = Vec::with_capacity(n);
        for i in 0..n {
            dots.push(state.apply(seq_char(i + 1, 'a')).unwrap().id);
        }
        // 40 overlapping bold spans scattered across the document (heavy formatting).
        for k in 0..40usize {
            let off = (k * 93) % (n - 60);
            state
                .apply(EditOp::Span(SpanOp::AddSpan {
                    start: Anchor {
                        id: dots[off],
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: dots[off + 50],
                        bias: Bias::After,
                    },
                    modifier: Modifier::Bold,
                }))
                .unwrap();
        }
        let m = 2000usize;
        for _ in 0..m {
            state.apply(seq_char(n / 2, 'b')).unwrap();
        }
        let cold = ProjectedState::from_graph(state.graph().clone()).unwrap();
        assert_eq!(state.projected(), cold.projected());
    }

    // Mix char ins/del + spans + every node op (block modifier, node style +
    // style registry, node carry) interleaved, then assert the incremental
    // spine converges to the cold full projection. Validates S4d's per-node-op
    // map updates + subtree effective recompute against the oracle.
    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 256, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn warm_apply_with_node_ops_matches_cold(
            actions in proptest::collection::vec(arb_span_action(), 0..50),
        ) {
            let mut warm = ProjectedState::empty();
            let para = warm
                .view()
                .root()
                .unwrap()
                .child_blocks()
                .next()
                .unwrap()
                .dot()
                .unwrap();
            let mut live: Vec<Dot> = Vec::new();
            let mut count = 1usize;
            for (kind, a, b, bias, ch) in actions {
                let pick = |i: u8, v: &[Dot]| v[(i as usize) % v.len()];
                let bias_s = if bias & 1 == 0 { Bias::Before } else { Bias::After };
                let bias_e = if bias & 2 == 0 { Bias::Before } else { Bias::After };
                match kind {
                    0..=4 => {
                        let pos = 1 + (a as usize) % count;
                        let d = warm.apply(seq_char(pos, ch)).unwrap().id;
                        live.push(d);
                        count += 1;
                    }
                    5..=6 if count > 1 => {
                        let pos = 1 + (a as usize) % (count - 1);
                        let len = 1 + (b as usize) % (count - pos);
                        warm.apply(EditOp::Seq(ListOp::Del { pos, len })).unwrap();
                        count -= len;
                    }
                    7..=8 if !live.is_empty() => {
                        let m = match bias % 3 {
                            0 => Modifier::Bold,
                            1 => Modifier::Italic,
                            _ => Modifier::FontSize { value: 1400 },
                        };
                        warm.apply(EditOp::Span(SpanOp::AddSpan {
                            start: Anchor { id: pick(a, &live), bias: bias_s },
                            end: Anchor { id: pick(b, &live), bias: bias_e },
                            modifier: m,
                        }))
                        .unwrap();
                    }
                    9..=10 => {
                        let op = if b & 1 == 0 {
                            ModifierAttrOp::SetModifier {
                                target: para,
                                modifier: Modifier::FontSize { value: 1200 + a as u32 },
                            }
                        } else {
                            ModifierAttrOp::ClearModifier {
                                target: para,
                                key: ModifierType::FontSize,
                            }
                        };
                        warm.apply(EditOp::BlockModifier(op)).unwrap();
                    }
                    14 => {
                        let op = if b & 1 == 0 {
                            ModifierAttrOp::SetModifier {
                                target: para,
                                modifier: Modifier::Bold,
                            }
                        } else {
                            ModifierAttrOp::ClearModifier {
                                target: para,
                                key: ModifierType::Bold,
                            }
                        };
                        warm.apply(EditOp::NodeCarry(op)).unwrap();
                    }
                    _ => {}
                }
            }
            let cold = ProjectedState::from_graph(warm.graph().clone())
                .expect("cold rebuild projects");
            proptest::prop_assert_eq!(warm.projected(), cold.projected());
            warm.assert_seg_index_matches_logs();
        }
    }

    // Freely-placeable inline atom (HardBreak/Tab) inserts and deletes mixed with
    // chars must stay incremental and converge to cold. (PageBreak is excluded: it
    // is position-constrained, so a mid-block insert is dropped by normalization —
    // a structural case that takes the fallback path.)
    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 192, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn warm_apply_with_atoms_matches_cold(
            actions in proptest::collection::vec(
                (0u8..10, proptest::prelude::any::<u8>(), proptest::prelude::any::<u8>(), 0u8..2),
                0..40,
            ),
        ) {
            use editor_model::AtomLeaf;
            let mut warm = ProjectedState::empty();
            let mut count = 1usize;
            for (kind, a, b, atom) in actions {
                match kind {
                    0..=4 => {
                        let pos = 1 + (a as usize) % count;
                        warm.apply(seq_char(pos, 'a')).unwrap();
                        count += 1;
                    }
                    5..=6 => {
                        let pos = 1 + (a as usize) % count;
                        let leaf = if atom == 0 { AtomLeaf::HardBreak } else { AtomLeaf::Tab };
                        warm.apply(EditOp::Seq(ListOp::Ins { pos, item: SeqItem::Atom(leaf) }))
                            .unwrap();
                        count += 1;
                    }
                    7..=8 if count > 1 => {
                        let pos = 1 + (a as usize) % (count - 1);
                        let len = 1 + (b as usize) % (count - pos);
                        warm.apply(EditOp::Seq(ListOp::Del { pos, len })).unwrap();
                        count -= len;
                    }
                    _ => {}
                }
            }
            let cold = ProjectedState::from_graph(warm.graph().clone())
                .expect("cold rebuild projects");
            proptest::prop_assert_eq!(warm.projected(), cold.projected());
        }
    }

    // Char inserts/deletes interleaved with top-level paragraph splits (Enter)
    // must stay incremental and converge to cold. Validates incremental block
    // insert (split_block_insert + index/run/effective updates).
    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 256, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn warm_apply_with_block_splits_matches_cold(
            actions in proptest::collection::vec(
                (0u8..10, proptest::prelude::any::<u8>(), proptest::prelude::any::<u8>(),
                 proptest::sample::select(vec!['a', 'b', 'c'])),
                0..44,
            ),
        ) {
            let mut warm = ProjectedState::empty();
            let mut count = 1usize;
            for (kind, a, b, ch) in actions {
                match kind {
                    0..=6 => {
                        let pos = 1 + (a as usize) % count;
                        warm.apply(seq_char(pos, ch)).unwrap();
                        count += 1;
                    }
                    7..=8 => {
                        let pos = 1 + (a as usize) % count;
                        warm.apply(seq_block(pos, NodeType::Paragraph, vec![Dot::ROOT])).unwrap();
                        count += 1;
                    }
                    _ if count > 1 => {
                        let pos = 1 + (a as usize) % (count - 1);
                        let len = 1 + (b as usize) % (count - pos);
                        warm.apply(EditOp::Seq(ListOp::Del { pos, len })).unwrap();
                        count -= len;
                    }
                    _ => {}
                }
            }
            let cold = ProjectedState::from_graph(warm.graph().clone())
                .expect("cold rebuild projects");
            proptest::prop_assert_eq!(warm.projected(), cold.projected());
        }
    }

    // Randomized structural stress: chars, top-level paragraph splits, blockquote
    // inserts, paragraphs nested in blockquotes, and range deletes (which merge
    // across block boundaries). All must converge to cold via incremental paths +
    // localized window re-projection — no full reproject on the editing path.
    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 256, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn warm_apply_structural_matches_cold(
            actions in proptest::collection::vec(
                (0u8..14, proptest::prelude::any::<u8>(), proptest::prelude::any::<u8>(),
                 proptest::sample::select(vec!['a', 'b', 'c'])),
                0..48,
            ),
        ) {
            let root = Dot::ROOT;
            let mut warm = ProjectedState::empty();
            let mut count = 1usize;
            let mut blockquotes: Vec<Dot> = Vec::new();
            for (kind, a, b, ch) in actions {
                match kind {
                    0..=5 => {
                        let pos = 1 + (a as usize) % count;
                        warm.apply(seq_char(pos, ch)).unwrap();
                        count += 1;
                    }
                    6..=7 => {
                        // `% (count + 1)` allows pos 0 — a top-level block inserted
                        // at the very document front, landing before the first
                        // block's marker (the `lift`-to-front shape).
                        let pos = (a as usize) % (count + 1);
                        warm.apply(seq_block(pos, NodeType::Paragraph, vec![root])).unwrap();
                        count += 1;
                    }
                    8 => {
                        let pos = (a as usize) % (count + 1);
                        let d = warm
                            .apply(seq_block(pos, NodeType::Blockquote, vec![root]))
                            .unwrap()
                            .id;
                        blockquotes.push(d);
                        count += 1;
                    }
                    9..=10 if !blockquotes.is_empty() => {
                        let bq = blockquotes[(a as usize) % blockquotes.len()];
                        let pos = 1 + (a as usize) % count;
                        warm.apply(seq_block(pos, NodeType::Paragraph, vec![root, bq])).unwrap();
                        count += 1;
                    }
                    _ if count > 1 => {
                        let pos = 1 + (a as usize) % (count - 1);
                        let len = 1 + (b as usize) % (count - pos);
                        warm.apply(EditOp::Seq(ListOp::Del { pos, len })).unwrap();
                        count -= len;
                    }
                    _ => {}
                }
            }
            let cold = ProjectedState::from_graph(warm.graph().clone())
                .expect("cold rebuild projects");
            proptest::prop_assert_eq!(warm.projected(), cold.projected());
        }
    }

    // Chars/blocks + spans + range deletes + undeletes (undo of a delete) must
    // converge to cold. Undel routes through localized re-projection (not a full
    // reproject), and restoring span-covered leaves exercises the coverage rebuild.
    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 256, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn warm_apply_with_undel_matches_cold(
            actions in proptest::collection::vec(
                (0u8..12, proptest::prelude::any::<u8>(), proptest::prelude::any::<u8>(),
                 proptest::sample::select(vec!['a', 'b', 'c'])),
                0..44,
            ),
        ) {
            let root = Dot::ROOT;
            let mut warm = ProjectedState::empty();
            let mut count = 1usize;
            let mut dels: Vec<(Dot, usize)> = Vec::new();
            let mut live: Vec<Dot> = Vec::new();
            for (kind, a, b, ch) in actions {
                match kind {
                    0..=4 => {
                        let pos = 1 + (a as usize) % count;
                        let d = warm.apply(seq_char(pos, ch)).unwrap().id;
                        live.push(d);
                        count += 1;
                    }
                    5 => {
                        let pos = 1 + (a as usize) % count;
                        warm.apply(seq_block(pos, NodeType::Paragraph, vec![root])).unwrap();
                        count += 1;
                    }
                    6..=7 if !live.is_empty() => {
                        let s = live[(a as usize) % live.len()];
                        let e = live[(b as usize) % live.len()];
                        let m = if a & 1 == 0 { Modifier::Bold } else { Modifier::Italic };
                        warm.apply(EditOp::Span(SpanOp::AddSpan {
                            start: Anchor { id: s, bias: Bias::Before },
                            end: Anchor { id: e, bias: Bias::After },
                            modifier: m,
                        }))
                        .unwrap();
                    }
                    8..=9 if count > 1 => {
                        let pos = 1 + (a as usize) % (count - 1);
                        let len = 1 + (b as usize) % (count - pos);
                        let op = warm.apply(EditOp::Seq(ListOp::Del { pos, len })).unwrap();
                        dels.push((op.id, len));
                        count -= len;
                    }
                    10..=11 if !dels.is_empty() => {
                        let i = (a as usize) % dels.len();
                        let (del, len) = dels.remove(i);
                        warm.apply(EditOp::Seq(ListOp::Undel { del })).unwrap();
                        count += len;
                    }
                    _ => {}
                }
            }
            let cold = ProjectedState::from_graph(warm.graph().clone())
                .expect("cold rebuild projects");
            proptest::prop_assert_eq!(warm.projected(), cold.projected());
            warm.assert_seg_index_matches_logs();
        }
    }

    // Inline atom inserts including the position-constrained PageBreak, mixed with
    // chars and range deletes. A mid-text PageBreak is dropped (with trailing
    // content) by normalization; deleting it must locally restore that content.
    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 400, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn warm_apply_with_pagebreaks_matches_cold(
            actions in proptest::collection::vec(
                (0u8..10, proptest::prelude::any::<u8>(), proptest::prelude::any::<u8>(), 0u8..3),
                0..40,
            ),
        ) {
            use editor_model::AtomLeaf;
            let mut warm = ProjectedState::empty();
            let mut count = 1usize;
            for (kind, a, b, atom) in actions {
                match kind {
                    0..=5 => { let pos = 1 + (a as usize) % count; warm.apply(seq_char(pos, 'a')).unwrap(); count += 1; }
                    6..=7 => {
                        let pos = 1 + (a as usize) % count;
                        let leaf = match atom { 0 => AtomLeaf::HardBreak, 1 => AtomLeaf::Tab, _ => AtomLeaf::PageBreak };
                        warm.apply(EditOp::Seq(ListOp::Ins { pos, item: SeqItem::Atom(leaf) })).unwrap();
                        count += 1;
                    }
                    _ if count > 1 => {
                        let pos = 1 + (a as usize) % (count - 1);
                        let len = 1 + (b as usize) % (count - pos);
                        warm.apply(EditOp::Seq(ListOp::Del { pos, len })).unwrap();
                        count -= len;
                    }
                    _ => {}
                }
            }
            let cold = ProjectedState::from_graph(warm.graph().clone()).unwrap();
            proptest::prop_assert_eq!(warm.projected(), cold.projected());
        }
    }

    #[test]
    fn nested_block_edits_match_cold() {
        let mut state = ProjectedState::empty();
        let root = Dot::ROOT;
        let bq = state
            .apply(seq_block(1, NodeType::Blockquote, vec![root]))
            .unwrap()
            .id;
        state
            .apply(seq_block(2, NodeType::Paragraph, vec![root, bq]))
            .unwrap();
        for (i, ch) in ['a', 'b', 'c', 'd'].iter().enumerate() {
            state.apply(seq_char(3 + i, *ch)).unwrap();
        }
        // Split the inner paragraph (a new paragraph nested in the blockquote).
        state
            .apply(seq_block(5, NodeType::Paragraph, vec![root, bq]))
            .unwrap();
        let cold = ProjectedState::from_graph(state.graph().clone()).unwrap();
        assert_eq!(state.projected(), cold.projected());
        // A delete spanning the nested split, then re-check.
        state
            .apply(EditOp::Seq(ListOp::Del { pos: 3, len: 3 }))
            .unwrap();
        let cold2 = ProjectedState::from_graph(state.graph().clone()).unwrap();
        assert_eq!(state.projected(), cold2.projected());
    }

    #[test]
    fn incremental_window_reprojection_reports_no_totality_violation() {
        // Drive the incremental window-splice path (a nested block split, then a delete
        // spanning it) and confirm the shared totality predicate — wired into that
        // path, not just the full projection — charges nothing. The cold cross-check
        // proves the window really rebuilt the whole document, so the zero is earned.
        let mut state = ProjectedState::empty();
        let root = Dot::ROOT;
        let bq = state
            .apply(seq_block(1, NodeType::Blockquote, vec![root]))
            .unwrap()
            .id;
        state
            .apply(seq_block(2, NodeType::Paragraph, vec![root, bq]))
            .unwrap();
        for (i, ch) in ['a', 'b', 'c', 'd'].iter().enumerate() {
            state.apply(seq_char(3 + i, *ch)).unwrap();
        }
        state
            .apply(seq_block(5, NodeType::Paragraph, vec![root, bq]))
            .unwrap();
        state
            .apply(EditOp::Seq(ListOp::Del { pos: 3, len: 3 }))
            .unwrap();
        let cold = ProjectedState::from_graph(state.graph().clone()).unwrap();
        assert_eq!(state.projected(), cold.projected());
        assert_eq!(
            state.take_repair_stats().totality_violations,
            0,
            "the incremental window path places every visible element"
        );
    }

    // Regression: after a Blockquote gains nested `[root, blockquote]` paragraphs and a
    // range delete removes the top-level paragraph separating them, the localized window
    // must extend over the following nested paragraphs (they reparent back into the now
    // re-opened Blockquote) so the incremental result matches a cold rebuild instead of
    // stranding one at Root. The deterministic `(kind, a, b)` script drives the exact
    // interleaving that first exposed the window-boundary shortfall.
    #[test]
    fn nested_blockquote_delete_reparents_paragraph_diverging_from_cold() {
        let root = Dot::ROOT;
        let mut warm = ProjectedState::empty();
        let mut count = 1usize;
        let mut bqs: Vec<Dot> = Vec::new();
        let script: [(u8, usize, usize); 13] = [
            (0, 0, 0),
            (8, 17, 0),
            (9, 2, 0),
            (0, 0, 0),
            (9, 148, 0),
            (6, 146, 0),
            (0, 42, 0),
            (6, 104, 0),
            (0, 140, 0),
            (0, 0, 0),
            (11, 135, 25),
            (0, 0, 0),
            (0, 107, 0),
        ];
        for (kind, a, b) in script {
            match kind {
                0..=5 => {
                    let pos = 1 + a % count;
                    warm.apply(seq_char(pos, 'a')).unwrap();
                    count += 1;
                }
                6..=7 => {
                    let pos = a % (count + 1);
                    warm.apply(seq_block(pos, NodeType::Paragraph, vec![root]))
                        .unwrap();
                    count += 1;
                }
                8 => {
                    let pos = a % (count + 1);
                    let d = warm
                        .apply(seq_block(pos, NodeType::Blockquote, vec![root]))
                        .unwrap()
                        .id;
                    bqs.push(d);
                    count += 1;
                }
                9..=10 if !bqs.is_empty() => {
                    let bq = bqs[a % bqs.len()];
                    let pos = 1 + a % count;
                    warm.apply(seq_block(pos, NodeType::Paragraph, vec![root, bq]))
                        .unwrap();
                    count += 1;
                }
                _ if count > 1 => {
                    let pos = 1 + a % (count - 1);
                    let len = 1 + b % (count - pos);
                    warm.apply(EditOp::Seq(ListOp::Del { pos, len })).unwrap();
                    count -= len;
                }
                _ => {}
            }
        }
        let cold = ProjectedState::from_graph(warm.graph().clone()).unwrap();
        assert_eq!(warm.projected(), cold.projected());
    }

    // Merge gate: a zombie-bearing history built purely cold, then a single
    // incremental `apply` adjacent to the zombie. A mid-paragraph PageBreak
    // SPLIT-HOISTs the trailing "bcde" run into synthetic scaffold paragraphs —
    // visible in the sequence, restructured in the tree. Rebuilding the base with
    // `from_graph` gives the incremental step no prior warm structure to lean on; the
    // real apply path (graph + projection, not `apply_warm_only`) must then converge
    // to a cold rebuild of the resulting graph rather than regrouping the split.
    #[test]
    fn incremental_apply_matches_cold_rebuild_with_zombie_history() {
        use editor_model::AtomLeaf;
        let mut authored = ProjectedState::empty();
        for (i, ch) in ['a', 'b', 'c', 'd', 'e'].iter().enumerate() {
            authored.apply(seq_char(1 + i, *ch)).unwrap();
        }
        authored
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 2,
                item: SeqItem::Atom(AtomLeaf::PageBreak),
            }))
            .unwrap();
        let mut incremental = ProjectedState::from_graph(authored.graph().clone()).unwrap();
        incremental.apply(seq_char(5, 'Z')).unwrap();
        let full = ProjectedState::from_graph(incremental.graph().clone()).unwrap();
        assert_eq!(incremental.projected(), full.projected());
    }

    // Regression (warm path): a reversed `Fold` — its `FoldContent` marker precedes
    // its `FoldTitle` in the sequence — is built cold, then a single incremental
    // `apply` inserts a Root-level Paragraph after it. That window reprojection
    // SPLIT-HOISTs the container-internal `FoldTitle` (holding 't') to Root, where the
    // Root content rule WRAPs it into a scaffold Fold. The shallow reconcile must
    // reparent the *existing* real `FoldTitle` by id — not clobber it with the empty
    // proxy nor forget it — or 't' is orphaned from the tree while a cold rebuild
    // keeps it. Asserts the reparent path runs to convergence and loses no dot. This
    // is the sole test that drives `graft_shallow_scaffold`'s reparent branch through
    // the ProjectedState warm path (the editor-transaction reversed-Fold tests assert
    // index-based preservation, which passes even when the block is tree-orphaned).
    #[test]
    fn incremental_insert_after_reversed_fold_preserves_hoisted_title_text() {
        let root = Dot::ROOT;
        let mut g = OpGraph::<EditOp>::with_actor(1);
        let fold = g
            .add_mut(seq_block(0, NodeType::Fold, vec![root]))
            .unwrap()
            .id;
        let content = g
            .add_mut(seq_block(1, NodeType::FoldContent, vec![root, fold]))
            .unwrap()
            .id;
        g.add_mut(seq_block(2, NodeType::Paragraph, vec![root, fold, content]))
            .unwrap();
        let title = g
            .add_mut(seq_block(3, NodeType::FoldTitle, vec![root, fold]))
            .unwrap()
            .id;
        let t = g.add_mut(seq_char(4, 't')).unwrap().id;

        let mut warm = ProjectedState::from_graph(g).unwrap();
        // Cold base keeps the whole reversed Fold reachable — the 't' included.
        assert!(
            reachable_real_dots(warm.projected()).contains(&t),
            "cold base keeps the reversed FoldTitle's text"
        );

        // One incremental Root-level insert after the Fold — drives the window
        // reprojection whose hoist + shallow WRAP formerly orphaned 't'.
        warm.apply(seq_block(5, NodeType::Paragraph, vec![root]))
            .unwrap();

        let cold = ProjectedState::from_graph(warm.graph().clone()).unwrap();
        assert_eq!(
            warm.projected(),
            cold.projected(),
            "incremental reparent of the hoisted FoldTitle converges to a cold rebuild"
        );
        let warm_reach = reachable_real_dots(warm.projected());
        for d in [fold, content, title, t] {
            assert!(
                warm_reach.contains(&d),
                "{d:?} stays reachable in the warm tree (no orphan)"
            );
        }
    }

    #[test]
    fn accessor_smoke_block_modifier_span() {
        use editor_model::{Anchor, Bias, ModifierAttrOp, SpanOp};

        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        let x = state.apply(seq_char(1, 'x')).unwrap().id;

        state
            .apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: para,
                modifier: Modifier::FontSize { value: 1400 },
            }))
            .unwrap();
        state
            .apply(EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: x,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: x,
                    bias: Bias::After,
                },
                modifier: Modifier::Bold,
            }))
            .unwrap();

        assert_eq!(
            state
                .block_modifiers()
                .modifiers_of(para)
                .get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1400 })
        );
        assert!(state.spans().iter().count() > 0);
    }

    #[test]
    fn seq_flat_pos_identifies_char_and_del_removes_it() {
        use editor_crdt::ListOp;

        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();

        let a_dot = state.apply(seq_char(1, 'a')).unwrap().id;
        state.apply(seq_char(2, 'b')).unwrap();

        assert_eq!(state.view().node(para).unwrap().inline_text(), "ab");

        let pos = state.seq_flat_pos(a_dot).expect("dot exists in seq");
        state
            .apply(EditOp::Seq(ListOp::Del { pos, len: 1 }))
            .unwrap();
        assert_eq!(state.view().node(para).unwrap().inline_text(), "b");
    }

    #[test]
    fn apply_batch_equivalent_to_per_op() {
        let mut batched = ProjectedState::empty();
        let para = batched
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        let batch_ops = batched
            .apply_batch(vec![seq_char(1, 'a'), seq_char(2, 'b'), seq_char(3, 'c')])
            .unwrap();
        assert_eq!(batch_ops.len(), 3);
        assert_eq!(batched.view().node(para).unwrap().inline_text(), "abc");

        let mut distinct: hashbrown::HashSet<Dot> = hashbrown::HashSet::new();
        for op in &batch_ops {
            assert!(distinct.insert(op.id), "returned dots must be distinct");
        }

        let mut per_op = ProjectedState::empty();
        let per_para = per_op
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        let a = per_op.apply(seq_char(1, 'a')).unwrap();
        let b = per_op.apply(seq_char(2, 'b')).unwrap();
        let c = per_op.apply(seq_char(3, 'c')).unwrap();

        assert_eq!(per_para, para);
        assert_eq!(
            per_op.view().node(per_para).unwrap().inline_text(),
            batched.view().node(para).unwrap().inline_text()
        );

        assert_eq!(batch_ops[0].id, a.id);
        assert_eq!(batch_ops[1].id, b.id);
        assert_eq!(batch_ops[2].id, c.id);
    }

    #[test]
    fn apply_batch_returned_dots_resolve_in_seq() {
        let mut state = ProjectedState::empty();
        let ops = state
            .apply_batch(vec![seq_char(1, 'a'), seq_char(2, 'b'), seq_char(3, 'c')])
            .unwrap();
        let positions: Vec<usize> = ops
            .iter()
            .map(|op| {
                state
                    .seq_flat_pos(op.id)
                    .expect("returned dot resolves in seq")
            })
            .collect();
        assert_eq!(positions, vec![1, 2, 3]);
    }

    #[test]
    fn large_single_paragraph_paste_is_correct_and_subquadratic() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        let n = 5000usize;
        for i in 0..n {
            state.apply(seq_char(1 + i, 'a')).expect("char applies");
        }
        let text = state.view().node(para).unwrap().inline_text();
        assert_eq!(text.len(), n);
        assert!(text.chars().all(|c| c == 'a'));
        let cold = ProjectedState::from_graph(state.graph().clone()).unwrap();
        assert_eq!(state.projected(), cold.projected());
    }

    #[test]
    fn large_type_and_backspace_is_correct_and_subquadratic() {
        let mut state = ProjectedState::empty();
        let para = state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        let n = 3000usize;
        for i in 0..n {
            state.apply(seq_char(1 + i, 'a')).expect("char applies");
        }
        for _ in 0..n {
            state
                .apply(EditOp::Seq(ListOp::Del { pos: 1, len: 1 }))
                .expect("delete applies");
        }
        assert_eq!(state.view().node(para).unwrap().inline_text(), "");
        let cold = ProjectedState::from_graph(state.graph().clone()).unwrap();
        assert_eq!(state.projected(), cold.projected());
    }

    // Builds a paragraph "abcde" then inserts a PageBreak right after 'a'. The block
    // content rule keeps `[a, PageBreak]` and drops the trailing `b,c,d,e` from the
    // tree — they remain in the live sequence as "ghosts" (visible to seq positions,
    // invisible in the projected tree). Returns the warm state; the four ghosts sit
    // at visible positions 3..=6.
    fn ghost_region_state() -> ProjectedState {
        use editor_model::AtomLeaf;
        let mut warm = ProjectedState::empty();
        for (i, ch) in ['a', 'b', 'c', 'd', 'e'].iter().enumerate() {
            warm.apply(seq_char(1 + i, *ch)).unwrap();
        }
        warm.apply(EditOp::Seq(ListOp::Ins {
            pos: 2,
            item: SeqItem::Atom(AtomLeaf::PageBreak),
        }))
        .unwrap();
        // Guard the geometry: only "a" survives in the tree, four ghosts trail it.
        let para_text = warm
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .inline_text();
        assert_eq!(para_text, "a", "expected a ghost region: only 'a' in tree");
        assert_eq!(warm.seq.visible_len(), 7, "expected 4 trailing ghosts");
        warm
    }

    // An insert whose ±1 sequence neighbours are all ghosts (deep inside the dropped
    // trailing run) must still localize to the enclosing top-level block — never a
    // full-document reprojection.
    #[test]
    fn insert_into_ghost_region_stays_local() {
        let mut warm = ghost_region_state();
        // Insert between ghosts c@4 and d@5: neighbours 4,5,6 are all ghosts.
        warm.apply(seq_char(5, 'Z')).unwrap();
        let cold = ProjectedState::from_graph(warm.graph().clone()).unwrap();
        assert_eq!(warm.projected(), cold.projected());
    }

    // The overarching guarantee: realistic editing (chars, inline atoms incl. the
    // position-constrained PageBreak, range deletes, and Blockquote inserts that force
    // a synthetic trailing paragraph) must NEVER trigger a whole-document reprojection
    // — every op stays localized — while still converging to the cold projection.
    // Deterministic seeds keep this reproducible (not flaky) across ~78k ops.
    #[test]
    fn realistic_editing_never_full_reprojects() {
        use editor_model::AtomLeaf;
        let mut total_ops = 0u64;
        for seed in 0..1000u64 {
            // cheap deterministic PRNG (Date/rand unavailable in this context)
            let mut s = seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(1);
            let mut next = || {
                s ^= s << 13;
                s ^= s >> 7;
                s ^= s << 17;
                s
            };
            let mut warm = ProjectedState::empty();
            let mut count = 1usize;
            for _ in 0..40 {
                let kind = next() % 12;
                let a = (next() % 256) as usize;
                let b = (next() % 256) as usize;
                match kind {
                    10 => {
                        // structural: insert a Blockquote — when it lands last, the Root
                        // content rule forces a synthetic trailing paragraph after it.
                        let pos = 1 + a % count;
                        warm.apply(seq_block(pos, NodeType::Blockquote, vec![Dot::ROOT]))
                            .unwrap();
                        count += 1;
                    }
                    0..=5 => {
                        let pos = 1 + a % count;
                        warm.apply(seq_char(pos, 'a')).unwrap();
                        count += 1;
                    }
                    6..=7 => {
                        let pos = 1 + a % count;
                        let leaf = match next() % 3 {
                            0 => AtomLeaf::HardBreak,
                            1 => AtomLeaf::Tab,
                            _ => AtomLeaf::PageBreak,
                        };
                        warm.apply(EditOp::Seq(ListOp::Ins {
                            pos,
                            item: SeqItem::Atom(leaf),
                        }))
                        .unwrap();
                        count += 1;
                    }
                    _ if count > 1 => {
                        let pos = 1 + a % (count - 1);
                        let len = 1 + b % (count - pos);
                        warm.apply(EditOp::Seq(ListOp::Del { pos, len })).unwrap();
                        count -= len;
                    }
                    _ => continue,
                }
                total_ops += 1;
            }
            // Convergence: the localized warm projection must equal a cold rebuild —
            // including the subtle structural case where a real block appended after a
            // synthetic trailing paragraph must supersede (not duplicate) the scaffold.
            let cold = ProjectedState::from_graph(warm.graph().clone()).unwrap();
            assert_eq!(warm.projected(), cold.projected(), "diverged (seed={seed})");
        }
        assert!(
            total_ops > 30_000,
            "expected broad coverage, got {total_ops}"
        );
    }

    // A delete whose every target is a ghost (the whole range was already dropped
    // from the tree) must also localize to the enclosing block, never full-reproject.
    #[test]
    fn delete_of_ghost_range_stays_local() {
        let mut warm = ghost_region_state();
        // Delete ghosts c@4 and d@5.
        warm.apply(EditOp::Seq(ListOp::Del { pos: 4, len: 2 }))
            .unwrap();
        let cold = ProjectedState::from_graph(warm.graph().clone()).unwrap();
        assert_eq!(warm.projected(), cold.projected());
    }

    // ---- Property oracles: the design's invariants, machine-checked over an
    // adversarial causal-script generator. Each op is a single-actor `add_mut`, so
    // clocks stay dense and positions stay in range; the projection parents chain
    // (not the CRDT causal parents) carries the injected damage.

    type CausalStep = (u8, u8, u8, u8);

    fn adversarial_block_type(y: usize) -> NodeType {
        match y % 6 {
            0 => NodeType::Paragraph,
            1 => NodeType::FoldTitle,
            2 => NodeType::ListItem,
            3 => NodeType::TableCell,
            4 => NodeType::FoldContent,
            _ => NodeType::TableRow,
        }
    }

    // Interpret `steps` into an `OpGraph`. `adversarial == false` seeds an immortal
    // leading Paragraph and restricts moves to char/marker Ins (pos >= 1), Del
    // (pos >= 1) and Undel, so nothing is ever orphaned at Root — a clean history
    // that projects with zero repairs. `adversarial == true` injects dead-parent
    // markers, marker-boundary chars, reversed/duplicate fixed slots, ListItem
    // surplus, nested Table, PageBreak inside a Blockquote and classless Unknown.
    fn build_causal_script(steps: &[CausalStep], adversarial: bool) -> OpGraph<EditOp> {
        let root = Dot::ROOT;
        let ghost = Dot::new(9, 999);
        let alphabet = ['a', 'b', 'c', 'd'];
        let mut g = OpGraph::<EditOp>::with_actor(1);
        let mut count: usize = 0;
        let mut blocks: Vec<Dot> = Vec::new();
        let mut containers: Vec<Dot> = Vec::new();
        let mut dels: Vec<(Dot, usize)> = Vec::new();

        if !adversarial {
            g.add_mut(seq_block(0, NodeType::Paragraph, vec![root]))
                .unwrap();
            count = 1;
        }

        // Clean-only: trailing run of freely-deletable chars (reset by any marker), so a
        // delete never removes a block marker and orphans a container child.
        let mut tail_chars: usize = 0;

        for &(op, x, y, z) in steps {
            let x = x as usize;
            let y = y as usize;
            let ch = alphabet[x % alphabet.len()];

            if !adversarial {
                match op % 6 {
                    0 | 1 => {
                        g.add_mut(seq_char(count, ch)).unwrap();
                        count += 1;
                        tail_chars += 1;
                    }
                    2 => {
                        g.add_mut(seq_block(count, NodeType::Paragraph, vec![root]))
                            .unwrap();
                        count += 1;
                        tail_chars = 0;
                    }
                    3 => {
                        // Well-formed Fold: Title, Content[Paragraph] — schema order.
                        let fold = g
                            .add_mut(seq_block(count, NodeType::Fold, vec![root]))
                            .unwrap()
                            .id;
                        count += 1;
                        g.add_mut(seq_block(count, NodeType::FoldTitle, vec![root, fold]))
                            .unwrap();
                        count += 1;
                        g.add_mut(seq_char(count, ch)).unwrap();
                        count += 1;
                        let content = g
                            .add_mut(seq_block(count, NodeType::FoldContent, vec![root, fold]))
                            .unwrap()
                            .id;
                        count += 1;
                        g.add_mut(seq_block(
                            count,
                            NodeType::Paragraph,
                            vec![root, fold, content],
                        ))
                        .unwrap();
                        count += 1;
                        g.add_mut(seq_char(count, ch)).unwrap();
                        count += 1;
                        tail_chars = 0;
                    }
                    4 => {
                        // Well-formed BulletList/OrderedList: ListItem[Paragraph].
                        let list_ty = if y & 1 == 0 {
                            NodeType::BulletList
                        } else {
                            NodeType::OrderedList
                        };
                        let list = g.add_mut(seq_block(count, list_ty, vec![root])).unwrap().id;
                        count += 1;
                        let item = g
                            .add_mut(seq_block(count, NodeType::ListItem, vec![root, list]))
                            .unwrap()
                            .id;
                        count += 1;
                        g.add_mut(seq_block(
                            count,
                            NodeType::Paragraph,
                            vec![root, list, item],
                        ))
                        .unwrap();
                        count += 1;
                        g.add_mut(seq_char(count, ch)).unwrap();
                        count += 1;
                        tail_chars = 0;
                    }
                    5 if z & 1 == 0 => {
                        // Well-formed Table: Row[Cell[Paragraph]].
                        let table = g
                            .add_mut(seq_block(count, NodeType::Table, vec![root]))
                            .unwrap()
                            .id;
                        count += 1;
                        let row = g
                            .add_mut(seq_block(count, NodeType::TableRow, vec![root, table]))
                            .unwrap()
                            .id;
                        count += 1;
                        let cell = g
                            .add_mut(seq_block(
                                count,
                                NodeType::TableCell,
                                vec![root, table, row],
                            ))
                            .unwrap()
                            .id;
                        count += 1;
                        g.add_mut(seq_block(
                            count,
                            NodeType::Paragraph,
                            vec![root, table, row, cell],
                        ))
                        .unwrap();
                        count += 1;
                        g.add_mut(seq_char(count, ch)).unwrap();
                        count += 1;
                        tail_chars = 0;
                    }
                    _ => {
                        if tail_chars > 0 {
                            let d = 1 + y % tail_chars;
                            let pos = count - d;
                            let del = g
                                .add_mut(EditOp::Seq(ListOp::Del { pos, len: d }))
                                .unwrap()
                                .id;
                            dels.push((del, d));
                            count -= d;
                            tail_chars -= d;
                        } else if let Some((del, len)) = dels.pop() {
                            g.add_mut(EditOp::Seq(ListOp::Undel { del })).unwrap();
                            count += len;
                        }
                    }
                }
                continue;
            }

            match op % 16 {
                0..=2 => {
                    let pos = x % (count + 1);
                    g.add_mut(seq_char(pos, ch)).unwrap();
                    count += 1;
                }
                3 => {
                    let pos = x % (count + 1);
                    let d = g
                        .add_mut(seq_block(pos, NodeType::Paragraph, vec![root]))
                        .unwrap()
                        .id;
                    blocks.push(d);
                    count += 1;
                }
                4 => {
                    let pos = x % (count + 1);
                    let d = g
                        .add_mut(seq_block(pos, NodeType::Blockquote, vec![root]))
                        .unwrap()
                        .id;
                    blocks.push(d);
                    containers.push(d);
                    count += 1;
                }
                5 => {
                    let parent = if !containers.is_empty() {
                        containers[x % containers.len()]
                    } else if !blocks.is_empty() {
                        blocks[x % blocks.len()]
                    } else {
                        root
                    };
                    let chain = if parent == root {
                        vec![root]
                    } else {
                        vec![root, parent]
                    };
                    let pos = x % (count + 1);
                    let d = g
                        .add_mut(seq_block(pos, NodeType::Paragraph, chain))
                        .unwrap()
                        .id;
                    blocks.push(d);
                    count += 1;
                }
                6 => {
                    if count > 1 {
                        let pos = 1 + x % (count - 1);
                        let len = 1 + y % (count - pos);
                        let d = g.add_mut(EditOp::Seq(ListOp::Del { pos, len })).unwrap().id;
                        dels.push((d, len));
                        count -= len;
                    }
                }
                7 => {
                    if let Some((d, len)) = dels.pop() {
                        g.add_mut(EditOp::Seq(ListOp::Undel { del: d })).unwrap();
                        count += len;
                    }
                }
                8 => {
                    let pos = x % (count + 1);
                    let nt = adversarial_block_type(y);
                    let d = g.add_mut(seq_block(pos, nt, vec![root, ghost])).unwrap().id;
                    blocks.push(d);
                    count += 1;
                }
                9 => {
                    let pos = x % (count + 1);
                    let d = g
                        .add_mut(seq_block(pos, NodeType::Unknown, vec![root]))
                        .unwrap()
                        .id;
                    blocks.push(d);
                    count += 1;
                }
                10 => {
                    let pos = x % (count + 1);
                    let leaf = match y % 3 {
                        0 => AtomLeaf::HardBreak,
                        1 => AtomLeaf::Tab,
                        _ => AtomLeaf::PageBreak,
                    };
                    g.add_mut(EditOp::Seq(ListOp::Ins {
                        pos,
                        item: SeqItem::Atom(leaf),
                    }))
                    .unwrap();
                    count += 1;
                }
                11 => {
                    let fold = g
                        .add_mut(seq_block(count, NodeType::Fold, vec![root]))
                        .unwrap()
                        .id;
                    count += 1;
                    if y & 1 == 1 {
                        let c = g
                            .add_mut(seq_block(count, NodeType::FoldContent, vec![root, fold]))
                            .unwrap()
                            .id;
                        count += 1;
                        g.add_mut(seq_block(count, NodeType::Paragraph, vec![root, fold, c]))
                            .unwrap();
                        count += 1;
                        g.add_mut(seq_char(count, 'f')).unwrap();
                        count += 1;
                        g.add_mut(seq_block(count, NodeType::FoldTitle, vec![root, fold]))
                            .unwrap();
                        count += 1;
                        g.add_mut(seq_char(count, 't')).unwrap();
                        count += 1;
                    } else {
                        g.add_mut(seq_block(count, NodeType::FoldTitle, vec![root, fold]))
                            .unwrap();
                        count += 1;
                        g.add_mut(seq_char(count, 't')).unwrap();
                        count += 1;
                        if z & 1 == 1 {
                            g.add_mut(seq_block(count, NodeType::FoldTitle, vec![root, fold]))
                                .unwrap();
                            count += 1;
                            g.add_mut(seq_char(count, 'u')).unwrap();
                            count += 1;
                        }
                        let c = g
                            .add_mut(seq_block(count, NodeType::FoldContent, vec![root, fold]))
                            .unwrap()
                            .id;
                        count += 1;
                        g.add_mut(seq_block(count, NodeType::Paragraph, vec![root, fold, c]))
                            .unwrap();
                        count += 1;
                        g.add_mut(seq_char(count, 'f')).unwrap();
                        count += 1;
                    }
                    blocks.push(fold);
                }
                12 => {
                    let list_ty = if y & 1 == 0 {
                        NodeType::BulletList
                    } else {
                        NodeType::OrderedList
                    };
                    let list = g.add_mut(seq_block(count, list_ty, vec![root])).unwrap().id;
                    count += 1;
                    let item = g
                        .add_mut(seq_block(count, NodeType::ListItem, vec![root, list]))
                        .unwrap()
                        .id;
                    count += 1;
                    g.add_mut(seq_block(
                        count,
                        NodeType::Paragraph,
                        vec![root, list, item],
                    ))
                    .unwrap();
                    count += 1;
                    g.add_mut(seq_char(count, 'l')).unwrap();
                    count += 1;
                    if z & 1 == 1 {
                        g.add_mut(seq_block(
                            count,
                            NodeType::Paragraph,
                            vec![root, list, item],
                        ))
                        .unwrap();
                        count += 1;
                        g.add_mut(seq_char(count, 'm')).unwrap();
                        count += 1;
                    }
                    blocks.push(list);
                }
                13 => {
                    let table = g
                        .add_mut(seq_block(count, NodeType::Table, vec![root]))
                        .unwrap()
                        .id;
                    count += 1;
                    let row = g
                        .add_mut(seq_block(count, NodeType::TableRow, vec![root, table]))
                        .unwrap()
                        .id;
                    count += 1;
                    let cell = g
                        .add_mut(seq_block(
                            count,
                            NodeType::TableCell,
                            vec![root, table, row],
                        ))
                        .unwrap()
                        .id;
                    count += 1;
                    g.add_mut(seq_block(
                        count,
                        NodeType::Paragraph,
                        vec![root, table, row, cell],
                    ))
                    .unwrap();
                    count += 1;
                    g.add_mut(seq_char(count, 'x')).unwrap();
                    count += 1;
                    if z & 1 == 1 {
                        g.add_mut(seq_block(
                            count,
                            NodeType::Table,
                            vec![root, table, row, cell],
                        ))
                        .unwrap();
                        count += 1;
                    }
                    blocks.push(table);
                }
                14 => {
                    let bq = g
                        .add_mut(seq_block(count, NodeType::Blockquote, vec![root]))
                        .unwrap()
                        .id;
                    count += 1;
                    g.add_mut(seq_block(count, NodeType::Paragraph, vec![root, bq]))
                        .unwrap();
                    count += 1;
                    g.add_mut(seq_char(count, 'q')).unwrap();
                    count += 1;
                    g.add_mut(EditOp::Seq(ListOp::Ins {
                        pos: count,
                        item: SeqItem::Atom(AtomLeaf::PageBreak),
                    }))
                    .unwrap();
                    count += 1;
                    blocks.push(bq);
                    containers.push(bq);
                }
                _ => {
                    let parent = if blocks.is_empty() {
                        root
                    } else {
                        blocks[x % blocks.len()]
                    };
                    let chain = if parent == root {
                        vec![root]
                    } else {
                        vec![root, parent]
                    };
                    let pos = x % (count + 1);
                    let d = g
                        .add_mut(seq_block(pos, adversarial_block_type(y), chain))
                        .unwrap()
                        .id;
                    blocks.push(d);
                    count += 1;
                }
            }
        }
        g
    }

    fn arb_adversarial_css() -> impl proptest::strategy::Strategy<Value = Vec<CausalStep>> {
        use proptest::prelude::*;
        proptest::collection::vec((any::<u8>(), any::<u8>(), any::<u8>(), any::<u8>()), 0..40)
    }

    fn arb_clean_css() -> impl proptest::strategy::Strategy<Value = Vec<CausalStep>> {
        use proptest::prelude::*;
        proptest::collection::vec((any::<u8>(), any::<u8>(), any::<u8>(), any::<u8>()), 0..40)
    }

    // A single trailing edit for the incremental warm≡cold oracle: `(is_delete, a, b)`
    // — a range delete or a char insert, positioned modulo the base's visible length.
    fn arb_tail() -> impl proptest::strategy::Strategy<Value = (bool, u8, u8)> {
        use proptest::prelude::*;
        (any::<bool>(), any::<u8>(), any::<u8>())
    }

    // Real (op) dots present in the current visible sequence — the "every visible
    // element" a total projection must reach.
    fn visible_real_dots(state: &ProjectedState) -> HashSet<Dot> {
        state
            .seq
            .snapshot(&state.logs.seq)
            .into_iter()
            .map(|(d, _)| d)
            .collect()
    }

    // Real dots in document order — a child-ordered preorder walk of the projected
    // tree (synthetic scaffolds descended through, not emitted).
    fn preorder_real_dots(doc: &ProjectedDoc) -> Vec<Dot> {
        fn walk(tree: &BlockTree, id: Dot, out: &mut Vec<Dot>) {
            let Some(node) = tree.get(id) else {
                return;
            };
            for c in node.children.iter() {
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
                        walk(tree, *id, out);
                    }
                }
            }
        }
        let mut out = Vec::new();
        walk(&doc.tree, doc.tree.root, &mut out);
        out
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 256, ..proptest::prelude::ProptestConfig::default() })]

        // Totality: the projection reaches every visible sequence element — the set
        // difference `visible − reachable` is empty, and the instance's own
        // set-difference tally agrees.
        #[test]
        fn projection_reaches_every_visible_op(css in arb_adversarial_css()) {
            let state = ProjectedState::from_graph(build_causal_script(&css, true))
                .expect("adversarial history projects");
            let visible = visible_real_dots(&state);
            let reachable = reachable_real_dots(state.projected());
            let missing: Vec<Dot> = visible.difference(&reachable).copied().collect();
            proptest::prop_assert!(missing.is_empty(), "unreachable visible dots: {missing:?}");
            proptest::prop_assert_eq!(state.projected().repair_stats.totality_violations, 0);
        }

        // Exact-once: no real dot is reached more than once in a document-order walk —
        // a scaffold graft must never duplicate a real subtree. Set-based totality
        // above collapses a duplicate, so this multiset invariant is a separate gate.
        #[test]
        fn projection_reaches_each_visible_op_at_most_once(css in arb_adversarial_css()) {
            let state = ProjectedState::from_graph(build_causal_script(&css, true))
                .expect("adversarial history projects");
            let preorder = preorder_real_dots(state.projected());
            let uniq: HashSet<Dot> = preorder.iter().copied().collect();
            proptest::prop_assert_eq!(
                preorder.len(),
                uniq.len(),
                "a real dot is reached more than once: {:?}",
                preorder
            );
        }

        // Schema validity: a non-degraded projection is a schema-valid nested tree.
        #[test]
        fn projection_result_is_schema_valid(css in arb_adversarial_css()) {
            let state = ProjectedState::from_graph(build_causal_script(&css, true))
                .expect("adversarial history projects");
            proptest::prop_assert!(!state.projected().repair_stats.projection_degraded);
            proptest::prop_assert!(
                editor_model::validate_block_tree(&state.projected().tree).is_ok()
            );
        }

        // Determinism: two independent cold projections of the same history agree.
        #[test]
        fn projection_is_deterministic(css in arb_adversarial_css()) {
            let a = ProjectedState::from_graph(build_causal_script(&css, true))
                .expect("projects");
            let b = ProjectedState::from_graph(build_causal_script(&css, true))
                .expect("projects");
            proptest::prop_assert_eq!(a.projected(), b.projected());
        }

        // Order invariant: repair moves content but never reorders it. A document-order
        // (preorder) walk of the projected tree's real dots visits them in visible
        // sequence order — the reversed-Fold case SPLIT-wraps (Content stays before
        // Title), it does not physically reorder. Stated as a subsequence so a
        // duplicated scaffold graft (caught by
        // `cold_projection_reaches_each_visible_op_exactly_once`) cannot mask a genuine
        // reorder: the sequence must embed in the preorder in order.
        #[test]
        fn projection_preserves_real_dot_order(css in arb_adversarial_css()) {
            let state = ProjectedState::from_graph(build_causal_script(&css, true))
                .expect("projects");
            let sequence: Vec<Dot> = state
                .seq
                .snapshot(&state.logs.seq)
                .into_iter()
                .map(|(d, _)| d)
                .collect();
            let preorder = preorder_real_dots(state.projected());
            let mut it = preorder.iter();
            for want in &sequence {
                let found = it.by_ref().any(|d| d == want);
                proptest::prop_assert!(
                    found,
                    "sequence dot {:?} not reached in document order; preorder={:?} sequence={:?}",
                    want,
                    preorder,
                    sequence
                );
            }
        }

        // Idempotence: re-normalizing the repaired RawTree is a fixed point. Kept in
        // RawTree form — a BlockTree→flatten round-trip debug-asserts on the synthetic
        // scaffold that repair-triggering input produces.
        #[test]
        fn normalize_is_idempotent(css in arb_adversarial_css()) {
            let state = ProjectedState::from_graph(build_causal_script(&css, true))
                .expect("projects");
            let elements = state.seq.snapshot(&state.logs.seq);
            let once = editor_model::normalize(project_blocks(&elements).expect("blocks"));
            let twice = editor_model::normalize(once.clone());
            proptest::prop_assert_eq!(once, twice);
        }

        // Incremental warm≡cold oracle (LIVE gate): apply one edit onto an arbitrary
        // adversarial cold base through the real `apply` path; the incrementally
        // maintained projection must equal a cold rebuild of the resulting graph. The
        // two window-reproject defects this oracle once surfaced — list-container
        // regrouping (`incremental_apply_over_list_container_diverges_from_cold`) and a
        // container-base totality loss (`incremental_apply_over_container_base_drops_visible_op`)
        // — are fixed and pinned as deterministic regressions below, so this now runs as
        // a gate rather than an ignored reproduction. It generalizes Task 5's live
        // warm≡cold suite past Paragraph/Blockquote to every container-internal type.
        #[test]
        fn incremental_apply_equals_cold(css in arb_adversarial_css(), tail in arb_tail()) {
            let mut warm = ProjectedState::from_graph(build_causal_script(&css, true))
                .expect("adversarial base projects");
            let (is_del, a, b) = tail;
            let vis = warm.seq.visible_len();
            let op = if is_del && vis > 1 {
                let pos = 1 + (a as usize) % (vis - 1);
                let len = 1 + (b as usize) % (vis - pos);
                EditOp::Seq(ListOp::Del { pos, len })
            } else {
                seq_char((a as usize) % (vis + 1), 'z')
            };
            // Some adversarial edits are rejected by the create-time guard; those never
            // reach projection, so skip rather than assert on an un-applied op.
            if warm.apply(op).is_err() {
                return Ok(());
            }
            let cold = ProjectedState::from_graph(warm.graph().clone())
                .expect("cold rebuild projects");
            proptest::prop_assert_eq!(warm.projected(), cold.projected());
        }

        // Repairs fire only on damage: a clean history reports no repair, no drop, no
        // totality deficit, no degradation — read off the instance's ProjectedDoc, not
        // a global counter.
        #[test]
        fn repairs_never_fire_on_clean_history(css in arb_clean_css()) {
            let state = ProjectedState::from_graph(build_causal_script(&css, false))
                .expect("clean history projects");
            let stats = state.projected().repair_stats;
            proptest::prop_assert_eq!(stats.repairs, 0, "clean history: {:?}", stats);
            proptest::prop_assert_eq!(stats.drops, 0);
            proptest::prop_assert_eq!(stats.totality_violations, 0);
            proptest::prop_assert!(!stats.projection_degraded);
        }

        // Degraded path: even with the repair budget lowered so the cap latches, the
        // unconditional totality check runs and finds every visible element.
        #[test]
        fn low_repair_budget_never_violates_totality(css in arb_adversarial_css()) {
            let g = build_causal_script(&css, true);
            let _guard = editor_model::override_repair_budget(1);
            let state = ProjectedState::from_graph(g).expect("projects even when degraded");
            let visible = visible_real_dots(&state);
            let reachable = reachable_real_dots(state.projected());
            let missing: Vec<Dot> = visible.difference(&reachable).copied().collect();
            proptest::prop_assert!(missing.is_empty(), "degraded lost visible dots: {missing:?}");
            proptest::prop_assert_eq!(state.projected().repair_stats.totality_violations, 0);
        }
    }

    // Instrumentation (anti-vacuity): the adversarial generator actually emits the
    // container-internal types (FoldTitle / FoldContent / ListItem / TableRow /
    // TableCell) plus the other injected shapes — sweep the move space and collect
    // every block/atom type that reaches the visible sequence.
    #[test]
    fn adversarial_generator_covers_container_internal_types() {
        let mut seen: HashSet<NodeType> = HashSet::new();
        for op in 0u8..16 {
            for y in 0u8..2 {
                for z in 0u8..2 {
                    let steps: Vec<CausalStep> =
                        (0..4).map(|i| (op, (i * 7) as u8, y, z)).collect();
                    let g = build_causal_script(&steps, true);
                    let state = ProjectedState::from_graph(g).expect("projects");
                    for (_, item) in state.seq.snapshot(&state.logs.seq) {
                        if let Some(t) = item.as_child_type() {
                            seen.insert(t);
                        }
                    }
                }
            }
        }
        for required in [
            NodeType::Fold,
            NodeType::FoldTitle,
            NodeType::FoldContent,
            NodeType::BulletList,
            NodeType::OrderedList,
            NodeType::ListItem,
            NodeType::Table,
            NodeType::TableRow,
            NodeType::TableCell,
            NodeType::Blockquote,
            NodeType::Unknown,
            NodeType::PageBreak,
        ] {
            assert!(
                seen.contains(&required),
                "generator never emitted {required:?}"
            );
        }
    }

    // Degraded path (deterministic): a fixture that needs several WRAP/SPLIT repairs,
    // projected once at full budget to confirm it does, then again with the budget
    // lowered to 1 so the cap latches. Degraded downgrades only schema validity — the
    // totality check still runs unconditionally and charges nothing.
    #[test]
    fn degraded_cap_projection_still_reaches_every_visible_op() {
        let steps: [CausalStep; 5] = [
            (11, 1, 1, 1),
            (12, 1, 1, 1),
            (8, 0, 1, 0),
            (8, 0, 2, 0),
            (15, 0, 3, 0),
        ];
        let full = ProjectedState::from_graph(build_causal_script(&steps, true))
            .expect("fixture projects");
        assert!(
            full.projected().repair_stats.repairs >= 2,
            "fixture must need multiple repairs to exercise the cap: {:?}",
            full.projected().repair_stats
        );
        assert!(!full.projected().repair_stats.projection_degraded);

        let g = build_causal_script(&steps, true);
        let state = {
            let _guard = editor_model::override_repair_budget(1);
            ProjectedState::from_graph(g).expect("projects even when degraded")
        };
        let stats = state.projected().repair_stats;
        assert!(
            stats.projection_degraded,
            "budget 1 latches the cap: {stats:?}"
        );
        assert_eq!(
            stats.totality_violations, 0,
            "degraded loses nothing: {stats:?}"
        );
        let visible = visible_real_dots(&state);
        let reachable = reachable_real_dots(state.projected());
        let missing: Vec<Dot> = visible.difference(&reachable).copied().collect();
        assert!(
            missing.is_empty(),
            "degraded orphaned visible dots: {missing:?}"
        );
    }

    // Regression for an incremental-vs-cold divergence on LIST containers. A document
    // holding a BulletList takes a range delete that removes the BulletList marker,
    // leaving a bare ListItem (with a surplus paragraph) as a Root-misfit. A cold
    // rebuild wraps the ListItem in a BulletList first, so the surplus SPLIT-HOISTs into
    // a second ListItem inside that list; the incremental window path formerly
    // normalized the bare ListItem's content under `[Root]` first — hoisting the surplus
    // one level too high, to Root — then wrapped only the ListItem, stranding the surplus
    // as a Root sibling (warm 1 ListItem, cold 2; repairs 8 vs 9). Fixed by normalizing
    // the whole window forest with the full-document Root algebra (WRAP before recurse),
    // minus the document-global Root completion. Reproduces identically whether the base
    // is built cold (`from_graph`) or entirely incrementally, so it was a genuine
    // window-reproject defect, not a base-construction artifact.
    #[test]
    fn incremental_apply_over_list_container_diverges_from_cold() {
        let css: [CausalStep; 10] = [
            (0, 0, 0, 0),
            (0, 0, 0, 0),
            (15, 0, 0, 0),
            (15, 0, 0, 0),
            (0, 0, 0, 0),
            (0, 0, 0, 0),
            (140, 0, 0, 30),
            (142, 0, 0, 0),
            (143, 0, 0, 0),
            (122, 73, 0, 0),
        ];
        let tail = EditOp::Seq(ListOp::Del { pos: 3, len: 5 });
        let g = build_causal_script(&css, true);

        // Cold-built base, then one incremental delete.
        let mut warm_fg = ProjectedState::from_graph(g.clone()).unwrap();
        warm_fg.apply(tail.clone()).unwrap();

        // Entirely-incremental: replay every base op through `apply`, then the same tail.
        let mut ops: Vec<Op<EditOp>> = g.iter_all().cloned().collect();
        ops.sort_by_key(|o| o.id.clock);
        let mut warm_inc = ProjectedState::from_graph(OpGraph::<EditOp>::with_actor(1)).unwrap();
        for o in &ops {
            warm_inc.apply(o.payload.clone()).unwrap();
        }
        warm_inc.apply(tail).unwrap();

        // Both incremental constructions agree — the divergence is not a from_graph artifact.
        assert_eq!(
            warm_fg.projected(),
            warm_inc.projected(),
            "cold-base and entirely-incremental warm states agree"
        );

        let cold = ProjectedState::from_graph(warm_fg.graph().clone()).unwrap();
        assert_eq!(warm_fg.projected(), cold.projected());
    }

    // Regression for a TOTALITY LOSS on the incremental apply path. A container-heavy
    // adversarial base (Fold + BulletList + nested/injected blocks) takes a single
    // incremental char insert; two visible chars formerly became unreachable from Root in
    // the warm tree while a cold rebuild kept them — the window reproject stranded ops a
    // cold rebuild keeps. An independent whole-document `visible − reachable` catches any
    // stranded op (the window totality tripwire also panics on it in a debug build).
    #[test]
    fn incremental_apply_over_container_base_drops_visible_op() {
        let css: [CausalStep; 12] = [
            (0, 0, 0, 0),
            (11, 0, 104, 1),
            (140, 0, 0, 50),
            (0, 3, 0, 0),
            (30, 0, 0, 0),
            (0, 22, 0, 0),
            (10, 103, 0, 0),
            (218, 2, 8, 0),
            (0, 0, 0, 0),
            (0, 0, 0, 0),
            (110, 0, 0, 0),
            (0, 0, 0, 0),
        ];
        let mut warm = ProjectedState::from_graph(build_causal_script(&css, true)).unwrap();
        warm.apply(seq_char(11, 'a')).unwrap();
        let visible = visible_real_dots(&warm);
        let reachable = reachable_real_dots(warm.projected());
        let missing: Vec<Dot> = visible.difference(&reachable).copied().collect();
        assert!(
            missing.is_empty(),
            "incremental apply stranded visible dots: {missing:?}"
        );
    }

    // Regression for a COLD-projection content DUPLICATION. For some adversarial list
    // histories the cold projection formerly listed the same synthetic scaffold block (a
    // BulletList's ListItem) twice in its parent's children; that scaffold holds real
    // chars, so a document-order (preorder) walk reached those real dots twice. Set-based
    // totality (`visible ⊆ reachable`) misses it — a duplicate collapses in a set — as do
    // schema validation and cold determinism (both cold rebuilds duplicate identically),
    // so this multiset exact-once invariant is a separate gate. Fixed by keying WRAP and
    // SPLIT-tail scaffolds distinctly (the `split_tail` kind bit) so the same real
    // content reaching both no longer collides to one id and grafts twice.
    #[test]
    fn cold_projection_reaches_each_visible_op_exactly_once() {
        let css: [CausalStep; 8] = [
            (184, 181, 173, 175),
            (52, 43, 19, 241),
            (30, 183, 200, 65),
            (162, 31, 53, 105),
            (95, 166, 50, 154),
            (249, 36, 117, 209),
            (222, 121, 193, 161),
            (17, 121, 226, 12),
        ];
        let state = ProjectedState::from_graph(build_causal_script(&css, true)).unwrap();
        let preorder = preorder_real_dots(state.projected());
        let uniq: HashSet<Dot> = preorder.iter().copied().collect();
        assert_eq!(
            preorder.len(),
            uniq.len(),
            "a real dot is reached more than once: {preorder:?}"
        );
    }
}
