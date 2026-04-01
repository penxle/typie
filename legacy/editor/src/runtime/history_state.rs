use crate::runtime::{ORIGIN_REMOTE_PREFIX, Runtime};
use crate::state::Selection;
use loro::{LoroValue, UndoItemMeta, UndoManager, UndoOrRedo};
use rustc_hash::FxHashMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

pub(in crate::runtime) const UNDO_MERGE_INTERVAL_MS: i64 = 1000;
static NEXT_HISTORY_UNDO_META_MARKER: AtomicI64 = AtomicI64::new(1);

fn next_history_undo_meta() -> UndoItemMeta {
    let mut meta = UndoItemMeta::new();
    let marker = NEXT_HISTORY_UNDO_META_MARKER.fetch_add(1, Ordering::Relaxed);
    meta.set_value(LoroValue::I64(marker));
    meta
}

#[derive(Clone)]
pub(in crate::runtime) struct HistorySelectionSnapshot {
    pub(in crate::runtime) selection: Selection,
}

#[derive(Clone, Copy)]
struct HistoryPopEvent {
    direction: UndoOrRedo,
    marker: i64,
}

#[derive(Default)]
pub(in crate::runtime) struct RuntimeHistory {
    pub(in crate::runtime) undo_selection_by_marker: FxHashMap<i64, HistorySelectionSnapshot>,
    pub(in crate::runtime) redo_selection_by_marker: FxHashMap<i64, HistorySelectionSnapshot>,
    pub(in crate::runtime) undo_marker_order: VecDeque<i64>,
    pub(in crate::runtime) redo_marker_order: VecDeque<i64>,
    pub(in crate::runtime) pending_history_selection: Option<HistorySelectionSnapshot>,
    pub(in crate::runtime) pending_history_group_start_selection: Option<HistorySelectionSnapshot>,
    pub(in crate::runtime) split_next_history_group: bool,
    pub(in crate::runtime) has_temp_merge_interval_override: bool,
    history_pop_events: Arc<Mutex<Vec<HistoryPopEvent>>>,
}

pub(in crate::runtime) struct HistoryFlushContext {
    undo_count_before: usize,
    top_undo_marker_before: Option<i64>,
    pending_selection: Option<HistorySelectionSnapshot>,
    group_start_selection: Option<HistorySelectionSnapshot>,
}

impl RuntimeHistory {
    pub(in crate::runtime) fn new() -> Self {
        Self {
            history_pop_events: Arc::new(Mutex::new(Vec::new())),
            ..Self::default()
        }
    }

    pub(in crate::runtime) fn configure_undo_manager(&self, undo_manager: &mut UndoManager) {
        undo_manager.set_merge_interval(UNDO_MERGE_INTERVAL_MS);
        undo_manager.set_on_push(Some(Box::new(|_, _, _| next_history_undo_meta())));
        undo_manager.add_exclude_origin_prefix(ORIGIN_REMOTE_PREFIX);

        let history_pop_events_for_callback = self.history_pop_events.clone();
        undo_manager.set_on_pop(Some(Box::new(move |direction, _, meta| {
            let LoroValue::I64(marker) = meta.value else {
                return;
            };
            if let Ok(mut events) = history_pop_events_for_callback.lock() {
                events.push(HistoryPopEvent { direction, marker });
            }
        })));

        undo_manager.clear();
    }
}

impl Runtime {
    pub(in crate::runtime) fn begin_history_flush(&mut self) -> HistoryFlushContext {
        HistoryFlushContext {
            undo_count_before: self.undo_manager.undo_count(),
            top_undo_marker_before: self.top_undo_marker(),
            pending_selection: self.history.pending_history_selection.take(),
            group_start_selection: self.history.pending_history_group_start_selection.take(),
        }
    }

    pub(in crate::runtime) fn finish_history_flush(&mut self, context: HistoryFlushContext) {
        let undo_count_after = self.undo_manager.undo_count();
        let top_undo_marker_after = self.top_undo_marker();
        let pushed_undo_item = undo_count_after > context.undo_count_before
            || (undo_count_after == context.undo_count_before
                && context.top_undo_marker_before != top_undo_marker_after);

        if pushed_undo_item {
            let selection = context.pending_selection.or(context.group_start_selection);
            debug_assert!(
                selection.is_some(),
                "flush pushed an undo item without a pending selection snapshot",
            );
            if let (Some(marker), Some(selection)) = (top_undo_marker_after, selection) {
                self.history_record_undo_snapshot(marker, selection);
            }
        }

        if self.history.has_temp_merge_interval_override {
            self.undo_manager.set_merge_interval(UNDO_MERGE_INTERVAL_MS);
            self.history.has_temp_merge_interval_override = false;
        }

        self.sync_history_selection_state();
    }

    pub(in crate::runtime) fn schedule_split_next_history_group(&mut self) {
        self.history.split_next_history_group = true;
    }

    pub(in crate::runtime) fn history_record_undo_snapshot(
        &mut self,
        marker: i64,
        snapshot: HistorySelectionSnapshot,
    ) {
        self.history.undo_marker_order.push_back(marker);
        self.history
            .undo_selection_by_marker
            .insert(marker, snapshot);
    }

    pub(in crate::runtime) fn history_record_redo_snapshot(
        &mut self,
        marker: i64,
        snapshot: HistorySelectionSnapshot,
    ) {
        self.history.redo_marker_order.push_back(marker);
        self.history
            .redo_selection_by_marker
            .insert(marker, snapshot);
    }

