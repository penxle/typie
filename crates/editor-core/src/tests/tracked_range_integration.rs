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

fn restored_offsets(editor: &Editor, id: &str) -> (usize, usize) {
    let range = editor.tracked_ranges().get(id).expect("range present");
    let state = editor.state();
    let view = state.view();
    let ctx = editor_state::StableResolveCtx::from_live(&view, state.projected.seq_checkout());
    let sel = range.selection.resolve(&ctx).expect("range restores");
    (sel.anchor.offset, sel.head.offset)
}

/// True when the range no longer maps back to its original covered content.
/// This is the actual signal hit-test/render/FFI use to treat a comment as
/// unlocated for current range semantics.
fn is_unlocated(editor: &Editor, id: &str) -> bool {
    let range = editor.tracked_ranges().get(id).expect("range present");
    range.locate(editor.state()).is_none()
}

fn located_text(editor: &Editor, id: &str) -> Option<String> {
    let range = editor.tracked_ranges().get(id).expect("range present");
    let sel = range.locate(editor.state())?;
    let view = editor.state().view();
    let resolved = sel.resolve(&view)?;
    Some(resolved.collect_text())
}

#[test]
fn range_position_shifts_when_text_inserted_before_it() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 1) -> (p1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    assert_eq!(restored_offsets(&editor, "r"), (1, 4));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(editor_state::Position::new(p1, 0)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });

    let (a, h) = restored_offsets(&editor, "r");
    assert_eq!((a, h), (2, 5), "range must shift by inserted length");
}

#[test]
fn range_does_not_expand_when_text_inserted_at_right_boundary() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 1) -> (p1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "comment", sel));

    assert_eq!(located_text(&editor, "r").as_deref(), Some("ell"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(editor_state::Position::new(p1, 4)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });

    assert_eq!(
        located_text(&editor, "r").as_deref(),
        Some("ell"),
        "typing at the right boundary must stay outside the comment range"
    );
}

#[test]
fn range_does_not_expand_when_text_inserted_at_left_boundary_at_paragraph_start() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 0) -> (p1, 3)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "comment", sel));

    assert_eq!(located_text(&editor, "r").as_deref(), Some("hel"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(editor_state::Position::new(p1, 0)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });

    assert_eq!(
        located_text(&editor, "r").as_deref(),
        Some("hel"),
        "typing at the left boundary must stay outside the comment range even at paragraph start"
    );
}

#[test]
fn reversed_range_does_not_expand_when_text_inserted_at_left_boundary_at_paragraph_start() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 3) -> (p1, 0)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "comment", sel));

    assert_eq!(located_text(&editor, "r").as_deref(), Some("hel"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(editor_state::Position::new(p1, 0)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });

    assert_eq!(
        located_text(&editor, "r").as_deref(),
        Some("hel"),
        "typing at the left boundary must stay outside a reversed comment range"
    );
}

#[test]
fn frozen_range_does_not_expand_when_text_inserted_at_right_boundary() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 1) -> (p1, 4)
    };
    let sel = initial.selection.unwrap();
    let frozen = editor_state::StableSelection::capture(&sel, &initial.view());
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::AddFrozen {
            id: "r".into(),
            group: "comment".into(),
            selection: frozen,
            metadata: String::new(),
        },
    });

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(editor_state::Position::new(p1, 4)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });

    assert_eq!(
        located_text(&editor, "r").as_deref(),
        Some("ell"),
        "persisted frozen comment ranges must be re-lowered to exclude right-boundary typing"
    );
}

#[test]
fn frozen_range_does_not_expand_when_text_inserted_at_left_boundary_at_paragraph_start() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 0) -> (p1, 3)
    };
    let sel = initial.selection.unwrap();
    let frozen = editor_state::StableSelection::capture(&sel, &initial.view());
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::AddFrozen {
            id: "r".into(),
            group: "comment".into(),
            selection: frozen,
            metadata: String::new(),
        },
    });

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(editor_state::Position::new(p1, 0)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });

    assert_eq!(
        located_text(&editor, "r").as_deref(),
        Some("hel"),
        "persisted frozen comment ranges must exclude left-boundary typing even at paragraph start"
    );
}

