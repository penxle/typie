use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

pub fn handle_doc_op(editor: &mut Editor, op: DocOp) -> Result<(), EditorError> {
    editor.transact(|tr| match op {
        DocOp::SetAttrs { attrs } => {
            tr.set_document_attrs(attrs)?;
            Ok(())
        }
    })
}
