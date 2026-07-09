use editor_commands as commands;
use editor_macros::state;
use editor_model::NodeType;
use editor_state::{Position, Selection, StableResolveCtx, StableSelection};
use editor_transaction::Transaction;

use crate::editor::Editor;
use crate::message::*;
use crate::tracked_range::TrackedRange;

fn resolve_text(editor: &Editor, sel: &StableSelection) -> (String, Selection) {
    let state = editor.state();
    let view = state.view();
    let ctx = StableResolveCtx::from_live(&view, state.projected.seq_checkout());
    let resolved = sel.resolve(&ctx).unwrap();
    let text = resolved.resolve(&view).unwrap().collect_text();
    (text, resolved)
}

#[test]
fn tracked_range_survives_lift() {
    let (initial, p1) = state! {
        doc {
            root {
                bullet_list {
                    list_item { p1: paragraph { text("item") } }
                }
                paragraph {}
            }
        }
        selection: (p1, 0)
    };

    let full_text_sel = Selection::new(Position::new(p1, 0), Position::new(p1, 4));
    let stable = StableSelection::capture(&full_text_sel, &initial.view());
    let range = TrackedRange::new(
        "r1".into(),
        "comment".into(),
        stable,
        String::new(),
        &initial,
    );

    let mut editor = Editor::new_test(initial);
    editor.tracked_ranges_mut().add(range);

    editor
        .transact(|tr| {
            commands::lift_list_item(tr)?;
            Ok(())
        })
        .unwrap();

    let located = editor
        .tracked_ranges()
        .get("r1")
        .unwrap()
        .locate(editor.state())
        .unwrap();
    let view = editor.state().view();
    assert_eq!(
        located.resolve(&view).unwrap().collect_text(),
        "item",
        "tracked range must still cover the lifted paragraph's text"
    );
}

#[test]
fn tracked_range_locate_survives_split_precisely() {
    let (state, p1) = state! {
        doc { root { p1: paragraph { text("abcd") } } }
        selection: (p1, 0)
    };
    let view_before = state.view();
    let sel_c_to_d = Selection::new(Position::new(p1, 2), Position::new(p1, 4));
    let stable = StableSelection::capture(&sel_c_to_d, &view_before);
    let range = TrackedRange::new("r".into(), "g".into(), stable, String::new(), &state);

    let mut tr = Transaction::new(&state);
    tr.split_node(p1, 2).unwrap();
    let (after, ..) = tr.commit();

    let located = range.locate(&after).unwrap();
    let view_after = after.view();
    assert_eq!(located.resolve(&view_after).unwrap().collect_text(), "cd");
}

#[test]
fn tracked_range_locate_survives_merge_precisely() {
    let (state, p1, p2) = state! {
        doc {
            root {
                p1: paragraph { text("ab") }
                p2: paragraph { text("cd") }
            }
        }
        selection: (p1, 0)
    };
    let view_before = state.view();
    let sel_in_p2 = Selection::new(Position::new(p2, 0), Position::new(p2, 2));
    let stable = StableSelection::capture(&sel_in_p2, &view_before);
    let range = TrackedRange::new("r".into(), "g".into(), stable, String::new(), &state);

    let mut tr = Transaction::new(&state);
    tr.merge_node(p1).unwrap();
    let (after, ..) = tr.commit();

    let located = range.locate(&after).unwrap();
    let view_after = after.view();
    assert_eq!(located.resolve(&view_after).unwrap().collect_text(), "cd");
}

#[test]
fn tracked_range_locate_survives_container_start_block_move() {
    // p1 is empty (0 children), so its offset-0 low endpoint captures as
    // p1 has no children, so its offset-0 low endpoint has no child dot to bind
    // against. The cross-block range keeps the overall selection non-collapsed,
    // since `TrackedRange::locate` filters collapsed ranges out structurally.
    let (state, p1, p2) = state! {
        doc {
            root {
                p1: paragraph {}
                p2: paragraph { text("world") }
            }
        }
        selection: (p1, 0)
    };
    let view_before = state.view();
    let root = view_before.root().unwrap().id();
    let sel_from_start = Selection::new(Position::new(p1, 0), Position::new(p2, 3));
    let stable = StableSelection::capture(&sel_from_start, &view_before);

    assert!(stable.anchor.child.is_none());
    assert!(stable.head.child.is_some());

    let range = TrackedRange::new("r".into(), "g".into(), stable, String::new(), &state);

    // Move p1 back into the same slot: `new_index: 0` targets root's post-deletion
    // child list (which is just [p2] once p1 is removed), so p1 lands at index 0
    // again and doc order is unchanged — but the move still deletes+reinserts p1
    // under a fresh dot and emits the alias pairing old -> new, exactly like a
    // real reordering move would.
    let mut tr = Transaction::new(&state);
    tr.move_node(p1, root, 0).unwrap();
    let (after, ..) = tr.commit();

    let located = range.locate(&after).unwrap();
    let view_after = after.view();
    assert_eq!(located.resolve(&view_after).unwrap().collect_text(), "wor");
}