#[test]
fn range_shrinks_at_right_edge_after_covering_delete_and_undo() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("ㅁㄴㅇㅁㅁㅁㅁㄴㅁㅇ") } } }
        selection: (p1, 4) -> (p1, 6)
    };
    let frozen =
        editor_state::StableSelection::capture(&initial.selection.unwrap(), &initial.view());
    let mut editor = Editor::new_test(initial);
    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::AddFrozen {
            id: "r".into(),
            group: "comment".into(),
            selection: frozen,
            metadata: String::new(),
        },
    });
    assert_eq!(located_text(&editor, "r").as_deref(), Some("ㅁㅁ"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 0),
                editor_state::Position::new(p1, 7),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });
    assert!(
        is_unlocated(&editor, "r"),
        "range should be unlocatable while its whole content is deleted"
    );

    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });
    assert_eq!(located_text(&editor, "r").as_deref(), Some("ㅁㅁ"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(editor_state::Position::new(p1, 6)),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Move {
            movement: Movement::Grapheme {
                direction: Direction::Backward,
            },
        },
    });

    assert_eq!(
        located_text(&editor, "r").as_deref(),
        Some("ㅁ"),
        "after undo, deleting the comment's right edge must shrink from the right"
    );
}

#[test]
fn range_with_deleted_trailing_edge_expands_for_insert_inside_remaining_content() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("aabbccdd") } } }
        selection: (p1, 2) -> (p1, 6)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "comment", sel));
    assert_eq!(located_text(&editor, "r").as_deref(), Some("bbcc"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 5),
                editor_state::Position::new(p1, 7),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });
    assert_eq!(located_text(&editor, "r").as_deref(), Some("bbc"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(editor_state::Position::new(p1, 3)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });

    assert_eq!(
        located_text(&editor, "r").as_deref(),
        Some("bXbc"),
        "inserting inside the remaining comment content must expand the range"
    );
}

#[test]
fn range_with_deleted_trailing_edge_keeps_content_after_insert_before_range() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("aabbccdd") } } }
        selection: (p1, 2) -> (p1, 6)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "comment", sel));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 5),
                editor_state::Position::new(p1, 7),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });
    assert_eq!(located_text(&editor, "r").as_deref(), Some("bbc"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(editor_state::Position::new(p1, 1)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });

    assert_eq!(
        located_text(&editor, "r").as_deref(),
        Some("bbc"),
        "inserting before the comment must not shrink the right edge"
    );
}

#[test]
fn range_restores_after_full_delete_undo_following_partial_boundary_delete() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("aabbcc") } } }
        selection: (p1, 2) -> (p1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.undo_history =
        editor_state::undo::UndoHistory::new(editor_common::time::Duration::from_millis(0));
    editor.apply(add_message("r", "comment", sel));
    assert_eq!(located_text(&editor, "r").as_deref(), Some("bb"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 3),
                editor_state::Position::new(p1, 5),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });
    assert_eq!(
        editor.state().view().node(p1).unwrap().inline_text(),
        "aabc"
    );
    assert_eq!(located_text(&editor, "r").as_deref(), Some("b"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 0),
                editor_state::Position::new(p1, 4),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });
    assert!(
        is_unlocated(&editor, "r"),
        "range should be unlocatable while all remaining text is deleted"
    );

    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });

    assert_eq!(
        editor.state().view().node(p1).unwrap().inline_text(),
        "aabc"
    );
    assert_eq!(
        located_text(&editor, "r").as_deref(),
        Some("b"),
        "undoing the full delete must restore the shrunken comment range"
    );
}

#[test]
fn range_marked_invalid_when_all_covered_text_deleted() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 1) -> (p1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));
    assert!(!is_unlocated(&editor, "r"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 1),
                editor_state::Position::new(p1, 4),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    assert!(
        is_unlocated(&editor, "r"),
        "deleting the covered text must collapse the range"
    );
}

