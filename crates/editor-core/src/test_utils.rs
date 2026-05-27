use editor_common::Size;
use editor_model::NodeId;
use editor_resource::ThemeVariant;
use editor_state::{Composition, PendingModifiers, Selection};
use editor_view::{GroupDecoration, Viewport};
use hashbrown::HashMap;

use crate::editor::Editor;
use crate::tracked_range::TrackedRangeRegistry;

#[derive(Clone, Debug, PartialEq)]
pub struct EditorSnapshot {
    doc: editor_model::Doc,
    selection: Option<Selection>,
    pending_modifiers: PendingModifiers,
    composition: Option<Composition>,
    undos_len: usize,
    redos_len: usize,
    viewport: Viewport,
    external_heights: HashMap<NodeId, f32>,
    fold_states: HashMap<NodeId, bool>,
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
            doc: editor.state().doc.clone(),
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

pub fn assert_probe_predicts_apply(state: editor_state::State, msg: crate::message::Message) {
    assert_probe_predicts_apply_with_setup(|| Editor::new_test(state.clone()), msg);
}

pub fn assert_probe_predicts_apply_with_setup<F>(mut build_editor: F, msg: crate::message::Message)
where
    F: FnMut() -> Editor,
{
    let mut probed = build_editor();
    let before = EditorSnapshot::capture(&probed);
    let predicted = probed.can(msg.clone()).expect("probe must succeed");
    let after = EditorSnapshot::capture(&probed);
    assert_eq!(before, after, "probe must not mutate observable state");

    let mut applied = build_editor();
    let before_applied = EditorSnapshot::capture(&applied);
    applied.apply(msg);
    let after_applied = EditorSnapshot::capture(&applied);
    let actually_changed = before_applied != after_applied;

    assert_eq!(
        predicted, actually_changed,
        "probe must predict whether apply changes observable state"
    );
}

pub fn assert_probe_is_safe(state: editor_state::State, msg: crate::message::Message) {
    let mut editor = Editor::new_test(state);
    let before = EditorSnapshot::capture(&editor);
    let _ = editor.can(msg);
    let after = EditorSnapshot::capture(&editor);
    assert_eq!(before, after, "probe must not mutate observable state");
}
