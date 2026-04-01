use editor_model::NodeId;
use hashbrown::HashMap;
use std::sync::Arc;

use crate::measure::Measurement;

#[derive(Debug, Default)]
pub struct LayoutCache {
    entries: HashMap<NodeId, Arc<Measurement>>,
}

impl LayoutCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, id: &NodeId) -> Option<&Arc<Measurement>> {
        self.entries.get(id)
    }

    pub fn insert(&mut self, id: NodeId, m: Arc<Measurement>) {
        self.entries.insert(id, m);
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
    use editor_common::{Alignment, Size};

    use super::*;
    use crate::measure::MeasuredContent;

    fn dummy() -> Arc<Measurement> {
        Arc::new(Measurement {
            size: Size {
                width: 100.0,
                height: 20.0,
            },
            gap_after: 0.0,
            alignment: Alignment::Start,
            content: MeasuredContent::Atom {
                parent_id: NodeId::ROOT,
                index: 0,
            },
        })
    }

    #[test]
    fn insert_and_get() {
        let mut cache = LayoutCache::new();

        let id = NodeId::new();
        cache.insert(id, dummy());

        assert!(cache.get(&id).is_some());
    }

    #[test]
    fn invalidate_removes() {
        let mut cache = LayoutCache::new();

        let id = NodeId::new();
        cache.insert(id, dummy());
        cache.invalidate(id);

        assert!(cache.get(&id).is_none());
    }

    #[test]
    fn clear_removes_all() {
        let mut cache = LayoutCache::new();

        let id1 = NodeId::new();
        let id2 = NodeId::new();
        cache.insert(id1, dummy());
        cache.insert(id2, dummy());
        cache.clear();

        assert!(cache.get(&id1).is_none());
        assert!(cache.get(&id2).is_none());
    }
}
