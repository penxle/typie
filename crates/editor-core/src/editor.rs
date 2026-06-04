use editor_clipboard::Slice;
use editor_common::{Movement, time::Duration};
use editor_crdt::{Changeset, CrdtError, Dot, Op};
use editor_model::{DocOp, ModifierState, ModifierType, Node, NodeId};
use editor_renderer::{Mark, MarkData, RenderSink, Renderer};
#[cfg(any(test, feature = "test-utils"))]
use editor_resource::ThemeVariant;
use editor_resource::{CharacterCount, Resource, count_text};
use editor_state::{
    DocFlatExt, Position, ResolvedPosition, ResolvedPositionFlatExt, Selection, StableSelection,
    State,
};
use editor_transaction::{Effect, HistoryMeta, HistoryTag, Step, StepError, Transaction};
use editor_view::{GapPhantom, PageRect, PendingStyle, View, Viewport};
use hashbrown::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use strum::IntoEnumIterator;

use crate::block_state::BlockState;
use crate::dnd::DndState;
use crate::error::EditorError;
use crate::event::{EditorEvent, FontData};
use crate::handle;
use crate::history::History;
use crate::ime::{Ime, ImeRange};
use crate::message::*;
use crate::state_field::StateField;
use crate::tracked_range::TrackedRangeRegistry;

#[derive(Clone, Copy, Debug)]
enum Mode {
    Apply,
    Probe { changed: bool },
}

fn normalize_pending_style(state: &State) -> Option<PendingStyle> {
    if state.pending_modifiers.is_empty() {
        return None;
    }
    let selection = state.selection.as_ref()?;
    let textblock = state
        .doc
        .node(selection.head.node_id)?
        .ancestors()
        .find(|n| n.spec().is_textblock())?;
    let is_empty = textblock
        .children()
        .all(|c| matches!(c.node(), Node::Text(t) if t.text.is_empty()));
    if !is_empty {
        return None;
    }
    Some(PendingStyle {
        node_id: textblock.id(),
        modifiers: state.pending_modifiers.clone(),
    })
}

fn normalize_gap_phantom(state: &State) -> Option<GapPhantom> {
    use editor_state::GapCursor;
    let rs = state.selection.as_ref()?.resolve(&state.doc)?;
    match rs.as_gap_cursor()? {
        GapCursor::LeadingUnit { .. } => Some(GapPhantom {
            parent: NodeId::ROOT,
            index: 0,
        }),
        GapCursor::BetweenMonolithic { parent, index, .. } => Some(GapPhantom {
            parent: parent.id(),
            index,
        }),
    }
}

pub struct Editor {
    pub(crate) state: State,
    pub(crate) view: View,
    pub(crate) history: History,
    pub(crate) renderer: Renderer,
    pub(crate) resource: Arc<Mutex<Resource>>,
    pub(crate) tracked_ranges: TrackedRangeRegistry,

    // drag-and-drop state
    pub(crate) dnd: DndState,

    focused: bool,
    message_queue: Vec<Message>,
    pending_events: Vec<EditorEvent>,
    pub(crate) pending_ops: Vec<Op<DocOp>>,
    pending_effects: HashSet<Effect>,
    pub(crate) pending_fonts: HashMap<(String, u16), HashMap<NodeId, HashSet<u32>>>,
    mode: Mode,
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
            history: History::new(Duration::from_millis(300)),
            renderer: Renderer::new(Arc::clone(&resource)),
            resource,
            tracked_ranges: TrackedRangeRegistry::new(),
            dnd: DndState::default(),
            focused: false,
            message_queue: Vec::new(),
            pending_events: Vec::new(),
            pending_ops: Vec::new(),
            pending_effects: HashSet::new(),
            pending_fonts: HashMap::new(),
            mode: Mode::Apply,
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn tracked_ranges(&self) -> &TrackedRangeRegistry {
        &self.tracked_ranges
    }

    pub(crate) fn tracked_ranges_mut(&mut self) -> &mut TrackedRangeRegistry {
        &mut self.tracked_ranges
    }

    pub fn find_matches(&self, query: &str, options: &SearchOptions) -> Vec<Selection> {
        crate::search::find_matches(&self.state.doc, query, options)
    }

    pub(crate) fn is_probing(&self) -> bool {
        matches!(self.mode, Mode::Probe { .. })
    }

