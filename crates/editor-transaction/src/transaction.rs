use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{DocView, Modifier, ModifierType, NodeAttr, NodeType, PlainNode, Subtree};
use editor_state::Selection;
use editor_state::undo::RecordedOp;
use editor_state::{BatchedState, Composition, PendingModifiers, State};
use strum::IntoEnumIterator;

use crate::steps::support;
use crate::{Effect, Step, StepEffect, StepError, StepRecord, TransactionMeta};

pub struct Transaction {
    state: State,
    steps: Vec<Step>,
    step_records: Vec<StepRecord>,
    recorded: Vec<RecordedOp>,
    effects: Vec<Effect>,
    meta: TransactionMeta,
    keep_pending: bool,
}

#[derive(Clone)]
pub struct Savepoint {
    state: State,
    steps_len: usize,
    step_records_len: usize,
    recorded_len: usize,
    effects_len: usize,
}

/// `Step::DeleteOpaque`'s `emitted` field does not exist at construction (the
/// CRDT delete op's own dot is only assigned once `apply_to` runs) — this
/// rebuilds the step with `emitted` populated from the ops it just produced, so
/// `Step::inverse()` can consume them (mechanism for a delete-only step whose
/// inverse is a dot-based `Undel`, not a re-inserted carrier).
fn finalize_step(step: &Step, batched: &BatchedState, before: usize) -> Step {
    match step {
        Step::DeleteOpaque { dots, .. } => Step::DeleteOpaque {
            dots: dots.clone(),
            emitted: batched.emitted_dots_since(before),
        },
        other => other.clone(),
    }
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
            keep_pending: false,
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

    pub fn has_pending_modifiers_step(&self) -> bool {
        self.steps.iter().any(|s| s.is_pending_modifiers_step())
    }

    pub fn keep_pending_modifiers(&mut self) {
        self.keep_pending = true;
    }

    pub fn keeps_pending_modifiers(&self) -> bool {
        self.keep_pending
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
        let mut final_step = step.clone();
        let recorded = self
            .state
            .batch_with_recorded_mut::<_, StepError>(|batched| {
                let before = batched.emitted_len();
                step.apply_to_with_effect(batched)?;
                final_step = finalize_step(&step, batched, before);
                Ok(())
            })?;
        self.recorded.extend(recorded);
        self.step_records.push(StepRecord {
            step: final_step.clone(),
            effect: StepEffect,
        });
        self.steps.push(final_step);
        Ok(())
    }

    pub fn savepoint(&self) -> Savepoint {
        Savepoint {
            state: self.state.clone(),
            steps_len: self.steps.len(),
            step_records_len: self.step_records.len(),
            recorded_len: self.recorded.len(),
            effects_len: self.effects.len(),
        }
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

    pub fn reissue_subtree(
        &mut self,
        parent: Dot,
        index: usize,
        subtree: Subtree,
    ) -> Result<(), StepError> {
        self.apply_step(Step::ReissueSubtree {
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

    /// Moves a block node or atom leaf to `new_parent` at `new_index`, counted
    /// in the same full child-slot index domain as `insert_subtree` and
    /// `remove_subtree`. Character leaves are handled by text operations.
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
            .or_else(|| self.state.projected.atom_leaf_node(block))
            .ok_or(StepError::NodeNotFound(block))?
            .to_plain();
        self.apply_step(Step::SetNode {
            block,
            old_node,
            new_node,
        })
    }

    pub fn set_node_attr(&mut self, block: Dot, new: NodeAttr) -> Result<(), StepError> {
        let node = self
            .state
            .projected
            .block_node(block)
            .or_else(|| self.state.projected.atom_leaf_node(block))
            .ok_or(StepError::NodeNotFound(block))?;
        let old = node
            .to_plain()
            .to_attrs()
            .into_iter()
            .find(|attr| attr.same_field(&new))
            .ok_or(StepError::NodeAttrKindMismatch { block })?;
        self.apply_step(Step::SetNodeAttr { block, old, new })
    }

    pub fn replace_block_type(&mut self, block: Dot, new_type: NodeType) -> Result<(), StepError> {
        let old_type = self
            .state
            .projected
            .block_node_type(block)
            .ok_or(StepError::NodeNotFound(block))?;
        self.apply_step(Step::ReplaceBlockType {
            block,
            old_type,
            new_type,
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

    pub fn set_carry_modifier(&mut self, block: Dot, modifier: Modifier) -> Result<(), StepError> {
        let ty = modifier.as_type();
        let old = self
            .state
            .projected
            .node_carries()
            .modifiers_of(block)
            .get(&ty)
            .cloned();
        self.apply_step(Step::SetNodeCarry {
            block,
            ty,
            old,
            new: Some(modifier),
        })
    }

    pub fn remove_carry_modifier(&mut self, block: Dot, ty: ModifierType) -> Result<(), StepError> {
        let old = self
            .state
            .projected
            .node_carries()
            .modifiers_of(block)
            .get(&ty)
            .cloned();
        self.apply_step(Step::SetNodeCarry {
            block,
            ty,
            old,
            new: None,
        })
    }

    pub fn replace_carry(&mut self, block: Dot, modifiers: Vec<Modifier>) -> Result<(), StepError> {
        let mut wanted: BTreeMap<ModifierType, Modifier> = BTreeMap::new();
        for m in modifiers {
            if m.as_type().is_carry_kind() {
                wanted.insert(m.as_type(), m);
            }
        }
        let current = self.state.projected.node_carries().modifiers_of(block);
        for ty in ModifierType::iter().filter(|t| t.is_carry_kind()) {
            let old = current.get(&ty).cloned();
            let new = wanted.get(&ty).cloned();
            self.apply_step(Step::SetNodeCarry {
                block,
                ty,
                old,
                new,
            })?;
        }
        Ok(())
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
        let mut modifiers = modifiers;
        modifiers.retain(|pm| pm.as_type().is_carry_kind());
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
        let mut finals: Vec<Step> = Vec::with_capacity(steps.len());
        let recorded = self
            .state
            .batch_with_recorded_mut::<_, StepError>(|batched| {
                for step in &steps {
                    let before = batched.emitted_len();
                    step.apply_to_with_effect(batched)?;
                    finals.push(finalize_step(step, batched, before));
                }
                Ok(())
            })?;
        self.recorded.extend(recorded);
        let records: Vec<StepRecord> = finals
            .iter()
            .cloned()
            .map(|step| StepRecord {
                step,
                effect: StepEffect,
            })
            .collect();
        self.step_records.extend(records.clone());
        self.steps.extend(finals);
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
        let mut finals: Vec<Step> = Vec::with_capacity(steps.len());
        let recorded = self
            .state
            .batch_with_recorded_mut::<_, StepError>(|batched| {
                batched.set_defer_deletes(true);
                for step in &steps {
                    let before = batched.emitted_len();
                    step.apply_to_with_effect(batched)?;
                    finals.push(finalize_step(step, batched, before));
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
        let records: Vec<StepRecord> = finals
            .iter()
            .cloned()
            .map(|step| StepRecord {
                step,
                effect: StepEffect,
            })
            .collect();
        self.step_records.extend(records.clone());
        self.steps.extend(finals);
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
    use std::sync::Arc;

    use super::*;
    use crate::HistoryMeta;
    use editor_crdt::ListOp;
    use editor_macros::state;
    use editor_model::{AtomLeaf, Child, EditOp, NodeType, SeqItem, UnknownNode};
    use editor_state::{Position, ProjectedState, Selection};

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

    /// `remove_child_slots_steps` must route a `SeqItem::Unknown` slot to
    /// `Step::DeleteOpaque` (position-based, `emitted` filled in post-apply) —
    /// not the old `Err(InvalidChildSlot)` — and its inverse must be a
    /// dot-based `Undel` that restores the SAME dot.
    #[test]
    fn remove_child_slots_routes_unknown_leaf_to_delete_opaque() {
        use editor_crdt::ListOp;
        use editor_model::{EditOp, SeqItem};

        let (base, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 0)
        };
        let mut state = base;
        let unknown = state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 2,
                item: SeqItem::Unknown {
                    tag: 42,
                    bytes: vec![0xFF],
                },
            }))
            .unwrap()
            .id;

        let mut tr = Transaction::new(&state);
        tr.remove_child_slots(p1, 1, 2).unwrap();
        let (after, records, ..) = tr.commit();

        assert!(after.view().leaf(unknown).is_none());
        let emitted = records
            .iter()
            .find_map(|r| match &r.step {
                Step::DeleteOpaque { dots, emitted } if dots == &vec![unknown] => {
                    Some(emitted.clone())
                }
                _ => None,
            })
            .expect("must record a DeleteOpaque step targeting the unknown dot");
        assert_eq!(
            emitted.len(),
            1,
            "must have recorded exactly one emitted delete op dot"
        );

        let restored = records
            .iter()
            .rev()
            .fold(after, |s, r| r.step.inverse().apply(&s).unwrap().state);
        assert!(
            restored.view().leaf(unknown).is_some(),
            "inverse (Undel) must restore the unknown leaf under the SAME dot"
        );
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

    fn collect_items(ps: &ProjectedState, block: Dot) -> BTreeMap<Dot, SeqItem> {
        fn walk(ps: &ProjectedState, block: Dot, out: &mut BTreeMap<Dot, SeqItem>) {
            if let Some(node_type) = ps.block_node_type(block) {
                out.insert(
                    block,
                    SeqItem::Block {
                        node_type,
                        parents: Vec::new(),
                        attrs: Vec::new(),
                    },
                );
            }
            if let Some(children) = ps.block_children(block) {
                for c in children {
                    match c {
                        Child::Leaf { id, item } => {
                            out.insert(id, item);
                        }
                        Child::Block(d) => walk(ps, d, out),
                    }
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(ps, block, &mut out);
        out
    }

    fn item_of(ps: &ProjectedState, dot: Dot) -> Option<SeqItem> {
        if let Some(node_type) = ps.block_node_type(dot) {
            return Some(SeqItem::Block {
                node_type,
                parents: Vec::new(),
                attrs: Vec::new(),
            });
        }
        let parent = ps.parent_of(dot)?;
        ps.block_children(parent)?
            .into_iter()
            .find_map(|c| match c {
                Child::Leaf { id, item } if id == dot => Some(item),
                _ => None,
            })
    }

    fn alias_ops_in(ops: &[editor_crdt::Op<editor_model::EditOp>]) -> Vec<&editor_model::AliasOp> {
        ops.iter()
            .filter_map(|op| match &op.payload {
                EditOp::Alias(a) => Some(a),
                _ => None,
            })
            .collect()
    }

    fn assert_pairs_match_content(
        before_items: &BTreeMap<Dot, SeqItem>,
        pd: &ProjectedState,
        pairs: &[editor_model::AliasRun],
    ) {
        for r in pairs {
            for i in 0..r.len as u64 {
                let old = Dot::new(r.old_start.actor, r.old_start.clock + i);
                let new = Dot::new(r.new_start.actor, r.new_start.clock + i);
                let old_item = before_items.get(&old).expect("move 전 스냅샷에 존재");
                let new_item = item_of(pd, new).expect("move 후 상태에 존재");
                match (old_item, &new_item) {
                    (SeqItem::Char(a), SeqItem::Char(b)) => assert_eq!(a, b, "char 페어링 불일치"),
                    (SeqItem::Atom(a), SeqItem::Atom(b)) => assert_eq!(a, b, "atom 페어링 불일치"),
                    (SeqItem::Block { node_type: a, .. }, SeqItem::Block { node_type: b, .. }) => {
                        assert_eq!(a, b, "block 페어링 불일치")
                    }
                    (a, b) => panic!("kind 불일치 페어링: {a:?} -> {b:?}"),
                }
            }
        }
    }

    fn assert_replace_type_pairs_match_content(
        before_items: &BTreeMap<Dot, SeqItem>,
        pd: &ProjectedState,
        replaced: Dot,
        new_type: NodeType,
        pairs: &[editor_model::AliasRun],
    ) {
        for r in pairs {
            for i in 0..r.len as u64 {
                let old = Dot::new(r.old_start.actor, r.old_start.clock + i);
                let new = Dot::new(r.new_start.actor, r.new_start.clock + i);
                let old_item = before_items.get(&old).expect("replace 전 스냅샷에 존재");
                let new_item = item_of(pd, new).expect("replace 후 상태에 존재");
                match (old_item, &new_item) {
                    (
                        SeqItem::Block {
                            node_type: old_type,
                            ..
                        },
                        SeqItem::Block {
                            node_type: actual_type,
                            ..
                        },
                    ) if old == replaced => {
                        assert_ne!(*old_type, new_type, "테스트 전제: root type changes");
                        assert_eq!(*actual_type, new_type, "replaced block type mismatch");
                    }
                    (SeqItem::Char(a), SeqItem::Char(b)) => assert_eq!(a, b, "char 페어링 불일치"),
                    (SeqItem::Atom(a), SeqItem::Atom(b)) => assert_eq!(a, b, "atom 페어링 불일치"),
                    (SeqItem::Block { node_type: a, .. }, SeqItem::Block { node_type: b, .. }) => {
                        assert_eq!(a, b, "descendant block 페어링 불일치")
                    }
                    (a, b) => panic!("kind 불일치 페어링: {a:?} -> {b:?}"),
                }
            }
        }
    }

    #[test]
    fn move_node_emits_single_alias_op_pairing_old_to_new() {
        let (state, root, p1) = state! {
            doc { root: root {
                p1: paragraph { text("ab") }
                paragraph { text("") }
            } }
            selection: (p1, 0)
        };
        let before_items = collect_items(&state.projected, p1);

        let mut tr = Transaction::new(&state);
        tr.move_node(p1, root, 1).unwrap();
        let ops = tr.ops_for_test();
        let alias_ops = alias_ops_in(&ops);
        assert_eq!(alias_ops.len(), 1, "move 1회당 alias op 1개");
        let total: u32 = alias_ops[0].pairs.iter().map(|r| r.len).sum();
        assert_eq!(total as usize, 3, "블록 1 + char 2 전부 페어링");

        assert_pairs_match_content(&before_items, &tr.state().projected, &alias_ops[0].pairs);
    }

    #[test]
    fn replace_block_type_emits_alias_op_pairing_old_to_new() {
        let (state, list, _p1) = state! {
            doc { root {
                list: ordered_list { list_item { p1: paragraph { text("ab") } } }
                paragraph {}
            } }
            selection: (p1, 0)
        };
        let before_items = collect_items(&state.projected, list);

        let mut tr = Transaction::new(&state);
        tr.replace_block_type(list, NodeType::BulletList).unwrap();

        let view = tr.state().view();
        assert!(view.node(list).is_none(), "old list dot is tombstoned");
        let new_list = view
            .alias_classes()
            .resolve_with(list, |dot| view.node(dot).is_some());
        assert_eq!(
            view.node(new_list).unwrap().node_type(),
            NodeType::BulletList
        );

        let ops = tr.ops_for_test();
        let alias_ops = alias_ops_in(&ops);
        assert_eq!(alias_ops.len(), 1, "replace 1회당 alias op 1개");
        let total: u32 = alias_ops[0].pairs.iter().map(|r| r.len).sum();
        assert_eq!(total as usize, before_items.len(), "subtree 전부 페어링");

        assert_replace_type_pairs_match_content(
            &before_items,
            &tr.state().projected,
            list,
            NodeType::BulletList,
            &alias_ops[0].pairs,
        );
    }

    #[test]
    fn replace_block_type_rejects_incompatible_children() {
        let (state, list, _p1) = state! {
            doc { root {
                list: ordered_list { list_item { p1: paragraph { text("ab") } } }
                paragraph {}
            } }
            selection: (p1, 0)
        };

        let mut tr = Transaction::new(&state);
        let result = tr.replace_block_type(list, NodeType::Paragraph);
        assert!(matches!(
            result,
            Err(StepError::IncompatibleBlockTypeReplacement { block, .. }) if block == list
        ));
    }

    #[test]
    fn stable_selection_resolves_mid_transaction_after_move_node() {
        use editor_state::{StableResolveCtx, StableSelection};

        let (state, root, p1) = state! {
            doc { root: root {
                p1: paragraph { text("hello") }
                paragraph { text("world") }
            } }
            selection: (p1, 0)
        };
        let before_view = state.view();
        let sel = Selection::new(Position::new(p1, 1), Position::new(p1, 4));
        let stable = StableSelection::capture(&sel, &before_view);

        let mut tr = Transaction::new(&state);
        tr.move_node(p1, root, 1).unwrap();

        // Resolve *before* commit: `tr.state()`/`tr.view()` must already reflect
        // the alias pairing this `move_node` call just emitted.
        let resolved = {
            let view = tr.view();
            let ctx = StableResolveCtx::from_live(&view, tr.state().projected.seq_checkout());
            stable.resolve(&ctx)
        }
        .expect("resolves against the transaction's in-flight state, before commit");

        assert_ne!(
            resolved.anchor.node, p1,
            "moved content now lives under a fresh dot mid-transaction"
        );
        let view = tr.view();
        assert_eq!(
            resolved.resolve(&view).unwrap().collect_text(),
            "ell",
            "mid-transaction resolve must follow the alias to the re-emitted paragraph"
        );
    }

    #[test]
    fn move_node_rejects_unknown_bearing_subtree_losslessly() {
        let (mut state, root, p1) = state! {
            doc { root: root {
                p1: paragraph { text("ab") }
                paragraph { text("") }
            } }
            selection: (p1, 0)
        };
        state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 3,
                item: SeqItem::Unknown {
                    tag: 7,
                    bytes: vec![0x01],
                },
            }))
            .unwrap();

        let mut tr = Transaction::new(&state);
        let before_ops = tr.ops_for_test().len();
        let before_projected = Arc::clone(&tr.state().projected);
        let result = tr.move_node(p1, root, 1);
        assert!(
            matches!(&result, Err(StepError::UnknownBearingMove { block }) if *block == p1),
            "unknown 포함 move는 전용 거부 변형으로 거부 — {result:?}"
        );
        assert_eq!(
            tr.ops_for_test().len(),
            before_ops,
            "거부 시 op 무발행 (무손실)"
        );
        assert!(
            Arc::ptr_eq(&before_projected, &tr.state().projected),
            "가드가 첫 op보다 먼저이므로 투영 상태 무변이"
        );
    }

    #[test]
    fn move_node_rejects_nested_unknown_block_losslessly() {
        let (mut state, root, bq, _) = state! {
            doc { root: root {
                bq: blockquote { p1: paragraph { text("a") } }
                paragraph { text("x") }
            } }
            selection: (p1, 0)
        };
        let pos =
            support::child_seq_insert_pos(&state.projected, bq, 1, NodeType::Paragraph).unwrap();
        state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos,
                item: SeqItem::Block {
                    node_type: NodeType::Unknown,
                    parents: vec![root, bq],
                    attrs: vec![],
                },
            }))
            .unwrap();

        let mut tr = Transaction::new(&state);
        let before_ops = tr.ops_for_test().len();
        let before_projected = Arc::clone(&tr.state().projected);
        let result = tr.move_node(bq, root, 1);
        assert!(
            matches!(&result, Err(StepError::UnknownBearingMove { block }) if *block == bq),
            "중첩 Unknown 블록 포함 move는 전용 거부 변형으로 거부 — {result:?}"
        );
        assert_eq!(
            tr.ops_for_test().len(),
            before_ops,
            "거부 시 op 무발행 (무손실)"
        );
        assert!(
            Arc::ptr_eq(&before_projected, &tr.state().projected),
            "가드가 첫 op보다 먼저이므로 투영 상태 무변이"
        );
    }

    #[test]
    fn move_node_rejects_unknown_bearing_atom_losslessly() {
        let (mut state, root, p1) = state! {
            doc { root: root {
                p1: paragraph { text("ab") }
                paragraph { text("") }
            } }
            selection: (p1, 0)
        };
        let pos =
            support::child_seq_insert_pos(&state.projected, p1, 2, NodeType::HardBreak).unwrap();
        state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos,
                item: SeqItem::BlockAtom {
                    leaf: AtomLeaf::Unknown(UnknownNode),
                    parents: vec![root, p1],
                },
            }))
            .unwrap();

        let mut tr = Transaction::new(&state);
        let before_ops = tr.ops_for_test().len();
        let before_projected = Arc::clone(&tr.state().projected);
        let result = tr.move_node(p1, root, 1);
        assert!(
            matches!(&result, Err(StepError::UnknownBearingMove { block }) if *block == p1),
            "unknown-bearing atom 포함 move는 전용 거부 변형으로 거부 — {result:?}"
        );
        assert_eq!(
            tr.ops_for_test().len(),
            before_ops,
            "거부 시 op 무발행 (무손실)"
        );
        assert!(
            Arc::ptr_eq(&before_projected, &tr.state().projected),
            "가드가 첫 op보다 먼저이므로 투영 상태 무변이"
        );
    }

    #[test]
    fn move_node_pairs_nested_blocks_and_chars_across_full_hierarchy() {
        let (state, root, bq, ..) = state! {
            doc { root: root {
                bq: blockquote {
                    p1: paragraph { text("a") }
                    p2: paragraph { text("b") }
                }
                paragraph { text("x") }
            } }
            selection: (bq, 0)
        };
        let before_items = collect_items(&state.projected, bq);

        let mut tr = Transaction::new(&state);
        tr.move_node(bq, root, 1).unwrap();
        let ops = tr.ops_for_test();
        let alias_ops = alias_ops_in(&ops);
        assert_eq!(alias_ops.len(), 1);
        let total: u32 = alias_ops[0].pairs.iter().map(|r| r.len).sum();
        assert_eq!(
            total as usize,
            before_items.len(),
            "blockquote + 2 paragraph + 2 char 전 계층 페어링"
        );

        assert_pairs_match_content(&before_items, &tr.state().projected, &alias_ops[0].pairs);
    }

    #[test]
    fn move_node_pairs_inline_atom_dot() {
        let (state, root, p1) = state! {
            doc { root: root {
                p1: paragraph { text("a") tab text("b") }
                paragraph { text("") }
            } }
            selection: (p1, 0)
        };
        let before_items = collect_items(&state.projected, p1);

        let mut tr = Transaction::new(&state);
        tr.move_node(p1, root, 1).unwrap();
        let ops = tr.ops_for_test();
        let alias_ops = alias_ops_in(&ops);
        assert_eq!(alias_ops.len(), 1);
        let total: u32 = alias_ops[0].pairs.iter().map(|r| r.len).sum();
        assert_eq!(
            total as usize,
            before_items.len(),
            "block + char 'a' + tab atom + char 'b' 전부 페어링"
        );

        assert_pairs_match_content(&before_items, &tr.state().projected, &alias_ops[0].pairs);
    }

    #[test]
    fn move_node_moves_inline_atom_slot() {
        let (state, p1) = state! {
            doc { root {
                p1: paragraph { text("a") tab text("b") }
            } }
            selection: (p1, 0)
        };
        let tab = state
            .projected
            .block_children(p1)
            .unwrap()
            .into_iter()
            .find_map(|child| match child {
                Child::Leaf {
                    id,
                    item: SeqItem::Atom(AtomLeaf::Tab),
                } => Some(id),
                _ => None,
            })
            .expect("tab atom");

        let mut tr = Transaction::new(&state);
        tr.move_node(tab, p1, 0).unwrap();

        let children = tr.state().projected.block_children(p1).unwrap();
        assert!(matches!(
            children.first(),
            Some(Child::Leaf {
                item: SeqItem::Atom(AtomLeaf::Tab),
                ..
            })
        ));
        assert_eq!(block_text(tr.state(), &p1), "ab");
        assert_eq!(alias_ops_in(&tr.ops_for_test()).len(), 1);
    }
}