#[test]
fn range_collapses_when_text_deleted_beyond_its_bounds() {
    // Comment covers 'ell' (1..4) of "hello", but the user deletes the whole
    // word (0..5). The range must still collapse — TR-225 repro.
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 1) -> (p1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));
    assert!(!is_unlocated(&editor, "r"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 0),
                editor_state::Position::new(p1, 5),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    let (a, h) = restored_offsets(&editor, "r");
    assert!(
        is_unlocated(&editor, "r"),
        "deleting text beyond the range bounds must collapse it, got anchor={a} head={h}"
    );
}

#[test]
fn range_across_two_paragraphs_collapses_when_wider_selection_deleted() {
    // Comment spans from p1's 'llo' into p2's 'wor'. User deletes a WIDER
    // selection covering all of both paragraphs' text, merging them.
    // TR-225 repro: does the cross-node range collapse, or leave a ghost?
    let (initial, p1, p2) = state! {
        doc { root {
            p1: paragraph { text("hello") }
            p2: paragraph { text("world") }
        } }
        selection: (p1, 2) -> (p2, 3)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));
    assert!(!is_unlocated(&editor, "r"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 0),
                editor_state::Position::new(p2, 5),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    let (a, h) = restored_offsets(&editor, "r");
    assert!(
        is_unlocated(&editor, "r"),
        "deleting both paragraphs must collapse the cross-node range, got anchor={a} head={h}"
    );
}

#[test]
fn range_across_two_paragraphs_collapses_when_tail_survives() {
    // Real-app repro: comment spans p1 'llo' -> p2 'wor', but the deletion keeps
    // the TAIL of p2 alive ('ld'). Paragraphs merge; t2 survives (not fully
    // tombstoned). Does the comment still collapse, or leave a ghost on 'ld'?
    let (initial, p1, p2) = state! {
        doc { root {
            p1: paragraph { text("hello") }
            p2: paragraph { text("world") }
        } }
        selection: (p1, 2) -> (p2, 3)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));
    assert!(!is_unlocated(&editor, "r"));

    // Delete p1 'llo' .. p2 'wor' (covers the comment), leaving p2 'ld' alive.
    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 2),
                editor_state::Position::new(p2, 3),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    let (a, h) = restored_offsets(&editor, "r");
    assert!(
        is_unlocated(&editor, "r"),
        "comment whose covered text was deleted (tail survives) must be unlocatable, got anchor={a} head={h}"
    );
}

#[test]
fn undo_restores_text_after_deleting_commented_cross_paragraph_range() {
    // Repro attempt: comment spans two paragraphs, delete a wide range, then
    // undo. Does the TEXT come back? (User reports it does NOT in the app.)
    let (initial, p1, p2) = state! {
        doc { root {
            p1: paragraph { text("hello") }
            p2: paragraph { text("world") }
        } }
        selection: (p1, 2) -> (p2, 3)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 0),
                editor_state::Position::new(p2, 5),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    // After delete: p1/p2 text gone (merged). Confirm.
    let text_after_delete: String = editor
        .state()
        .view()
        .node(p1)
        .map(|n| n.inline_text())
        .unwrap_or_default();

    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });

    // After undo: p1 should read "hello" and p2 "world" again.
    let view = editor.state().view();
    let p1_text = view.node(p1).map(|n| n.inline_text());
    let p2_text = view.node(p2).map(|n| n.inline_text());
    assert_eq!(
        (p1_text.as_deref(), p2_text.as_deref()),
        (Some("hello"), Some("world")),
        "undo must restore both paragraphs' text (text_after_delete={text_after_delete:?})"
    );
}

