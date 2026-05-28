use editor_state::StableSelection;
use editor_view::PageRect;
use hashbrown::{HashMap, HashSet};

pub type TrackedRangeId = String;

#[derive(Clone, Debug, PartialEq)]
pub struct TrackedRange {
    pub id: TrackedRangeId,
    pub group: String,
    pub selection: StableSelection,
    pub metadata: String,
    pub explicitly_invalid: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackedRangeHit {
    pub id: TrackedRangeId,
    pub group: String,
    /// Range rects on the queried `page_idx` only (filtered by `Editor::tracked_ranges_at`).
    pub rects: Vec<PageRect>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TrackedRangeRegistry {
    by_id: HashMap<TrackedRangeId, TrackedRange>,
    by_group: HashMap<String, HashSet<TrackedRangeId>>,
}

impl TrackedRangeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, range: TrackedRange) -> Option<TrackedRange> {
        let id = range.id.clone();
        let new_group = range.group.clone();
        let prev = self.by_id.insert(id.clone(), range);
        if let Some(prev_range) = &prev
            && prev_range.group != new_group
            && let Some(set) = self.by_group.get_mut(&prev_range.group)
        {
            set.remove(&id);
            if set.is_empty() {
                self.by_group.remove(&prev_range.group);
            }
        }
        self.by_group.entry(new_group).or_default().insert(id);
        prev
    }

    pub fn remove(&mut self, id: &str) -> Option<TrackedRange> {
        let prev = self.by_id.remove(id)?;
        if let Some(set) = self.by_group.get_mut(&prev.group) {
            set.remove(id);
            if set.is_empty() {
                self.by_group.remove(&prev.group);
            }
        }
        Some(prev)
    }

    pub fn clear_group(&mut self, group: &str) -> Vec<TrackedRange> {
        let Some(ids) = self.by_group.remove(group) else {
            return Vec::new();
        };
        ids.into_iter()
            .filter_map(|id| self.by_id.remove(&id))
            .collect()
    }

    pub fn invalidate(&mut self, id: &str) -> bool {
        match self.by_id.get_mut(id) {
            Some(range) if !range.explicitly_invalid => {
                range.explicitly_invalid = true;
                true
            }
            _ => false,
        }
    }

    pub fn get(&self, id: &str) -> Option<&TrackedRange> {
        self.by_id.get(id)
    }

    pub fn contains(&self, id: &str) -> bool {
        self.by_id.contains_key(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &TrackedRange> {
        self.by_id.values()
    }

    pub fn iter_group<'a>(&'a self, group: &'a str) -> impl Iterator<Item = &'a TrackedRange> + 'a {
        self.by_group
            .get(group)
            .into_iter()
            .flat_map(move |ids| ids.iter().filter_map(move |id| self.by_id.get(id)))
    }

    pub fn group_size(&self, group: &str) -> usize {
        self.by_group.get(group).map(|s| s.len()).unwrap_or(0)
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn sorted_ids(&self) -> Vec<TrackedRangeId> {
        let mut ids: Vec<_> = self.by_id.keys().cloned().collect();
        ids.sort();
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;

    fn make_range(id: &str, group: &str) -> TrackedRange {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let sel = s.selection.unwrap();
        TrackedRange {
            id: id.into(),
            group: group.into(),
            selection: StableSelection::freeze(&sel, &s.doc),
            metadata: String::new(),
            explicitly_invalid: false,
        }
    }

    #[test]
    fn add_inserts_into_both_indices() {
        let mut reg = TrackedRangeRegistry::new();
        reg.add(make_range("a", "g1"));
        assert!(reg.contains("a"));
        assert_eq!(reg.group_size("g1"), 1);
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn add_same_id_replaces() {
        let mut reg = TrackedRangeRegistry::new();
        reg.add(make_range("a", "g1"));
        let prev = reg.add(make_range("a", "g1"));
        assert!(prev.is_some());
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn add_same_id_different_group_migrates() {
        let mut reg = TrackedRangeRegistry::new();
        reg.add(make_range("a", "g1"));
        reg.add(make_range("a", "g2"));
        assert_eq!(reg.group_size("g1"), 0);
        assert_eq!(reg.group_size("g2"), 1);
    }

    #[test]
    fn remove_clears_from_both_indices() {
        let mut reg = TrackedRangeRegistry::new();
        reg.add(make_range("a", "g1"));
        assert!(reg.remove("a").is_some());
        assert!(!reg.contains("a"));
        assert_eq!(reg.group_size("g1"), 0);
    }

    #[test]
    fn remove_nonexistent_returns_none() {
        let mut reg = TrackedRangeRegistry::new();
        assert!(reg.remove("x").is_none());
    }

    #[test]
    fn clear_group_removes_only_targeted_group() {
        let mut reg = TrackedRangeRegistry::new();
        reg.add(make_range("a", "g1"));
        reg.add(make_range("b", "g1"));
        reg.add(make_range("c", "g2"));
        let cleared = reg.clear_group("g1");
        assert_eq!(cleared.len(), 2);
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.group_size("g2"), 1);
    }

    #[test]
    fn clear_empty_group_returns_empty() {
        let mut reg = TrackedRangeRegistry::new();
        let cleared = reg.clear_group("nothing");
        assert!(cleared.is_empty());
    }

    #[test]
    fn invalidate_flips_flag_once() {
        let mut reg = TrackedRangeRegistry::new();
        reg.add(make_range("a", "g1"));
        assert!(reg.invalidate("a"));
        assert!(!reg.invalidate("a"));
        assert!(reg.get("a").unwrap().explicitly_invalid);
    }

    #[test]
    fn invalidate_unknown_id_returns_false() {
        let mut reg = TrackedRangeRegistry::new();
        assert!(!reg.invalidate("x"));
    }

    #[test]
    fn iter_group_returns_only_members() {
        let mut reg = TrackedRangeRegistry::new();
        reg.add(make_range("a", "g1"));
        reg.add(make_range("b", "g1"));
        reg.add(make_range("c", "g2"));
        let g1: Vec<_> = reg.iter_group("g1").map(|r| r.id.clone()).collect();
        assert_eq!(g1.len(), 2);
        assert!(g1.contains(&"a".to_string()));
        assert!(g1.contains(&"b".to_string()));
    }

    #[test]
    fn sorted_ids_returns_stable_order() {
        let mut reg = TrackedRangeRegistry::new();
        reg.add(make_range("c", "g"));
        reg.add(make_range("a", "g"));
        reg.add(make_range("b", "g"));
        let ids = reg.sorted_ids();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }
}
