use editor_crdt::{Dot, OpGraph, OrMap};
use hashbrown::HashSet;
use std::collections::VecDeque;

use crate::apply_doc_op;
use crate::doc_op::DocOp;
use crate::entry::NodeEntry;
use crate::error::ModelError;
use crate::id::NodeId;
use crate::node_ref::NodeRef;
use crate::nodes::{Node, NodeType};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Doc {
    pub(crate) nodes: OrMap<NodeId, NodeType>,
    pub(crate) entries: imbl::HashMap<NodeId, NodeEntry>,
}

impl Doc {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_op_graph(graph: &OpGraph<DocOp>) -> Result<Self, ModelError> {
        let dots: HashSet<Dot> = graph.iter_all().map(|op| op.id).collect();
        let mut doc = Doc::empty();
        for op in graph.topo_sort(&dots) {
            doc = apply_doc_op(doc, &op)?;
        }
        Ok(doc)
    }

    pub fn node(&self, id: NodeId) -> Option<NodeRef<'_>> {
        self.get_entry(id).map(|_| NodeRef::new(self, id))
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

    fn extract_text_recursive(&self, node_id: NodeId, out: &mut String) {
        let Some(entry) = self.get_entry(node_id) else {
            return;
        };
        match &entry.node {
            Node::Text(t) => out.push_str(&t.text.to_string()),
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
            if let Some(parent_id) = entry.parent.get().clone() {
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
}

#[cfg(test)]
mod tests {
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
