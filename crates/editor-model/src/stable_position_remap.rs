use editor_crdt::{Dot, EntryDot};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StableEntryResolution {
    Live(EntryDot),
    Deleted(EntryDot),
    Cycle(EntryDot),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StablePositionReplacement {
    pub(crate) target: EntryDot,
    pub(crate) op_id: Dot,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct StablePositionRemapStore {
    replacements: imbl::HashMap<EntryDot, StablePositionReplacement>,
}

impl StablePositionRemapStore {
    pub(crate) fn record(&mut self, from: EntryDot, to: EntryDot, op_id: Dot) {
        match self.replacements.get(&from) {
            Some(existing) if existing.op_id > op_id => {}
            _ => {
                self.replacements
                    .insert(from, StablePositionReplacement { target: to, op_id });
            }
        }
    }

    pub(crate) fn replacement_for(&self, from: EntryDot) -> Option<EntryDot> {
        self.replacements
            .get(&from)
            .map(|replacement| replacement.target)
    }
}