#[test]
fn comment_in_second_paragraph_collapses_when_its_text_deleted_via_merge() {
    // The real ghost: a comment fully inside p2 ('wor' of "world"). A wide
    // delete spanning p1..into-p2 removes the commented text AND merges the
    // paragraphs, leaving p2's tail ('ld') alive. The comment's endpoints both
    // fall back onto the surviving tail -> must be unlocatable, not a ghost.
    let (initial, p1, p2) = state! {
        doc { root {
            p1: paragraph { text("hello") }
            p2: paragraph { text("world") }
        } }
        selection: (p2, 0) -> (p2, 3)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));
    assert!(!is_unlocated(&editor, "r"));

    // Delete all of p1 + 'wor' of p2, merging paragraphs, leaving p2 'ld'.
    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 0),
                editor_state::Position::new(p2, 3),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    let (a, h) = restored_offsets(&editor, "r");
    assert!(
        is_unlocated(&editor, "r"),
        "comment inside p2 whose text was deleted must be unlocatable, got anchor={a} head={h}"
    );
}

#[test]
fn range_stays_locatable_when_one_covered_char_survives() {
    // Policy: a comment must survive as long as ANY of its original characters
    // is still alive. Comment covers 'ell' (1..4) of "hello"; delete only 'el'
    // (1..3), leaving 'l'. The range must remain locatable.
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 1) -> (p1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 1),
                editor_state::Position::new(p1, 3),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    assert!(
        !is_unlocated(&editor, "r"),
        "range must survive while one covered char ('l') is still alive"
    );
}

#[test]
fn range_unaffected_by_deletion_elsewhere_stays_locatable() {
    // Guard: deleting text OUTSIDE the comment must not invalidate it.
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello world") } } }
        selection: (p1, 6) -> (p1, 11)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    // Delete 'hello ' (0..6), before the comment on 'world'.
    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 0),
                editor_state::Position::new(p1, 6),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    assert!(
        !is_unlocated(&editor, "r"),
        "comment on 'world' must survive deletion of preceding 'hello '"
    );
}

#[test]
fn collapsed_range_on_live_text_is_handled_consistently() {
    // Guard: a collapsed range (caret-position comment) on still-live text is
    // still unlocatable by the is_collapsed() rule.
    let (initial, _p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 2)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    // No deletion: locate must reject collapsed tracked ranges.
    let range = editor.tracked_ranges().get("r").unwrap();
    let located = range.locate(editor.state());
    let state = editor.state();
    let view = state.view();
    let ctx = editor_state::StableResolveCtx::from_live(&view, state.projected.seq_checkout());
    let restored_collapsed = range
        .selection
        .resolve(&ctx)
        .expect("collapsed range still restores")
        .is_collapsed();
    assert_eq!(
        located.is_none(),
        restored_collapsed,
        "tracked range locate must reject collapsed selections"
    );
}

#[test]
fn range_collapses_when_covered_text_deleted_but_following_char_survives() {
    // The real app repro that still leaks: comment covers 'wor' (0..3) of
    // "world". Delete exactly 'wor', leaving 'ld'. NO paragraph merge.
    // head was captured Bind::Right onto the char AT offset 3 ('l'), which is
    // OUTSIDE the comment and survives -> head's anchor dot is alive even though
    // every covered char ('w','o','r') is gone. Must still be unlocatable.
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("world") } } }
        selection: (p1, 0) -> (p1, 3)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));
    assert!(!is_unlocated(&editor, "r"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 0),
                editor_state::Position::new(p1, 3),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    let (a, h) = restored_offsets(&editor, "r");
    assert!(
        is_unlocated(&editor, "r"),
        "comment whose covered chars are all gone must be unlocatable even if the following char survives, got anchor={a} head={h}"
    );
}

#[test]
fn range_stays_locatable_when_endpoint_chars_die_but_middle_survives() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("abcdef") } } }
        selection: (p1, 1) -> (p1, 5)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 1),
                editor_state::Position::new(p1, 2),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 3),
                editor_state::Position::new(p1, 4),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    assert!(
        !is_unlocated(&editor, "r"),
        "range must survive while covered middle chars remain alive"
    );
}

