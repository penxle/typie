use editor_common::Size;
use editor_crdt::Dot;
use editor_resource::ThemeVariant;
use editor_state::{Composition, PendingModifiers, Selection};
use editor_view::{GroupDecoration, Viewport};
use hashbrown::HashMap;

use crate::editor::Editor;
use crate::tracked_range::TrackedRangeRegistry;

#[derive(Clone, Debug, PartialEq)]
pub struct EditorSnapshot {
    doc: editor_model::ProjectedDoc,
    selection: Option<Selection>,
    pending_modifiers: PendingModifiers,
    composition: Option<Composition>,
    undos_len: usize,
    redos_len: usize,
    viewport: Viewport,
    external_heights: HashMap<Dot, f32>,
    fold_states: HashMap<Dot, bool>,
    preferred_x: Option<f32>,
    focused: bool,
    theme_variant: ThemeVariant,
    page_sizes: Vec<Size>,
    tracked_ranges: TrackedRangeRegistry,
    tracked_decoration_groups: HashMap<String, GroupDecoration>,
}

impl EditorSnapshot {
    pub fn capture(editor: &Editor) -> Self {
        let view_state = editor.view().view_state();
        let page_sizes: Vec<Size> = editor.view().pages().iter().map(|p| p.size).collect();
        Self {
            doc: editor.state().projected.projected().clone(),
            selection: editor.state().selection,
            pending_modifiers: editor.state().pending_modifiers.clone(),
            composition: editor.state().composition,
            undos_len: editor.history_undos_len(),
            redos_len: editor.history_redos_len(),
            viewport: *editor.view().viewport(),
            external_heights: view_state.external_heights.clone(),
            fold_states: view_state.fold_states.clone(),
            preferred_x: view_state.preferred_x,
            focused: editor.is_focused(),
            theme_variant: editor.theme_variant(),
            page_sizes,
            tracked_ranges: editor.tracked_ranges().clone(),
            tracked_decoration_groups: view_state.tracked_decoration_groups.clone(),
        }
    }
}

pub fn apply_and_report_change(editor: &mut Editor, msg: crate::message::Message) -> bool {
    let _ = editor.tick();
    let before = EditorSnapshot::capture(editor);
    editor.apply(msg);
    EditorSnapshot::capture(editor) != before
}

pub fn assert_apply_changes_state(state: editor_state::State, msg: crate::message::Message) {
    let mut editor = Editor::new_test(state);
    assert!(
        apply_and_report_change(&mut editor, msg),
        "apply must observably change state"
    );
}

pub fn assert_apply_preserves_state(state: editor_state::State, msg: crate::message::Message) {
    let mut editor = Editor::new_test(state);
    assert!(
        !apply_and_report_change(&mut editor, msg),
        "apply must not observably change state"
    );
}
