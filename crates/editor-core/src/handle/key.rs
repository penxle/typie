use std::sync::Arc;

use editor_commands::{self as commands};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_key_event(editor: &mut Editor, event: KeyEvent) -> Result<(), EditorError> {
    let resource = Arc::clone(&editor.resource);
    let resource = resource.lock().unwrap();
    editor.transact(|tr| {
        match (event.key, event.modifiers) {
            (Key::Enter, m) if m.shift => {
                commands::first!(
                    tr,
                    commands::insert_paragraph_before_unit_selection(),
                    |tr| commands::chain!(
                        tr,
                        commands::optional!(commands::ensure_paragraph()),
                        commands::optional!(commands::delete_selection()),
                        commands::insert_hard_break(),
                    ),
                )?;
            }
            (Key::Enter, _) => {
                commands::first!(
                    tr,
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
            }
            (Key::Backspace, _) => {
                commands::first!(
                    tr,
                    commands::delete_selection(),
                    commands::delete_text_backward(&resource),
                    commands::delete_node_backward(),
                    commands::select_node_backward(),
                    commands::delete_page_break_backward(),
                    commands::lift_empty_list_item(),
                    commands::merge_list_item_backward(),
                    commands::lift_first_list_item(),
                    commands::join_paragraph_backward(),
                    commands::sink_paragraph_backward(),
                    commands::lift_first_paragraph(),
                    commands::delete_empty_paragraph_backward(),
                )?;
            }
            (Key::Delete, _) => {
                commands::first!(
                    tr,
                    commands::delete_selection(),
                    commands::delete_text_forward(&resource),
                    commands::delete_node_forward(),
                    commands::select_node_forward(),
                    commands::delete_page_break_forward(),
                    commands::merge_list_item_forward(),
                    commands::join_paragraph_forward(),
                    commands::lift_paragraph_forward(),
                    commands::delete_empty_paragraph_forward(),
                )?;
            }
            (Key::Tab, m) if m.shift => {
                commands::lift_list_item(tr)?;
            }
            (Key::Tab, _) => {
                commands::sink_list_item(tr)?;
            }
            (Key::Escape, _) => {}
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::assert_state_eq;

    use super::*;

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
    fn enter_splits_paragraph() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Enter));
        let (expected, ..) = state! {
            doc { root { paragraph { text("hel") } paragraph { t1: text("lo") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn shift_enter_inserts_hard_break() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key_shift(Key::Enter));
        let (expected, ..) = state! {
            doc { root { paragraph { text("hel") hard_break {} t1: text("lo") } } }
            selection: (t1, 0)
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
    fn backspace_deletes_text_backward() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("helo") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_at_start_joins_paragraph() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("hello") }
                    paragraph { t2: text("world") }
                }
            }
            selection: (t2, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("helloworld") } } }
            selection: (t1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_deletes_text_forward() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("helo") } } }
            selection: (t1, 3)
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
                    paragraph { t1: text("1234") }
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {}
                    paragraph { t1: text("1234") }
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn two_backspaces_merge_page_break_paragraph_into_text_paragraph() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { page_break }
                    paragraph { t1: text("1234") }
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("1234") }
                }
            }
            selection: (t1, 0)
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
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("a") }
                    paragraph { text("b") }
                }
            }
            selection: (t1, 1)
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
                    paragraph { t1: text("ab") }
                }
            }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn enter_splits_list_item() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { t1: text("Hello") } } }
                    paragraph {}
                }
            }
            selection: (t1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Enter));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("Hello") } }
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
    fn backspace_merges_list_items() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("Hello") } }
                        list_item { paragraph { t2: text("World") } }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("HelloWorld") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn backspace_lifts_first_list_item() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("A") } }
                        list_item { paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                    bullet_list { list_item { paragraph { text("B") } } }
                    paragraph {}
                }
            }
            selection: (t1, 0)
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
                        list_item { paragraph { t1: text("Hello") } }
                        list_item { paragraph { text("World") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("HelloWorld") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_at_end_of_last_list_item_pulls_next_paragraph() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { t1: text("A") } } }
                    paragraph { text("B") }
                }
            }
            selection: (t1, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { t1: text("AB") } } }
                    paragraph {}
                }
            }
            selection: (t1, 1)
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
                        list_item { paragraph { t1: text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Tab));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list { list_item { paragraph { t1: text("B") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
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
                            bullet_list { list_item { paragraph { t1: text("B") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key_shift(Key::Tab));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { paragraph { t1: text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn tab_outside_list_no_op() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Tab));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn tab_first_item_no_op() {
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { t1: text("A") } } }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Tab));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { t1: text("A") } } }
                    paragraph {}
                }
            }
            selection: (t1, 0)
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
            doc { root { paragraph { text("ab") hard_break t: text("cd") } } }
            selection: (t, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Backspace));
        let (expected, ..) = state! {
            doc { root { paragraph { text("ab") t: text("cd") } } }
            selection: (t, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_over_hard_break_is_unaffected_by_unit_gate() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("ab") hard_break text("cd") } } }
            selection: (t1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(key(Key::Delete));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("ab") text("cd") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}
