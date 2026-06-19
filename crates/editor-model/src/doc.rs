use editor_crdt::{Dot, OpGraph, OrMap, Text, TextPlacement};
use hashbrown::{HashMap, HashSet};
use std::collections::VecDeque;
use std::sync::OnceLock;

use crate::apply_doc_op;
use crate::doc_op::DocOp;
use crate::doc_text_store::DocTextStore;
use crate::entry::NodeEntry;
use crate::error::ModelError;
use crate::id::NodeId;
use crate::node_ref::NodeRef;
use crate::nodes::{Node, NodeType};
use crate::stable_position_remap::StablePositionRemapStore;
use crate::style::StyleEntry;
use crate::text_view::{TextIdentityView, TextView};

#[derive(Clone, Copy, Debug)]
pub(crate) struct NodePos {
    pub(crate) index: usize,
    pub(crate) prev: Option<NodeId>,
    pub(crate) next: Option<NodeId>,
}

#[derive(Default)]
struct ChildIndex {
    pos: HashMap<NodeId, NodePos>,
    ordered: HashMap<NodeId, Vec<NodeId>>,
    /// Each child's dot in its parent's `children` RGA, so resolving a node's
    /// stable anchor doesn't scan the parent's children (`Rga::dot_for` is
    /// `O(siblings)` — `O(N)` for a document whose root holds every block).
    dots: HashMap<NodeId, Dot>,
}

#[derive(Default)]
struct ChildIndexCache(OnceLock<std::sync::Arc<ChildIndex>>);

impl Clone for ChildIndexCache {
    fn clone(&self) -> Self {
        // Share the built index via Arc (O(1)): it depends only on doc content,
        // identical at clone time. Structural ops (Parent/Children) invalidate
        // the mutated doc's own pointer (`invalidate_child_index`); the Arc stays
        // valid for clones that keep it. A pure text edit clones the doc per op
        // (`apply_internal`) but leaves structure untouched, so this avoids an
        // O(N) index rebuild every keystroke.
        Self(self.0.clone())
    }
}

impl PartialEq for ChildIndexCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl std::fmt::Debug for ChildIndexCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ChildIndexCache(..)")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlatKind {
    Open,
    Close,
    Text,
    Break,
    Atom,
}

/// One leaf of the flattened document: its node and flat kind. The flat *size*
/// is carried by the sum tree, not stored here, so a text-size change is an
/// `O(log N)` tree update rather than an `O(N)` absolute-offset shift.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlatLeaf {
    pub node_id: NodeId,
    pub kind: FlatKind,
}

/// Flattened view of the document as an order-statistics sum tree of leaves.
/// Offsets are derived from cached subtree sizes (`O(log N)`); a text-size edit
/// updates one leaf in `O(log N)` while sharing the rest (`with_text_size`), so
/// the layout is maintained incrementally rather than rebuilt per keystroke.
#[derive(Clone, Default)]
pub struct FlatLayout {
    tree: editor_common::SumTree<FlatLeaf>,
    text_positions: imbl::HashMap<NodeId, usize>,
}

impl FlatLayout {
    /// Total flat size — `O(1)`.
    pub fn size(&self) -> usize {
        self.tree.total_size() as usize
    }

    /// Flat start offset of a text node — `O(log N)`.
    pub fn text_start(&self, node_id: NodeId) -> Option<usize> {
        self.text_positions
            .get(&node_id)
            .map(|&i| self.tree.offset_before(i) as usize)
    }

    /// Visits every flat leaf in order as `(start_offset, node_id, kind, size)`.
    pub fn for_each_segment(&self, mut f: impl FnMut(usize, NodeId, FlatKind, usize)) {
        let mut offset = 0usize;
        self.tree.for_each(|leaf, size| {
            f(offset, leaf.node_id, leaf.kind, size as usize);
            offset += size as usize;
        });
    }

