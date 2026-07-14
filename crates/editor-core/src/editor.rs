use editor_clipboard::Slice;
use editor_common::{HistoryTag, Movement, time::Duration};
use editor_crdt::{Changeset, CrdtError, Dot, Op};
use editor_model::{EditOp, ModifierState, ModifierType};
use editor_renderer::{Mark, MarkData, RenderSink, Renderer, damage::IRect};
#[cfg(any(test, feature = "test-utils"))]
use editor_resource::ThemeVariant;
use editor_resource::{CharacterCount, Resource, count_text};
use editor_state::{
    LayoutDirty, Position, ResolvedPosition, ResolvedPositionFlatExt, Selection, StableSelection,
    State, closest_empty_paragraph_break_end_between, farther_endpoint, is_unit_node_selection,
    remap_selection,
};
use editor_transaction::{Effect, HistoryMeta, MergeKind, StepError, Transaction};
use editor_view::{GapPhantom, PageRect, PendingOverlay, View, Viewport};
use hashbrown::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use strum::IntoEnumIterator;

use crate::block_state::BlockState;
use crate::dnd::DndState;
use crate::error::EditorError;
use crate::event::{EditorEvent, FontData};
use crate::handle;
use crate::ime::{Ime, ImeRange};
use crate::message::*;
use crate::state_field::StateField;
use crate::tracked_range::TrackedRangeRegistry;
use editor_common::time::Instant;
use editor_state::undo::{RecordMerge, TransientState, UndoEntry, UndoHistory};

#[derive(Clone, Copy, Debug)]
enum Mode {
    Apply,
    Probe { changed: bool },
}

fn normalize_pending_overlay(state: &State) -> Option<PendingOverlay> {
    let modifiers = state.pending_modifiers.clone();

    if modifiers.is_empty() {
        return None;
    }
    let selection = state.selection.as_ref()?;
    let view = state.view();
    let textblock = view
        .node(selection.head.node)?
        .ancestors()
        .find(|n| n.spec().is_textblock())?;
    let is_empty = textblock.children().next().is_none();
    if !is_empty {
        return None;
    }
    Some(PendingOverlay {
        node_id: textblock.id(),
        modifiers,
    })
}

fn normalize_gap_phantom(state: &State) -> Option<GapPhantom> {
    use editor_state::{GapCursor, as_gap_cursor};
    let view = state.view();
    let rs = state.selection.as_ref()?.resolve(&view)?;
    match as_gap_cursor(&rs)? {
        GapCursor::BetweenMonolithic { parent, index, .. } => Some(GapPhantom {
            parent: parent.id(),
            index,
        }),
        GapCursor::IsolatingBoundary { host, index, .. } => Some(GapPhantom {
            parent: host.id(),
            index,
        }),
    }
}

/// Snapshot of the transient (non-document) editor state recorded with each undo
/// entry so undo/redo restore the caret.
///
/// The caret is captured as a [`StableSelection`] (path + boundary binding) so it
/// survives document restructuring by concurrent remote ops between record and
/// restore; on restore it is re-resolved against the current doc. The selection
/// must be live (resolvable) at capture time — a non-resolving caret is dropped
/// rather than captured, since `StableSelection::capture` requires a live host.
///
/// This walks the host node's children, so it is O(host children). It runs once
/// per recorded entry and once per undo/redo — never on the per-keystroke
/// `undoable` comparison, which uses [`transient_fields_changed`] instead.
fn capture_transient(state: &State) -> TransientState {
    let view = state.view();
    let selection = state
        .selection
        .filter(|s| s.resolve(&view).is_some())
        .map(|s| StableSelection::capture(&s, &view));
    TransientState { selection }
}

/// Whether the transient (non-document) state differs between two states. Used to
/// decide if a selection-only transaction is undoable, without paying for a
/// `StableSelection` capture on the per-keystroke path.
fn transient_fields_changed(before: &State, after: &State) -> bool {
    before.selection != after.selection
}

fn logical_caret_moved(before: Option<Selection>, after: Option<Selection>) -> bool {
    fn key(sel: Option<Selection>) -> Option<(Dot, usize, Dot, usize)> {
        sel.map(|s| (s.anchor.node, s.anchor.offset, s.head.node, s.head.offset))
    }
    key(before) != key(after)
}

fn collapsed_caret(state: &State) -> Option<Position> {
    let sel = state.selection?;
    (sel.anchor == sel.head).then_some(sel.head)
}

fn typing_run(kind: MergeKind, before: &State, after: &State) -> RecordMerge {
    if !matches!(kind, MergeKind::Typing) {
        return RecordMerge::Isolated;
    }
    match (collapsed_caret(before), collapsed_caret(after)) {
        (Some(from), Some(to)) if from.node == to.node => RecordMerge::Typing {
            block: to.node,
            before: from.offset,
            after: to.offset,
        },
        _ => RecordMerge::Isolated,
    }
}

type SelectionMarkRectsCache = Mutex<Option<(Selection, u64, Arc<Vec<PageRect>>)>>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ManifestRequestClass {
    Prefetch,
    Required,
}

pub struct Editor {
    pub(crate) state: State,
    pub(crate) view: View,
    pub(crate) undo_history: UndoHistory,
    pub(crate) renderer: Renderer,
    pub(crate) resource: Arc<Mutex<Resource>>,
    pub(crate) tracked_ranges: TrackedRangeRegistry,

    // drag-and-drop state
    pub(crate) dnd: DndState,

    focused: bool,
    /// Monotonic counter bumped whenever a change can alter rendered page pixels
    /// beyond the selection overlay (doc edits, layout/reflow, font, theme).
    /// Selection-only changes intentionally do NOT bump it, so `page_render_signature`
    /// stays stable for pages whose selection rects are unchanged, letting
    /// `render_surface` skip re-rasterizing them.
    render_epoch: u64,
    message_queue: Vec<Message>,
    pending_events: Vec<EditorEvent>,
    pub(crate) pending_ops: Vec<Op<EditOp>>,
    pending_effects: HashSet<Effect>,
    pub(crate) pending_fonts: HashMap<(String, u16), HashMap<Dot, HashSet<u32>>>,
    pub(crate) requested_manifests: HashMap<(u16, u16), ManifestRequestClass>,
    pub(crate) composition_paint: Option<Vec<editor_model::Modifier>>,
    pub(crate) ime_delete_paint: Option<(usize, Vec<editor_model::Modifier>)>,
    mode: Mode,
    // Selection mark rects for the current (selection, render_epoch), shared by
    // the per-page render signatures and the selection mark. Interaction
    // geometry stays separate because external elements are host-painted but
    // still selectable.
    selection_mark_rects_cache: SelectionMarkRectsCache,
}

struct ProbeGuard<'e> {
    editor: &'e mut Editor,
    prev: Mode,
    dnd: DndState,
    finished: bool,
}

impl<'e> ProbeGuard<'e> {
    fn enter(editor: &'e mut Editor) -> Self {
        let dnd = editor.dnd.clone();
        let prev = std::mem::replace(&mut editor.mode, Mode::Probe { changed: false });
        Self {
            editor,
            prev,
            dnd,
            finished: false,
        }
    }

    fn finish(mut self) -> bool {
        let restored = std::mem::replace(&mut self.editor.mode, self.prev);
        self.editor.dnd = self.dnd.clone();
        self.finished = true;
        let Mode::Probe { changed } = restored else {
            unreachable!("ProbeGuard installed Probe at enter; only finish replaces it")
        };
        changed
    }
}

impl Drop for ProbeGuard<'_> {
    fn drop(&mut self) {
        // panic 등 비정상 경로에서 mode를 안전하게 복원.
        if !self.finished {
            self.editor.mode = self.prev;
            self.editor.dnd = self.dnd.clone();
        }
    }
}

