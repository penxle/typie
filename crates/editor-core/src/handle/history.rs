use editor_transaction::HistoryMeta;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_history_op(editor: &mut Editor, op: HistoryOp) -> Result<(), EditorError> {
    let steps = match op {
        HistoryOp::Undo => editor.history.undo(),
        HistoryOp::Redo => editor.history.redo(),
    };

    if let Some(steps) = steps {
        editor.transact(|tr| {
            tr.update_meta(|m| m.history = HistoryMeta::Skip);
            tr.apply_steps(steps.to_vec())?;
            Ok(())
        })?;
    }
    Ok(())
}
