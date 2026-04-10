use editor_commands::{self as commands};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_insertion_op(editor: &mut Editor, op: InsertionOp) -> Result<(), EditorError> {
    editor.transact(|tr| {
        match op {
            InsertionOp::Text { text } => {
                commands::chain!(
                    tr,
                    commands::optional!(commands::delete_selection()),
                    commands::insert_text(&text),
                )?;
            }
            InsertionOp::Break {
                kind: Break::Paragraph,
            } => {
                commands::chain!(
                    tr,
                    commands::optional!(commands::delete_selection()),
                    |tr| commands::first!(
                        tr,
                        commands::lift_paragraph_forward(),
                        commands::split_paragraph(),
                    ),
                )?;
            }
            InsertionOp::Break { kind: Break::Line } => {
                commands::chain!(
                    tr,
                    commands::optional!(commands::delete_selection()),
                    commands::insert_hard_break(),
                )?;
            }
            InsertionOp::Break { kind: Break::Page } => {}
            InsertionOp::Fragment { fragment } => {
                commands::chain!(
                    tr,
                    commands::optional!(commands::delete_selection()),
                    commands::insert_fragment(fragment.clone()),
                )?;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::assert_state_eq;

    use super::*;

    #[test]
    fn insert_text() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Text {
                text: " world".into(),
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 11)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn insert_break_block() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Break {
                kind: Break::Paragraph,
            },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { text("hel") } paragraph { t1: text("lo") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn insert_break_line() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Break { kind: Break::Line },
        });
        let (expected, ..) = state! {
            doc { root { paragraph { text("hel") hard_break {} t1: text("lo") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}
