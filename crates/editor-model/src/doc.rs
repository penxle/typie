use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::entry::NodeEntry;
use crate::id::NodeId;
use crate::node_ref::NodeRef;
use crate::nodes::{Node, RootNode};

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Doc {
    pub nodes: imbl::HashMap<NodeId, NodeEntry>,
}

impl Default for Doc {
    fn default() -> Self {
        Self {
            nodes: imbl::hashmap! {
                NodeId::ROOT => NodeEntry {
                    node: Node::Root(RootNode::default()),
                    parent: None,
                    children: imbl::Vector::new(),
                    modifiers: vec![],
                }
            },
        }
    }
}

impl Doc {
    pub fn node(&self, id: NodeId) -> Option<NodeRef<'_>> {
        if self.nodes.contains_key(&id) {
            Some(NodeRef::new(self, id))
        } else {
            None
        }
    }

    pub fn root(&self) -> NodeRef<'_> {
        NodeRef::new(self, NodeId::ROOT)
    }

    pub fn get_entry(&self, id: NodeId) -> Option<&NodeEntry> {
        self.nodes.get(&id)
    }

    pub fn with_node(&self, id: NodeId, entry: NodeEntry) -> Doc {
        let mut new = self.clone();
        new.nodes = new.nodes.update(id, entry);
        new
    }

    pub fn with_node_updated(&self, id: NodeId, f: impl FnOnce(NodeEntry) -> NodeEntry) -> Doc {
        let mut new = self.clone();
        if let Some(entry) = new.nodes.get(&id).cloned() {
            new.nodes = new.nodes.update(id, f(entry));
        }
        new
    }

    pub fn insert_node(&self, id: NodeId, entry: NodeEntry) -> Doc {
        let mut new = self.clone();
        new.nodes = new.nodes.update(id, entry);
        new
    }

    pub fn remove_node(&self, id: NodeId) -> Doc {
        let mut new = self.clone();
        new.nodes = new.nodes.without(&id);
        new
    }
}

#[cfg(any(test, feature = "test-utils"))]
impl Doc {
    pub fn new_test() -> Self {
        use crate::default_modifiers;

        Self {
            nodes: imbl::hashmap! {
                NodeId::ROOT => NodeEntry {
                    node: Node::Root(RootNode::default()),
                    parent: None,
                    children: imbl::Vector::new(),
                    modifiers: default_modifiers(),
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;
    use crate::*;

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
    fn node_returns_some_for_existing() {
        let doc = make_doc();
        assert!(doc.node(NodeId::ROOT).is_some());
    }

    #[test]
    fn node_returns_none_for_missing() {
        let doc = make_doc();
        assert!(doc.node(NodeId::new()).is_none());
    }

    #[test]
    fn root_returns_root_node() {
        let doc = make_doc();
        let root = doc.root();
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
    fn with_node_returns_new_doc() {
        let doc = make_doc();
        let new_id = NodeId::new();
        let doc2 = doc.insert_node(new_id, NodeEntry::new(Node::HardBreak(HardBreakNode {})));
        assert!(doc.node(new_id).is_none());
        assert!(doc2.node(new_id).is_some());
    }

    #[test]
    fn with_node_updated() {
        let doc = make_doc();
        let root = doc.root();
        let p1 = root.entry().children[0];

        let doc2 = doc.with_node_updated(p1, |mut entry| {
            entry.modifiers.push(Modifier::Bold);
            entry
        });

        let updated = doc2.node(p1).unwrap();
        assert!(updated.modifiers().contains(&Modifier::Bold));

        let original = doc.node(p1).unwrap();
        assert!(!original.modifiers().contains(&Modifier::Bold));
    }

    #[test]
    fn remove_node() {
        let doc = make_doc();
        let new_id = NodeId::new();
        let doc2 = doc.insert_node(new_id, NodeEntry::new(Node::HardBreak(HardBreakNode {})));
        let doc3 = doc2.remove_node(new_id);
        assert!(doc3.node(new_id).is_none());
    }

    #[test]
    fn root_default_has_continuous_layout_and_default_modifiers() {
        let doc = make_doc();
        let root = doc.get_entry(NodeId::ROOT).unwrap();
        match &root.node {
            Node::Root(r) => assert!(matches!(r.layout_mode, LayoutMode::Continuous { .. })),
            _ => panic!("expected Root"),
        }
        assert!(
            root.modifiers
                .iter()
                .any(|m| matches!(m, Modifier::FontFamily { value } if value == "Pretendard"))
        );
    }
}
