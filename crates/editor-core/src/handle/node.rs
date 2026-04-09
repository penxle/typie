use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_node_op(_editor: &mut Editor, _op: NodeOp) -> Result<(), EditorError> {
    Ok(())
}
