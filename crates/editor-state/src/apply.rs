use std::sync::Arc;

use editor_crdt::{Changeset, CrdtError, Dot, ListOp, Op};
use editor_model::EditOp;
use hashbrown::HashSet;

use crate::Selection;
use crate::undo::{RecordedOp, capture_prior};
use crate::{Composition, PendingModifiers, State, StateError};

pub struct BatchedState<'a> {
    inner: &'a mut State,
    pub(crate) emitted: Vec<RecordedOp>,
    defer_deletes: bool,
    deferred: usize,
}

impl<'a> std::ops::Deref for BatchedState<'a> {
    type Target = State;
    fn deref(&self) -> &State {
        self.inner
    }
}

impl<'a> BatchedState<'a> {
    pub fn apply(&mut self, payload: EditOp) -> Result<Op<EditOp>, StateError> {
        // Capture the prior value (for op-level undo) against the pre-op state,
        // then apply. Seq ops carry no prior, so this is a cheap match.
        let prior = capture_prior(&self.inner.projected, &payload);
        let pm = self.inner.projected_mut();
        let op = if self.defer_deletes && matches!(payload, EditOp::Seq(ListOp::Del { .. })) {
            self.deferred += 1;
            pm.apply_warm_only(payload)?
        } else {
            if self.deferred > 0 {
                // A non-delete arrived mid-deferral; its projection pass reads the
                // tree, so the pending deletions must land first.
                pm.reproject_after_delete()?;
                self.deferred = 0;
            }
            pm.apply(payload)?
        };
        self.emitted.push(RecordedOp {
            op: op.clone(),
            prior,
        });
        Ok(op)
    }

    /// Defer the projection of subsequent sequence-delete ops: each applies
    /// warm-only, leaving the projection stale until [`deferred_deletes`]
    /// (Self::deferred_deletes) are flushed by one
    /// [`ProjectedState::reproject_after_delete`]. Only sound for delete-only
    /// batches (that reprojection carries the pre-delete indexes forward); a
    /// non-delete op arriving while deferred flushes automatically before it
    /// projects.
    pub fn set_defer_deletes(&mut self, on: bool) {
        self.defer_deletes = on;
    }

    /// How many deletes were applied warm-only and still await a flush.
    pub fn deferred_deletes(&self) -> usize {
        self.deferred
    }

    /// Count of ops emitted so far in this batch — a checkpoint a caller can
    /// pair with [`emitted_dots_since`](Self::emitted_dots_since) to learn which
    /// op dots a single step's `apply_to` produced, without threading that
    /// bookkeeping through every step.
    pub fn emitted_len(&self) -> usize {
        self.emitted.len()
    }

    /// The dots of ops emitted since the `emitted_len()` checkpoint `from`.
    pub fn emitted_dots_since(&self, from: usize) -> Vec<Dot> {
        self.emitted[from..].iter().map(|r| r.op.id).collect()
    }

    pub fn set_selection(&mut self, selection: Option<Selection>) {
        self.inner.selection = selection;
    }

    pub fn set_pending_modifiers(&mut self, pending: PendingModifiers) {
        self.inner.pending_modifiers = pending;
    }

    pub fn set_composition(&mut self, composition: Option<Composition>) {
        self.inner.composition = composition;
    }
}

impl State {
    pub fn apply(&self, payload: EditOp) -> Result<(Self, Op<EditOp>), StateError> {
        let mut next = self.clone();
        let op = next.projected_mut().apply(payload)?;
        Ok((next, op))
    }

    pub fn receive_remote_changeset(
        &self,
        changeset: Changeset<EditOp>,
    ) -> Result<(Self, Vec<Op<EditOp>>), StateError> {
        self.receive_remote_changesets(vec![changeset])
    }

    pub fn receive_remote_changesets(
        &self,
        css: Vec<Changeset<EditOp>>,
    ) -> Result<(Self, Vec<Op<EditOp>>), StateError> {
        let (next_projected, applied) = self.projected.receive_changesets(css)?;
        if applied.is_empty() {
            return Ok((self.clone(), applied));
        }
        let mut next = self.clone();
        next.projected = Arc::new(next_projected);
        Ok((next, applied))
    }

    pub fn would_receive_remote_changeset(
        &self,
        changeset: &Changeset<EditOp>,
    ) -> Result<bool, StateError> {
        if changeset
            .ops
            .iter()
            .all(|op| self.projected.graph().contains(&op.id))
        {
            return Ok(false);
        }
        let (_next, ops) = self.receive_remote_changeset(changeset.clone())?;
        Ok(!ops.is_empty())
    }

    pub fn local_changesets_since(
        &self,
        remote_heads: &HashSet<Dot>,
    ) -> Result<Vec<Changeset<EditOp>>, CrdtError> {
        self.projected.graph().local_changesets_since(remote_heads)
    }

    pub fn missing_changesets_tolerant(
        &self,
        remote_heads: &HashSet<Dot>,
    ) -> Vec<Changeset<EditOp>> {
        self.projected
            .graph()
            .missing_changesets_tolerant(remote_heads)
    }

