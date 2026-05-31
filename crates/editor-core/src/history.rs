use editor_common::time::{Duration, Instant};
use editor_transaction::{HistoryTag, Step};

pub struct HistoryEntry {
    pub steps: Vec<Step>,
    pub tag: Option<HistoryTag>,
}

pub struct History {
    undos: Vec<HistoryEntry>,
    redos: Vec<HistoryEntry>,
    last_tag: Option<HistoryTag>,
    last_tag_revision: u64,
    last_push_time: Option<Instant>,
    merge_interval: Duration,
}

impl History {
    pub fn new(merge_interval: Duration) -> Self {
        Self {
            undos: Vec::new(),
            redos: Vec::new(),
            last_tag: None,
            last_tag_revision: 0,
            last_push_time: None,
            merge_interval,
        }
    }

    pub fn push(&mut self, steps: &[Step]) {
        self.push_at(steps, Instant::now());
    }

    pub fn push_at(&mut self, steps: &[Step], now: Instant) {
        self.redos.clear();

        let should_merge = self
            .last_push_time
            .map(|t| now.duration_since(t) < self.merge_interval)
            .unwrap_or(false);
        let can_merge_into_last =
            should_merge && matches!(self.undos.last(), Some(e) if e.tag.is_none());

        if can_merge_into_last {
            self.undos
                .last_mut()
                .expect("can_merge_into_last guarantees Some")
                .steps
                .extend_from_slice(steps);
        } else {
            self.undos.push(HistoryEntry {
                steps: steps.to_vec(),
                tag: None,
            });
        }
        self.clear_last_tag();
        self.last_push_time = Some(now);
    }

    pub fn push_tagged(&mut self, steps: &[Step], tag: HistoryTag) {
        self.push_tagged_at(steps, tag, Instant::now());
    }

    pub fn push_tagged_at(&mut self, steps: &[Step], tag: HistoryTag, now: Instant) {
        self.redos.clear();

        self.undos.push(HistoryEntry {
            steps: steps.to_vec(),
            tag: Some(tag.clone()),
        });

        self.last_push_time = Some(now);
        self.last_tag = Some(tag);
        self.bump_last_tag_revision();
    }

    pub fn undo(&mut self) -> Option<Vec<Step>> {
        let entry = self.undos.pop()?;
        let inverse_steps: Vec<Step> = entry.steps.iter().rev().map(|s| s.inverse()).collect();
        self.redos.push(entry);
        Some(inverse_steps)
    }

    pub fn last_inverse_steps(&self) -> Option<Vec<Step>> {
        let entry = self.undos.last()?;
        Some(entry.steps.iter().rev().map(|s| s.inverse()).collect())
    }

    pub fn redo(&mut self) -> Option<Vec<Step>> {
        let entry = self.redos.pop()?;
        let steps = entry.steps.clone();
        self.undos.push(entry);
        Some(steps)
    }

