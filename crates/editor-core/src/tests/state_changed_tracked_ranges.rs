use editor_macros::state;

use crate::editor::Editor;
use crate::event::EditorEvent;
use crate::message::*;
use crate::state_field::StateField;

fn drain_state_changed_fields(events: Vec<EditorEvent>) -> Vec<StateField> {
    let mut all = Vec::new();
    for ev in events {
        if let EditorEvent::StateChanged { fields } = ev {
            all.extend(fields);
        }
    }
    all
}

fn stale_ids(events: &[EditorEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|ev| match ev {
            EditorEvent::TrackedRangesStale { ids } => Some(ids.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

#[test]
fn doc_edit_leaving_range_text_intact_emits_no_tracked_ranges_field() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello world") } } }
        selection: (p1, 0) -> (p1, 5)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);

    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::Add {
            id: "r1".into(),
            group: "spellcheck".into(),
            selection: sel,
            metadata: String::new(),
            invalidate_on_text_change: true,
        },
    });

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: editor_state::Selection::collapsed(editor_state::Position::new(p1, 11)),
        },
    });
    let events = editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });
    let fields = drain_state_changed_fields(events);

    assert!(
        fields.contains(&StateField::Doc),
        "doc edit must emit Doc field; got {fields:?}"
    );
    assert!(
        !fields.contains(&StateField::TrackedRanges),
        "edits that leave every tracked range's text intact must not emit TrackedRanges; got {fields:?}"
    );
    assert!(editor.tracked_ranges().contains("r1"));
}

#[test]
fn doc_edit_changing_sensitive_range_text_removes_it_and_reports_stale() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello world") } } }
        selection: (p1, 0) -> (p1, 5)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);

    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::Add {
            id: "r1".into(),
            group: "spellcheck".into(),
            selection: sel,
            metadata: String::new(),
            invalidate_on_text_change: true,
        },
    });

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: editor_state::Selection::collapsed(editor_state::Position::new(p1, 2)),
        },
    });
    let events = editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });

    assert!(
        !editor.tracked_ranges().contains("r1"),
        "typing inside a text-sensitive range must remove it"
    );
    assert_eq!(stale_ids(&events), vec!["r1".to_string()]);
    let fields = drain_state_changed_fields(events);
    assert!(
        fields.contains(&StateField::TrackedRanges),
        "stale removal must emit TrackedRanges field; got {fields:?}"
    );
}

#[test]
fn doc_edit_inside_insensitive_range_keeps_it_and_emits_nothing() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello world") } } }
        selection: (p1, 0) -> (p1, 5)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);

    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::Add {
            id: "comment".into(),
            group: "comment".into(),
            selection: sel,
            metadata: String::new(),
            invalidate_on_text_change: false,
        },
    });

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: editor_state::Selection::collapsed(editor_state::Position::new(p1, 2)),
        },
    });
    let events = editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });

    assert!(
        editor.tracked_ranges().contains("comment"),
        "insensitive ranges survive text changes"
    );
    assert!(stale_ids(&events).is_empty());
    let fields = drain_state_changed_fields(events);
    assert!(
        !fields.contains(&StateField::TrackedRanges),
        "insensitive ranges never trigger TrackedRanges on doc edits; got {fields:?}"
    );
}

#[test]
fn state_changed_omits_tracked_ranges_when_no_range_registered() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello world") } } }
        selection: (p1, 5)
    };
    let mut editor = Editor::new_test(initial);

    let events = editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });
    let fields = drain_state_changed_fields(events);

    assert!(
        fields.contains(&StateField::Doc),
        "doc edit must emit Doc field; got {fields:?}"
    );
    assert!(
        !fields.contains(&StateField::TrackedRanges),
        "no TrackedRanges emit when registry is empty; got {fields:?}"
    );

    let _ = p1;
}
