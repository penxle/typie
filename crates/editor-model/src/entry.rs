use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::id::NodeId;
use crate::modifier::Modifier;
use crate::nodes::*;

#[ffi]
#[derive(Clone, Debug, Serialize, Deserialize)]
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_is_structural_sharing() {
        let mut children = imbl::Vector::new();
        children.push_back(NodeId::new());
        children.push_back(NodeId::new());
        children.push_back(NodeId::new());

        let entry = NodeEntry {
            node: Node::Root(RootNode {}),
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
}
