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

/// True when the range no longer maps back to its original covered content.
/// This is the actual signal hit-test/render/FFI use to treat a comment as
/// "no location".
fn is_unlocatable(editor: &Editor, id: &str) -> bool {
    let range = editor.tracked_ranges().get(id).expect("range present");
    range.locate(&editor.state().doc).is_none()
}

fn located_text(editor: &Editor, id: &str) -> Option<String> {
    let range = editor.tracked_ranges().get(id).expect("range present");
    range
        .locate(&editor.state().doc)
        .and_then(|sel| sel.resolve(&editor.state().doc))
        .map(|resolved| resolved.collect_text())
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
    assert!(!is_unlocatable(&editor, "r"));

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
        is_unlocatable(&editor, "r"),
        "deleting the covered text must collapse the range"
    );
}

#[test]
fn range_collapses_when_text_deleted_beyond_its_bounds() {
    // Comment covers 'ell' (1..4) of "hello", but the user deletes the whole
    // word (0..5). The range must still collapse — TR-225 repro.
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));
    assert!(!is_unlocatable(&editor, "r"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(t1, 0),
                editor_state::Position::new(t1, 5),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    let (a, h) = thawed_offsets(&editor, "r");
    assert!(
        is_unlocatable(&editor, "r"),
        "deleting text beyond the range bounds must collapse it, got anchor={a} head={h}"
    );
}

#[test]
fn range_across_two_paragraphs_collapses_when_wider_selection_deleted() {
    // Comment spans from p1's 'llo' into p2's 'wor'. User deletes a WIDER
    // selection covering all of both paragraphs' text, merging them.
    // TR-225 repro: does the cross-node range collapse, or leave a ghost?
    let (initial, _p1, t1, _p2, t2) = state! {
        doc { root {
            p1: paragraph { t1: text("hello") }
            p2: paragraph { t2: text("world") }
        } }
        selection: (t1, 2) -> (t2, 3)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));
    assert!(!is_unlocatable(&editor, "r"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(t1, 0),
                editor_state::Position::new(t2, 5),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    let (a, h) = thawed_offsets(&editor, "r");
    assert!(
        is_unlocatable(&editor, "r"),
        "deleting both paragraphs must collapse the cross-node range, got anchor={a} head={h}"
    );
}

#[test]
fn range_across_two_paragraphs_collapses_when_tail_survives() {
    // Real-app repro: comment spans p1 'llo' -> p2 'wor', but the deletion keeps
    // the TAIL of p2 alive ('ld'). Paragraphs merge; t2 survives (not fully
    // tombstoned). Does the comment still collapse, or leave a ghost on 'ld'?
    let (initial, _p1, t1, _p2, t2) = state! {
        doc { root {
            p1: paragraph { t1: text("hello") }
            p2: paragraph { t2: text("world") }
        } }
        selection: (t1, 2) -> (t2, 3)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));
    assert!(!is_unlocatable(&editor, "r"));

    // Delete p1 'llo' .. p2 'wor' (covers the comment), leaving p2 'ld' alive.
    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(t1, 2),
                editor_state::Position::new(t2, 3),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    let (a, h) = thawed_offsets(&editor, "r");
    assert!(
        is_unlocatable(&editor, "r"),
        "comment whose covered text was deleted (tail survives) must be unlocatable, got anchor={a} head={h}"
    );
}

#[test]
fn undo_restores_text_after_deleting_commented_cross_paragraph_range() {
    // Repro attempt: comment spans two paragraphs, delete a wide range, then
    // undo. Does the TEXT come back? (User reports it does NOT in the app.)
    let (initial, _p1, t1, _p2, t2) = state! {
        doc { root {
            p1: paragraph { t1: text("hello") }
            p2: paragraph { t2: text("world") }
        } }
        selection: (t1, 2) -> (t2, 3)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(t1, 0),
                editor_state::Position::new(t2, 5),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    // After delete: t1/t2 text gone (merged). Confirm.
    let text_after_delete: String = editor
        .state()
        .doc
        .get_entry(t1)
        .and_then(|e| match &e.node {
            editor_model::Node::Text(t) => Some(t.text.to_string()),
            _ => None,
        })
        .unwrap_or_default();

    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });

    // After undo: t1 should read "hello" and t2 "world" again.
    let t1_text = editor
        .state()
        .doc
        .get_entry(t1)
        .and_then(|e| match &e.node {
            editor_model::Node::Text(t) => Some(t.text.to_string()),
            _ => None,
        });
    let t2_text = editor
        .state()
        .doc
        .get_entry(t2)
        .and_then(|e| match &e.node {
            editor_model::Node::Text(t) => Some(t.text.to_string()),
            _ => None,
        });
    assert_eq!(
        (t1_text.as_deref(), t2_text.as_deref()),
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
    let (initial, _p1, _t1, _p2, t2) = state! {
        doc { root {
            p1: paragraph { t1: text("hello") }
            p2: paragraph { t2: text("world") }
        } }
        selection: (t2, 0) -> (t2, 3)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));
    assert!(!is_unlocatable(&editor, "r"));

    // Delete all of p1 + 'wor' of p2, merging paragraphs, leaving p2 'ld'.
    let t1 = _t1;
    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(t1, 0),
                editor_state::Position::new(t2, 3),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    let (a, h) = thawed_offsets(&editor, "r");
    assert!(
        is_unlocatable(&editor, "r"),
        "comment inside p2 whose text was deleted must be unlocatable, got anchor={a} head={h}"
    );
}

