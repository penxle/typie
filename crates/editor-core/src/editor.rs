use editor_common::time::{Duration, SystemTime, UNIX_EPOCH};
use editor_model::{Node, NodeId};
use editor_renderer::{Mark, MarkData, RenderSink, Renderer, ThemeVariant};
use editor_resource::Resource;
use editor_state::{DocFlatExt, Position, ResolvedPosition, ResolvedPositionFlatExt, State};
use editor_transaction::{Effect, HistoryMeta, Step, Transaction};
use editor_view::{PendingStyle, View, Viewport};
use hashbrown::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::error::EditorError;
use crate::event::{EditorEvent, FontData};
use crate::handle;
use crate::history::History;
use crate::ime::{Ime, ImeRange};
use crate::message::*;
use crate::state_field::StateField;

fn normalize_pending_style(state: &State) -> Option<PendingStyle> {
    if state.pending_modifiers.is_empty() {
        return None;
    }
    let textblock = state
        .doc
        .node(state.selection.head.node_id)?
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

pub struct Editor {
    pub(crate) state: State,
    pub(crate) view: View,
    pub(crate) history: History,
    pub(crate) renderer: Renderer,
    pub(crate) resource: Arc<Mutex<Resource>>,
    pub(crate) is_dragging: bool,
    message_queue: Vec<Message>,
    pending_events: Vec<EditorEvent>,
    pending_steps: Vec<Step>,
    pending_effects: HashSet<Effect>,
    pub(crate) pending_fonts: HashMap<(String, u16), HashMap<NodeId, HashSet<u32>>>,
}

impl Editor {
    pub fn new(state: State, viewport: Viewport, resource: Arc<Mutex<Resource>>) -> Self {
        Self {
            state,
            view: View::new(viewport, Arc::clone(&resource)),
            history: History::new(Duration::from_millis(300)),
            renderer: Renderer::new(ThemeVariant::LightWhite, Arc::clone(&resource)),
            resource,
            is_dragging: false,
            message_queue: Vec::new(),
            pending_events: Vec::new(),
            pending_steps: Vec::new(),
            pending_effects: HashSet::new(),
            pending_fonts: HashMap::new(),
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn view(&self) -> &View {
        &self.view
    }

    pub fn modifier_state(&self) -> editor_model::ModifierState {
        editor_state::resolve_modifier_state(&self.state)
    }

    pub fn block_state(&self) -> crate::block_state::BlockState {
        crate::block_state::resolve_block_state(&self.state)
    }

    pub fn set_theme_variant(&mut self, variant: ThemeVariant) {
        self.renderer.set_theme_variant(variant);
    }

    pub fn ime(&self, before_limit: usize, after_limit: usize) -> Result<Ime, EditorError> {
        let state = self.state();
        let doc = &state.doc;
        let doc_size = doc.flat_size();

        let sel = state.selection;
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

        let messages = std::mem::take(&mut self.message_queue);
        for msg in messages {
            self.process_message(msg)?;
        }

        let effects = std::mem::take(&mut self.pending_effects);
        if !effects.is_empty() {
            self.process_effects(effects);
        }

        let steps = std::mem::take(&mut self.pending_steps);
        let mut fields = HashSet::new();

        let pending_style = normalize_pending_style(&self.state);
        if !steps.is_empty()
            && self
                .view
                .reconcile(&old_doc, &self.state.doc, &steps, pending_style)
        {
            fields.insert(StateField::Cursor);
            fields.insert(StateField::PageSizes);
            self.push_event(EditorEvent::RenderInvalidated);
        }

        if steps.iter().any(|s| s.is_doc_step()) {
            fields.insert(StateField::Doc);
            fields.insert(StateField::Ime);
            fields.insert(StateField::Modifiers);
            fields.insert(StateField::Block);
        }

        if steps.iter().any(|s| {
            matches!(
                s,
                Step::SetNode { node_id, .. } if *node_id == NodeId::ROOT
            )
        }) {
            fields.insert(StateField::Doc);
            fields.insert(StateField::RootAttrs);
        }

        if steps.iter().any(|s| s.is_selection_step()) {
            fields.insert(StateField::Cursor);
            fields.insert(StateField::Ime);
            fields.insert(StateField::Selection);
            fields.insert(StateField::Modifiers);
            fields.insert(StateField::Block);
            self.push_event(EditorEvent::RenderInvalidated);
        }

        if steps.iter().any(|s| s.is_pending_modifiers_step()) {
            fields.insert(StateField::Modifiers);
        }

        if !fields.is_empty() {
            self.push_event(EditorEvent::StateChanged {
                fields: fields.iter().copied().collect(),
            });
        }

        Ok(std::mem::take(&mut self.pending_events))
    }

    pub fn render_page(&mut self, page_idx: u32, sink: &mut dyn RenderSink, scale_factor: f32) {
        let mut marks: Vec<Mark> = Vec::new();

        if let Some(resolved) = self.state.selection.resolve(&self.state.doc) {
            if !resolved.is_collapsed() {
                let rects = self
                    .view
                    .selection_rects(&resolved)
                    .iter()
                    .map(|r| r.without_meta())
                    .collect();
                marks.push(Mark {
                    data: MarkData::Selection,
                    rects,
                });
            }
        }

        if let Some(composition) = self.state.composition {
            if let (Some(from), Some(to)) = (
                ResolvedPosition::from_flat(&self.state.doc, composition.start),
                ResolvedPosition::from_flat(&self.state.doc, composition.end),
            ) {
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

    fn process_message(&mut self, msg: Message) -> Result<(), EditorError> {
        match msg {
            Message::Key { event } => handle::handle_key_event(self, event)?,
            Message::Pointer { event } => handle::handle_pointer_event(self, event)?,
            Message::Insertion { op } => handle::handle_insertion_op(self, op)?,
            Message::Deletion { op } => handle::handle_deletion_op(self, op)?,
            Message::Modifier { op } => handle::handle_modifier_op(self, op)?,
            Message::Selection { op } => handle::handle_selection_op(self, op)?,
            Message::Node { op } => handle::handle_node_op(self, op)?,
            Message::Clipboard { op } => handle::handle_clipboard_op(self, op)?,
            Message::Composition { op } => handle::handle_composition_op(self, op)?,
            Message::Navigation { op } => handle::handle_navigation_op(self, op)?,
            Message::History { op } => handle::handle_history_op(self, op)?,
            Message::System { event } => handle::handle_system_event(self, event)?,
        }
        Ok(())
    }

    pub(crate) fn transact(
        &mut self,
        f: impl FnOnce(&mut Transaction) -> Result<(), EditorError>,
    ) -> Result<(), EditorError> {
        let old_doc = self.state.doc.clone();
        let mut tr = Transaction::new(&self.state);
        f(&mut tr)?;

        let (state, steps, effects, meta) = tr.commit();

        let commitable: Vec<Step> = steps
            .iter()
            .filter(|s| s.is_commitable())
            .cloned()
            .collect();
        if !commitable.is_empty() {
            let mut affected: Vec<editor_model::NodeId> = commitable
                .iter()
                .flat_map(|s| s.affected_node_ids(&old_doc, &state.doc))
                .collect();
            affected.sort();
            affected.dedup();

            let (root_object_hash, objects) = state.doc.derive_objects_for_path(&affected);

            self.push_event(EditorEvent::TransactionCommitted {
                commit: crate::event::Commit {
                    root_object_hash,
                    objects,
                    steps: commitable,
                    meta: meta.clone(),
                    committed_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as i64,
                },
            });
        }

        if !steps.is_empty() {
            match meta.history {
                HistoryMeta::Record => self.history.push(&steps),
                HistoryMeta::Tagged { tag } => self.history.push_tagged(&steps, tag),
                HistoryMeta::Skip => {}
            }
        }

        self.state = state;
        self.pending_steps.extend(steps);
        self.pending_effects.extend(effects);

        Ok(())
    }

    pub(crate) fn push_event(&mut self, event: EditorEvent) {
        let event = match event {
            EditorEvent::StateChanged { mut fields } => {
                fields.sort_unstable();
                EditorEvent::StateChanged { fields }
            }
            other => other,
        };
        // TransactionCommitted is always pushed; multiple transacts in a single tick
        // must each emit a distinct event even if their (steps, meta) compare equal.
        if matches!(event, EditorEvent::TransactionCommitted { .. })
            || !self.pending_events.contains(&event)
        {
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
            renderer: Renderer::new(ThemeVariant::LightWhite, Arc::clone(&resource)),
            resource,
            is_dragging: false,
            message_queue: Vec::new(),
            pending_events: Vec::new(),
            pending_steps: Vec::new(),
            pending_effects: HashSet::new(),
            pending_fonts: HashMap::new(),
        }
    }

    pub fn apply(&mut self, msg: Message) -> Vec<EditorEvent> {
        self.enqueue(msg);
        self.tick().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::{doc, state};
    use editor_model::{Modifier, NodeId};
    use editor_state::{PendingModifier, PendingModifiers, Position, Selection};

    use super::*;

    fn build_state(doc: editor_model::Doc, head: Position, pending: PendingModifiers) -> State {
        let mut s = State::new(doc, Selection::collapsed(head));
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
        let mut pending = PendingModifiers::new();
        pending.push(PendingModifier::Set {
            modifier: Modifier::Bold,
        });
        let s = build_state(doc, Position::new(p1, 0), pending);
        assert!(normalize_pending_style(&s).is_none());
    }

    #[test]
    fn normalize_empty_paragraph_returns_some_with_container_id() {
        let (doc, p1) = doc! { root { p1: paragraph } };
        let mut pending = PendingModifiers::new();
        pending.push(PendingModifier::Set {
            modifier: Modifier::FontSize { value: 9600 },
        });
        let s = build_state(doc, Position::new(p1, 0), pending.clone());
        let ps = normalize_pending_style(&s).expect("Some");
        assert_eq!(ps.node_id, p1);
        assert_eq!(ps.modifiers, pending);
    }

    #[test]
    fn normalize_head_on_text_child_ascends_to_textblock() {
        let (doc, _p1, t1) = doc! { root { _p1: paragraph { t1: text("hi") } } };
        let mut pending = PendingModifiers::new();
        pending.push(PendingModifier::Set {
            modifier: Modifier::Bold,
        });
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

        assert_eq!(editor.state().selection, target);
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
        assert_eq!(editor.state().selection, selection);
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
        let s = editor.modifier_state();
        assert_eq!(s.bold, editor_common::Tri::Uniform { value: () });
    }

    #[test]
    fn editor_exposes_block_state() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hi") } } }
            selection: (t1, 1)
        };
        let editor = Editor::new_test(state);
        let bs = editor.block_state();
        assert_eq!(bs.ancestors.len(), 2);
        assert!(bs.nodes.is_empty());
    }

    #[test]
    fn transact_emits_transaction_committed_with_commitable_steps() {
        let (mut editor, _) = test_editor();
        let events = editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "hi".into() },
        });
        let committed: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                EditorEvent::TransactionCommitted { commit } => Some(commit.steps.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            committed.len(),
            1,
            "expected exactly one TransactionCommitted event"
        );
        assert!(!committed[0].is_empty(), "steps should not be empty");
        assert!(
            committed[0].iter().all(|s| s.is_commitable()),
            "all emitted steps should be commitable"
        );
    }

    #[test]
    fn transact_emits_commit_with_objects_and_root_hash() {
        let (mut editor, _) = test_editor();
        let events = editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "hi".into() },
        });
        let commit = events
            .iter()
            .find_map(|e| match e {
                EditorEvent::TransactionCommitted { commit } => Some(commit),
                _ => None,
            })
            .expect("expected one TransactionCommitted event");
        assert_eq!(commit.root_object_hash.len(), 32, "32-char xxh3 hex");
        assert!(
            !commit.objects.is_empty(),
            "InsertText must derive at least one object"
        );
        assert!(
            commit
                .objects
                .iter()
                .any(|o| o.hash == commit.root_object_hash),
            "root_object_hash must match one of the derived objects"
        );
    }

    #[test]
    fn transact_does_not_emit_transaction_committed_for_selection_only() {
        let (mut editor, t) = test_editor();
        let target = Selection::collapsed(Position::new(t, 3));
        let events = editor.apply(Message::Selection {
            op: SelectionOp::Set { selection: target },
        });
        let has_committed = events
            .iter()
            .any(|e| matches!(e, EditorEvent::TransactionCommitted { .. }));
        assert!(
            !has_committed,
            "selection-only transaction should not emit TransactionCommitted"
        );
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
    fn multiple_transacts_in_single_tick_emit_separate_events_when_steps_differ() {
        let (mut editor, _) = test_editor();
        editor.enqueue(Message::Insertion {
            op: InsertionOp::Text { text: "a".into() },
        });
        editor.enqueue(Message::Insertion {
            op: InsertionOp::Text { text: "b".into() },
        });
        let events = editor.tick().unwrap();
        let committed_count = events
            .iter()
            .filter(|e| matches!(e, EditorEvent::TransactionCommitted { .. }))
            .count();
        assert_eq!(
            committed_count, 2,
            "two distinct transactions should emit two events"
        );
    }

    #[test]
    fn tick_does_not_dedup_transaction_committed() {
        let (mut editor, _) = test_editor();
        let event = EditorEvent::TransactionCommitted {
            commit: crate::event::Commit {
                root_object_hash: String::new(),
                objects: vec![],
                steps: vec![],
                meta: editor_transaction::TransactionMeta::default(),
                committed_at: 0,
            },
        };
        editor.push_event(event.clone());
        editor.push_event(event);
        let events = editor.tick().unwrap();
        let count = events
            .iter()
            .filter(|e| matches!(e, EditorEvent::TransactionCommitted { .. }))
            .count();
        assert_eq!(count, 2, "TransactionCommitted must not be deduplicated");
    }
}
