use editor_macros::state;
use editor_model::StableEntryResolution;
use editor_state::Selection;

use crate::editor::Editor;
use crate::event::{EditorEvent, TrackedRangeReplaceOutcome};
use crate::message::*;

fn add_range(editor: &mut Editor, id: &str, selection: Selection) {
    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::Add {
            id: id.into(),
            group: "spell".into(),
            selection,
            metadata: String::new(),
        },
    });
}

fn replace_msg(id: &str, expected_text: Option<&str>, replacement: &str) -> Message {
    Message::TrackedRange {
        op: TrackedRangeOp::ReplaceText {
            id: id.into(),
            expected_text: expected_text.map(str::to_owned),
            replacement: replacement.into(),
        },
    }
}

fn outcome_from_events(events: &[EditorEvent], id: &str) -> Option<TrackedRangeReplaceOutcome> {
    events.iter().find_map(|e| match e {
        EditorEvent::TrackedRangeReplaceResult { id: ev_id, outcome } if ev_id == id => {
            Some(outcome.clone())
        }
        _ => None,
    })
}

fn paragraph_text(editor: &Editor) -> String {
    editor.state().doc.extract_text()
}

fn visible_entry_dots(
    editor: &Editor,
    node_id: editor_model::NodeId,
) -> Vec<editor_crdt::EntryDot> {
    editor
        .state()
        .doc
        .text_view(node_id)
        .expect("text node")
        .visible_entries()
        .map(|(entry, _)| entry)
        .collect()
}

#[test]
fn happy_path_replaces_text_and_emits_replaced() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 6) -> (t1, 11)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    add_range(&mut editor, "r", sel);

    let before_undos = editor.history_undos_len();
    let events = editor.apply(replace_msg("r", Some("world"), "earth"));

    assert_eq!(
        outcome_from_events(&events, "r"),
        Some(TrackedRangeReplaceOutcome::Replaced)
    );
    assert!(paragraph_text(&editor).contains("hello earth"));
    assert_eq!(editor.history_undos_len(), before_undos + 1);
    let _ = t1;
}

#[test]
fn replace_undo_redo_remaps_original_and_replacement_entries() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("abcd") } } }
        selection: (t1, 1) -> (t1, 3)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    add_range(&mut editor, "r", sel);
    let original_bc = visible_entry_dots(&editor, t1)[1..3].to_vec();

    editor.apply(replace_msg("r", Some("bc"), "xy"));
    assert_eq!(editor.state().doc.text_view(t1).unwrap().text(), "axyd");
    let first_xy = visible_entry_dots(&editor, t1)[1..3].to_vec();

    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });
    assert_eq!(editor.state().doc.text_view(t1).unwrap().text(), "abcd");
    let fresh_bc = visible_entry_dots(&editor, t1)[1..3].to_vec();
    assert_eq!(
        original_bc
            .iter()
            .map(|entry| editor
                .state()
                .doc
                .text_identity()
                .resolve_stable_entry(*entry))
            .collect::<Vec<_>>(),
        fresh_bc
            .iter()
            .map(|entry| StableEntryResolution::Live(*entry))
            .collect::<Vec<_>>()
    );

    editor.apply(Message::History {
        op: HistoryOp::Redo,
    });
    assert_eq!(editor.state().doc.text_view(t1).unwrap().text(), "axyd");
    let fresh_xy = visible_entry_dots(&editor, t1)[1..3].to_vec();
    assert_eq!(
        first_xy
            .iter()
            .map(|entry| editor
                .state()
                .doc
                .text_identity()
                .resolve_stable_entry(*entry))
            .collect::<Vec<_>>(),
        fresh_xy
            .iter()
            .map(|entry| StableEntryResolution::Live(*entry))
            .collect::<Vec<_>>()
    );
}