#[test]
fn range_stays_locatable_when_one_covered_char_survives() {
    // Policy: a comment must survive as long as ANY of its original characters
    // is still alive. Comment covers 'ell' (1..4) of "hello"; delete only 'el'
    // (1..3), leaving 'l'. The range must remain locatable.
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(t1, 1),
                editor_state::Position::new(t1, 3),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    assert!(
        !is_unlocatable(&editor, "r"),
        "range must survive while one covered char ('l') is still alive"
    );
}

#[test]
fn range_unaffected_by_deletion_elsewhere_stays_locatable() {
    // Guard: deleting text OUTSIDE the comment must not invalidate it.
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("hello world") } } }
        selection: (t1, 6) -> (t1, 11)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    // Delete 'hello ' (0..6), before the comment on 'world'.
    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(t1, 0),
                editor_state::Position::new(t1, 6),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    assert!(
        !is_unlocatable(&editor, "r"),
        "comment on 'world' must survive deletion of preceding 'hello '"
    );
}

#[test]
fn collapsed_range_on_live_text_is_handled_consistently() {
    // Guard: a collapsed range (caret-position comment) on still-live text is
    // still unlocatable by the is_collapsed() rule.
    let (initial, _t1) = state! {
        doc { root { paragraph { t1: text("hello") } } }
        selection: (t1, 2)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    // No deletion: locate must agree with is_collapsed().
    let range = editor.tracked_ranges().get("r").unwrap();
    let located = range.locate(&editor.state().doc);
    let thawed_collapsed = range.selection.thaw(&editor.state().doc).is_collapsed();
    assert_eq!(
        located.is_none(),
        thawed_collapsed,
        "locate must agree with is_collapsed for a collapsed range on live text"
    );
}

#[test]
fn range_collapses_when_covered_text_deleted_but_following_char_survives() {
    // The real app repro that still leaks: comment covers 'wor' (0..3) of
    // "world". Delete exactly 'wor', leaving 'ld'. NO paragraph merge.
    // head was frozen Bind::Right onto the char AT offset 3 ('l'), which is
    // OUTSIDE the comment and survives -> head's anchor dot is alive even though
    // every covered char ('w','o','r') is gone. Must still be unlocatable.
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("world") } } }
        selection: (t1, 0) -> (t1, 3)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));
    assert!(!is_unlocatable(&editor, "r"));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(t1, 0),
                editor_state::Position::new(t1, 3),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    let (a, h) = thawed_offsets(&editor, "r");
    assert!(
        is_unlocatable(&editor, "r"),
        "comment whose covered chars are all gone must be unlocatable even if the following char survives, got anchor={a} head={h}"
    );
}

#[test]
fn range_stays_locatable_when_endpoint_chars_die_but_middle_survives() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("abcdef") } } }
        selection: (t1, 1) -> (t1, 5)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "g", sel));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(t1, 1),
                editor_state::Position::new(t1, 2),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(t1, 3),
                editor_state::Position::new(t1, 4),
            ),
        },
    });
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });

    assert!(
        !is_unlocatable(&editor, "r"),
        "range must survive while covered middle chars remain alive"
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
    assert!(is_unlocatable(&b, "r"));
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
    assert!(!is_unlocatable(&editor, "r"));

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
        is_unlocatable(&editor, "r"),
        "deleting covered text must collapse the range"
    );

    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });
    assert!(
        !is_unlocatable(&editor, "r"),
        "undo must restore range to valid"
    );
}

#[test]
fn set_group_preserves_deleted_trailing_boundary_before_later_insert() {
    let (initial, t1) = state! {
        doc { root { paragraph { t1: text("abcdef") } } }
        selection: (t1, 1) -> (t1, 4)
    };
    let sel = initial.selection.unwrap();
    let mut editor = Editor::new_test(initial);
    editor.apply(add_message("r", "comment", sel));

    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                editor_state::Position::new(t1, 3),
                editor_state::Position::new(t1, 5),
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
            selection: Selection::collapsed(editor_state::Position::new(t1, 3)),
        },
    });
    editor.apply(Message::Insertion {
        op: InsertionOp::Text { text: "X".into() },
    });

    assert_eq!(
        located_text(&editor, "r").as_deref(),
        Some("bc"),
        "moving a range between decoration groups must not refreeze its deleted trailing boundary"
    );
}
