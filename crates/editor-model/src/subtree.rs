use crate::marker::Marker;
use crate::modifier::Modifier;
use crate::nodes::PlainNode;

#[derive(Clone, Debug, PartialEq)]
pub struct Subtree {
    pub node: PlainNode,
    pub modifiers: Vec<Modifier>,
    pub style: Option<String>,
    pub marker: Option<Marker>,
    pub children: Vec<Subtree>,
}

impl Subtree {
    pub fn leaf(node: PlainNode) -> Self {
        Self {
            node,
            modifiers: vec![],
            style: None,
            marker: None,
            children: vec![],
        }
    }

    pub fn with_children(mut self, children: Vec<Subtree>) -> Self {
        self.children = children;
        self
    }

    pub fn with_modifiers(mut self, modifiers: Vec<Modifier>) -> Self {
        self.modifiers = modifiers;
        self
    }
}
