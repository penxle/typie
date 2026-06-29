use crate::prelude::*;

pub(crate) fn decode_length_prefixed(data: &[u8]) -> Result<Vec<Vec<u8>>, FfiError> {
    if data.is_empty() {
        return Ok(Vec::new());
    }
    if data.len() < 4 {
        return Err(FfiError::Deserialization("pending: truncated count".into()));
    }
    let count = u32::from_le_bytes(data[..4].try_into().unwrap()) as usize;
    let mut pos = 4;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if pos + 4 > data.len() {
            return Err(FfiError::Deserialization(
                "pending: truncated length".into(),
            ));
        }
        let len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        if pos + len > data.len() {
            return Err(FfiError::Deserialization("pending: truncated blob".into()));
        }
        out.push(data[pos..pos + len].to_vec());
        pos += len;
    }
    Ok(out)
}

pub(crate) fn state_from_changesets(changesets: Vec<u8>) -> EditorResult<editor_state::State> {
    let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
        editor_crdt::wire::decode(&changesets[..])
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
    Ok(editor_state::State::from_changesets(css, None)?)
}

pub(crate) fn state_from_changesets_with_pending(
    server: Vec<u8>,
    pending: Vec<Vec<u8>>,
) -> EditorResult<editor_state::State> {
    let mut all: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
        editor_crdt::wire::decode(&server[..])
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
    for blob in pending {
        if blob.is_empty() {
            continue;
        }
        let mut css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_crdt::wire::decode(&blob[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        all.append(&mut css);
    }
    if all.is_empty() {
        return Ok(editor_state::State::new(
            editor_state::ProjectedState::empty(),
            None,
        ));
    }
    let (graph, _dropped) =
        editor_crdt::OpGraph::<editor_model::EditOp>::new().receive_changesets_ordered(all);
    let projected =
        editor_state::ProjectedState::from_graph(graph).map_err(|e| EditorError::General {
            msg: format!("{e:?}"),
        })?;
    Ok(editor_state::State::new(projected, None))
}
