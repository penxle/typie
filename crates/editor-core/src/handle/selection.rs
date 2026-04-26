use editor_commands::{self as commands};
use editor_state::{Position, ResolvedPosition, ResolvedPositionFlatExt, Selection};
use editor_transaction::HistoryMeta;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_selection_op(editor: &mut Editor, op: SelectionOp) -> Result<(), EditorError> {
    editor.view.clear_preferred_x();
    editor.transact(|tr| {
        tr.update_meta(|m| m.history = HistoryMeta::Skip);
        match op {
            SelectionOp::Set { selection } => {
                commands::set_selection(tr, selection)?;
            }
            SelectionOp::All => {
                commands::select_all(tr)?;
            }
            SelectionOp::SetFlat { start, end } => {
                let doc = tr.doc();
                let start_pos = match ResolvedPosition::from_flat(&doc, start) {
                    Some(p) => p,
                    None => return Ok(()),
                };
                let end_pos = match ResolvedPosition::from_flat(&doc, end) {
                    Some(p) => p,
                    None => return Ok(()),
                };
                commands::set_selection(
                    tr,
                    Selection::new(Position::from(&start_pos), Position::from(&end_pos)),
                )?;
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
        editor.apply(Message::Selection {
            op: SelectionOp::Set { selection: target },
        });
        assert_eq!(editor.state().selection, target);
        assert!(!editor.history.can_undo());
    }
}
