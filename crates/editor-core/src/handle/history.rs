use editor_transaction::HistoryMeta;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_history_intent(
    editor: &mut Editor,
    intent: HistoryIntent,
) -> Result<(), EditorError> {
    let steps = match intent {
        HistoryIntent::Undo => editor.history.undo(),
        HistoryIntent::Redo => editor.history.redo(),
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