    /// Visits flat leaves overlapping `[start, end)` as `(start_offset, node_id,
    /// kind, size)` — `O(span + log N)`.
    pub fn for_each_segment_in_range(
        &self,
        start: usize,
        end: usize,
        mut f: impl FnMut(usize, NodeId, FlatKind, usize),
    ) {
        self.tree
            .for_each_in_range(start as u64, end as u64, |item_start, leaf, size| {
                f(item_start as usize, leaf.node_id, leaf.kind, size as usize)
            });
    }

    /// A layout with the given text node's flat size updated, sharing all other
    /// structure (`O(log N)`). `None` if the node is not a text leaf here.
    pub(crate) fn with_text_size(&self, node_id: NodeId, new_size: usize) -> Option<FlatLayout> {
        let &index = self.text_positions.get(&node_id)?;
        let mut tree = self.tree.clone();
        if !tree.set_size(index, new_size as u64) {
            return None;
        }
        Some(FlatLayout {
            tree,
            text_positions: self.text_positions.clone(),
        })
    }
}

enum NodeFlatClass {
    Text,
    Break,
    Atom,
    Container,
}

fn classify_flat(node: &Node) -> NodeFlatClass {
    match node {
        Node::Text(_) => NodeFlatClass::Text,
        other => {
            let spec = other.spec();
            if spec.inline {
                NodeFlatClass::Break
            } else if spec.is_leaf() {
                NodeFlatClass::Atom
            } else {
                NodeFlatClass::Container
            }
        }
    }
}

#[derive(Default)]
struct FlatLayoutCache(OnceLock<std::sync::Arc<FlatLayout>>);

impl Clone for FlatLayoutCache {
    fn clone(&self) -> Self {
        // Share the built layout with the clone via Arc (O(1)). The flattening
        // depends only on doc content, which is identical at clone time, so the
        // clone may reuse it. Any mutation invalidates the mutated doc's own
        // pointer (see `invalidate_flat_layout`); the immutable Arc stays valid
        // for every other doc that still holds it. This avoids the O(N) rebuild
        // when a transaction clones the doc for read-only access.
        Self(self.0.clone())
    }
}

impl PartialEq for FlatLayoutCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl std::fmt::Debug for FlatLayoutCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FlatLayoutCache(..)")
    }
}

/// Text nodes whose materialized projection (`Text::visible`) is stale after a
/// batch of text ops. Rebuilding a node's projection is `O(node)`, so doing it
/// per op turns a K-character delete into `O(K·node)`; deferring lets the batch
/// boundary refresh each node once. Excluded from `Doc` equality (a transient
/// cache): always flushed before the doc is observed/compared.
#[derive(Clone, Debug, Default)]
struct PendingTextRefresh(imbl::HashSet<NodeId>);

impl PartialEq for PendingTextRefresh {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Doc {
    pub(crate) nodes: OrMap<NodeId, NodeType>,
    pub(crate) entries: imbl::HashMap<NodeId, NodeEntry>,
    pub(crate) text: DocTextStore,
    pub(crate) stable_position_remap: StablePositionRemapStore,
    pub(crate) styles: OrMap<String, ()>,
    pub(crate) style_entries: imbl::HashMap<String, StyleEntry>,
    child_index: ChildIndexCache,
    flat_layout: FlatLayoutCache,
    pending_text_refresh: PendingTextRefresh,
}

impl Doc {
    pub fn empty() -> Self {
        Self::default()
    }

    pub(crate) fn child_pos(&self, id: NodeId) -> Option<NodePos> {
        self.child_index().pos.get(&id).copied()
    }

    pub(crate) fn nth_child(&self, parent: NodeId, index: usize) -> Option<NodeId> {
        self.child_index().ordered.get(&parent)?.get(index).copied()
    }

    /// This node's dot in its parent's `children` RGA (`O(1)`), or `None` for the
    /// root / orphans.
    pub fn child_dot(&self, id: NodeId) -> Option<Dot> {
        self.child_index().dots.get(&id).copied()
    }

    fn child_index(&self) -> &ChildIndex {
        self.child_index
            .0
            .get_or_init(|| std::sync::Arc::new(self.build_child_index()))
    }

