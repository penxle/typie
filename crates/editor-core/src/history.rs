use editor_common::time::{Duration, Instant};
use editor_transaction::{HistoryTag, Step};

pub struct HistoryEntry {
    pub steps: Vec<Step>,
    pub tag: Option<HistoryTag>,
}

pub struct History {
    undos: Vec<HistoryEntry>,
    redos: Vec<HistoryEntry>,
    last_push_time: Option<Instant>,
    merge_interval: Duration,
}

impl History {
    pub fn new(merge_interval: Duration) -> Self {
        Self {
            undos: Vec::new(),
            redos: Vec::new(),
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

        if should_merge {
            if let Some(last) = self.undos.last_mut() {
                if last.tag.is_none() {
                    last.steps.extend_from_slice(steps);
                } else {
                    self.undos.push(HistoryEntry {
                        steps: steps.to_vec(),
                        tag: None,
                    });
                }
            } else {
                self.undos.push(HistoryEntry {
                    steps: steps.to_vec(),
                    tag: None,
                });
            }
        } else {
            self.undos.push(HistoryEntry {
                steps: steps.to_vec(),
                tag: None,
            });
        }

        self.last_push_time = Some(now);
    }

    pub fn push_tagged(&mut self, steps: &[Step], tag: HistoryTag) {
        self.push_tagged_at(steps, tag, Instant::now());
    }

    pub fn push_tagged_at(&mut self, steps: &[Step], tag: HistoryTag, now: Instant) {
        self.redos.clear();

        self.undos.push(HistoryEntry {
            steps: steps.to_vec(),
            tag: Some(tag),
        });

        self.last_push_time = Some(now);
    }

    pub fn undo(&mut self) -> Option<Vec<Step>> {
        let entry = self.undos.pop()?;
        let inverse_steps: Vec<Step> = entry.steps.iter().rev().map(|s| s.inverse()).collect();
        self.redos.push(entry);
        Some(inverse_steps)
    }

    pub fn redo(&mut self) -> Option<Vec<Step>> {
        let entry = self.redos.pop()?;
        let steps = entry.steps.clone();
        self.undos.push(entry);
        Some(steps)
    }

    pub fn can_undo(&self) -> bool {
        !self.undos.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redos.is_empty()
    }

    pub fn last_tag(&self) -> Option<&HistoryTag> {
        self.undos.last()?.tag.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_model::NodeId;
    use editor_state::{Position, Selection};

    fn sel_step(from: usize, to: usize) -> Step {
        Step::SetSelection {
            old: Selection::collapsed(Position::new(NodeId::ROOT, from)),
            new: Selection::collapsed(Position::new(NodeId::ROOT, to)),
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
        let mut h = History::new(Duration::from_millis(300));
        h.push_at(&[text_step(), sel_step(0, 1)], Instant::now());

        let undone = h.undo().unwrap();
        assert_eq!(undone.len(), 2);
        assert!(matches!(&undone[0], Step::SetSelection { old, new }
            if old.head.offset == 1 && new.head.offset == 0));
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

        // 시간 내여도 별도 entry
        h.undo();
        assert!(h.undo().is_some());
    }

    #[test]
    fn push_after_push_tagged_does_not_merge() {
        let mut h = History::new(Duration::from_millis(300));
        let t = Instant::now();
        h.push_tagged_at(&[text_step()], HistoryTag::AutoReplacement, t);
        h.push_at(&[text_step()], t + Duration::from_millis(100));

        // tagged entry 직후 push도 별도 entry
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
    fn last_tag_updates_after_undo() {
        let mut h = History::new(Duration::from_millis(300));
        let t = Instant::now();
        h.push_tagged_at(&[text_step()], HistoryTag::AutoReplacement, t);
        h.push_at(&[text_step()], t + Duration::from_secs(1));

        // 마지막은 untagged
        assert!(h.last_tag().is_none());

        // undo하면 tagged entry가 마지막
        h.undo();
        assert!(matches!(h.last_tag(), Some(HistoryTag::AutoReplacement)));
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
}
