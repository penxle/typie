use editor_crdt::{EntryDot, PlacementId};

use crate::id::NodeId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TextEntryMeta {
    pub(crate) ch: char,
    pub(crate) birth_node: NodeId,
    pub(crate) initial_placement: PlacementId,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct TextEntryStore {
    entry_meta: imbl::HashMap<EntryDot, TextEntryMeta>,
    deleted_entries: imbl::HashMap<EntryDot, imbl::OrdSet<editor_crdt::Dot>>,
}

impl TextEntryStore {
    pub(crate) fn register_insert(
        &mut self,
        entry_dot: EntryDot,
        birth_node: NodeId,
        initial_placement: PlacementId,
        ch: char,
    ) {
        let meta = TextEntryMeta {
            ch,
            birth_node,
            initial_placement,
        };
        if let Some(existing) = self.entry_meta.get(&entry_dot) {
            debug_assert_eq!(existing, &meta);
            return;
        }
        self.entry_meta.insert(entry_dot, meta);
    }

    pub(crate) fn char_for(&self, entry_dot: EntryDot) -> Option<char> {
        self.entry_meta.get(&entry_dot).map(|meta| meta.ch)
    }

    pub(crate) fn birth_location_for(&self, entry_dot: EntryDot) -> Option<(NodeId, PlacementId)> {
        self.entry_meta
            .get(&entry_dot)
            .map(|meta| (meta.birth_node, meta.initial_placement))
    }

    pub(crate) fn mark_deleted(&mut self, entry_dot: EntryDot, remove_dot: editor_crdt::Dot) {
        self.deleted_entries
            .entry(entry_dot)
            .or_default()
            .insert(remove_dot);
    }

    pub(crate) fn is_deleted(&self, entry_dot: EntryDot) -> bool {
        self.deleted_entries
            .get(&entry_dot)
            .is_some_and(|dots| !dots.is_empty())
    }

    pub(crate) fn delete_horizon_for(&self, entry_dot: EntryDot) -> Option<editor_crdt::Dot> {
        self.deleted_entries
            .get(&entry_dot)
            .and_then(|dots| dots.iter().next().copied())
    }

    #[cfg(test)]
    pub(crate) fn delete_dots_for(&self, entry_dot: EntryDot) -> Vec<editor_crdt::Dot> {
        self.deleted_entries
            .get(&entry_dot)
            .map(|dots| dots.iter().copied().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::{Dot, EntryDot, PlacementId};

    use crate::NodeId;

    use super::TextEntryStore;

    #[test]
    fn deletion_tracks_remove_op_dots() {
        let entry = EntryDot(Dot::new(1, 0));
        let remove_a = Dot::new(2, 0);
        let remove_b = Dot::new(3, 0);
        let mut store = TextEntryStore::default();
        store.register_insert(entry, NodeId::ROOT, PlacementId(entry.0), 'x');

        store.mark_deleted(entry, remove_a);
        store.mark_deleted(entry, remove_a);
        store.mark_deleted(entry, remove_b);

        assert!(store.is_deleted(entry));
        assert_eq!(store.delete_dots_for(entry), vec![remove_a, remove_b]);
    }
}
