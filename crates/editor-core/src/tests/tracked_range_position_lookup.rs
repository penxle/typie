use editor_macros::state;
use editor_state::{Position, Selection};

use crate::editor::Editor;
use crate::message::*;

fn add_msg(id: &str, group: &str, sel: Selection) -> Message {
    Message::TrackedRange {
        op: TrackedRangeOp::Add {
            id: id.into(),
            group: group.into(),
            selection: sel,
            metadata: String::new(),
        },
    }
}

#[test]
fn containing_position_is_start_inclusive_and_end_exclusive() {
    let (state, p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 1) -> (p1, 4)
    };
    let sel = state.selection.unwrap();
    let mut editor = Editor::new_test(state);
    editor.apply(add_msg("r", "comment", sel));

    let at_start = editor.tracked_ranges_containing_position(Position::new(p1, 1), Some("comment"));
    assert_eq!(
        at_start.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["r"]
    );

    let at_end = editor.tracked_ranges_containing_position(Position::new(p1, 4), Some("comment"));
    assert!(
        at_end.is_empty(),
        "cursor immediately after the range must not be treated as inside it"
    );
}

#[test]
fn containing_position_filters_group_and_sorts_narrowest_first() {
    let (state, p1) = state! {
        doc { root { p1: paragraph { text("hello world") } } }
        selection: (p1, 0) -> (p1, 11)
    };
    let outer = state.selection.unwrap();
    let mut editor = Editor::new_test(state);
    editor.apply(add_msg("outer", "comment", outer));
    editor.apply(add_msg(
        "inner",
        "comment-active",
        Selection::new(Position::new(p1, 1), Position::new(p1, 4)),
    ));
    editor.apply(add_msg(
        "spellcheck",
        "spellcheck",
        Selection::new(Position::new(p1, 1), Position::new(p1, 4)),
    ));

    let all = editor.tracked_ranges_containing_position(Position::new(p1, 2), None);
    assert_eq!(
        all.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["inner", "spellcheck", "outer"],
        "all groups should still be sorted by range width, then id"
    );

    let comments = editor.tracked_ranges_containing_position(Position::new(p1, 2), Some("comment"));
    assert_eq!(
        comments.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["outer"]
    );
}
