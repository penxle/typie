use editor_commands::{self as commands};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_key_event(editor: &mut Editor, event: KeyEvent) -> Result<(), EditorError> {
    editor.transact(|tr| {
        match (event.key, event.modifiers) {
            (Key::Enter, m) if m.shift => {
                commands::insert_hard_break(tr)?;
            }
            (Key::Enter, _) => {
                commands::first!(
                    tr,
                    commands::delete_selection(),
                    commands::split_paragraph(),
                )?;
            }
            (Key::Backspace, _) => {
                commands::first!(
                    tr,
                    commands::delete_selection(),
                    commands::delete_text_backward(),
                    commands::delete_node_backward(),
                    commands::join_paragraph_backward(),
                    commands::sink_paragraph_backward(),
                )?;
            }
            (Key::Delete, _) => {
                commands::first!(
                    tr,
                    commands::delete_selection(),
                    commands::delete_text_forward(),
                    commands::delete_node_forward(),
                    commands::join_paragraph_forward(),
                    commands::lift_paragraph_forward()
                )?;
            }
            (Key::Escape, _) | (Key::Tab, _) => {}
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
}