    pub(in crate::runtime) fn history_take_undo_snapshot(
        &mut self,
        marker: i64,
    ) -> Option<HistorySelectionSnapshot> {
        Self::remove_history_marker(&mut self.history.undo_marker_order, marker);
        self.history.undo_selection_by_marker.remove(&marker)
    }

    pub(in crate::runtime) fn history_take_redo_snapshot(
        &mut self,
        marker: i64,
    ) -> Option<HistorySelectionSnapshot> {
        Self::remove_history_marker(&mut self.history.redo_marker_order, marker);
        self.history.redo_selection_by_marker.remove(&marker)
    }

    pub(in crate::runtime) fn sync_history_selection_state(&mut self) {
        let undo_count = self.undo_manager.undo_count();
        let redo_count = self.undo_manager.redo_count();

        while self.history.undo_marker_order.len() > undo_count {
            if let Some(marker) = self.history.undo_marker_order.pop_front() {
                self.history.undo_selection_by_marker.remove(&marker);
            }
        }

        while self.history.redo_marker_order.len() > redo_count {
            if let Some(marker) = self.history.redo_marker_order.pop_front() {
                self.history.redo_selection_by_marker.remove(&marker);
            }
        }
    }

    pub(in crate::runtime) fn capture_history_selection(
        &self,
        selection: Selection,
    ) -> HistorySelectionSnapshot {
        let selection = self.validate_selection(selection);
        HistorySelectionSnapshot { selection }
    }

    pub(in crate::runtime) fn resolve_history_selection(
        &self,
        snapshot: &HistorySelectionSnapshot,
    ) -> Selection {
        let current = self.validate_selection(self.state.selection);
        let raw = snapshot.selection;
        let anchor = if self.doc().node(raw.anchor.node_id).is_some() {
            raw.anchor
        } else {
            current.anchor
        };
        let head = if self.doc().node(raw.head.node_id).is_some() {
            raw.head
        } else {
            current.head
        };

        self.validate_selection(Selection::new(anchor, head))
    }

    pub(in crate::runtime) fn clear_history_pop_events(&self) {
        if let Ok(mut events) = self.history.history_pop_events.lock() {
            events.clear();
        }
    }

    pub(in crate::runtime) fn take_history_pop_markers(&self, direction: UndoOrRedo) -> Vec<i64> {
        let Ok(mut events) = self.history.history_pop_events.lock() else {
            return Vec::new();
        };

        let mut popped_markers = Vec::new();
        let mut remaining = Vec::new();

        for event in events.drain(..) {
            if event.direction == direction {
                popped_markers.push(event.marker);
            } else {
                remaining.push(event);
            }
        }

        *events = remaining;
        popped_markers
    }

    pub(in crate::runtime) fn remove_history_marker(order: &mut VecDeque<i64>, marker: i64) {
        if order.back().copied() == Some(marker) {
            let _ = order.pop_back();
            return;
        }

        if let Some(index) = order.iter().position(|existing| *existing == marker) {
            let _ = order.remove(index);
        }
    }

    pub(in crate::runtime) fn top_undo_marker(&self) -> Option<i64> {
        match self.undo_manager.top_undo_value() {
            Some(LoroValue::I64(value)) => Some(value),
            _ => None,
        }
    }

    pub(in crate::runtime) fn top_redo_marker(&self) -> Option<i64> {
        match self.undo_manager.top_redo_value() {
            Some(LoroValue::I64(value)) => Some(value),
            _ => None,
        }
    }

    pub(in crate::runtime) fn capture_history_for_pending_doc_change(
        &mut self,
        snapshot: HistorySelectionSnapshot,
    ) {
        if self.history.split_next_history_group {
            self.undo_manager.set_merge_interval(0);
            self.history.has_temp_merge_interval_override = true;
            self.history.split_next_history_group = false;
        }

        if self.history.pending_history_group_start_selection.is_none() {
            self.history.pending_history_group_start_selection = Some(snapshot.clone());
        }

        if self.history.pending_history_selection.is_none() {
            self.history.pending_history_selection = Some(snapshot);
        }

        self.history.redo_selection_by_marker.clear();
        self.history.redo_marker_order.clear();
    }

    #[cfg(test)]
    pub fn history_undo_marker_len(&self) -> usize {
        self.history.undo_marker_order.len()
    }

    #[cfg(test)]
    pub fn history_redo_marker_len(&self) -> usize {
        self.history.redo_marker_order.len()
    }

    #[cfg(test)]
    pub fn history_has_pending_selection(&self) -> bool {
        self.history.pending_history_selection.is_some()
    }

    #[cfg(test)]
    pub fn history_has_pending_group_start_selection(&self) -> bool {
        self.history.pending_history_group_start_selection.is_some()
    }

    #[cfg(test)]
    pub fn history_drop_pending_selection_for_test(&mut self) {
        self.history.pending_history_selection = None;
    }

    #[cfg(test)]
    pub fn history_clear_undo_snapshots_for_test(&mut self) {
        self.history.undo_selection_by_marker.clear();
        self.history.undo_marker_order.clear();
    }

    #[cfg(test)]
    pub fn history_first_undo_marker(&self) -> Option<i64> {
        self.history.undo_marker_order.front().copied()
    }

    #[cfg(test)]
    pub fn history_undo_selection_for_marker(&self, marker: i64) -> Option<Selection> {
        self.history
            .undo_selection_by_marker
            .get(&marker)
            .map(|snapshot| snapshot.selection)
    }
}
