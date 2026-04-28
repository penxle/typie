use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_node_op(editor: &mut Editor, op: NodeOp) -> Result<(), EditorError> {
    editor.transact(|tr| match op {
        NodeOp::SetAttrs { id, attrs } => {
            tr.set_node(id, attrs)?;
            Ok(())
        }
        NodeOp::Delete { .. } | NodeOp::Table { .. } => Ok(()),
    })
}
