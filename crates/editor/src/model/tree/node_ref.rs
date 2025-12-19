use crate::model::tree::{DocInner, NodeMut};
use crate::model::*;
use crate::schema::{NodeSpec, Schema};
use std::cell::OnceCell;

#[derive(Debug)]
pub struct NodeRef<'a> {
    inner: &'a DocInner,
    node_id: NodeId,
    parent_id: OnceCell<Option<NodeId>>,
    node: OnceCell<Node>,
}

impl<'a> NodeRef<'a> {
    pub fn new(inner: &'a DocInner, node_id: NodeId) -> Option<Self> {
        if inner.get_node_map(node_id).is_none() {
            return None;
        }

        Some(Self {
            inner,
            node_id,
            parent_id: OnceCell::new(),
            node: OnceCell::new(),
        })
    }

    pub fn new_unchecked(inner: &'a DocInner, node_id: NodeId) -> Self {
        Self {
            inner,
            node_id,
            parent_id: OnceCell::new(),
            node: OnceCell::new(),
        }
    }

    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn node_type(&self) -> NodeType {
        self.node().as_type()
    }

    pub fn index(&self) -> Option<usize> {
        let Some(parent_id) = self.parent_id() else {
            return None;
        };

        let children = self.inner.get_children_list(parent_id)?;

        for i in 0..children.len() {
            let child = children.get(i)?;
            if self.node_id == *child.as_value()?.as_string()? {
                return Some(i);
            }
        }

        None
    }

    pub fn parent_id(&self) -> Option<NodeId> {
        *self.parent_id.get_or_init(|| {
            let map = self.inner.get_node_map(self.node_id)?;
            map.get("parent")
                .and_then(|v| v.into_value().ok())
                .and_then(|v| v.into_string().ok())
                .map(|v| NodeId::from_string(&v).unwrap())
        })
    }

