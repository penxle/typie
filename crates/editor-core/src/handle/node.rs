use editor_commands as commands;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_node_op(editor: &mut Editor, op: NodeOp) -> Result<(), EditorError> {
    editor.transact(|tr| match op {
        NodeOp::SetAttrs { id, attrs } => {
            tr.set_node(id, attrs)?;
            Ok(())
        }
        NodeOp::Delete { id } => {
            commands::delete_node(tr, id)?;
            Ok(())
        }
        NodeOp::Table { .. } => Ok(()),
    })
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::assert_state_eq;

    use super::*;

    #[test]
    fn delete_node_removes_selected_external_block_and_records_history() {
        let (initial, _root, _t1, img, ..) = state! {
            doc { r: root {
                paragraph { t1: text("Before") }
                img: image
                paragraph { t2: text("After") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = Editor::new_test(initial.clone());

        editor.apply(Message::Node {
            op: NodeOp::Delete { id: img },
        });

        let (deleted, ..) = state! {
            doc { root {
                paragraph { t1: text("Before") }
                paragraph { t2: text("After") }
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(editor.state(), &deleted);
        assert!(editor.history.can_undo());

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
        assert_state_eq!(editor.state(), &initial);

        editor.apply(Message::History {
            op: HistoryOp::Redo,
        });
        assert_state_eq!(editor.state(), &deleted);
    }
}
