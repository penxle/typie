use editor_crdt::Dot;
use hashbrown::HashMap;
use std::sync::Arc;

use crate::measure::types::MeasuredNode;

#[derive(Debug, Default)]
pub(crate) struct MeasureCache {
    entries: HashMap<Dot, (f32, Arc<MeasuredNode>)>,
}

impl MeasureCache {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn get(&self, id: Dot, width: f32) -> Option<&Arc<MeasuredNode>> {
        match self.entries.get(&id) {
            Some((w, node)) if *w == width => Some(node),
            _ => None,
        }
    }
    pub(crate) fn insert(&mut self, id: Dot, width: f32, node: Arc<MeasuredNode>) {
        self.entries.insert(id, (width, node));
    }
    pub(crate) fn invalidate(&mut self, id: Dot) -> bool {
        self.entries.remove(&id).is_some()
    }
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::types::{MeasuredContent, MeasuredNode};

    fn dummy() -> Arc<MeasuredNode> {
        Arc::new(MeasuredNode {
            width: 100.0,
            height: 20.0,
            content: MeasuredContent::Spacing(0.0),
        })
    }

    #[test]
    fn get_requires_width_match() {
        let mut c = MeasureCache::new();
        let id = Dot::new(1, 1);
        c.insert(id, 300.0, dummy());
        assert!(c.get(id, 300.0).is_some());
        assert!(
            c.get(id, 250.0).is_none(),
            "width mismatch must be a miss (multi-pass safety)"
        );
        c.insert(id, 250.0, dummy());
        assert!(c.get(id, 250.0).is_some());
        assert!(c.invalidate(id));
        assert!(c.get(id, 250.0).is_none());
        assert!(!c.invalidate(id));
    }
}
