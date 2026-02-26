use crate::model::{Doc, NodeId};
use anyhow::Result;
use std::rc::Rc;

pub struct BlockTraverser<'a> {
    doc: &'a Doc,
    stack: Vec<(NodeId, Rc<Vec<NodeId>>, usize)>,
}

#[derive(Clone, Copy)]
enum StartPosition {
    // for forward traversal from node
    AtStart,
    // for forward traversal after subtree
    AfterChildren,
    // for backward traversal before subtree
    BeforeChildren,
}

impl<'a> BlockTraverser<'a> {
    pub fn empty(doc: &'a Doc) -> Self {
        Self {
            doc,
            stack: Vec::new(),
        }
    }

    pub fn new(doc: &'a Doc, start_node: NodeId) -> Result<Self> {
        Self::with_start(doc, start_node, StartPosition::AtStart)
    }

    pub fn new_after_subtree(doc: &'a Doc, start_node: NodeId) -> Result<Self> {
        Self::with_start(doc, start_node, StartPosition::AfterChildren)
    }

    pub fn new_before_subtree(doc: &'a Doc, start_node: NodeId) -> Result<Self> {
        Self::with_start(doc, start_node, StartPosition::BeforeChildren)
    }

    fn with_start(doc: &'a Doc, start_node: NodeId, position: StartPosition) -> Result<Self> {
        let mut stack = Vec::new();
        let mut path = vec![start_node];
        let mut current = start_node;
        while let Some(parent) = doc.get_parent_id(current) {
            path.push(parent);
            current = parent;
        }
        path.reverse();

        for i in 0..path.len() {
            let node_id = path[i];
            let children = doc.get_children_ids(node_id);

            let index = if i + 1 < path.len() {
                let next_node = path[i + 1];
                let child_pos = children.iter().position(|&id| id == next_node).unwrap_or(0);
                match position {
                    StartPosition::AtStart | StartPosition::AfterChildren => child_pos + 1,
                    StartPosition::BeforeChildren => child_pos,
                }
            } else {
                match position {
                    StartPosition::AtStart => 0,
                    StartPosition::AfterChildren => children.len(),
                    StartPosition::BeforeChildren => 0,
                }
            };

            stack.push((node_id, children, index));
        }

        Ok(Self { doc, stack })
    }

    pub fn next(&mut self) -> Option<NodeId> {
        loop {
            if self.stack.is_empty() {
                return None;
            }

            let len = self.stack.len();
            let (_parent_id, children, idx) = &mut self.stack[len - 1];

            if *idx >= children.len() {
                self.stack.pop();
                continue;
            }

            let child_id = children[*idx];
            *idx += 1;

            if let Some(node_type) = self.doc.get_node_type(child_id) {
                let spec = self.doc.schema().node_spec(node_type);
                if !spec.inline {
                    let grandchildren = self.doc.get_children_ids(child_id);
                    self.stack.push((child_id, grandchildren, 0));
                    return Some(child_id);
                }
            }
        }
    }

    pub fn prev(&mut self) -> Option<NodeId> {
        loop {
            if self.stack.is_empty() {
                return None;
            }

            let len = self.stack.len();
            let (_parent_id, children, idx) = &mut self.stack[len - 1];

            if *idx == 0 {
                self.stack.pop();
                continue;
            }

            *idx -= 1;
            let child_id = children[*idx];

            if let Some(node_type) = self.doc.get_node_type(child_id) {
                let spec = self.doc.schema().node_spec(node_type);
                if !spec.inline {
                    let grandchildren = self.doc.get_children_ids(child_id);
                    let len = grandchildren.len();
                    self.stack.push((child_id, grandchildren, len));
                    return Some(child_id);
                }
            }
        }
    }
}
