use crate::id::NodeId;
use crate::modifier::Modifier;
use crate::nodes::PlainNode;
use crate::subtree::Subtree;
use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fragment {
    pub node: PlainNode,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub modifiers: Vec<Modifier>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub style: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<Fragment>,
}

impl Fragment {
    pub fn leaf(node: PlainNode) -> Self {
        Self {
            node,
            modifiers: vec![],
            style: None,
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
            style: self.style,
            marker: None,
            children: self
                .children
                .into_iter()
                .map(|f| f.into_subtree())
                .collect(),
        }
    }
}