    /// Called after redo so that backspace shortcuts still fire correctly.
    pub fn sync_last_tag_from_top(&mut self) {
        let last_tag = self.undos.last().and_then(|e| e.tag.clone());
        let should_bump = self.last_tag != last_tag || last_tag.is_some();
        self.last_tag = last_tag;
        if should_bump {
            self.bump_last_tag_revision();
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undos.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redos.is_empty()
    }

    pub fn last_tag(&self) -> Option<&HistoryTag> {
        self.last_tag.as_ref()
    }

    pub fn last_tag_revision(&self) -> u64 {
        self.last_tag_revision
    }

    pub fn clear_last_tag(&mut self) {
        if self.last_tag.is_some() {
            self.last_tag = None;
            self.bump_last_tag_revision();
        }
    }

    fn bump_last_tag_revision(&mut self) {
        self.last_tag_revision = self.last_tag_revision.wrapping_add(1);
    }

    pub fn undos_len(&self) -> usize {
        self.undos.len()
    }

    pub fn redos_len(&self) -> usize {
        self.redos.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;
    use editor_model::NodeId;
    use editor_state::{Position, Selection, StableSelection, State};

    fn fixture_state() -> State {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("x") } } }
            selection: (t1, 0)
        };
        s
    }

    fn sel_step(s: &State, from: usize, to: usize) -> Step {
        let from_sel = Selection::collapsed(Position::new(NodeId::ROOT, from));
        let to_sel = Selection::collapsed(Position::new(NodeId::ROOT, to));
        Step::SetSelection {
            old: Some(StableSelection::freeze(&from_sel, &s.doc)),
            new: Some(StableSelection::freeze(&to_sel, &s.doc)),
        }
    }

    fn text_step() -> Step {
        Step::InsertText {
            node_id: NodeId::ROOT,
            offset: 0,
            text: "x".into(),
        }
    }

    #[test]
    fn undo_returns_inverse_steps_in_reverse() {
        let s = fixture_state();
        let mut h = History::new(Duration::from_millis(300));
        h.push_at(&[text_step(), sel_step(&s, 0, 1)], Instant::now());

        let undone = h.undo().unwrap();
        assert_eq!(undone.len(), 2);
        assert!(matches!(&undone[0], Step::SetSelection { old, new }
            if old.as_ref().map(|ss| ss.thaw(&s.doc).head.offset) == Some(1)
                && new.as_ref().map(|ss| ss.thaw(&s.doc).head.offset) == Some(0)));
        assert!(matches!(&undone[1], Step::RemoveText { .. }));
    }

    #[test]
    fn redo_returns_original_steps() {
        let mut h = History::new(Duration::from_millis(300));
        h.push_at(&[text_step()], Instant::now());
        h.undo();

        let redone = h.redo().unwrap();
        assert_eq!(redone.len(), 1);
        assert!(matches!(&redone[0], Step::InsertText { text, .. } if text == "x"));
    }

    #[test]
    fn undo_empty_returns_none() {
        let mut h = History::new(Duration::from_millis(300));
        assert!(h.undo().is_none());
    }

    #[test]
    fn redo_empty_returns_none() {
        let mut h = History::new(Duration::from_millis(300));
        assert!(h.redo().is_none());
    }

    #[test]
    fn push_clears_redo_stack() {
        let mut h = History::new(Duration::from_millis(300));
        let t = Instant::now();
        h.push_at(&[text_step()], t);
        h.undo();
        h.push_at(&[text_step()], t + Duration::from_secs(10));
        assert!(h.redo().is_none());
    }

    #[test]
    fn time_merge_combines_entries() {
        let mut h = History::new(Duration::from_millis(300));
        let t = Instant::now();
        h.push_at(&[text_step()], t);
        h.push_at(&[text_step()], t + Duration::from_millis(100));

        let undone = h.undo().unwrap();
        assert_eq!(undone.len(), 2);
        assert!(h.undo().is_none());
    }

    #[test]
    fn time_gap_separates_entries() {
        let mut h = History::new(Duration::from_millis(300));
        let t = Instant::now();
        h.push_at(&[text_step()], t);
        h.push_at(&[text_step()], t + Duration::from_secs(1));

        h.undo();
        assert!(h.undo().is_some());
    }

    #[test]
    fn push_tagged_always_creates_separate_entry() {
        let mut h = History::new(Duration::from_millis(300));
        let t = Instant::now();
        h.push_at(&[text_step()], t);
        h.push_tagged_at(
            &[text_step()],
            HistoryTag::AutoReplacement,
            t + Duration::from_millis(100),
        );

        // within the merge window, tagged entry is still separate
        h.undo();
        assert!(h.undo().is_some());
    }

    #[test]
    fn push_after_push_tagged_does_not_merge() {
        let mut h = History::new(Duration::from_millis(300));
        let t = Instant::now();
        h.push_tagged_at(&[text_step()], HistoryTag::AutoReplacement, t);
        h.push_at(&[text_step()], t + Duration::from_millis(100));

        // push immediately after a tagged entry also creates a separate entry
        h.undo();
        assert!(h.undo().is_some());
    }

    #[test]
    fn push_tagged_clears_redo_stack() {
        let mut h = History::new(Duration::from_millis(300));
        let t = Instant::now();
        h.push_at(&[text_step()], t);
        h.undo();
        h.push_tagged_at(
            &[text_step()],
            HistoryTag::AutoReplacement,
            t + Duration::from_secs(10),
        );
        assert!(h.redo().is_none());
    }

    #[test]
    fn last_tag_returns_tagged_entry_tag() {
        let mut h = History::new(Duration::from_millis(300));
        h.push_tagged_at(
            &[text_step()],
            HistoryTag::PasteHtml {
                plain_text: "hello".into(),
            },
            Instant::now(),
        );

        assert!(
            matches!(h.last_tag(), Some(HistoryTag::PasteHtml { plain_text }) if plain_text == "hello")
        );
    }

    #[test]
    fn last_tag_returns_none_for_untagged_entry() {
        let mut h = History::new(Duration::from_millis(300));
        h.push_at(&[text_step()], Instant::now());
        assert!(h.last_tag().is_none());
    }

    #[test]
    fn last_tag_returns_none_on_empty_history() {
        let h = History::new(Duration::from_millis(300));
        assert!(h.last_tag().is_none());
    }

    #[test]
    fn push_after_undo_within_merge_window_records_steps() {
        let mut h = History::new(Duration::from_millis(300));
        let t = Instant::now();
        h.push_at(&[text_step()], t);
        h.undo();
        h.push_at(&[text_step()], t + Duration::from_millis(150));

        assert!(h.can_undo(), "steps must not be dropped");
        let undone = h.undo().unwrap();
        assert_eq!(undone.len(), 1);
    }

    #[test]
    fn tagged_entry_undo_redo_roundtrip() {
        let mut h = History::new(Duration::from_millis(300));
        h.push_tagged_at(&[text_step()], HistoryTag::AutoReplacement, Instant::now());

        let undone = h.undo().unwrap();
        assert_eq!(undone.len(), 1);
        assert!(matches!(&undone[0], Step::RemoveText { .. }));

        let redone = h.redo().unwrap();
        assert_eq!(redone.len(), 1);
        assert!(matches!(&redone[0], Step::InsertText { .. }));
    }

    #[test]
    fn untagged_push_after_tagged_clears_last_tag() {
        let mut h = History::new(Duration::from_millis(300));
        let t = Instant::now();
        h.push_tagged_at(
            &[text_step()],
            HistoryTag::PasteHtml {
                plain_text: "hi".into(),
            },
            t,
        );
        h.push_at(&[text_step()], t + Duration::from_millis(100));
        assert!(h.last_tag().is_none());
    }

    #[test]
    fn untagged_push_in_merge_window_keeps_last_tag_none() {
        let mut h = History::new(Duration::from_millis(300));
        let t = Instant::now();
        h.push_at(&[text_step()], t);
        h.push_at(&[text_step()], t + Duration::from_millis(50));
        assert!(h.last_tag().is_none());
    }

    #[test]
    fn untagged_push_after_auto_replacement_clears_last_tag() {
        let mut h = History::new(Duration::from_millis(300));
        let t = Instant::now();
        h.push_tagged_at(&[text_step()], HistoryTag::AutoReplacement, t);
        h.push_at(&[text_step()], t + Duration::from_millis(100));
        assert!(h.last_tag().is_none());
    }

    #[test]
    fn last_inverse_steps_returns_inverse_of_top_entry_without_mutation() {
        let mut h = History::new(Duration::from_millis(300));
        h.push_tagged_at(
            &[text_step()],
            HistoryTag::PasteHtml {
                plain_text: "hi".into(),
            },
            Instant::now(),
        );
        let undos_before = h.undos_len();
        let redos_before = h.redos_len();
        let tag_before = h.last_tag().cloned();

        let inverse = h.last_inverse_steps().expect("Some");
        assert_eq!(inverse.len(), 1);
        assert!(matches!(&inverse[0], Step::RemoveText { .. }));

        assert_eq!(h.undos_len(), undos_before);
        assert_eq!(h.redos_len(), redos_before);
        assert_eq!(h.last_tag().cloned(), tag_before);
    }

    #[test]
    fn last_inverse_steps_returns_none_on_empty() {
        let h = History::new(Duration::from_millis(300));
        assert!(h.last_inverse_steps().is_none());
    }

    #[test]
    fn untagged_push_after_clear_last_tag_does_not_merge_into_tagged_top() {
        let mut h = History::new(Duration::from_millis(300));
        let t = Instant::now();
        h.push_tagged_at(
            &[text_step()],
            HistoryTag::PasteHtml {
                plain_text: "hi".into(),
            },
            t,
        );
        h.clear_last_tag();

        let undos_before = h.undos_len();
        h.push_at(&[text_step()], t + Duration::from_millis(50));

        assert_eq!(
            h.undos_len(),
            undos_before + 1,
            "untagged push must not merge into tagged top entry"
        );
        assert!(
            h.last_tag().is_none(),
            "last_tag remains None after untagged push"
        );
    }

    #[test]
    fn merge_extend_clears_stale_last_tag() {
        let mut h = History::new(Duration::from_millis(300));
        let t = Instant::now();
        h.push_at(&[text_step()], t);
        h.push_tagged_at(
            &[text_step()],
            HistoryTag::PasteHtml {
                plain_text: "x".into(),
            },
            t + Duration::from_millis(100),
        );
        h.undo();
        h.push_at(&[text_step()], t + Duration::from_millis(150));
        assert!(h.last_tag().is_none());
    }
}
