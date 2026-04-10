use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::id::NodeId;
use crate::modifier::Modifier;
use crate::nodes::Node;
use crate::subtree::Subtree;

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fragment {
    pub node: Node,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub modifiers: Vec<Modifier>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<Fragment>,
}

impl Fragment {
    pub fn leaf(node: Node) -> Self {
        Self {
            node,
            modifiers: vec![],
            children: vec![],
        }
    }

    pub fn with_children(mut self, children: Vec<Fragment>) -> Self {
        self.children = children;
        self
    }

    pub fn with_modifiers(mut self, modifiers: Vec<Modifier>) -> Self {
        self.modifiers = modifiers;
        self
    }

    pub fn into_subtree(self) -> Subtree {
        Subtree {
            id: NodeId::new(),
            node: self.node,
            modifiers: self.modifiers,
            children: self
                .children
                .into_iter()
                .map(|c| c.into_subtree())
                .collect(),
        }
    }
}
