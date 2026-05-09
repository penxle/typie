use editor_crdt::Changeset;
use editor_model::DocOp;
use editor_transaction::StepError;

use crate::editor::Editor;
use crate::error::EditorError;

pub fn handle_remote(editor: &mut Editor, changeset: Changeset<DocOp>) -> Result<(), EditorError> {
    let (next, applied_ops) = editor
        .state
        .receive_remote_changeset(changeset)
        .map_err(|e| EditorError::Step(StepError::State(e)))?;
    editor.state = next;
    editor.pending_ops.extend(applied_ops);
    Ok(())
}
