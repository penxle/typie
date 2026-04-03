use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_composition_intent(
    _editor: &mut Editor,
    _intent: CompositionIntent,
) -> Result<(), EditorError> {
    Ok(())
}
