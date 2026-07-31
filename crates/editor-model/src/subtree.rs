use editor_crdt::Dot;

use crate::modifier::Modifier;
use crate::nodes::PlainNode;

#[derive(Debug, PartialEq)]
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

    pub fn into_parts(
        mut self,
    ) -> (
        PlainNode,
        Vec<Modifier>,
        Vec<Modifier>,
        Vec<Subtree>,
        Vec<Dot>,
    ) {
        (
            std::mem::replace(&mut self.node, PlainNode::Unknown),
            std::mem::take(&mut self.modifiers),
            std::mem::take(&mut self.carry),
            std::mem::take(&mut self.children),
            std::mem::take(&mut self.source_dots),
        )
    }
}

impl Clone for Subtree {
    fn clone(&self) -> Self {
        struct Frame<'a> {
            source: &'a Subtree,
            next_child: usize,
            children: Vec<Subtree>,
        }

        let mut stack = vec![Frame {
            source: self,
            next_child: 0,
            children: Vec::with_capacity(self.children.len()),
        }];
        loop {
            let next = {
                let frame = stack.last_mut().expect("root clone frame");
                let child = frame.source.children.get(frame.next_child);
                frame.next_child += usize::from(child.is_some());
                child
            };
            if let Some(child) = next {
                stack.push(Frame {
                    source: child,
                    next_child: 0,
                    children: Vec::with_capacity(child.children.len()),
                });
                continue;
            }
            let frame = stack.pop().expect("root clone frame");
            let cloned = Subtree {
                node: frame.source.node.clone(),
                modifiers: frame.source.modifiers.clone(),
                carry: frame.source.carry.clone(),
                children: frame.children,
                source_dots: frame.source.source_dots.clone(),
            };
            let Some(parent) = stack.last_mut() else {
                return cloned;
            };
            parent.children.push(cloned);
        }
    }
}

impl Drop for Subtree {
    fn drop(&mut self) {
        let mut stack = std::mem::take(&mut self.children);
        while let Some(mut subtree) = stack.pop() {
            stack.append(&mut subtree.children);
        }
    }
}