    pub fn receive_changesets_ordered(
        &self,
        css: Vec<Changeset<EditOp>>,
    ) -> (Self, Vec<Changeset<EditOp>>) {
        let (graph, dropped) = self.projected.graph().receive_changesets_ordered(css);
        let projected = crate::projected_state::ProjectedState::from_graph(graph)
            .expect("merged graph projects");
        let mut next = self.clone();
        next.projected = Arc::new(projected);
        (next, dropped)
    }

    pub fn partition_ready_indices(&self, css: &[Changeset<EditOp>]) -> (Vec<usize>, Vec<usize>) {
        self.projected.graph().partition_ready_indices(css)
    }

    pub fn batch_with_ops<F, E>(&self, f: F) -> Result<(Self, Vec<Op<EditOp>>), E>
    where
        F: FnOnce(&mut BatchedState) -> Result<(), E>,
        E: From<StateError>,
    {
        let mut next = self.clone();
        let ops = {
            let mut batched = BatchedState {
                inner: &mut next,
                emitted: Vec::new(),
                defer_deletes: false,
                deferred: 0,
            };
            f(&mut batched)?;
            std::mem::take(&mut batched.emitted)
                .into_iter()
                .map(|r| r.op)
                .collect()
        };
        Ok((next, ops))
    }

    /// In-place variant of `batch_with_ops` for callers that already own a
    /// mutable State (e.g. inside `Transaction`). Returns the applied ops with
    /// their captured prior values (for op-level undo). Skips the per-call
    /// `self.clone()` since the caller's state is already isolated. On error the
    /// state is left mutated; callers discard it (Transaction is dropped without
    /// commit, so the editor's authoritative state is unaffected).
    pub fn batch_with_recorded_mut<F, E>(&mut self, f: F) -> Result<Vec<RecordedOp>, E>
    where
        F: FnOnce(&mut BatchedState) -> Result<(), E>,
        E: From<StateError>,
    {
        let recorded = {
            let mut batched = BatchedState {
                inner: self,
                emitted: Vec::new(),
                defer_deletes: false,
                deferred: 0,
            };
            f(&mut batched)?;
            std::mem::take(&mut batched.emitted)
        };
        Ok(recorded)
    }

    pub fn batch<F, E>(&self, f: F) -> Result<Self, E>
    where
        F: FnOnce(&mut BatchedState) -> Result<(), E>,
        E: From<StateError>,
    {
        self.batch_with_ops(f).map(|(state, _ops)| state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::ListOp;
    use editor_model::SeqItem;

    fn para_dot(state: &State) -> editor_crdt::Dot {
        state
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap()
    }

    fn seq_char(pos: usize, c: char) -> EditOp {
        EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Char(c),
        })
    }

    #[test]
    fn batch_applies_chars_and_collects_ops() {
        let state = State::empty();
        let para = para_dot(&state);
        let (next, ops): (State, Vec<Op<EditOp>>) = state
            .batch_with_ops(|b| {
                b.apply(seq_char(1, 'a'))?;
                b.apply(seq_char(2, 'b'))?;
                Ok::<(), StateError>(())
            })
            .unwrap();
        assert_eq!(ops.len(), 2);
        assert_eq!(next.view().node(para).unwrap().inline_text(), "ab");
    }

    #[test]
    fn batch_does_not_mutate_original() {
        let state = State::empty();
        let para = para_dot(&state);
        let _next = state
            .batch(|b| {
                b.apply(seq_char(1, 'x'))?;
                Ok::<(), StateError>(())
            })
            .unwrap();
        assert_eq!(state.view().node(para).unwrap().inline_text(), "");
    }

    #[test]
    fn missing_changesets_tolerant_returns_all_unconfirmed_without_actor_filter() {
        let mut authored = State::empty();
        authored.projected_mut().apply(seq_char(1, 'z')).unwrap();
        authored.projected_mut().commit();
        let empty: HashSet<Dot> = HashSet::new();
        let css = authored.missing_changesets_tolerant(&empty);
        assert!(!css.is_empty(), "unconfirmed local changes are returned");
    }

    #[test]
    fn receive_changesets_ordered_merges_pending_into_state() {
        let mut src = State::empty();
        src.projected_mut().commit();
        let para = src
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .dot()
            .unwrap();
        src.projected_mut().apply(seq_char(1, 'h')).unwrap();
        src.projected_mut().apply(seq_char(2, 'i')).unwrap();
        src.projected_mut().commit();

        let base = State::empty();
        let base_heads: HashSet<Dot> = base.projected.graph().current_heads().copied().collect();
        let pending = src.missing_changesets_tolerant(&base_heads);

        let (merged, dropped) = base.receive_changesets_ordered(pending);
        assert!(dropped.is_empty(), "all pending applies onto matching base");
        assert_eq!(merged.view().node(para).unwrap().inline_text(), "hi");
    }

    #[test]
    fn would_receive_is_false_for_already_contained_ops() {
        let mut authored = State::empty();
        authored.projected_mut().apply(seq_char(1, 'z')).unwrap();
        authored.projected_mut().commit();
        let css = authored.local_changesets_since(&HashSet::new()).unwrap();
        assert!(!css.is_empty());
        for cs in css {
            assert!(
                !authored.would_receive_remote_changeset(&cs).unwrap(),
                "authored already contains every op in its own changesets"
            );
        }
    }
}
