use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_clipboard_op(_editor: &mut Editor, _op: ClipboardOp) -> Result<(), EditorError> {
    Ok(())
}
