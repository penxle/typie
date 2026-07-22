use std::sync::{Arc, Mutex};

use crate::error::{EditorError, FfiError};
use crate::host::EditorHost;
use crate::prelude::*;

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct GraphIngest {
    resource: Arc<Mutex<editor_resource::Resource>>,
    buffer: Mutex<Option<Vec<u8>>>,
}

impl GraphIngest {
    fn take_buffer(&self) -> EditorResult<Vec<u8>> {
        let mut guard = self.buffer.lock().map_err(|_| FfiError::LockPoisoned)?;
        guard.take().ok_or_else(|| EditorError::General {
            msg: "graph ingest already finished".to_string(),
        })
    }

    fn build_editor(
        &self,
        state: editor_state::State,
        carrier_bytes: crate::editor::CarrierStash,
        viewport: editor_view::Viewport,
    ) -> EditorResult<Owned<crate::editor::Editor>> {
        let core = editor_core::Editor::new(state, viewport, Arc::clone(&self.resource));
        Ok(into_owned(crate::editor::Editor::new(core, carrier_bytes)))
    }
}

#[cfg_attr(feature = "uniffi", editor_macros::ffi_export(uniffi))]
impl GraphIngest {
    pub fn append_chunk(&self, data: Vec<u8>) -> EditorResult<()> {
        let mut guard = self.buffer.lock().map_err(|_| FfiError::LockPoisoned)?;
        match guard.as_mut() {
            Some(buffer) => {
                buffer.extend_from_slice(&data);
                Ok(())
            }
            None => Err(EditorError::General {
                msg: "graph ingest already finished".to_string(),
            }),
        }
    }

    pub fn total_bytes(&self) -> EditorResult<i64> {
        let guard = self.buffer.lock().map_err(|_| FfiError::LockPoisoned)?;
        match guard.as_ref() {
            Some(buffer) => Ok(buffer.len() as i64),
            None => Err(EditorError::General {
                msg: "graph ingest already finished".to_string(),
            }),
        }
    }

    pub fn abort(&self) -> EditorResult<()> {
        self.take_buffer()?;
        Ok(())
    }

    pub fn finish(
        &self,
        viewport: Complex<editor_view::Viewport>,
    ) -> EditorResult<Owned<crate::editor::Editor>> {
        let changesets = self.take_buffer()?;
        let (state, carrier_bytes) = crate::graph::state_from_changesets(changesets)?;
        let viewport = viewport.from_ffi()?;
        self.build_editor(state, carrier_bytes, viewport)
    }

    pub fn finish_with_pending(
        &self,
        pending_encoded: Vec<u8>,
        viewport: Complex<editor_view::Viewport>,
    ) -> EditorResult<Owned<crate::editor::Editor>> {
        let server = self.take_buffer()?;
        let pending = crate::graph::decode_length_prefixed(&pending_encoded)?;
        let (state, carrier_bytes) =
            crate::graph::state_from_changesets_with_pending(server, pending)?;
        let viewport = viewport.from_ffi()?;
        self.build_editor(state, carrier_bytes, viewport)
    }
}

