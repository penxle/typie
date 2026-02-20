use crate::model::attr::Attr;
use crate::model::tree::DocInner;
use crate::model::tree::node_ref::{CASCADE_ATTRS_KEY, REMARKS_KEY};
use crate::model::*;
use anyhow::{Context, Result};
use loro::LoroMap;

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
        anyhow::ensure!(
            !self.inner.is_reachable(node_id),
            "Node {} already exists in the document",
            node_id
        );

        let child_map = self
            .inner
            .create_node_map(node_id)
            .context("Failed to create child node map")?;

        node.encode(&child_map)?;
        child_map.insert("parent", self.node_ref.node_id().to_string())?;

        let children = self
            .inner
            .get_or_create_children_list(self.node_ref.node_id())
            .context("Failed to get or create children list")?;

        children.insert(index, node_id.to_string())?;
        self.inner
            .invalidate_children_cache_for(self.node_ref.node_id());
        self.inner.mark_reachable(node_id);

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

        anyhow::ensure!(
            node_id != parent_id && !self.inner.is_ancestor_of(node_id, parent_id),
            "Cycle detected: node {} cannot be moved under {}",
            node_id,
            parent_id
        );

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

        for i in 0..new_children.len() {
            let child = new_children.get(i).context("Failed to get child")?;
            if node_id
                == child
                    .into_value()
                    .ok()
                    .and_then(|v| v.into_string().ok())
                    .and_then(|s| NodeId::from_string(&s))
                    .context("Failed to convert child ID to NodeId")?
            {
                anyhow::bail!(
                    "Duplicate child: node {} is already a child of {}",
                    node_id,
                    parent_id
                );
            }
        }

        new_children.insert(index, node_id.to_string())?;
        self.inner.invalidate_children_cache_for(parent_id);

        let map = self
            .inner
            .get_node_map(node_id)
            .context("Failed to get node map")?;

        map.insert("parent", parent_id.to_string())?;

        Ok(())
    }

    pub fn set_cascade_attrs(&self, attrs: &[Attr]) -> Result<()> {
        anyhow::ensure!(
            !self.node_ref.is_inline(),
            "Cannot set cascade_attrs on inline node"
        );
        let map = self
            .inner
            .get_node_map(self.node_ref.node_id())
            .context("Node map not found")?;
        let attrs_map = map
            .get_or_create_container(CASCADE_ATTRS_KEY, LoroMap::new())
            .context("Failed to create cascade_attrs map")?;

        let deep = attrs_map.get_deep_value();
        if let Ok(entries) = deep.into_map() {
            for key in entries.keys() {
                let _ = attrs_map.delete(key);
            }
        }

        for attr in attrs {
            attrs_map.insert(attr.key(), attr.to_loro_value())?;
        }

        Ok(())
    }

    pub fn add_remark(&self, remark: &Remark) -> Result<()> {
        anyhow::ensure!(
            !self.node_ref.is_inline(),
            "Cannot add remark on inline node"
        );
        let map = self
            .inner
            .get_node_map(self.node_ref.node_id())
            .context("Node map not found")?;
        let remarks_map = map
            .get_or_create_container(REMARKS_KEY, LoroMap::new())
            .context("Failed to create remarks map")?;
        let entry = remarks_map
            .insert_container(&remark.id.to_string(), LoroMap::new())
            .context("Failed to create remark entry")?;
        entry.insert("user_id", remark.user_id.as_str())?;
        entry.insert("text", remark.text.as_str())?;
        entry.insert("created_at", remark.created_at)?;
        Ok(())
    }

    pub fn update_remark(&self, remark_id: RemarkId, text: &str) -> Result<()> {
        let map = self
            .inner
            .get_node_map(self.node_ref.node_id())
            .context("Node map not found")?;
        let remarks_map = map
            .get(REMARKS_KEY)
            .and_then(|v| v.into_container().ok())
            .and_then(|c| c.into_map().ok())
            .context("Remarks map not found")?;
        let entry = remarks_map
            .get(&remark_id.to_string())
            .and_then(|v| v.into_container().ok())
            .and_then(|c| c.into_map().ok())
            .context("Remark not found")?;
        entry.insert("text", text)?;
        Ok(())
    }

    pub fn remove_remark(&self, remark_id: RemarkId) -> Result<()> {
        let map = self
            .inner
            .get_node_map(self.node_ref.node_id())
            .context("Node map not found")?;
        let remarks_map = map
            .get(REMARKS_KEY)
            .and_then(|v| v.into_container().ok())
            .and_then(|c| c.into_map().ok())
            .context("Remarks map not found")?;
        remarks_map.delete(&remark_id.to_string())?;
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
        self.inner.mark_unreachable_subtree(self.node_ref.node_id());

        Ok(())
    }
}
