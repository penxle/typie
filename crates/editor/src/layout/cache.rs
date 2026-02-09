use crate::layout::LayoutNode;
use crate::model::NodeId;
use rustc_hash::FxHashMap;
use std::rc::Rc;

pub struct LayoutCache {
    cache: FxHashMap<NodeId, Rc<LayoutNode>>,
    prev: FxHashMap<NodeId, Rc<LayoutNode>>,
}

impl LayoutCache {
    pub fn new() -> Self {
        Self {
            cache: FxHashMap::default(),
            prev: FxHashMap::default(),
        }
    }

    pub fn get(&self, node_id: NodeId) -> Option<Rc<LayoutNode>> {
        self.cache.get(&node_id).cloned()
    }

    pub fn take_prev(&mut self, node_id: NodeId) -> Option<Rc<LayoutNode>> {
        self.prev.remove(&node_id)
    }

    pub fn insert(&mut self, node_id: NodeId, layout: Rc<LayoutNode>) {
        self.cache.insert(node_id, layout);
    }

    pub fn invalidate(&mut self, node_id: NodeId) {
        if let Some(old) = self.cache.remove(&node_id) {
            self.prev.insert(node_id, old);
        }
    }

    pub fn invalidate_with_ancestors(
        &mut self,
        node_id: NodeId,
        ancestors: impl Iterator<Item = NodeId>,
    ) {
        if let Some(old) = self.cache.remove(&node_id) {
            self.prev.insert(node_id, old);
        }
        for ancestor_id in ancestors {
            if let Some(old) = self.cache.remove(&ancestor_id) {
                self.prev.insert(ancestor_id, old);
            }
        }
    }

    pub fn invalidate_with_descendants(
        &mut self,
        node_id: NodeId,
        descendants: impl Iterator<Item = NodeId>,
    ) {
        if let Some(old) = self.cache.remove(&node_id) {
            self.prev.insert(node_id, old);
        }
        for descendant_id in descendants {
            if let Some(old) = self.cache.remove(&descendant_id) {
                self.prev.insert(descendant_id, old);
            }
        }
    }

    pub fn invalidate_all(&mut self) {
        self.prev.clear();
        std::mem::swap(&mut self.cache, &mut self.prev);
    }

    pub fn clear_prev(&mut self) {
        self.prev.clear();
    }
}

impl Default for LayoutCache {
    fn default() -> Self {
        Self::new()
    }
}