impl Editor {
    pub fn new(state: State, viewport: Viewport, resource: Arc<Mutex<Resource>>) -> Self {
        Self {
            state,
            view: View::new(viewport, Arc::clone(&resource)),
            undo_history: UndoHistory::new(Duration::from_millis(300)),
            renderer: Renderer::new(Arc::clone(&resource)),
            resource,
            tracked_ranges: TrackedRangeRegistry::new(),
            dnd: DndState::default(),
            focused: false,
            render_epoch: 0,
            message_queue: Vec::new(),
            pending_events: Vec::new(),
            pending_ops: Vec::new(),
            pending_effects: HashSet::new(),
            pending_fonts: HashMap::new(),
            requested_manifests: HashMap::new(),
            composition_paint: None,
            ime_delete_paint: None,
            mode: Mode::Apply,
            selection_mark_rects_cache: Mutex::new(None),
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn resource(&self) -> &Arc<Mutex<Resource>> {
        &self.resource
    }

    pub fn tracked_ranges(&self) -> &TrackedRangeRegistry {
        &self.tracked_ranges
    }

    pub(crate) fn tracked_ranges_mut(&mut self) -> &mut TrackedRangeRegistry {
        &mut self.tracked_ranges
    }

    pub fn find_matches(&self, query: &str, options: &SearchOptions) -> Vec<Selection> {
        crate::search::find_matches(&self.state.view(), query, options)
    }

    pub(crate) fn is_probing(&self) -> bool {
        matches!(self.mode, Mode::Probe { .. })
    }

    pub(crate) fn mark_probed_change(&mut self, would_change: bool) {
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= would_change;
        }
    }

    pub fn receive_remote_changeset(&mut self, changeset: Changeset<EditOp>) {
        self.enqueue(Message::Remote { changeset });
    }

    pub fn local_changesets_since(
        &self,
        remote_heads: &HashSet<Dot>,
    ) -> Result<Vec<Changeset<EditOp>>, CrdtError> {
        self.state.local_changesets_since(remote_heads)
    }

    pub fn missing_changesets_tolerant(
        &self,
        remote_heads: &HashSet<Dot>,
    ) -> Vec<Changeset<EditOp>> {
        self.state.missing_changesets_tolerant(remote_heads)
    }

    pub fn partition_ready_indices(&self, css: &[Changeset<EditOp>]) -> (Vec<usize>, Vec<usize>) {
        self.state.partition_ready_indices(css)
    }

    pub fn current_heads(&self) -> Vec<Dot> {
        self.state.graph().current_heads().copied().collect()
    }

    pub fn view(&self) -> &View {
        &self.view
    }

    pub fn modifier_state(&self) -> Option<ModifierState> {
        let sel = self.state.selection.as_ref()?;
        if self.state.composition.is_some()
            && let Some(paint) = &self.composition_paint
        {
            let pending: editor_state::PendingModifiers = paint
                .iter()
                .map(|m| editor_state::PendingModifier::Set {
                    modifier: m.clone(),
                })
                .collect();
            return editor_state::resolve_modifier_state(&self.state.projected, sel, &pending);
        }
        editor_state::resolve_modifier_state(
            &self.state.projected,
            sel,
            &self.state.pending_modifiers,
        )
    }

    /// Selection covering the whole link/ruby span containing `pos`.
    ///
    /// Used when entering edit mode (toolbar button, hover tooltip) to extend a
    /// collapsed caret over the entire mark. Returns `None` when `pos` is not
    /// inside such a span.
    pub fn modifier_span_selection(
        &self,
        pos: &editor_state::Position,
        modifier_type: ModifierType,
    ) -> Option<editor_state::Selection> {
        editor_state::resolve_modifier_span_selection(&self.state, pos, modifier_type)
    }

    pub fn block_state(&self) -> Option<BlockState> {
        crate::block_state::resolve_block_state(&self.state)
    }

    pub fn character_counts(&self) -> (CharacterCount, CharacterCount) {
        let doc = self.state.view();
        let doc_text = editor_state::flat_text(&doc, 0..editor_state::flat_size(&doc));
        let selection_text = Slice::extract(&self.state)
            .map(|s| s.to_text())
            .unwrap_or_default();

        let resource = self.resource.lock().unwrap();
        let doc = count_text(&doc_text, &resource.general_category);
        let selection = count_text(&selection_text, &resource.general_category);
        (doc, selection)
    }

    pub fn interactive_hit_test(
        &self,
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> Option<editor_view::InteractiveHit> {
        self.view.interactive_hit_test(&self.state, page_idx, x, y)
    }

    pub fn page_link_rects(&self, page_idx: usize) -> Vec<editor_view::LinkRect> {
        self.view.page_link_rects(page_idx)
    }

    pub fn link_rects(&self) -> Vec<editor_view::LinkRect> {
        self.view.link_rects()
    }

    pub fn link_hit_test(&self, page_idx: usize, x: f32, y: f32) -> Option<editor_view::LinkRect> {
        self.view.link_hit_test(page_idx, x, y)
    }

    pub fn selection_endpoints(&self) -> Option<editor_view::SelectionEndpoints> {
        let doc = self.state.view();
        let resolved = self.state.selection.as_ref()?.resolve(&doc)?;
        self.view.selection_endpoints(&resolved)
    }

    pub fn selection_hit_test(&self, page_idx: usize, x: f32, y: f32) -> bool {
        let doc = self.state.view();
        let Some(resolved) = self.state.selection.as_ref().and_then(|s| s.resolve(&doc)) else {
            return false;
        };
        self.view.selection_hit_test(&resolved, page_idx, x, y)
    }

    pub fn tracked_ranges_at(
        &self,
        page_idx: usize,
        x: f32,
        y: f32,
        group: Option<&str>,
    ) -> Vec<crate::tracked_range::TrackedRangeHit> {
        use crate::tracked_range::{TrackedRange, TrackedRangeHit};
        use editor_state::ResolvedPositionFlatExt;

        let iter: Box<dyn Iterator<Item = &TrackedRange>> = match group {
            Some(g) => Box::new(self.tracked_ranges.iter_group(g)),
            None => Box::new(self.tracked_ranges.iter()),
        };

        let doc = self.state.view();
        let mut scored: Vec<(usize, String, TrackedRangeHit)> = Vec::new();
        for range in iter {
            let Some(sel) = range.locate(&self.state) else {
                continue;
            };
            let Some(resolved) = sel.resolve(&doc) else {
                continue;
            };
            let rects: Vec<editor_view::SelectionRect> = self
                .view
                .selection_rects(&resolved)
                .into_iter()
                .filter(|pr| pr.page_idx == page_idx)
                .collect();
            if rects.is_empty() {
                continue;
            }
            if !rects.iter().any(|pr| pr.rect.contains(x, y)) {
                continue;
            }

            let a = resolved.anchor().to_flat();
            let h = resolved.head().to_flat();
            let chars = a.abs_diff(h);

            let stripped: Vec<editor_view::PageRect> =
                rects.into_iter().map(|pr| pr.without_meta()).collect();
            scored.push((
                chars,
                range.id.clone(),
                TrackedRangeHit {
                    id: range.id.clone(),
                    group: range.group.clone(),
                    rects: stripped,
                },
            ));
        }

        scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        scored.into_iter().map(|(_, _, hit)| hit).collect()
    }

    /// Returns located tracked ranges containing `position`, sorted by shortest range first.
    ///
    /// Containment is start-inclusive and end-exclusive. This lets a cursor at the
    /// start of a range continue that range while keeping the cursor immediately
    /// after the range outside it.
    pub fn tracked_ranges_containing_position(
        &self,
        position: editor_state::Position,
        group: Option<&str>,
    ) -> Vec<crate::tracked_range::TrackedRange> {
        use crate::tracked_range::TrackedRange;
        use editor_state::ResolvedPositionFlatExt;

        let doc = self.state.view();
        let Some(pos) = position.resolve(&doc) else {
            return Vec::new();
        };
        let pos = pos.to_flat();

        let iter: Box<dyn Iterator<Item = &TrackedRange>> = match group {
            Some(g) => Box::new(self.tracked_ranges.iter_group(g)),
            None => Box::new(self.tracked_ranges.iter()),
        };

        let mut scored: Vec<(usize, String, TrackedRange)> = Vec::new();
        for range in iter {
            let Some(sel) = range.locate(&self.state) else {
                continue;
            };
            let Some(resolved) = sel.resolve(&doc) else {
                continue;
            };

            let anchor = resolved.anchor().to_flat();
            let head = resolved.head().to_flat();
            let start = anchor.min(head);
            let end = anchor.max(head);
            if start <= pos && pos < end {
                scored.push((end - start, range.id.clone(), range.clone()));
            }
        }

        scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        scored.into_iter().map(|(_, _, range)| range).collect()
    }

    pub fn cursor_hit_test(&self, page_idx: usize, x: f32, y: f32) -> bool {
        let Some(selection) = self.state.selection else {
            return false;
        };
        if !selection.is_collapsed() {
            return false;
        }

        let Some(hit_selection) = self.view.hit_test(page_idx, x, y) else {
            return false;
        };
        if !hit_selection.is_collapsed() {
            return false;
        }

        let doc = self.state.view();
        let Some(current_head) = selection.head.resolve(&doc) else {
            return false;
        };
        let Some(hit_head) = hit_selection.head.resolve(&doc) else {
            return false;
        };
        current_head == hit_head
    }

    pub fn pointer_style(
        &self,
        page_idx: usize,
        x: f32,
        y: f32,
        read_only: bool,
    ) -> Option<editor_view::PointerStyle> {
        self.view
            .pointer_style_at(&self.state, page_idx, x, y, read_only)
    }

    pub fn ime(&self, before_limit: usize, after_limit: usize) -> Result<Option<Ime>, EditorError> {
        let state = self.state();
        let doc = state.view();
        let doc_size = editor_state::flat_size(&doc);

        let Some(sel) = state.selection else {
            return Ok(None);
        };
        let anchor_flat = sel
            .anchor
            .resolve(&doc)
            .ok_or_else(|| EditorError::General {
                msg: "invariant violated: state.selection.anchor must resolve against state.doc"
                    .into(),
            })?
            .to_flat();
        let head_flat = sel
            .head
            .resolve(&doc)
            .ok_or_else(|| EditorError::General {
                msg: "invariant violated: state.selection.head must resolve against state.doc"
                    .into(),
            })?
            .to_flat();
        let (sel_start, sel_end) = (anchor_flat.min(head_flat), anchor_flat.max(head_flat));

        let window_start = sel_start.saturating_sub(before_limit);
        let window_end = sel_end.saturating_add(after_limit).min(doc_size);

        let text = editor_state::flat_text(&doc, window_start..window_end);
        let composing = state.composition.map(|c| ImeRange {
            start: c.start,
            end: c.end,
        });

        Ok(Some(Ime {
            text,
            window_start,
            selection: ImeRange {
                start: sel_start,
                end: sel_end,
            },
            composing,
        }))
    }

    pub fn enqueue(&mut self, msg: Message) {
        self.message_queue.push(msg);
    }

    pub fn tick(&mut self) -> Result<Vec<EditorEvent>, EditorError> {
        let old_selection = self.state.selection;
        let old_pending_modifiers = self.state.pending_modifiers.clone();
        let old_composition = self.state.composition;
        let old_composition_paint = self.composition_paint.clone();
        let old_last_history_tag_revision = self.undo_history.last_tag_revision();

        let messages = std::mem::take(&mut self.message_queue);
        // Coalesce a consecutive run of remote changesets into one batched receive:
        // a sync burst enqueues many `Message::Remote` back-to-back, and applying
        // each separately re-clones the whole projected state (O(N)) per changeset.
        let mut iter = messages.into_iter().peekable();
        while let Some(msg) = iter.next() {
            match msg {
                Message::Remote { changeset } => {
                    self.ime_delete_paint = None;
                    let mut batch = vec![changeset];
                    while matches!(iter.peek(), Some(Message::Remote { .. })) {
                        if let Some(Message::Remote { changeset }) = iter.next() {
                            batch.push(changeset);
                        }
                    }
                    self.apply_remote_changesets(batch)?;
                }
                // Coalesce a consecutive run of text-input batches into one reduce +
                // transact: an IME composition update enqueues several `TextInput`
                // messages per frame, and each separately rebuilds the flat window
                // and re-derives the edit. A batch containing `CommitAsIs` ends the
                // run — the commit's text replacement must see the committed text
                // before a following batch reopens a composition (it no-ops while
                // one is active).
                Message::TextInput { ops } => {
                    let mut batch = ops;
                    while matches!(iter.peek(), Some(Message::TextInput { .. }))
                        && !batch.iter().any(|op| matches!(op, FlatImeOp::CommitAsIs))
                    {
                        if let Some(Message::TextInput { ops }) = iter.next() {
                            batch.extend(ops);
                        }
                    }
                    self.process_message(Message::TextInput { ops: batch })?;
                }
                other => {
                    self.ime_delete_paint = None;
                    self.process_message(other)?;
                }
            }
        }
        // Defense-in-depth: every op-applying path seals its own changeset
        // (`Transaction::commit`, `apply_undo_result`). If a future path leaks
        // unsealed ops past this boundary, the sync layer snapshots
        // `current_heads` and permanently skips any changeset sealed later —
        // so seal leftovers here and make the leak loud.
        if !self.state.projected.graph().pending().is_empty() {
            log::warn!("tick sealed leftover unsealed ops; an edit path failed to commit");
            self.state.projected_mut().commit();
        }

        let layout_dirty = self.view.take_layout_dirty(&mut self.state);

        if !self.pending_ops.is_empty() {
            self.augment_font_state_from_ops(&layout_dirty);
        }

        let effects = std::mem::take(&mut self.pending_effects);
        if !effects.is_empty() {
            self.process_effects(effects);
        }

        let ops = std::mem::take(&mut self.pending_ops);
        let pending_overlay = normalize_pending_overlay(&self.state);
        let gap_phantom = normalize_gap_phantom(&self.state);
        // `reconcile` reports view-level change sources (pending overlay, gap phantom,
        // layout fingerprint); applied doc ops also dirty the rendered layout.
        let dirty = self
            .view
            .reconcile(&self.state, layout_dirty, pending_overlay, gap_phantom)
            || !ops.is_empty();

        let mut fields: HashSet<StateField> = HashSet::new();

        if dirty {
            fields.insert(StateField::Cursor);
            fields.insert(StateField::PageSizes);
            fields.insert(StateField::ExternalElements);
            fields.insert(StateField::TableOverlays);
            fields.insert(StateField::LinkRects);
            fields.insert(StateField::Placeholder);
            self.invalidate_render();
        }

        if !ops.is_empty() {
            fields.insert(StateField::Doc);
            fields.insert(StateField::TableOverlays);
            fields.insert(StateField::LinkRects);
            fields.insert(StateField::Ime);
            fields.insert(StateField::Modifiers);
            fields.insert(StateField::Block);
            fields.insert(StateField::Placeholder);
            if !self.tracked_ranges.is_empty() {
                fields.insert(StateField::TrackedRanges);
            }
        }

        if ops
            .iter()
            .any(|op| matches!(&op.payload, EditOp::NodeAttr(attr) if attr.target == Dot::ROOT))
        {
            fields.insert(StateField::Doc);
            fields.insert(StateField::RootAttrs);
        }

        if old_selection != self.state.selection {
            fields.insert(StateField::Cursor);
            fields.insert(StateField::ExternalElements);
            fields.insert(StateField::TableOverlays);
            fields.insert(StateField::Ime);
            fields.insert(StateField::Selection);
            fields.insert(StateField::Modifiers);
            fields.insert(StateField::Block);
            // Selection-only invalidation: deliberately a bare push (NOT
            // `invalidate_render`), so `render_epoch` stays put and pages whose
            // selection rects are unchanged keep a stable signature and get
            // skipped by `render_surface`. This is the drag hot path.
            self.push_event(EditorEvent::RenderInvalidated);
        }

        if old_pending_modifiers != self.state.pending_modifiers {
            fields.insert(StateField::Modifiers);
        }

        if old_composition_paint != self.composition_paint {
            fields.insert(StateField::Modifiers);
        }

        if old_composition != self.state.composition {
            fields.insert(StateField::Ime);
            self.invalidate_render();
        }

        if old_last_history_tag_revision != self.undo_history.last_tag_revision() {
            fields.insert(StateField::LastHistoryTag);
        }

        if !fields.is_empty() {
            self.push_event(EditorEvent::StateChanged {
                fields: fields.into_iter().collect(),
            });
        }

        Ok(std::mem::take(&mut self.pending_events))
    }

    #[cfg(test)]
    pub(crate) fn tracked_decoration_marks_for_test(&self) -> Vec<Mark> {
        let mut marks = Vec::new();
        self.collect_tracked_decoration_marks(&mut marks);
        marks
    }

    fn collect_tracked_decoration_marks(&self, marks: &mut Vec<Mark>) {
        let view_state = self.view.view_state();
        if view_state.tracked_decoration_groups.is_empty() {
            return;
        }
        let mut entries: Vec<(i32, &editor_view::GroupDecoration, Vec<PageRect>)> = Vec::new();
        let doc = self.state.view();
        for range in self.tracked_ranges.iter() {
            let Some(group) = view_state.group_decoration(&range.group) else {
                continue;
            };
            if !group.enabled {
                continue;
            }
            let Some(sel) = range.locate(&self.state) else {
                continue;
            };
            let Some(resolved) = sel.resolve(&doc) else {
                continue;
            };
            let selection_rects: Vec<PageRect> = self
                .view
                .selection_rects(&resolved)
                .iter()
                .map(|r| r.without_meta())
                .collect();
            if selection_rects.is_empty() {
                continue;
            }
            entries.push((group.z_index, group, selection_rects));
        }
        entries.sort_by_key(|(z, _, _)| *z);
        for (_, group, selection_rects) in entries {
            if let Some(theme_key) = group.style.background.clone() {
                marks.push(Mark {
                    data: MarkData::TrackedBackground {
                        theme_key,
                        border_radius: group.style.background_radius.unwrap_or(0.0),
                        vertical_inset: group.style.background_inset.unwrap_or(0.0),
                    },
                    rects: selection_rects.clone(),
                });
            }
            if let Some(underline) = group.style.underline.clone() {
                marks.push(Mark {
                    data: MarkData::TrackedUnderline { underline },
                    rects: selection_rects,
                });
            }
        }
    }

    fn cached_selection_mark_rects(&self) -> Option<Arc<Vec<PageRect>>> {
        let sel = self.state.selection?;
        if let Some((csel, cepoch, rects)) =
            self.selection_mark_rects_cache.lock().unwrap().as_ref()
            && *csel == sel
            && *cepoch == self.render_epoch
        {
            return Some(Arc::clone(rects));
        }
        let doc = self.state.view();
        let resolved = sel.resolve(&doc)?;
        if resolved.is_collapsed() {
            return None;
        }
        let rects: Arc<Vec<PageRect>> = Arc::new(
            self.view
                .selection_mark_rects(&resolved)
                .iter()
                .map(|r| r.without_meta())
                .collect(),
        );
        *self.selection_mark_rects_cache.lock().unwrap() =
            Some((sel, self.render_epoch, Arc::clone(&rects)));
        Some(rects)
    }

    fn selection_mark_rects(&self) -> Option<Vec<PageRect>> {
        Some(self.cached_selection_mark_rects()?.as_ref().clone())
    }

    #[cfg(test)]
    pub(crate) fn cell_selection_rects_for_test(&self) -> Vec<PageRect> {
        self.selection_mark_rects().unwrap_or_default()
    }

    /// Cheap fingerprint of everything `render_page` would draw for `page_idx`.
    /// Equal signatures across two calls guarantee identical page pixels, so
    /// `render_surface` can skip re-rasterizing a page whose signature is unchanged.
    ///
    /// Inputs:
    /// - `render_epoch` — bumped by every non-selection invalidation (doc edits,
    ///   reflow), so it covers content/layout pixel changes and the
    ///   non-selection marks (composition, dnd, tracked decorations).
    /// - `focused` — the selection mark's color depends on it.
    /// - the selection mark rects on this page — the only mark that moves without
    ///   bumping the epoch (the drag hot path).
    /// - `theme.variant()` / `font_generation()` — hashed directly since
    ///   `set_theme_variant`/`set_fonts` mutate resource without bumping the epoch.
    pub fn page_render_signature(&self, page_idx: u32) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.render_epoch.hash(&mut hasher);
        self.focused.hash(&mut hasher);
        if let Some(rects) = self.cached_selection_mark_rects() {
            for r in rects.iter().filter(|r| r.page_idx == page_idx as usize) {
                r.rect.x.to_bits().hash(&mut hasher);
                r.rect.y.to_bits().hash(&mut hasher);
                r.rect.width.to_bits().hash(&mut hasher);
                r.rect.height.to_bits().hash(&mut hasher);
            }
        }
        let res = self.resource.lock().unwrap();
        res.theme.variant().hash(&mut hasher);
        res.font_registry.font_generation().hash(&mut hasher);
        drop(res);
        hasher.finish()
    }

    fn collect_page_marks(&self) -> Vec<Mark> {
        let mut marks: Vec<Mark> = Vec::new();

        // Push before selection so selection draws on top within BelowContent.
        self.collect_tracked_decoration_marks(&mut marks);

        if let Some(rects) = self.selection_mark_rects() {
            marks.push(Mark {
                data: MarkData::Selection {
                    focused: self.focused,
                },
                rects,
            });
        }

        let doc = self.state.view();
        if let Some(composition) = self.state.composition
            && let (Some(from), Some(to)) = (
                ResolvedPosition::from_flat(&doc, composition.start),
                ResolvedPosition::from_flat(&doc, composition.end),
            )
        {
            let rects = self
                .view
                .composition_rects(&Position::from(&from), &Position::from(&to))
                .iter()
                .map(|r| r.without_meta())
                .collect();
            marks.push(Mark {
                data: MarkData::Composition,
                rects,
            });
        }

        if let Some(target) = self.dnd.drop_target() {
            marks.push(Mark {
                data: MarkData::DropIndicator,
                rects: vec![target.indicator.rect()],
            });
        }

        marks
    }

    pub fn render_page(&mut self, page_idx: u32, sink: &mut dyn RenderSink, scale_factor: f32) {
        let marks = self.collect_page_marks();
        let doc = self.state.view();
        self.renderer.render_page(
            sink,
            &doc,
            &self.view,
            page_idx as usize,
            scale_factor,
            &marks,
        );
    }

    /// `None` when `page_idx` is out of range: hosts render pages asynchronously
    /// from document mutations, so a render request may still arrive for a page
    /// an edit just removed.
    pub fn build_display_list(
        &mut self,
        page_idx: u32,
        scale_factor: f32,
    ) -> Option<(editor_renderer::display_list::DisplayList, IRect)> {
        let size = self.view.pages().get(page_idx as usize)?.size;
        let page_bounds = IRect {
            x0: 0,
            y0: 0,
            x1: (size.width * scale_factor).round() as i32,
            y1: (size.height * scale_factor).round() as i32,
        };

        let marks = self.collect_page_marks();
        let doc = self.state.view();
        let mut recorder = editor_renderer::display_list::DisplayListRecorder::new(page_bounds);
        self.renderer.render_page(
            &mut recorder,
            &doc,
            &self.view,
            page_idx as usize,
            scale_factor,
            &marks,
        );
        Some((recorder.into_list(), page_bounds))
    }

    pub fn export_page_vector(&mut self, page_idx: u32, scale_factor: f32) -> Vec<u8> {
        let doc = self.state.view();
        self.renderer
            .export_page_vector(&doc, &self.view, page_idx as usize, scale_factor)
    }

    fn process_message(&mut self, msg: Message) -> Result<(), EditorError> {
        match msg {
            Message::Key { event } => handle::handle_key_event(self, event)?,
            Message::Insertion { op } => handle::handle_insertion_op(self, op)?,
            Message::Deletion { op } => handle::handle_deletion_op(self, op)?,
            Message::Modifier { op } => handle::handle_modifier_op(self, op)?,
            Message::Selection { op } => handle::handle_selection_op(self, op)?,
            Message::Node { op } => handle::handle_node_op(self, op)?,
            Message::Block { op } => handle::handle_block_op(self, op)?,
            Message::List { op } => handle::handle_list_op(self, op)?,
            Message::View { op } => handle::handle_view_op(self, op)?,
            Message::Clipboard { op } => handle::handle_clipboard_op(self, op)?,
            Message::TextInput { ops } => handle::handle_flat_ime_ops(self, ops)?,
            Message::Dnd { op } => handle::handle_dnd_op(self, op)?,
            Message::Navigation { op } => handle::handle_navigation_op(self, op)?,
            Message::History { op } => handle::handle_history_op(self, op)?,
            Message::System { event } => handle::handle_system_event(self, event)?,
            Message::TrackedRange { op } => handle::handle_tracked_range_op(self, op)?,
            Message::Remote { changeset } => handle::handle_remote(self, changeset)?,
        }
        Ok(())
    }

    pub(crate) fn transact(
        &mut self,
        f: impl FnOnce(&mut Transaction) -> Result<(), EditorError>,
    ) -> Result<(), EditorError> {
        self.transact_inner(f, false)
    }

    /// Like [`Editor::transact`], but discards the transaction when it produced
    /// no observable state change. For commands that can legitimately no-op
    /// (e.g. an inline toggle over a range with no applicable target): committing
    /// their recorded ops would push a history entry `editor.can` disagrees
    /// with, and probing with a separate throwaway transaction would run the
    /// whole command twice.
    pub(crate) fn transact_observable(
        &mut self,
        f: impl FnOnce(&mut Transaction) -> Result<(), EditorError>,
    ) -> Result<(), EditorError> {
        self.transact_inner(f, true)
    }

    fn transact_inner(
        &mut self,
        f: impl FnOnce(&mut Transaction) -> Result<(), EditorError>,
        only_if_observable: bool,
    ) -> Result<(), EditorError> {
        let mut tr = Transaction::new(&self.state);
        f(&mut tr)?;

        if !tr.keeps_pending_modifiers()
            && !tr.has_pending_modifiers_step()
            && logical_caret_moved(self.state.selection, tr.state().selection)
        {
            tr.clear_pending_format()?;
        }

        let composition_paint_signal = tr.meta().composition_paint.is_some();

        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= composition_paint_signal
                || editor_state::state_observably_changed(&self.state, tr.state());
            return Ok(());
        }

        if only_if_observable
            && !composition_paint_signal
            && !editor_state::state_observably_changed(&self.state, tr.state())
        {
            return Ok(());
        }

        let prev_composition = self.state.composition;
        let (state, _step_records, recorded, effects, meta) = tr.commit();
        let composition_paint_update = meta.composition_paint;
        let ops: Vec<Op<EditOp>> = recorded.iter().map(|r| r.op.clone()).collect();
        // A Record/Tagged transaction is undoable when it changes the document
        // OR the transient state (selection), matching the old
        // step-history's SetSelection entries; composition-only changes are
        // not recorded. `self.state` is still the pre-transaction state here
        // (reassigned below), so this compares pre vs post; the entry's
        // transient is the *pre* state, captured stably against the
        // pre-transaction view.
        let undoable = !recorded.is_empty() || transient_fields_changed(&self.state, &state);
        let merge = typing_run(meta.merge, &self.state, &state);

        match meta.history {
            HistoryMeta::Skip if undoable => self.undo_history.invalidate_last_tag(),
            HistoryMeta::Skip => self.undo_history.clear_last_tag(),
            HistoryMeta::Record if undoable => self.undo_history.record(
                UndoEntry {
                    ops: recorded,
                    tag: None,
                    transient: capture_transient(&self.state),
                    merge,
                },
                Instant::now(),
            ),
            HistoryMeta::Tagged { tag } if undoable => self.undo_history.record(
                UndoEntry {
                    ops: recorded,
                    tag: Some(tag),
                    transient: capture_transient(&self.state),
                    merge: RecordMerge::Isolated,
                },
                Instant::now(),
            ),
            _ => self.undo_history.clear_last_tag(),
        }

        self.state = state;
        if let Some(paint) = composition_paint_update {
            self.composition_paint = Some(paint);
        }
        if prev_composition.is_some() && self.state.composition.is_none() {
            self.composition_paint = None;
        }
        self.pending_ops.extend(ops);
        self.pending_effects.extend(effects);

        Ok(())
    }

    pub(crate) fn push_event(&mut self, event: EditorEvent) {
        if matches!(self.mode, Mode::Probe { .. }) {
            return;
        }
        let event = match event {
            EditorEvent::StateChanged { mut fields } => {
                fields.sort_unstable();
                EditorEvent::StateChanged { fields }
            }
            other => other,
        };
        if !self.pending_events.contains(&event) {
            self.pending_events.push(event);
        }
    }

    /// Invalidate the rendered surface for a content change (anything that alters
    /// page pixels beyond the selection overlay). Bumps `render_epoch` so every
    /// page's signature changes, then emits `RenderInvalidated`. Use this for all
    /// non-selection invalidations; a bare selection move uses `push_event` directly
    /// so unaffected pages keep a stable signature and can be skipped.
    pub(crate) fn invalidate_render(&mut self) {
        if matches!(self.mode, Mode::Probe { .. }) {
            return;
        }
        self.render_epoch = self.render_epoch.wrapping_add(1);
        self.push_event(EditorEvent::RenderInvalidated);
    }

