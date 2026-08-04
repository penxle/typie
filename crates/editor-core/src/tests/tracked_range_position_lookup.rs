use editor_macros::state;
use editor_state::{Position, Selection};

use crate::editor::Editor;
use crate::message::*;

fn add_msg(id: &str, group: &str, selection: Selection) -> Message {
    Message::TrackedRange {
        op: TrackedRangeOp::Add {
            id: id.into(),
            group: group.into(),
            selection,
            metadata: String::new(),
            invalidate_on_text_change: false,
        },
    }
}

fn ids_at(editor: &Editor, position: Position, group: Option<&str>) -> Vec<String> {
    editor
        .tracked_ranges_containing_position(position, group)
        .into_iter()
        .map(|range| range.id.clone())
        .collect()
}

#[test]
fn membership_includes_both_boundaries_and_excludes_outside_positions() {
    let (state, p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 1) -> (p1, 4)
    };
    let selection = state.selection.unwrap();
    let mut editor = Editor::new_test(state);
    editor.apply(add_msg("r", "comment", selection));

    for offset in [1, 2, 4] {
        assert_eq!(
            ids_at(&editor, Position::new(p1, offset), Some("comment")),
            ["r"]
        );
    }
    assert!(ids_at(&editor, Position::new(p1, 0), Some("comment")).is_empty());
    assert!(ids_at(&editor, Position::new(p1, 5), Some("comment")).is_empty());
}

#[test]
fn membership_prefers_exact_end_over_exact_start_and_interior() {
    let (state, p1) = state! {
        doc { root { p1: paragraph { text("abcdef") } } }
        selection: (p1, 0)
    };
    let mut editor = Editor::new_test(state);
    editor.apply(add_msg(
        "a",
        "comment",
        Selection::new(Position::new(p1, 0), Position::new(p1, 3)),
    ));
    editor.apply(add_msg(
        "b",
        "comment",
        Selection::new(Position::new(p1, 3), Position::new(p1, 5)),
    ));
    editor.apply(add_msg(
        "outer",
        "comment",
        Selection::new(Position::new(p1, 0), Position::new(p1, 6)),
    ));

    assert_eq!(
        ids_at(&editor, Position::new(p1, 3), Some("comment")),
        ["a", "b", "outer"]
    );
}

#[test]
fn membership_keeps_shortest_first_without_exact_end() {
    let (state, p1) = state! {
        doc { root { p1: paragraph { text("abcdefghij") } } }
        selection: (p1, 0)
    };
    let mut editor = Editor::new_test(state);
    editor.apply(add_msg(
        "outer",
        "comment",
        Selection::new(Position::new(p1, 0), Position::new(p1, 10)),
    ));
    editor.apply(add_msg(
        "inner",
        "comment",
        Selection::new(Position::new(p1, 3), Position::new(p1, 5)),
    ));

    assert_eq!(
        ids_at(&editor, Position::new(p1, 3), Some("comment")),
        ["inner", "outer"]
    );
}

#[test]
fn membership_sorts_exact_end_ties_by_length_then_id() {
    let (state, p1) = state! {
        doc { root { p1: paragraph { text("abcdef") } } }
        selection: (p1, 0)
    };
    let mut editor = Editor::new_test(state);
    for (id, start) in [("b", 1), ("c", 2), ("a", 2)] {
        editor.apply(add_msg(
            id,
            "comment",
            Selection::new(Position::new(p1, start), Position::new(p1, 4)),
        ));
    }

    assert_eq!(
        ids_at(&editor, Position::new(p1, 4), Some("comment")),
        ["a", "c", "b"]
    );
}

#[test]
fn membership_normalizes_reversed_ranges_and_preserves_group_filter_order() {
    let (state, p1) = state! {
        doc { root { p1: paragraph { text("hello world") } } }
        selection: (p1, 0)
    };
    let mut editor = Editor::new_test(state);
    editor.apply(add_msg(
        "outer",
        "comment",
        Selection::new(Position::new(p1, 0), Position::new(p1, 11)),
    ));
    editor.apply(add_msg(
        "inner",
        "comment-active",
        Selection::new(Position::new(p1, 4), Position::new(p1, 1)),
    ));
    editor.apply(add_msg(
        "spellcheck",
        "spellcheck",
        Selection::new(Position::new(p1, 1), Position::new(p1, 4)),
    ));

    assert_eq!(
        ids_at(&editor, Position::new(p1, 2), None),
        ["inner", "spellcheck", "outer"]
    );
    assert_eq!(
        ids_at(&editor, Position::new(p1, 2), Some("comment")),
        ["outer"]
    );
}

#[test]
fn membership_excludes_invalid_and_collapsed_ranges_after_deletion() {
    let (state, p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 0)
    };
    let mut editor = Editor::new_test(state);
    editor.apply(add_msg(
        "invalid",
        "comment",
        Selection::new(Position::new(p1, 0), Position::new(p1, 1)),
    ));
    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::Invalidate {
            id: "invalid".into(),
        },
    });
    editor.apply(add_msg(
        "collapsed",
        "comment",
        Selection::collapsed(Position::new(p1, 1)),
    ));
    editor.apply(add_msg(
        "deleted",
        "comment",
        Selection::new(Position::new(p1, 1), Position::new(p1, 4)),
    ));
    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(Position::new(p1, 1), Position::new(p1, 4)),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    assert!(ids_at(&editor, Position::new(p1, 1), None).is_empty());
}

#[test]
fn membership_uses_document_positions_across_paragraph_boundaries() {
    let (state, p1, p2) = state! {
        doc {
            root {
                p1: paragraph { text("ab") }
                p2: paragraph { text("cd") }
            }
        }
        selection: (p1, 0)
    };
    let mut editor = Editor::new_test(state);
    editor.apply(add_msg(
        "before",
        "comment",
        Selection::new(Position::new(p1, 1), Position::new(p2, 0)),
    ));
    editor.apply(add_msg(
        "after",
        "comment",
        Selection::new(Position::new(p2, 0), Position::new(p2, 2)),
    ));

    assert_eq!(
        ids_at(&editor, Position::new(p1, 1), Some("comment")),
        ["before"]
    );
    assert_eq!(
        ids_at(&editor, Position::new(p2, 0), Some("comment")),
        ["before", "after"]
    );
    assert_eq!(
        ids_at(&editor, Position::new(p2, 2), Some("comment")),
        ["after"]
    );
}
