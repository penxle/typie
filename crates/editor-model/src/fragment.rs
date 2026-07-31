use crate::modifier::Modifier;
use crate::nodes::PlainNode;
use crate::subtree::Subtree;
use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Fragment {
    pub node: PlainNode,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub modifiers: Vec<Modifier>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub carry: Vec<Modifier>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: Vec<Fragment>,
}

impl Fragment {
    pub fn leaf(node: PlainNode) -> Self {
        Self {
            node,
            modifiers: vec![],
            carry: vec![],
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
        struct Frame {
            node: PlainNode,
            modifiers: Vec<Modifier>,
            carry: Vec<Modifier>,
            children: std::vec::IntoIter<Fragment>,
            converted: Vec<Subtree>,
        }

        impl Frame {
            fn new(fragment: Fragment) -> Self {
                let (node, modifiers, carry, children) = fragment.into_parts();
                Self {
                    node,
                    modifiers,
                    carry,
                    converted: Vec::with_capacity(children.len()),
                    children: children.into_iter(),
                }
            }

            fn finish(self) -> Subtree {
                Subtree {
                    node: self.node,
                    modifiers: self.modifiers,
                    carry: self.carry,
                    children: self.converted,
                    source_dots: Vec::new(),
                }
            }
        }

        let mut stack = vec![Frame::new(self)];
        loop {
            if let Some(child) = stack.last_mut().and_then(|frame| frame.children.next()) {
                stack.push(Frame::new(child));
                continue;
            }
            let converted = stack.pop().expect("root conversion frame").finish();
            let Some(parent) = stack.last_mut() else {
                return converted;
            };
            parent.converted.push(converted);
        }
    }

    pub fn into_parts(mut self) -> (PlainNode, Vec<Modifier>, Vec<Modifier>, Vec<Fragment>) {
        (
            std::mem::replace(&mut self.node, PlainNode::Unknown),
            std::mem::take(&mut self.modifiers),
            std::mem::take(&mut self.carry),
            std::mem::take(&mut self.children),
        )
    }
}

impl Clone for Fragment {
    fn clone(&self) -> Self {
        struct Frame<'a> {
            source: &'a Fragment,
            next_child: usize,
            children: Vec<Fragment>,
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
            let cloned = Fragment {
                node: frame.source.node.clone(),
                modifiers: frame.source.modifiers.clone(),
                carry: frame.source.carry.clone(),
                children: frame.children,
            };
            let Some(parent) = stack.last_mut() else {
                return cloned;
            };
            parent.children.push(cloned);
        }
    }
}

impl Drop for Fragment {
    fn drop(&mut self) {
        let mut stack = std::mem::take(&mut self.children);
        while let Some(mut fragment) = stack.pop() {
            stack.append(&mut fragment.children);
        }
    }
}
