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
}

#[derive(Default)]
struct ChildIndexCache(OnceLock<ChildIndex>);

impl Clone for ChildIndexCache {
    fn clone(&self) -> Self {
        Self(OnceLock::new())
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

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Doc {
    pub(crate) nodes: OrMap<NodeId, NodeType>,
    pub(crate) entries: imbl::HashMap<NodeId, NodeEntry>,
    pub(crate) text: DocTextStore,
    pub(crate) stable_position_remap: StablePositionRemapStore,
    pub(crate) styles: OrMap<String, ()>,
    pub(crate) style_entries: imbl::HashMap<String, StyleEntry>,
    child_index: ChildIndexCache,
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

    fn child_index(&self) -> &ChildIndex {
        self.child_index.0.get_or_init(|| self.build_child_index())
    }

    fn build_child_index(&self) -> ChildIndex {
        let mut pos = HashMap::new();
        let mut ordered = HashMap::new();
        for (parent_id, entry) in self.entries.iter() {
            let children: Vec<NodeId> = entry.children.iter().copied().collect();
            for (i, &child) in children.iter().enumerate() {
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
            }
            ordered.insert(*parent_id, children);
        }
        ChildIndex { pos, ordered }
    }

    pub(crate) fn invalidate_child_index(&mut self) {
        self.child_index.0.take();
    }

    pub fn from_op_graph(graph: &OpGraph<DocOp>) -> Result<Self, ModelError> {
        let dots: HashSet<Dot> = graph.iter_all().map(|op| op.id).collect();
        let mut doc = Doc::empty();
        for op in graph.topo_sort(&dots) {
            doc = apply_doc_op(doc, &op)?;
        }
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
                let parent_entry =
                    self.get_entry(parent_id)
                        .ok_or(ModelError::ParentChildDesync {
                            parent: parent_id,
                            child: id,
                        })?;
                if !parent_entry.children.iter().any(|c| c == &id) {
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
