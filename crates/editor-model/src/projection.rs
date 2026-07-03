use std::collections::{BTreeMap, HashMap, HashSet};

use editor_crdt::Dot;
use editor_crdt::OpLog;
use editor_crdt::sequence::{SeqCheckout, SeqResolve, checkout_with_resolver};

use crate::{
    BlockNode, BlockTree, Child, ChildList, Modifier, ModifierAttrLog, ModifierType, NodeType,
    OwnModifier, ProjectError, SchemaError, anchor_dot,
};
use crate::{
    Marker, Node, NodeAttrLog, NodeMarkerLog, NodeStyleLog, Run, SeqItem, SpanLog, StyleEntry,
    StyleLog, derive_full_effective, normalize, project_blocks, validate_block_tree,
};

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
    pub node_styles: NodeStyleLog,
    pub node_markers: NodeMarkerLog,
    pub styles: StyleLog,
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
    pub effective: imbl::HashMap<Dot, LeafEff>,
    pub block_effective: imbl::HashMap<Dot, BTreeMap<ModifierType, Modifier>>,
    pub own_modifiers: imbl::HashMap<Dot, LeafOwn>,
    pub run_index: crate::span::BlockRuns,
    pub block_modifiers: imbl::HashMap<Dot, BTreeMap<ModifierType, Modifier>>,
    pub node_attrs: imbl::HashMap<Dot, Node>,
    pub node_styles: imbl::HashMap<Dot, Option<String>>,
    pub node_markers: imbl::HashMap<Dot, Option<Marker>>,
    pub styles: imbl::HashMap<String, StyleEntry>,
}

impl ProjectedDoc {
    pub fn runs(&self) -> Vec<Run> {
        self.run_index.materialize(&self.tree)
    }

    pub fn splice_char(
        &mut self,
        block: Dot,
        offset: usize,
        leaf: Dot,
        item: SeqItem,
        eff: LeafEff,
        is_atom: bool,
    ) {
        insert_leaf_at(&mut self.tree, block, offset, leaf, &item);
        self.effective.insert(leaf, eff.clone());
        self.run_index
            .splice_insert(block, offset, leaf, eff, is_atom);
    }

    /// Tree-anchored leaf insert: place `leaf` after `after` (or at the block
    /// start when `None`) using tree identity, not a sequence offset. Returns
    /// false if `after` isn't a leaf of `block`. Ghost-safe.
    pub fn splice_leaf_after(
        &mut self,
        block: Dot,
        after: Option<Dot>,
        leaf: Dot,
        item: SeqItem,
        eff: LeafEff,
        is_atom: bool,
    ) -> bool {
        let Some(offset) = insert_leaf_after(&mut self.tree, block, after, leaf, &item) else {
            return false;
        };
        self.effective.insert(leaf, eff.clone());
        self.run_index
            .splice_insert(block, offset, leaf, eff, is_atom);
        true
    }

    pub fn splice_delete_leaf(&mut self, block: Dot, leaf: Dot) -> bool {
        let Some(offset) = remove_leaf_from_block(&mut self.tree, block, leaf) else {
            return false;
        };
        self.effective.remove(&leaf);
        self.own_modifiers.remove(&leaf);
        self.node_styles.remove(&leaf);
        self.node_attrs.remove(&leaf);
        self.node_markers.remove(&leaf);
        self.run_index.splice_delete(block, offset);
        true
    }

    pub fn set_leaf_effective(&mut self, leaf: Dot, eff: LeafEff) {
        self.effective.insert(leaf, eff);
    }

    /// Re-segment a single leaf in `block`'s run index after its effective changed
    /// (e.g. a span was added/removed over it), without re-segmenting the whole block.
    /// Removing then re-inserting the leaf at the same offset lets the run splice
    /// merge/split runs locally — `O(log K)` instead of `O(block size)` per span, the
    /// difference between a styled paste being `O(N log N)` and `O(N²)`.
    pub fn respan_leaf(
        &mut self,
        block: Dot,
        offset: usize,
        leaf: Dot,
        eff: LeafEff,
        is_atom: bool,
    ) {
        self.effective.insert(leaf, eff.clone());
        self.run_index.splice_delete(block, offset);
        self.run_index
            .splice_insert(block, offset, leaf, eff, is_atom);
    }

