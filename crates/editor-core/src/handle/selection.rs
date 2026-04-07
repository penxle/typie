use editor_commands::{self as commands};
use editor_schema::ResolvedPositionFlatExt;
use editor_state::{Position, ResolvedPosition, Selection};
use editor_transaction::HistoryMeta;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_selection_intent(
    editor: &mut Editor,
    intent: SelectionIntent,
) -> Result<(), EditorError> {
    editor.view.clear_preferred_x();
    editor.transact(|tr| {
        tr.update_meta(|m| m.history = HistoryMeta::Skip);
        match intent {
            SelectionIntent::Set { selection } => {
                commands::set_selection(tr, selection)?;
            }
            SelectionIntent::All => {
                commands::select_all(tr)?;
            }
            SelectionIntent::SetFlat { start, end } => {
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
        editor.apply(Message::Intent {
            intent: Intent::Selection {
                intent: SelectionIntent::Set { selection: target },
            },
        });
        assert_eq!(editor.state().selection, target);
        assert!(!editor.history.can_undo());
    }

    #[test]
    fn set_flat_valid_range() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::Intent {
            intent: Intent::Selection {
                intent: SelectionIntent::SetFlat { start: 1, end: 3 },
            },
        });
        // Selection anchor at flat 1, head at flat 3
        assert_eq!(editor.state().selection.anchor.node_id, t1);
        assert_eq!(editor.state().selection.anchor.offset, 1);
        assert_eq!(editor.state().selection.head.node_id, t1);
        assert_eq!(editor.state().selection.head.offset, 3);
    }

    #[test]
    fn set_flat_out_of_range_is_noop() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        let before = editor.state().selection;
        editor.apply(Message::Intent {
            intent: Intent::Selection {
                intent: SelectionIntent::SetFlat { start: 0, end: 100 },
            },
        });
        assert_eq!(editor.state().selection, before);
    }
}
