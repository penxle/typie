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

#[test]
fn state_changed_includes_tracked_ranges_when_doc_edit_with_registered_range() {
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
        fields.contains(&StateField::TrackedRanges),
        "doc edit with registered range must emit TrackedRanges field; got {fields:?}"
    );

    let _ = p1;
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
