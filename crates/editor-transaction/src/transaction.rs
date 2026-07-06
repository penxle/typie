use editor_crdt::Dot;
use editor_model::{DocView, Marker, Modifier, PlainNode, Subtree};
use editor_state::Selection;
use editor_state::undo::RecordedOp;
use editor_state::{Composition, PendingModifiers, State};

use crate::steps::support;
use crate::{Effect, Step, StepEffect, StepError, StepRecord, TransactionMeta};

pub struct Transaction {
    state: State,
    steps: Vec<Step>,
    step_records: Vec<StepRecord>,
    recorded: Vec<RecordedOp>,
    effects: Vec<Effect>,
    meta: TransactionMeta,
}

#[derive(Clone)]
pub struct Savepoint {
    state: State,
    steps_len: usize,
    step_records_len: usize,
    recorded_len: usize,
    effects_len: usize,
}

impl Transaction {
    pub fn new(state: &State) -> Self {
        Self {
            state: state.clone(),
            steps: Vec::new(),
            step_records: Vec::new(),
            recorded: Vec::new(),
            effects: Vec::new(),
            meta: TransactionMeta::default(),
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn view(&self) -> DocView<'_> {
        self.state.view()
    }

    pub fn selection(&self) -> Option<Selection> {
        self.state.selection
    }

    pub fn pending_modifiers(&self) -> &PendingModifiers {
        &self.state.pending_modifiers
    }

    pub fn composition(&self) -> Option<&Composition> {
        self.state.composition.as_ref()
    }

    pub fn doc_changed(&self) -> bool {
        self.steps.iter().any(|s| s.is_doc_step())
    }

    pub fn selection_changed(&self) -> bool {
        self.steps.iter().any(|s| s.is_selection_step())
    }

    pub fn step_records_len(&self) -> usize {
        self.step_records.len()
    }

    pub fn step_records_since(&self, start: usize) -> &[StepRecord] {
        &self.step_records[start.min(self.step_records.len())..]
    }

    pub fn push_effect(&mut self, effect: Effect) {
        self.effects.push(effect);
    }

    fn apply_step(&mut self, step: Step) -> Result<(), StepError> {
        let recorded = self
            .state
            .batch_with_recorded_mut::<_, StepError>(|batched| {
                step.apply_to_with_effect(batched)
            })?;
        self.recorded.extend(recorded);
        self.step_records.push(StepRecord {
            step: step.clone(),
            effect: StepEffect,
        });
        self.steps.push(step);
        Ok(())
    }

    pub fn savepoint(&self) -> Savepoint {
        let sp = Savepoint {
            state: self.state.clone(),
            steps_len: self.steps.len(),
            step_records_len: self.step_records.len(),
            recorded_len: self.recorded.len(),
            effects_len: self.effects.len(),
        };
        sp
    }

    pub fn rollback(&mut self, sp: Savepoint) {
        self.state = sp.state;
        self.steps.truncate(sp.steps_len);
        self.step_records.truncate(sp.step_records_len);
        self.recorded.truncate(sp.recorded_len);
        self.effects.truncate(sp.effects_len);
    }

    pub fn insert_text(&mut self, block: Dot, offset: usize, text: &str) -> Result<(), StepError> {
        self.apply_step(Step::InsertText {
            block,
            offset,
            text: text.to_string(),
        })
    }

    /// Removes a contiguous run of text chars addressed by child-slot offset.
    /// Every slot in the range must be a `Char`; inline atoms belong to
    /// `remove_child_slots`.
    pub fn remove_text(&mut self, block: Dot, offset: usize, len: usize) -> Result<(), StepError> {
        let text = support::read_text(&self.state.projected, block, offset, len)?;
        self.apply_step(Step::RemoveText {
            block,
            offset,
            text,
        })
    }

    /// Removes direct child slots `[from, to)`, lowering char runs to
    /// `RemoveText` and atom/block slots to `RemoveSubtree`.
    pub fn remove_child_slots(
        &mut self,
        parent: Dot,
        from: usize,
        to: usize,
    ) -> Result<(), StepError> {
        let steps = support::remove_child_slots_steps(&self.state.projected, parent, from, to)?;
        self.apply_steps(steps)?;
        Ok(())
    }

    pub fn insert_subtree(
        &mut self,
        parent: Dot,
        index: usize,
        subtree: Subtree,
    ) -> Result<(), StepError> {
        self.apply_step(Step::InsertSubtree {
            parent,
            index,
            subtree,
        })
    }

    pub fn remove_subtree(&mut self, block: Dot) -> Result<(), StepError> {
        let (parent, index, subtree) = {
            let ps = &self.state.projected;
            let parent = ps.parent_of(block).ok_or(StepError::NodeNotFound(block))?;
            let index = ps
                .child_elem_dots(parent)
                .iter()
                .position(|d| *d == block)
                .ok_or(StepError::NodeNotFound(block))?;
            let subtree =
                support::capture_subtree(ps, block).ok_or(StepError::NodeNotFound(block))?;
            (parent, index, subtree)
        };
        self.apply_step(Step::RemoveSubtree {
            parent,
            index,
            subtree,
        })
    }

    /// Moves a block node to `new_parent` at `new_index`, counted in the same
    /// full child-slot index domain as `insert_subtree` and `remove_subtree`.
    /// Leaf siblings such as image atoms are included in this index domain.
    pub fn move_node(
        &mut self,
        block: Dot,
        new_parent: Dot,
        new_index: usize,
    ) -> Result<(), StepError> {
        let (old_parent, old_index) = {
            let ps = &self.state.projected;
            let parent = ps.parent_of(block).ok_or(StepError::NodeNotFound(block))?;
            let index = ps
                .child_elem_dots(parent)
                .iter()
                .position(|d| *d == block)
                .ok_or(StepError::NodeNotFound(block))?;
            (parent, index)
        };
        self.apply_step(Step::MoveNode {
            block,
            old_parent,
            old_index,
            new_parent,
            new_index,
        })
    }

    pub fn set_node(&mut self, block: Dot, new_node: PlainNode) -> Result<(), StepError> {
        let old_node = self
            .state
            .projected
            .block_node(block)
            .ok_or(StepError::NodeNotFound(block))?
            .to_plain();
        self.apply_step(Step::SetNode {
            block,
            old_node,
            new_node,
        })
    }

    pub fn split_node(&mut self, block: Dot, offset: usize) -> Result<(), StepError> {
        self.apply_step(Step::SplitNode { block, offset })
    }

    pub fn merge_node(&mut self, block: Dot) -> Result<(), StepError> {
        let offset = support::children_count(&self.state.projected, block)
            .ok_or(StepError::NodeNotFound(block))?;
        self.apply_step(Step::MergeNode { block, offset })
    }

    pub fn add_modifier(&mut self, block: Dot, modifier: Modifier) -> Result<(), StepError> {
        self.apply_step(Step::AddModifier { block, modifier })
    }

    pub fn remove_modifier(&mut self, block: Dot, modifier: Modifier) -> Result<(), StepError> {
        self.apply_step(Step::RemoveModifier { block, modifier })
    }

    pub fn add_span_modifier(
        &mut self,
        first: Dot,
        last: Dot,
        modifier: Modifier,
    ) -> Result<(), StepError> {
        self.apply_step(Step::AddSpanModifier {
            first,
            last,
            modifier,
        })
    }

    pub fn remove_span_modifier(
        &mut self,
        first: Dot,
        last: Dot,
        modifier: Modifier,
    ) -> Result<(), StepError> {
        self.apply_step(Step::RemoveSpanModifier {
            first,
            last,
            modifier,
        })
    }

    pub fn set_marker(&mut self, block: Dot, marker: Option<Marker>) -> Result<(), StepError> {
        let old = self.state.projected.node_markers().value_of(block);
        if old == marker {
            return Ok(());
        }
        self.apply_step(Step::SetNodeMarker {
            block,
            old,
            new: marker,
        })
    }

    pub fn set_selection(&mut self, selection: Option<Selection>) -> Result<(), StepError> {
        if self.state.selection == selection {
            return Ok(());
        }
        let old = self.state.selection;
        self.apply_step(Step::SetSelection {
            old,
            new: selection,
        })
    }

    pub fn set_pending_modifiers(&mut self, modifiers: PendingModifiers) -> Result<(), StepError> {
        if self.state.pending_modifiers == modifiers {
            return Ok(());
        }
        let old = self.state.pending_modifiers.clone();
        self.apply_step(Step::SetPendingModifiers {
            old,
            new: modifiers,
        })
    }

    pub fn clear_pending_format(&mut self) -> Result<(), StepError> {
        self.set_pending_modifiers(PendingModifiers::new())
    }

    pub fn set_composition(&mut self, composition: Option<Composition>) -> Result<(), StepError> {
        let old = self.state.composition;
        if old == composition {
            return Ok(());
        }
        self.apply_step(Step::SetComposition {
            old,
            new: composition,
        })
    }

    pub fn batch<F, E>(&mut self, f: F) -> Result<(), E>
    where
        F: FnOnce(&mut Transaction) -> Result<(), E>,
        E: From<StepError>,
    {
        let sp = self.savepoint();
        match f(self) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.rollback(sp);
                Err(e)
            }
        }
    }

    pub fn apply_steps(&mut self, steps: Vec<Step>) -> Result<Vec<StepRecord>, StepError> {
        let recorded = self
            .state
            .batch_with_recorded_mut::<_, StepError>(|batched| {
                for step in &steps {
                    step.apply_to_with_effect(batched)?;
                }
                Ok(())
            })?;
        self.recorded.extend(recorded);
        let records: Vec<StepRecord> = steps
            .iter()
            .cloned()
            .map(|step| StepRecord {
                step,
                effect: StepEffect,
            })
            .collect();
        self.step_records.extend(records.clone());
        self.steps.extend(steps);
        Ok(records)
    }

    /// Like [`apply_steps`](Self::apply_steps), for a run of steps that lower purely
    /// to sequence deletions (`RemoveText` / `RemoveSubtree`). Their ops apply
    /// warm-only, leaving the projection stale across the run — safe because every
    /// step addresses pre-delete slots and a deletion never changes a surviving
    /// element's addressing — and ONE coverage-preserving reprojection restores it at
    /// the end, instead of one window reprojection per step (the `O(steps · window)`
    /// cost of deleting many blocks). A step that emits a non-delete op flushes the
    /// deferral before that op projects, so misuse degrades to `apply_steps` cost.
    pub fn apply_steps_bulk_delete(
        &mut self,
        steps: Vec<Step>,
    ) -> Result<Vec<StepRecord>, StepError> {
        if steps.is_empty() {
            return Ok(Vec::new());
        }
        let mut deferred = 0usize;
        let recorded = self
            .state
            .batch_with_recorded_mut::<_, StepError>(|batched| {
                batched.set_defer_deletes(true);
                for step in &steps {
                    step.apply_to_with_effect(batched)?;
                }
                deferred = batched.deferred_deletes();
                Ok(())
            })?;
        if deferred > 0 {
            self.state
                .projected_mut()
                .reproject_after_delete()
                .map_err(editor_state::StateError::from)?;
        }
        self.recorded.extend(recorded);
        let records: Vec<StepRecord> = steps
            .iter()
            .cloned()
            .map(|step| StepRecord {
                step,
                effect: StepEffect,
            })
            .collect();
        self.step_records.extend(records.clone());
        self.steps.extend(steps);
        Ok(records)
    }

    pub fn meta(&self) -> &TransactionMeta {
        &self.meta
    }

    pub fn update_meta(&mut self, f: impl FnOnce(&mut TransactionMeta)) {
        f(&mut self.meta);
    }

    pub fn commit(
        mut self,
    ) -> (
        State,
        Vec<StepRecord>,
        Vec<RecordedOp>,
        Vec<Effect>,
        TransactionMeta,
    ) {
        // Only take the copy-on-write clone when there is something to commit.
        // Selection/caret-only transactions leave the graph's pending buffer
        // empty, so they keep sharing the `Arc<ProjectedState>` untouched.
        if !self.state.projected.graph().pending().is_empty() {
            self.state.projected_mut().commit();
        }
        (
            self.state,
            self.step_records,
            self.recorded,
            self.effects,
            self.meta,
        )
    }

    #[cfg(test)]
    pub(crate) fn ops_for_test(&self) -> Vec<editor_crdt::Op<editor_model::EditOp>> {
        self.recorded.iter().map(|r| r.op.clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HistoryMeta;
    use editor_macros::state;
    use editor_state::{Position, Selection};

    fn block_text(state: &State, elem: &Dot) -> String {
        state
            .view()
            .node(*elem)
            .map(|n| n.inline_text())
            .unwrap_or_default()
    }

    #[test]
    fn new_transaction_reads_state() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 0)
        };
        let tr = Transaction::new(&state);
        assert_eq!(
            tr.selection(),
            Some(Selection::collapsed(Position::new(p1, 0)))
        );
        assert!(tr.view().node(p1).is_some());
    }

    #[test]
    fn insert_text_records_step() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.insert_text(p1, 5, " Beautiful").unwrap();
        assert_eq!(block_text(tr.state(), &p1), "Hello Beautiful World");
        let (_, steps, _, _, _) = tr.commit();
        assert_eq!(steps.len(), 1);
        assert!(matches!(&steps[0].step, Step::InsertText { .. }));
    }

    #[test]
    fn remove_text_derives_content_from_state() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.remove_text(p1, 5, 6).unwrap();
        assert_eq!(block_text(tr.state(), &p1), "Hello");
        let (_, steps, _, _, _) = tr.commit();
        match &steps[0].step {
            Step::RemoveText { text, .. } => assert_eq!(text, " World"),
            _ => panic!("expected RemoveText"),
        }
    }

    #[test]
    fn remove_text_after_atom_derives_content_from_child_slots() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.remove_text(p1, 2, 1).unwrap();
        assert_eq!(block_text(tr.state(), &p1), "a");
        let (_, steps, _, _, _) = tr.commit();
        match &steps[0].step {
            Step::RemoveText { text, .. } => assert_eq!(text, "b"),
            _ => panic!("expected RemoveText"),
        }
    }

    #[test]
    fn remove_text_rejects_range_containing_atom() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        let result = tr.remove_text(p1, 1, 2);
        assert!(matches!(
            result,
            Err(StepError::ExpectedText {
                block,
                offset: 1,
            }) if block == p1
        ));
        assert_eq!(block_text(tr.state(), &p1), "ab");
    }

    #[test]
    fn remove_text_step_rejects_atom_slot() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 0)
        };
        let result = Step::RemoveText {
            block: p1,
            offset: 1,
            text: "b".into(),
        }
        .apply(&state);
        assert!(matches!(
            result,
            Err(StepError::ExpectedText {
                block,
                offset: 1,
            }) if block == p1
        ));
    }

    #[test]
    fn remove_text_step_rejects_text_mismatch() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("a") } } }
            selection: (p1, 0)
        };
        let result = Step::RemoveText {
            block: p1,
            offset: 0,
            text: "x".into(),
        }
        .apply(&state);
        assert!(matches!(
            result,
            Err(StepError::TextMismatch {
                block,
                offset: 0,
                expected: 'x',
                actual: 'a',
            }) if block == p1
        ));
    }

    #[test]
    fn remove_child_slots_deletes_mixed_inline_range() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.remove_child_slots(p1, 1, 3).unwrap();
        assert_eq!(block_text(tr.state(), &p1), "a");
        let (_, steps, _, _, _) = tr.commit();
        assert_eq!(steps.len(), 2);
        assert!(steps.iter().any(|r| matches!(
            &r.step,
            Step::RemoveText { offset: 2, text, .. } if text == "b"
        )));
        assert!(
            steps
                .iter()
                .any(|r| matches!(&r.step, Step::RemoveSubtree { index: 1, .. }))
        );
    }

    #[test]
    fn remove_child_slots_captures_atom_leaf_modifiers() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("a") tab [font_size(2400)] } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.remove_child_slots(p1, 1, 2).unwrap();

        let (_, steps, _, _, _) = tr.commit();
        let subtree = steps
            .iter()
            .find_map(|r| match &r.step {
                Step::RemoveSubtree { subtree, .. } => Some(subtree),
                _ => None,
            })
            .expect("tab deletion must record RemoveSubtree");

        assert_eq!(subtree.modifiers, vec![Modifier::FontSize { value: 2400 }]);
    }

    #[test]
    fn insert_text_error_on_missing_node() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        let result = tr.insert_text(editor_crdt::Dot::new(9, 9), 0, "X");
        assert!(result.is_err());
    }

    #[test]
    fn savepoint_rollback_preserves_earlier_steps() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.insert_text(p1, 11, "!").unwrap();
        let sp = tr.savepoint();
        tr.remove_text(p1, 0, 5).unwrap();
        tr.rollback(sp);
        assert_eq!(block_text(tr.state(), &p1), "Hello World!");
        let (_, steps, _, _, _) = tr.commit();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn savepoint_rollback_truncates_ops() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hi") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.insert_text(p1, 2, "x").unwrap();
        let sp = tr.savepoint();
        tr.insert_text(p1, 3, "y").unwrap();
        let after_two = tr.ops_for_test().len();
        tr.rollback(sp);
        assert!(after_two > tr.ops_for_test().len());
    }

    #[test]
    fn batch_rolls_back_on_error() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        let result = tr.batch::<_, StepError>(|tr| {
            tr.insert_text(p1, 0, "abc")?;
            tr.insert_text(p1, 999, "x")?;
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(block_text(tr.state(), &p1), "hello");
    }

    #[test]
    fn commit_seals_one_changeset() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let baseline = state.graph().changesets().len();
        let mut tr = Transaction::new(&state);
        tr.insert_text(p1, 0, "a").unwrap();
        tr.insert_text(p1, 1, "b").unwrap();
        let (new_state, ..) = tr.commit();
        assert_eq!(new_state.graph().changesets().len(), baseline + 1);
        assert!(new_state.graph().pending().is_empty());
    }

    #[test]
    fn commit_with_no_steps_seals_no_changeset() {
        let (state, _) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let baseline = state.graph().changesets().len();
        let tr = Transaction::new(&state);
        let (new_state, ..) = tr.commit();
        assert_eq!(new_state.graph().changesets().len(), baseline);
    }

    #[test]
    fn commit_returns_ops_alongside_steps() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hi") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.insert_text(p1, 2, "!").unwrap();
        let (_state, steps, ops, _effects, _meta) = tr.commit();
        assert!(!steps.is_empty());
        assert!(!ops.is_empty());
    }

    #[test]
    fn apply_steps_records_preserve_input_order() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let steps = vec![
            Step::SetComposition {
                old: None,
                new: Some(Composition { start: 0, end: 0 }),
            },
            Step::InsertText {
                block: p1,
                offset: 5,
                text: "!".into(),
            },
            Step::SetComposition {
                old: Some(Composition { start: 0, end: 0 }),
                new: None,
            },
        ];
        let mut tr = Transaction::new(&state);
        let records = tr.apply_steps(steps.clone()).unwrap();
        assert_eq!(records.len(), steps.len());
        for (record, step) in records.iter().zip(&steps) {
            assert_eq!(&record.step, step);
        }
        assert_eq!(block_text(tr.state(), &p1), "Hello!");
    }

    #[test]
    fn set_selection_records_step_and_inverts() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        let new_sel = Selection::collapsed(Position::new(p1, 5));
        tr.set_selection(Some(new_sel)).unwrap();
        assert_eq!(tr.selection(), Some(new_sel));
        let (after, records, ..) = tr.commit();
        let restored = records
            .iter()
            .rev()
            .fold(after, |s, r| r.step.inverse().apply(&s).unwrap().state);
        assert_eq!(restored.selection, state.selection);
    }

    #[test]
    fn steps_produce_valid_inverses() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.insert_text(p1, 5, " Beautiful").unwrap();
        tr.set_selection(Some(Selection::collapsed(Position::new(p1, 15))))
            .unwrap();
        let (_, step_records, _, _, _) = tr.commit();
        let mut current = step_records.iter().fold(state.clone(), |s, record| {
            record.step.apply(&s).unwrap().state
        });
        for record in step_records.iter().rev() {
            current = record.step.inverse().apply(&current).unwrap().state;
        }
        assert_eq!(block_text(&current, &p1), "Hello World");
        assert_eq!(current.selection, state.selection);
    }

    #[test]
    fn new_transaction_has_default_meta() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        assert!(matches!(tr.meta().history, HistoryMeta::Record));
        tr.update_meta(|m| m.history = HistoryMeta::Skip);
        let (_, _, _, _, meta) = tr.commit();
        assert!(matches!(meta.history, HistoryMeta::Skip));
    }
}
