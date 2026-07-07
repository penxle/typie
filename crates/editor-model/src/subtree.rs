use editor_crdt::Dot;

use crate::modifier::Modifier;
use crate::nodes::PlainNode;

#[derive(Clone, Debug, PartialEq)]
pub struct Subtree {
    pub node: PlainNode,
    pub modifiers: Vec<Modifier>,
    pub carry: Vec<Modifier>,
    pub children: Vec<Subtree>,
    /// The real op dots this subtree was captured from, in walk order — `Text`:
    /// one per char; `Block`/`Atom`: the node's own dot; a described (not
    /// captured) subtree: empty. Only `capture_subtree` fills this; every other
    /// constructor leaves it empty. Consumed by `emit_subtree` to pair each
    /// freshly-emitted dot back to the dot it replaces.
    pub source_dots: Vec<Dot>,
}

impl Subtree {
    pub fn leaf(node: PlainNode) -> Self {
        Self {
            node,
            modifiers: vec![],
            carry: vec![],
            children: vec![],
            source_dots: vec![],
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
