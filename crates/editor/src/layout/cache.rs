use crate::layout::LayoutNode;
use crate::model::NodeId;
use rustc_hash::FxHashMap;
use std::rc::Rc;

pub struct LayoutCache {
    cache: FxHashMap<NodeId, Rc<LayoutNode>>,
}

impl LayoutCache {
    pub fn new() -> Self {
        Self {
            cache: FxHashMap::default(),
        }
    }

    pub fn get(&self, node_id: NodeId) -> Option<Rc<LayoutNode>> {
        self.cache.get(&node_id).cloned()
    }

    pub fn insert(&mut self, node_id: NodeId, layout: Rc<LayoutNode>) {
        self.cache.insert(node_id, layout);
    }

    pub fn invalidate(&mut self, node_id: NodeId) {
        self.cache.remove(&node_id);
    }

    pub fn invalidate_with_ancestors(
        &mut self,
        node_id: NodeId,
        ancestors: impl Iterator<Item = NodeId>,
    ) {
        self.cache.remove(&node_id);
        for ancestor_id in ancestors {
            self.cache.remove(&ancestor_id);
        }
    }

    pub fn invalidate_with_descendants(
        &mut self,
        node_id: NodeId,
        descendants: impl Iterator<Item = NodeId>,
    ) {
        self.cache.remove(&node_id);
        for descendant_id in descendants {
            self.cache.remove(&descendant_id);
        }
    }

    pub fn invalidate_all(&mut self) {
        self.cache.clear();
    }
}

impl Default for LayoutCache {
    fn default() -> Self {
        Self::new()
    }
}
