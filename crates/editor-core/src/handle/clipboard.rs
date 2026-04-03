use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_clipboard_intent(
    _editor: &mut Editor,
    _intent: ClipboardIntent,
) -> Result<(), EditorError> {
    Ok(())
}
