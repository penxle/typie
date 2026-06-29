use crate::prelude::*;

pub(crate) fn state_from_changesets(changesets: Vec<u8>) -> EditorResult<editor_state::State> {
    let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
        editor_crdt::wire::decode(&changesets[..])
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
    Ok(editor_state::State::from_changesets(css, None)?)
}