#[cfg_attr(feature = "uniffi", editor_macros::ffi_export(uniffi))]
impl EditorHost {
    pub fn begin_graph_ingest(&self) -> EditorResult<Owned<GraphIngest>> {
        Ok(into_owned(GraphIngest {
            resource: Arc::clone(&self.resource),
            buffer: Mutex::new(Some(Vec::new())),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_host() -> EditorHost {
        EditorHost::new_test()
    }

    fn test_viewport() -> editor_view::Viewport {
        editor_view::Viewport::new(320.0, 640.0, 1.0)
    }

    #[test]
    fn append_accumulates_and_total_bytes_tracks() {
        let host = make_host();
        let ingest = host.begin_graph_ingest().unwrap();
        assert_eq!(ingest.total_bytes().unwrap(), 0);
        ingest.append_chunk(vec![1, 2, 3]).unwrap();
        ingest.append_chunk(vec![]).unwrap();
        ingest.append_chunk(vec![4]).unwrap();
        assert_eq!(ingest.total_bytes().unwrap(), 4);
    }

    #[test]
    fn abort_is_terminal() {
        let host = make_host();
        let ingest = host.begin_graph_ingest().unwrap();
        ingest.append_chunk(vec![9, 9, 9]).unwrap();
        ingest.abort().unwrap();
        assert!(ingest.append_chunk(vec![1]).is_err());
        assert!(ingest.total_bytes().is_err());
        assert!(ingest.finish(test_viewport()).is_err());
        assert!(ingest.abort().is_err());
    }

    #[test]
    fn finish_consumes_and_further_ops_error() {
        let host = make_host();
        let ingest = host.begin_graph_ingest().unwrap();
        let _editor = ingest.finish(test_viewport()).unwrap();
        let err = ingest.append_chunk(vec![1]).unwrap_err();
        assert!(format!("{err:?}").contains("already finished"));
        assert!(ingest.total_bytes().is_err());
        assert!(ingest.finish(test_viewport()).is_err());
    }

    #[test]
    fn empty_ingest_finish_builds_empty_editor() {
        let host = make_host();
        let ingest = host.begin_graph_ingest().unwrap();
        let editor = ingest.finish(test_viewport()).unwrap();
        assert_eq!(editor.prose_text().unwrap(), "");
    }

    fn assert_editor_equivalent(a: &crate::editor::Editor, b: &crate::editor::Editor) {
        assert_eq!(a.current_heads().unwrap(), b.current_heads().unwrap());
        assert_eq!(a.prose_text().unwrap(), b.prose_text().unwrap());
        assert_eq!(a.character_counts().unwrap(), b.character_counts().unwrap());
    }

    fn graph_from_state(state: &editor_state::State) -> Vec<u8> {
        editor_codec::encode_changesets(editor_codec::ReencodableChangesets::from_local_ops(
            state.graph().changesets_as_vec(),
        ))
        .unwrap()
    }

    fn default_graph() -> Vec<u8> {
        use editor_macros::state;

        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello ingest") } } }
            selection: none
        };
        graph_from_state(&state)
    }

    #[test]
    fn two_chunk_split_equivalent_to_full_load() {
        let host = make_host();
        let graph = default_graph();
        let full = host
            .create_editor_from_graph(graph.clone(), test_viewport())
            .unwrap();

        let mid = graph.len() / 2;
        let ingest = host.begin_graph_ingest().unwrap();
        ingest.append_chunk(graph[..mid].to_vec()).unwrap();
        ingest.append_chunk(graph[mid..].to_vec()).unwrap();
        let streamed = ingest.finish(test_viewport()).unwrap();

        assert_editor_equivalent(&streamed, &full);
    }

    #[test]
    fn odd_sized_chunks_equivalent_to_full_load() {
        let host = make_host();
        let graph = default_graph();
        let full = host
            .create_editor_from_graph(graph.clone(), test_viewport())
            .unwrap();

        let ingest = host.begin_graph_ingest().unwrap();
        for chunk in graph.chunks(13) {
            ingest.append_chunk(chunk.to_vec()).unwrap();
        }
        let streamed = ingest.finish(test_viewport()).unwrap();

        assert_editor_equivalent(&streamed, &full);
    }

    #[test]
    fn boundary_splits_equivalent_to_full_load() {
        let host = make_host();
        let graph = default_graph();
        let full = host
            .create_editor_from_graph(graph.clone(), test_viewport())
            .unwrap();

        for split in [0usize, 1, graph.len() - 1, graph.len()] {
            let ingest = host.begin_graph_ingest().unwrap();
            ingest.append_chunk(graph[..split].to_vec()).unwrap();
            ingest.append_chunk(graph[split..].to_vec()).unwrap();
            let streamed = ingest.finish(test_viewport()).unwrap();
            assert_editor_equivalent(&streamed, &full);
        }
    }

    #[test]
    fn abort_then_new_begin_equivalent_to_full_load() {
        let host = make_host();
        let graph = default_graph();
        let full = host
            .create_editor_from_graph(graph.clone(), test_viewport())
            .unwrap();

        let aborted = host.begin_graph_ingest().unwrap();
        aborted
            .append_chunk(graph[..graph.len() / 3].to_vec())
            .unwrap();
        aborted.abort().unwrap();

        let ingest = host.begin_graph_ingest().unwrap();
        ingest.append_chunk(graph.clone()).unwrap();
        let streamed = ingest.finish(test_viewport()).unwrap();

        assert_editor_equivalent(&streamed, &full);
    }

    #[test]
    fn finish_with_pending_uses_ingested_server_bytes() {
        let host = make_host();
        let graph = default_graph();
        let full = host
            .create_editor_from_graph(graph.clone(), test_viewport())
            .unwrap();

        let ingest = host.begin_graph_ingest().unwrap();
        ingest.append_chunk(graph.clone()).unwrap();
        let streamed = ingest
            .finish_with_pending(Vec::new(), test_viewport())
            .unwrap();

        assert_editor_equivalent(&streamed, &full);
    }

    #[test]
    fn finish_with_pending_equivalent_to_full_load_with_pending() {
        let host = make_host();
        let graph = default_graph();

        let mut pending_encoded = Vec::new();
        pending_encoded.extend_from_slice(&1u32.to_le_bytes());
        pending_encoded.extend_from_slice(&(graph.len() as u32).to_le_bytes());
        pending_encoded.extend_from_slice(&graph);

        let full = host
            .create_editor_from_graph_with_pending(
                Vec::new(),
                pending_encoded.clone(),
                test_viewport(),
            )
            .unwrap();

        let ingest = host.begin_graph_ingest().unwrap();
        let streamed = ingest
            .finish_with_pending(pending_encoded, test_viewport())
            .unwrap();

        assert_editor_equivalent(&streamed, &full);
    }
}