    fn build_child_index(&self) -> ChildIndex {
        let mut pos = HashMap::new();
        let mut ordered = HashMap::new();
        let mut dots = HashMap::new();
        for (parent_id, entry) in self.entries.iter() {
            let children_with_dots: Vec<(Dot, NodeId)> = entry
                .children
                .iter_with_dot()
                .map(|(d, id)| (d, *id))
                .collect();
            let children: Vec<NodeId> = children_with_dots.iter().map(|(_, id)| *id).collect();
            for (i, &(dot, child)) in children_with_dots.iter().enumerate() {
                let parent_matches = self
                    .entries
                    .get(&child)
                    .map(|child_entry| child_entry.parent.get())
                    .is_some_and(|parent| parent.as_ref() == Some(parent_id));
                if !parent_matches {
                    continue;
                }
                pos.insert(
                    child,
                    NodePos {
                        index: i,
                        prev: i.checked_sub(1).map(|p| children[p]),
                        next: children.get(i + 1).copied(),
                    },
                );
                dots.insert(child, dot);
            }
            ordered.insert(*parent_id, children);
        }
        ChildIndex { pos, ordered, dots }
    }

    pub(crate) fn invalidate_child_index(&mut self) {
        self.child_index.0.take();
    }

    pub fn flat_layout(&self) -> &FlatLayout {
        self.flat_layout
            .0
            .get_or_init(|| std::sync::Arc::new(self.build_flat_layout()))
    }

    fn build_flat_layout(&self) -> FlatLayout {
        let mut leaves: Vec<(FlatLeaf, u64)> = Vec::new();
        let mut text_positions = imbl::HashMap::new();
        if let Some(root) = self.root() {
            self.visit_flat_layout(root.id(), &mut leaves, &mut text_positions);
        }
        FlatLayout {
            tree: editor_common::SumTree::from_items(leaves),
            text_positions,
        }
    }

    fn visit_flat_layout(
        &self,
        node_id: NodeId,
        leaves: &mut Vec<(FlatLeaf, u64)>,
        text_positions: &mut imbl::HashMap<NodeId, usize>,
    ) {
        let Some(entry) = self.get_entry(node_id) else {
            return;
        };
        for child_id in entry.children.iter().copied() {
            let Some(child) = self.get_entry(child_id) else {
                continue;
            };
            match classify_flat(&child.node) {
                NodeFlatClass::Text => {
                    let text_len = match &child.node {
                        Node::Text(t) => t.text.len(),
                        _ => unreachable!("classified as Text"),
                    };
                    text_positions.insert(child_id, leaves.len());
                    leaves.push((
                        FlatLeaf {
                            node_id: child_id,
                            kind: FlatKind::Text,
                        },
                        text_len as u64,
                    ));
                }
                NodeFlatClass::Break => leaves.push((
                    FlatLeaf {
                        node_id: child_id,
                        kind: FlatKind::Break,
                    },
                    1,
                )),
                NodeFlatClass::Atom => leaves.push((
                    FlatLeaf {
                        node_id: child_id,
                        kind: FlatKind::Atom,
                    },
                    1,
                )),
                NodeFlatClass::Container => {
                    leaves.push((
                        FlatLeaf {
                            node_id: child_id,
                            kind: FlatKind::Open,
                        },
                        1,
                    ));
                    self.visit_flat_layout(child_id, leaves, text_positions);
                    leaves.push((
                        FlatLeaf {
                            node_id: child_id,
                            kind: FlatKind::Close,
                        },
                        1,
                    ));
                }
            }
        }
    }

    pub(crate) fn invalidate_flat_layout(&mut self) {
        self.flat_layout.0.take();
    }

    pub fn from_op_graph(graph: &OpGraph<DocOp>) -> Result<Self, ModelError> {
        let dots: HashSet<Dot> = graph.iter_all().map(|op| op.id).collect();
        let mut doc = Doc::empty();
        for op in graph.topo_sort(&dots) {
            doc = apply_doc_op(doc, &op)?;
        }
        doc.flush_text_projections();
        Ok(doc)
    }

