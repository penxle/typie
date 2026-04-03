use editor_model::NodeId;
use hashbrown::HashMap;
use std::sync::Arc;

use crate::measure::MeasuredNode;

#[derive(Debug, Default)]
pub struct MeasureCache {
    entries: HashMap<NodeId, Arc<MeasuredNode>>,
}

impl MeasureCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, id: NodeId) -> Option<&Arc<MeasuredNode>> {
        self.entries.get(&id)
    }

    pub fn insert(&mut self, id: NodeId, node: Arc<MeasuredNode>) {
        self.entries.insert(id, node);
    }

    pub fn invalidate(&mut self, id: NodeId) -> bool {
        self.entries.remove(&id).is_some()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::*;

    fn dummy_node() -> Arc<MeasuredNode> {
        Arc::new(MeasuredNode {
            width: 100.0,
            height: 20.0,
            content: MeasuredContent::Spacing(0.0),
        })
    }

    #[test]
    fn insert_and_get() {
        let mut cache = MeasureCache::new();
        let id = NodeId::new();
        cache.insert(id, dummy_node());
        assert!(cache.get(id).is_some());
    }

    #[test]
    fn invalidate_removes() {
        let mut cache = MeasureCache::new();
        let id = NodeId::new();
        cache.insert(id, dummy_node());
        assert!(cache.invalidate(id));
        assert!(cache.get(id).is_none());
    }

    #[test]
    fn clear_removes_all() {
        let mut cache = MeasureCache::new();
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        cache.insert(id1, dummy_node());
        cache.insert(id2, dummy_node());
        cache.clear();
        assert!(cache.get(id1).is_none());
        assert!(cache.get(id2).is_none());
    }
}
