use editor_commands::{self as commands};

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_insertion_intent(
    editor: &mut Editor,
    intent: InsertionIntent,
) -> Result<(), EditorError> {
    editor.transact(|tr| {
        match intent {
            InsertionIntent::Text(text) => {
                commands::insert_text(tr, &text)?;
            }
            InsertionIntent::Break(Break::Paragraph) => {
                editor_commands::first!(
                    tr,
                    commands::lift_paragraph_forward(),
                    commands::split_paragraph(),
                )?;
            }
            InsertionIntent::Break(Break::Line) => {
                commands::insert_hard_break(tr)?;
            }
            InsertionIntent::Break(Break::Page) | InsertionIntent::Node(_) => {}
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
        editor.apply(Message::Intent(Intent::Insertion(InsertionIntent::Text(
            " world".into(),
        ))));
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
        editor.apply(Message::Intent(Intent::Insertion(InsertionIntent::Break(
            Break::Paragraph,
        ))));
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
        editor.apply(Message::Intent(Intent::Insertion(InsertionIntent::Break(
            Break::Line,
        ))));
        let (expected, ..) = state! {
            doc { root { paragraph { text("hel") hard_break {} t1: text("lo") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}
