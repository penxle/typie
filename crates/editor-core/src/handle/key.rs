use std::sync::Arc;

use editor_commands::{self as commands};
use editor_state::Selection;
use editor_transaction::HistoryMeta;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_key_event(editor: &mut Editor, event: KeyEvent) -> Result<(), EditorError> {
    match (event.key, event.modifiers) {
        (Key::Enter, m) if m.shift => editor.transact(|tr| {
            commands::first!(
                tr,
                commands::materialize_gap_paragraph(),
                commands::insert_paragraph_before_unit_selection(),
                |tr| commands::chain!(
                    tr,
                    commands::optional!(commands::ensure_paragraph()),
                    commands::optional!(commands::delete_selection()),
                    commands::insert_hard_break(),
                ),
            )?;
            Ok(())
        }),
        (Key::Enter, _) => editor.transact(|tr| {
            let applied = commands::first!(
                tr,
                commands::materialize_gap_paragraph(),
                commands::insert_paragraph_after_unit_selection(),
                |tr| commands::chain!(
                    tr,
                    commands::optional!(commands::delete_selection()),
                    |tr| commands::first!(
                        tr,
                        commands::lift_empty_list_item(),
                        commands::split_list_item(),
                        commands::lift_last_paragraph(),
                        commands::split_paragraph(),
                    ),
                ),
            )?;
            if applied {
                tr.clear_pending_format()?;
            }
            Ok(())
        }),
        (Key::Backspace, _) if editor.try_undo_auto_replacement() => {
            if !editor.state().pending_modifiers.is_empty() {
                editor.transact(|tr| {
                    tr.update_meta(|m| m.history = HistoryMeta::Skip);
                    tr.clear_pending_format()?;
                    Ok(())
                })?;
            }
            Ok(())
        }
        (Key::Backspace, _) => {
            let resource = Arc::clone(&editor.resource);
            let resource = resource.lock().unwrap();
            editor.transact(|tr| {
                commands::first!(
                    tr,
                    commands::delete_selection(),
                    commands::select_unit_across_gap_backward(),
                    commands::delete_text_backward(&resource),
                    commands::delete_node_backward(),
                    commands::select_node_backward(),
                    commands::delete_page_break_backward(),
                    commands::lift_empty_list_item(),
                    commands::merge_list_item_backward(),
                    commands::lift_first_list_item(),
                    commands::join_paragraph_backward_into_prev_list_item(),
                    commands::join_paragraph_backward(),
                    commands::sink_paragraph_backward(),
                    commands::lift_first_paragraph(),
                    commands::delete_empty_paragraph_backward(),
                )?;
                Ok(())
            })
        }
        (Key::Delete, _) => {
            let resource = Arc::clone(&editor.resource);
            let resource = resource.lock().unwrap();
            editor.transact(|tr| {
                commands::first!(
                    tr,
                    commands::delete_selection(),
                    commands::select_unit_across_gap_forward(),
                    commands::delete_text_forward(&resource),
                    commands::delete_node_forward(),
                    commands::select_node_forward(),
                    commands::delete_page_break_forward(),
                    commands::merge_list_item_forward(),
                    commands::join_next_paragraph_forward_into_list_item(),
                    commands::join_paragraph_forward(),
                    commands::lift_paragraph_forward(),
                    commands::delete_empty_paragraph_forward(),
                )?;
                Ok(())
            })
        }
        (Key::Tab, m) if m.shift => editor.transact(|tr| {
            let applied = commands::first!(
                tr,
                commands::lift_list_items_in_range(),
                commands::lift_list_item_at_start(),
                commands::delete_preceding_tab(),
            )?;
            if applied {
                tr.clear_pending_format()?;
            }
            Ok(())
        }),
        (Key::Tab, _) => editor.transact(|tr| {
            let applied = commands::first!(
                tr,
                commands::sink_list_items_in_range(),
                commands::sink_list_item_at_start(),
                commands::insert_tab(),
            )?;
            if applied {
                tr.clear_pending_format()?;
            }
            Ok(())
        }),
        (Key::Escape, _) => editor.transact(|tr| {
            if let Some(current) = tr.selection() {
                let collapsed = Selection::collapsed(current.head);
                let normalized = collapsed.normalize(&tr.view()).unwrap_or(collapsed);
                // Unit selections re-normalize to the direction-flipped form;
                // an anchor/head swap brackets the same content.
                let unchanged = normalized == current
                    || (normalized.anchor == current.head && normalized.head == current.anchor);
                if unchanged {
                    tr.set_selection(None)?;
                } else {
                    tr.set_selection(Some(collapsed))?;
                }
                tr.clear_pending_format()?;
            }
            Ok(())
        }),
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::{Modifier, NodeType};
    use editor_state::{
        Affinity, PendingModifier, Position, Selection, assert_doc_eq, assert_state_eq,
    };

    use super::*;
    use crate::test_utils::assert_probe_predicts_apply;

    fn set_pending_format(editor: &mut Editor) {
        editor
            .transact(|tr| {
                tr.set_pending_modifiers(vec![PendingModifier::Set {
                    modifier: Modifier::Bold,
                }])?;
                Ok(())
            })
            .unwrap();
    }

    fn assert_pending_format_cleared(editor: &Editor) {
        assert!(
            editor.state().pending_modifiers.is_empty(),
            "pending modifiers cleared"
        );
    }

    #[test]
    fn probe_backspace_at_boundary() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        assert_probe_predicts_apply(
            state,
            Message::Key {
                event: KeyEvent {
                    key: Key::Backspace,
                    modifiers: InputModifiers::default(),
                },
            },
        );
    }

    fn key(k: Key) -> Message {
        Message::Key {
            event: KeyEvent {
                key: k,
                modifiers: InputModifiers::default(),
            },
        }
    }

    fn key_shift(k: Key) -> Message {
        Message::Key {
            event: KeyEvent {
                key: k,
                modifiers: InputModifiers {
                    shift: true,
                    ..Default::default()
                },
            },
        }
    }

    #[test]
    fn enter_in_fold_title_is_noop() {
        let (state, ..) = state! {
            doc {
                root {
                    fold {
                        ft1: fold_title {}
                        fold_content { paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (ft1, 0)
        };
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        ft1: fold_title {}
                        fold_content { paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (ft1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Enter));
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_at_cell_paragraph_end_does_not_cross_cell_boundary() {
        let (state, ..) = state! {
            doc { root { table { table_row {
                table_cell { p1: paragraph { text("A") } }
                table_cell { paragraph { text("B") } }
            } } } }
            selection: (p1, 1)
        };
        let (expected, ..) = state! {
            doc { root { table { table_row {
                table_cell { p1: paragraph { text("A") } }
                table_cell { paragraph { text("B") } }
            } } } }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn enter_splits_paragraph() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Enter));
        let (expected, ..) = state! {
            doc { root { paragraph { text("hel") } p2: paragraph { text("lo") } } }
            selection: (p2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn enter_in_empty_blockquote_after_image_before_text_paragraph() {
        let (state, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    image
                    blockquote {
                        p1: paragraph {}
                    }
                    paragraph {
                        text("1")
                    }
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Enter));
        let (expected, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    image
                    p1: paragraph {}
                    paragraph {
                        text("1")
                    }
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn enter_in_empty_blockquote_after_image_before_empty_paragraph() {
        let (state, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    image
                    blockquote {
                        p1: paragraph {}
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Enter));
        let (expected, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    image
                    p1: paragraph {}
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn enter_in_synthetic_trailing_paragraph_after_image() {
        let (state, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    image
                }
            }
            selection: none
        };
        let synth_p = {
            let view = state.view();
            let root = view.root().unwrap();
            root.child_blocks()
                .find(|b| b.node_type() == NodeType::Paragraph)
                .map(|b| b.id())
                .expect("synthetic trailing paragraph")
        };
        assert!(
            synth_p.is_synthetic(),
            "trailing paragraph must be synthetic"
        );

        let mut editor = Editor::new_test(state);
        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: Selection::collapsed(Position {
                    node: synth_p,
                    offset: 0,
                    affinity: Affinity::Downstream,
                }),
            },
        });
        editor.apply(key(Key::Enter));
        let (expected, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    image
                    paragraph {}
                    p2: paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn shift_enter_inserts_hard_break() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key_shift(Key::Enter));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hel") hard_break {} text("lo") } } }
            selection: (p1, 4)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn enter_on_unit_node_selection_inserts_paragraph_after() {
        let (state, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Enter));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                horizontal_rule
                p1: paragraph
                paragraph { text("c") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn shift_enter_on_unit_node_selection_inserts_paragraph_above() {
        let (state, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key_shift(Key::Enter));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                p1: paragraph
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_then_undo_restores_multi_paragraph_selection() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("11") }
                    p2: paragraph { text("22") }
                }
            }
            selection: (p1, 1, >) -> (p2, 1, <)
        };
        let initial = state.clone();
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);
    }

    #[test]
    fn backspace_then_undo_restores_multi_char_inner_selection() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("abc") }
                    p2: paragraph { text("def") }
                }
            }
            selection: (p1, 2, >) -> (p2, 2, <)
        };
        let initial = state.clone();
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);
    }

    #[test]
    fn undo_after_redeleting_restored_text_restores_cursor_after_restored_text() {
        let (state, ..) = state! {
            doc {
                root { p1: paragraph { text("aabb") } }
            }
            selection: (p1, 2) -> (p1, 4)
        };
        let after_restore = state.clone();
        let mut editor = Editor::new_test(state);

        editor.apply(key(Key::Backspace));
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &after_restore);

        editor.apply(Message::Navigation {
            op: NavigationOp::Move {
                movement: Movement::Grapheme {
                    direction: Direction::Forward,
                },
                extend: false,
            },
        });
        let (collapsed_after_restore, ..) = state! {
            doc {
                root { p1: paragraph { text("aabb") } }
            }
            selection: (p1, 4)
        };
        assert_state_eq!(editor.state(), &collapsed_after_restore);

        editor.apply(key(Key::Backspace));
        editor.apply(key(Key::Backspace));
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });

        let (expected, ..) = state! {
            doc {
                root { p1: paragraph { text("aabb") } }
            }
            selection: (p1, 4)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_across_callout_boundary_round_trips_through_undo() {
        let (state, ..) = state! {
            doc {
                root {
                    callout { p1: paragraph { text("1") } }
                    callout { p2: paragraph { text("2") } }
                    paragraph {}
                }
            }
            selection: (p1, 1, >) -> (p2, 1, <)
        };
        let initial = state.clone();
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);
    }

    #[test]
    fn backspace_across_blockquote_boundary_round_trips_through_undo() {
        let (state, ..) = state! {
            doc {
                root {
                    blockquote { p1: paragraph { text("a") } }
                    blockquote { p2: paragraph { text("b") } }
                    paragraph {}
                }
            }
            selection: (p1, 1, >) -> (p2, 1, <)
        };
        let initial = state.clone();
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);
    }

    #[test]
    fn backspace_at_list_item_start_round_trips_through_undo() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("a") } }
                        list_item { p1: paragraph { text("b") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let initial = state.clone();
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);
    }

    #[test]
    fn delete_at_list_item_end_round_trips_through_undo() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("a") } }
                        list_item { paragraph { text("b") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        let initial = state.clone();
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);
    }

    #[test]
    fn delete_lifts_callout_paragraph_round_trips_through_undo() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    callout { paragraph { text("b") } }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        let initial = state.clone();
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);
    }

    #[test]
    fn backspace_deletes_text_backward() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("helo") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_at_start_joins_paragraph() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("hello") }
                    p2: paragraph { text("world") }
                }
            }
            selection: (p2, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("helloworld") } } }
            selection: (p1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_deletes_text_forward() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("helo") } } }
            selection: (p1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_removes_empty_paragraph_before_fold() {
        let (state, ..) = state! {
            doc { root {
                p1: paragraph {}
                fold {
                    fold_title { text("title") }
                    fold_content { paragraph { text("content") } }
                }
                paragraph {}
            } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc { r1: root {
                fold {
                    fold_title { text("title") }
                    fold_content { paragraph { text("content") } }
                }
                paragraph {}
            } }
            selection: (r1, 1, <) -> (r1, 0, >)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_at_start_of_only_callout_paragraph_removes_callout() {
        let (state, ..) = state! {
            doc {
                root {
                    callout { p1: paragraph {} }
                    horizontal_rule
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc {
                r1: root {
                    horizontal_rule
                    paragraph {}
                }
            }
            selection: (r1, 1, <) -> (r1, 0, >)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_removes_empty_paragraph_after_fold() {
        let (state, ..) = state! {
            doc { root {
                paragraph {}
                fold {
                    fold_title { text("title") }
                    fold_content { paragraph { text("content") } }
                }
                p2: paragraph {}
            } }
            selection: (p2, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc { r1: root {
                paragraph {}
                fold {
                    fold_title { text("title") }
                    fold_content { paragraph { text("content") } }
                }
                paragraph {}
            } }
            selection: (r1, 1, >) -> (r1, 2, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_at_start_of_only_callout_paragraph_removes_callout() {
        let (state, ..) = state! {
            doc {
                root {
                    horizontal_rule
                    callout { p1: paragraph {} }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc {
                r1: root {
                    horizontal_rule
                    paragraph {}
                }
            }
            selection: (r1, 0, >) -> (r1, 1, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_at_start_of_paragraph_after_page_break_paragraph_removes_marker() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { page_break }
                    p1: paragraph { text("1234") }
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {}
                    p1: paragraph { text("1234") }
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn two_backspaces_merge_page_break_paragraph_into_text_paragraph() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { page_break }
                    p1: paragraph { text("1234") }
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("1234") }
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_at_paragraph_end_with_trailing_page_break_removes_marker() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") page_break }
                    paragraph { text("b") }
                }
            }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    paragraph { text("b") }
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn two_deletes_merge_text_paragraph_with_page_break_paragraph() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") page_break }
                    paragraph { text("b") }
                }
            }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("ab") }
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn enter_splits_list_item() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("Hello") } } }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Enter));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                        list_item { p2: paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn enter_on_empty_list_item_lifts_out() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Enter));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { text("A") } } }
                    p1: paragraph {}
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn enter_on_empty_list_item_after_image_keeps_selection_live() {
        let (state, ..) = state! {
            doc {
                root {
                    image
                    bullet_list {
                        list_item { p1: paragraph {} }
                    }
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Enter));
        let (expected, ..) = state! {
            doc {
                root {
                    image
                    p1: paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_merges_list_items() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                        list_item { p2: paragraph { text("World") } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("HelloWorld") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_merges_list_items_preserves_bold() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") [bold] } }
                        list_item { p2: paragraph { text("World") [bold] } }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("HelloWorld") [bold] } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_lifts_first_list_item() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                        list_item { paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    bullet_list { list_item { paragraph { text("B") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_on_empty_nested_list_item_unindents() {
        // An empty list_item at any nesting level should unindent on Backspace
        // (matches Google Docs / Notion). The presence of a prev sibling does
        // not change this — empty list_items always lift; merge is reserved
        // for non-empty content that has somewhere to flow into.
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("a") }
                            bullet_list {
                                list_item {
                                    paragraph { text("b") }
                                    bullet_list {
                                        list_item { paragraph { text("c") } }
                                    }
                                }
                                list_item { p1: paragraph {} }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("a") }
                            bullet_list {
                                list_item {
                                    paragraph { text("b") }
                                    bullet_list {
                                        list_item { paragraph { text("c") } }
                                    }
                                }
                            }
                        }
                        list_item { p1: paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_merges_list_items_forward() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                        list_item { paragraph { text("World") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("HelloWorld") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_at_end_of_last_list_item_pulls_next_paragraph() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph { text("B") }
                }
            }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("AB") } } }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_at_end_of_synthetic_last_list_item_pulls_next_paragraph() {
        let (state, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    bullet_list {
                        synthetic list_item {
                            p1: synthetic paragraph {}
                        }
                    }
                    paragraph { text("B") }
                }
            }
            selection: (p1, 0)
        };
        assert_probe_predicts_apply(state.clone(), key(Key::Delete));
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    bullet_list {
                        list_item { p1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_at_paragraph_start_after_list_joins_into_last_list_item() {
        let (state, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                    }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p2, 0)
        };
        assert_probe_predicts_apply(state.clone(), key(Key::Backspace));
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    bullet_list {
                        list_item { p1: paragraph { text("AB") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_at_paragraph_start_after_synthetic_empty_list_joins_into_item() {
        let (state, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    bullet_list {
                        synthetic list_item {
                            p1: synthetic paragraph {}
                        }
                    }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p2, 0)
        };
        assert_probe_predicts_apply(state.clone(), key(Key::Backspace));
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    bullet_list {
                        list_item { p1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn tab_indents_list_item() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Tab));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list { list_item { p1: paragraph { text("B") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn shift_tab_unindents_list_item() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list { list_item { p1: paragraph { text("B") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key_shift(Key::Tab));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn tab_indents_non_collapsed_list_range() {
        let (state, _, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                        list_item { p2: paragraph { text("C") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Tab));
        let (expected, _, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list {
                                list_item { p1: paragraph { text("B") } }
                                list_item { p2: paragraph { text("C") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn shift_tab_unindents_non_collapsed_mixed_range_list_items_only() {
        let (state, _, _) = state! {
            doc {
                root {
                    p0: paragraph { text("plain") }
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p0, 0) -> (p1, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key_shift(Key::Tab));
        let (expected, _, _) = state! {
            doc {
                root {
                    p0: paragraph { text("plain") }
                    p1: paragraph { text("A") }
                    paragraph {}
                }
            }
            selection: (p0, 0) -> (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn shift_tab_unindents_non_collapsed_list_range_preserves_selection() {
        let (state, _, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list {
                                list_item { p1: paragraph { text("B") } }
                                list_item { p2: paragraph { text("C") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key_shift(Key::Tab));
        let (expected, _, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                        list_item { p2: paragraph { text("C") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn tab_in_paragraph_inserts_tab() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Tab));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("He") tab text("llo") } } }
            selection: (p1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn tab_first_item_no_op() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Tab));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn shift_tab_deletes_preceding_tab_in_paragraph() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key_shift(Key::Tab));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("a") text("b") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn tab_at_list_item_start_still_indents() {
        let (state, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { paragraph { text("A") } }
                    list_item { p1: paragraph { text("B") } }
                }
                paragraph {}
            } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Tab));
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item {
                        paragraph { text("A") }
                        bullet_list { list_item { p1: paragraph { text("B") } } }
                    }
                }
                paragraph {}
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn tab_mid_list_item_inserts_tab() {
        let (state, ..) = state! {
            doc { root {
                bullet_list { list_item { p1: paragraph { text("AB") } } }
                paragraph {}
            } }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Tab));
        let (expected, ..) = state! {
            doc { root {
                bullet_list { list_item { p1: paragraph { text("A") tab text("B") } } }
                paragraph {}
            } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    // Characterization guard for the editor-commands `is_unit()` gate change:
    // an inline leaf (`hard_break`) as the backward/forward neighbor must be
    // consumed by `delete_node_*` before `select_node_*` ever sees it, so the
    // `is_unit` vs `leaf || monolithic` divergence is unreachable here.
    #[test]
    fn backspace_over_hard_break_is_unaffected_by_unit_gate() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("ab") hard_break text("cd") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ab") text("cd") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_over_hard_break_is_unaffected_by_unit_gate() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("ab") hard_break text("cd") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ab") text("cd") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_over_tab_removes_it() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("ab") tab text("cd") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ab") text("cd") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_over_tab_removes_it() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("ab") tab text("cd") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ab") text("cd") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_after_text_following_tab_removes_text() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("a") tab } } }
            selection: (p1, 2, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_before_text_following_tab_removes_text() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("a") tab } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_at_between_monolithic_gap_selects_prev_monolithic() {
        let (state, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_selection_only_clears_pending_format() {
        let (state, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 1)
        };
        let mut editor = Editor::new_test(state);
        set_pending_format(&mut editor);

        editor.apply(key(Key::Backspace));

        assert_pending_format_cleared(&editor);
    }

    #[test]
    fn second_backspace_at_between_monolithic_gap_deletes_prev_monolithic() {
        let (state, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc { r: root {
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 0, <)
        };
        assert_doc_eq!(editor.state().clone(), expected);
    }

    #[test]
    fn delete_at_between_monolithic_gap_selects_next_monolithic() {
        let (state, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn forward_delete_preserves_pending_format_when_cursor_stays_put() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        set_pending_format(&mut editor);

        editor.apply(key(Key::Delete));

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("helo") } } }
            selection: (p1, 2)
            pending_modifiers: [bold]
        };
        assert_state_eq!(editor.state(), &expected);
        assert!(
            !editor.state().pending_modifiers.is_empty(),
            "forward delete keeps pending when the caret stays put"
        );
    }

    #[test]
    fn second_delete_at_between_monolithic_gap_deletes_next_monolithic() {
        let (state, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                paragraph {}
            } }
            selection: (r, 1)
        };
        assert_doc_eq!(editor.state().clone(), expected);
    }

    #[test]
    fn backspace_at_leading_gap_selects_leading_unit() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_at_leading_gap_selects_leading_unit() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn second_backspace_at_leading_gap_deletes_leading_unit() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("b") } } }
            selection: (p1, 0)
        };
        assert_doc_eq!(editor.state().clone(), expected);
    }

    #[test]
    fn second_delete_at_leading_gap_deletes_leading_unit() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("b") } } }
            selection: (p1, 0)
        };
        assert_doc_eq!(editor.state().clone(), expected);
    }

    #[test]
    fn escape_collapses_text_range_to_head() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Escape));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 4)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn escape_clears_pending_format() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let mut editor = Editor::new_test(state);
        set_pending_format(&mut editor);

        editor.apply(key(Key::Escape));

        assert_pending_format_cleared(&editor);
    }

    #[test]
    fn escape_on_collapsed_caret_clears_selection() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Escape));
        assert!(editor.state().selection.is_none());
    }

    #[test]
    fn escape_on_unit_node_selection_clears_selection() {
        let (state, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                image
                paragraph { text("b") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Escape));
        assert!(editor.state().selection.is_none());
    }

    #[test]
    fn escape_when_selection_is_none_is_noop() {
        let (state, ..) = state! {
            doc { root { paragraph { text("hello") } } }
            selection: none
        };
        let initial = state.clone();
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Escape));
        assert_state_eq!(editor.state(), &initial);
    }

    #[test]
    fn enter_dissolves_pending_and_new_paragraph_typing_is_plain() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        set_pending_format(&mut editor);

        editor.apply(key(Key::Enter));
        assert_pending_format_cleared(&editor);

        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "X".into() },
        });

        let view = editor.state().view();
        let second = view
            .root()
            .unwrap()
            .child_blocks()
            .nth(1)
            .expect("second paragraph");
        let items = second.inline();
        let first = items.first().expect("typed text in second paragraph");
        assert!(
            !first
                .effective
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Bold)),
            "typing after Enter must not inherit the dissolved pending bold"
        );
    }

    #[test]
    fn undo_dissolves_pending_rather_than_restoring() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "X".into() },
        });
        set_pending_format(&mut editor);
        assert!(!editor.state().pending_modifiers.is_empty());

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });

        assert_pending_format_cleared(&editor);
    }

    #[test]
    fn redo_dissolves_pending_rather_than_restoring() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "X".into() },
        });
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        set_pending_format(&mut editor);
        assert!(!editor.state().pending_modifiers.is_empty());

        editor.apply(Message::History {
            op: HistoryOp::Redo,
        });

        assert_pending_format_cleared(&editor);
    }
}
