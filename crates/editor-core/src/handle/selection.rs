use editor_commands::{self as commands};
use editor_transaction::HistoryMeta;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_selection_intent(
    editor: &mut Editor,
    intent: SelectionIntent,
) -> Result<(), EditorError> {
    editor.transact(|tr| {
        tr.update_meta(|m| m.history = HistoryMeta::Skip);
        match intent {
            SelectionIntent::Set { selection } => {
                commands::set_selection(tr, selection)?;
            }
            SelectionIntent::All => {
                commands::select_all(tr)?;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::{Position, Selection};

    use super::*;

    #[test]
    fn select_set() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let target = Selection::collapsed(Position::new(t1, 3));
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Selection {
                intent: SelectionIntent::Set { selection: target },
            },
        });
        assert_eq!(editor.state().selection, target);
        assert!(!editor.history.can_undo());
    }
}