    pub fn from_op_graph_at(
        graph: &OpGraph<DocOp>,
        heads: &HashSet<Dot>,
    ) -> Result<Self, ModelError> {
        if let Some(missing) = heads.iter().find(|d| !graph.contains(d)) {
            return Err(ModelError::InvalidHead { dot: *missing });
        }
        let ancestry = graph.ancestry_of(heads);
        let mut doc = Doc::empty();
        for op in graph.topo_sort(&ancestry) {
            doc = apply_doc_op(doc, &op)?;
        }
        doc.flush_text_projections();
        Ok(doc)
    }

    pub fn node(&self, id: NodeId) -> Option<NodeRef<'_>> {
        self.get_entry(id).map(|_| NodeRef::new(self, id))
    }

    pub fn text_view(&self, id: NodeId) -> Option<TextView<'_>> {
        self.node(id)?.as_text()
    }

    pub fn text_identity(&self) -> TextIdentityView<'_> {
        TextIdentityView::new(self)
    }

    pub fn root(&self) -> Option<NodeRef<'_>> {
        self.nodes
            .iter()
            .find(|(_, kind)| **kind == NodeType::Root)
            .map(|(id, _)| NodeRef::new(self, *id))
    }

    pub fn get_entry(&self, id: NodeId) -> Option<&NodeEntry> {
        if !self.nodes.contains_key(&id) {
            return None;
        }
        self.entries.get(&id)
    }

    pub fn style_entry(&self, style_id: &str) -> Option<&StyleEntry> {
        self.style_entries.get(style_id)
    }

    pub fn style_entries_iter(&self) -> impl Iterator<Item = (&String, &StyleEntry)> + '_ {
        self.style_entries.iter()
    }

    pub fn style_present(&self, style_id: &str) -> bool {
        self.styles.contains_key(&style_id.to_string())
    }

    pub fn styles_iter(&self) -> impl Iterator<Item = (&String, &())> + '_ {
        self.styles.iter()
    }

    pub fn styles_tags_for<'a>(
        &'a self,
        style_id: &'a String,
    ) -> impl Iterator<Item = &'a Dot> + 'a {
        self.styles.tags_for(style_id)
    }

    pub fn nodes_iter(&self) -> impl Iterator<Item = (&NodeId, &NodeType)> + '_ {
        self.nodes.iter()
    }

    pub fn nodes_tags_for<'a>(&'a self, id: &'a NodeId) -> impl Iterator<Item = &'a Dot> + 'a {
        self.nodes.tags_for(id)
    }

    pub fn extract_text(&self) -> String {
        let mut out = String::new();
        if let Some(root) = self.root() {
            self.extract_text_recursive(root.id(), &mut out);
        }
        out.trim_end_matches('\n').to_string()
    }

    /// Marks a text node's projection stale to be rebuilt at the next
    /// [`flush_text_projections`](Self::flush_text_projections) instead of
    /// immediately, so a multi-op batch refreshes each node once.
    pub(crate) fn mark_text_dirty(&mut self, node_id: NodeId) {
        self.pending_text_refresh.0.insert(node_id);
    }

    /// Rebuilds every text node marked dirty since the last flush (projection +
    /// its flat-layout size). Must be called at each apply boundary before the
    /// doc is measured, queried, or compared.
    pub fn flush_text_projections(&mut self) {
        if self.pending_text_refresh.0.is_empty() {
            return;
        }
        for node_id in std::mem::take(&mut self.pending_text_refresh.0) {
            self.refresh_text_projection(node_id);
            self.update_flat_text_size(node_id);
        }
    }

    pub(crate) fn refresh_text_projection(&mut self, node_id: NodeId) {
        let Some(visible) = self.text_projection_for(node_id) else {
            return;
        };
        let Some(entry) = self.entries.get_mut(&node_id) else {
            return;
        };
        let Node::Text(text_node) = &mut entry.node else {
            return;
        };
        text_node.text = Text::from_visible_placements(visible);
        // Flat-layout maintenance is handled at the `apply_doc_op` level (text
        // ops update incrementally, structural ops invalidate), so this internal
        // text refresh does not touch the cache itself.
    }

    /// Incrementally updates the cached flat layout after a text op changed
    /// `node_id`'s content: `O(log N)` if the layout is built and the node is a
    /// text leaf, otherwise invalidate (lazy rebuild). Keeps typing off the
    /// O(N) full-rebuild path.
    pub(crate) fn update_flat_text_size(&mut self, node_id: NodeId) {
        let new_size = match self.get_entry(node_id).map(|e| &e.node) {
            Some(Node::Text(t)) => t.text.len(),
            _ => {
                self.invalidate_flat_layout();
                return;
            }
        };
        if let Some(old) = self.flat_layout.0.take() {
            if let Some(updated) = old.with_text_size(node_id, new_size) {
                let _ = self.flat_layout.0.set(std::sync::Arc::new(updated));
                #[cfg(debug_assertions)]
                self.debug_assert_flat_layout_matches_rebuild(node_id);
            }
            // `None` (node not a text leaf in this layout) → leave empty for a
            // lazy rebuild, which will be correct.
        }
        // Not yet built → nothing to do; the lazy build reflects current content.
    }

    /// Debug-only guard: the incrementally-maintained flat layout must agree with
    /// a from-scratch rebuild. Catches any divergence in dev/CI at no release cost.
    #[cfg(debug_assertions)]
    fn debug_assert_flat_layout_matches_rebuild(&self, node_id: NodeId) {
        let Some(maintained) = self.flat_layout.0.get() else {
            return;
        };
        let fresh = self.build_flat_layout();
        debug_assert_eq!(
            maintained.size(),
            fresh.size(),
            "incremental flat size diverged from rebuild"
        );
        debug_assert_eq!(
            maintained.text_start(node_id),
            fresh.text_start(node_id),
            "incremental flat text_start diverged from rebuild"
        );
    }

    fn text_projection_for(&self, node_id: NodeId) -> Option<Vec<TextPlacement>> {
        let entry = self.get_entry(node_id)?;
        let Node::Text(_) = &entry.node else {
            return None;
        };
        Some(self.text.visible_placements_for_node(node_id))
    }

    fn extract_text_recursive(&self, node_id: NodeId, out: &mut String) {
        let Some(entry) = self.get_entry(node_id) else {
            return;
        };
        match &entry.node {
            Node::Text(_) => out.push_str(
                &self
                    .text_view(node_id)
                    .map(|text| text.text())
                    .unwrap_or_default(),
            ),
            Node::HardBreak(_)
            | Node::PageBreak(_)
            | Node::Image(_)
            | Node::File(_)
            | Node::Embed(_)
            | Node::Archived(_) => {}
            _ => {
                for child_id in entry.children.iter().copied() {
                    self.extract_text_recursive(child_id, out);
                }
                out.push('\n');
            }
        }
    }

    pub fn verify(&self) -> Result<(), ModelError> {
        self.verify_root_uniqueness()?;
        self.verify_tree_reciprocity()?;
        #[cfg(any(test, debug_assertions))]
        self.verify_text_store()?;
        Ok(())
    }

    fn verify_root_uniqueness(&self) -> Result<(), ModelError> {
        let count = self
            .nodes_iter()
            .filter(|(_, k)| **k == NodeType::Root)
            .count();
        if count != 1 {
            return Err(ModelError::RootUniquenessViolation { count });
        }
        Ok(())
    }

    fn verify_tree_reciprocity(&self) -> Result<(), ModelError> {
        let Some(root) = self.root() else {
            return Ok(());
        };
        let root_id = root.id();

        let mut listed_edges: HashSet<(NodeId, NodeId)> = HashSet::new();
        for (parent_id, _kind) in self.nodes_iter() {
            if let Some(entry) = self.get_entry(*parent_id) {
                for child_id in entry.children.iter().copied() {
                    listed_edges.insert((*parent_id, child_id));
                }
            }
        }

        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<NodeId> = VecDeque::new();
        queue.push_back(root_id);

        while let Some(id) = queue.pop_front() {
            if !visited.insert(id) {
                return Err(ModelError::ParentChildDesync {
                    parent: id,
                    child: id,
                });
            }
            let entry = self.get_entry(id).ok_or(ModelError::NodeNotFound(id))?;
            for child_id in entry.children.iter().copied() {
                let child_entry =
                    self.get_entry(child_id)
                        .ok_or(ModelError::ParentChildDesync {
                            parent: id,
                            child: child_id,
                        })?;
                if child_entry.parent.get() != &Some(id) {
                    return Err(ModelError::ParentChildDesync {
                        parent: id,
                        child: child_id,
                    });
                }
                queue.push_back(child_id);
            }
            if let Some(parent_id) = *entry.parent.get() {
                if !listed_edges.contains(&(parent_id, id)) {
                    return Err(ModelError::ParentChildDesync {
                        parent: parent_id,
                        child: id,
                    });
                }
            }
        }

        for (id, _kind) in self.nodes_iter() {
            if !visited.contains(id) {
                return Err(ModelError::NodeUnreachable { node_id: *id });
            }
        }

        Ok(())
    }

    #[cfg(any(test, debug_assertions))]
    fn verify_text_store(&self) -> Result<(), ModelError> {
        if !self.text.index_matches_rebuild(self) {
            return Err(ModelError::TextIndexDesync);
        }

        for (node_id, node_type) in self.nodes_iter() {
            if *node_type != NodeType::Text {
                continue;
            }
            let entry = self
                .get_entry(*node_id)
                .ok_or(ModelError::NodeNotFound(*node_id))?;
            let Node::Text(text_node) = &entry.node else {
                return Err(ModelError::TextProjectionDesync { node_id: *node_id });
            };
            let actual: Vec<TextPlacement> = text_node.text.iter_visible_placements().collect();
            let expected = self.text.visible_placements_for_node(*node_id);
            if actual != expected {
                return Err(ModelError::TextProjectionDesync { node_id: *node_id });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::{EntryDot, PlacementId};
    use editor_macros::doc;

    use super::*;
    use crate::*;

    #[test]
    fn empty_doc_has_no_root() {
        let doc = Doc::empty();
        assert!(doc.root().is_none());
    }

    #[test]
    fn node_returns_none_for_missing() {
        let doc = Doc::empty();
        assert!(doc.node(NodeId::new()).is_none());
    }

    fn make_doc() -> Doc {
        let (doc, ..) = doc! {
            root {
                paragraph {
                    text("Hello")
                }
            }
        };
        doc
    }

    #[test]
    fn verify_accepts_rooted_doc() {
        let (doc, ..) = doc! { root {} };
        assert!(doc.verify().is_ok());
    }

    #[test]
    fn verify_rejects_stale_text_projection() {
        let (mut doc, t1) = doc! {
            root {
                paragraph {
                    t1: text("a")
                }
            }
        };
        let entry = doc.entries.get_mut(&t1).unwrap();
        let Node::Text(text_node) = &mut entry.node else {
            panic!("expected text node");
        };
        text_node.text = Text::new();

        assert_eq!(
            doc.verify(),
            Err(ModelError::TextProjectionDesync { node_id: t1 })
        );
    }

    #[test]
    fn verify_rejects_zero_roots() {
        let doc = Doc::empty();
        let result = doc.verify();
        assert!(matches!(
            result,
            Err(ModelError::RootUniquenessViolation { count: 0 })
        ));
    }

    #[test]
    fn node_returns_some_for_existing() {
        let doc = make_doc();
        assert!(doc.node(NodeId::ROOT).is_some());
    }

    #[test]
    fn root_returns_root_node() {
        let doc = make_doc();
        let root = doc.root().unwrap();
        assert!(matches!(root.node(), &Node::Root(_)));
    }

    #[test]
    fn clone_is_o1() {
        let doc = make_doc();
        let doc2 = doc.clone();
        assert!(doc.node(NodeId::ROOT).is_some());
        assert!(doc2.node(NodeId::ROOT).is_some());
    }

    #[test]
    fn extract_text_concatenates_text_nodes() {
        let (doc, ..) = doc! {
            root {
                paragraph {
                    text("hello")
                    text(" world")
                }
            }
        };
        let text = doc.extract_text();
        assert!(text.contains("hello"));
        assert!(text.contains("world"));
    }

    #[test]
    fn extract_text_exact_output() {
        let (doc, ..) = doc! {
            root {
                paragraph {
                    text("hello world")
                }
            }
        };
        let text = doc.extract_text();
        assert_eq!(text, "hello world");
    }

    #[test]
    fn extract_text_hard_break_does_not_add_newline() {
        let (doc, ..) = doc! {
            root {
                paragraph {
                    text("first")
                    hard_break
                    text("second")
                }
            }
        };
        let text = doc.extract_text();
        assert_eq!(text, "firstsecond");
    }

    #[test]
    fn extract_text_preserves_block_separation() {
        let (doc, ..) = doc! {
            root {
                paragraph {
                    text("first")
                }
                paragraph {
                    text("second")
                }
            }
        };
        let text = doc.extract_text();
        assert!(text.contains("first"));
        assert!(text.contains("second"));
        let pos1 = text.find("first").unwrap();
        let pos2 = text.find("second").unwrap();
        assert!(pos2 > pos1);
        let between = &text[pos1 + 5..pos2];
        assert!(
            between.contains('\n'),
            "expected newline between blocks: {:?}",
            between
        );
    }

    #[test]
    fn from_op_graph_at_materializes_past_point() {
        use crate::doc_op::DocOp;
        use editor_crdt::Dot;
        use hashbrown::HashSet;

        let mut g: OpGraph<DocOp> = OpGraph::with_actor(1);
        let root = NodeId::ROOT;
        let para = NodeId::new();
        let txt = NodeId::new();

        let add = |g: &mut OpGraph<DocOp>, payload: DocOp| {
            let (ng, op) = g.clone().add(payload).unwrap();
            *g = ng;
            op.id
        };
        add(
            &mut g,
            DocOp::Presence {
                node_id: root,
                op: editor_crdt::OrMapOp::Set {
                    key: root,
                    value: NodeType::Root,
                },
            },
        );
        add(
            &mut g,
            DocOp::Presence {
                node_id: para,
                op: editor_crdt::OrMapOp::Set {
                    key: para,
                    value: NodeType::Paragraph,
                },
            },
        );
        add(
            &mut g,
            DocOp::Parent {
                node_id: para,
                op: editor_crdt::LwwRegOp::Set { value: Some(root) },
            },
        );
        add(
            &mut g,
            DocOp::Children {
                node_id: root,
                op: editor_crdt::RgaOp::Insert {
                    after: None,
                    value: para,
                },
            },
        );
        add(
            &mut g,
            DocOp::Presence {
                node_id: txt,
                op: editor_crdt::OrMapOp::Set {
                    key: txt,
                    value: NodeType::Text,
                },
            },
        );
        add(
            &mut g,
            DocOp::Parent {
                node_id: txt,
                op: editor_crdt::LwwRegOp::Set { value: Some(para) },
            },
        );
        add(
            &mut g,
            DocOp::Children {
                node_id: para,
                op: editor_crdt::RgaOp::Insert {
                    after: None,
                    value: txt,
                },
            },
        );
        let a_dot = add(
            &mut g,
            DocOp::Text {
                node_id: txt,
                op: editor_crdt::TextOp::InsertChar {
                    after: None,
                    ch: 'a',
                },
            },
        );
        let heads_at_a: HashSet<Dot> = [a_dot].into_iter().collect();
        let _b_dot = add(
            &mut g,
            DocOp::Text {
                node_id: txt,
                op: editor_crdt::TextOp::InsertChar {
                    after: Some(PlacementId(a_dot)),
                    ch: 'b',
                },
            },
        );

        let now = Doc::from_op_graph(&g).unwrap();
        assert_eq!(now.extract_text(), "ab");

        let past = Doc::from_op_graph_at(&g, &heads_at_a).unwrap();
        assert_eq!(past.extract_text(), "a");
    }

    #[test]
    fn from_op_graph_at_rejects_unknown_head() {
        use crate::doc_op::DocOp;
        use editor_crdt::Dot;
        use hashbrown::HashSet;

        let g: OpGraph<DocOp> = OpGraph::with_actor(1);
        let unknown: HashSet<Dot> = [Dot::new(42, 7)].into_iter().collect();
        assert!(matches!(
            Doc::from_op_graph_at(&g, &unknown),
            Err(ModelError::InvalidHead { .. })
        ));
    }

    #[test]
    fn text_index_uses_birth_location_fallback_for_unmoved_entries() {
        use crate::doc_op::DocOp;

        let mut graph = OpGraph::<DocOp>::new();
        let text_id = NodeId::new();

        let apply = |graph: &mut OpGraph<DocOp>, doc: Doc, payload: DocOp| {
            let (next_graph, op) = graph.clone().add(payload).unwrap();
            *graph = next_graph;
            let doc = apply_doc_op(doc, &op).unwrap();
            (doc, op)
        };

        let (doc, _) = apply(
            &mut graph,
            Doc::empty(),
            DocOp::Presence {
                node_id: text_id,
                op: editor_crdt::OrMapOp::Set {
                    key: text_id,
                    value: NodeType::Text,
                },
            },
        );
        let (doc, insert) = apply(
            &mut graph,
            doc,
            DocOp::Text {
                node_id: text_id,
                op: editor_crdt::TextOp::InsertChar {
                    after: None,
                    ch: 'a',
                },
            },
        );

        let entry = EntryDot(insert.id);
        assert_eq!(
            doc.text_identity()
                .current_location(entry)
                .map(|loc| (loc.node_id, loc.placement_id)),
            Some((text_id, PlacementId(insert.id)))
        );
        assert!(doc.text.moved_location(entry).is_none());
        assert!(doc.text.index_matches_rebuild(&doc));
    }

    #[test]
    fn text_current_location_uses_materialized_index_after_move() {
        use crate::doc_op::DocOp;

        let mut graph = OpGraph::<DocOp>::new();
        let t1 = NodeId::new();
        let t2 = NodeId::new();

        let apply = |graph: &mut OpGraph<DocOp>, doc: Doc, payload: DocOp| {
            let (next_graph, op) = graph.clone().add(payload).unwrap();
            *graph = next_graph;
            let doc = apply_doc_op(doc, &op).unwrap();
            (doc, op)
        };

        let (doc, _) = apply(
            &mut graph,
            Doc::empty(),
            DocOp::Presence {
                node_id: t1,
                op: editor_crdt::OrMapOp::Set {
                    key: t1,
                    value: NodeType::Text,
                },
            },
        );
        let (doc, _) = apply(
            &mut graph,
            doc,
            DocOp::Presence {
                node_id: t2,
                op: editor_crdt::OrMapOp::Set {
                    key: t2,
                    value: NodeType::Text,
                },
            },
        );
        let (doc, insert) = apply(
            &mut graph,
            doc,
            DocOp::Text {
                node_id: t1,
                op: editor_crdt::TextOp::InsertChar {
                    after: None,
                    ch: 'a',
                },
            },
        );
        let (doc, move_op) = apply(
            &mut graph,
            doc,
            DocOp::MoveText {
                entry: EntryDot(insert.id),
                to_node_id: t2,
                after: None,
            },
        );

        let current = doc.text.moved_location(EntryDot(insert.id)).unwrap();
        assert_eq!(current.owner_text_node, t2);
        assert_eq!(current.placement, PlacementId(move_op.id));
        assert_eq!(
            doc.text_identity()
                .current_location(EntryDot(insert.id))
                .map(|loc| (loc.node_id, loc.placement_id)),
            Some((t2, PlacementId(move_op.id)))
        );
        assert!(doc.text.index_matches_rebuild(&doc));
    }

    #[test]
    fn root_default_has_continuous_layout_and_default_modifiers() {
        let doc = make_doc();
        let root = doc.get_entry(NodeId::ROOT).unwrap();
        match &root.node {
            Node::Root(r) => {
                assert!(matches!(r.layout_mode.get(), LayoutMode::Continuous { .. }))
            }
            _ => panic!("expected Root"),
        }
        assert!(
            root.modifiers
                .iter()
                .any(|(_, m)| matches!(m, Modifier::FontFamily { value } if value == "Pretendard"))
        );
    }
}
