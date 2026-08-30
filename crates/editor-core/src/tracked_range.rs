use editor_crdt::Dot;
use editor_state::{Selection, StableResolveCtx, StableSelection, State, blocks_in_range};
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
    /// When set, the editor re-verifies this range after document edits and
    /// removes it (reporting `EditorEvent::TrackedRangesStale`) once the text
    /// it covers no longer equals `captured_text`.
    pub invalidate_on_text_change: bool,
    /// Covered text at install time; `None` when the selection did not
    /// resolve at install, which opts the range out of text-change checks.
    pub captured_text: Option<String>,
    /// Blocks intersecting the range at its last successful resolution — the
    /// keys under which a text-sensitive range is indexed for dirty-block
    /// scoped re-verification.
    pub covered_blocks: Vec<Dot>,
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
    sensitive_by_block: HashMap<Dot, HashSet<TrackedRangeId>>,
}

impl TrackedRangeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn unindex_sensitive(&mut self, id: &str, blocks: &[Dot]) {
        for block in blocks {
            if let Some(set) = self.sensitive_by_block.get_mut(block) {
                set.remove(id);
                if set.is_empty() {
                    self.sensitive_by_block.remove(block);
                }
            }
        }
    }

    fn index_sensitive(&mut self, id: &TrackedRangeId, blocks: &[Dot]) {
        for block in blocks {
            self.sensitive_by_block
                .entry(*block)
                .or_default()
                .insert(id.clone());
        }
    }

    fn range_is_verifiable(range: &TrackedRange) -> bool {
        range.invalidate_on_text_change
            && !range.explicitly_invalid
            && range.captured_text.is_some()
    }

    pub fn add(&mut self, range: TrackedRange) -> Option<TrackedRange> {
        let id = range.id.clone();
        let new_group = range.group.clone();
        let sensitive_blocks = if Self::range_is_verifiable(&range) {
            range.covered_blocks.clone()
        } else {
            Default::default()
        };
        let prev = self.by_id.insert(id.clone(), range);
        if let Some(prev_range) = &prev {
            let prev_blocks = prev_range.covered_blocks.clone();
            self.unindex_sensitive(&id, &prev_blocks);
            if prev_range.group != new_group
                && let Some(set) = self.by_group.get_mut(&prev_range.group)
            {
                set.remove(&id);
                if set.is_empty() {
                    self.by_group.remove(&prev_range.group);
                }
            }
        }
        self.index_sensitive(&id, &sensitive_blocks);
        self.by_group.entry(new_group).or_default().insert(id);
        prev
    }

    pub fn remove(&mut self, id: &str) -> Option<TrackedRange> {
        let prev = self.by_id.remove(id)?;
        self.unindex_sensitive(id, &prev.covered_blocks);
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

    pub fn set_selection(
        &mut self,
        id: &str,
        selection: StableSelection,
        covered_blocks: Vec<Dot>,
    ) -> bool {
        let Some(range) = self.by_id.get_mut(id) else {
            return false;
        };
        range.selection = selection;
        let old_blocks = std::mem::replace(&mut range.covered_blocks, covered_blocks);
        let verifiable = Self::range_is_verifiable(range);
        let new_blocks = verifiable.then(|| range.covered_blocks.clone());
        let id = range.id.clone();
        self.unindex_sensitive(&id, &old_blocks);
        if let Some(blocks) = new_blocks {
            self.index_sensitive(&id, &blocks);
        }
        true
    }

    /// Refreshes the block index of a range whose resolved extent moved while
    /// its text stayed intact (dirty-scoped re-verification).
    pub(crate) fn reindex(&mut self, id: &str, covered_blocks: Vec<Dot>) {
        let Some(range) = self.by_id.get_mut(id) else {
            return;
        };
        let old_blocks = std::mem::replace(&mut range.covered_blocks, covered_blocks);
        let verifiable = Self::range_is_verifiable(range);
        let new_blocks = verifiable.then(|| range.covered_blocks.clone());
        let id = range.id.clone();
        self.unindex_sensitive(&id, &old_blocks);
        if let Some(blocks) = new_blocks {
            self.index_sensitive(&id, &blocks);
        }
    }

    pub fn clear_group(&mut self, group: &str) -> Vec<TrackedRange> {
        let Some(ids) = self.by_group.remove(group) else {
            return Vec::new();
        };
        let removed: Vec<TrackedRange> = ids
            .into_iter()
            .filter_map(|id| self.by_id.remove(&id))
            .collect();
        for range in &removed {
            let id = range.id.clone();
            let blocks = range.covered_blocks.clone();
            self.unindex_sensitive(&id, &blocks);
        }
        removed
    }

    pub fn invalidate(&mut self, id: &str) -> bool {
        match self.by_id.get_mut(id) {
            Some(range) if !range.explicitly_invalid => {
                range.explicitly_invalid = true;
                let id = range.id.clone();
                let blocks = range.covered_blocks.clone();
                self.unindex_sensitive(&id, &blocks);
                true
            }
            _ => false,
        }
    }

    pub fn has_text_sensitive(&self) -> bool {
        !self.sensitive_by_block.is_empty()
    }

    pub fn text_sensitive_ids(&self) -> Vec<TrackedRangeId> {
        let mut ids: Vec<TrackedRangeId> = self
            .by_id
            .values()
            .filter(|r| Self::range_is_verifiable(r))
            .map(|r| r.id.clone())
            .collect();
        ids.sort();
        ids
    }

    pub fn text_sensitive_ids_in_blocks<'a>(
        &self,
        blocks: impl Iterator<Item = &'a Dot>,
    ) -> Vec<TrackedRangeId> {
        let mut seen: HashSet<&TrackedRangeId> = HashSet::new();
        for block in blocks {
            if let Some(ids) = self.sensitive_by_block.get(block) {
                seen.extend(ids.iter());
            }
        }
        let mut ids: Vec<TrackedRangeId> = seen.into_iter().cloned().collect();
        ids.sort();
        ids
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

    pub fn iter_group<'a>(&'a self, group: &str) -> impl Iterator<Item = &'a TrackedRange> + 'a {
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
    pub fn new(
        id: TrackedRangeId,
        group: String,
        selection: StableSelection,
        metadata: String,
        invalidate_on_text_change: bool,
        state: &State,
    ) -> Self {
        let mut range = Self {
            id,
            group,
            selection,
            metadata,
            explicitly_invalid: false,
            invalidate_on_text_change,
            captured_text: None,
            covered_blocks: Vec::new(),
        };
        let view = state.view();
        if let Some(resolved) = range.locate(state).and_then(|sel| sel.resolve(&view)) {
            range.captured_text = Some(resolved.collect_text());
            range.covered_blocks = blocks_in_range(&resolved).iter().map(|b| b.id()).collect();
        }
        range
    }

    pub fn locate(&self, state: &State) -> Option<Selection> {
        if self.explicitly_invalid {
            return None;
        }
        let view = state.view();
        let ctx = StableResolveCtx::from_live(&view, state.projected.seq_checkout());
        let sel = self.selection.resolve(&ctx)?;
        let sel = sel.normalize(&view)?;
        sel.resolve(&view)?;
        (!sel.is_collapsed()).then_some(sel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;

    fn make_range(id: &str, group: &str) -> TrackedRange {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let sel = s.selection.unwrap();
        TrackedRange::new(
            id.into(),
            group.into(),
            StableSelection::capture(&sel, &s.view()),
            String::new(),
            false,
            &s,
        )
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
    fn locate_returns_none_when_range_is_explicitly_invalid() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let sel = state.selection.unwrap();
        let mut range = TrackedRange {
            id: "a".into(),
            group: "g1".into(),
            selection: StableSelection::capture(&sel, &state.view()),
            metadata: String::new(),
            explicitly_invalid: false,
            invalidate_on_text_change: false,
            captured_text: None,
            covered_blocks: Vec::new(),
        };

        range.explicitly_invalid = true;

        assert!(range.locate(&state).is_none());
    }

    #[test]
    fn locate_resolves_captured_range() {
        let (state, p1, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let range = TrackedRange {
            id: "a".into(),
            group: "g1".into(),
            selection: StableSelection::capture(state.selection.as_ref().unwrap(), &state.view()),
            metadata: String::new(),
            explicitly_invalid: false,
            invalidate_on_text_change: false,
            captured_text: None,
            covered_blocks: Vec::new(),
        };

        let resolved = range.locate(&state).expect("range locates");
        assert_eq!(resolved.anchor.node, p1);
        assert_eq!(resolved.head.node, p1);
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