    fn process_effects(&mut self, effects: HashSet<Effect>) {
        for effect in effects {
            match effect {
                Effect::LoadFont {
                    family,
                    weight,
                    codepoints,
                } => {
                    self.resolve_fonts(&family, weight, &codepoints);
                }
            }
        }
    }

    fn augment_font_state_from_ops(&mut self, layout_dirty: &LayoutDirty) {
        let updates = match layout_dirty {
            LayoutDirty::Full => {
                let resource = self.resource.lock().unwrap();
                let view = self.state.view();
                crate::font::derive_font_updates_from_ops(
                    &view,
                    &resource.font_registry,
                    &self.pending_ops,
                )
            }
            LayoutDirty::Incremental {
                content,
                structural,
            } => {
                let mut fresh = crate::font::FontRequests::new();
                let mut clear_dots: Vec<Dot> = Vec::new();
                {
                    let resource = self.resource.lock().unwrap();
                    let view = self.state.view();
                    let mut seen: HashSet<Dot> = HashSet::new();
                    for id in content.iter().chain(structural.iter()) {
                        let Some(block) = view
                            .node(*id)
                            .or_else(|| view.leaf(*id).and_then(|l| l.parent()))
                        else {
                            continue;
                        };
                        if !seen.insert(block.id()) {
                            continue;
                        }
                        crate::font::collect_subtree_block_dots(&block, &mut clear_dots);
                        crate::font::collect_block_recursive(
                            &block,
                            &resource.font_registry,
                            &mut fresh,
                        );
                    }
                }
                for dot in &clear_dots {
                    for nodes in self.pending_fonts.values_mut() {
                        nodes.remove(dot);
                    }
                }
                self.pending_fonts.retain(|_, nodes| !nodes.is_empty());
                fresh
            }
        };

        self.merge_font_requests(updates);
    }

    fn merge_font_requests(&mut self, updates: crate::font::FontRequests) {
        for ((family, weight), nodes) in updates {
            let entry = self
                .pending_fonts
                .entry((family.clone(), weight))
                .or_default();
            let mut all_cps: HashSet<u32> = HashSet::new();
            for (node_id, cps) in nodes {
                all_cps.extend(cps.iter().copied());
                entry.entry(node_id).or_default().extend(cps);
            }
            self.pending_effects.insert(Effect::LoadFont {
                family,
                weight,
                codepoints: all_cps.into_iter().collect(),
            });
        }
    }

    pub fn can(&mut self, msg: Message) -> Result<bool, EditorError> {
        self.probe(|editor| editor.process_message(msg))
    }

    pub(crate) fn probe(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<(), EditorError>,
    ) -> Result<bool, EditorError> {
        let guard = ProbeGuard::enter(self);
        let result = f(guard.editor);
        let changed = guard.finish();
        result.map(|()| changed)
    }

    pub(crate) fn set_focused(&mut self, focused: bool) {
        let would_change = self.focused != focused;
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= would_change;
            return;
        }
        if would_change {
            self.focused = focused;
            self.invalidate_render();
        }
    }

    pub(crate) fn resize_view(&mut self, viewport: Viewport) -> bool {
        let would_change = self.view.would_resize(viewport, &self.state);
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= would_change;
            return would_change;
        }
        let did_change = self.view.resize(viewport, &self.state);
        if did_change {
            self.push_event(EditorEvent::StateChanged {
                fields: vec![
                    StateField::Cursor,
                    StateField::PageSizes,
                    StateField::ExternalElements,
                    StateField::TableOverlays,
                    StateField::LinkRects,
                    StateField::Placeholder,
                ],
            });
            self.invalidate_render();
        }
        did_change
    }

    pub(crate) fn set_external_height(&mut self, node_id: Dot, height: f32) -> bool {
        let would_change = self
            .view
            .would_set_external_height(&self.state, node_id, height);
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= would_change;
            return would_change;
        }
        let did_change = self.view.set_external_height(&self.state, node_id, height);
        if did_change {
            self.push_event(EditorEvent::StateChanged {
                fields: vec![
                    StateField::Cursor,
                    StateField::PageSizes,
                    StateField::ExternalElements,
                ],
            });
            self.invalidate_render();
        }
        did_change
    }

    pub(crate) fn toggle_fold(&mut self, id: Dot) -> bool {
        let would_change = self.view.would_toggle_fold(&self.state, id);
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= would_change;
            return would_change;
        }
        let did_change = self.view.toggle_fold(&self.state, id);
        if did_change {
            self.push_event(EditorEvent::StateChanged {
                fields: vec![
                    StateField::Cursor,
                    StateField::PageSizes,
                    StateField::ExternalElements,
                ],
            });
            self.invalidate_render();
        }
        did_change
    }

    pub(crate) fn set_fold_expanded(&mut self, id: Dot, expanded: bool) -> bool {
        if self.view.fold_expanded(id) == expanded {
            return false;
        }
        self.toggle_fold(id)
    }

    pub(crate) fn fold_expanded(&self, id: Dot) -> bool {
        self.view.fold_expanded(id)
    }

    pub(crate) fn clear_preferred_x(&mut self) {
        let would_change = self.view.would_clear_preferred_x();
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= would_change;
            return;
        }
        self.view.clear_preferred_x();
    }

    pub(crate) fn ensure_preferred_x_at(&mut self, pos: &Position) {
        let would_change = self.view.would_ensure_preferred_x_at(pos);
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= would_change;
            return;
        }
        self.view.ensure_preferred_x_at(pos);
    }

    pub(crate) fn resolve_movement(
        &mut self,
        pos: &Position,
        movement: &Movement,
    ) -> Option<Selection> {
        let probe = matches!(self.mode, Mode::Probe { .. });
        if probe {
            let current_sel = self.state.selection;
            let current_px = self.view.view_state().preferred_x;
            let probe_result = {
                let resource = self.resource.lock().unwrap();
                self.view.would_resolve_movement(pos, movement, &resource)
            };
            let (target, px_changed) = match probe_result {
                Some((sel, would_px)) => (sel, would_px != current_px),
                None => (None, false),
            };
            let sel = target;
            if let Mode::Probe { ref mut changed } = self.mode {
                *changed |= sel.is_some() && sel != current_sel || px_changed;
            };
            return sel;
        }

        {
            let resource = self.resource.lock().unwrap();
            self.view.resolve_movement(pos, movement, &resource)
        }
    }

    /// Rebinds the latest live selection by identity into the document snapshot
    /// that produced the current layout.
    pub(crate) fn layout_input_state(&self) -> Option<State> {
        let mut state = self.view.layout_state()?.clone();
        state.selection = self
            .state
            .selection
            .and_then(|selection| remap_selection(selection, &self.state, &state));
        Some(state)
    }

    pub(crate) fn resolve_extend_movement(
        &mut self,
        selection: Selection,
        movement_from: Position,
        movement: &Movement,
        state: &State,
    ) -> Option<Selection> {
        let target = self.resolve_movement(&movement_from, movement)?;
        let doc = state.view();
        let current_is_unit = is_unit_node_selection(&selection, &doc);
        let fixed = if current_is_unit {
            farther_endpoint(&doc, &target.head, &selection.anchor, &selection.head)
        } else {
            selection.anchor
        };
        let mut head = farther_endpoint(&doc, &fixed, &target.anchor, &target.head);
        if !current_is_unit
            && matches!(
                movement,
                Movement::Grapheme { .. } | Movement::Word { .. } | Movement::Sentence { .. }
            )
            && let Some(stop) =
                closest_empty_paragraph_break_end_between(&movement_from, &head, &doc)
        {
            head = stop;
        }
        Selection::new(fixed, head).normalize(&doc)
    }

    pub fn last_history_tag(&self) -> Option<HistoryTag> {
        self.undo_history.last_tag().cloned()
    }

    fn apply_undo_result(&mut self, result: Option<(Vec<Op<EditOp>>, TransientState)>) -> bool {
        match result {
            Some((ops, transient)) => {
                // Seal the inverse ops like `Transaction::commit` seals edits:
                // unsealed ops are invisible to `missing_changesets_tolerant`
                // (the sync capture/push source) while still advancing
                // `current_heads`, so leaving them pending until the next edit
                // lets a sync capture skip them permanently.
                if !self.state.projected.graph().pending().is_empty() {
                    self.state.projected_mut().commit();
                }
                // The inverse ops have already been applied to `self.state.projected`,
                // so this re-resolves the recorded `StableSelection` against the
                // post-undo doc. Concurrent remote ops may have restructured the doc
                // since the entry was recorded; re-resolving (rather than restoring a
                // raw position) keeps `state.selection` resolvable. Mirrors the
                // remote-changeset path.
                self.state.selection = transient.selection.and_then(|ss| {
                    let view = self.state.view();
                    let ctx = editor_state::StableResolveCtx::from_live(
                        &view,
                        self.state.projected.seq_checkout(),
                    );
                    let restored = ss.resolve(&ctx)?;
                    Some(restored.normalize(&view).unwrap_or(restored))
                });
                self.state.composition = None;
                self.composition_paint = None;
                self.ime_delete_paint = None;
                self.state.pending_modifiers.clear();
                self.pending_ops.extend(ops);
                true
            }
            None => false,
        }
    }

    pub(crate) fn try_undo(&mut self) -> bool {
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= self.undo_history.can_undo();
            return false;
        }
        let current = capture_transient(&self.state);
        let result = self.undo_history.undo(self.state.projected_mut(), current);
        self.apply_undo_result(result)
    }

    pub(crate) fn try_redo(&mut self) -> bool {
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= self.undo_history.can_redo();
            return false;
        }
        let current = capture_transient(&self.state);
        let result = self.undo_history.redo(self.state.projected_mut(), current);
        self.apply_undo_result(result)
    }

    pub(crate) fn try_undo_auto_replacement(&mut self) -> bool {
        let is_auto = self
            .last_history_tag()
            .is_some_and(|t| matches!(t, HistoryTag::AutoReplacement));
        if !is_auto {
            return false;
        }
        self.try_undo()
    }

    pub(crate) fn apply_remote_changeset(
        &mut self,
        changeset: Changeset<EditOp>,
    ) -> Result<(), EditorError> {
        self.apply_remote_changesets(vec![changeset])
    }

    /// Apply a batch of remote changesets as a single unit. A sync burst enqueues
    /// many `Message::Remote`; the tick loop coalesces the consecutive run and
    /// hands it here so the whole batch pays one selection freeze/restore and one
    /// `ProjectedState` clone instead of one per changeset.
    pub(crate) fn apply_remote_changesets(
        &mut self,
        changesets: Vec<Changeset<EditOp>>,
    ) -> Result<(), EditorError> {
        if changesets.is_empty() {
            return Ok(());
        }
        if let Mode::Probe { ref mut changed } = self.mode {
            for cs in &changesets {
                let would_change = self
                    .state
                    .would_receive_remote_changeset(cs)
                    .map_err(|e| EditorError::Step(StepError::State(e)))?;
                *changed |= would_change;
            }
            return Ok(());
        }

        let frozen = self.state.selection.as_ref().map(|s| {
            let view = self.state.view();
            StableSelection::capture(s, &view)
        });
        let (mut next, applied_ops) = self
            .state
            .receive_remote_changesets(changesets)
            .map_err(|e| EditorError::Step(StepError::State(e)))?;
        // A batch that applied nothing (every changeset already known) leaves the
        // projection byte-identical, so the selection still resolves as-is — skip the
        // `O(N)` `StableResolveCtx` rebuild + restore entirely. This is the duplicate
        // re-receive hot path (a sync burst re-delivering known changesets).
        if !applied_ops.is_empty() {
            next.selection = frozen.and_then(|f| {
                let view = next.view();
                let ctx =
                    editor_state::StableResolveCtx::from_live(&view, next.projected.seq_checkout());
                let restored = f.resolve(&ctx)?;
                Some(restored.normalize(&view).unwrap_or(restored))
            });
            self.undo_history.invalidate_last_tag();
        }
        self.state = next;
        self.pending_ops.extend(applied_ops);
        Ok(())
    }

    pub fn set_doc(&mut self, plain: editor_model::PlainDoc) {
        self.state = State::from_plain(&plain).expect("set_doc template must build");
        self.composition_paint = None;
        self.ime_delete_paint = None;
        crate::font::reresolve_fonts(self).ok();
        self.view.layout(&self.state);
        self.push_event(EditorEvent::StateChanged {
            fields: StateField::iter().collect(),
        });
        self.invalidate_render();
    }

    pub fn insert_template_fragment(
        &mut self,
        template: editor_model::PlainDoc,
    ) -> Result<(), EditorError> {
        let root_entry = &template.root;
        let root_modifiers: Vec<editor_model::Modifier> =
            root_entry.modifiers.values().cloned().collect();

        // Build the template as a projected state and capture each root child as a
        // subtree (subtrees carry their own modifiers/style/carry recursively, so
        // inserting them re-establishes per-node styling in the eg-walker model).
        let template_state =
            editor_state::State::from_plain(&template).map_err(|e| EditorError::General {
                msg: format!("{e:?}"),
            })?;
        let subtrees: Vec<editor_model::Subtree> = {
            let view = template_state.view();
            match view.root() {
                Some(root) => root
                    .child_blocks()
                    .filter_map(|b| {
                        editor_transaction::capture_subtree(&template_state.projected, b.id())
                    })
                    .collect(),
                None => Vec::new(),
            }
        };

        if subtrees.is_empty() {
            return Ok(());
        }

        self.transact(|tr| {
            tr.batch::<_, EditorError>(|tr| {
                let existing: Vec<Dot> = tr
                    .view()
                    .node(Dot::ROOT)
                    .ok_or_else(|| EditorError::General {
                        msg: "ROOT not found".into(),
                    })?
                    .child_blocks()
                    .map(|b| b.id())
                    .collect();
                for id in existing {
                    tr.remove_subtree(id)?;
                }
                for (i, st) in subtrees.into_iter().enumerate() {
                    tr.insert_subtree(Dot::ROOT, i, st)?;
                }

                let existing_root_mods: Vec<editor_model::Modifier> = tr
                    .state()
                    .projected
                    .block_modifiers()
                    .modifiers_of(Dot::ROOT)
                    .into_values()
                    .collect();
                for m in existing_root_mods {
                    tr.remove_modifier(Dot::ROOT, m)?;
                }
                for modifier in root_modifiers {
                    editor_commands::set_node_modifier(tr, Dot::ROOT, modifier)?;
                }

                Ok(())
            })?;

            let view = tr.view();
            if let Some(root) = view.node(Dot::ROOT)
                && let Some(editor_model::ChildView::Block(first)) = root.first_child()
                && let Some(pos) = editor_state::first_cursor_position(&first)
            {
                tr.set_selection(Some(editor_state::Selection::collapsed(pos)))?;
            }
            Ok(())
        })
    }

    pub(crate) fn run_initialize(&mut self) -> Result<(), EditorError> {
        if matches!(self.mode, Mode::Probe { .. }) {
            return Ok(());
        }
        crate::font::reresolve_fonts(self)?;
        self.view.layout(&self.state);
        self.push_event(EditorEvent::StateChanged {
            fields: StateField::iter().collect(),
        });
        Ok(())
    }

    pub(crate) fn retry_font_load(&mut self, family: &str, base_loaded: bool) {
        if matches!(self.mode, Mode::Probe { .. }) {
            return;
        }
        crate::font::retry_pending_on_load(self, family, base_loaded);
    }

    pub(crate) fn resolve_pending_fonts(&mut self) {
        crate::font::resolve_pending_fonts(self);
    }

    pub(crate) fn manifest_loaded(&mut self, family: &str, weight: u16) {
        if matches!(self.mode, Mode::Probe { .. }) {
            return;
        }
        let family_id = {
            let resource = self.resource.lock().unwrap();
            resource.font_registry.intern_id(family)
        };
        let Some(fid) = family_id else { return };
        if self.requested_manifests.remove(&(fid, weight)).is_none() {
            return;
        }
        self.resolve_pending_fonts();
    }

    pub(crate) fn reresolve_fonts(&mut self) -> Result<(), EditorError> {
        if matches!(self.mode, Mode::Probe { .. }) {
            return Ok(());
        }
        crate::font::reresolve_fonts(self)
    }

    pub(crate) fn resolve_fonts(&mut self, family: &str, weight: u16, codepoints: &[u32]) {
        use editor_resource::Resolution;

        let mut resource = self.resource.lock().unwrap();
        let family_id = resource.font_registry.intern(family);

        let mut grouped: HashMap<(u16, u16), HashSet<u16>> = HashMap::default();
        let mut manifest_needed: HashSet<(u16, u16)> = HashSet::default();
        for &cp in codepoints {
            match resource.font_registry.resolve(family_id, weight, cp) {
                Resolution::Pending { target, .. } => {
                    grouped
                        .entry((target.family_id, target.weight))
                        .or_default()
                        .insert(target.chunk_id);
                }
                Resolution::AwaitingManifest {
                    family_id: fid,
                    weight: w,
                } => {
                    manifest_needed.insert((fid, w));
                }
                Resolution::Ready(_) | Resolution::Missing => {}
            }
        }

        let events: Vec<_> = grouped
            .into_iter()
            .filter_map(|((fid, w), used_chunks)| {
                let is_primary = fid == family_id;
                let base_loaded = resource.font_registry.is_base_loaded(fid, w);

                let mut required: Vec<FontData> = Vec::new();
                if !base_loaded {
                    required.push(FontData::Base);
                }
                let mut unloaded_required: Vec<u16> = used_chunks
                    .iter()
                    .copied()
                    .filter(|&cid| !resource.font_registry.is_chunk_loaded(fid, w, cid))
                    .collect();
                unloaded_required.sort_unstable();
                for cid in &unloaded_required {
                    required.push(FontData::Chunk { id: *cid });
                }

                let prefetch: Vec<FontData> = if is_primary {
                    if let Some(manifest) = resource.font_registry.manifest(fid, w) {
                        manifest
                            .all_chunk_ids()
                            .filter(|cid| !used_chunks.contains(cid))
                            .filter(|cid| !resource.font_registry.is_chunk_loaded(fid, w, *cid))
                            .map(|id| FontData::Chunk { id })
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                if required.is_empty() && prefetch.is_empty() {
                    return None;
                }

                let family_name = resource
                    .font_registry
                    .family_name_opt(fid)
                    .unwrap_or("")
                    .to_string();
                Some(EditorEvent::FontDataMissing {
                    family: family_name,
                    weight: w,
                    required,
                    prefetch,
                })
            })
            .collect();

        let mut manifest_events: Vec<EditorEvent> = Vec::new();
        let mut prefetch_targets: HashSet<(u16, u16)> = HashSet::default();
        for &(fid, w) in &manifest_needed {
            let prev = self
                .requested_manifests
                .insert((fid, w), ManifestRequestClass::Required);
            if prev == Some(ManifestRequestClass::Required) {
                continue;
            }
            let family_name = resource
                .font_registry
                .family_name_opt(fid)
                .unwrap_or("")
                .to_string();
            manifest_events.push(EditorEvent::FontDataMissing {
                family: family_name.clone(),
                weight: w,
                required: vec![FontData::Manifest],
                prefetch: Vec::new(),
            });
            if prev.is_some() {
                continue;
            }
            if let Some(ws) = resource.font_registry.weights(&family_name) {
                for &sibling in ws {
                    if sibling != w && !resource.font_registry.has_manifest(fid, sibling) {
                        prefetch_targets.insert((fid, sibling));
                    }
                }
            }
        }
        let awaiting_fallback = manifest_needed.iter().any(|&(fid, _)| {
            resource.font_registry.family_source(fid)
                == Some(editor_resource::FontFamilySource::Fallback)
        });
        if awaiting_fallback {
            let fb_ids: Vec<u16> = resource.font_registry.fallback_family_ids().collect();
            for fb_id in fb_ids {
                let Some(name) = resource.font_registry.family_name_opt(fb_id) else {
                    continue;
                };
                let name = name.to_string();
                let Some(ws) = resource.font_registry.weights(&name) else {
                    continue;
                };
                for &w in ws {
                    if !resource.font_registry.has_manifest(fb_id, w)
                        && !manifest_needed.contains(&(fb_id, w))
                    {
                        prefetch_targets.insert((fb_id, w));
                    }
                }
            }
        }
        for (fid, w) in prefetch_targets {
            if manifest_needed.contains(&(fid, w)) {
                continue;
            }
            if self.requested_manifests.contains_key(&(fid, w)) {
                continue;
            }
            self.requested_manifests
                .insert((fid, w), ManifestRequestClass::Prefetch);
            let family_name = resource
                .font_registry
                .family_name_opt(fid)
                .unwrap_or("")
                .to_string();
            manifest_events.push(EditorEvent::FontDataMissing {
                family: family_name,
                weight: w,
                required: Vec::new(),
                prefetch: vec![FontData::Manifest],
            });
        }

        drop(resource);
        for event in events.into_iter().chain(manifest_events) {
            self.push_event(event);
        }
    }
}

#[cfg(test)]
pub(crate) fn probe_guard_for_test(editor: &mut Editor) -> impl Drop + '_ {
    ProbeGuard::enter(editor)
}

