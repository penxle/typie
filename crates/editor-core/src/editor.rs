use editor_common::time::Duration;
use editor_model::NodeId;
use editor_renderer::{RenderSink, Renderer, ThemeVariant};
use editor_resource::Resource;
use editor_state::State;
use editor_transaction::{Effect, HistoryMeta, Transaction};
use editor_view::View;
use editor_view::Viewport;
use hashbrown::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::FontData;
use crate::error::EditorError;
use crate::event::EditorEvent;
use crate::handle;
use crate::history::History;
use crate::message::*;
use crate::state_field::StateField;

pub struct Editor {
    pub(crate) state: State,
    pub(crate) view: View,
    pub(crate) history: History,
    pub(crate) renderer: Renderer,
    pub(crate) resource: Arc<Mutex<Resource>>,
    message_queue: Vec<Message>,
    pending_events: Vec<EditorEvent>,
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
            message_queue: Vec::new(),
            pending_events: Vec::new(),
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

    pub fn enqueue(&mut self, msg: Message) {
        self.message_queue.push(msg);
    }

    pub fn tick(&mut self) -> Result<Vec<EditorEvent>, EditorError> {
        let messages = std::mem::take(&mut self.message_queue);
        for msg in messages {
            self.process_message(msg)?;
        }

        Ok(std::mem::take(&mut self.pending_events))
    }

    pub fn render_page(&mut self, page_idx: u32, sink: &mut dyn RenderSink, scale_factor: f32) {
        self.view.visit_page(
            page_idx as usize,
            &mut self
                .renderer
                .page_visitor(sink, &self.state.doc, scale_factor),
        );
    }

    fn process_message(&mut self, msg: Message) -> Result<(), EditorError> {
        match msg {
            Message::Key { event } => handle::handle_key_event(self, event)?,
            Message::Pointer { event } => handle::handle_pointer_event(self, event)?,
            Message::Intent { intent } => match intent {
                Intent::History { intent } => handle::handle_history_intent(self, intent)?,
                Intent::Navigation { intent } => handle::handle_navigation_intent(self, intent)?,
                Intent::Insertion { intent } => handle::handle_insertion_intent(self, intent)?,
                Intent::Deletion { intent } => handle::handle_deletion_intent(self, intent)?,
                Intent::Selection { intent } => handle::handle_selection_intent(self, intent)?,
                Intent::Formatting { intent } => handle::handle_formatting_intent(self, intent)?,
                Intent::Node { intent } => handle::handle_node_intent(self, intent)?,
                Intent::Clipboard { intent } => handle::handle_clipboard_intent(self, intent)?,
                Intent::Composition { intent } => handle::handle_composition_intent(self, intent)?,
            },
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

        self.state = state;
        self.process_effects(effects);

        if !steps.is_empty() {
            match meta.history {
                HistoryMeta::Record => self.history.push(&steps),
                HistoryMeta::Tagged(tag) => self.history.push_tagged(&steps, tag),
                HistoryMeta::Skip => {}
            }
        }

        let mut fields = Vec::new();

        if self.view.reconcile(&self.state.doc, &steps) {
            fields.push(StateField::PageSizes);
            self.push_event(EditorEvent::RenderInvalidated);
        }

        if steps.iter().any(|s| s.is_doc_step()) {
            fields.push(StateField::Doc);
        }

        if steps.iter().any(|s| s.is_selection_step()) {
            fields.push(StateField::Cursor);
            fields.push(StateField::Selection);
        }

        if !fields.is_empty() {
            self.push_event(EditorEvent::StateChanged { fields });
        }

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
                .resolve(mapping.family_id)
                .to_string();

            let is_primary = mapping.family_id == family_id;

            let Some(manifest) = resource
                .font_registry
                .manifest(mapping.family_id, mapping.weight)
            else {
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
        let resource = Arc::new(Mutex::new(Resource::new()));
        Self {
            state,
            view: View::new_test(),
            history: History::new(Duration::from_millis(300)),
            renderer: Renderer::new(ThemeVariant::LightWhite, Arc::clone(&resource)),
            resource,
            message_queue: Vec::new(),
            pending_events: Vec::new(),
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
            message_queue: Vec::new(),
            pending_events: Vec::new(),
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

        editor.apply(Message::Intent {
            intent: Intent::Selection {
                intent: SelectionIntent::Set { selection: target },
            },
        });

        assert_eq!(editor.state().selection, target);
    }

    #[test]
    fn undo_on_empty_history_is_noop() {
        let (mut editor, _) = test_editor();
        let before = editor.state().selection;
        editor.apply(Message::Intent {
            intent: Intent::History {
                intent: HistoryIntent::Undo,
            },
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
        editor.enqueue(Message::Intent {
            intent: Intent::Selection {
                intent: SelectionIntent::Set { selection },
            },
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
        editor.enqueue(Message::Intent {
            intent: Intent::Selection {
                intent: SelectionIntent::Set { selection: target },
            },
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
        let events = editor.apply(Message::Intent {
            intent: Intent::Insertion {
                intent: InsertionIntent::Text {
                    text: "a".to_string(),
                },
            },
        });

        let has_doc_changed = events
            .iter()
            .any(|e| matches!(e, EditorEvent::StateChanged { fields } if fields.contains(&StateField::Doc)));
        assert!(has_doc_changed);
    }
}
