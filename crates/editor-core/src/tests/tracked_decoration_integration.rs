use editor_common::{DecorationStyle, Underline, UnderlineStyle};
use editor_macros::state;
use editor_renderer::MarkData;
use editor_state::{Position, Selection};

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

fn set_group(group: &str, style: DecorationStyle, enabled: bool) -> Message {
    Message::TrackedRange {
        op: TrackedRangeOp::SetGroupDecoration {
            group: group.into(),
            style,
            enabled,
            z_index: 0,
        },
    }
}

fn background_only_style() -> DecorationStyle {
    DecorationStyle {
        background: Some("selection".into()),
        underline: None,
        ..Default::default()
    }
}

fn underline_only_style() -> DecorationStyle {
    DecorationStyle {
        background: None,
        underline: Some(Underline {
            color: "selection".into(),
            style: UnderlineStyle::Dashed,
            thickness: 1.5,
        }),
        ..Default::default()
    }
}

fn both_style() -> DecorationStyle {
    DecorationStyle {
        background: Some("selection".into()),
        underline: Some(Underline {
            color: "selection".into(),
            style: UnderlineStyle::Solid,
            thickness: 1.0,
        }),
        ..Default::default()
    }
}

fn init_editor_with_range(group: &str) -> Editor {
    let (initial, _t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    editor.apply(add_message("r", group, sel));
    editor
}

#[test]
fn no_group_decoration_produces_no_marks() {
    let editor = init_editor_with_range("spell");
    let marks = editor.tracked_decoration_marks_for_test();
    assert!(
        marks.is_empty(),
        "range without registered group decoration must produce no marks"
    );
}

#[test]
fn background_only_style_emits_single_background_mark() {
    let mut editor = init_editor_with_range("spell");
    editor.apply(set_group("spell", background_only_style(), true));
    let marks = editor.tracked_decoration_marks_for_test();
    assert_eq!(marks.len(), 1);
    assert!(matches!(marks[0].data, MarkData::TrackedBackground { .. }));
    assert!(!marks[0].rects.is_empty(), "must have at least one rect");
}

#[test]
fn underline_only_style_emits_single_underline_mark() {
    let mut editor = init_editor_with_range("spell");
    editor.apply(set_group("spell", underline_only_style(), true));
    let marks = editor.tracked_decoration_marks_for_test();
    assert_eq!(marks.len(), 1);
    assert!(matches!(marks[0].data, MarkData::TrackedUnderline { .. }));
}

#[test]
fn both_styles_emit_two_marks() {
    let mut editor = init_editor_with_range("ai");
    editor.apply(set_group("ai", both_style(), true));
    let marks = editor.tracked_decoration_marks_for_test();
    assert_eq!(marks.len(), 2);
    assert!(
        marks
            .iter()
            .any(|m| matches!(m.data, MarkData::TrackedBackground { .. }))
    );
    assert!(
        marks
            .iter()
            .any(|m| matches!(m.data, MarkData::TrackedUnderline { .. }))
    );
}

#[test]
fn disabled_group_produces_no_marks() {
    let mut editor = init_editor_with_range("spell");
    editor.apply(set_group("spell", background_only_style(), false));
    let marks = editor.tracked_decoration_marks_for_test();
    assert!(marks.is_empty(), "disabled group must not draw");
}

#[test]
fn toggling_enabled_brings_marks_back() {
    let mut editor = init_editor_with_range("spell");
    editor.apply(set_group("spell", background_only_style(), false));
    assert!(editor.tracked_decoration_marks_for_test().is_empty());
    editor.apply(set_group("spell", background_only_style(), true));
    assert_eq!(editor.tracked_decoration_marks_for_test().len(), 1);
}

#[test]
fn invalidated_range_produces_no_marks() {
    let mut editor = init_editor_with_range("spell");
    editor.apply(set_group("spell", background_only_style(), true));
    assert_eq!(editor.tracked_decoration_marks_for_test().len(), 1);
    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::Invalidate { id: "r".into() },
    });
    assert!(editor.tracked_decoration_marks_for_test().is_empty());
}

#[test]
fn collapsed_on_restore_produces_no_marks() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    editor.apply(add_message("r", "spell", sel));
    editor.apply(set_group("spell", background_only_style(), true));
    assert_eq!(editor.tracked_decoration_marks_for_test().len(), 1);

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(Position::new(t1, 1), Position::new(t1, 4)),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    assert!(
        editor.tracked_decoration_marks_for_test().is_empty(),
        "collapsed-on-restore range must not draw"
    );
}

#[test]
fn remove_group_decoration_stops_drawing() {
    let mut editor = init_editor_with_range("spell");
    editor.apply(set_group("spell", background_only_style(), true));
    assert_eq!(editor.tracked_decoration_marks_for_test().len(), 1);
    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::RemoveGroupDecoration {
            group: "spell".into(),
        },
    });
    assert!(editor.tracked_decoration_marks_for_test().is_empty());
}

#[test]
fn other_group_is_independent() {
    let mut editor = init_editor_with_range("spell");
    editor.apply(set_group("spell", background_only_style(), true));
    editor.apply(set_group("ai", underline_only_style(), true));
    let marks = editor.tracked_decoration_marks_for_test();
    assert_eq!(marks.len(), 1);
    assert!(matches!(marks[0].data, MarkData::TrackedBackground { .. }));
}

#[test]
fn rects_shift_after_text_insertion() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    editor.apply(add_message("r", "spell", sel));
    editor.apply(set_group("spell", background_only_style(), true));

    let before = editor.tracked_decoration_marks_for_test()[0].rects[0].rect;

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(Position::new(t1, 0)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "XYZ".into() },
    });

    let after = editor.tracked_decoration_marks_for_test()[0].rects[0].rect;
    assert!(
        after.x > before.x,
        "rect must shift right after insertion before the range (before x={}, after x={})",
        before.x,
        after.x
    );
}

#[test]
fn rects_restored_after_undo() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    editor.apply(add_message("r", "spell", sel));
    editor.apply(set_group("spell", background_only_style(), true));

    let original = editor.tracked_decoration_marks_for_test()[0].rects[0].rect;

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(Position::new(t1, 0)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "XYZ".into() },
    });
    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });

    let restored = editor.tracked_decoration_marks_for_test()[0].rects[0].rect;
    assert_eq!(
        restored.x, original.x,
        "rect x must return to original after undo"
    );
}
