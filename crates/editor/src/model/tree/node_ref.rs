use crate::model::tree::{DocInner, NodeMut};
use crate::model::*;
use crate::schema::{NodeSpec, Schema};
use std::cell::OnceCell;
use std::rc::Rc;

#[derive(Debug)]
pub struct NodeRef<'a> {
    inner: &'a DocInner,
    node_id: NodeId,
    parent_id: OnceCell<Option<NodeId>>,
    node: OnceCell<Node>,
}

impl<'a> NodeRef<'a> {
    pub fn new(inner: &'a DocInner, node_id: NodeId) -> Option<Self> {
        if !inner.is_reachable(node_id) {
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
        let parent_id = self.parent_id()?;
        let children = self.inner.get_children_ids_cached(parent_id);
        children.iter().position(|&id| id == self.node_id)
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
        let children = self.inner.get_children_ids_cached(parent_id);
        let idx = children.iter().position(|&id| id == self.node_id)?;
        let prev_id = *children.get(idx.checked_sub(1)?)?;
        Self::new(self.inner, prev_id)
    }

    pub fn next_sibling(&self) -> Option<NodeRef<'_>> {
        let parent_id = self.parent_id()?;
        let children = self.inner.get_children_ids_cached(parent_id);
        let idx = children.iter().position(|&id| id == self.node_id)?;
        let next_id = *children.get(idx + 1)?;
        Self::new(self.inner, next_id)
    }

    pub fn child(&self, index: usize) -> Option<NodeRef<'_>> {
        let children = self.inner.get_children_ids_cached(self.node_id);
        let &child_id = children.get(index)?;
        Some(Self::new_unchecked(self.inner, child_id))
    }

    pub fn first_child(&self) -> Option<NodeRef<'_>> {
        self.child(0)
    }

    pub fn last_child(&self) -> Option<NodeRef<'_>> {
        let children = self.inner.get_children_ids_cached(self.node_id);
        let &child_id = children.last()?;
        Some(Self::new_unchecked(self.inner, child_id))
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
        let node_ids = self.inner.get_children_ids_cached(self.node_id);

        NodeRefIter {
            inner: self.inner,
            node_ids: NodeRefIterIds::Shared(node_ids),
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
            node_ids: NodeRefIterIds::Owned(node_ids),
            index: 0,
        }
    }

    pub fn descendants(&self) -> NodeRefIter<'_> {
        let mut node_ids = Vec::new();
        let mut queue = vec![self.node_id];

        while let Some(current_id) = queue.pop() {
            let children = self.inner.get_children_ids_cached(current_id);
            for &child_id in children.iter() {
                node_ids.push(child_id);
                queue.push(child_id);
            }
        }

        NodeRefIter {
            inner: self.inner,
            node_ids: NodeRefIterIds::Owned(node_ids),
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

            let children = self.inner.get_children_ids_cached(parent_id);
            if let Some(idx) = children.iter().position(|&id| id == node_id) {
                path.push(idx);
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

enum NodeRefIterIds {
    Shared(Rc<Vec<NodeId>>),
    Owned(Vec<NodeId>),
}

impl NodeRefIterIds {
    fn get(&self, index: usize) -> Option<&NodeId> {
        match self {
            Self::Shared(rc) => rc.get(index),
            Self::Owned(vec) => vec.get(index),
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Shared(rc) => rc.len(),
            Self::Owned(vec) => vec.len(),
        }
    }
}

pub struct NodeRefIter<'a> {
    inner: &'a DocInner,
    node_ids: NodeRefIterIds,
    index: usize,
}

impl<'a> Iterator for NodeRefIter<'a> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let &node_id = self.node_ids.get(self.index)?;
        self.index += 1;
        Some(NodeRef::new_unchecked(self.inner, node_id))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.node_ids.len() - self.index;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for NodeRefIter<'a> {}