#[cfg(any(test, feature = "test-utils"))]
impl Editor {
    pub fn new_test(state: State) -> Self {
        let resource = Arc::new(Mutex::new(Resource::new_test()));
        Self::new_test_with_resource(state, resource)
    }

    pub fn new_test_with_resource(state: State, resource: Arc<Mutex<Resource>>) -> Self {
        let mut editor = Self {
            state,
            view: View::new_test(),
            undo_history: UndoHistory::new(Duration::from_millis(300)),
            renderer: Renderer::new(Arc::clone(&resource)),
            resource,
            tracked_ranges: TrackedRangeRegistry::new(),
            dnd: DndState::default(),
            focused: false,
            render_epoch: 0,
            message_queue: Vec::new(),
            pending_events: Vec::new(),
            pending_ops: Vec::new(),
            pending_effects: HashSet::new(),
            pending_fonts: HashMap::new(),
            requested_manifests: HashMap::new(),
            composition_paint: None,
            ime_delete_paint: None,
            mode: Mode::Apply,
            selection_mark_rects_cache: Mutex::new(None),
        };
        // Lay out the view once so the first `tick()` reconciles clean (matches the
        // production `run_initialize` path); otherwise every test's first tick would
        // spuriously report the whole view dirty.
        editor.view.layout(&editor.state);
        editor
    }

    pub fn apply(&mut self, msg: Message) -> Vec<EditorEvent> {
        self.enqueue(msg);
        self.tick().unwrap()
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn history_undos_len(&self) -> usize {
        self.undo_history.undos_len()
    }

    pub fn history_redos_len(&self) -> usize {
        self.undo_history.redos_len()
    }

    #[cfg(test)]
    pub(crate) fn drop_indicator_for_test(&self) -> Option<editor_view::DropIndicator> {
        self.dnd.drop_target().map(|target| target.indicator)
    }

    #[cfg(any(test, feature = "test-utils"))]
    pub(crate) fn theme_variant(&self) -> ThemeVariant {
        self.resource.lock().unwrap().theme.variant()
    }
}

#[cfg(test)]
mod tests {
    use crate::dnd::DndState;
    use crate::editor::Editor;
    use editor_crdt::{Changeset, Dot, ListOp};
    use editor_macros::state;
    use editor_model::{
        ChildView, EditOp, LayoutMode, Modifier, ModifierType, Node, NodeType, PlainDoc, PlainNode,
        PlainNodeEntry, PlainParagraphNode, PlainRootNode, PlainTextNode, SeqItem,
    };
    use editor_state::{
        Affinity, PendingModifier, PendingModifiers, Position, Selection, StablePosition,
        StableResolveCtx, State,
    };
    use hashbrown::HashSet;
    use std::collections::BTreeMap;

    use super::*;