#[test]
fn move_undo_redo_selection_resolves_precisely_across_generations() {
    let (initial, p1, _p2) = state! {
        doc {
            root {
                p1: paragraph { text("hello") }
                p2: paragraph { text("world") }
            }
        }
        selection: (p1, 0)
    };
    let before_view = initial.view();
    let root = before_view.root().unwrap().id();
    let sel_in_p1 = Selection::new(Position::new(p1, 1), Position::new(p1, 4));
    let captured = StableSelection::capture(&sel_in_p1, &before_view);

    let mut editor = Editor::new_test(initial);

    editor
        .transact(|tr| {
            tr.move_node(p1, root, 1)?;
            Ok(())
        })
        .unwrap();
    let (text_after_move, resolved_after_move) = resolve_text(&editor, &captured);
    assert_eq!(text_after_move, "ell");
    assert_ne!(
        resolved_after_move.anchor.node, p1,
        "the moved content must now live under a fresh dot"
    );

    let post_move_view = editor.state().view();
    let recaptured = StableSelection::capture(&resolved_after_move, &post_move_view);

    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });
    let (text_orig_via_gen1, resolved_orig) = resolve_text(&editor, &captured);
    assert_eq!(text_orig_via_gen1, "ell");
    assert_eq!(
        resolved_orig.anchor.node, p1,
        "undo must restore the original (gen-1) dot"
    );

    let (text_orig_via_gen2, _) = resolve_text(&editor, &recaptured);
    assert_eq!(
        text_orig_via_gen2, "ell",
        "an anchor captured against the moved (gen-2) dots must still resolve to the \
         original content after undo"
    );

    editor.apply(Message::History {
        op: HistoryOp::Redo,
    });
    let (text_after_redo, resolved_after_redo) = resolve_text(&editor, &captured);
    assert_eq!(text_after_redo, "ell");
    assert_ne!(
        resolved_after_redo.anchor.node, p1,
        "redo must re-apply the move onto a fresh dot, not the original"
    );
}

#[test]
fn replace_block_type_undo_redo_selection_resolves_precisely_across_generations() {
    let (initial, list, p1) = state! {
        doc {
            root {
                list: ordered_list {
                    list_item { p1: paragraph { text("hello") } }
                }
                paragraph {}
            }
        }
        selection: (p1, 0)
    };
    let before_view = initial.view();
    let sel_in_p1 = Selection::new(Position::new(p1, 1), Position::new(p1, 4));
    let captured = StableSelection::capture(&sel_in_p1, &before_view);

    let mut editor = Editor::new_test(initial);

    editor
        .transact(|tr| {
            tr.replace_block_type(list, NodeType::BulletList)?;
            Ok(())
        })
        .unwrap();
    let (text_after_replace, resolved_after_replace) = resolve_text(&editor, &captured);
    assert_eq!(text_after_replace, "ell");
    assert_ne!(
        resolved_after_replace.anchor.node, p1,
        "the replaced subtree must now live under fresh dots"
    );

    let post_replace_view = editor.state().view();
    let recaptured = StableSelection::capture(&resolved_after_replace, &post_replace_view);

    editor.apply(Message::History {
        op: HistoryOp::Undo,
    });
    let (text_orig_via_gen1, resolved_orig) = resolve_text(&editor, &captured);
    assert_eq!(text_orig_via_gen1, "ell");
    assert_eq!(
        resolved_orig.anchor.node, p1,
        "undo must restore the original paragraph dot"
    );

    let (text_orig_via_gen2, _) = resolve_text(&editor, &recaptured);
    assert_eq!(
        text_orig_via_gen2, "ell",
        "an anchor captured against the replacement dots must still resolve to the \
         original content after undo"
    );

    editor.apply(Message::History {
        op: HistoryOp::Redo,
    });
    let (text_after_redo, resolved_after_redo) = resolve_text(&editor, &captured);
    assert_eq!(text_after_redo, "ell");
    assert_ne!(
        resolved_after_redo.anchor.node, p1,
        "redo must re-apply the replacement onto fresh dots"
    );
}

#[test]
fn stable_selection_serde_roundtrip_resolves_after_remote_move() {
    let (initial, p1, _p2) = state! {
        doc {
            root {
                p1: paragraph { text("hello") }
                p2: paragraph { text("world") }
            }
        }
        selection: (p1, 0)
    };
    let view = initial.view();
    let root = view.root().unwrap().id();
    let sel = Selection::new(Position::new(p1, 1), Position::new(p1, 4));
    let captured = StableSelection::capture(&sel, &view);
    let json = serde_json::to_string(&captured).unwrap();
    let restored: StableSelection = serde_json::from_str(&json).unwrap();

    let mut editor = Editor::new_test(initial);
    editor
        .transact(|tr| {
            tr.move_node(p1, root, 1)?;
            Ok(())
        })
        .unwrap();

    let (text, resolved) = resolve_text(&editor, &restored);
    assert_eq!(text, "ell");
    assert_ne!(
        resolved.anchor.node, p1,
        "resolved onto the moved block's fresh dot, not the original"
    );
}
