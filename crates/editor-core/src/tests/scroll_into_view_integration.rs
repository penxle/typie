use editor_macros::state;
use editor_state::{Position, Selection};

use crate::editor::Editor;
use crate::event::EditorEvent;
use crate::message::*;

fn add_message(id: &str, selection: Selection) -> Message {
    Message::TrackedRange {
        op: TrackedRangeOp::Add {
            id: id.into(),
            group: "g".into(),
            selection,
            metadata: String::new(),
        },
    }
}

fn scroll_to_tracked(id: &str) -> Message {
    Message::View {
        op: ViewOp::ScrollIntoView {
            target: ScrollTarget::TrackedItem { id: id.into() },
        },
    }
}

fn scroll_to_selection() -> Message {
    Message::View {
        op: ViewOp::ScrollIntoView {
            target: ScrollTarget::Selection,
        },
    }
}

#[test]
fn tracked_item_does_not_change_selection() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    editor.apply(add_message("r", sel));

    let other = Selection::collapsed(Position::new(t1, 0));
    editor.apply(Message::Selection {
        op: SelectionOp::Set { selection: other },
    });

    editor.apply(scroll_to_tracked("r"));

    let now = editor.state().selection.expect("selection must remain set");
    assert_eq!(
        now, other,
        "scroll_into_view is scroll-only and must not modify selection"
    );
}

#[test]
fn tracked_item_emits_scroll_event() {
    let (initial, _t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    editor.apply(add_message("r", sel));

    let events = editor.apply(scroll_to_tracked("r"));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, EditorEvent::Scroll { .. })),
        "tracked_item scroll must emit Scroll event"
    );
}

#[test]
fn tracked_item_unknown_id_is_noop() {
    let (initial, ..) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 0)
    };
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    let before = editor.state().selection;

    let events = editor.apply(scroll_to_tracked("missing"));

    assert_eq!(editor.state().selection, before);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, EditorEvent::Scroll { .. })),
        "missing id must not emit Scroll"
    );
}

#[test]
fn tracked_item_explicitly_invalid_is_noop() {
    let (initial, _t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    editor.apply(add_message("r", sel));
    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::Invalidate { id: "r".into() },
    });

    let before = editor.state().selection;
    let events = editor.apply(scroll_to_tracked("r"));
    assert_eq!(editor.state().selection, before);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, EditorEvent::Scroll { .. })),
        "explicitly_invalid must not emit Scroll"
    );
}

#[test]
fn tracked_item_collapsed_on_thaw_is_noop() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    editor.apply(add_message("r", sel));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(Position::new(t1, 1), Position::new(t1, 4)),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    let before = editor.state().selection;
    let events = editor.apply(scroll_to_tracked("r"));
    assert_eq!(editor.state().selection, before);
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, EditorEvent::Scroll { .. })),
        "collapsed-on-thaw must not emit Scroll"
    );
}

#[test]
fn tracked_item_still_emits_after_document_edits() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    editor.apply(add_message("r", sel));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(Position::new(t1, 0)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });

    let events = editor.apply(scroll_to_tracked("r"));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, EditorEvent::Scroll { .. })),
        "scroll must still emit Scroll after edits shift the tracked range"
    );
}

#[test]
fn selection_emits_scroll_event() {
    let (initial, _t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });

    let events = editor.apply(scroll_to_selection());
    assert!(
        events
            .iter()
            .any(|e| matches!(e, EditorEvent::Scroll { .. })),
        "selection target must emit Scroll event when selection has rects"
    );
}

#[test]
fn selection_collapsed_is_noop() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 0)
    };
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(Position::new(t1, 2)),
        },
    });

    let events = editor.apply(scroll_to_selection());
    assert!(
        !events
            .iter()
            .any(|e| matches!(e, EditorEvent::Scroll { .. })),
        "collapsed selection has no endpoints rect, so no Scroll event"
    );
}

#[test]
fn can_returns_false_for_missing_tracked_item() {
    let (initial, ..) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 0)
    };
    let mut editor = Editor::new_test(initial);
    let probed = editor.can(scroll_to_tracked("missing")).unwrap();
    assert!(!probed, "can() must report false for unknown id");
}

#[test]
fn can_returns_true_for_revealable_tracked_item() {
    let (initial, _t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    editor.apply(add_message("r", sel));

    let probed = editor.can(scroll_to_tracked("r")).unwrap();
    assert!(probed, "can() must report true for revealable tracked item");
}