#[test]
fn range_positions_revert_after_undo() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 1) -> (p1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(editor_state::Position::new(p1, 0)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });
    assert_eq!(restored_offsets(&editor, "r"), (2, 5));

    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });

    assert_eq!(
        restored_offsets(&editor, "r"),
        (1, 4),
        "undo must restore the original positions"
    );
}

#[test]
fn registry_membership_survives_undo() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 1) -> (p1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(editor_state::Position::new(p1, 0)),
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
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 1) -> (p1, 4)
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
    let (state, _p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 1) -> (p1, 4)
    };
    let sel = state.selection.unwrap();

    let mut a = Editor::new_test(state.clone());
    a.apply(add_message("r", "g", sel));
    let from_add = a.tracked_ranges().get("r").unwrap().clone();

    let stable = editor_state::StableSelection::capture(&sel, &state.view());
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
fn add_frozen_with_unresolvable_dots_remains_unlocated() {
    let (state_a, _p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 1) -> (p1, 4)
    };
    let sel = state_a.selection.unwrap();
    let stable = editor_state::StableSelection::capture(&sel, &state_a.view());

    let (state_b_actor1, _) = state! {
        doc { root { p2: paragraph { text("world") } } }
        selection: (p2, 0)
    };
    // Rebuild state_b under a distinct actor so none of state_a's actor-1 Dots
    // collide; otherwise the captured selection would spuriously resolve here.
    let plain_b = editor_state::to_plain(state_b_actor1.projected.projected());
    let (state_b, _) = editor_state::test_utils::build_state_from_plain_with_actor(plain_b, 2);

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
    assert!(is_unlocated(&b, "r"));
}

#[test]
fn range_recovers_from_invalid_after_undo() {
    let (state, p1) = state! {
        doc { root { p1: paragraph { text("hello") } } }
        selection: (p1, 1) -> (p1, 4)
    };
    let sel = state.selection.unwrap();
    let mut editor = Editor::new_test(state);
    editor.apply(add_message("r", "g", sel));
    assert!(!is_unlocated(&editor, "r"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 1),
                editor_state::Position::new(p1, 4),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });
    assert!(
        is_unlocated(&editor, "r"),
        "deleting covered text must collapse the range"
    );

    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });
    assert!(
        !is_unlocated(&editor, "r"),
        "undo must restore range to valid"
    );
    assert_eq!(
        located_text(&editor, "r").as_deref(),
        Some("ell"),
        "restored range must cover its original content"
    );
}

#[test]
fn redo_restores_inserted_text() {
    let (state, p1) = state! {
        doc { root { p1: paragraph { text("a") } } }
        selection: (p1, 1)
    };
    let mut editor = Editor::new_test(state);

    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "x".into() },
    });
    assert_eq!(editor.state().view().node(p1).unwrap().inline_text(), "ax");

    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });
    assert_eq!(editor.state().view().node(p1).unwrap().inline_text(), "a");

    editor.apply(Message::History {
        op: HistoryOp::Redo,
    });
    assert_eq!(editor.state().view().node(p1).unwrap().inline_text(), "ax");
}

#[test]
fn set_group_preserves_deleted_trailing_boundary_before_later_insert() {
    let (initial, p1) = state! {
        doc { root { p1: paragraph { text("abcdef") } } }
        selection: (p1, 1) -> (p1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "comment", sel));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(p1, 3),
                editor_state::Position::new(p1, 5),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });
    assert_eq!(located_text(&editor, "r").as_deref(), Some("bc"));

    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::SetGroup {
            id: "r".into(),
            group: "comment-active".into(),
        },
    });
    assert_eq!(
        editor.tracked_ranges().get("r").unwrap().group,
        "comment-active"
    );
    assert_eq!(editor.tracked_ranges().group_size("comment"), 0);
    assert_eq!(editor.tracked_ranges().group_size("comment-active"), 1);

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::collapsed(editor_state::Position::new(p1, 3)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });

    assert_eq!(
        located_text(&editor, "r").as_deref(),
        Some("bc"),
        "moving a range between decoration groups must not recapture its deleted trailing boundary"
    );
}
