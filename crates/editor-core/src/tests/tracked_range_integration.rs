use editor_macros::state;
use editor_state::Selection;

use crate::editor::Editor;
use crate::message::*;

fn add_message(id: &str, group: &str, selection: Selection) -> Message {
    Message::TrackedRange {
        op: TrackedRangeOp::Add {
            id: id.into(),
            group: group.into(),
            selection,
            metadata: String::new(),
        },
    }
}

fn thawed_offsets(editor: &Editor, id: &str) -> (usize, usize) {
    let range = editor.tracked_ranges().get(id).expect("range present");
    let sel = range.selection.thaw(&editor.state().doc);
    (sel.anchor.offset, sel.head.offset)
}

fn is_collapsed_on_thaw(editor: &Editor, id: &str) -> bool {
    let range = editor.tracked_ranges().get(id).expect("range present");
    range.selection.thaw(&editor.state().doc).is_collapsed()
}

#[test]
fn range_position_shifts_when_text_inserted_before_it() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    assert_eq!(thawed_offsets(&editor, "r"), (1, 4));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(editor_state::Position::new(t1, 0)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });

    let (a, h) = thawed_offsets(&editor, "r");
    assert_eq!((a, h), (2, 5), "range must shift by inserted length");
}

#[test]
fn range_marked_invalid_when_all_covered_text_deleted() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));
    assert!(!is_collapsed_on_thaw(&editor, "r"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(t1, 1),
                editor_state::Position::new(t1, 4),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    assert!(
        is_collapsed_on_thaw(&editor, "r"),
        "deleting the covered text must collapse the range"
    );
}

#[test]
fn range_positions_revert_after_undo() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(editor_state::Position::new(t1, 0)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });
    assert_eq!(thawed_offsets(&editor, "r"), (2, 5));

    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });

    assert_eq!(
        thawed_offsets(&editor, "r"),
        (1, 4),
        "undo must restore the original positions"
    );
}

#[test]
fn registry_membership_survives_undo() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(editor_state::Position::new(t1, 0)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });
    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });

    assert!(
        editor.tracked_ranges().contains("r"),
        "registry membership is independent of doc history"
    );
}

#[test]
fn groups_are_independent() {
    let (initial, ..) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("a", "spellcheck", sel));
    editor.apply(add_message("b", "ai", sel));

    assert_eq!(editor.tracked_ranges().group_size("spellcheck"), 1);
    assert_eq!(editor.tracked_ranges().group_size("ai"), 1);

    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::ClearGroup {
            group: "spellcheck".into(),
        },
    });
    assert_eq!(editor.tracked_ranges().group_size("spellcheck"), 0);
    assert_eq!(editor.tracked_ranges().group_size("ai"), 1);
}

#[test]
fn freeze_then_add_frozen_roundtrip_yields_same_registry_entry() {
    let (state, _t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = state.selection.unwrap();

    let mut a = Editor::new_test(state.clone());
    a.apply(add_message("r", "g", sel));
    let from_add = a.tracked_ranges().get("r").unwrap().clone();

    let stable = editor_state::StableSelection::freeze(&sel, &state.doc);
    let json = serde_json::to_string(&stable).unwrap();
    let restored: editor_state::StableSelection = serde_json::from_str(&json).unwrap();

    let mut b = Editor::new_test(state);
    b.apply(Message::TrackedRange {
        op: TrackedRangeOp::AddFrozen {
            id: "r".into(),
            group: "g".into(),
            selection: restored,
            metadata: String::new(),
        },
    });
    let from_addfrozen = b.tracked_ranges().get("r").unwrap().clone();

    assert_eq!(from_add, from_addfrozen);
}

#[test]
fn add_frozen_with_unresolvable_dots_marks_invalid_on_thaw() {
    let (state_a, _t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = state_a.selection.unwrap();
    let stable = editor_state::StableSelection::freeze(&sel, &state_a.doc);

    let (state_b, _) = state! {
        doc { root { paragraph { t2: text("world") } } }
        selection: (t2, 0)
    };

    let mut b = Editor::new_test(state_b);
    b.apply(Message::TrackedRange {
        op: TrackedRangeOp::AddFrozen {
            id: "r".into(),
            group: "g".into(),
            selection: stable,
            metadata: String::new(),
        },
    });
    assert!(b.tracked_ranges().contains("r"));
    assert!(is_collapsed_on_thaw(&b, "r"));
}

#[test]
fn range_recovers_from_invalid_after_undo() {
    let (state, t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = state.selection.unwrap();
    let mut editor = Editor::new_test(state);
    editor.apply(add_message("r", "g", sel));
    assert!(!is_collapsed_on_thaw(&editor, "r"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(t1, 1),
                editor_state::Position::new(t1, 4),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });
    assert!(
        is_collapsed_on_thaw(&editor, "r"),
        "deleting covered text must collapse the range"
    );

    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });
    assert!(
        !is_collapsed_on_thaw(&editor, "r"),
        "undo must restore range to valid"
    );
}