    pub fn parent(&self) -> Option<NodeRef<'_>> {
        self.parent_id().and_then(|id| Self::new(self.inner, id))
    }

    pub fn prev_sibling(&self) -> Option<NodeRef<'_>> {
        let parent_id = self.parent_id()?;
        let children = self.inner.get_children_list(parent_id)?;

        for i in 0..children.len() {
            let child = children.get(i)?;
            let child_id = child
                .into_value()
                .ok()
                .and_then(|v| v.into_string().ok())
                .and_then(|s| NodeId::from_string(&s))?;

            if self.node_id == child_id {
                let prev_index = i.checked_sub(1)?;
                let prev_child = children.get(prev_index)?;
                let prev_id = prev_child
                    .into_value()
                    .ok()
                    .and_then(|v| v.into_string().ok())
                    .and_then(|s| NodeId::from_string(&s))?;
                return Self::new(self.inner, prev_id);
            }
        }

        None
    }

    pub fn next_sibling(&self) -> Option<NodeRef<'_>> {
        let parent_id = self.parent_id()?;
        let children = self.inner.get_children_list(parent_id)?;

        for i in 0..children.len() {
            let child = children.get(i)?;
            let child_id = child
                .into_value()
                .ok()
                .and_then(|v| v.into_string().ok())
                .and_then(|s| NodeId::from_string(&s))?;

            if self.node_id == child_id {
                let next_index = i.checked_add(1)?;
                let next_child = children.get(next_index)?;
                let next_id = next_child
                    .into_value()
                    .ok()
                    .and_then(|v| v.into_string().ok())
                    .and_then(|s| NodeId::from_string(&s))?;
                return Self::new(self.inner, next_id);
            }
        }

        None
    }

    pub fn child(&self, index: usize) -> Option<NodeRef<'_>> {
        let children = self.inner.get_children_list(self.node_id)?;
        let node_id = NodeId::from_string(&children.get(index)?.as_value()?.as_string()?)?;
        Some(Self::new_unchecked(self.inner, node_id))
    }

    pub fn first_child(&self) -> Option<NodeRef<'_>> {
        self.child(0)
    }

    pub fn last_child(&self) -> Option<NodeRef<'_>> {
        let children = self.inner.get_children_list(self.node_id)?;
        let len = children.len();
        self.child(len.checked_sub(1)?)
    }

    pub fn node(&self) -> &Node {
        self.node.get_or_init(|| {
            let map = self
                .inner
                .get_node_map(self.node_id)
                .expect("Node map not found");
            Node::decode(&map).expect("Failed to decode node")
        })
    }

    pub fn children(&self) -> NodeRefIter<'_> {
        let mut node_ids = Vec::new();

        if let Some(children) = self.inner.get_children_list(self.node_id) {
            if let loro::LoroValue::List(values) = children.get_value() {
                node_ids.reserve(values.len());
                for i in 0..values.len() {
                    if let Some(loro::LoroValue::String(s)) = values.get(i) {
                        if let Some(node_id) = NodeId::from_string(s) {
                            node_ids.push(node_id);
                        }
                    }
                }
            }
        };

        NodeRefIter {
            inner: self.inner,
            node_ids,
            index: 0,
        }
    }

    pub fn ancestors(&self) -> NodeRefIter<'_> {
        let mut node_ids = vec![self.node_id];

        let mut current_id = self.node_id;
        loop {
            let Some(map) = self.inner.get_node_map(current_id) else {
                break;
            };

            let parent_id = map
                .get("parent")
                .and_then(|v| v.into_value().ok())
                .and_then(|v| v.into_string().ok())
                .and_then(|v| NodeId::from_string(&v));

            if let Some(parent_id) = parent_id {
                node_ids.push(parent_id);
                current_id = parent_id;
            } else {
                break;
            }
        }

        NodeRefIter {
            inner: self.inner,
            node_ids,
            index: 0,
        }
    }

    pub fn depth(&self) -> usize {
        self.ancestors().count() - 1
    }

    pub fn ancestor(&self, depth: usize) -> Option<NodeRef<'_>> {
        self.ancestors().nth(self.depth().checked_sub(depth)?)
    }

    pub fn path(&self) -> Vec<usize> {
        let mut node_ids = vec![self.node_id];

        let mut current_id = self.node_id;
        loop {
            let Some(map) = self.inner.get_node_map(current_id) else {
                break;
            };

            let parent_id = map
                .get("parent")
                .and_then(|v| v.into_value().ok())
                .and_then(|v| v.into_string().ok())
                .and_then(|v| NodeId::from_string(&v));

            if let Some(parent_id) = parent_id {
                node_ids.push(parent_id);
                current_id = parent_id;
            } else {
                break;
            }
        }

        let mut path = Vec::new();

        for i in (0..node_ids.len() - 1).rev() {
            let node_id = node_ids[i];
            let parent_id = node_ids[i + 1];

            if let Some(children) = self.inner.get_children_list(parent_id) {
                for j in 0..children.len() {
                    let found = children
                        .get(j)
                        .and_then(|child| child.into_value().ok())
                        .and_then(|value| value.into_string().ok())
                        .and_then(|child_id| NodeId::from_string(&child_id))
                        .map(|child_id_str| node_id == child_id_str)
                        .unwrap_or(false);

                    if found {
                        path.push(j);
                        break;
                    }
                }
            }
        }

        path
    }

    pub fn spec(&self) -> &NodeSpec {
        self.inner.schema.node_spec(self.node_type())
    }

    pub fn is_inline(&self) -> bool {
        self.spec().inline
    }

    pub fn is_block(&self) -> bool {
        !self.spec().inline
    }

    pub fn schema(&self) -> &Schema {
        &self.inner.schema
    }

    pub fn as_mut(&self) -> NodeMut<'_> {
        NodeMut::from_node_ref(self.inner, self)
    }
}

pub struct NodeRefIter<'a> {
    inner: &'a DocInner,
    node_ids: Vec<NodeId>,
    index: usize,
}

impl<'a> Iterator for NodeRefIter<'a> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let node_id = self.node_ids.get(self.index)?;
        self.index += 1;
        Some(NodeRef::new_unchecked(self.inner, *node_id))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.node_ids.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for NodeRefIter<'a> {}
