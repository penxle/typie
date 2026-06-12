use editor_model::Doc;
use editor_state::{Selection, StableSelection};
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

    pub fn set_group(&mut self, id: &str, group: String) -> bool {
        let Some(range) = self.by_id.get_mut(id) else {
            return false;
        };
        if range.group == group {
            return false;
        }

        let old_group = std::mem::replace(&mut range.group, group.clone());
        if let Some(set) = self.by_group.get_mut(&old_group) {
            set.remove(id);
            if set.is_empty() {
                self.by_group.remove(&old_group);
            }
        }
        self.by_group
            .entry(group)
            .or_default()
            .insert(id.to_string());
        true
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

impl TrackedRange {
    pub fn from_selection(
        id: TrackedRangeId,
        group: String,
        selection: Selection,
        metadata: String,
        doc: &Doc,
    ) -> Self {
        let selection = StableSelection::freeze_covered_range(&selection, doc)
            .unwrap_or_else(|| StableSelection::freeze(&selection, doc));
        Self {
            id,
            group,
            selection,
            metadata,
            explicitly_invalid: false,
        }
    }

    pub fn from_stable_selection(
        id: TrackedRangeId,
        group: String,
        selection: StableSelection,
        metadata: String,
        doc: &Doc,
    ) -> Self {
        // FFI/persisted callers can pass a StableSelection frozen with user
        // selection semantics. If it still locates, lower it again with tracked
        // range boundary policy so typing at the boundary stays outside.
        let selection = selection
            .locate(doc)
            .and_then(|sel| StableSelection::freeze_covered_range(&sel, doc))
            .unwrap_or(selection);
        Self {
            id,
            group,
            selection,
            metadata,
            explicitly_invalid: false,
        }
    }

    pub fn locate(&self, doc: &Doc) -> Option<Selection> {
        self.selection.locate(doc)
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
        TrackedRange::from_selection(id.into(), group.into(), sel, String::new(), &s.doc)
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
    fn locate_follows_moved_entry_dot() {
        let (state, t1, t2) = state! {
            doc {
                root {
                    paragraph { t1: text("a") }
                    paragraph { t2: text("") }
                }
            }
            selection: (t1, 0) -> (t1, 1)
        };
        let range = TrackedRange::from_selection(
            "a".into(),
            "g1".into(),
            *state.selection.as_ref().unwrap(),
            String::new(),
            &state.doc,
        );
        let selected_entry = state
            .doc
            .text_view(t1)
            .unwrap()
            .visible_entries()
            .next()
            .unwrap()
            .0;
        let (state, _) = state
            .apply(editor_model::DocOp::MoveText {
                entry: selected_entry,
                to_node_id: t2,
                after: None,
            })
            .unwrap();

        let located = range.locate(&state.doc).expect("range locates after move");
        assert_eq!(located.anchor.node_id, t2);
        assert_eq!(located.head.node_id, t2);
        assert_eq!(state.doc.text_view(t2).unwrap().text(), "a");
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
