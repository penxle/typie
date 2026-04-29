use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::id::NodeId;
use crate::modifier::Modifier;
use crate::nodes::*;
use crate::object::ChildRef;

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeEntry {
    pub node: Node,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub parent: Option<NodeId>,
    #[serde(skip_serializing_if = "imbl::Vector::is_empty", default)]
    #[cfg_attr(feature = "wasm", tsify(type = "NodeId[]"))]
    pub children: imbl::Vector<NodeId>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub modifiers: Vec<Modifier>,
}

impl NodeEntry {
    pub fn new(node: Node) -> Self {
        Self {
            node,
            parent: None,
            children: imbl::Vector::new(),
            modifiers: vec![],
        }
    }

    pub fn with_parent(mut self, parent: NodeId) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn with_children(mut self, children: imbl::Vector<NodeId>) -> Self {
        self.children = children;
        self
    }

    pub fn with_modifiers(mut self, modifiers: Vec<Modifier>) -> Self {
        self.modifiers = modifiers;
        self
    }

    pub fn content_hash(&self, node_id: NodeId, children_hashes: &[ChildRef]) -> String {
        crate::object::ObjectContent {
            node_id,
            node: self.node.clone(),
            parent: self.parent,
            modifiers: self.modifiers.clone(),
            children: children_hashes.to_vec(),
        }
        .hash()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_is_deterministic() {
        let entry = NodeEntry {
            node: Node::Text(TextNode {
                text: "hello".into(),
            }),
            parent: None,
            children: imbl::Vector::new(),
            modifiers: vec![],
        };
        let id = NodeId::new();
        let h1 = entry.content_hash(id, &[]);
        let h2 = entry.content_hash(id, &[]);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 32, "hex 32자 (xxh3 128-bit)");
    }

    #[test]
    fn content_hash_changes_with_text() {
        let id = NodeId::new();
        let e1 = NodeEntry {
            node: Node::Text(TextNode {
                text: "hello".into(),
            }),
            parent: None,
            children: imbl::Vector::new(),
            modifiers: vec![],
        };
        let e2 = NodeEntry {
            node: Node::Text(TextNode {
                text: "world".into(),
            }),
            ..e1.clone()
        };
        assert_ne!(e1.content_hash(id, &[]), e2.content_hash(id, &[]));
    }

    #[test]
    fn content_hash_changes_with_children_hashes() {
        let entry = NodeEntry {
            node: Node::Root(RootNode::default()),
            parent: None,
            children: imbl::Vector::new(),
            modifiers: vec![],
        };
        let id = NodeId::new();
        let cid = NodeId::new();
        let h1 = entry.content_hash(
            id,
            &[ChildRef {
                node_id: cid,
                hash: "aaa".into(),
            }],
        );
        let h2 = entry.content_hash(
            id,
            &[ChildRef {
                node_id: cid,
                hash: "bbb".into(),
            }],
        );
        assert_ne!(h1, h2);
    }

    #[test]
    fn clone_is_structural_sharing() {
        let mut children = imbl::Vector::new();
        children.push_back(NodeId::new());
        children.push_back(NodeId::new());
        children.push_back(NodeId::new());

        let entry = NodeEntry {
            node: Node::Root(RootNode::default()),
            parent: None,
            children: children.clone(),
            modifiers: vec![],
        };

        let cloned = entry.clone();
        assert_eq!(entry.children, cloned.children);
        assert_eq!(entry.children.len(), 3);
    }

    #[test]
    fn default_entry() {
        let entry = NodeEntry::new(Node::HardBreak(HardBreakNode {}));
        assert!(entry.parent.is_none());
        assert!(entry.children.is_empty());
        assert!(entry.modifiers.is_empty());
    }

    #[test]
    fn with_modifiers() {
        let entry = NodeEntry::new(Node::Text(TextNode { text: "hi".into() }))
            .with_modifiers(vec![Modifier::Bold]);
        assert_eq!(entry.modifiers, vec![Modifier::Bold]);
    }

    #[test]
    fn content_hash_changes_with_parent() {
        let id = NodeId::new();
        let e1 = NodeEntry::new(Node::Text(TextNode { text: "x".into() }));
        let e2 = e1.clone().with_parent(NodeId::new());
        assert_ne!(e1.content_hash(id, &[]), e2.content_hash(id, &[]));
    }

    #[test]
    fn content_hash_changes_with_modifiers() {
        let id = NodeId::new();
        let e1 = NodeEntry::new(Node::Text(TextNode { text: "x".into() }));
        let e2 = e1.clone().with_modifiers(vec![Modifier::Bold]);
        assert_ne!(e1.content_hash(id, &[]), e2.content_hash(id, &[]));
    }
}