#[test]
fn unknown_id_emits_unknown_id_and_no_op() {
    let (initial, _t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 0)
    };
    let mut editor = Editor::new_test(initial);
    let text_before = paragraph_text(&editor);
    let before_undos = editor.history_undos_len();
    let events = editor.apply(replace_msg("missing", None, "x"));

    assert_eq!(
        outcome_from_events(&events, "missing"),
        Some(TrackedRangeReplaceOutcome::UnknownId)
    );
    assert_eq!(paragraph_text(&editor), text_before);
    assert_eq!(editor.history_undos_len(), before_undos);
}

#[test]
fn explicitly_invalid_range_emits_invalid_and_no_op() {
    let (initial, _t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 6) -> (t1, 11)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    add_range(&mut editor, "r", sel);
    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::Invalidate { id: "r".into() },
    });

    let text_before = paragraph_text(&editor);
    let events = editor.apply(replace_msg("r", None, "earth"));

    assert_eq!(
        outcome_from_events(&events, "r"),
        Some(TrackedRangeReplaceOutcome::Invalid)
    );
    assert_eq!(paragraph_text(&editor), text_before);
}

#[test]
fn collapsed_on_thaw_emits_invalid() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 6) -> (t1, 11)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    add_range(&mut editor, "r", sel);

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(t1, 6),
                editor_state::Position::new(t1, 11),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    let text_before = paragraph_text(&editor);
    let events = editor.apply(replace_msg("r", None, "earth"));

    assert_eq!(
        outcome_from_events(&events, "r"),
        Some(TrackedRangeReplaceOutcome::Invalid)
    );
    assert_eq!(paragraph_text(&editor), text_before);
}

#[test]
fn text_mismatch_emits_text_mismatch_and_no_op() {
    let (initial, _t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 6) -> (t1, 11)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    add_range(&mut editor, "r", sel);

    let text_before = paragraph_text(&editor);
    let events = editor.apply(replace_msg("r", Some("WORLD"), "earth"));

    assert_eq!(
        outcome_from_events(&events, "r"),
        Some(TrackedRangeReplaceOutcome::TextMismatch)
    );
    assert_eq!(paragraph_text(&editor), text_before);
}

#[test]
fn expected_none_skips_text_comparison() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 6) -> (t1, 11)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    add_range(&mut editor, "r", sel);

    let events = editor.apply(replace_msg("r", None, "earth"));
    assert_eq!(
        outcome_from_events(&events, "r"),
        Some(TrackedRangeReplaceOutcome::Replaced)
    );
    assert!(paragraph_text(&editor).contains("hello earth"));
    let _ = t1;
}

#[test]
fn undo_restores_original_text() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 6) -> (t1, 11)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    add_range(&mut editor, "r", sel);

    let text_before = paragraph_text(&editor);
    editor.apply(replace_msg("r", Some("world"), "earth"));
    assert!(paragraph_text(&editor).contains("hello earth"));

    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });
    assert_eq!(paragraph_text(&editor), text_before);
    let _ = t1;
}

#[test]
fn replacement_with_newline_is_no_op_and_emits_invalid_replacement() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 6) -> (t1, 11)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    add_range(&mut editor, "r", sel);

    let text_before = paragraph_text(&editor);
    let before_undos = editor.history_undos_len();
    let events = editor.apply(replace_msg("r", None, "a\nb"));

    assert_eq!(
        outcome_from_events(&events, "r"),
        Some(TrackedRangeReplaceOutcome::InvalidReplacement)
    );
    assert_eq!(paragraph_text(&editor), text_before);
    assert_eq!(editor.history_undos_len(), before_undos);
    let _ = t1;
}

#[test]
fn replace_with_empty_replacement_deletes_range() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 5) -> (t1, 11)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    add_range(&mut editor, "r", sel);

    let events = editor.apply(replace_msg("r", None, ""));
    assert_eq!(
        outcome_from_events(&events, "r"),
        Some(TrackedRangeReplaceOutcome::Replaced)
    );
    assert_eq!(paragraph_text(&editor), "hello");
    let _ = t1;
}
