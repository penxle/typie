use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_node_intent(_editor: &mut Editor, _intent: NodeIntent) -> Result<(), EditorError> {
    Ok(())
}
