use editor_transaction::HistoryMeta;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_history_op(editor: &mut Editor, op: HistoryOp) -> Result<(), EditorError> {
    let (steps, is_redo) = match op {
        HistoryOp::Undo => (editor.history.undo(), false),
        HistoryOp::Redo => (editor.history.redo(), true),
    };

    if let Some(steps) = steps {
        editor.transact(|tr| {
            tr.update_meta(|m| m.history = HistoryMeta::Skip);
            tr.apply_steps(steps.to_vec())?;
            Ok(())
        })?;
        // Redo re-applies the original tagged entry. Restore last_tag so that
        // shortcut gestures (e.g. backspace after auto-replacement) still fire.
        if is_redo {
            editor.history.sync_last_tag_from_top();
        }
    }
    Ok(())
}
