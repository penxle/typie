use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_history_op(editor: &mut Editor, op: HistoryOp) -> Result<(), EditorError> {
    match op {
        HistoryOp::Undo => {
            editor.try_undo();
        }
        HistoryOp::Redo => {
            editor.try_redo();
        }
    }
    Ok(())
}
