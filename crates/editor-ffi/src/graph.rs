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

pub(crate) fn state_from_changesets(
    changesets: Vec<u8>,
) -> EditorResult<(editor_state::State, crate::editor::CarrierStash)> {
    let decoded = editor_codec::decode_changeset_stream(&changesets[..])
        .map_err(|e| FfiError::Deserialization(e.to_string()))?;
    let lossless = decoded.lossless();
    let css = decoded.into_graph_input();
    let mut stash = crate::editor::CarrierStash::default();
    crate::editor::stash_carriers(&css, &changesets, lossless, &mut stash)?;
    let state = build_state_tolerant(css)?;
    Ok((state, stash))
}

pub(crate) fn state_from_changesets_with_pending(
    server: Vec<u8>,
    pending: Vec<Vec<u8>>,
) -> EditorResult<(editor_state::State, crate::editor::CarrierStash)> {
    let mut stash = crate::editor::CarrierStash::default();
    let decoded = editor_codec::decode_changeset_stream(&server[..])
        .map_err(|e| FfiError::Deserialization(e.to_string()))?;
    let lossless = decoded.lossless();
    let mut all = decoded.into_graph_input();
    crate::editor::stash_carriers(&all, &server, lossless, &mut stash)?;
    for blob in pending {
        if blob.is_empty() {
            continue;
        }
        let decoded = editor_codec::decode_changeset_stream(&blob[..])
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let lossless = decoded.lossless();
        let mut css = decoded.into_graph_input();
        crate::editor::stash_carriers(&css, &blob, lossless, &mut stash)?;
        all.append(&mut css);
    }
    let state = build_state_tolerant(all)?;
    Ok((state, stash))
}

pub(crate) fn build_state_tolerant(
    css: Vec<editor_crdt::Changeset<editor_model::EditOp>>,
) -> EditorResult<editor_state::State> {
    if css.is_empty() {
        return Ok(editor_state::State::new(
            editor_state::ProjectedState::empty(),
            None,
        ));
    }
    let (graph, dropped) =
        editor_crdt::OpGraph::<editor_model::EditOp>::new().receive_changesets_ordered(css);
    if !dropped.is_empty() {
        let ids: Vec<String> = dropped
            .iter()
            .filter_map(|cs| cs.ops.first())
            .map(|op| format!("{}:{}", op.id.actor, op.id.clock))
            .collect();
        log::warn!(
            "state build dropped {} unapplied changeset(s) (missing parents or rejected): {}",
            dropped.len(),
            ids.join(", ")
        );
    }
    let projected =
        editor_state::ProjectedState::from_graph(graph).map_err(|e| EditorError::General {
            msg: format!("{e:?}"),
        })?;
    Ok(editor_state::State::new(projected, None))
}