    // A frame's worth of IME composition traffic arrives as several consecutive
    // `TextInput` messages; `tick` coalesces them into batched reduces. The
    // coalesced drain must land on the same document, composition, and selection
    // as ticking each message separately (the run splits after `CommitAsIs` so
    // the commit's text replacement still sees the committed text).
    #[test]
    fn tick_coalesces_consecutive_text_input_messages() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 2)
        };
        let mut coalesced = Editor::new_test(state.clone());
        let mut sequential = Editor::new_test(state);
        let batches: Vec<Vec<FlatImeOp>> = vec![
            vec![FlatImeOp::Compose { text: "ㄱ".into() }],
            vec![FlatImeOp::Compose { text: "가".into() }],
            vec![FlatImeOp::CommitAsIs],
            vec![FlatImeOp::Compose { text: "ㄴ".into() }],
        ];
        for ops in batches.clone() {
            coalesced.enqueue(Message::TextInput { ops });
        }
        coalesced.tick().unwrap();
        for ops in batches {
            sequential.enqueue(Message::TextInput { ops });
            sequential.tick().unwrap();
        }

        let (a, b) = (coalesced.state(), sequential.state());
        let (av, bv) = (a.view(), b.view());
        let text_a = editor_state::flat_text(&av, 0..editor_state::flat_size(&av));
        let text_b = editor_state::flat_text(&bv, 0..editor_state::flat_size(&bv));
        assert_eq!(text_a, text_b);
        assert!(text_a.contains("ab가ㄴ"), "unexpected text: {text_a:?}");
        assert_eq!(a.composition, b.composition);
        assert!(a.composition.is_some(), "trailing compose keeps composing");
        assert_eq!(
            a.selection
                .and_then(|s| s.head.resolve(&av))
                .map(|r| r.to_flat()),
            b.selection
                .and_then(|s| s.head.resolve(&bv))
                .map(|r| r.to_flat()),
        );
    }

    /// Produce a remote changeset by replaying `ops` (in a single commit) onto a
    /// clone of `base`'s projected graph and extracting the resulting local
    /// changeset. The `state!` fixtures build their graph with actor 1 and
    /// deterministic clocks, so replica_a and replica_b share the same base op
    /// identities — the new op continues replica_a's clock and is unknown to
    /// replica_b. Mirrors `handle/remote.rs`'s helper.
    fn remote_change(base: &State, ops: Vec<EditOp>) -> Changeset<EditOp> {
        let mut pa = base.projected.as_ref().clone();
        let baseline: HashSet<Dot> = pa.graph().current_heads().copied().collect();
        pa.apply_batch(ops).unwrap();
        pa.commit();
        pa.graph()
            .local_changesets_since(&baseline)
            .unwrap()
            .remove(0)
    }

    /// Like `remote_change` but commits each op separately, yielding one
    /// sequential changeset per op (each builds on the previous so their op Dots
    /// are distinct and all are unknown to replica_b).
    fn remote_changes(base: &State, ops: Vec<EditOp>) -> Vec<Changeset<EditOp>> {
        let mut pa = base.projected.as_ref().clone();
        let mut out = Vec::new();
        for op in ops {
            let baseline: HashSet<Dot> = pa.graph().current_heads().copied().collect();
            pa.apply_batch(vec![op]).unwrap();
            pa.commit();
            out.push(
                pa.graph()
                    .local_changesets_since(&baseline)
                    .unwrap()
                    .remove(0),
            );
        }
        out
    }

    fn with_pending(mut state: State, pending: PendingModifiers) -> State {
        state.pending_modifiers = pending;
        state
    }

    #[test]
    fn normalize_empty_pending_returns_none() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph } }
            selection: (p1, 0)
        };
        let s = with_pending(state, PendingModifiers::new());
        assert!(normalize_pending_overlay(&s).is_none());
    }

    #[test]
    fn normalize_non_empty_paragraph_returns_none() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let pending: PendingModifiers = vec![PendingModifier::Set {
            modifier: Modifier::Bold,
        }];
        let s = with_pending(state, pending);
        assert!(normalize_pending_overlay(&s).is_none());
    }

    #[test]
    fn normalize_empty_paragraph_returns_some_with_container_id() {
        let (state, p1) = state! {
            doc { root { p1: paragraph } }
            selection: (p1, 0)
        };
        let pending: PendingModifiers = vec![PendingModifier::Set {
            modifier: Modifier::FontSize { value: 9600 },
        }];
        let s = with_pending(state, pending.clone());
        let ps = normalize_pending_overlay(&s).expect("Some");
        assert_eq!(ps.node_id, p1);
        assert_eq!(ps.modifiers, pending);
    }

    #[test]
    fn normalize_head_on_text_child_ascends_to_textblock() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let pending: PendingModifiers = vec![PendingModifier::Set {
            modifier: Modifier::Bold,
        }];
        let s = with_pending(state, pending);
        assert!(normalize_pending_overlay(&s).is_none());
    }

    fn test_editor() -> (Editor, editor_crdt::Dot) {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        (Editor::new_test(state), p1)
    }

    #[test]
    fn select_set_changes_selection() {
        let (mut editor, t) = test_editor();
        let target = Selection::collapsed(Position::new(t, 3));

        editor.apply(Message::Selection {
            op: SelectionOp::Set { selection: target },
        });

        assert_eq!(editor.state().selection, Some(target));
    }

    #[test]
    fn undo_on_empty_history_is_noop() {
        let (mut editor, _) = test_editor();
        let before = editor.state().selection;
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_eq!(editor.state().selection, before);
    }

    #[test]
    fn noop_undo_without_history_keeps_pending() {
        let (mut editor, _) = test_editor();
        editor
            .transact(|tr| {
                tr.set_pending_modifiers(vec![PendingModifier::Set {
                    modifier: Modifier::Bold,
                }])?;
                Ok(())
            })
            .unwrap();
        assert!(!editor.undo_history.can_undo());

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });

        assert_eq!(
            editor.state().pending_modifiers.as_slice(),
            &[PendingModifier::Set {
                modifier: Modifier::Bold
            }]
        );
    }

    // Inverse ops must be sealed at the message boundary: unsealed ops are
    // invisible to `missing_changesets_tolerant` (the sync capture/push source)
    // while still advancing `current_heads`, so a sync capture landing between
    // an undo and the next edit permanently skips the late-sealed changeset —
    // and every later changeset parents on it, so a reload drops them all.
    #[test]
    fn undo_and_redo_seal_inverse_ops_into_changesets() {
        let (mut editor, _) = test_editor();
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "x".into() },
        });
        let sealed = editor.state().projected.graph().changesets().len();

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        let graph = editor.state().projected.graph();
        assert!(graph.pending().is_empty(), "undo left unsealed pending ops");
        assert_eq!(graph.changesets().len(), sealed + 1);

        editor.apply(Message::History {
            op: HistoryOp::Redo,
        });
        let graph = editor.state().projected.graph();
        assert!(graph.pending().is_empty(), "redo left unsealed pending ops");
        assert_eq!(graph.changesets().len(), sealed + 2);
    }

    // Defense-in-depth for the same contract: if a future code path applies ops
    // without sealing (as undo/redo once did), the tick boundary must seal the
    // leftovers — past it the sync layer snapshots `current_heads`, and a
    // changeset sealed after that snapshot is permanently skipped by capture.
    #[test]
    fn tick_seals_leftover_pending_ops_from_unsealing_paths() {
        let (mut editor, _) = test_editor();
        editor
            .state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('z'),
            }))
            .unwrap();
        assert!(!editor.state.projected.graph().pending().is_empty());

        let _ = editor.tick().unwrap();

        let graph = editor.state.projected.graph();
        assert!(graph.pending().is_empty(), "tick left unsealed pending ops");
    }

    #[test]
    fn undo_after_typing_over_multi_paragraph_selection_restores_initial_selection() {
        use editor_state::assert_state_eq;

        let (initial, _p1, _p2) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    paragraph { text("b") }
                    p2: paragraph { text("c") }
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let mut editor = Editor::new_test(initial.clone());

        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "a".into() },
        });
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });

        assert_state_eq!(editor.state(), &initial);
    }

    // The batched undo path (a run of sequence inverses deferred into one reproject,
    // flushed around non-sequence inverses) must round-trip a multi-block edit exactly:
    // undo restores the pre-edit doc, redo re-applies it. Exercises the flush boundary
    // via the carry/span ops the typed replacement introduces.
    #[test]
    fn batched_undo_redo_round_trips_multi_paragraph_replace() {
        use editor_state::assert_state_eq;

        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("alpha") }
                    paragraph { text("beta") }
                    paragraph { text("gamma") }
                    p3: paragraph { text("delta") }
                }
            }
            selection: (p1, 0) -> (p3, 5)
        };
        let mut editor = Editor::new_test(initial.clone());

        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "z".into() },
        });
        let after_edit = editor.state().clone();

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);

        editor.apply(Message::History {
            op: HistoryOp::Redo,
        });
        assert_state_eq!(editor.state(), &after_edit);

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);
    }

    // The forward bulk-delete path (`delete_child_slots` defers per-step projection
    // into one reproject when most root children go away) records the same sequence
    // deletes as the eager path, so undo/redo must round-trip exactly.
    #[test]
    fn bulk_select_all_delete_undo_redo_round_trips() {
        use editor_state::assert_state_eq;

        let (initial, _r) = state! {
            doc { r: root {
                paragraph { text("p00") }
                paragraph { text("p01") }
                paragraph { text("p02") }
                paragraph { text("p03") }
                paragraph { text("p04") }
                paragraph { text("p05") }
                paragraph { text("p06") }
                paragraph { text("p07") }
                paragraph { text("p08") }
                paragraph { text("p09") }
                paragraph { text("p10") }
                paragraph { text("p11") }
            } }
            selection: (r, 0) -> (r, 12)
        };
        let mut editor = Editor::new_test(initial.clone());

        editor.apply(Message::Deletion {
            op: DeletionOp::Selection,
        });
        let after_edit = editor.state().clone();

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);

        editor.apply(Message::History {
            op: HistoryOp::Redo,
        });
        assert_state_eq!(editor.state(), &after_edit);

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);
    }

    #[test]
    fn system_resize_updates_viewport() {
        let (mut editor, _) = test_editor();
        editor.apply(Message::System {
            event: SystemEvent::Resize {
                width: 1024.0,
                height: 768.0,
                scale_factor: 1.0,
            },
        });
        assert_eq!(editor.view().viewport().width, 1024.0);
    }

    #[test]
    fn tick_processes_all_enqueued_messages() {
        let (mut editor, t) = test_editor();

        let selection = Selection::collapsed(Position::new(t, 3));

        editor.enqueue(Message::System {
            event: SystemEvent::Resize {
                width: 1024.0,
                height: 768.0,
                scale_factor: 2.0,
            },
        });
        editor.enqueue(Message::Selection {
            op: SelectionOp::Set { selection },
        });
        editor.tick().unwrap();

        assert_eq!(editor.view().viewport().width, 1024.0);
        assert_eq!(editor.state().selection, Some(selection));
    }

    #[test]
    fn editor_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Editor>();
    }

    #[test]
    fn tick_returns_state_changed_on_selection_set() {
        let (mut editor, t) = test_editor();
        let target = Selection::collapsed(Position::new(t, 3));
        editor.enqueue(Message::Selection {
            op: SelectionOp::Set { selection: target },
        });

        let events = editor.tick().unwrap();

        let has_selection_changed = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::StateChanged { fields } if fields.contains(&StateField::Selection)
            )
        });
        assert!(has_selection_changed);
    }

    #[test]
    fn tick_returns_empty_events_when_no_messages() {
        let (mut editor, _) = test_editor();
        let events = editor.tick().unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn process_effects_emits_font_data_missing() {
        let (mut editor, _) = test_editor();
        // Configure Inter/400 so resolve_codepoint_mappings finds the primary.
        {
            let mut resource = editor.resource.lock().unwrap();
            let families = vec![editor_resource::FontFamily {
                name: "Inter".into(),
                source: editor_resource::FontFamilySource::Default,
                weights: vec![editor_resource::FontWeight {
                    value: 400,
                    hash: "inter-400".into(),
                }],
            }];
            resource.set_fonts(families);
            let inter_id = resource.font_registry.intern_id("Inter").unwrap();
            resource.font_registry.set_manifest(
                inter_id,
                400,
                editor_resource::FontManifest::from_coverages(&[vec![0x41, 0x42]]),
            );
        }

        editor.process_effects(HashSet::from([Effect::LoadFont {
            family: "Inter".to_string(),
            weight: 400,
            codepoints: vec![65, 66],
        }]));
        let events = std::mem::take(&mut editor.pending_events);

        let has_data_missing = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::FontDataMissing { family, weight, required, .. }
                    if family == "Inter"
                        && *weight == 400
                        && required.len() == 2
                        && matches!(required[0], FontData::Base)
                        && matches!(required[1], FontData::Chunk { id: 0 })
            )
        });
        assert!(has_data_missing);
    }

    #[test]
    fn prefetched_manifest_escalates_to_required_once_when_hit_as_blocking() {
        let (state, ..) = state! {
            doc {
                root [font_family("TestFont".to_string()), font_weight(400)] {
                    p1: paragraph { text("A") }
                }
            }
            selection: (p1, 0)
        };

        let mut editor = Editor::new_test(state);
        {
            let mut resource = editor.resource.lock().unwrap();
            resource.set_fonts(vec![editor_resource::FontFamily {
                name: "TestFont".into(),
                source: editor_resource::FontFamilySource::Default,
                weights: vec![
                    editor_resource::FontWeight {
                        value: 400,
                        hash: "tf-400".into(),
                    },
                    editor_resource::FontWeight {
                        value: 700,
                        hash: "tf-700".into(),
                    },
                ],
            }]);
        }

        let init = editor.apply(Message::System {
            event: SystemEvent::Initialize,
        });
        let init_prefetch_700 = init
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    EditorEvent::FontDataMissing { family, weight, required, prefetch }
                        if family == "TestFont"
                            && *weight == 700
                            && required.is_empty()
                            && prefetch.len() == 1
                            && matches!(prefetch[0], FontData::Manifest)
                )
            })
            .count();
        assert_eq!(
            init_prefetch_700, 1,
            "sibling weight 700's manifest must start as a prefetch-only request"
        );

        editor.resolve_fonts("TestFont", 700, &['A' as u32]);
        let first = std::mem::take(&mut editor.pending_events);
        let first_required_700 = first
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    EditorEvent::FontDataMissing { family, weight, required, prefetch }
                        if family == "TestFont"
                            && *weight == 700
                            && required.len() == 1
                            && matches!(required[0], FontData::Manifest)
                            && prefetch.is_empty()
                )
            })
            .count();
        assert_eq!(
            first_required_700, 1,
            "a prefetch-only manifest hit as blocking must emit its required manifest exactly once"
        );

        editor.resolve_fonts("TestFont", 700, &['A' as u32]);
        let second = std::mem::take(&mut editor.pending_events);
        let second_manifest_700 = second
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    EditorEvent::FontDataMissing { family, weight, .. }
                        if family == "TestFont" && *weight == 700
                )
            })
            .count();
        assert_eq!(
            second_manifest_700, 0,
            "a second blocking hit on an already-required manifest must not re-emit"
        );
    }

    #[test]
    fn tick_returns_doc_changed_on_text_insert() {
        let (mut editor, _) = test_editor();
        let events = editor.apply(Message::Insertion {
            op: InsertionOp::Text {
                text: "a".to_string(),
            },
        });

        let has_doc_changed = events
            .iter()
            .any(|e| matches!(e, EditorEvent::StateChanged { fields } if fields.contains(&StateField::Doc)));
        assert!(has_doc_changed);
    }

    #[test]
    fn tick_emits_modifiers_and_block_on_selection_step() {
        let (mut editor, t) = test_editor();
        let target = Selection::collapsed(Position::new(t, 3));
        editor.enqueue(Message::Selection {
            op: SelectionOp::Set { selection: target },
        });

        let events = editor.tick().unwrap();

        let has_modifiers = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::StateChanged { fields } if fields.contains(&StateField::Modifiers)
            )
        });
        let has_block = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::StateChanged { fields } if fields.contains(&StateField::Block)
            )
        });
        assert!(has_modifiers);
        assert!(has_block);
    }

    #[test]
    fn tick_emits_modifiers_and_block_on_doc_step() {
        let (mut editor, _) = test_editor();
        let events = editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "x".into() },
        });

        let has_modifiers = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::StateChanged { fields } if fields.contains(&StateField::Modifiers)
            )
        });
        let has_block = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::StateChanged { fields } if fields.contains(&StateField::Block)
            )
        });
        assert!(has_modifiers);
        assert!(has_block);
    }

    #[test]
    fn input_context_without_selection_returns_none() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: none
        };
        let editor = Editor::new_test(state);
        assert!(editor.ime(usize::MAX, usize::MAX).unwrap().is_none());
    }

    #[test]
    fn input_context_full_window_returns_whole_doc() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let editor = Editor::new_test(state);
        let ctx = editor.ime(usize::MAX, usize::MAX).unwrap().unwrap();
        // flat: O(p)=0, "hello"=1..6, C(p)=6 → flat_size=7
        // (p1,2) → flat 3; window covers full doc [0,7)
        assert_eq!(ctx.text, "\u{2028}hello\u{2029}");
        assert_eq!(ctx.window_start, 0);
        assert_eq!(ctx.selection.start, 3);
        assert_eq!(ctx.selection.end, 3);
        assert!(ctx.composing.is_none());
    }

    #[test]
    fn input_context_limited_window_clamps() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 6)
        };
        let editor = Editor::new_test(state);
        let ctx = editor.ime(3, 3).unwrap().unwrap();
        // flat: O(p)=0, "hello world"=1..12, C(p)=12 → flat_size=13
        // (p1,6) → flat 7; window [7-3, 7+3) = [4, 10) → "lo wor"
        assert_eq!(ctx.window_start, 4);
        assert_eq!(ctx.text, "lo wor");
        assert_eq!(ctx.selection.start, 7);
        assert_eq!(ctx.selection.end, 7);
    }

    #[test]
    fn input_context_with_non_collapsed_selection() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 2) -> (p1, 8)
        };
        let editor = Editor::new_test(state);
        let ctx = editor.ime(usize::MAX, usize::MAX).unwrap().unwrap();
        // flat: O(p)=0, "hello world"=1..12, C(p)=12 → flat_size=13
        // (p1,2)→flat 3, (p1,8)→flat 9; window covers full doc [0,13)
        assert_eq!(ctx.text, "\u{2028}hello world\u{2029}");
        assert_eq!(ctx.selection.start, 3);
        assert_eq!(ctx.selection.end, 9);
    }

    #[test]
    fn input_context_empty_blockquote_has_tokens() {
        let (state, _p1) = state! {
            doc { root { blockquote { p1: paragraph { text("") } } paragraph {} } }
            selection: (p1, 0)
        };
        let editor = Editor::new_test(state);
        let ctx = editor.ime(100, 100).unwrap().unwrap();
        assert!(
            !ctx.text.is_empty(),
            "IME buffer must not be empty for empty blockquote"
        );

        let cursor_in_window = ctx.selection.start - ctx.window_start;
        assert!(cursor_in_window > 0, "cursor should be after Open tokens");
    }

    #[test]
    fn tick_dedups_render_invalidated() {
        let (mut editor, _) = test_editor();
        editor.push_event(EditorEvent::RenderInvalidated);
        editor.push_event(EditorEvent::RenderInvalidated);
        editor.push_event(EditorEvent::RenderInvalidated);

        let events = editor.tick().unwrap();

        let count = events
            .iter()
            .filter(|e| matches!(e, EditorEvent::RenderInvalidated))
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn tick_dedups_identical_state_changed() {
        let (mut editor, _) = test_editor();
        let ev = EditorEvent::StateChanged {
            fields: vec![StateField::Doc],
        };
        editor.push_event(ev.clone());
        editor.push_event(ev);

        let events = editor.tick().unwrap();

        let count = events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    EditorEvent::StateChanged { fields } if fields == &vec![StateField::Doc]
                )
            })
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn tick_keeps_state_changed_with_different_fields() {
        let (mut editor, _) = test_editor();
        editor.push_event(EditorEvent::StateChanged {
            fields: vec![StateField::Doc],
        });
        editor.push_event(EditorEvent::StateChanged {
            fields: vec![StateField::Selection],
        });

        let events = editor.tick().unwrap();

        let count = events
            .iter()
            .filter(|e| matches!(e, EditorEvent::StateChanged { .. }))
            .count();
        assert_eq!(count, 2);
    }

    #[test]
    fn pending_effects_dedups_identical_load_font() {
        let (mut editor, _) = test_editor();

        editor
            .transact(|tr| {
                tr.push_effect(Effect::LoadFont {
                    family: "X".into(),
                    weight: 400,
                    codepoints: vec![65],
                });
                Ok(())
            })
            .unwrap();
        editor
            .transact(|tr| {
                tr.push_effect(Effect::LoadFont {
                    family: "X".into(),
                    weight: 400,
                    codepoints: vec![65],
                });
                Ok(())
            })
            .unwrap();

        assert_eq!(editor.pending_effects.len(), 1);
    }

    #[test]
    fn editor_exposes_modifier_state() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("Hi") [bold] } } }
            selection: (p1, 1)
        };
        let editor = Editor::new_test(state);
        let s = editor.modifier_state().unwrap();
        assert_eq!(s.bold, editor_common::Tri::Uniform { value: () });
    }

    // Selecting exactly a bold run that is followed by more text must light up the
    // bold toolbar button — the trailing run's first char must not be over-collected.
    #[test]
    fn editor_bold_active_for_exact_bold_run_with_trailing_text() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph {
                text("hello")
                text("world") [font_weight(700)]
                text("hi")
            } } }
            selection: (p1, 5) -> (p1, 10)
        };
        let editor = Editor::new_test(state);
        let s = editor.modifier_state().unwrap();
        assert_eq!(
            s.effective_bold,
            editor_common::Tri::Uniform { value: () },
            "selecting exactly the bold run [5,10) must report bold active"
        );
    }

    #[test]
    fn editor_exposes_uniform_link_for_paragraph_child_range_selection() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph {
                text("Hello") [link(href: "https://a.com".to_string())]
                text("World") [link(href: "https://a.com".to_string())]
            } } }
            selection: (p1, 0) -> (p1, 10)
        };
        let editor = Editor::new_test(state);
        let s = editor.modifier_state().unwrap();
        assert_eq!(
            s.link,
            editor_common::Tri::Uniform {
                value: editor_model::LinkValue {
                    href: "https://a.com".to_string(),
                },
            }
        );
    }

    #[test]
    fn editor_exposes_mixed_link_for_paragraph_child_range_selection() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph {
                text("Hello") [link(href: "https://a.com".to_string())]
                text("World") [link(href: "https://b.com".to_string())]
            } } }
            selection: (p1, 0) -> (p1, 10)
        };
        let editor = Editor::new_test(state);
        let s = editor.modifier_state().unwrap();
        assert_eq!(s.link, editor_common::Tri::Mixed);
    }

    #[test]
    fn editor_exposes_block_state() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("Hi") } } }
            selection: (p1, 1)
        };
        let editor = Editor::new_test(state);
        let bs = editor.block_state().unwrap();
        assert_eq!(bs.ancestors.len(), 2);
        assert!(bs.nodes.is_empty());
    }

    #[test]
    fn character_counts_empty_doc_is_all_zero() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let editor = Editor::new_test(state);
        let (doc, sel) = editor.character_counts();
        assert_eq!(doc.with_whitespace, 0);
        assert_eq!(doc.without_whitespace, 0);
        assert_eq!(doc.without_whitespace_and_punctuation, 0);
        assert_eq!(sel.with_whitespace, 0);
        assert_eq!(sel.without_whitespace, 0);
        assert_eq!(sel.without_whitespace_and_punctuation, 0);
    }

    #[test]
    fn character_counts_single_block_no_selection() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let editor = Editor::new_test(state);
        let (doc, sel) = editor.character_counts();
        assert_eq!(doc.with_whitespace, 5);
        assert_eq!(doc.without_whitespace, 5);
        assert_eq!(doc.without_whitespace_and_punctuation, 5);
        assert_eq!(sel.with_whitespace, 0);
        assert_eq!(sel.without_whitespace, 0);
        assert_eq!(sel.without_whitespace_and_punctuation, 0);
    }

    #[test]
    fn state_changed_dedups_regardless_of_fields_order() {
        let (mut editor, _) = test_editor();
        editor.push_event(EditorEvent::StateChanged {
            fields: vec![StateField::Doc, StateField::Selection],
        });
        editor.push_event(EditorEvent::StateChanged {
            fields: vec![StateField::Selection, StateField::Doc],
        });

        let events = editor.tick().unwrap();

        let count = events
            .iter()
            .filter(|e| matches!(e, EditorEvent::StateChanged { .. }))
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn remote_message_triggers_view_invalidation_and_events() {
        let (state_a, _p1) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let css_a = state_a.graph().changesets_as_vec();
        let state_b = State::from_changesets(css_a, state_a.selection).unwrap();

        // Insert 'x' at the start of the text ("hi" -> "xhi"); seq pos 1
        // ([para0, 'h', 'i']).
        let cs = remote_change(
            &state_a,
            vec![EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('x'),
            })],
        );

        // Pre-layout populates the measurer cache; without this, invalidation has nothing
        // to evict and dirty stays false.
        let mut editor = Editor::new_test(state_b);
        editor.view.layout(&editor.state);

        editor.receive_remote_changeset(cs);
        let events = editor.tick().unwrap();

        let has_render = events
            .iter()
            .any(|e| matches!(e, EditorEvent::RenderInvalidated));
        let state_changed_fields: Option<&Vec<StateField>> = events.iter().find_map(|e| match e {
            EditorEvent::StateChanged { fields } => Some(fields),
            _ => None,
        });
        assert!(has_render, "RenderInvalidated must fire for remote receive");
        let fields = state_changed_fields.expect("StateChanged must fire for remote receive");
        for required in [
            StateField::Doc,
            StateField::Ime,
            StateField::Modifiers,
            StateField::Block,
        ] {
            assert!(
                fields.contains(&required),
                "StateChanged must include {required:?}"
            );
        }

        let view = editor.state().view();
        let full_text = editor_state::flat_text(&view, 0..editor_state::flat_size(&view));
        assert!(full_text.contains('x'), "remote insert applied");
    }

    #[test]
    fn multiple_remote_messages_processed_in_single_tick() {
        let (state_a, _p1) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let css_a = state_a.graph().changesets_as_vec();
        let state_b = State::from_changesets(css_a, state_a.selection).unwrap();

        // Three sequential inserts at the start of the text (seq pos 1), each
        // committed separately so they arrive as three distinct changesets.
        let mut css = remote_changes(
            &state_a,
            vec![
                EditOp::Seq(ListOp::Ins {
                    pos: 1,
                    item: SeqItem::Char('x'),
                }),
                EditOp::Seq(ListOp::Ins {
                    pos: 1,
                    item: SeqItem::Char('y'),
                }),
                EditOp::Seq(ListOp::Ins {
                    pos: 1,
                    item: SeqItem::Char('z'),
                }),
            ],
        );
        let cs3 = css.remove(2);
        let cs2 = css.remove(1);
        let cs1 = css.remove(0);

        let mut editor = Editor::new_test(state_b);
        editor.view.layout(&editor.state);

        editor.receive_remote_changeset(cs1);
        editor.receive_remote_changeset(cs2);
        editor.receive_remote_changeset(cs3);
        let events = editor.tick().unwrap();

        let view = editor.state().view();
        let text_str = editor_state::flat_text(&view, 0..editor_state::flat_size(&view));
        assert!(text_str.contains('x'), "op1 ('x') applied");
        assert!(text_str.contains('y'), "op2 ('y') applied");
        assert!(text_str.contains('z'), "op3 ('z') applied");

        let render_count = events
            .iter()
            .filter(|e| matches!(e, EditorEvent::RenderInvalidated))
            .count();
        let doc_state_count = events
            .iter()
            .filter(|e| {
                matches!(e, EditorEvent::StateChanged { fields } if fields.contains(&StateField::Doc))
            })
            .count();
        assert_eq!(
            render_count, 1,
            "RenderInvalidated deduplicated to one event"
        );
        assert_eq!(
            doc_state_count, 1,
            "StateChanged(Doc) deduplicated to one event"
        );
    }

    #[test]
    fn remote_children_remove_invalidates_and_fires_events() {
        let (state_a, _p1) = state! {
            doc { root { p1: paragraph { text("a") } paragraph {} } }
            selection: (p1, 0)
        };
        let css_a = state_a.graph().changesets_as_vec();
        let state_b = State::from_changesets(css_a, state_a.selection).unwrap();

        // Delete the second (empty) paragraph block; seq pos 2
        // ([para0, 'a', para1]).
        let cs = remote_change(&state_a, vec![EditOp::Seq(ListOp::Del { pos: 2, len: 1 })]);

        // Pre-layout so both paragraphs are cached; otherwise sibling-shift invalidation
        // has nothing to evict and dirty stays false.
        let mut editor = Editor::new_test(state_b);
        editor.view.layout(&editor.state);

        editor.receive_remote_changeset(cs);
        let events = editor.tick().unwrap();

        // The remote changeset must still emit a single pair of events — covers dedup
        // along the actual remote-receive path, not just the push_event helper.
        let render_count = events
            .iter()
            .filter(|e| matches!(e, EditorEvent::RenderInvalidated))
            .count();
        let state_changed_count = events
            .iter()
            .filter(|e| matches!(e, EditorEvent::StateChanged { .. }))
            .count();
        assert_eq!(
            render_count, 1,
            "RenderInvalidated deduplicated to one event"
        );
        assert_eq!(
            state_changed_count, 1,
            "StateChanged deduplicated to one event"
        );

        let fields = events
            .iter()
            .find_map(|e| match e {
                EditorEvent::StateChanged { fields } => Some(fields),
                _ => None,
            })
            .expect("StateChanged must fire");
        for required in [
            StateField::Doc,
            StateField::Ime,
            StateField::Modifiers,
            StateField::Block,
        ] {
            assert!(
                fields.contains(&required),
                "StateChanged must include {required:?}"
            );
        }

        let view = editor.state().view();
        let block_count = view.root().expect("root exists").child_blocks().count();
        assert_eq!(
            block_count, 1,
            "second paragraph must no longer be a live child of root after removal"
        );
    }

    #[test]
    fn editor_interactive_hit_test_delegates_to_view() {
        use editor_macros::state;
        let (initial, f1, ft1) = state! {
            doc { root {
                f1: fold {
                    ft1: fold_title { text("Title") }
                    fold_content { paragraph { text("Body") } }
                }
            } }
            selection: (ft1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });

        let rects = editor.view().node_box_rects(&[ft1]);
        let r = rects.first().expect("fold-title box rect");
        // +2 from the box left edge = chevron/padding region (text starts at padding.left=40).
        let hit = editor.interactive_hit_test(r.page_idx, r.rect.x + 2.0, r.rect.y + 2.0);
        assert!(
            matches!(
                hit,
                Some(editor_view::InteractiveHit::FoldTitle { id, .. }) if id == f1
            ),
            "got {hit:?}"
        );
    }

    #[test]
    fn editor_selection_endpoints_delegates_to_view() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 1) -> (p1, 8)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });

        let endpoints = editor
            .selection_endpoints()
            .expect("range selection has endpoints");
        assert_eq!(endpoints.from.page_idx, 0);
        assert!(endpoints.from.rect.width == 0.0);
        assert!(endpoints.to.rect.x > endpoints.from.rect.x);
    }

    #[test]
    fn editor_selection_endpoints_collapsed_is_none() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        assert!(editor.selection_endpoints().is_none());
    }

    #[test]
    fn editor_selection_hit_test_inside_rect() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });

        let view = editor.state.view();
        let resolved = editor
            .state
            .selection
            .expect("selection exists in test")
            .resolve(&view)
            .unwrap();
        let rect = editor.view().selection_rects(&resolved)[0].rect;
        let probe_x = rect.x + rect.width * 0.5;
        let probe_y = rect.y + rect.height * 0.5;
        assert!(editor.selection_hit_test(0, probe_x, probe_y));
    }

    #[test]
    fn editor_selection_hit_test_collapsed_is_false() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        assert!(!editor.selection_hit_test(0, 10.0, 10.0));
    }

    #[test]
    fn editor_selection_hit_test_uses_cell_rect_marks() {
        let (state, _, c00, c01, _, c10, c11) = state! {
            doc { root { table {
                tr0: table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                }
                tr1: table_row {
                    c10: table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                }
            } } }
            selection: (c00, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);
        let cell_sel = {
            let view = editor.state.view();
            editor_state::cell_rect_selection(c00, c10, &view).unwrap()
        };
        editor.state.selection = Some(cell_sel);

        let center = |editor: &Editor, cell| {
            let rect = editor.view.node_box_rects(&[cell])[0].rect;
            (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
        };
        let (x00, y00) = center(&editor, c00);
        let (x01, y01) = center(&editor, c01);
        let (x10, y10) = center(&editor, c10);
        let (x11, y11) = center(&editor, c11);

        assert!(editor.selection_hit_test(0, x00, y00));
        assert!(!editor.selection_hit_test(0, x01, y01));
        assert!(editor.selection_hit_test(0, x10, y10));
        assert!(!editor.selection_hit_test(0, x11, y11));
    }

    #[test]
    fn editor_cursor_hit_test_matches_current_collapsed_position() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });

        let sel = editor.state.selection.expect("selection exists in test");
        let cursor = editor
            .view()
            .cursor_metrics(&editor.state, &sel.head)
            .expect("collapsed cursor has metrics");
        let probe_x = cursor.caret.x;
        let probe_y = cursor.line.y + cursor.line.height * 0.5;

        assert!(editor.cursor_hit_test(0, probe_x, probe_y));
    }

    #[test]
    fn editor_cursor_hit_test_rejects_different_position() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });

        let sel = editor.state.selection.expect("selection exists in test");
        let cursor = editor
            .view()
            .cursor_metrics(&editor.state, &sel.head)
            .expect("collapsed cursor has metrics");
        let probe_x = cursor.caret.x + 100.0;
        let probe_y = cursor.line.y + cursor.line.height * 0.5;

        assert!(!editor.cursor_hit_test(0, probe_x, probe_y));
    }

    #[test]
    fn editor_cursor_hit_test_rejects_range_selection() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });

        assert!(!editor.cursor_hit_test(0, 10.0, 10.0));
    }

    #[test]
    fn cursor_metrics_at_span_boundary_uses_input_font_size() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("a") text("a") [font_size(2400)] } } }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });

        let at_start = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 1))
            .expect("cursor metrics at p1 offset 1");
        let at_end = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 2))
            .expect("cursor metrics at p1 offset 2");

        assert!(
            at_start.caret.height < at_end.caret.height,
            "offset 1 (입력은 이전 span 폰트) caret이 offset 2 (24pt) caret보다 작아야 함: \
             start={}, end={}",
            at_start.caret.height,
            at_end.caret.height,
        );
    }

    #[test]
    fn dnd_over_text_sets_drop_indicator_and_invalidates_render() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 2))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::Text,
            },
        });
        let events = editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                modifiers: InputModifiers::default(),
            },
        });

        assert!(
            events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated))
        );
        assert!(editor.drop_indicator_for_test().is_some());
    }

    #[test]
    fn dnd_over_without_active_session_does_not_set_drop_indicator() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 2))
            .expect("cursor metrics")
            .caret;

        let events = editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                modifiers: InputModifiers::default(),
            },
        });

        assert!(
            !events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated))
        );
        assert!(editor.drop_indicator_for_test().is_none());
    }

    #[test]
    fn dnd_leave_clears_drop_indicator_and_invalidates_render() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 2))
            .expect("cursor metrics")
            .caret;
        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::Text,
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                modifiers: InputModifiers::default(),
            },
        });

        let events = editor.apply(Message::Dnd { op: DndOp::Leave });

        assert!(
            events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated))
        );
        assert!(editor.drop_indicator_for_test().is_none());
    }

    #[test]
    fn dnd_leave_preserves_internal_drag_session_while_clearing_drop_indicator() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 5))
            .expect("cursor metrics")
            .caret;
        editor.apply(Message::Dnd {
            op: DndOp::StartInternalSelection,
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                modifiers: InputModifiers::default(),
            },
        });
        assert!(editor.drop_indicator_for_test().is_some());

        let events = editor.apply(Message::Dnd { op: DndOp::Leave });

        assert!(
            events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated))
        );
        assert!(editor.drop_indicator_for_test().is_none());
        assert!(matches!(editor.dnd, DndState::InternalDnd { .. }));
    }

    #[test]
    fn internal_dnd_over_inside_source_selection_rejects_drop_indicator() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 2))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::StartInternalSelection,
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                modifiers: InputModifiers::default(),
            },
        });

        assert!(editor.drop_indicator_for_test().is_none());
    }

    #[test]
    fn internal_dnd_page_break_source_rejects_nested_inline_drop_indicator() {
        let (initial, _p1, p2) = state! {
            doc { root {
                p1: paragraph { text("a") page_break }
                blockquote { p2: paragraph { text("inside") } }
                paragraph {}
            } }
            selection: (p1, 1) -> (p1, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p2, 2))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::StartInternalSelection,
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                modifiers: InputModifiers::default(),
            },
        });

        assert!(editor.drop_indicator_for_test().is_none());
    }

    #[test]
    fn internal_dnd_text_and_page_break_source_allows_nested_inline_drop_indicator() {
        let (initial, _p1, p2) = state! {
            doc { root {
                p1: paragraph { text("a") page_break }
                blockquote { p2: paragraph { text("inside") } }
                paragraph {}
            } }
            selection: (p1, 0) -> (p1, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p2, 2))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::StartInternalSelection,
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                modifiers: InputModifiers::default(),
            },
        });

        assert!(editor.drop_indicator_for_test().is_some());
    }

    #[test]
    fn internal_dnd_page_break_source_allows_root_inline_drop_indicator() {
        let (initial, _p1, p2) = state! {
            doc { root {
                p1: paragraph { text("a") page_break }
                p2: paragraph { text("root") }
                paragraph {}
            } }
            selection: (p1, 1) -> (p1, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p2, 2))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::StartInternalSelection,
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                modifiers: InputModifiers::default(),
            },
        });

        assert!(editor.drop_indicator_for_test().is_some());
    }

    #[test]
    fn drop_text_inserts_at_drop_target_not_current_selection() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 5))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::Text,
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                modifiers: InputModifiers::default(),
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                payload: DndDropPayload::Text {
                    text: "!".into(),
                    html: None,
                },
                modifiers: InputModifiers::default(),
            },
        });

        let (expected, _p1_e) = state! {
            doc { root { p1: paragraph { text("hello!") } } }
            selection: (p1, 5) -> (p1, 6)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn drop_text_without_external_enter_is_noop() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial.clone());
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 5))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                payload: DndDropPayload::Text {
                    text: "!".into(),
                    html: None,
                },
                modifiers: InputModifiers::default(),
            },
        });

        editor_state::assert_state_eq!(editor.state(), &initial);
    }

    #[test]
    fn drop_text_without_drop_target_is_noop() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial.clone());
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 5))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::Text,
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                payload: DndDropPayload::Text {
                    text: "!".into(),
                    html: None,
                },
                modifiers: InputModifiers::default(),
            },
        });

        editor_state::assert_state_eq!(editor.state(), &initial);
    }

    #[test]
    fn drop_text_falls_back_to_plain_text_when_html_is_empty() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 5))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::Html,
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                modifiers: InputModifiers::default(),
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                payload: DndDropPayload::Text {
                    text: "plain".into(),
                    html: Some("<style>p { color: red; }</style>".into()),
                },
                modifiers: InputModifiers::default(),
            },
        });

        let (expected, _p1_e) = state! {
            doc { root { p1: paragraph { text("helloplain") } } }
            selection: (p1, 5) -> (p1, 10)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn drop_files_inserts_placeholders_at_drop_target_not_current_selection() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 5))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::EnterExternal {
                payload: ExternalDndPayloadKind::MixedFiles,
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                modifiers: InputModifiers::default(),
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                payload: DndDropPayload::Files {
                    image_count: 1,
                    file_count: 1,
                },
                modifiers: InputModifiers::default(),
            },
        });

        let view = editor.state().view();
        let root = view.node(Dot::ROOT).unwrap();
        let kinds: Vec<NodeType> = root
            .children()
            .map(|c| match c {
                ChildView::Block(b) => b.node_type(),
                ChildView::Leaf(l) => l.node_type(),
            })
            .collect();
        assert!(matches!(kinds.first(), Some(NodeType::Paragraph)));
        assert!(matches!(kinds.get(1), Some(NodeType::Image)));
        assert!(matches!(kinds.get(2), Some(NodeType::File)));
        assert!(
            matches!(kinds.last(), Some(NodeType::Paragraph)),
            "root schema should keep a trailing paragraph after inserted file blocks",
        );
        let first_text = root.child_blocks().next().map(|p| p.inline_text());
        assert_eq!(first_text.as_deref(), Some("hello"));
    }

    #[test]
    fn drop_internal_selection_without_start_is_noop() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let mut editor = Editor::new_test(initial.clone());
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 11))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                modifiers: InputModifiers::default(),
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                payload: DndDropPayload::InternalSelection,
                modifiers: InputModifiers::default(),
            },
        });

        editor_state::assert_state_eq!(editor.state(), &initial);
    }

    #[test]
    fn drop_internal_image_into_text_middle_selects_inserted_range() {
        let (initial, root, p1) = state! {
            doc { root: root {
                image
                p1: paragraph { text("hello") }
            } }
            selection: (root, 0, >) -> (root, 1, <)
        };
        let mut editor = Editor::new_test(initial);

        editor.apply(Message::Dnd {
            op: DndOp::StartInternalSelection,
        });
        let DndState::InternalDnd { drop_target, .. } = &mut editor.dnd else {
            panic!("internal dnd should start from selected image");
        };
        *drop_target = Some(editor_view::DropTarget {
            position: StablePosition::capture(&Position::new(p1, 3), &editor.state.view()),
            indicator: editor_view::DropIndicator::Inline {
                page_idx: 0,
                x: 0.0,
                y: 0.0,
                height: 1.0,
            },
        });

        editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: 0.0,
                y: 0.0,
                payload: DndDropPayload::InternalSelection,
                modifiers: InputModifiers::default(),
            },
        });

        let view = editor.state().view();
        let root_node = view.node(root).expect("root exists");
        let kinds: Vec<NodeType> = root_node
            .children()
            .map(|c| match c {
                ChildView::Block(b) => b.node_type(),
                ChildView::Leaf(l) => l.node_type(),
            })
            .collect();
        assert!(matches!(
            kinds.as_slice(),
            [NodeType::Paragraph, NodeType::Image, NodeType::Paragraph]
        ));
        let mut blocks = root_node.child_blocks();
        let first_text = blocks.next().map(|p| p.inline_text());
        let last_text = blocks.next().map(|p| p.inline_text());
        assert_eq!(first_text.as_deref(), Some("hel"));
        assert_eq!(last_text.as_deref(), Some("lo"));
        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                Position {
                    node: root,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node: root,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn drop_text_at_block_position_before_image_keeps_image() {
        let (initial, ..) = state! {
            doc { root: root {
                paragraph { text("before") }
                image
                paragraph { text("after") }
            } }
            selection: (root, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.dnd = DndState::ExternalDnd {
            payload: ExternalDndPayloadKind::Text,
            drop_target: Some(editor_view::DropTarget {
                position: StablePosition::capture(
                    &Position::new(Dot::ROOT, 1),
                    &editor.state.view(),
                ),
                indicator: editor_view::DropIndicator::Block {
                    page_idx: 0,
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                },
            }),
        };

        editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: 0.0,
                y: 0.0,
                payload: DndDropPayload::Text {
                    text: "dropped".into(),
                    html: None,
                },
                modifiers: InputModifiers::default(),
            },
        });

        let view = editor.state().view();
        let root = view.node(Dot::ROOT).unwrap();
        let kinds: Vec<NodeType> = root
            .children()
            .map(|c| match c {
                ChildView::Block(b) => b.node_type(),
                ChildView::Leaf(l) => l.node_type(),
            })
            .collect();
        assert!(matches!(
            kinds.as_slice(),
            [
                NodeType::Paragraph,
                NodeType::Paragraph,
                NodeType::Image,
                NodeType::Paragraph,
            ]
        ));
        let inserted_text = root.child_blocks().nth(1).map(|p| p.inline_text());
        assert_eq!(inserted_text.as_deref(), Some("dropped"));
    }

    #[test]
    fn drop_text_at_block_position_before_file_keeps_file() {
        let (initial, ..) = state! {
            doc { root: root {
                paragraph { text("before") }
                file
                paragraph { text("after") }
            } }
            selection: (root, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.dnd = DndState::ExternalDnd {
            payload: ExternalDndPayloadKind::Text,
            drop_target: Some(editor_view::DropTarget {
                position: StablePosition::capture(
                    &Position::new(Dot::ROOT, 1),
                    &editor.state.view(),
                ),
                indicator: editor_view::DropIndicator::Block {
                    page_idx: 0,
                    x: 0.0,
                    y: 0.0,
                    width: 1.0,
                },
            }),
        };

        editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: 0.0,
                y: 0.0,
                payload: DndDropPayload::Text {
                    text: "dropped".into(),
                    html: None,
                },
                modifiers: InputModifiers::default(),
            },
        });

        let view = editor.state().view();
        let root = view.node(Dot::ROOT).unwrap();
        let kinds: Vec<NodeType> = root
            .children()
            .map(|c| match c {
                ChildView::Block(b) => b.node_type(),
                ChildView::Leaf(l) => l.node_type(),
            })
            .collect();
        assert!(matches!(
            kinds.as_slice(),
            [
                NodeType::Paragraph,
                NodeType::Paragraph,
                NodeType::File,
                NodeType::Paragraph,
            ]
        ));
        let inserted_text = root.child_blocks().nth(1).map(|p| p.inline_text());
        assert_eq!(inserted_text.as_deref(), Some("dropped"));
    }

    #[test]
    fn internal_drop_move_remaps_target_after_deleting_source() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 11))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::StartInternalSelection,
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                modifiers: InputModifiers::default(),
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                payload: DndDropPayload::InternalSelection,
                modifiers: InputModifiers::default(),
            },
        });

        let (expected, _p1_e) = state! {
            doc { root { p1: paragraph { text(" worldhello") } } }
            selection: (p1, 6) -> (p1, 11)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn internal_drop_copy_preserves_source_selection_content() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 11))
            .expect("cursor metrics")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::StartInternalSelection,
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                modifiers: InputModifiers {
                    alt: true,
                    ..InputModifiers::default()
                },
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: caret.x,
                y: caret.y + caret.height * 0.5,
                payload: DndDropPayload::InternalSelection,
                modifiers: InputModifiers {
                    alt: true,
                    ..InputModifiers::default()
                },
            },
        });

        let (expected, _p1_e) = state! {
            doc { root { p1: paragraph { text("hello worldhello") } } }
            selection: (p1, 11) -> (p1, 16)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn internal_drop_moves_text_and_discards_page_break_at_nested_destination() {
        let (initial, _p1, p2) = state! {
            doc { root {
                p1: paragraph { text("a") page_break }
                blockquote { p2: paragraph { text("inside") } }
                paragraph {}
            } }
            selection: (p1, 0) -> (p1, 2)
        };
        let source = initial.selection.expect("source selection");
        let mut editor = Editor::new_test(initial);

        crate::handle::apply_drop_for_test(
            &mut editor,
            Position::new(p2, 2),
            DndDropPayload::InternalSelection,
            InputModifiers::default(),
            Some(source),
        )
        .expect("drop succeeds");

        let (expected, ..) = state! {
            doc { root {
                paragraph {}
                blockquote { p2: paragraph { text("inaside") } }
                paragraph {}
            } }
            selection: (p2, 2) -> (p2, 3)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn internal_drop_page_break_only_nested_preserves_source() {
        let (initial, _p1, p2) = state! {
            doc { root {
                p1: paragraph { text("a") page_break }
                blockquote { p2: paragraph { text("inside") } }
                paragraph {}
            } }
            selection: (p1, 1) -> (p1, 2)
        };
        let source = initial.selection.expect("source selection");
        let mut editor = Editor::new_test(initial.clone());

        crate::handle::apply_drop_for_test(
            &mut editor,
            Position::new(p2, 2),
            DndDropPayload::InternalSelection,
            InputModifiers::default(),
            Some(source),
        )
        .expect("drop is a no-op");

        editor_state::assert_state_eq!(editor.state(), &initial);
    }

    #[test]
    fn external_drop_inserts_text_and_discards_page_break_at_nested_destination() {
        let (source, ..) = state! {
            doc { root { p1: paragraph { text("a") page_break } } }
            selection: (p1, 0) -> (p1, 2)
        };
        let payload = Slice::extract(&source)
            .unwrap()
            .to_payload(&Resource::new_test());
        let (target, p2) = state! {
            doc { root {
                blockquote { p2: paragraph { text("inside") } }
                paragraph {}
            } }
            selection: (p2, 0)
        };
        let mut editor = Editor::new_test(target);

        crate::handle::apply_drop_for_test(
            &mut editor,
            Position::new(p2, 2),
            DndDropPayload::Text {
                text: payload.text,
                html: Some(payload.html),
            },
            InputModifiers::default(),
            None,
        )
        .expect("drop succeeds");

        let (expected, ..) = state! {
            doc { root {
                blockquote { p2: paragraph { text("inaside") } }
                paragraph {}
            } }
            selection: (p2, 2) -> (p2, 3)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn internal_drop_move_block_preserves_bold_and_alignment() {
        let (initial, _root, _p_src, _p_after) = state! {
            doc {
                root: root {
                    p_src: paragraph [alignment(editor_model::Alignment::Center)] {
                        text("X") [bold]
                    }
                    p_after: paragraph { text("Y") }
                }
            }
            selection: (root, 0) -> (root, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let source = editor.state().selection.expect("block selection");
        let moved = crate::handle::apply_drop_for_test(
            &mut editor,
            Position::new(Dot::ROOT, 2),
            DndDropPayload::InternalSelection,
            InputModifiers::default(),
            Some(source),
        );
        assert!(moved.is_ok(), "block move should succeed: {moved:?}");

        let (expected, ..) = state! {
            doc {
                root {
                    py: paragraph { text("Y") }
                    paragraph [alignment(editor_model::Alignment::Center)] {
                        text("X") [bold]
                    }
                }
            }
            selection: (py, 0)
        };
        editor_state::assert_doc_eq!(editor.state(), &expected);
    }

    #[test]
    fn internal_drop_moves_only_selected_callout_out_of_fold_content() {
        let (initial, root, _content, _after) = state! {
            doc {
                root: root {
                    fold {
                        fold_title { text("title") }
                        content: fold_content {
                            paragraph { text("inside") }
                            callout { paragraph { text("moved") } }
                        }
                    }
                    after: paragraph { text("after") }
                }
            }
            selection: (content, 1) -> (content, 2)
        };
        let slice = editor_clipboard::Slice::extract(&initial).expect("callout slice");
        assert_eq!((slice.open_start, slice.open_end), (0, 0));
        assert_eq!(slice.content.len(), 1);
        assert!(matches!(slice.content[0].node, PlainNode::Callout(_)));

        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let source = editor.state().selection.expect("callout selection");

        crate::handle::apply_drop_for_test(
            &mut editor,
            Position::new(root, 1),
            DndDropPayload::InternalSelection,
            InputModifiers::default(),
            Some(source),
        )
        .expect("callout move should succeed");

        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        fold_title { text("title") }
                        fold_content {
                            paragraph { text("inside") }
                            paragraph {}
                        }
                    }
                    callout { paragraph { text("moved") } }
                    paragraph { text("after") }
                }
            }
            selection: none
        };
        editor_state::assert_doc_eq!(editor.state(), &expected);
    }

    #[test]
    fn normalize_gap_phantom_some_for_leading_unit() {
        let (state, ..) = state! {
            doc { root: root { image paragraph { text("b") } } }
            selection: (root, 0, <)
        };
        let gp = super::normalize_gap_phantom(&state).expect("leading image is a gap");
        assert_eq!(gp.parent, Dot::ROOT);
        assert_eq!(gp.index, 0);
    }

    #[test]
    fn normalize_gap_phantom_none_for_normal_caret() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 1)
        };
        assert!(super::normalize_gap_phantom(&state).is_none());
    }

    #[test]
    fn normalize_gap_phantom_some_between_two_folds() {
        let (state, ..) = state! {
            doc {
                root: root {
                    fold { fold_title { text("a") } fold_content { paragraph { text("x") } } }
                    fold { fold_title { text("b") } fold_content { paragraph { text("y") } } }
                    paragraph {}
                }
            }
            selection: (root, 1)
        };
        let gp = super::normalize_gap_phantom(&state).expect("between two folds");
        assert_eq!(gp.parent, Dot::ROOT);
        assert_eq!(gp.index, 1);
    }

    #[test]
    fn gap_cursor_yields_caret_then_recovers() {
        let (state, _root, p1) = state! {
            doc { root: root { image p1: paragraph { text("b") } } }
            selection: (root, 0, <)
        };
        let mut editor = Editor::new_test(state);
        // Prime the measurer cache before the gap appears; reconcile needs a
        // populated layout to invalidate when gap_phantom flips on. layout()
        // itself clears gap_phantom, so it must run before the gap tick.
        editor.view.layout(&editor.state);
        let _ = editor.tick().unwrap();
        let st = editor.state();
        assert!(
            editor
                .view()
                .cursor_metrics(st, &st.selection.expect("selection exists in test").head)
                .is_some(),
            "gap cursor must produce a caret via the phantom line"
        );

        editor.enqueue(Message::Selection {
            op: SelectionOp::Set {
                selection: Selection::collapsed(Position::new(p1, 1)),
            },
        });
        let _ = editor.tick().unwrap();
        let st2 = editor.state();
        assert!(
            editor
                .view()
                .cursor_metrics(st2, &st2.selection.expect("selection exists in test").head)
                .is_some(),
            "normal caret still valid after leaving the gap (phantom space recovered)"
        );
    }

    #[test]
    fn no_op_messages_when_selection_is_none() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: none
        };
        let mut editor = Editor::new_test(initial);

        let pre_state = editor.state().clone();
        let pre_selection = editor.state().selection;
        let pre_can_undo = editor.undo_history.can_undo();

        editor.enqueue(Message::Key {
            event: KeyEvent {
                key: Key::Backspace,
                modifiers: InputModifiers::default(),
            },
        });
        editor.enqueue(Message::Insertion {
            op: InsertionOp::Text { text: "X".into() },
        });
        editor.enqueue(Message::Navigation {
            op: NavigationOp::Move {
                movement: Movement::Grapheme {
                    direction: Direction::Forward,
                },
                extend: false,
            },
        });
        editor.tick().unwrap();

        editor_state::assert_doc_eq!(editor.state(), &pre_state);
        assert_eq!(
            editor.state().selection,
            pre_selection,
            "selection must stay None"
        );
        assert_eq!(
            editor.undo_history.can_undo(),
            pre_can_undo,
            "no history entries pushed"
        );
    }

    #[test]
    fn can_returns_true_for_insertion_with_selection() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        let probed = editor.can(Message::Insertion {
            op: InsertionOp::Text { text: "x".into() },
        });
        assert!(probed.unwrap());
    }

    #[test]
    fn can_returns_false_for_undo_with_empty_history() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        let probed = editor.can(Message::History {
            op: HistoryOp::Undo,
        });
        assert!(!probed.unwrap());
    }

    #[test]
    fn can_returns_false_for_set_same_selection() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        let same = editor_state::Selection::collapsed(editor_state::Position::new(p1, 2));
        let probed = editor.can(Message::Selection {
            op: SelectionOp::Set { selection: same },
        });
        assert!(!probed.unwrap());
    }

    #[test]
    fn can_does_not_mutate_state_for_insertion() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        let before_state = editor.state().clone();
        let before_sel = editor.state().selection;
        let _ = editor.can(Message::Insertion {
            op: InsertionOp::Text { text: "x".into() },
        });
        editor_state::assert_doc_eq!(editor.state(), &before_state);
        assert_eq!(editor.state().selection, before_sel);
    }

    #[test]
    fn render_uses_node_box_rects_for_cell_rect_selection() {
        let (state, c00, _c01, _c10, c11) = state! {
            doc { root { table {
                table_row {
                    c00: table_cell { paragraph { text("a") } }
                    c01: table_cell { paragraph { text("b") } }
                    table_cell { paragraph { text("x") } }
                }
                table_row {
                    c10: table_cell { paragraph { text("c") } }
                    c11: table_cell { paragraph { text("d") } }
                    table_cell { paragraph { text("y") } }
                }
            } } }
            selection: (c00, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state);

        let sel = {
            let view = editor.state.view();
            editor_state::cell_rect_selection(c00, c11, &view).unwrap()
        };
        editor.state.selection = Some(sel);

        let view = editor.state.view();
        let resolved = editor
            .state
            .selection
            .expect("selection set above")
            .resolve(&view)
            .unwrap();
        assert!(
            resolved.as_cell_rect().is_some(),
            "precondition: selection is a cell-rect"
        );

        let ids: Vec<_> = resolved
            .as_cell_rect()
            .unwrap()
            .cells()
            .into_iter()
            .map(|c| c.id())
            .collect();
        let expected = editor.view.node_box_rects(&ids);
        assert_eq!(expected.len(), 4, "4 cells → 4 rects");

        let cell_path = editor.cell_selection_rects_for_test();
        assert_eq!(
            cell_path,
            expected
                .iter()
                .map(|r| r.without_meta())
                .collect::<Vec<_>>()
        );
    }

    fn fold_editor_with_unit_selection() -> (Editor, editor_crdt::Dot, editor_crdt::Dot) {
        let (initial, root, fold_node, _p1) = state! {
            doc {
                root: root {
                    fold_node: fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("content") } }
                    }
                    p1: paragraph { text("after") }
                }
            }
            selection: (root, 0) -> (root, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        (editor, root, fold_node)
    }

    // drop_target_at이 fold 문서에서 유효한 위치를 반환하는지 확인
    #[test]
    fn fold_dnd_drop_target_at_returns_position_below_paragraph() {
        let (initial, r, _fnode, p1) = state! {
            doc {
                r: root {
                    _fnode: fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("content") } }
                    }
                    p1: paragraph { text("after") }
                }
            }
            selection: (r, 0) -> (r, 1)
        };
        let _ = r;
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let page_bottom = editor.view().pages()[0].size.height;
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 5))
            .expect("cursor metrics")
            .caret;

        let target = editor.view.drop_target_at(0, caret.x, page_bottom - 1.0);

        assert!(
            target.is_some(),
            "drop_target_at must return Some for y=page_bottom-1 with fold+paragraph doc, caret at y={}, height={}",
            caret.y,
            caret.height
        );
        if let Some(t) = target {
            let view = editor.state.view();
            let ctx = StableResolveCtx::from_live(&view, editor.state.projected.seq_checkout());
            assert_eq!(
                t.position.resolve(&ctx),
                Some(Position::new(Dot::ROOT, 2)),
                "drop target position must be (root, 2) - after paragraph"
            );
        }
    }

    // fold 선택 후 StartInternalSelection → DndState가 InternalDnd로 전환되어야 한다
    #[test]
    fn fold_unit_selection_starts_internal_dnd() {
        let (mut editor, _, _) = fold_editor_with_unit_selection();
        editor.apply(Message::Dnd {
            op: DndOp::StartInternalSelection,
        });
        assert!(
            matches!(editor.dnd, DndState::InternalDnd { .. }),
            "fold unit selection must produce InternalDnd state, got {:?}",
            editor.dnd
        );
    }

    // drop_internal_selection 단계별 디버그: 어느 단계에서 실패하는지 확인
    #[test]
    fn fold_dnd_debug_drop_internal_selection_steps() {
        use editor_clipboard::Slice;
        use editor_commands::{self as commands};
        let (initial, _root, _fold_node, _p1) = state! {
            doc {
                root: root {
                    _fold_node: fold {
                        fold_title { text("t") }
                        fold_content { paragraph { text("c") } }
                    }
                    p1: paragraph { text("after") }
                }
            }
            selection: (root, 0) -> (root, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        editor.apply(Message::Dnd {
            op: DndOp::StartInternalSelection,
        });

        let sel = {
            let DndState::InternalDnd { ref source, .. } = editor.dnd else {
                panic!("expected InternalDnd");
            };
            let view = editor.state.view();
            let ctx = StableResolveCtx::from_live(&view, editor.state.projected.seq_checkout());
            source.resolve(&ctx).expect("source restores")
        };

        // Extract slice
        let mut state = editor.state().clone();
        state.selection = Some(sel);
        let slice = Slice::extract(&state).expect("Slice::extract must succeed");

        // Print slice content for debugging
        for (i, child) in slice.content.iter().enumerate() {
            eprintln!(
                "  root[{}]: {:?}, modifiers: {:?}",
                i, child.node, child.modifiers
            );
            for (j, gc) in child.children.iter().enumerate() {
                eprintln!(
                    "    grandchild[{}]: {:?}, modifiers: {:?}",
                    j, gc.node, gc.modifiers
                );
                for (k, ggc) in gc.children.iter().enumerate() {
                    eprintln!(
                        "      great-grandchild[{}]: {:?}, modifiers: {:?}",
                        k, ggc.node, ggc.modifiers
                    );
                }
            }
        }

        // Step 1: set_selection + delete_selection in separate transact
        let stable_target = {
            let view = editor.state.view();
            StableSelection::capture(&Selection::collapsed(Position::new(Dot::ROOT, 2)), &view)
        };
        let r1 = editor.transact(|tr| {
            commands::set_selection(tr, sel).map_err(crate::error::EditorError::from)?;
            commands::delete_selection(tr).map_err(crate::error::EditorError::from)?;
            Ok(())
        });
        match r1 {
            Ok(_) => eprintln!("Step 1 (set_selection + delete_selection): OK"),
            Err(e) => panic!("Step 1 failed: {:?}", e),
        }

        // Step 2: insert_slice_at in separate transact
        let target = {
            let view = editor.state.view();
            let ctx = StableResolveCtx::from_live(&view, editor.state.projected.seq_checkout());
            stable_target.resolve(&ctx).expect("target restores").head
        };
        eprintln!(
            "target position after deletion: node={:?} offset={}",
            target.node, target.offset
        );
        let r2 = editor.transact(|tr| {
            commands::insert_slice_at(
                tr,
                target,
                slice.clone(),
                commands::types::SliceProvenance::Formatted,
            )
            .map_err(crate::error::EditorError::from)?;
            Ok(())
        });
        match r2 {
            Ok(_) => eprintln!("Step 2 (insert_slice_at): OK"),
            Err(e) => panic!("Step 2 failed: {:?}", e),
        }
    }

    // can_apply_drop이 fold → after-paragraph 이동에 대해 TRUE 반환하는지 검증
    #[test]
    fn fold_dnd_can_apply_drop_at_root_2_returns_true() {
        let (initial, _root, _fold_node, _p1) = state! {
            doc {
                root: root {
                    _fold_node: fold {
                        fold_title { text("t") }
                        fold_content { paragraph { text("c") } }
                    }
                    p1: paragraph { text("after") }
                }
            }
            selection: (root, 0) -> (root, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        editor.apply(Message::Dnd {
            op: DndOp::StartInternalSelection,
        });

        // probe can_apply_drop at (root, 2)
        let target_pos = Position::new(Dot::ROOT, 2);
        let result = editor.probe(|editor| {
            let dnd = editor.dnd.clone();
            let DndState::InternalDnd { source, .. } = dnd else {
                panic!("expected InternalDnd state");
            };
            let sel = {
                let view = editor.state.view();
                let ctx = StableResolveCtx::from_live(&view, editor.state.projected.seq_checkout());
                source.resolve(&ctx).expect("source restores")
            };
            assert!(
                !sel.is_collapsed(),
                "source selection must not be collapsed"
            );
            crate::handle::apply_drop_for_test(
                editor,
                target_pos,
                DndDropPayload::InternalSelection,
                InputModifiers::default(),
                Some(sel),
            )
        });
        match result {
            Ok(true) => {}
            Ok(false) => {
                panic!("apply_drop at (root, 2) returned Ok(false) — state did not change")
            }
            Err(e) => panic!("apply_drop at (root, 2) returned Err: {:?}", e),
        }
    }

    // fold 선택 드래그 중 fold 아래 paragraph 하단에 hover → indicator가 나타나야 한다
    #[test]
    fn fold_dnd_over_below_paragraph_shows_drop_indicator() {
        let (initial, _root, _fold_node, p1) = state! {
            doc {
                root: root {
                    _fold_node: fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("content") } }
                    }
                    p1: paragraph { text("after") }
                }
            }
            selection: (root, 0) -> (root, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 5))
            .expect("cursor metrics for paragraph after fold")
            .caret;

        editor.apply(Message::Dnd {
            op: DndOp::StartInternalSelection,
        });
        let page_bottom = editor.view().pages()[0].size.height;
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: page_bottom - 1.0,
                modifiers: InputModifiers::default(),
            },
        });

        assert!(
            editor.drop_indicator_for_test().is_some(),
            "hovering below paragraph while fold is dragged must show drop indicator"
        );
    }

    // fold 드롭 → fold가 paragraph 다음으로 이동해야 한다
    #[test]
    fn fold_dnd_drop_moves_fold_after_paragraph() {
        let (initial, _root, _fold_node, p1) = state! {
            doc {
                root: root {
                    fold_node: fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("content") } }
                    }
                    p1: paragraph { text("after") }
                }
            }
            selection: (root, 0) -> (root, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state, &Position::new(p1, 5))
            .expect("cursor metrics")
            .caret;

        let page_bottom = editor.view().pages()[0].size.height;
        editor.apply(Message::Dnd {
            op: DndOp::StartInternalSelection,
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: caret.x,
                y: page_bottom - 1.0,
                modifiers: InputModifiers::default(),
            },
        });
        editor.apply(Message::Dnd {
            op: DndOp::Drop {
                page: 0,
                x: caret.x,
                y: page_bottom - 1.0,
                payload: DndDropPayload::InternalSelection,
                modifiers: InputModifiers::default(),
            },
        });

        let view = editor.state().view();
        let root_children: Vec<_> = view.node(Dot::ROOT).unwrap().child_blocks().collect();
        // ROOT schema requires a trailing Paragraph, so fulfill adds one after the inserted fold.
        // [paragraph("after"), fold, paragraph("")] = 3 children.
        assert!(
            root_children.len() >= 2,
            "root must have at least 2 children after move"
        );
        assert!(
            !matches!(root_children[0].node(), Node::Fold(_)),
            "fold must have moved away from index 0"
        );
        assert!(
            matches!(root_children[1].node(), Node::Fold(_)),
            "index 1 must be a Fold node (moved after paragraph)"
        );
    }

    // fold 위에 hover → fold 내부이므로 indicator가 나타나면 안 된다
    #[test]
    fn fold_dnd_over_inside_fold_shows_no_indicator() {
        let (initial, _root, _fold_node, _p1) = state! {
            doc {
                root: root {
                    _fold_node: fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("content") } }
                    }
                    p1: paragraph { text("after") }
                }
            }
            selection: (root, 0) -> (root, 1)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let page_top_y = editor.view().pages()[0].y_start + 1.0;

        editor.apply(Message::Dnd {
            op: DndOp::StartInternalSelection,
        });
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page: 0,
                x: 50.0,
                y: page_top_y,
                modifiers: InputModifiers::default(),
            },
        });

        assert!(
            editor.drop_indicator_for_test().is_none(),
            "hovering inside the dragged fold must not show a drop indicator"
        );
    }

    #[test]
    fn insert_template_fragment_replaces_empty_doc() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let (template, _p2, _p3) = state! {
            doc { root { p2: paragraph { text("Title") } p3: paragraph { text("Body") } } }
            selection: (p2, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor
            .insert_template_fragment(template.to_plain())
            .expect("template insert ok");

        let view = editor.state().view();
        let texts: Vec<String> = view
            .root()
            .unwrap()
            .child_blocks()
            .map(|p| p.inline_text())
            .collect();
        assert_eq!(texts, vec!["Title".to_string(), "Body".to_string()]);
    }

    #[test]
    fn insert_template_fragment_hides_placeholder() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let (template, _p2) = state! {
            doc { root { p2: paragraph { text("X") } } }
            selection: (p2, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor
            .insert_template_fragment(template.to_plain())
            .expect("template insert ok");
        assert!(editor.view().placeholder_metrics(editor.state()).is_none());
    }

    #[test]
    fn placeholder_metrics_follow_collapsed_input_modifiers() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(initial);

        let events = editor.apply(Message::Modifier {
            op: ModifierOp::Set {
                modifier: editor_model::Modifier::FontSize { value: 2400 },
            },
        });

        assert!(events.iter().any(|e| matches!(
            e,
            EditorEvent::StateChanged { fields } if fields.contains(&StateField::Placeholder)
        )));
        let metrics = editor
            .view()
            .placeholder_metrics(editor.state())
            .expect("empty document placeholder");
        assert_eq!(metrics.font_size, Some(2400));

        let events = editor.apply(Message::Modifier {
            op: ModifierOp::Set {
                modifier: editor_model::Modifier::LetterSpacing { value: 8 },
            },
        });

        assert!(events.iter().any(|e| matches!(
            e,
            EditorEvent::StateChanged { fields } if fields.contains(&StateField::Placeholder)
        )));
        let metrics = editor
            .view()
            .placeholder_metrics(editor.state())
            .expect("empty document placeholder");
        assert_eq!(metrics.font_size, Some(2400));
        assert_eq!(metrics.letter_spacing, Some(8));
    }

    #[test]
    fn insert_template_fragment_reseeds_root_modifiers() {
        // destination root has macro-default root.modifiers populated.
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("orig") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);

        let (tstate, ..) = state! {
            doc {
                root [] { tp: paragraph { text("tpl") } }
            }
            selection: (tp, 0)
        };
        let template = tstate.to_plain();

        editor.insert_template_fragment(template).unwrap();

        // destination's stale root.modifiers must be cleared and replaced by the
        // template's root modifiers (here: none).
        assert_eq!(
            editor
                .state()
                .projected
                .block_modifiers()
                .modifiers_of(Dot::ROOT)
                .len(),
            0,
            "stale root modifiers must be cleared"
        );
    }

    #[test]
    fn page_signature_changes_when_selection_extends_on_same_page() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello world foo bar baz") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);

        editor.state.selection = Some(Selection::new(Position::new(p1, 1), Position::new(p1, 3)));
        let sig_short = editor.page_render_signature(0);

        editor.state.selection = Some(Selection::new(Position::new(p1, 1), Position::new(p1, 12)));
        let sig_long = editor.page_render_signature(0);

        assert_ne!(
            sig_short, sig_long,
            "extending the selection on page 0 must change page 0's render signature"
        );
    }

    #[test]
    fn page_signature_stable_for_page_untouched_by_selection_change() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello world foo bar baz") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);

        editor.state.selection = Some(Selection::new(Position::new(p1, 1), Position::new(p1, 3)));
        let sig_before = editor.page_render_signature(7);

        editor.state.selection = Some(Selection::new(Position::new(p1, 1), Position::new(p1, 12)));
        let sig_after = editor.page_render_signature(7);

        assert_eq!(
            sig_before, sig_after,
            "a page with no selection-rect change keeps a stable signature so render_surface can skip it"
        );
    }

    #[test]
    fn page_signature_changes_after_text_edit() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.tick().unwrap();

        let sig_before = editor.page_render_signature(0);

        editor.enqueue(Message::Insertion {
            op: InsertionOp::Text {
                text: "x".to_string(),
            },
        });
        editor.tick().unwrap();

        let sig_after = editor.page_render_signature(0);
        assert_ne!(
            sig_before, sig_after,
            "a text edit changes content, so the render signature must change even with a collapsed caret"
        );
    }

    #[test]
    fn page_signature_unchanged_on_caret_move() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.tick().unwrap();

        let sig_before = editor.page_render_signature(0);

        editor.enqueue(Message::Navigation {
            op: NavigationOp::Move {
                movement: Movement::Grapheme {
                    direction: Direction::Forward,
                },
                extend: false,
            },
        });
        editor.tick().unwrap();

        let sig_after = editor.page_render_signature(0);
        assert_eq!(
            sig_before, sig_after,
            "moving a collapsed caret changes no surface pixels, so the render signature must stay stable"
        );
    }

    #[test]
    fn page_signature_changes_when_theme_variant_changes() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.tick().unwrap();

        let sig_before = editor.page_render_signature(0);

        editor
            .resource
            .lock()
            .unwrap()
            .theme
            .set_variant(ThemeVariant::DarkBlack);

        let sig_after = editor.page_render_signature(0);
        assert_ne!(
            sig_before, sig_after,
            "a theme variant change must change the render signature so a stale page isn't skipped"
        );
    }

    #[test]
    fn page_signature_changes_when_font_generation_changes() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.tick().unwrap();

        let sig_before = editor.page_render_signature(0);

        editor
            .resource
            .lock()
            .unwrap()
            .add_font_base("test", 400, compressed_test_font())
            .unwrap();

        let sig_after = editor.page_render_signature(0);
        assert_ne!(
            sig_before, sig_after,
            "a font_generation bump must change the render signature so a stale page isn't skipped"
        );
    }

    #[test]
    fn build_display_list_replay_matches_full_render() {
        use editor_renderer::backend::cpu::CpuSink;
        use editor_renderer::damage::IRect;
        let (state, _p) =
            state! { doc { root { p1: paragraph { text("hello world") } } } selection: (p1, 0) };
        let mut editor = Editor::new_test(state);
        editor.tick().unwrap();
        let sf = 2.0f32;
        let size = editor.view().pages()[0].size;
        let (pw, ph) = (
            (size.width * sf).round() as u16,
            (size.height * sf).round() as u16,
        );
        let full = IRect {
            x0: 0,
            y0: 0,
            x1: pw as i32,
            y1: ph as i32,
        };

        let mut a = CpuSink::new(pw, ph);
        editor.render_page(0, &mut a, sf);
        let mut buf_full = vec![0u8; pw as usize * ph as usize * 4];
        a.read_back_rect(&mut buf_full, pw as usize * 4, full);

        let (dl, _b) = editor.build_display_list(0, sf).unwrap();
        let mut c = CpuSink::new(pw, ph);
        editor_renderer::diff::replay(&dl, full, &mut c);
        let mut buf_dl = vec![0u8; pw as usize * ph as usize * 4];
        c.read_back_rect(&mut buf_dl, pw as usize * 4, full);

        assert_eq!(buf_full, buf_dl);
    }

    #[test]
    fn build_display_list_replay_matches_full_render_with_strokes() {
        use editor_renderer::backend::cpu::CpuSink;
        use editor_renderer::damage::IRect;
        let (state, _callout, _p1) = state! {
            doc { root { c: callout { p1: paragraph { text("quoted line") } } } } selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.tick().unwrap();
        let sf = 2.0f32;
        let size = editor.view().pages()[0].size;
        let (pw, ph) = (
            (size.width * sf).round() as u16,
            (size.height * sf).round() as u16,
        );
        let full = IRect {
            x0: 0,
            y0: 0,
            x1: pw as i32,
            y1: ph as i32,
        };

        let mut a = CpuSink::new(pw, ph);
        editor.render_page(0, &mut a, sf);
        let mut buf_full = vec![0u8; pw as usize * ph as usize * 4];
        a.read_back_rect(&mut buf_full, pw as usize * 4, full);

        let (dl, _b) = editor.build_display_list(0, sf).unwrap();
        let mut c = CpuSink::new(pw, ph);
        editor_renderer::diff::replay(&dl, full, &mut c);
        let mut buf_dl = vec![0u8; pw as usize * ph as usize * 4];
        c.read_back_rect(&mut buf_dl, pw as usize * 4, full);

        assert_eq!(buf_full, buf_dl);
    }

    #[test]
    fn build_display_list_records_text_content_primitive_for_real_font() {
        use editor_renderer::display_list::PrimPayload;
        let (state, _p) = state! { doc { root [font_family("test".to_string()), font_weight(400)] {
            p1: paragraph { text("hello") }
        } } selection: (p1, 0) };
        let mut editor = editor_with_real_font(state);

        let (dl, _b) = editor.build_display_list(0, 2.0).unwrap();

        assert!(
            dl.primitives.iter().any(|p| matches!(
                p.payload,
                PrimPayload::Glyph { .. } | PrimPayload::FillPath { .. }
            )),
            "build_display_list must record a text content primitive for real-font text"
        );
    }

    #[test]
    fn build_display_list_tolerates_out_of_range_page() {
        // Hosts render pages asynchronously from document mutations: when an
        // edit collapses the page count (select-all + type), the host's page
        // surfaces outlive the pages for one frame and still request a render
        // with the old index. That must report "no page", not panic.
        let (state, _p) =
            state! { doc { root { p1: paragraph { text("hello") } } } selection: (p1, 0) };
        let mut editor = Editor::new_test(state);
        editor.tick().unwrap();
        assert_eq!(editor.view().pages().len(), 1);
        assert!(editor.build_display_list(19, 2.0).is_none());
    }

    use editor_renderer::backend::cpu::CpuSink;
    use editor_renderer::display_list::DisplayList;
    fn full_page_buf(editor: &mut Editor, i: usize, sf: f32, w: u16, h: u16) -> Vec<u8> {
        let mut s = CpuSink::new(w, h);
        editor.render_page(i as u32, &mut s, sf);
        let mut buf = vec![0u8; w as usize * h as usize * 4];
        s.read_back_rect(
            &mut buf,
            w as usize * 4,
            IRect {
                x0: 0,
                y0: 0,
                x1: w as i32,
                y1: h as i32,
            },
        );
        buf
    }

    fn changes_within_damage(old: &[u8], new: &[u8], w: u16, damage: &[IRect]) -> bool {
        let pitch = w as usize * 4;
        for (idx, (a, b)) in old.chunks_exact(4).zip(new.chunks_exact(4)).enumerate() {
            if a != b {
                let px = (idx * 4 % pitch / 4) as i32;
                let py = (idx * 4 / pitch) as i32;
                if !damage
                    .iter()
                    .any(|r| r.x0 <= px && px < r.x1 && r.y0 <= py && py < r.y1)
                {
                    return false;
                }
            }
        }
        true
    }

    const TEST_FONT_TTF: &[u8] = include_bytes!("../../../assets/Pretendard-Regular.ttf");

    fn compressed_test_font() -> &'static [u8] {
        static C: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
        C.get_or_init(|| editor_resource::compress_zstd(TEST_FONT_TTF))
    }

    fn real_font_resource() -> Arc<Mutex<editor_resource::Resource>> {
        use editor_resource::{FontFamily, FontFamilySource, FontManifest, FontWeight};
        let mut res = editor_resource::Resource::new_test();
        res.set_fonts(vec![FontFamily {
            name: "test".into(),
            source: FontFamilySource::Default,
            weights: vec![FontWeight {
                value: 400,
                hash: "test-400".into(),
            }],
        }]);
        let test_id = res.font_registry.intern_id("test").unwrap();
        res.font_registry.set_manifest(
            test_id,
            400,
            FontManifest::from_coverages(&[vec![0x0000, 0xFFFF]]),
        );
        res.add_font_base("test", 400, compressed_test_font())
            .unwrap();
        Arc::new(Mutex::new(res))
    }

    fn root_font_modifiers() -> BTreeMap<ModifierType, Modifier> {
        BTreeMap::from([
            (
                ModifierType::FontFamily,
                Modifier::FontFamily {
                    value: "test".into(),
                },
            ),
            (
                ModifierType::FontWeight,
                Modifier::FontWeight { value: 400 },
            ),
        ])
    }

    fn editor_with_real_font(state: State) -> Editor {
        let res = real_font_resource();
        let mut editor = Editor::new_test_with_resource(state, Arc::clone(&res));
        editor.view = View::new(Viewport::new(800.0, 600.0, 1.0), res);
        editor.view.layout(&editor.state);
        editor.tick().unwrap();
        editor
    }

    fn big_doc_editor(n_paras: usize) -> Editor {
        let mut paras = Vec::with_capacity(n_paras);
        for _ in 0..n_paras {
            let text = PlainNodeEntry {
                node: PlainNode::Text(PlainTextNode {
                    text: "the quick brown fox jumps over the lazy dog".into(),
                }),
                modifiers: BTreeMap::new(),
                carry: Vec::new(),
                children: vec![],
            };
            paras.push(PlainNodeEntry {
                node: PlainNode::Paragraph(PlainParagraphNode {}),
                modifiers: BTreeMap::new(),
                carry: Vec::new(),
                children: vec![text],
            });
        }
        let plain = PlainDoc {
            root: PlainNodeEntry {
                node: PlainNode::Root(PlainRootNode {
                    layout_mode: LayoutMode::Paginated {
                        page_width: 400,
                        page_height: 600,
                        page_margin_top: 20,
                        page_margin_bottom: 20,
                        page_margin_left: 20,
                        page_margin_right: 20,
                    },
                }),
                modifiers: root_font_modifiers(),
                carry: Vec::new(),
                children: paras,
            },
        };
        let (mut state, handles) = editor_state::test_utils::build_state_from_plain(plain);
        state.selection = Some(Selection::collapsed(Position::new(handles[&vec![0, 0]], 0)));
        editor_with_real_font(state)
    }

    #[test]
    fn localized_edit_damage_is_small_and_correct() {
        let mut editor = big_doc_editor(80);
        let sf = 2.0f32;
        let size = editor.view().pages()[0].size;
        let (w, h) = (
            (size.width * sf).round() as u16,
            (size.height * sf).round() as u16,
        );
        let full = IRect {
            x0: 0,
            y0: 0,
            x1: w as i32,
            y1: h as i32,
        };

        let mut sink = CpuSink::new(w, h);
        let (dl0, _b) = editor.build_display_list(0, sf).unwrap();
        editor_renderer::diff::render_incremental(None, &dl0, &mut sink, full);
        let old_full = full_page_buf(&mut editor, 0, sf, w, h);
        let prev = dl0;

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "x".into() }],
        });
        editor.tick().unwrap();
        let (dl1, _b) = editor.build_display_list(0, sf).unwrap();
        let damage = editor_renderer::diff::render_incremental(Some(&prev), &dl1, &mut sink, full);

        let mut inc = vec![0u8; w as usize * h as usize * 4];
        sink.read_back_rect(&mut inc, w as usize * 4, full);
        let fresh = full_page_buf(&mut editor, 0, sf, w, h);

        assert_eq!(inc, fresh, "incremental must equal full render");
        let dmg_area: i64 = damage.iter().map(|r| r.area()).sum();
        assert!(
            dmg_area > 0 && dmg_area * 5 < full.area(),
            "localized edit damage {dmg_area} must be small vs full {}",
            full.area()
        );
        assert!(changes_within_damage(&old_full, &fresh, w, &damage));
    }

    struct SeedPage {
        sink: CpuSink,
        prev: DisplayList,
        w: u16,
        h: u16,
    }

    #[test]
    fn top_insert_reflows_lower_page() {
        let mut editor = big_doc_editor(60);
        let sf = 1.0f32;
        assert!(
            editor.view().pages().len() >= 2,
            "seed must span 2+ pages -- increase paragraph count"
        );

        let mut pages: Vec<SeedPage> = Vec::new();
        for i in 0..editor.view().pages().len() {
            let size = editor.view().pages()[i].size;
            let (w, h) = (
                (size.width * sf).round() as u16,
                (size.height * sf).round() as u16,
            );
            let full = IRect {
                x0: 0,
                y0: 0,
                x1: w as i32,
                y1: h as i32,
            };
            let mut sink = CpuSink::new(w, h);
            let (dl, _b) = editor.build_display_list(i as u32, sf).unwrap();
            editor_renderer::diff::render_incremental(None, &dl, &mut sink, full);
            pages.push(SeedPage {
                sink,
                prev: dl,
                w,
                h,
            });
        }

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection {
                text: "the quick brown fox jumps over the lazy dog ".repeat(20),
            }],
        });
        editor.tick().unwrap();

        let n = editor.view().pages().len();
        pages.truncate(n);
        let mut lower_damaged = false;
        for i in 0..n {
            let size = editor.view().pages()[i].size;
            let (w, h) = (
                (size.width * sf).round() as u16,
                (size.height * sf).round() as u16,
            );
            let full = IRect {
                x0: 0,
                y0: 0,
                x1: w as i32,
                y1: h as i32,
            };
            let (dl, _b) = editor.build_display_list(i as u32, sf).unwrap();
            if i >= pages.len() || pages[i].w != w || pages[i].h != h {
                let mut sink = CpuSink::new(w, h);
                editor_renderer::diff::render_incremental(None, &dl, &mut sink, full);
                let mut inc = vec![0u8; w as usize * h as usize * 4];
                sink.read_back_rect(&mut inc, w as usize * 4, full);
                let fresh = full_page_buf(&mut editor, i, sf, w, h);
                assert_eq!(inc, fresh, "new page {i} mismatch");
                continue;
            }
            let pb = &mut pages[i];
            let damage =
                editor_renderer::diff::render_incremental(Some(&pb.prev), &dl, &mut pb.sink, full);
            if i >= 1 && !damage.is_empty() {
                lower_damaged = true;
            }
            let mut inc = vec![0u8; w as usize * h as usize * 4];
            pb.sink.read_back_rect(&mut inc, w as usize * 4, full);
            let fresh = full_page_buf(&mut editor, i, sf, w, h);
            assert_eq!(
                inc, fresh,
                "page {i} incremental(prev->new) must equal full after top reflow"
            );
        }
        assert!(
            lower_damaged,
            "top insert must propagate damage to a lower page (reflow), not just page 0"
        );
    }
}
