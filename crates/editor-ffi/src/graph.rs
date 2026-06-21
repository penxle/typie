use crate::prelude::*;

pub(crate) fn doc_from_changesets(
    changesets: Vec<u8>,
) -> EditorResult<(editor_model::Doc, editor_crdt::OpGraph<editor_model::DocOp>)> {
    let css: Vec<editor_crdt::Changeset<editor_model::DocOp>> =
        editor_crdt::wire::decode(&changesets[..])
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
    let graph = editor_crdt::OpGraph::from_changesets(css)?;
    let doc = editor_model::Doc::from_op_graph(&graph)?;
    Ok((doc, graph))
}