    pub(crate) fn mark_probed_change(&mut self, would_change: bool) {
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= would_change;
        }
    }

    pub fn receive_remote_changeset(&mut self, changeset: Changeset<DocOp>) {
        self.enqueue(Message::Remote { changeset });
    }

    pub fn local_changesets_since(
        &self,
        remote_heads: &HashSet<Dot>,
    ) -> Result<Vec<Changeset<DocOp>>, CrdtError> {
        self.state.local_changesets_since(remote_heads)
    }

    pub fn current_heads(&self) -> Vec<Dot> {
        self.state.graph.current_heads().copied().collect()
    }

    pub fn view(&self) -> &View {
        &self.view
    }

    pub fn modifier_state(&self) -> Option<ModifierState> {
        editor_state::resolve_modifier_state(&self.state)
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

    pub fn style_entries(&self) -> Vec<crate::style_state::StyleInfo> {
        crate::style_state::resolve_style_entries(&self.state.doc)
    }

    pub fn applied_style(&self) -> editor_common::Tri<crate::style_state::StyleRefValue> {
        crate::style_state::resolve_applied_style(&self.state)
    }

    pub fn style_divergence(&self) -> bool {
        crate::style_state::resolve_style_divergence(&self.state)
    }

    pub fn character_counts(&self) -> (CharacterCount, CharacterCount) {
        let doc_text = self.state.doc.extract_text();
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
        self.view
            .interactive_hit_test(&self.state.doc, page_idx, x, y)
    }

    pub fn page_link_rects(&self, page_idx: usize) -> Vec<editor_view::LinkRect> {
        self.view.page_link_rects(&self.state.doc, page_idx)
    }

    pub fn link_rects(&self) -> Vec<editor_view::LinkRect> {
        self.view.link_rects(&self.state.doc)
    }

    pub fn link_hit_test(&self, page_idx: usize, x: f32, y: f32) -> Option<editor_view::LinkRect> {
        self.view.link_hit_test(&self.state.doc, page_idx, x, y)
    }

    pub fn selection_endpoints(&self) -> Option<editor_view::SelectionEndpoints> {
        let resolved = self.state.selection.as_ref()?.resolve(&self.state.doc)?;
        self.view.selection_endpoints(&resolved)
    }

    pub fn selection_hit_test(&self, page_idx: usize, x: f32, y: f32) -> bool {
        let Some(resolved) = self
            .state
            .selection
            .as_ref()
            .and_then(|s| s.resolve(&self.state.doc))
        else {
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

        let mut scored: Vec<(usize, String, TrackedRangeHit)> = Vec::new();
        for range in iter {
            if range.explicitly_invalid {
                continue;
            }
            let Some(sel) = range.locate(&self.state.doc) else {
                continue;
            };
            let Some(resolved) = sel.resolve(&self.state.doc) else {
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
            let chars = if a >= h { a - h } else { h - a };

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

        let Some(current_head) = selection.head.resolve(&self.state.doc) else {
            return false;
        };
        let Some(hit_head) = hit_selection.head.resolve(&self.state.doc) else {
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
            .pointer_style_at(&self.state.doc, page_idx, x, y, read_only)
    }

    pub fn ime(&self, before_limit: usize, after_limit: usize) -> Result<Ime, EditorError> {
        let state = self.state();
        let doc = &state.doc;
        let doc_size = doc.flat_size();

        let sel = state.selection.ok_or_else(|| EditorError::General {
            msg: "IME requires an active selection".into(),
        })?;
        let anchor_flat = sel
            .anchor
            .resolve(doc)
            .ok_or_else(|| EditorError::General {
                msg: "invariant violated: state.selection.anchor must resolve against state.doc"
                    .into(),
            })?
            .to_flat();
        let head_flat = sel
            .head
            .resolve(doc)
            .ok_or_else(|| EditorError::General {
                msg: "invariant violated: state.selection.head must resolve against state.doc"
                    .into(),
            })?
            .to_flat();
        let (sel_start, sel_end) = (anchor_flat.min(head_flat), anchor_flat.max(head_flat));

        let window_start = sel_start.saturating_sub(before_limit);
        let window_end = sel_end.saturating_add(after_limit).min(doc_size);

        let text = doc.flat_text(window_start..window_end);
        let composing = state.composition.map(|c| ImeRange {
            start: c.start,
            end: c.end,
        });

        Ok(Ime {
            text,
            window_start,
            selection: ImeRange {
                start: sel_start,
                end: sel_end,
            },
            composing,
        })
    }

    pub fn enqueue(&mut self, msg: Message) {
        self.message_queue.push(msg);
    }

    pub fn tick(&mut self) -> Result<Vec<EditorEvent>, EditorError> {
        let old_doc = self.state.doc.clone();
        let old_selection = self.state.selection;
        let old_pending_modifiers = self.state.pending_modifiers.clone();
        let old_composition = self.state.composition;
        let old_last_history_tag_revision = self.history.last_tag_revision();

        let messages = std::mem::take(&mut self.message_queue);
        for msg in messages {
            self.process_message(msg)?;
        }

        if !self.pending_ops.is_empty() {
            self.augment_font_state_from_ops();
        }

        let effects = std::mem::take(&mut self.pending_effects);
        if !effects.is_empty() {
            self.process_effects(effects);
        }

        let ops = std::mem::take(&mut self.pending_ops);
        let pending_style = normalize_pending_style(&self.state);
        let gap_phantom = normalize_gap_phantom(&self.state);
        let dirty = self.view.reconcile_with_ops(
            &old_doc,
            &self.state.doc,
            &ops,
            pending_style,
            gap_phantom,
        );

        let mut fields: HashSet<StateField> = HashSet::new();

        if dirty {
            fields.insert(StateField::Cursor);
            fields.insert(StateField::PageSizes);
            fields.insert(StateField::ExternalElements);
            fields.insert(StateField::TableOverlays);
            fields.insert(StateField::LinkRects);
            fields.insert(StateField::Placeholder);
            self.push_event(EditorEvent::RenderInvalidated);
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
            .any(|op| matches!(&op.payload, DocOp::Style { .. } | DocOp::NodeStyle { .. }))
        {
            fields.insert(StateField::Styles);
        }

        if ops.iter().any(
            |op| matches!(&op.payload, DocOp::Attr { node_id, .. } if *node_id == NodeId::ROOT),
        ) {
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
            fields.insert(StateField::Styles);
            self.push_event(EditorEvent::RenderInvalidated);
        }

        if old_pending_modifiers != self.state.pending_modifiers {
            fields.insert(StateField::Modifiers);
        }

        if old_composition != self.state.composition {
            fields.insert(StateField::Ime);
        }

        if old_last_history_tag_revision != self.history.last_tag_revision() {
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
        for range in self.tracked_ranges.iter() {
            let Some(group) = view_state.group_decoration(&range.group) else {
                continue;
            };
            if !group.enabled {
                continue;
            }
            if range.explicitly_invalid {
                continue;
            }
            let Some(sel) = range.locate(&self.state.doc) else {
                continue;
            };
            let Some(resolved) = sel.resolve(&self.state.doc) else {
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

    fn selection_mark_rects(&self) -> Option<Vec<PageRect>> {
        let resolved = self.state.selection.as_ref()?.resolve(&self.state.doc)?;
        if resolved.is_collapsed() {
            return None;
        }
        let rects = self.view.selection_rects(&resolved);
        Some(rects.iter().map(|r| r.without_meta()).collect())
    }

    #[cfg(test)]
    pub(crate) fn cell_selection_rects_for_test(&self) -> Vec<PageRect> {
        self.selection_mark_rects().unwrap_or_default()
    }

    pub fn render_page(&mut self, page_idx: u32, sink: &mut dyn RenderSink, scale_factor: f32) {
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

        if let Some(composition) = self.state.composition
            && let (Some(from), Some(to)) = (
                ResolvedPosition::from_flat(&self.state.doc, composition.start),
                ResolvedPosition::from_flat(&self.state.doc, composition.end),
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

        self.renderer.render_page(
            sink,
            &self.state.doc,
            &self.view,
            page_idx as usize,
            scale_factor,
            &marks,
        );
    }

    pub fn export_page_vector(&mut self, page_idx: u32, scale_factor: f32) -> Vec<u8> {
        self.renderer.export_page_vector(
            &self.state.doc,
            &self.view,
            page_idx as usize,
            scale_factor,
        )
    }

    fn process_message(&mut self, msg: Message) -> Result<(), EditorError> {
        match msg {
            Message::Key { event } => handle::handle_key_event(self, event)?,
            Message::Insertion { op } => handle::handle_insertion_op(self, op)?,
            Message::Deletion { op } => handle::handle_deletion_op(self, op)?,
            Message::Modifier { op } => handle::handle_modifier_op(self, op)?,
            Message::Style { op } => handle::handle_style_op(self, op)?,
            Message::Selection { op } => handle::handle_selection_op(self, op)?,
            Message::Node { op } => handle::handle_node_op(self, op)?,
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
        let mut tr = Transaction::new(&self.state);
        f(&mut tr)?;

        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= editor_state::state_observably_changed(&self.state, tr.state());
            return Ok(());
        }

        let (state, steps, ops, effects, meta) = tr.commit();

        if !steps.is_empty() {
            match meta.history {
                HistoryMeta::Record => self.history.push(&steps),
                HistoryMeta::Tagged { tag } => self.history.push_tagged(&steps, tag),
                HistoryMeta::Skip => self.history.clear_last_tag(),
            }
        }

        self.state = state;
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

    fn augment_font_state_from_ops(&mut self) {
        let updates = {
            let resource = self.resource.lock().unwrap();
            crate::font::derive_font_updates_from_ops(
                &self.state.doc,
                &resource.font_registry,
                &self.pending_ops,
            )
        };

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
            self.push_event(EditorEvent::RenderInvalidated);
        }
    }

    pub(crate) fn resize_view(&mut self, viewport: Viewport) -> bool {
        let would_change = self.view.would_resize(viewport, &self.state.doc);
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= would_change;
            return would_change;
        }
        let did_change = self.view.resize(viewport, &self.state.doc);
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
            self.push_event(EditorEvent::RenderInvalidated);
        }
        did_change
    }

    pub(crate) fn set_external_height(&mut self, node_id: NodeId, height: f32) -> bool {
        let would_change = self
            .view
            .would_set_external_height(&self.state.doc, node_id, height);
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= would_change;
            return would_change;
        }
        let did_change = self
            .view
            .set_external_height(&self.state.doc, node_id, height);
        if did_change {
            self.push_event(EditorEvent::StateChanged {
                fields: vec![
                    StateField::Cursor,
                    StateField::PageSizes,
                    StateField::ExternalElements,
                ],
            });
            self.push_event(EditorEvent::RenderInvalidated);
        }
        did_change
    }

    pub(crate) fn toggle_fold(&mut self, id: NodeId) -> bool {
        let would_change = self.view.would_toggle_fold(&self.state.doc, id);
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= would_change;
            return would_change;
        }
        let did_change = self.view.toggle_fold(&self.state.doc, id);
        if did_change {
            self.push_event(EditorEvent::StateChanged {
                fields: vec![
                    StateField::Cursor,
                    StateField::PageSizes,
                    StateField::ExternalElements,
                ],
            });
            self.push_event(EditorEvent::RenderInvalidated);
        }
        did_change
    }

    pub(crate) fn fold_expanded(&self, id: NodeId) -> bool {
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
            return match probe_result {
                Some((sel, would_px)) => {
                    let sel_changed = sel != current_sel;
                    let px_changed = would_px != current_px;
                    if let Mode::Probe { ref mut changed } = self.mode {
                        *changed |= sel_changed || px_changed;
                    }
                    sel
                }
                None => None,
            };
        }
        let resource = self.resource.lock().unwrap();
        self.view.resolve_movement(pos, movement, &resource)
    }

    pub fn last_history_tag(&self) -> Option<&HistoryTag> {
        self.history.last_tag()
    }

    pub(crate) fn history_last_inverse_steps(&self) -> Option<Vec<Step>> {
        self.history.last_inverse_steps()
    }

    pub(crate) fn try_undo(&mut self) -> Option<Vec<Step>> {
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= self.history.can_undo();
            return None;
        }
        self.history.undo()
    }

    pub(crate) fn try_redo(&mut self) -> Option<Vec<Step>> {
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= self.history.can_redo();
            return None;
        }
        self.history.redo()
    }

    pub(crate) fn try_undo_auto_replacement(&mut self) -> Option<Vec<Step>> {
        let is_auto = matches!(self.history.last_tag(), Some(HistoryTag::AutoReplacement));
        if !is_auto {
            return None;
        }
        if let Mode::Probe { ref mut changed } = self.mode {
            *changed |= self.history.can_undo();
            return None;
        }
        self.history.undo()
    }

    pub(crate) fn sync_history_last_tag_from_top(&mut self) {
        if matches!(self.mode, Mode::Probe { .. }) {
            return;
        }
        self.history.sync_last_tag_from_top();
    }

    pub(crate) fn apply_remote_changeset(
        &mut self,
        changeset: Changeset<DocOp>,
    ) -> Result<(), EditorError> {
        if let Mode::Probe { ref mut changed } = self.mode {
            let would_change = self
                .state
                .would_receive_remote_changeset(&changeset)
                .map_err(|e| EditorError::Step(StepError::State(e)))?;
            *changed |= would_change;
            return Ok(());
        }

        let frozen = self
            .state
            .selection
            .as_ref()
            .map(|s| StableSelection::freeze(s, &self.state.doc));
        let (mut next, applied_ops) = self
            .state
            .receive_remote_changeset(changeset)
            .map_err(|e| EditorError::Step(StepError::State(e)))?;
        next.selection = frozen.map(|f| {
            let thawed = f.thaw(&next.doc);
            thawed.normalize(&next.doc).unwrap_or(thawed)
        });
        self.state = next;
        if !applied_ops.is_empty() {
            self.history.clear_last_tag();
        }
        self.pending_ops.extend(applied_ops);
        Ok(())
    }

    pub fn set_doc(&mut self, plain: editor_model::PlainDoc) {
        let (doc, graph) = editor_model::Doc::from_plain(plain);
        self.state = State::new(doc, graph, None);
        crate::font::reresolve_fonts(self).ok();
        self.view.layout(&self.state.doc);
        self.push_event(EditorEvent::StateChanged {
            fields: StateField::iter().collect(),
        });
        self.push_event(EditorEvent::RenderInvalidated);
    }

    pub fn insert_template_fragment(
        &mut self,
        template: editor_model::PlainDoc,
    ) -> Result<(), EditorError> {
        use editor_state::NodeRefCursorExt;

        let root_entry = template
            .nodes
            .get(&editor_model::NodeId::ROOT)
            .ok_or_else(|| EditorError::General {
                msg: "template has no ROOT".into(),
            })?;
        let root_plain_node = root_entry.node.clone();
        let root_modifiers: Vec<editor_model::Modifier> =
            root_entry.modifiers.values().cloned().collect();
        let child_ids: Vec<editor_model::NodeId> = root_entry.children.clone();

        let styles = template.styles.clone();
        let node_styles: Vec<(editor_model::NodeId, String)> = template
            .nodes
            .iter()
            .filter(|(id, _)| **id != editor_model::NodeId::ROOT)
            .filter_map(|(id, e)| e.style.clone().map(|s| (*id, s)))
            .collect();

        let (template_doc, _graph) = editor_model::Doc::from_plain(template);

        let subtrees: Vec<editor_model::Subtree> = child_ids
            .iter()
            .filter_map(|&id| editor_model::Subtree::capture(&template_doc, id))
            .collect();

        if subtrees.is_empty() {
            return Ok(());
        }

        self.transact(|tr| {
            tr.batch::<_, EditorError>(|tr| {
                let existing: Vec<editor_model::NodeId> = tr
                    .doc()
                    .node(editor_model::NodeId::ROOT)
                    .ok_or_else(|| EditorError::General {
                        msg: "ROOT not found".into(),
                    })?
                    .children()
                    .map(|c| c.id())
                    .collect();
                for id in existing {
                    tr.remove_subtree(id)?;
                }
                for (i, st) in subtrees.into_iter().enumerate() {
                    tr.insert_subtree(editor_model::NodeId::ROOT, i, st)?;
                }
                tr.set_node(editor_model::NodeId::ROOT, root_plain_node)?;
                for modifier in root_modifiers {
                    editor_commands::set_node_modifier(tr, editor_model::NodeId::ROOT, modifier)?;
                }
                for (name, entry) in styles {
                    tr.set_style(name, Some(entry))?;
                }
                for (id, name) in node_styles {
                    tr.set_node_style(id, Some(name))?;
                }
                Ok(())
            })?;

            let doc = tr.doc();
            if let Some(pos) = doc
                .node(editor_model::NodeId::ROOT)
                .and_then(|r| r.first_child())
                .and_then(|first| first.first_cursor_position())
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
        self.view.layout(&self.state.doc);
        self.push_event(EditorEvent::StateChanged {
            fields: StateField::iter().collect(),
        });
        Ok(())
    }

    pub(crate) fn retry_font_load(&mut self, family: &str) {
        if matches!(self.mode, Mode::Probe { .. }) {
            return;
        }
        crate::font::retry_pending_on_load(self, family);
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
        for &cp in codepoints {
            match resource.font_registry.resolve(family_id, weight, cp) {
                Resolution::Pending { target, .. } => {
                    grouped
                        .entry((target.family_id, target.weight))
                        .or_default()
                        .insert(target.chunk_id);
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

        drop(resource);
        for event in events {
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
        Self {
            state,
            view: View::new_test(),
            history: History::new(Duration::from_millis(300)),
            renderer: Renderer::new(Arc::clone(&resource)),
            resource,
            tracked_ranges: TrackedRangeRegistry::new(),
            dnd: DndState::default(),
            focused: false,
            message_queue: Vec::new(),
            pending_events: Vec::new(),
            pending_ops: Vec::new(),
            pending_effects: HashSet::new(),
            pending_fonts: HashMap::new(),
            mode: Mode::Apply,
        }
    }

    pub fn apply(&mut self, msg: Message) -> Vec<EditorEvent> {
        self.enqueue(msg);
        self.tick().unwrap()
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn history_undos_len(&self) -> usize {
        self.history.undos_len()
    }

    pub fn history_redos_len(&self) -> usize {
        self.history.redos_len()
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
    use editor_crdt::{OpGraph, RgaOp};
    use editor_macros::{doc, state};
    use editor_model::{
        Doc, DocOp, Modifier, ModifierType, Node, NodeId, PlainDoc, PlainNode, PlainNodeEntry,
        PlainParagraphNode, PlainRootNode, PlainTextNode,
    };
    use editor_state::{Affinity, PendingModifier, PendingModifiers, Position, Selection, State};
    use hashbrown::HashSet;
    use std::collections::BTreeMap;

    use super::*;

    /// Builds a two-replica pair from a shared PlainDoc so both replicas have
    /// identical NodeIds and a common base OpGraph — required for remote ops
    /// from replica A to resolve on replica B.
    fn bootstrap_two_replicas(plain: PlainDoc) -> (State, State) {
        let (doc, graph) = Doc::from_plain(plain);
        let sel = Selection::collapsed(Position::new(NodeId::ROOT, 0));
        let seed = State::new(doc, graph, Some(sel));

        let seed_css = seed.graph.changesets_as_vec();
        let replica_b =
            State::from_changesets(seed_css, Some(sel)).expect("from_changesets on bootstrap");
        (seed, replica_b)
    }

    fn root_default_font_modifiers() -> BTreeMap<ModifierType, Modifier> {
        // The editor's font resolver assumes every node can find a FontFamily and FontWeight
        // by walking ancestors up to the root. PlainDoc-built fixtures need to honor that
        // invariant explicitly because they bypass the doc!/state! macros that supply defaults.
        BTreeMap::from([
            (
                ModifierType::FontFamily,
                Modifier::FontFamily {
                    value: "Pretendard".into(),
                },
            ),
            (
                ModifierType::FontWeight,
                Modifier::FontWeight { value: 400 },
            ),
        ])
    }

    fn plain_doc_with_one_text() -> (PlainDoc, NodeId) {
        let para_id = NodeId::new();
        let text_id = NodeId::new();

        let mut nodes = BTreeMap::new();
        nodes.insert(
            NodeId::ROOT,
            PlainNodeEntry {
                parent: None,
                children: vec![para_id],
                modifiers: root_default_font_modifiers(),
                style: None,
                node: PlainNode::Root(PlainRootNode::default()),
            },
        );
        nodes.insert(
            para_id,
            PlainNodeEntry {
                parent: Some(NodeId::ROOT),
                children: vec![text_id],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Paragraph(PlainParagraphNode {}),
            },
        );
        nodes.insert(
            text_id,
            PlainNodeEntry {
                parent: Some(para_id),
                children: vec![],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Text(PlainTextNode {
                    text: "hi".to_string(),
                }),
            },
        );
        (
            PlainDoc {
                nodes,
                styles: BTreeMap::new(),
            },
            text_id,
        )
    }

    fn plain_doc_with_two_paragraphs() -> (PlainDoc, NodeId, NodeId, NodeId) {
        let para1_id = NodeId::new();
        let text1_id = NodeId::new();
        let para2_id = NodeId::new();
        let text2_id = NodeId::new();

        let mut nodes = BTreeMap::new();
        nodes.insert(
            NodeId::ROOT,
            PlainNodeEntry {
                parent: None,
                children: vec![para1_id, para2_id],
                modifiers: root_default_font_modifiers(),
                style: None,
                node: PlainNode::Root(PlainRootNode::default()),
            },
        );
        nodes.insert(
            para1_id,
            PlainNodeEntry {
                parent: Some(NodeId::ROOT),
                children: vec![text1_id],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Paragraph(PlainParagraphNode {}),
            },
        );
        nodes.insert(
            text1_id,
            PlainNodeEntry {
                parent: Some(para1_id),
                children: vec![],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Text(PlainTextNode {
                    text: String::new(),
                }),
            },
        );
        nodes.insert(
            para2_id,
            PlainNodeEntry {
                parent: Some(NodeId::ROOT),
                children: vec![text2_id],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Paragraph(PlainParagraphNode {}),
            },
        );
        nodes.insert(
            text2_id,
            PlainNodeEntry {
                parent: Some(para2_id),
                children: vec![],
                modifiers: BTreeMap::new(),
                style: None,
                node: PlainNode::Text(PlainTextNode {
                    text: String::new(),
                }),
            },
        );
        (
            PlainDoc {
                nodes,
                styles: BTreeMap::new(),
            },
            para1_id,
            para2_id,
            text2_id,
        )
    }

    fn build_state(doc: editor_model::Doc, head: Position, pending: PendingModifiers) -> State {
        let mut s = State::new(
            doc,
            OpGraph::<DocOp>::new(),
            Some(Selection::collapsed(head)),
        );
        s.pending_modifiers = pending;
        s
    }

    #[test]
    fn normalize_empty_pending_returns_none() {
        let (doc, p1) = doc! { root { p1: paragraph } };
        let s = build_state(doc, Position::new(p1, 0), PendingModifiers::new());
        assert!(normalize_pending_style(&s).is_none());
    }

    #[test]
    fn normalize_non_empty_paragraph_returns_none() {
        let (doc, p1) = doc! { root { p1: paragraph { text("hi") } } };
        let pending: PendingModifiers = vec![PendingModifier::Set {
            modifier: Modifier::Bold,
        }];
        let s = build_state(doc, Position::new(p1, 0), pending);
        assert!(normalize_pending_style(&s).is_none());
    }

    #[test]
    fn normalize_empty_paragraph_returns_some_with_container_id() {
        let (doc, p1) = doc! { root { p1: paragraph } };
        let pending: PendingModifiers = vec![PendingModifier::Set {
            modifier: Modifier::FontSize { value: 9600 },
        }];
        let s = build_state(doc, Position::new(p1, 0), pending.clone());
        let ps = normalize_pending_style(&s).expect("Some");
        assert_eq!(ps.node_id, p1);
        assert_eq!(ps.modifiers, pending);
    }

    #[test]
    fn normalize_head_on_text_child_ascends_to_textblock() {
        let (doc, _p1, t1) = doc! { root { _p1: paragraph { t1: text("hi") } } };
        let pending: PendingModifiers = vec![PendingModifier::Set {
            modifier: Modifier::Bold,
        }];
        let s = build_state(doc, Position::new(t1, 0), pending);
        assert!(normalize_pending_style(&s).is_none());
    }

    fn test_editor() -> (Editor, NodeId) {
        let (state, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        (Editor::new_test(state), t)
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
    fn undo_after_typing_over_multi_paragraph_selection_restores_initial_selection() {
        use editor_state::assert_state_eq;

        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("a") }
                    paragraph { text("b") }
                    paragraph { t2: text("c") }
                }
            }
            selection: (t1, 0) -> (t2, 1)
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

    #[test]
    fn perf_undo_20_chars() {
        struct StderrLogger;
        impl log::Log for StderrLogger {
            fn enabled(&self, _m: &log::Metadata) -> bool {
                true
            }
            fn log(&self, record: &log::Record) {
                eprintln!("{}", record.args());
            }
            fn flush(&self) {}
        }
        let _ = log::set_logger(Box::leak(Box::new(StderrLogger)));
        log::set_max_level(log::LevelFilter::Info);

        let (state, _) = state! {
            doc { root { paragraph { t: text("") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.view.layout(&editor.state.doc);

        for ch in "abcdefghijklmnopqrst".chars() {
            editor.apply(Message::Insertion {
                op: InsertionOp::Text {
                    text: ch.to_string(),
                },
            });
        }

        eprintln!("==== UNDO STARTS ====");
        let t = editor_common::time::Instant::now();
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        eprintln!("==== UNDO TICK TOTAL: {:?} ====", t.elapsed());
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
                    chunks: vec![vec![0x41, 0x42]],
                }],
            }];
            resource.set_fonts(families);
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
    fn input_context_full_window_returns_whole_doc() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        let editor = Editor::new_test(state);
        let ctx = editor.ime(usize::MAX, usize::MAX).unwrap();
        // flat: O(p)=0, "hello"=1..6, C(p)=6 → flat_size=7
        // (t1,2) → flat 3; window covers full doc [0,7)
        assert_eq!(ctx.text, "\u{2028}hello\u{2029}");
        assert_eq!(ctx.window_start, 0);
        assert_eq!(ctx.selection.start, 3);
        assert_eq!(ctx.selection.end, 3);
        assert!(ctx.composing.is_none());
    }

    #[test]
    fn input_context_limited_window_clamps() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 6)
        };
        let editor = Editor::new_test(state);
        let ctx = editor.ime(3, 3).unwrap();
        // flat: O(p)=0, "hello world"=1..12, C(p)=12 → flat_size=13
        // (t1,6) → flat 7; window [7-3, 7+3) = [4, 10) → "lo wor"
        assert_eq!(ctx.window_start, 4);
        assert_eq!(ctx.text, "lo wor");
        assert_eq!(ctx.selection.start, 7);
        assert_eq!(ctx.selection.end, 7);
    }

    #[test]
    fn input_context_with_non_collapsed_selection() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 2) -> (t1, 8)
        };
        let editor = Editor::new_test(state);
        let ctx = editor.ime(usize::MAX, usize::MAX).unwrap();
        // flat: O(p)=0, "hello world"=1..12, C(p)=12 → flat_size=13
        // (t1,2)→flat 3, (t1,8)→flat 9; window covers full doc [0,13)
        assert_eq!(ctx.text, "\u{2028}hello world\u{2029}");
        assert_eq!(ctx.selection.start, 3);
        assert_eq!(ctx.selection.end, 9);
    }

    #[test]
    fn input_context_empty_blockquote_has_tokens() {
        let (state, ..) = state! {
            doc { root { blockquote { paragraph { t1: text("") } } paragraph {} } }
            selection: (t1, 0)
        };
        let editor = Editor::new_test(state);
        let ctx = editor.ime(100, 100).unwrap();
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
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hi") [bold] } } }
            selection: (t1, 1)
        };
        let editor = Editor::new_test(state);
        let s = editor.modifier_state().unwrap();
        assert_eq!(s.bold, editor_common::Tri::Uniform { value: () });
    }

    #[test]
    fn editor_exposes_uniform_link_for_paragraph_child_range_selection() {
        let (state, _p1, ..) = state! {
            doc { root { p1: paragraph {
                t1: text("Hello") [link(href: "https://a.com".to_string())]
                t2: text("World") [link(href: "https://a.com".to_string())]
            } } }
            selection: (p1, 0) -> (p1, 2)
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
        let (state, _p1, ..) = state! {
            doc { root { p1: paragraph {
                t1: text("Hello") [link(href: "https://a.com".to_string())]
                t2: text("World") [link(href: "https://b.com".to_string())]
            } } }
            selection: (p1, 0) -> (p1, 2)
        };
        let editor = Editor::new_test(state);
        let s = editor.modifier_state().unwrap();
        assert_eq!(s.link, editor_common::Tri::Mixed);
    }

    #[test]
    fn editor_exposes_block_state() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hi") } } }
            selection: (t1, 1)
        };
        let editor = Editor::new_test(state);
        let bs = editor.block_state().unwrap();
        assert_eq!(bs.ancestors.len(), 2);
        assert!(bs.nodes.is_empty());
    }

    #[test]
    fn character_counts_empty_doc_is_all_zero() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("") } } }
            selection: (t1, 0)
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
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
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
        use editor_crdt::TextOp;

        let (plain, text_id) = plain_doc_with_one_text();
        let (state_a, state_b) = bootstrap_two_replicas(plain);

        // Snapshot heads before A's edit so local_changesets_since returns only the new op.
        let baseline: HashSet<_> = state_a.graph.current_heads().copied().collect();

        let (state_a, _) = state_a
            .apply(DocOp::Text {
                node_id: text_id,
                op: TextOp::InsertChar {
                    after: None,
                    ch: 'x',
                },
            })
            .unwrap();
        let state_a = State {
            graph: state_a.graph.commit(),
            ..state_a
        };

        let mut css = state_a.local_changesets_since(&baseline).unwrap();
        let cs = css.remove(0);

        // Pre-layout populates the measurer cache; without this, invalidation has nothing
        // to evict and dirty stays false.
        let mut editor = Editor::new_test(state_b);
        editor.view.layout(&editor.state.doc);

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

        let entry = editor
            .state
            .doc
            .get_entry(text_id)
            .expect("text node exists");
        let Node::Text(t) = &entry.node else {
            panic!("t1 must be Text")
        };
        assert!(
            t.text.iter_with_dot().any(|(_, c)| c == 'x'),
            "remote insert applied"
        );
    }

    #[test]
    fn multiple_remote_messages_processed_in_single_tick() {
        use editor_crdt::TextOp;

        let (plain, text_id) = plain_doc_with_one_text();
        let (state_a, state_b) = bootstrap_two_replicas(plain);

        let baseline1: HashSet<_> = state_a.graph.current_heads().copied().collect();
        let (state_a, _) = state_a
            .apply(DocOp::Text {
                node_id: text_id,
                op: TextOp::InsertChar {
                    after: None,
                    ch: 'x',
                },
            })
            .unwrap();
        let state_a = State {
            graph: state_a.graph.commit(),
            ..state_a
        };
        let cs1 = state_a
            .local_changesets_since(&baseline1)
            .unwrap()
            .remove(0);

        let baseline2: HashSet<_> = state_a.graph.current_heads().copied().collect();
        let (state_a, _) = state_a
            .apply(DocOp::Text {
                node_id: text_id,
                op: TextOp::InsertChar {
                    after: None,
                    ch: 'y',
                },
            })
            .unwrap();
        let state_a = State {
            graph: state_a.graph.commit(),
            ..state_a
        };
        let cs2 = state_a
            .local_changesets_since(&baseline2)
            .unwrap()
            .remove(0);

        let baseline3: HashSet<_> = state_a.graph.current_heads().copied().collect();
        let (state_a, _) = state_a
            .apply(DocOp::Text {
                node_id: text_id,
                op: TextOp::InsertChar {
                    after: None,
                    ch: 'z',
                },
            })
            .unwrap();
        let state_a = State {
            graph: state_a.graph.commit(),
            ..state_a
        };
        let cs3 = state_a
            .local_changesets_since(&baseline3)
            .unwrap()
            .remove(0);

        let mut editor = Editor::new_test(state_b);
        editor.view.layout(&editor.state.doc);

        editor.receive_remote_changeset(cs1);
        editor.receive_remote_changeset(cs2);
        editor.receive_remote_changeset(cs3);
        let events = editor.tick().unwrap();

        let entry = editor
            .state
            .doc
            .get_entry(text_id)
            .expect("text node exists");
        let Node::Text(t) = &entry.node else {
            panic!("t1 must be Text")
        };
        let text_str = t.text.to_string();
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
        use editor_crdt::OrMapOp;

        let (plain, _para1_id, para2_id, text2_id) = plain_doc_with_two_paragraphs();
        let (state_a, state_b) = bootstrap_two_replicas(plain);

        let baseline: HashSet<_> = state_a.graph.current_heads().copied().collect();

        // Read all dot lookups up front so the batch closure below doesn't need to reborrow state_a.
        let para2_dot = {
            let root_entry = state_a.doc.get_entry(NodeId::ROOT).expect("root exists");
            root_entry
                .children
                .iter_with_dot()
                .find(|(_, v)| **v == para2_id)
                .map(|(d, _)| d)
                .expect("para2 must be in root's children")
        };
        let text2_presence_dots: Vec<_> = state_a.doc.nodes_tags_for(&text2_id).copied().collect();
        let para2_presence_dots: Vec<_> = state_a.doc.nodes_tags_for(&para2_id).copied().collect();

        let (state_a, _ops) = state_a
            .batch_with_ops::<_, editor_state::StateError>(|b| {
                b.apply(DocOp::Presence {
                    node_id: text2_id,
                    op: OrMapOp::Unset {
                        observed: text2_presence_dots.clone(),
                    },
                })?;
                b.apply(DocOp::Presence {
                    node_id: para2_id,
                    op: OrMapOp::Unset {
                        observed: para2_presence_dots.clone(),
                    },
                })?;
                b.apply(DocOp::Children {
                    node_id: NodeId::ROOT,
                    op: RgaOp::Remove {
                        observed: para2_dot,
                    },
                })?;
                Ok(())
            })
            .unwrap();
        let state_a = State {
            graph: state_a.graph.commit(),
            ..state_a
        };
        let cs = state_a.local_changesets_since(&baseline).unwrap().remove(0);

        // Pre-layout so both paragraphs are cached; otherwise sibling-shift invalidation
        // has nothing to evict and dirty stays false.
        let mut editor = Editor::new_test(state_b);
        editor.view.layout(&editor.state.doc);

        editor.receive_remote_changeset(cs);
        let events = editor.tick().unwrap();

        // Multi-op changesets must still emit a single pair of events — covers dedup
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

        let root_after = editor
            .state
            .doc
            .get_entry(NodeId::ROOT)
            .expect("root exists");
        let still_present = root_after.children.iter().any(|&c| c == para2_id);
        assert!(
            !still_present,
            "para2 must no longer be a live child of root after removal"
        );
    }

    #[test]
    fn editor_interactive_hit_test_delegates_to_view() {
        use editor_macros::state;
        let (initial, f1, ft1, ..) = state! {
            doc { root {
                f1: fold {
                    ft1: fold_title { t1: text("Title") }
                    fold_content { paragraph { text("Body") } }
                }
            } }
            selection: (t1, 0)
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
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 1) -> (t, 8)
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
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        assert!(editor.selection_endpoints().is_none());
    }

    #[test]
    fn editor_selection_hit_test_inside_rect() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0) -> (t, 5)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });

        let resolved = editor
            .state
            .selection
            .expect("selection exists in test")
            .resolve(&editor.state.doc)
            .unwrap();
        let rect = editor.view().selection_rects(&resolved)[0].rect;
        let probe_x = rect.x + rect.width * 0.5;
        let probe_y = rect.y + rect.height * 0.5;
        assert!(editor.selection_hit_test(0, probe_x, probe_y));
    }

    #[test]
    fn editor_selection_hit_test_collapsed_is_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 2)
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
        editor.view.layout(&editor.state.doc);
        editor.state.selection =
            Some(editor_state::cell_rect_selection(&editor.state.doc, c00, c10).unwrap());

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
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });

        let sel = editor.state.selection.expect("selection exists in test");
        let cursor = editor
            .view()
            .cursor_metrics(&editor.state.doc, &sel.head)
            .expect("collapsed cursor has metrics");
        let probe_x = cursor.caret.x;
        let probe_y = cursor.line.y + cursor.line.height * 0.5;

        assert!(editor.cursor_hit_test(0, probe_x, probe_y));
    }

    #[test]
    fn editor_cursor_hit_test_rejects_different_position() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });

        let sel = editor.state.selection.expect("selection exists in test");
        let cursor = editor
            .view()
            .cursor_metrics(&editor.state.doc, &sel.head)
            .expect("collapsed cursor has metrics");
        let probe_x = cursor.caret.x + 100.0;
        let probe_y = cursor.line.y + cursor.line.height * 0.5;

        assert!(!editor.cursor_hit_test(0, probe_x, probe_y));
    }

    #[test]
    fn editor_cursor_hit_test_rejects_range_selection() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0) -> (t, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });

        assert!(!editor.cursor_hit_test(0, 10.0, 10.0));
    }

    #[test]
    fn dnd_over_text_sets_drop_indicator_and_invalidates_render() {
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state.doc, &Position::new(t, 2))
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
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state.doc, &Position::new(t, 2))
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
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state.doc, &Position::new(t, 2))
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
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0) -> (t, 2)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state.doc, &Position::new(t, 5))
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
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 1) -> (t, 4)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state.doc, &Position::new(t, 2))
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
        let (initial, _p1, t2) = state! {
            doc { root {
                p1: paragraph { text("a") page_break }
                blockquote { paragraph { t2: text("inside") } }
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
            .cursor_metrics(&editor.state.doc, &Position::new(t2, 2))
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
    fn internal_dnd_page_break_source_allows_root_inline_drop_indicator() {
        let (initial, _p1, t2) = state! {
            doc { root {
                p1: paragraph { text("a") page_break }
                paragraph { t2: text("root") }
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
            .cursor_metrics(&editor.state.doc, &Position::new(t2, 2))
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
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state.doc, &Position::new(t, 5))
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

        let (expected, ..) = state! {
            doc { root { paragraph { t: text("hello!") } } }
            selection: (t, 5) -> (t, 6)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn drop_text_without_external_enter_is_noop() {
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(initial.clone());
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state.doc, &Position::new(t, 5))
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
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(initial.clone());
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state.doc, &Position::new(t, 5))
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
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state.doc, &Position::new(t, 5))
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

        let (expected, ..) = state! {
            doc { root { paragraph { t: text("helloplain") } } }
            selection: (t, 5) -> (t, 10)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn drop_files_inserts_placeholders_at_drop_target_not_current_selection() {
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state.doc, &Position::new(t, 5))
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

        let root = editor.state().doc.node(editor_model::NodeId::ROOT).unwrap();
        let children: Vec<_> = root.children().map(|c| c.node().clone()).collect();
        assert!(matches!(
            children.first(),
            Some(editor_model::Node::Paragraph(_))
        ));
        assert!(matches!(
            children.get(1),
            Some(editor_model::Node::Image(_))
        ));
        assert!(matches!(children.get(2), Some(editor_model::Node::File(_))));
        assert!(
            matches!(children.last(), Some(editor_model::Node::Paragraph(_))),
            "root schema should keep a trailing paragraph after inserted file blocks",
        );
        let first_text = root
            .children()
            .next()
            .and_then(|p| p.first_child())
            .and_then(|n| match n.node() {
                editor_model::Node::Text(t) => Some(t.text.to_string()),
                _ => None,
            });
        assert_eq!(first_text.as_deref(), Some("hello"));
    }

    #[test]
    fn drop_internal_selection_without_start_is_noop() {
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0) -> (t, 5)
        };
        let mut editor = Editor::new_test(initial.clone());
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state.doc, &Position::new(t, 11))
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
        let (initial, root, t) = state! {
            doc { root: root {
                image
                paragraph { t: text("hello") }
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
            position: Position::new(t, 3),
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

        let root_node = editor.state().doc.node(root).expect("root exists");
        let children: Vec<_> = root_node.children().map(|c| c.node()).collect();
        assert!(matches!(
            children.as_slice(),
            [Node::Paragraph(_), Node::Image(_), Node::Paragraph(_)]
        ));
        let first_text = root_node
            .children()
            .next()
            .and_then(|p| p.first_child())
            .and_then(|n| match n.node() {
                Node::Text(t) => Some(t.text.to_string()),
                _ => None,
            });
        let last_text = root_node
            .children()
            .nth(2)
            .and_then(|p| p.first_child())
            .and_then(|n| match n.node() {
                Node::Text(t) => Some(t.text.to_string()),
                _ => None,
            });
        assert_eq!(first_text.as_deref(), Some("hel"));
        assert_eq!(last_text.as_deref(), Some("lo"));
        assert_eq!(
            editor.state().selection,
            Some(Selection::new(
                Position {
                    node_id: root,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: root,
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
                position: Position::new(NodeId::ROOT, 1),
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

        let root = editor.state().doc.node(NodeId::ROOT).unwrap();
        let children: Vec<_> = root.children().map(|c| c.node()).collect();
        assert!(matches!(
            children.as_slice(),
            [
                Node::Paragraph(_),
                Node::Paragraph(_),
                Node::Image(_),
                Node::Paragraph(_),
            ]
        ));
        let inserted = root.children().nth(1).unwrap();
        let inserted_text = inserted.first_child().and_then(|n| match n.node() {
            Node::Text(t) => Some(t.text.to_string()),
            _ => None,
        });
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
                position: Position::new(NodeId::ROOT, 1),
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

        let root = editor.state().doc.node(NodeId::ROOT).unwrap();
        let children: Vec<_> = root.children().map(|c| c.node()).collect();
        assert!(matches!(
            children.as_slice(),
            [
                Node::Paragraph(_),
                Node::Paragraph(_),
                Node::File(_),
                Node::Paragraph(_),
            ]
        ));
        let inserted = root.children().nth(1).unwrap();
        let inserted_text = inserted.first_child().and_then(|n| match n.node() {
            Node::Text(t) => Some(t.text.to_string()),
            _ => None,
        });
        assert_eq!(inserted_text.as_deref(), Some("dropped"));
    }

    #[test]
    fn internal_drop_move_remaps_target_after_deleting_source() {
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0) -> (t, 5)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state.doc, &Position::new(t, 11))
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

        let (expected, ..) = state! {
            doc { root { paragraph { t: text(" worldhello") } } }
            selection: (t, 6) -> (t, 11)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn internal_drop_copy_preserves_source_selection_content() {
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello world") } } }
            selection: (t, 0) -> (t, 5)
        };
        let mut editor = Editor::new_test(initial);
        editor.apply(Message::System {
            event: crate::message::SystemEvent::Initialize,
        });
        let caret = editor
            .view()
            .cursor_metrics(&editor.state.doc, &Position::new(t, 11))
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

        let (expected, ..) = state! {
            doc { root { paragraph { t: text("hello worldhello") } } }
            selection: (t, 11) -> (t, 16)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn normalize_gap_phantom_some_for_leading_unit() {
        let (state, ..) = state! {
            doc { root: root { image paragraph { text("b") } } }
            selection: (root, 0, <)
        };
        let gp = super::normalize_gap_phantom(&state).expect("leading image is a gap");
        assert_eq!(gp.parent, editor_model::NodeId::ROOT);
        assert_eq!(gp.index, 0);
    }

    #[test]
    fn normalize_gap_phantom_none_for_normal_caret() {
        let (state, ..) = state! {
            doc { root { paragraph { t: text("hi") } } }
            selection: (t, 1)
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
        assert_eq!(gp.parent, editor_model::NodeId::ROOT);
        assert_eq!(gp.index, 1);
    }

    #[test]
    fn gap_cursor_yields_caret_then_recovers() {
        let (state, _, t) = state! {
            doc { root: root { image paragraph { t: text("b") } } }
            selection: (root, 0, <)
        };
        let mut editor = Editor::new_test(state);
        // Prime the measurer cache before the gap appears; reconcile needs a
        // populated layout to invalidate when gap_phantom flips on. layout()
        // itself clears gap_phantom, so it must run before the gap tick.
        editor.view.layout(&editor.state.doc);
        let _ = editor.tick().unwrap();
        let st = editor.state();
        assert!(
            editor
                .view()
                .cursor_metrics(
                    &st.doc,
                    &st.selection.expect("selection exists in test").head
                )
                .is_some(),
            "gap cursor must produce a caret via the phantom line"
        );

        editor.enqueue(Message::Selection {
            op: SelectionOp::Set {
                selection: Selection::collapsed(Position::new(t, 1)),
            },
        });
        let _ = editor.tick().unwrap();
        let st2 = editor.state();
        assert!(
            editor
                .view()
                .cursor_metrics(
                    &st2.doc,
                    &st2.selection.expect("selection exists in test").head
                )
                .is_some(),
            "normal caret still valid after leaving the gap (phantom space recovered)"
        );
    }

    #[test]
    fn no_op_messages_when_selection_is_none() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("Hello") } } }
            selection: none
        };
        let mut editor = Editor::new_test(initial);

        let pre_doc = editor.state().doc.clone();
        let pre_selection = editor.state().selection;
        let pre_can_undo = editor.history.can_undo();

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

        assert_eq!(
            editor.state().doc,
            pre_doc,
            "doc must not change when selection is None"
        );
        assert_eq!(
            editor.state().selection,
            pre_selection,
            "selection must stay None"
        );
        assert_eq!(
            editor.history.can_undo(),
            pre_can_undo,
            "no history entries pushed"
        );
    }

    #[test]
    fn can_returns_true_for_insertion_with_selection() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        let probed = editor.can(Message::Insertion {
            op: InsertionOp::Text { text: "x".into() },
        });
        assert_eq!(probed.unwrap(), true);
    }

    #[test]
    fn can_returns_false_for_undo_with_empty_history() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        let probed = editor.can(Message::History {
            op: HistoryOp::Undo,
        });
        assert_eq!(probed.unwrap(), false);
    }

    #[test]
    fn can_returns_false_for_set_same_selection() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        let mut editor = Editor::new_test(state);
        let same = editor_state::Selection::collapsed(editor_state::Position::new(t1, 2));
        let probed = editor.can(Message::Selection {
            op: SelectionOp::Set { selection: same },
        });
        assert_eq!(probed.unwrap(), false);
    }

    #[test]
    fn can_does_not_mutate_state_for_insertion() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        let before_doc = editor.state().doc.clone();
        let before_sel = editor.state().selection;
        let _ = editor.can(Message::Insertion {
            op: InsertionOp::Text { text: "x".into() },
        });
        assert_eq!(editor.state().doc, before_doc);
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
        editor.view.layout(&editor.state.doc);

        let sel = editor_state::cell_rect_selection(&editor.state.doc, c00, c11).unwrap();
        editor.state.selection = Some(sel);

        let resolved = editor
            .state
            .selection
            .expect("selection set above")
            .resolve(&editor.state.doc)
            .unwrap();
        assert!(
            resolved.as_cell_rect().is_some(),
            "precondition: selection is a cell-rect"
        );

        let ids: Vec<_> = resolved
            .as_cell_rect()
            .unwrap()
            .cells()
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

    fn fold_editor_with_unit_selection() -> (Editor, NodeId, NodeId) {
        let (initial, root, fold_node, _para_text) = state! {
            doc {
                root: root {
                    fold_node: fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("content") } }
                    }
                    paragraph { _para_text: text("after") }
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
        let (initial, r, _fnode, para_text) = state! {
            doc {
                r: root {
                    _fnode: fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("content") } }
                    }
                    paragraph { para_text: text("after") }
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
            .cursor_metrics(&editor.state.doc, &Position::new(para_text, 5))
            .expect("cursor metrics")
            .caret;

        let target = editor
            .view
            .drop_target_at(&editor.state.doc, 0, caret.x, page_bottom - 1.0);

        assert!(
            target.is_some(),
            "drop_target_at must return Some for y=page_bottom-1 with fold+paragraph doc, caret at y={}, height={}",
            caret.y,
            caret.height
        );
        if let Some(t) = target {
            assert_eq!(
                t.position,
                Position::new(NodeId::ROOT, 2),
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
        let (initial, root, _fold_node, _para_text) = state! {
            doc {
                root: root {
                    _fold_node: fold {
                        fold_title { text("t") }
                        fold_content { paragraph { text("c") } }
                    }
                    paragraph { _para_text: text("after") }
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

        let DndState::InternalDnd { ref source, .. } = editor.dnd else {
            panic!("expected InternalDnd");
        };
        let sel = source.thaw(&editor.state.doc);

        // Extract slice
        let mut state = editor.state().clone();
        state.selection = Some(sel.clone());
        let slice = Slice::extract(&state).expect("Slice::extract must succeed");

        // Print slice content for debugging
        eprintln!("slice fragment node: {:?}", slice.fragment.node);
        eprintln!(
            "slice fragment modifiers count: {}",
            slice.fragment.modifiers.len()
        );
        for m in &slice.fragment.modifiers {
            eprintln!("  root modifier: {:?}", m);
        }
        for (i, child) in slice.fragment.children.iter().enumerate() {
            eprintln!(
                "  child[{}]: {:?}, modifiers: {:?}",
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
        let stable_target = StableSelection::freeze(
            &Selection::collapsed(Position::new(NodeId::ROOT, 2)),
            &editor.state.doc,
        );
        let r1 = editor.transact(|tr| {
            commands::set_selection(tr, sel.clone())
                .map_err(|e| crate::error::EditorError::from(e))?;
            commands::delete_selection(tr).map_err(|e| crate::error::EditorError::from(e))?;
            Ok(())
        });
        match r1 {
            Ok(_) => eprintln!("Step 1 (set_selection + delete_selection): OK"),
            Err(e) => panic!("Step 1 failed: {:?}", e),
        }

        // Step 2: insert_slice_at in separate transact
        let target = stable_target.thaw(&editor.state.doc).head;
        eprintln!(
            "target position after deletion: node={:?} offset={}",
            target.node_id, target.offset
        );
        let r2 = editor.transact(|tr| {
            commands::insert_slice_at(tr, target, slice.clone())
                .map_err(|e| crate::error::EditorError::from(e))?;
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
        let (initial, root, _fold_node, _para_text) = state! {
            doc {
                root: root {
                    _fold_node: fold {
                        fold_title { text("t") }
                        fold_content { paragraph { text("c") } }
                    }
                    paragraph { _para_text: text("after") }
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
        let target_pos = Position::new(NodeId::ROOT, 2);
        let result = editor.probe(|editor| {
            let DndState::InternalDnd { ref source, .. } = editor.dnd.clone() else {
                panic!("expected InternalDnd state");
            };
            let sel = source.thaw(&editor.state.doc);
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
        let (initial, root, _fold_node, para_text) = state! {
            doc {
                root: root {
                    _fold_node: fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("content") } }
                    }
                    paragraph { para_text: text("after") }
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
            .cursor_metrics(&editor.state.doc, &Position::new(para_text, 5))
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
        let (initial, root, fold_node, para_text) = state! {
            doc {
                root: root {
                    fold_node: fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("content") } }
                    }
                    paragraph { para_text: text("after") }
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
            .cursor_metrics(&editor.state.doc, &Position::new(para_text, 5))
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

        let doc = &editor.state().doc;
        let root_children: Vec<_> = doc.node(NodeId::ROOT).unwrap().children().collect();
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
        let (initial, root, _fold_node, _para_text) = state! {
            doc {
                root: root {
                    _fold_node: fold {
                        fold_title { text("title") }
                        fold_content { paragraph { text("content") } }
                    }
                    paragraph { _para_text: text("after") }
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
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("") } } }
            selection: (t1, 0)
        };
        let (template, ..) = state! {
            doc { root { paragraph { a: text("Title") } paragraph { b: text("Body") } } }
            selection: (a, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor
            .insert_template_fragment(template.doc.to_plain())
            .expect("template insert ok");

        let root = editor.state().doc.root().unwrap();
        let texts: Vec<String> = root
            .children()
            .map(|p| {
                let mut s = String::new();
                for c in p.children() {
                    if let Node::Text(t) = c.node() {
                        s.push_str(&t.text.to_string());
                    }
                }
                s
            })
            .collect();
        assert_eq!(texts, vec!["Title".to_string(), "Body".to_string()]);
    }

    #[test]
    fn insert_template_fragment_hides_placeholder() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("") } } }
            selection: (t1, 0)
        };
        let (template, ..) = state! {
            doc { root { paragraph { a: text("X") } } }
            selection: (a, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor
            .insert_template_fragment(template.doc.to_plain())
            .expect("template insert ok");
        assert!(
            editor
                .view()
                .placeholder_metrics(&editor.state().doc)
                .is_none()
        );
    }

    #[test]
    fn insert_template_fragment_preserves_named_styles() {
        use editor_transaction::Transaction;
        let (tpl_initial, p1, _t) = state! {
            doc { root { p1: paragraph { t1: text("Styled") } } }
            selection: (t1, 0)
        };
        let mut tr = Transaction::new(&tpl_initial);
        tr.set_style(
            "h1".into(),
            Some(editor_model::PlainStyleEntry {
                name: "Heading".into(),
                modifiers: vec![editor_model::Modifier::FontSize { value: 1800 }]
                    .into_iter()
                    .collect(),
            }),
        )
        .unwrap();
        tr.set_node_style(p1, Some("h1".into())).unwrap();
        let (tpl_state, ..) = tr.commit();

        let (initial, ..) = state! {
            doc { root { paragraph { e: text("") } } }
            selection: (e, 0)
        };
        let mut editor = Editor::new_test(initial);
        editor
            .insert_template_fragment(tpl_state.doc.to_plain())
            .expect("ok");

        let plain = editor.state().doc.to_plain();
        assert!(
            plain.styles.contains_key("h1"),
            "named style entry must transfer"
        );
        let para = editor.state().doc.root().unwrap().first_child().unwrap();
        let entry = plain.nodes.get(&para.id()).unwrap();
        assert_eq!(
            entry.style.as_deref(),
            Some("h1"),
            "node style ref must be reapplied"
        );
    }
}
