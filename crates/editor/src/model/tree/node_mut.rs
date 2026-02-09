use crate::model::tree::DocInner;
use crate::model::*;
use anyhow::{Context, Result};

#[derive(Debug)]
pub struct NodeMut<'a> {
    inner: &'a DocInner,
    node_ref: &'a NodeRef<'a>,
}

impl<'a> NodeMut<'a> {
    pub fn from_node_ref(inner: &'a DocInner, node_ref: &'a NodeRef<'a>) -> Self {
        Self { inner, node_ref }
    }

    pub fn insert_child(&mut self, index: usize, node: Node) -> Result<NodeId> {
        self.insert_child_with_id(index, NodeId::new(), node)
    }

    pub fn insert_child_with_id(
        &mut self,
        index: usize,
        node_id: NodeId,
        mut node: Node,
    ) -> Result<NodeId> {
        let child_map = self
            .inner
            .get_or_create_node_map(node_id)
            .context("Failed to get or create child node map")?;

        node.encode(&child_map)?;
        child_map.insert("parent", self.node_ref.node_id().to_string())?;

        let children = self
            .inner
            .get_or_create_children_list(self.node_ref.node_id())
            .context("Failed to get or create children list")?;

        children.insert(index, node_id.to_string())?;
        self.inner
            .invalidate_children_cache_for(self.node_ref.node_id());

        Ok(node_id)
    }

    pub fn update<F>(&mut self, f: F) -> Result<()>
    where
        F: FnOnce(&mut Node),
    {
        let map = self
            .inner
            .get_node_map(self.node_ref.node_id())
            .context("Failed to get node map")?;

        let mut node = Node::decode(&map)?;
        f(&mut node);
        node.encode(&map)?;

        Ok(())
    }

    pub fn move_to(&mut self, parent_id: NodeId, index: usize) -> Result<()> {
        let current_parent_id = self.node_ref.parent_id();
        let node_id = self.node_ref.node_id();

        if let Some(current_parent_id) = current_parent_id {
            let children = self
                .inner
                .get_children_list(current_parent_id)
                .context("Failed to get current parent's children list")?;

            for i in 0..children.len() {
                let child = children.get(i).context("Failed to get child")?;
                if node_id
                    == child
                        .into_value()
                        .ok()
                        .and_then(|v| v.into_string().ok())
                        .and_then(|s| NodeId::from_string(&s))
                        .context("Failed to convert child ID to NodeId")?
                {
                    children.delete(i, 1)?;
                    break;
                }
            }
            self.inner.invalidate_children_cache_for(current_parent_id);
        }

        let new_children = self
            .inner
            .get_or_create_children_list(parent_id)
            .context("Failed to get or create new parent's children list")?;

        new_children.insert(index, node_id.to_string())?;
        self.inner.invalidate_children_cache_for(parent_id);

        let map = self
            .inner
            .get_node_map(node_id)
            .context("Failed to get node map")?;

        map.insert("parent", parent_id.to_string())?;

        Ok(())
    }

    pub fn delete(&mut self) -> Result<()> {
        let parent_id = self.node_ref.parent_id().context("Node has no parent")?;
        let children = self
            .inner
            .get_children_list(parent_id)
            .context("Failed to get children list")?;

        for i in 0..children.len() {
            let child = children.get(i).context("Failed to get child")?;
            if self.node_ref.node_id()
                == child
                    .into_value()
                    .ok()
                    .and_then(|v| v.into_string().ok())
                    .and_then(|s| NodeId::from_string(&s))
                    .context("Failed to convert child ID to NodeId")?
            {
                children.delete(i, 1)?;
                break;
            }
        }

        self.inner.invalidate_children_cache_for(parent_id);

        let nodes = self.inner.loro.get_map("nodes");
        nodes.delete(&self.node_ref.node_id().to_string())?;

        Ok(())
    }
}
