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

    pub fn tick(&mut self) -> Vec<EditorEvent> {
        let messages = std::mem::take(&mut self.message_queue);
        for msg in messages {
            self.process_message(msg);
        }

        std::mem::take(&mut self.pending_events)
    }

    pub fn render_page(&mut self, page_idx: u32, sink: &mut dyn RenderSink) {
        if let Some(page) = self.view.pages().get(page_idx as usize) {
            self.renderer.render_page(sink, page, &self.state.doc);
        }
    }

    fn process_message(&mut self, msg: Message) {
        match msg {
            Message::Key(event) => handle::handle_key_event(self, event),
            Message::Pointer(event) => handle::handle_pointer_event(self, event),
            Message::Intent(intent) => match intent {
                Intent::History(h) => handle::handle_history_intent(self, h),
                Intent::Navigation(n) => handle::handle_navigation_intent(self, n),
                Intent::Insertion(i) => handle::handle_insertion_intent(self, i),
                Intent::Deletion(d) => handle::handle_deletion_intent(self, d),
                Intent::Selection(s) => handle::handle_selection_intent(self, s),
                Intent::Formatting(f) => handle::handle_formatting_intent(self, f),
                Intent::Node(n) => handle::handle_node_intent(self, n),
                Intent::Clipboard(c) => handle::handle_clipboard_intent(self, c),
                Intent::Composition(c) => handle::handle_composition_intent(self, c),
            },
            Message::System(event) => handle::handle_system_event(self, event),
        }
    }

    pub(crate) fn transact(&mut self, f: impl FnOnce(&mut Transaction) -> Result<(), EditorError>) {
        let mut tr = Transaction::new(&self.state);
        if f(&mut tr).is_err() {
            return;
        }

        let (state, steps, effects, meta) = tr.commit();

        self.state = state;

        if !steps.is_empty() {
            match meta.history {
                HistoryMeta::Record => self.history.push(&steps),
                HistoryMeta::Tagged(tag) => self.history.push_tagged(&steps, tag),
                HistoryMeta::Skip => {}
            }
        }

        if self.view.reconcile(&self.state.doc, &steps) {
            self.push_event(EditorEvent::RenderInvalidated);
        }

        if steps.iter().any(|s| s.is_doc_step()) {
            self.push_event(EditorEvent::DocumentChanged);
        }

        self.process_effects(effects);

        let mut changed_fields = Vec::new();

        if steps.iter().any(|s| s.is_selection_step()) {
            changed_fields.push(StateField::Selection);
        }

        if !changed_fields.is_empty() {
            self.push_event(EditorEvent::StateChanged {
                fields: changed_fields,
            });
        }
    }

    pub(crate) fn push_event(&mut self, event: EditorEvent) {
        self.pending_events.push(event);
    }

    fn process_effects(&mut self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::LoadFont { family, weight, .. } => {
                    self.push_event(EditorEvent::FontMissing { family, weight });
                }
            }
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
        self.tick()
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

        editor.apply(Message::Intent(Intent::Selection(SelectionIntent::Set(
            target,
        ))));

        assert_eq!(editor.state().selection, target);
    }

    #[test]
    fn undo_on_empty_history_is_noop() {
        let (mut editor, _) = test_editor();
        let before = editor.state().selection;
        editor.apply(Message::Intent(Intent::History(HistoryIntent::Undo)));
        assert_eq!(editor.state().selection, before);
    }

    #[test]
    fn system_resize_updates_viewport() {
        let (mut editor, _) = test_editor();
        editor.apply(Message::System(SystemEvent::Resize {
            width: 1024.0,
            height: 768.0,
            scale_factor: 1.0,
        }));
        assert_eq!(editor.view().viewport().width, 1024.0);
    }

    // can_undo/can_redo 통합 테스트도 content 변경 command 이식 후 작성

    #[test]
    fn tick_processes_all_enqueued_messages() {
        let (mut editor, t) = test_editor();

        let selection = Selection::collapsed(Position::new(t, 3));

        editor.enqueue(Message::System(SystemEvent::Resize {
            width: 1024.0,
            height: 768.0,
            scale_factor: 2.0,
        }));
        editor.enqueue(Message::Intent(Intent::Selection(SelectionIntent::Set(
            selection,
        ))));
        editor.tick();

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
        editor.enqueue(Message::Intent(Intent::Selection(SelectionIntent::Set(
            target,
        ))));

        let events = editor.tick();

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
        let events = editor.tick();
        assert!(events.is_empty());
    }

    #[test]
    fn process_effects_converts_load_font_to_font_missing() {
        let (mut editor, _) = test_editor();
        editor.process_effects(vec![Effect::LoadFont {
            family: "Inter".to_string(),
            weight: 400,
            codepoints: vec![65, 66],
        }]);
        let events = std::mem::take(&mut editor.pending_events);

        let has_font_missing = events.iter().any(|e| {
            matches!(
                e,
                EditorEvent::FontMissing { family, weight } if family == "Inter" && *weight == 400
            )
        });
        assert!(has_font_missing);
    }

    #[test]
    fn tick_returns_doc_changed_on_text_insert() {
        let (mut editor, _) = test_editor();
        let events = editor.apply(Message::Intent(Intent::Insertion(InsertionIntent::Text(
            "a".to_string(),
        ))));

        let has_doc_changed = events
            .iter()
            .any(|e| matches!(e, EditorEvent::DocumentChanged));
        assert!(has_doc_changed);
    }
}
