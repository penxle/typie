use editor_common::time::Duration;
use editor_model::NodeId;
use editor_renderer::{Mark, MarkData, RenderSink, Renderer, ThemeVariant};
use editor_resource::Resource;
use editor_schema::{DocFlatExt, ResolvedPositionFlatExt};
use editor_state::{Position, ResolvedPosition, State};
use editor_transaction::{Effect, HistoryMeta, Step, Transaction};
use editor_view::View;
use editor_view::Viewport;
use hashbrown::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::error::EditorError;
use crate::event::{EditorEvent, FontData};
use crate::handle;
use crate::history::History;
use crate::ime::{Ime, ImeRange};
use crate::message::*;
use crate::state_field::StateField;

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
    pending_effects: Vec<Effect>,
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
            pending_effects: Vec::new(),
            pending_fonts: HashMap::new(),
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn view(&self) -> &View {
        &self.view
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
        let mut fields = Vec::new();

        if !steps.is_empty() && self.view.reconcile(&old_doc, &self.state.doc, &steps) {
            fields.push(StateField::PageSizes);
            self.push_event(EditorEvent::RenderInvalidated);
        }

        if steps.iter().any(|s| s.is_doc_step()) {
            fields.push(StateField::Doc);
            fields.push(StateField::Ime);
        }

        if steps.iter().any(|s| s.is_selection_step()) {
            fields.push(StateField::Cursor);
            fields.push(StateField::Selection);
            fields.push(StateField::Ime);
            self.push_event(EditorEvent::RenderInvalidated);
        }

        if !fields.is_empty() {
            self.push_event(EditorEvent::StateChanged { fields });
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
        let mut tr = Transaction::new(&self.state);
        f(&mut tr)?;

        let (state, steps, effects, meta) = tr.commit();

        if !steps.is_empty() {
            match meta.history {
                HistoryMeta::Record => self.history.push(&steps),
                HistoryMeta::Tagged(tag) => self.history.push_tagged(&steps, tag),
                HistoryMeta::Skip => {}
            }
        }

        self.state = state;
        self.pending_steps.extend(steps);
        self.pending_effects.extend(effects);

        Ok(())
    }

    pub(crate) fn push_event(&mut self, event: EditorEvent) {
        self.pending_events.push(event);
    }

    fn process_effects(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::LoadFont {
                    family,
                    weight,
                    codepoints,
                } => {
                    let has_manifest = {
                        let resource = self.resource.lock().unwrap();
                        resource
                            .font_registry
                            .intern_id(&family)
                            .map(|id| resource.font_registry.has_manifest(id, weight))
                            .unwrap_or(false)
                    };

                    if has_manifest {
                        self.resolve_fonts(&family, weight, &codepoints);
                    } else {
                        self.push_event(EditorEvent::FontManifestMissing { family, weight });
                    }
                }
            }
        }
    }

    pub(crate) fn resolve_fonts(&mut self, family: &str, weight: u16, codepoints: &[u32]) {
        let mut resource = self.resource.lock().unwrap();

        let family_id = resource.font_registry.intern(family);
        let mappings = resource
            .font_registry
            .resolve_codepoint_mappings(family_id, weight, codepoints);

        for mapping in &mappings {
            for &cp in &mapping.codepoints {
                resource.font_registry.add_codepoint_mapping(
                    family_id,
                    weight,
                    cp,
                    mapping.family_id,
                    mapping.weight,
                );
            }
        }

        let mut events = Vec::new();
        for mapping in &mappings {
            let resolved_family = resource
                .font_registry
                .resolve_opt(mapping.family_id)
                .map(|s| s.to_string());

            let is_primary = mapping.family_id == family_id;

            let Some(manifest) = resource
                .font_registry
                .manifest(mapping.family_id, mapping.weight)
            else {
                continue;
            };

            let Some(resolved_family) = resolved_family else {
                continue;
            };

            let required_chunks = manifest.chunk_indices(&mapping.codepoints);

            let required: Vec<_> = itertools::chain!(
                [FontData::Base],
                required_chunks
                    .iter()
                    .map(|&i| FontData::Chunk { index: i }),
            )
            .collect();

            let prefetch = if is_primary {
                let required_set: HashSet<u16> = required_chunks.iter().copied().collect();
                manifest
                    .all_chunk_indices()
                    .filter(|i| !required_set.contains(i))
                    .map(|i| FontData::Chunk { index: i })
                    .collect()
            } else {
                vec![]
            };

            events.push(EditorEvent::FontDataMissing {
                family: resolved_family,
                weight: mapping.weight,
                required,
                prefetch,
            });
        }

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
            pending_effects: Vec::new(),
            pending_fonts: HashMap::new(),
        }
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
            pending_effects: Vec::new(),
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
    use editor_macros::state;
    use editor_model::NodeId;
    use editor_state::{Position, Selection};

    use super::*;

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
    fn process_effects_converts_load_font_to_font_manifest_missing() {
        let (mut editor, _) = test_editor();
        editor.process_effects(vec![Effect::LoadFont {
            family: "Inter".to_string(),
            weight: 400,
            codepoints: vec![65, 66],
        }]);
        let events = std::mem::take(&mut editor.pending_events);

        let has_manifest_missing = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::FontManifestMissing { family, weight } if family == "Inter" && *weight == 400
            )
        });
        assert!(has_manifest_missing);
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
}