    pub fn set_leaf_own(&mut self, leaf: Dot, own: LeafOwn) {
        if own.is_empty() {
            self.own_modifiers.remove(&leaf);
        } else {
            self.own_modifiers.insert(leaf, own);
        }
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

    pub fn resegment_block(&mut self, block: Dot) {
        self.run_index.resegment_from_runs(block, &self.effective);
    }
}

impl PartialEq for ProjectedDoc {
    fn eq(&self, o: &Self) -> bool {
        self.tree == o.tree
            && self.effective == o.effective
            && self.block_effective == o.block_effective
            && self.own_modifiers == o.own_modifiers
            && self.block_modifiers == o.block_modifiers
            && self.node_attrs == o.node_attrs
            && self.node_styles == o.node_styles
            && self.node_markers == o.node_markers
            && self.styles == o.styles
            && self.runs() == o.runs()
    }
}

fn collect_real_ids(tree: &BlockTree) -> HashMap<Dot, NodeType> {
    fn walk(tree: &BlockTree, node: &BlockNode, out: &mut HashMap<Dot, NodeType>) {
        if let Some(d) = anchor_dot(node.id) {
            out.insert(d, node.node_type);
        }
        for c in &node.children {
            match c {
                Child::Leaf { id, item } => {
                    out.insert(*id, item.as_child_type());
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

fn filter_live<T: Clone>(map: imbl::HashMap<Dot, T>, live: &HashSet<Dot>) -> imbl::HashMap<Dot, T> {
    map.into_iter().filter(|(d, _)| live.contains(d)).collect()
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
    pub coverage: crate::span::LeafSpanCoverage,
}

impl ProjectionIndexes {
    pub fn rebuild_from<R: SeqResolve>(
        projected: &ProjectedDoc,
        spans: &SpanLog,
        elements: &[(Dot, SeqItem)],
        resolver: &R,
    ) -> Self {
        Self {
            paths: BlockPaths::from_tree(&projected.tree),
            spans: crate::span::SpanAnchorIndex::build(spans),
            coverage: crate::span::LeafSpanCoverage::build(elements, spans, resolver),
        }
    }
}

/// Resolve a single leaf's effective + own modifier sets from the spans in
/// `covering`. Reuses the projected node-style/attr/style maps (a freshly
/// inserted leaf carries none of its own) so only the leaf's span contribution
/// is recomputed — O(covering + depth), independent of document size.
pub fn derive_leaf_from_covering(
    paths: &BlockPaths,
    logs: &DocLogs,
    projected: &ProjectedDoc,
    block: Dot,
    leaf: Dot,
    leaf_type: NodeType,
    covering: &[Dot],
) -> (
    BTreeMap<ModifierType, Modifier>,
    BTreeMap<ModifierType, OwnModifier>,
) {
    let block_path = paths.block_path_of(block);
    let mut leaf_path: Vec<NodeType> = block_path.iter().map(|(t, _)| *t).collect();
    leaf_path.push(leaf_type);
    let explicit = crate::span::explicit_from_covering(covering, &logs.spans, &leaf_path);
    let mut ex_map: HashMap<Dot, BTreeMap<ModifierType, crate::span::ExplicitEffect>> =
        HashMap::new();
    ex_map.insert(leaf, explicit);
    let src = crate::span::EffectiveSources {
        block_modifiers: &logs.block_modifiers,
        explicit_spans: &ex_map,
        node_styles: &projected.node_styles,
        styles: &projected.styles,
        node_attrs: &projected.node_attrs,
    };
    let effective = crate::span::resolve_effective(&block_path, Some(leaf), leaf_type, true, &src);
    let own = crate::span::own_modifiers_for_leaf(leaf, &leaf_path, &src);
    (effective, own)
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
    let empty: HashMap<Dot, BTreeMap<ModifierType, crate::span::ExplicitEffect>> = HashMap::new();
    let src = crate::span::EffectiveSources {
        block_modifiers: &logs.block_modifiers,
        explicit_spans: &empty,
        node_styles: &projected.node_styles,
        styles: &projected.styles,
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

/// Insert `leaf` into `block` immediately after the leaf `after` (or at the
/// block's start when `after` is `None`), returning the new leaf's offset (count
/// of leaves before it). Tree-anchored — unaffected by sequence "ghosts" (live
/// seq elements dropped from the tree by normalization). O(block size).
fn insert_leaf_after(
    tree: &mut BlockTree,
    block: Dot,
    after: Option<Dot>,
    leaf: Dot,
    item: &SeqItem,
) -> Option<usize> {
    let mut result: Option<usize> = None;
    with_block_children(tree, block, |children| {
        let (at, offset) = match after {
            None => (0, 0),
            Some(a) => {
                let mut seen = 0usize;
                let mut found = None;
                for (i, c) in children.iter().enumerate() {
                    if let Child::Leaf { id, .. } = c {
                        seen += 1;
                        if *id == a {
                            found = Some((i + 1, seen));
                            break;
                        }
                    }
                }
                match found {
                    Some(f) => f,
                    None => return,
                }
            }
        };
        children.insert(
            at,
            Child::Leaf {
                id: leaf,
                item: item.clone(),
            },
        );
        result = Some(offset);
    });
    result
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
    let node_type_of = collect_real_ids(&tree);
    let live: HashSet<Dot> = node_type_of.keys().copied().collect();
    let block_modifiers = collect_block_modifiers(&tree, &logs.block_modifiers);

    let node_attrs = logs.node_attrs.project(|d| node_type_of.get(&d).copied());
    let node_styles = filter_live(logs.node_styles.project(), &live);
    let node_markers = filter_live(logs.node_markers.project(), &live);
    let styles = logs.styles.registered_entries();

    let explicit_spans: HashMap<Dot, BTreeMap<ModifierType, crate::span::ExplicitEffect>> =
        crate::span::derive_explicit_effect(elements, &tree, resolver, &logs.spans)
            .into_iter()
            .collect();

    let (effective, block_effective, own_modifiers) = {
        let src = crate::span::EffectiveSources {
            block_modifiers: &logs.block_modifiers,
            explicit_spans: &explicit_spans,
            node_styles: &node_styles,
            styles: &styles,
            node_attrs: &node_attrs,
        };
        let effective: imbl::HashMap<Dot, LeafEff> =
            derive_full_effective(&tree, &src).into_iter().collect();
        let block_effective = crate::span::derive_block_effective(&tree, &src);
        let own_modifiers = crate::span::derive_own_modifiers(&tree, &src);
        (effective, block_effective, own_modifiers)
    };

    let run_index = crate::span::BlockRuns::build(&tree, &effective);

    ProjectedDoc {
        tree,
        effective,
        block_effective,
        own_modifiers,
        run_index,
        block_modifiers,
        node_attrs,
        node_styles,
        node_markers,
        styles,
    }
}

/// Build a document from the post-delete sequence while reusing a precomputed span
/// coverage. Identical to [`project_core`]/[`project_from_tree`] except explicit span
/// effects come from `coverage` instead of re-resolving the whole span log — the
/// coverage-preserving bulk-delete reprojection path (`O(survivors)`, not `O(#spans)`).
pub fn project_core_with_coverage(
    elements: &[(Dot, SeqItem)],
    logs: &DocLogs,
    coverage: &crate::span::LeafSpanCoverage,
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

    let node_type_of = collect_real_ids(&tree);
    let live: HashSet<Dot> = node_type_of.keys().copied().collect();
    let block_modifiers = collect_block_modifiers(&tree, &logs.block_modifiers);

    let node_attrs = logs.node_attrs.project(|d| node_type_of.get(&d).copied());
    let node_styles = filter_live(logs.node_styles.project(), &live);
    let node_markers = filter_live(logs.node_markers.project(), &live);
    let styles = logs.styles.registered_entries();

    let explicit_spans: HashMap<Dot, BTreeMap<ModifierType, crate::span::ExplicitEffect>> =
        crate::span::derive_explicit_effect_from_coverage(&tree, coverage, &logs.spans)
            .into_iter()
            .collect();

    let (effective, block_effective, own_modifiers) = {
        let src = crate::span::EffectiveSources {
            block_modifiers: &logs.block_modifiers,
            explicit_spans: &explicit_spans,
            node_styles: &node_styles,
            styles: &styles,
            node_attrs: &node_attrs,
        };
        let effective: imbl::HashMap<Dot, LeafEff> =
            derive_full_effective(&tree, &src).into_iter().collect();
        let block_effective = crate::span::derive_block_effective(&tree, &src);
        let own_modifiers = crate::span::derive_own_modifiers(&tree, &src);
        (effective, block_effective, own_modifiers)
    };

    let run_index = crate::span::BlockRuns::build(&tree, &effective);

    Ok(ProjectedDoc {
        tree,
        effective,
        block_effective,
        own_modifiers,
        run_index,
        block_modifiers,
        node_attrs,
        node_styles,
        node_markers,
        styles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AtomLeaf, SeqItem, derive_runs, project_blocks};

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
        let effective: imbl::HashMap<Dot, LeafEff> = imbl::HashMap::new();
        let runs = derive_runs(&tree, &effective);
        let projected = ProjectedDoc {
            tree: tree.clone(),
            effective: effective.clone(),
            block_effective: imbl::HashMap::new(),
            own_modifiers: imbl::HashMap::new(),
            run_index: crate::span::BlockRuns::build(&tree, &effective),
            block_modifiers: imbl::HashMap::new(),
            node_attrs: imbl::HashMap::new(),
            node_styles: imbl::HashMap::new(),
            node_markers: imbl::HashMap::new(),
            styles: imbl::HashMap::new(),
        };
        let idx =
            ProjectionIndexes::rebuild_from(&projected, &SpanLog::new(), &[], &SeqCheckout::new());
        assert_eq!(projected.runs(), runs);
        assert_eq!(idx.paths, BlockPaths::from_tree(&tree));
    }

    #[test]
    fn collect_real_ids_covers_blocks_chars_atoms() {
        let tree = BlockTree::from_raw(&project_blocks(&elems_nested()).unwrap());
        let ids = collect_real_ids(&tree);
        assert_eq!(ids.get(&Dot::ROOT), Some(&NodeType::Root));
        assert_eq!(ids.get(&Dot::new(1, 1)), Some(&NodeType::Paragraph));
        assert_eq!(ids.get(&Dot::new(1, 2)), Some(&NodeType::Text));
        assert_eq!(ids.get(&Dot::new(1, 4)), Some(&NodeType::HardBreak));
        assert_eq!(ids.get(&Dot::new(1, 5)), Some(&NodeType::Blockquote));
    }

    #[test]
    fn filter_live_keeps_only_live_keys() {
        let mut m: imbl::HashMap<Dot, u32> = imbl::HashMap::new();
        m.insert(Dot::new(1, 1), 10);
        m.insert(Dot::new(9, 9), 20);
        let live: HashSet<Dot> = [Dot::new(1, 1)].into_iter().collect();
        let out = filter_live(m, &live);
        assert_eq!(out.len(), 1);
        assert_eq!(out.get(&Dot::new(1, 1)), Some(&10));
    }

    use crate::{
        Anchor, Bias, CalloutNodeAttr, CalloutVariant, ImageNodeAttr, Modifier, ModifierAttrOp,
        ModifierType, NodeAttr, NodeAttrOp, NodeLwwOp, SpanOp, StyleOp, StyleRegOp,
    };
    use editor_crdt::{InputEvent, ListOp, LwwRegOp, build_oplog};

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
            node_styles: NodeStyleLog::new(),
            node_markers: NodeMarkerLog::new(),
            styles: StyleLog::new(),
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

    #[test]
    fn empty_document_projects_ok() {
        let pd = project_document(&logs_of(&[])).unwrap();
        assert_eq!(pd.tree.root_node().iter().count(), 1);
        assert_eq!(pd.tree.root_node().unwrap().node_type, NodeType::Root);
        assert!(pd.effective.is_empty());
        assert!(pd.runs().is_empty());
        assert!(pd.node_attrs.is_empty());
    }

    #[test]
    fn projects_nested_blocks() {
        let pd = project_document(&logs_of(&elems_nested())).unwrap();
        assert_eq!(pd.tree.root_node().iter().count(), 1);
        assert_eq!(pd.tree.root_node().unwrap().node_type, NodeType::Root);
        assert_eq!(pd.effective.len(), 5);
    }

    #[test]
    fn bold_span_splits_runs() {
        let (elems, _root, _para) = para_abc();
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
            pd.effective
                .get(&Dot::new(1, 4))
                .and_then(|m| m.get(&ModifierType::Bold)),
            Some(&Modifier::Bold)
        );
        assert!(
            pd.effective
                .get(&Dot::new(1, 2))
                .is_none_or(|m| !m.contains_key(&ModifierType::Bold))
        );
        assert!(pd.runs().len() >= 2);
    }

    #[test]
    fn runs_split_under_style_enriched_effective() {
        let (elems, _root, _para) = para_abc();
        let mut l = logs_of(&elems);
        let styled = Dot::new(1, 2);
        l.node_styles = NodeStyleLog::new()
            .apply(
                Dot::new(2, 0),
                NodeLwwOp {
                    target: styled,
                    op: LwwRegOp::Set {
                        value: Some("s1".to_string()),
                    },
                },
            )
            .unwrap();
        l.styles = StyleLog::new()
            .apply(
                Dot::new(2, 1),
                StyleRegOp {
                    style_id: "s1".to_string(),
                    op: StyleOp::Presence(editor_crdt::OrMapOp::Set {
                        key: "s1".to_string(),
                        value: (),
                    }),
                },
            )
            .unwrap()
            .apply(
                Dot::new(2, 2),
                StyleRegOp {
                    style_id: "s1".to_string(),
                    op: StyleOp::Modifiers(editor_crdt::OrSetOp::Add {
                        elem: Modifier::Bold,
                    }),
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert_eq!(
            pd.effective
                .get(&styled)
                .and_then(|m| m.get(&ModifierType::Bold)),
            Some(&Modifier::Bold)
        );
        assert_eq!(pd.runs().len(), 2);
        assert!(pd.runs().iter().any(|r| {
            r.leaves == vec![styled]
                && r.modifiers.get(&ModifierType::Bold) == Some(&Modifier::Bold)
        }));
        assert!(pd.runs().iter().any(|r| {
            r.leaves == vec![Dot::new(1, 3), Dot::new(1, 4)]
                && !r.modifiers.contains_key(&ModifierType::Bold)
        }));
    }

    #[test]
    fn overlays_attr_style_marker() {
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
        l.node_styles = NodeStyleLog::new()
            .apply(
                Dot::new(2, 1),
                NodeLwwOp {
                    target: callout,
                    op: LwwRegOp::Set {
                        value: Some("s1".to_string()),
                    },
                },
            )
            .unwrap();
        l.node_markers = NodeMarkerLog::new()
            .apply(
                Dot::new(2, 2),
                NodeLwwOp {
                    target: callout,
                    op: LwwRegOp::Set {
                        value: Some(Marker {
                            modifiers: vec![],
                            style: Some("s1".to_string()),
                        }),
                    },
                },
            )
            .unwrap();
        l.styles = StyleLog::new()
            .apply(
                Dot::new(2, 3),
                StyleRegOp {
                    style_id: "s1".to_string(),
                    op: StyleOp::Presence(editor_crdt::OrMapOp::Set {
                        key: "s1".to_string(),
                        value: (),
                    }),
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert!(pd.node_attrs.contains_key(&callout));
        assert_eq!(pd.node_styles.get(&callout), Some(&Some("s1".to_string())));
        assert!(pd.node_markers.get(&callout).is_some());
        assert!(pd.styles.contains_key("s1"));
    }

    #[test]
    fn font_size_inheritable_double_source() {
        let (elems, _root, para) = para_abc();
        let mut l = logs_of(&elems);
        l.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::new(2, 0),
                ModifierAttrOp::SetModifier {
                    target: para,
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert_eq!(
            pd.block_modifiers
                .get(&para)
                .and_then(|m| m.get(&ModifierType::FontSize)),
            Some(&Modifier::FontSize { value: 1600 })
        );
        assert_eq!(
            pd.effective
                .get(&Dot::new(1, 2))
                .and_then(|m| m.get(&ModifierType::FontSize)),
            Some(&Modifier::FontSize { value: 1600 })
        );
    }

    #[test]
    fn alignment_block_resolves_onto_descendant_text() {
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
            pd.effective
                .get(&Dot::new(1, 2))
                .and_then(|m| m.get(&ModifierType::Alignment)),
            Some(&Modifier::Alignment {
                value: Alignment::Center
            })
        );
    }

    #[test]
    fn image_atom_attr_projected() {
        let image = Dot::new(1, 1);
        let img_node = match NodeType::Image.into_node() {
            Node::Image(n) => n,
            _ => unreachable!(),
        };
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
        assert!(pd.node_attrs.contains_key(&image));
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
        let ids = collect_real_ids(&pd.tree);
        assert!(!ids.contains_key(&Dot::new(1, 1)));
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
        let ids = collect_real_ids(&pd.tree);
        assert!(!ids.contains_key(&c));
        assert!(!ids.contains_key(&p2));
        assert!(!ids.contains_key(&b));
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
        let ids = collect_real_ids(&pd.tree);
        assert!(!ids.contains_key(&Dot::new(2, 0)));
        assert!(validate_block_tree(&pd.tree).is_ok());
        assert!(pd.runs().is_empty());
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
        assert!(
            pd.effective
                .get(&Dot::new(1, 2))
                .is_none_or(|m| !m.contains_key(&ModifierType::Bold))
        );
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
        l.node_styles = NodeStyleLog::new()
            .apply(
                Dot::new(2, 0),
                NodeLwwOp {
                    target: Dot::new(9, 9),
                    op: LwwRegOp::Set {
                        value: Some("ghost".to_string()),
                    },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert!(!pd.node_styles.contains_key(&Dot::new(9, 9)));
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
        l.node_styles = NodeStyleLog::new()
            .apply(
                Dot::new(2, 0),
                NodeLwwOp {
                    target: loser,
                    op: LwwRegOp::Set {
                        value: Some("x".to_string()),
                    },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        assert!(!pd.node_styles.contains_key(&loser));
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
        let node_type_of = collect_real_ids(&tree);
        let live: HashSet<Dot> = node_type_of.keys().copied().collect();
        let node_attrs = l.node_attrs.project(|d| node_type_of.get(&d).copied());
        let node_styles = filter_live(l.node_styles.project(), &live);
        let styles = l.styles.registered_entries();
        let explicit: HashMap<Dot, _> =
            crate::span::derive_explicit_effect(&els, &tree, &resolver, &l.spans)
                .into_iter()
                .collect();
        let src = crate::span::EffectiveSources {
            block_modifiers: &l.block_modifiers,
            explicit_spans: &explicit,
            node_styles: &node_styles,
            styles: &styles,
            node_attrs: &node_attrs,
        };
        let direct: imbl::HashMap<Dot, LeafEff> =
            derive_full_effective(&tree, &src).into_iter().collect();
        let pd = project_document(&l).unwrap();
        assert_eq!(pd.effective, direct);
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
            pd.own_modifiers
                .get(&Dot::new(1, 2))
                .and_then(|m| m.get(&ModifierType::Bold)),
            Some(&crate::OwnModifier {
                value: Modifier::Bold,
                from_style: false
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

            let live = collect_real_ids(&pd.tree);
            let live_leaves: HashSet<Dot> = live.iter()
                .filter(|(_, t)| t.spec().is_leaf())
                .map(|(d, _)| *d)
                .collect();

            let eff_keys: HashSet<Dot> = pd.effective.keys().copied().collect();
            prop_assert_eq!(eff_keys, live_leaves.clone());

            let mut covered: Vec<Dot> = pd.runs().iter().flat_map(|r| r.leaves.clone()).collect();
            covered.sort();
            let mut expect: Vec<Dot> = live_leaves.into_iter().collect();
            expect.sort();
            prop_assert_eq!(covered, expect);

            let all_ids: HashSet<Dot> = live.keys().copied().collect();
            for d in pd.node_attrs.keys() { prop_assert!(all_ids.contains(d)); }
            for d in pd.node_styles.keys() { prop_assert!(all_ids.contains(d)); }
            for d in pd.node_markers.keys() { prop_assert!(all_ids.contains(d)); }
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
                projected.splice_char(para, i, leaf, SeqItem::Char(ch), LeafEff::default(), false);
            }

            let full_pd = project_document(&logs_of(&full)).unwrap();
            prop_assert_eq!(&projected.tree, &full_pd.tree);
            prop_assert_eq!(projected.runs(), full_pd.runs());
            prop_assert_eq!(&projected.effective, &full_pd.effective);
        }
    }
}
