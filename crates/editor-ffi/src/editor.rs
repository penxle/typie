#[cfg(not(feature = "wasm-server"))]
use hashbrown::HashMap;
use std::sync::Mutex;

use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[cfg(not(feature = "wasm-server"))]
use crate::platform::{PlatformHandle, SurfaceHandle};
use crate::prelude::*;

pub(crate) type CarrierStash = hashbrown::HashMap<editor_crdt::Dot, Vec<u8>>;

pub(crate) fn stash_carriers(
    css: &[editor_crdt::Changeset<editor_model::EditOp>],
    payload: &[u8],
    lossless: bool,
    stash: &mut CarrierStash,
) -> Result<(), FfiError> {
    if lossless {
        return Ok(());
    }
    let parts = editor_codec::split_bundle_bytes(payload)
        .map_err(|e| FfiError::Deserialization(e.to_string()))?;
    debug_assert_eq!(css.len(), parts.len());
    for (cs, part) in css.iter().zip(parts) {
        if editor_codec::bundle_contains_unknown(&part)
            .map_err(|e| FfiError::Deserialization(e.to_string()))?
            && let Some(first) = cs.ops.first()
        {
            stash.insert(first.id, part);
        }
    }
    Ok(())
}

fn assemble_send_payload(
    css: Vec<editor_crdt::Changeset<editor_model::EditOp>>,
    stash: &CarrierStash,
) -> Result<(Vec<u8>, u32), FfiError> {
    let mut out = Vec::new();
    let mut run: Vec<editor_crdt::Changeset<editor_model::EditOp>> = Vec::new();
    let total = css.len();
    let mut emitted = 0usize;
    let flush = |run: &mut Vec<editor_crdt::Changeset<editor_model::EditOp>>,
                 out: &mut Vec<u8>|
     -> Result<(), FfiError> {
        if run.is_empty() {
            return Ok(());
        }
        let bytes = editor_codec::encode_changesets(
            editor_codec::ReencodableChangesets::from_local_ops(std::mem::take(run)),
        )
        .map_err(|e| FfiError::Serialization(e.to_string()))?;
        out.extend_from_slice(&bytes);
        Ok(())
    };
    for cs in css {
        if let Some(bytes) = cs.ops.first().and_then(|op| stash.get(&op.id)) {
            flush(&mut run, &mut out)?;
            out.extend_from_slice(bytes);
            emitted += 1;
        } else if editor_codec::changesets_contain_unknown(std::slice::from_ref(&cs)) {
            if let Some(op) = cs.ops.first() {
                log::error!(
                    "carrier changeset {}:{} missing from stash; truncating send window",
                    op.id.actor,
                    op.id.clock
                );
            }
            break;
        } else {
            run.push(cs);
            emitted += 1;
        }
    }
    flush(&mut run, &mut out)?;
    let withheld = (total - emitted) as u32;
    Ok((out, withheld))
}

struct EditorInner {
    editor: editor_core::Editor,
    carrier_bytes: CarrierStash,
}

// Raster/present state lives behind its own lock so a page rasterization never
// holds the editor lock; hosts that read or tick the editor concurrently with a
// render only contend for the brief display-list build. Lock order is render →
// editor (render_surface nests them in that direction); never lock render while
// holding the editor lock.
#[cfg(not(feature = "wasm-server"))]
struct RenderState {
    surfaces: HashMap<u32, SurfaceHandle>,
    // Last render signature presented for each page. `render_surface` skips
    // re-rasterizing a page whose signature is unchanged (the selection-drag hot
    // path, where only the page under the moving endpoint actually changes).
    // Cleared whenever the page's pixel buffer is (re)created: attach/detach/resize.
    last_render_sig: HashMap<u32, u64>,
    prev_dl: HashMap<u32, editor_renderer::display_list::DisplayList>,
    // Content height (device px) last painted per page. The surface backing is
    // fixed at the page's max height, so when content grows we force-damage the
    // newly revealed strip [prev, current) that no primitive diff would cover.
    last_content_height: HashMap<u32, i32>,
}

#[cfg(not(feature = "wasm-server"))]
impl RenderState {
    fn clear_page_present_state(&mut self, page: u32) {
        self.last_render_sig.remove(&page);
        self.prev_dl.remove(&page);
        self.last_content_height.remove(&page);
    }
}

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub struct Editor {
    inner: Mutex<EditorInner>,
    #[cfg(not(feature = "wasm-server"))]
    render: Mutex<RenderState>,
}

#[ffi]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CharacterCounts {
    pub doc_with_whitespace: u32,
    pub doc_without_whitespace: u32,
    pub doc_without_whitespace_and_punctuation: u32,
    pub selection_with_whitespace: u32,
    pub selection_without_whitespace: u32,
    pub selection_without_whitespace_and_punctuation: u32,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TrackedRange {
    pub id: String,
    pub group: String,
    pub anchor: editor_state::Position,
    pub head: editor_state::Position,
    pub metadata: String,
    pub rects: Vec<editor_view::PageRect>,
    pub text: String,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TrackedRangeEndpoints {
    pub id: String,
    pub group: String,
    pub anchor: editor_state::Position,
    pub head: editor_state::Position,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TrackedRangeHit {
    pub id: String,
    pub group: String,
    pub rects: Vec<editor_view::PageRect>,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PartitionedChangesets {
    #[serde(with = "serde_bytes")]
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array"))]
    pub ready: Vec<u8>,
    #[serde(with = "serde_bytes")]
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array"))]
    pub blocked: Vec<u8>,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ChangesetEntry {
    pub id: String,
    #[serde(with = "serde_bytes")]
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array"))]
    pub bytes: Vec<u8>,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MissingChangesets {
    #[serde(with = "serde_bytes")]
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array"))]
    pub bytes: Vec<u8>,
    pub withheld: u32,
}

fn public_tracked_range(
    editor: &editor_core::Editor,
    range: &editor_core::TrackedRange,
) -> Option<TrackedRange> {
    let state = editor.state();
    let view = state.view();
    let sel = range.locate(state)?;
    let resolved = sel.resolve(&view)?;
    let rects = editor
        .view()
        .selection_rects(&resolved)
        .into_iter()
        .map(|sr| sr.without_meta())
        .collect();
    let text = resolved.collect_text();
    Some(TrackedRange {
        id: range.id.clone(),
        group: range.group.clone(),
        anchor: sel.anchor,
        head: sel.head,
        metadata: range.metadata.clone(),
        rects,
        text,
    })
}

fn public_tracked_range_endpoints(
    state: &editor_state::State,
    range: &editor_core::TrackedRange,
) -> Option<TrackedRangeEndpoints> {
    let sel = range.locate(state)?;
    Some(TrackedRangeEndpoints {
        id: range.id.clone(),
        group: range.group.clone(),
        anchor: sel.anchor,
        head: sel.head,
    })
}

#[cfg_attr(feature = "uniffi", editor_macros::ffi_export(uniffi))]
#[cfg_attr(feature = "wasm", editor_macros::ffi_export(wasm))]
impl Editor {
    pub fn enqueue(&self, message: Complex<editor_core::Message>) -> EditorResult<()> {
        self.with_inner(|inner| {
            inner.editor.enqueue(message.from_ffi()?);
            Ok(())
        })
    }

    pub fn last_history_tag(&self) -> EditorResult<Option<Complex<editor_common::HistoryTag>>> {
        self.with_inner(|inner| Ok(inner.editor.last_history_tag().into_ffi()?))
    }

    pub fn tick(&self) -> EditorResult<Vec<Complex<editor_core::EditorEvent>>> {
        self.with_inner(|inner| Ok(inner.editor.tick()?.into_ffi()?))
    }

    pub fn cursor(&self) -> EditorResult<Option<Complex<editor_view::CursorMetrics>>> {
        self.with_inner(|inner| {
            let state = inner.editor.state();
            let Some(selection) = state.selection.as_ref() else {
                return Ok(None);
            };
            if selection.is_collapsed() {
                Ok(inner
                    .editor
                    .view()
                    .cursor_metrics(state, &selection.head)
                    .into_ffi()?)
            } else {
                Ok(None)
            }
        })
    }

    pub fn placeholder(&self) -> EditorResult<Option<Complex<editor_view::PlaceholderMetrics>>> {
        self.with_inner(|inner| {
            let state = inner.editor.state();
            Ok(inner.editor.view().placeholder_metrics(state).into_ffi()?)
        })
    }

    pub fn selection(&self) -> EditorResult<Option<Complex<editor_state::Selection>>> {
        self.with_inner(|inner| Ok(inner.editor.state().selection.into_ffi()?))
    }

    pub fn copy_selection(
        &self,
    ) -> EditorResult<Option<Complex<editor_clipboard::ClipboardPayload>>> {
        self.with_inner(|inner| {
            let payload = editor_clipboard::Slice::extract(inner.editor.state()).map(|slice| {
                let resource = inner.editor.resource().lock().unwrap();
                slice.to_payload(&resource)
            });
            Ok(payload.into_ffi()?)
        })
    }

    pub fn root_attrs(&self) -> EditorResult<Complex<editor_model::PlainRootNode>> {
        self.with_inner(|inner| {
            let view = inner.editor.state().view();
            let root = crate::root::attrs(&view).expect("root entry must exist");
            Ok(root.into_ffi()?)
        })
    }

    pub fn root_modifiers(&self) -> EditorResult<Vec<Complex<editor_model::Modifier>>> {
        self.with_inner(|inner| {
            Ok(crate::root::root_default_modifiers(inner.editor.state()).into_ffi()?)
        })
    }

    pub fn modifier_state(&self) -> EditorResult<Option<Complex<editor_model::ModifierState>>> {
        self.with_inner(|inner| Ok(inner.editor.modifier_state().into_ffi()?))
    }

    pub fn modifier_span_selection(
        &self,
        pos: Complex<editor_state::Position>,
        modifier_type: Complex<editor_model::ModifierType>,
    ) -> EditorResult<Option<Complex<editor_state::Selection>>> {
        self.with_inner(|inner| {
            let pos: editor_state::Position = pos.from_ffi()?;
            let modifier_type: editor_model::ModifierType = modifier_type.from_ffi()?;
            Ok(inner
                .editor
                .modifier_span_selection(&pos, modifier_type)
                .into_ffi()?)
        })
    }

    pub fn block_state(&self) -> EditorResult<Option<Complex<editor_core::BlockState>>> {
        self.with_inner(|inner| Ok(inner.editor.block_state().into_ffi()?))
    }

    pub fn character_counts(&self) -> EditorResult<Complex<CharacterCounts>> {
        self.with_inner(|inner| {
            let (doc, sel) = inner.editor.character_counts();
            Ok(CharacterCounts {
                doc_with_whitespace: doc.with_whitespace,
                doc_without_whitespace: doc.without_whitespace,
                doc_without_whitespace_and_punctuation: doc.without_whitespace_and_punctuation,
                selection_with_whitespace: sel.with_whitespace,
                selection_without_whitespace: sel.without_whitespace,
                selection_without_whitespace_and_punctuation: sel
                    .without_whitespace_and_punctuation,
            }
            .into_ffi()?)
        })
    }

    pub fn interactive_hit_test(
        &self,
        page: u32,
        x: f32,
        y: f32,
    ) -> EditorResult<Option<Complex<editor_view::InteractiveHit>>> {
        self.with_inner(|inner| {
            Ok(inner
                .editor
                .interactive_hit_test(page as usize, x, y)
                .into_ffi()?)
        })
    }

    pub fn page_link_rects(&self, page: u32) -> EditorResult<Vec<Complex<editor_view::LinkRect>>> {
        self.with_inner(|inner| Ok(inner.editor.page_link_rects(page as usize).into_ffi()?))
    }

    pub fn link_rects(&self) -> EditorResult<Vec<Complex<editor_view::LinkRect>>> {
        self.with_inner(|inner| Ok(inner.editor.link_rects().into_ffi()?))
    }

    pub fn link_hit_test(
        &self,
        page: u32,
        x: f32,
        y: f32,
    ) -> EditorResult<Option<Complex<editor_view::LinkRect>>> {
        self.with_inner(|inner| Ok(inner.editor.link_hit_test(page as usize, x, y).into_ffi()?))
    }

    pub fn selection_endpoints(
        &self,
    ) -> EditorResult<Option<Complex<editor_view::SelectionEndpoints>>> {
        self.with_inner(|inner| Ok(inner.editor.selection_endpoints().into_ffi()?))
    }

    pub fn selection_hit_test(&self, page: u32, x: f32, y: f32) -> EditorResult<bool> {
        self.with_inner(|inner| Ok(inner.editor.selection_hit_test(page as usize, x, y)))
    }

    pub fn selection_hit_rects(&self) -> EditorResult<Vec<Complex<editor_view::PageRect>>> {
        self.with_inner(|inner| Ok(inner.editor.selection_hit_rects().into_ffi()?))
    }

    pub fn cursor_hit_test(&self, page: u32, x: f32, y: f32) -> EditorResult<bool> {
        self.with_inner(|inner| Ok(inner.editor.cursor_hit_test(page as usize, x, y)))
    }

    pub fn cursor_hit_rects(&self) -> EditorResult<Vec<Complex<editor_view::PageRect>>> {
        self.with_inner(|inner| Ok(inner.editor.cursor_hit_rects().into_ffi()?))
    }

    pub fn interactive_regions(
        &self,
    ) -> EditorResult<Vec<Complex<editor_view::InteractiveRegion>>> {
        self.with_inner(|inner| Ok(inner.editor.interactive_regions().into_ffi()?))
    }

    pub fn pointer_style(
        &self,
        page: u32,
        x: f32,
        y: f32,
        read_only: bool,
    ) -> EditorResult<Complex<editor_view::PointerStyle>> {
        self.with_inner(|inner| {
            Ok(inner
                .editor
                .pointer_style(page as usize, x, y, read_only)
                .unwrap_or(editor_view::PointerStyle::Default)
                .into_ffi()?)
        })
    }

    pub fn inspect_state(
        &self,
        options: Option<Complex<editor_introspection::InspectStateOptions>>,
    ) -> EditorResult<String> {
        self.with_inner(|inner| {
            let options = match options {
                Some(o) => o.from_ffi()?,
                None => editor_introspection::InspectStateOptions::default(),
            };
            Ok(editor_introspection::inspect_state(
                inner.editor.state(),
                &options,
            ))
        })
    }

    pub fn inspect_state_as_macro(&self) -> EditorResult<String> {
        self.with_inner(|inner| {
            Ok(editor_introspection::inspect_state_as_macro(
                inner.editor.state(),
            ))
        })
    }

    pub fn page_sizes(&self) -> EditorResult<Vec<Complex<editor_common::Size>>> {
        self.with_inner(|inner| {
            Ok(inner
                .editor
                .view()
                .pages()
                .iter()
                .map(|p| p.size)
                .collect::<Vec<_>>()
                .into_ffi()?)
        })
    }

    /// Fixed per-page backing sizes for the incremental renderer. Surfaces are
    /// allocated at this size (>= any content height) so content-height changes
    /// never resize (and clear) the canvas. See `View::page_backing_sizes`.
    pub fn page_backing_sizes(&self) -> EditorResult<Vec<Complex<editor_common::Size>>> {
        self.with_inner(|inner| Ok(inner.editor.view().page_backing_sizes().into_ffi()?))
    }

    pub fn external_elements(&self) -> EditorResult<Vec<Complex<editor_view::ExternalElement>>> {
        self.with_inner(|inner| {
            let state = inner.editor.state();
            let view = state.view();
            let resolved = state.selection.as_ref().and_then(|s| s.resolve(&view));
            Ok(inner
                .editor
                .view()
                .external_elements(state, resolved.as_ref())
                .into_ffi()?)
        })
    }

    pub fn page_external_elements(
        &self,
        page: u32,
    ) -> EditorResult<Vec<Complex<editor_view::ExternalElement>>> {
        self.with_inner(|inner| {
            let state = inner.editor.state();
            let view = state.view();
            let resolved = state.selection.as_ref().and_then(|s| s.resolve(&view));
            Ok(inner
                .editor
                .view()
                .page_external_elements(state, page as usize, resolved.as_ref())
                .into_ffi()?)
        })
    }

    pub fn table_overlays(&self) -> EditorResult<Vec<Complex<editor_view::TableOverlay>>> {
        self.with_inner(|inner| {
            let state = inner.editor.state();
            let view = state.view();
            let resolved = state.selection.as_ref().and_then(|s| s.resolve(&view));
            Ok(inner
                .editor
                .view()
                .table_overlays(state, resolved.as_ref())
                .into_ffi()?)
        })
    }

    pub fn page_table_overlays(
        &self,
        page: u32,
    ) -> EditorResult<Vec<Complex<editor_view::TableOverlay>>> {
        self.with_inner(|inner| {
            let state = inner.editor.state();
            let view = state.view();
            let resolved = state.selection.as_ref().and_then(|s| s.resolve(&view));
            Ok(inner
                .editor
                .view()
                .page_table_overlays(state, page as usize, resolved.as_ref())
                .into_ffi()?)
        })
    }

    pub fn ime(
        &self,
        before_limit: usize,
        after_limit: usize,
    ) -> EditorResult<Option<Complex<editor_core::Ime>>> {
        self.with_inner(|inner| Ok(inner.editor.ime(before_limit, after_limit)?.into_ffi()?))
    }

    pub fn receive_remote_changeset(&self, payload: Vec<u8>) -> EditorResult<()> {
        self.with_inner(|inner| {
            let decoded = editor_codec::decode_changeset_stream(&payload[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
            let lossless = decoded.lossless();
            let css = decoded.into_graph_input();
            stash_carriers(&css, &payload, lossless, &mut inner.carrier_bytes)?;
            for changeset in css {
                inner.editor.receive_remote_changeset(changeset);
            }
            Ok(())
        })
    }

    pub fn local_changesets_since(&self, remote_heads_payload: Vec<u8>) -> EditorResult<Vec<u8>> {
        self.with_inner(|inner| {
            let heads_vec = editor_codec::decode_dots(&remote_heads_payload[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
            let heads_set: hashbrown::HashSet<editor_crdt::Dot> = heads_vec.into_iter().collect();
            let css = inner.editor.local_changesets_since(&heads_set)?;
            if css.is_empty() {
                return Ok(Vec::new());
            }
            let bytes = editor_codec::encode_changesets(
                editor_codec::ReencodableChangesets::from_local_ops(css),
            )
            .map_err(|e| FfiError::Serialization(e.to_string()))?;
            Ok(bytes)
        })
    }

    pub fn missing_changesets_tolerant(
        &self,
        remote_heads_payload: Vec<u8>,
    ) -> EditorResult<Complex<MissingChangesets>> {
        self.with_inner(|inner| {
            let heads_vec = editor_codec::decode_dots(&remote_heads_payload[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
            let heads_set: hashbrown::HashSet<editor_crdt::Dot> = heads_vec.into_iter().collect();
            let css = inner.editor.missing_changesets_tolerant(&heads_set);
            let (bytes, withheld) = assemble_send_payload(css, &inner.carrier_bytes)?;
            Ok(MissingChangesets { bytes, withheld }.into_ffi()?)
        })
    }

    pub fn partition_remote_changesets(
        &self,
        payload: Vec<u8>,
    ) -> EditorResult<Complex<PartitionedChangesets>> {
        self.with_inner(|inner| {
            // Classification-only decode (readiness needs op id/parents); the byte
            // split below is what actually produces `ready`/`blocked` payloads, so
            // remote (possibly v-next) bytes are never value-reencoded.
            let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
                editor_codec::decode_changeset_stream(&payload[..])
                    .map_err(|e| FfiError::Deserialization(e.to_string()))?
                    .into_graph_input();
            let parts = editor_codec::split_bundle_bytes(&payload)
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
            debug_assert_eq!(css.len(), parts.len());
            let (ready_idx, blocked_idx) = inner.editor.partition_ready_indices(&css);
            let concat =
                |idx: &[usize]| -> Vec<u8> { idx.iter().flat_map(|&i| parts[i].clone()).collect() };
            Ok(PartitionedChangesets {
                ready: concat(&ready_idx),
                blocked: concat(&blocked_idx),
            }
            .into_ffi()?)
        })
    }

    pub fn split_changesets(&self, payload: Vec<u8>) -> EditorResult<Vec<Complex<ChangesetEntry>>> {
        // Classification-only decode for the `id` key; `parts[i]` (from the
        // byte-level split, no value reencode) is the entry's `bytes` directly.
        let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&payload[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        let parts = editor_codec::split_bundle_bytes(&payload)
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        debug_assert_eq!(css.len(), parts.len());
        let mut out = Vec::with_capacity(css.len());
        for (cs, bytes) in css.into_iter().zip(parts) {
            let first = cs
                .ops
                .first()
                .ok_or(FfiError::Deserialization("empty changeset".into()))?;
            let id = format!("{}:{}", first.id.actor, first.id.clock);
            out.push(ChangesetEntry { id, bytes }.into_ffi()?);
        }
        Ok(out)
    }

    pub fn current_heads(&self) -> EditorResult<Vec<u8>> {
        self.with_inner(|inner| {
            let heads = inner.editor.current_heads();
            if heads.is_empty() {
                return Ok(Vec::new());
            }
            let bytes = editor_codec::encode_dots(&heads)
                .map_err(|e| FfiError::Serialization(e.to_string()))?;
            Ok(bytes)
        })
    }

    /// The `id` of every local changeset (its first op's `actor:clock`), read straight
    /// from the graph — `O(#changesets)`. Callers that only need the id set must use
    /// this instead of `missing_changesets_tolerant(&[])` + `split_changesets`, which
    /// walk, clone, and re-encode the entire history on every push cycle.
    pub fn changeset_ids(&self) -> EditorResult<Vec<String>> {
        self.with_inner(|inner| {
            let ids = inner
                .editor
                .state()
                .graph()
                .changesets()
                .iter()
                .filter_map(|cs| cs.first().map(|d| format!("{}:{}", d.actor, d.clock)))
                .collect();
            Ok(ids)
        })
    }

    pub fn set_doc(&self, plain: Complex<editor_model::PlainDoc>) -> EditorResult<()> {
        self.with_inner(|inner| {
            inner.editor.set_doc(plain.from_ffi()?);
            Ok(())
        })
    }

    pub fn insert_template_fragment(&self, changesets: Vec<u8>) -> EditorResult<()> {
        self.with_inner(|inner| {
            // 빈 bytes: bundle_stream_contains_unknown([]) == Ok(false) — 무해 통과
            if editor_codec::bundle_stream_contains_unknown(&changesets)
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
            {
                return Err(FfiError::Deserialization(
                    "template contains ops newer than this reader".to_owned(),
                )
                .into());
            }
            let css = editor_codec::decode_changeset_stream(&changesets)
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_reencodable()
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
            let template = editor_state::State::from_changesets(css.as_slice().to_vec(), None)?;
            inner.editor.insert_template_fragment(template.to_plain())?;
            Ok(())
        })
    }

    pub fn materialize_at(
        &self,
        heads: Vec<u8>,
        sweep_tombstones: Vec<String>,
    ) -> EditorResult<Complex<editor_model::PlainDoc>> {
        self.with_inner(|inner| {
            let heads_vec = editor_codec::decode_dots(&heads[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
            let heads_set: hashbrown::HashSet<editor_crdt::Dot> = heads_vec.into_iter().collect();
            let state = inner.editor.state();
            let graph = state.graph();
            if let Some(missing) = heads_set.iter().find(|d| !graph.contains(d)) {
                return Err(EditorError::General {
                    msg: format!("unknown head: {missing:?}"),
                });
            }
            let ancestry = graph.ancestry_of(&heads_set);
            let ops = graph.topo_sort(&ancestry);
            let subgraph =
                editor_crdt::OpGraph::from_changesets(vec![editor_crdt::Changeset { ops }])?;
            let overlay = crate::graph::parse_sweep_tombstones(&sweep_tombstones);
            let projected = editor_state::ProjectedState::from_graph_with_overlay(
                subgraph, &overlay,
            )
            .map_err(|e| EditorError::General {
                msg: format!("materialize projection failed: {e:?}"),
            })?;
            if projected.projected().repair_stats.projection_degraded {
                return Err(EditorError::General {
                    msg: "materialize projection is degraded".to_string(),
                });
            }
            Ok(editor_state::to_plain(projected.projected()).into_ffi()?)
        })
    }

    pub fn freeze_selection(
        &self,
        selection: Complex<editor_state::Selection>,
    ) -> EditorResult<Option<Complex<editor_state::StableSelection>>> {
        self.with_inner(|inner| {
            let sel: editor_state::Selection = selection.from_ffi()?;
            let view = inner.editor.state().view();
            if !position_is_addressable(&sel.anchor, &view)
                || !position_is_addressable(&sel.head, &view)
            {
                return Ok(None);
            }
            Ok(Some(
                editor_state::StableSelection::capture(&sel, &view).into_ffi()?,
            ))
        })
    }

    pub fn find_matches(
        &self,
        query: String,
        options: Option<Complex<editor_core::SearchOptions>>,
    ) -> EditorResult<Vec<Complex<editor_state::Selection>>> {
        self.with_inner(|inner| {
            let opts = match options {
                Some(o) => o.from_ffi()?,
                None => editor_core::SearchOptions::default(),
            };
            Ok(inner.editor.find_matches(&query, &opts).into_ffi()?)
        })
    }

    pub fn tracked_ranges(
        &self,
        group: Option<String>,
    ) -> EditorResult<Vec<Complex<TrackedRange>>> {
        self.with_inner(|inner| {
            let registry = inner.editor.tracked_ranges();
            let ranges: Box<dyn Iterator<Item = &editor_core::TrackedRange>> = match &group {
                Some(g) => Box::new(registry.iter_group(g)),
                None => Box::new(registry.iter()),
            };
            let result: Vec<TrackedRange> = ranges
                .filter_map(|r| public_tracked_range(&inner.editor, r))
                .collect();
            Ok(result.into_ffi()?)
        })
    }

    pub fn tracked_ranges_containing_position(
        &self,
        position: Complex<editor_state::Position>,
        group: Option<String>,
    ) -> EditorResult<Vec<Complex<TrackedRangeEndpoints>>> {
        self.with_inner(|inner| {
            let position = position.from_ffi()?;
            let state = inner.editor.state();
            let ranges = inner
                .editor
                .tracked_ranges_containing_position(position, group.as_deref());
            let result: Vec<TrackedRangeEndpoints> = ranges
                .iter()
                .filter_map(|r| public_tracked_range_endpoints(state, r))
                .collect();
            Ok(result.into_ffi()?)
        })
    }

    pub fn export_page_vector(&self, page: u32, scale_factor: f64) -> EditorResult<Vec<u8>> {
        self.with_inner(|inner| Ok(inner.editor.export_page_vector(page, scale_factor as f32)))
    }

    pub fn tracked_ranges_at(
        &self,
        page: u32,
        x: f32,
        y: f32,
        group: Option<String>,
    ) -> EditorResult<Vec<Complex<TrackedRangeHit>>> {
        self.with_inner(|inner| {
            let hits = inner
                .editor
                .tracked_ranges_at(page as usize, x, y, group.as_deref());
            let public: Vec<TrackedRangeHit> = hits
                .into_iter()
                .map(|h| TrackedRangeHit {
                    id: h.id,
                    group: h.group,
                    rects: h.rects,
                })
                .collect();
            Ok(public.into_ffi()?)
        })
    }

    pub fn prose_text(&self) -> EditorResult<String> {
        self.with_inner(|inner| {
            let view = inner.editor.state().view();
            Ok(editor_state::prose(&view).text().to_string())
        })
    }

    pub fn prose_to_selection(
        &self,
        start: u32,
        end: u32,
    ) -> EditorResult<Option<Complex<editor_state::Selection>>> {
        self.with_inner(|inner| {
            let view = inner.editor.state().view();
            let prose = editor_state::prose(&view);
            Ok(prose
                .to_selection(&view, (start as usize)..(end as usize))
                .into_ffi()?)
        })
    }
}

#[cfg(not(feature = "wasm-server"))]
#[cfg_attr(feature = "uniffi", editor_macros::ffi_export(uniffi))]
#[cfg_attr(feature = "wasm-browser", editor_macros::ffi_export(wasm))]
impl Editor {
    pub fn attach_surface(
        &self,
        page: u32,
        handle: PlatformHandle,
        width: f64,
        height: f64,
        scale_factor: f64,
    ) -> EditorResult<()> {
        self.with_render(|render| {
            // Drop the old handle before the budget check in `SurfaceHandle::new`.
            render.surfaces.remove(&page);
            let surface = SurfaceHandle::new(handle, width, height, scale_factor)?;
            render.surfaces.insert(page, surface);
            // Fresh buffer: drop any stale signature/display-list so the first paint always renders.
            render.clear_page_present_state(page);
            Ok(())
        })
    }

    pub fn detach_surface(&self, page: u32) -> EditorResult<()> {
        self.with_render(|render| {
            render.surfaces.remove(&page);
            render.clear_page_present_state(page);
            Ok(())
        })
    }

    pub fn invalidate_surface(&self, page: u32) -> EditorResult<()> {
        self.with_render(|render| {
            render.clear_page_present_state(page);
            Ok(())
        })
    }

    pub fn resize_surface(
        &self,
        page: u32,
        width: f64,
        height: f64,
        scale_factor: f64,
    ) -> EditorResult<()> {
        self.with_render(|render| {
            let changed = render
                .surfaces
                .get_mut(&page)
                .map(|surface| surface.resize(width, height, scale_factor))
                .unwrap_or(false);
            // Only when the backing dims actually changed was the buffer recreated
            // (and cleared) — its old signature/display-list no longer reflects
            // pixels. A no-op resize (same backing size) keeps the retained state so
            // a content-height change never forces a full repaint.
            if changed {
                render.clear_page_present_state(page);
            }
            Ok(())
        })
    }

    /// Returns whether a new frame was presented. `false` means no frame will arrive
    /// from this call — the page's pixels already match the current state — so hosts
    /// that wait for a present (the mobile settle handshake) must treat the page as
    /// settled instead of waiting.
    pub fn render_surface(&self, page: u32) -> EditorResult<bool> {
        self.with_render(|render| {
            let scale_factor = match render.surfaces.get(&page) {
                Some(surface) => surface.scale_factor() as f32,
                None => return Ok(false),
            };
            // The editor lock is held only for the signature check and the
            // display-list build; the raster + present below run with the editor
            // free for concurrent ticks and reads.
            let (sig, built) = {
                let mut inner = self.inner.lock().map_err(|_| FfiError::LockPoisoned)?;
                // Skip the rebuild + incremental raster + present when nothing this page
                // draws has changed since the last presented frame. During a selection
                // drag only the page under the moving endpoint changes; every other
                // visible page keeps a stable signature and is skipped here.
                let sig = inner.editor.page_render_signature(page);
                if render.last_render_sig.get(&page) == Some(&sig) {
                    return Ok(false);
                }
                (sig, inner.editor.build_display_list(page, scale_factor))
            };
            // The page may have been removed by an edit this frame (the host's
            // surfaces outlive a shrinking document until it re-renders), in
            // which case no frame will arrive from this call.
            let Some((dl, page_bounds)) = built else {
                return Ok(false);
            };
            let prev_content_h = render.last_content_height.get(&page).copied();
            let prev = render.prev_dl.get(&page);
            let surface = render.surfaces.get_mut(&page).unwrap();
            let mut damage = match prev {
                None => vec![page_bounds],
                Some(prev) => editor_renderer::diff::diff(prev, &dl, page_bounds),
            };
            // The surface backing is fixed at the page's max height, so a taller
            // content region reveals rows that were outside the previous page
            // bounds (or hold stale pixels from an earlier shrink). No primitive
            // diff covers those gaps, so paint the revealed strip explicitly.
            if let Some(prev_h) = prev_content_h {
                let strip = editor_renderer::damage::IRect {
                    x0: 0,
                    y0: prev_h.max(0),
                    x1: page_bounds.x1,
                    y1: page_bounds.y1,
                };
                if !strip.is_empty() {
                    damage.push(strip);
                }
            }
            let committed = surface.apply_damage(&dl, &damage);
            if committed {
                render.prev_dl.insert(page, dl);
                render.last_render_sig.insert(page, sig);
                render.last_content_height.insert(page, page_bounds.y1);
                Ok(true)
            } else {
                render.clear_page_present_state(page);
                Ok(false)
            }
        })
    }
}

// Separate from the block above: `surface_backend`/`refresh_surface` are browser
// present-path specific (the `cpu-oversized` report, the visibility-return recovery),
// so they stay in a wasm-browser-only block rather than the shared uniffi/wasm block
// above — a `#[cfg]` on an individual method inside a shared uniffi/wasm block isn't
// honored by uniffi's own codegen.
#[cfg(feature = "wasm-browser")]
#[editor_macros::ffi_export(wasm)]
impl Editor {
    pub fn surface_backend(&self, page: u32) -> EditorResult<String> {
        self.with_render(|render| {
            Ok(match render.surfaces.get(&page) {
                None => "none".to_string(),
                Some(surface) if surface.is_oversized() => "cpu-oversized".to_string(),
                Some(_) => "cpu".to_string(),
            })
        })
    }

    /// Visibility-return recovery. CPU keeps the invalidate semantics (clear → full
    /// re-render on the next `render_surface`).
    pub fn refresh_surface(&self, page: u32) -> EditorResult<()> {
        self.with_render(|render| {
            if render.surfaces.contains_key(&page) {
                render.clear_page_present_state(page);
            }
            Ok(())
        })
    }
}

impl Editor {
    pub(crate) fn new(core: editor_core::Editor, carrier_bytes: CarrierStash) -> Self {
        Self {
            inner: Mutex::new(EditorInner {
                editor: core,
                carrier_bytes,
            }),
            #[cfg(not(feature = "wasm-server"))]
            render: Mutex::new(RenderState {
                surfaces: HashMap::new(),
                last_render_sig: HashMap::new(),
                prev_dl: HashMap::new(),
                last_content_height: HashMap::new(),
            }),
        }
    }

    fn with_inner<F, R>(&self, f: F) -> EditorResult<R>
    where
        F: FnOnce(&mut EditorInner) -> EditorResult<R>,
    {
        let mut inner = self.inner.lock().map_err(|_| FfiError::LockPoisoned)?;
        f(&mut inner)
    }

    #[cfg(not(feature = "wasm-server"))]
    fn with_render<F, R>(&self, f: F) -> EditorResult<R>
    where
        F: FnOnce(&mut RenderState) -> EditorResult<R>,
    {
        let mut render = self.render.lock().map_err(|_| FfiError::LockPoisoned)?;
        f(&mut render)
    }
}

fn position_is_addressable(pos: &editor_state::Position, view: &editor_model::DocView) -> bool {
    pos.resolve(view).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;

    fn make_ffi_editor(initial: editor_state::State) -> Editor {
        let mut core = editor_core::Editor::new_test(initial);
        core.apply(editor_core::Message::System {
            event: editor_core::SystemEvent::Initialize,
        });
        Editor::new(core, CarrierStash::default())
    }

    #[test]
    fn materialize_at_refuses_a_degraded_projection() {
        use editor_crdt::{Changeset, Dot, ListOp, Op};
        use editor_model::{EditOp, NodeType, SeqItem};

        // A flat `TableCell` under Root needs several schema repairs — enough to
        // latch the cap once the budget is lowered to 1.
        let css = vec![Changeset {
            ops: vec![
                Op {
                    id: Dot::new(1, 0),
                    parents: vec![],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 0,
                        item: SeqItem::Block {
                            node_type: NodeType::TableCell,
                            parents: vec![Dot::ROOT],
                            attrs: vec![],
                        },
                    }),
                },
                Op {
                    id: Dot::new(1, 1),
                    parents: vec![Dot::new(1, 0)],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 1,
                        item: SeqItem::Char('a'),
                    }),
                },
            ],
        }];

        let state = editor_state::State::from_changesets(css, None).unwrap();
        let heads: Vec<Dot> = state.graph().current_heads().copied().collect();
        let heads_bytes = editor_codec::encode_dots(&heads).unwrap();
        let editor = make_ffi_editor(state);

        let result = {
            let _guard = editor_model::override_repair_budget(1);
            editor.materialize_at(heads_bytes, Vec::new())
        };
        assert!(
            result.is_err(),
            "materialize_at must surface a degraded projection as an error"
        );
    }

    #[test]
    fn ffi_selection_endpoints_resolves_and_forwards() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 1) -> (p1, 8)
        };
        let editor = make_ffi_editor(initial);
        let result = editor.selection_endpoints().expect("ffi call returns Ok");
        assert!(
            result.is_some(),
            "range selection must produce endpoints through FFI",
        );
    }

    #[test]
    fn ffi_selection_endpoints_exposes_external_only_image_selection() {
        let (initial, root) = state! {
            doc { root: root { image } }
            selection: (root, 0, >) -> (root, 1, <)
        };
        let editor = make_ffi_editor(initial);
        let endpoints = editor
            .selection_endpoints()
            .expect("ffi call returns Ok")
            .expect("external-only image selection must expose endpoints through FFI");

        assert_eq!(endpoints.from_position.node, root);
        assert_eq!(endpoints.from_position.offset, 0);
        assert_eq!(
            endpoints.from_position.affinity,
            editor_state::Affinity::Downstream
        );
        assert_eq!(endpoints.to_position.node, root);
        assert_eq!(endpoints.to_position.offset, 1);
        assert_eq!(
            endpoints.to_position.affinity,
            editor_state::Affinity::Upstream
        );
        assert_eq!(endpoints.from.rect.width, 0.0);
        assert_eq!(endpoints.to.rect.width, 0.0);
        assert!(
            endpoints.from.rect.height > 0.0 && endpoints.to.rect.height > 0.0,
            "external-only endpoint handles must keep image geometry"
        );
    }

    #[test]
    fn ffi_selection_endpoints_collapsed_is_none() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let editor = make_ffi_editor(initial);
        let result = editor.selection_endpoints().expect("ffi call returns Ok");
        assert!(result.is_none());
    }

    #[test]
    fn ffi_selection_hit_test_resolves_and_forwards() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let editor = make_ffi_editor(initial);
        let endpoints = editor
            .selection_endpoints()
            .expect("ffi call returns Ok")
            .expect("range selection has endpoints");
        let probe_x = endpoints.from.rect.x + 5.0;
        let probe_y = endpoints.from.rect.y + endpoints.from.rect.height * 0.5;
        let hit = editor
            .selection_hit_test(0, probe_x, probe_y)
            .expect("ffi call returns Ok");
        assert!(
            hit,
            "probe inside selection rect must register as hit through FFI"
        );
    }

    #[test]
    fn copy_selection_returns_payload_for_text_range() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let editor = make_ffi_editor(state);
        let payload = editor
            .copy_selection()
            .expect("ffi call returns Ok")
            .expect("non-collapsed selection produces payload");
        assert_eq!(payload.text, "Hello");
        assert!(payload.html.contains("data-slice"));
    }

    #[test]
    fn copy_selection_returns_none_for_collapsed() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let editor = make_ffi_editor(state);
        assert!(
            editor
                .copy_selection()
                .expect("ffi call returns Ok")
                .is_none()
        );
    }

    #[test]
    fn ffi_missing_changesets_tolerant_roundtrips_and_skips_unknown_heads() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let empty_heads = editor_codec::encode_dots(&[]).unwrap();
        let out: MissingChangesets = editor
            .missing_changesets_tolerant(empty_heads)
            .unwrap()
            .from_ffi()
            .unwrap();
        let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&out.bytes)
                .unwrap()
                .into_graph_input();
        assert!(
            !css.is_empty(),
            "seed paragraph is unconfirmed vs empty heads"
        );

        let bogus = editor_codec::encode_dots(&[editor_crdt::Dot::new(424242, 7)]).unwrap();
        assert!(
            editor.missing_changesets_tolerant(bogus).is_ok(),
            "unknown head must not error"
        );
    }

    #[test]
    fn ffi_split_changesets_keys_by_first_op_dot() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let out: MissingChangesets = editor
            .missing_changesets_tolerant(editor_codec::encode_dots(&[]).unwrap())
            .unwrap()
            .from_ffi()
            .unwrap();
        let entries = editor.split_changesets(out.bytes).unwrap();
        assert!(!entries.is_empty());
    }

    #[test]
    fn ffi_split_changesets_empty_input_is_empty_list() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let entries = editor.split_changesets(Vec::new()).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn ffi_local_changesets_since_current_heads_is_empty_bytes() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let heads = editor.current_heads().unwrap();
        let bytes = editor.local_changesets_since(heads).unwrap();
        assert!(
            bytes.is_empty(),
            "no changesets past current heads must be exactly 0 bytes, not a minimal envelope"
        );
    }

    /// Hand-assembles a single-changeset bundle whose one op is an unrecognized
    /// (v-next) op tag, using only editor-codec's public low-level primitives —
    /// mirrors editor-codec's own `vnext.rs` synth pattern (the `test-util`-gated
    /// helpers there aren't visible from this crate). `parents` are the changeset's
    /// parents (empty = Genesis), so the carrier can sit on top of an existing graph.
    fn synth_bundle_with_unknown_op(actor: u64, parents: &[editor_crdt::Dot]) -> Vec<u8> {
        use editor_codec::ctx::{CollectCtx, EncCtx, write_dot, write_preamble};
        use editor_codec::durable::Durable;
        use editor_codec::envelope::{Envelope, PayloadKind, wrap};
        use editor_codec::framing::{UnknownPayload, write_frame};
        use editor_codec::types::DurableOp;
        use editor_codec::varint::write_varint;

        let id = editor_crdt::Dot::new(actor, 0);
        let mut parents = parents.to_vec();
        parents.sort();
        let mut cc = CollectCtx::new();
        cc.observe(&id);
        for p in &parents {
            cc.observe(p);
        }
        let (actors, baselines) = cc.finalize();
        let ctx = EncCtx::from_parts(&actors, baselines.clone()).unwrap();
        let mut body = Vec::new();
        write_preamble(&actors, &baselines, &mut body).unwrap();
        write_varint(1, &mut body); // changeset_count
        if parents.is_empty() {
            body.push(0); // cs_parents: Genesis
        } else {
            body.push(2); // cs_parents: Explicit
            write_varint(parents.len() as u64, &mut body);
            for p in &parents {
                write_dot(p, &ctx, &mut body).unwrap();
            }
        }
        write_varint(1, &mut body); // op_count
        write_frame(&mut body, |b| {
            write_dot(&id, &ctx, b)?;
            DurableOp::Unknown(UnknownPayload {
                tag: 12345,
                bytes: vec![0xAB],
            })
            .encode(&ctx, b)
        })
        .unwrap();
        wrap(&Envelope::new(PayloadKind::ChangesetBundle, body)).unwrap()
    }

    fn synth_bundle_with_record_tail(
        actor: u64,
        parents: &[editor_crdt::Dot],
        pos: u64,
    ) -> Vec<u8> {
        use editor_codec::ctx::{CollectCtx, EncCtx, write_dot, write_preamble};
        use editor_codec::durable::Durable;
        use editor_codec::envelope::{Envelope, PayloadKind, wrap};
        use editor_codec::framing::write_frame;
        use editor_codec::types::{DurableItem, DurableOp};
        use editor_codec::varint::write_varint;

        let id = editor_crdt::Dot::new(actor, 0);
        let mut parents = parents.to_vec();
        parents.sort();
        let mut cc = CollectCtx::new();
        cc.observe(&id);
        for p in &parents {
            cc.observe(p);
        }
        let (actors, baselines) = cc.finalize();
        let ctx = EncCtx::from_parts(&actors, baselines.clone()).unwrap();
        let mut body = Vec::new();
        write_preamble(&actors, &baselines, &mut body).unwrap();
        write_varint(1, &mut body); // changeset_count
        if parents.is_empty() {
            body.push(0); // cs_parents: Genesis
        } else {
            body.push(2); // cs_parents: Explicit
            write_varint(parents.len() as u64, &mut body);
            for p in &parents {
                write_dot(p, &ctx, &mut body).unwrap();
            }
        }
        write_varint(1, &mut body); // op_count
        write_frame(&mut body, |b| {
            write_dot(&id, &ctx, b)?;
            DurableOp::SeqIns {
                pos,
                item: DurableItem::Char('x'),
            }
            .encode(&ctx, b)?;
            b.extend_from_slice(&[0xEE, 0x07]);
            Ok(())
        })
        .unwrap();
        wrap(&Envelope::new(PayloadKind::ChangesetBundle, body)).unwrap()
    }

    #[test]
    fn stash_carriers_registers_only_carrier_changesets_with_verbatim_bytes() {
        let clean_cs = editor_crdt::Changeset {
            ops: vec![editor_crdt::Op {
                id: editor_crdt::Dot::new(50, 0),
                parents: vec![],
                payload: editor_model::EditOp::Seq(editor_crdt::ListOp::Ins {
                    pos: 0,
                    item: editor_model::SeqItem::Char('a'),
                }),
            }],
        };
        let mut stream = editor_codec::encode_changesets(
            editor_codec::ReencodableChangesets::from_local_ops(vec![clean_cs]),
        )
        .unwrap();
        stream.extend_from_slice(&synth_bundle_with_unknown_op(7, &[]));

        let decoded = editor_codec::decode_changeset_stream(&stream).unwrap();
        let lossless = decoded.lossless();
        let css = decoded.into_graph_input();
        assert_eq!(css.len(), 2);
        let parts = editor_codec::split_bundle_bytes(&stream).unwrap();

        let mut stash = CarrierStash::default();
        stash_carriers(&css, &stream, lossless, &mut stash).unwrap();

        assert_eq!(stash.len(), 1, "only the carrier changeset must be stashed");
        assert!(!stash.contains_key(&editor_crdt::Dot::new(50, 0)));
        assert_eq!(
            stash.get(&editor_crdt::Dot::new(7, 0)).unwrap(),
            &parts[1],
            "stashed bytes must be the carrier's split part, byte-identical"
        );
    }

    #[test]
    fn stash_carriers_all_known_leaves_stash_unchanged() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let stream =
            editor_codec::encode_changesets(editor_codec::ReencodableChangesets::from_local_ops(
                initial.projected.graph().changesets_as_vec(),
            ))
            .unwrap();
        let decoded = editor_codec::decode_changeset_stream(&stream).unwrap();
        let lossless = decoded.lossless();
        let css = decoded.into_graph_input();
        assert!(!css.is_empty());

        let mut stash = CarrierStash::default();
        stash_carriers(&css, &stream, lossless, &mut stash).unwrap();

        assert!(stash.is_empty());
    }

    #[test]
    fn stash_carriers_registers_record_tail_carrier_despite_clean_value_shape() {
        let clean_cs = editor_crdt::Changeset {
            ops: vec![editor_crdt::Op {
                id: editor_crdt::Dot::new(51, 0),
                parents: vec![],
                payload: editor_model::EditOp::Seq(editor_crdt::ListOp::Ins {
                    pos: 0,
                    item: editor_model::SeqItem::Char('a'),
                }),
            }],
        };
        let mut stream = editor_codec::encode_changesets(
            editor_codec::ReencodableChangesets::from_local_ops(vec![clean_cs]),
        )
        .unwrap();
        stream.extend_from_slice(&synth_bundle_with_record_tail(8, &[], 0));

        let decoded = editor_codec::decode_changeset_stream(&stream).unwrap();
        let lossless = decoded.lossless();
        let css = decoded.into_graph_input();
        assert_eq!(css.len(), 2);
        assert!(
            !editor_codec::changesets_contain_unknown(&css),
            "a record-tail carrier must look clean at the value level"
        );
        let parts = editor_codec::split_bundle_bytes(&stream).unwrap();

        let mut stash = CarrierStash::default();
        stash_carriers(&css, &stream, lossless, &mut stash).unwrap();

        assert_eq!(
            stash.len(),
            1,
            "only the record-tail changeset must be stashed"
        );
        assert!(!stash.contains_key(&editor_crdt::Dot::new(51, 0)));
        assert_eq!(
            stash.get(&editor_crdt::Dot::new(8, 0)).unwrap(),
            &parts[1],
            "stashed bytes must be the carrier's split part, byte-identical"
        );
    }

    #[test]
    fn assemble_send_payload_splits_runs_around_carriers_in_order() {
        let clean = |actor: u64, ch: char| editor_crdt::Changeset {
            ops: vec![editor_crdt::Op {
                id: editor_crdt::Dot::new(actor, 0),
                parents: vec![],
                payload: editor_model::EditOp::Seq(editor_crdt::ListOp::Ins {
                    pos: 0,
                    item: editor_model::SeqItem::Char(ch),
                }),
            }],
        };
        let carrier = |actor: u64| {
            let bundle = synth_bundle_with_unknown_op(actor, &[]);
            let part = editor_codec::split_bundle_bytes(&bundle).unwrap().remove(0);
            let cs = editor_codec::decode_changeset_stream(&bundle)
                .unwrap()
                .into_graph_input()
                .remove(0);
            (cs, part)
        };

        let (c1, p1) = carrier(70);
        let (c2, p2) = carrier(71);
        let (c3, p3) = carrier(72);
        let mut stash = CarrierStash::default();
        for (cs, part) in [(&c1, &p1), (&c2, &p2), (&c3, &p3)] {
            stash.insert(cs.ops[0].id, part.clone());
        }

        let css = vec![c1, c2, clean(80, 'x'), clean(81, 'y'), c3];
        let expected_ids: Vec<editor_crdt::Dot> = css.iter().map(|cs| cs.ops[0].id).collect();
        let (bytes, withheld) = assemble_send_payload(css, &stash).unwrap();
        assert_eq!(withheld, 0);

        let decoded = editor_codec::decode_changeset_stream(&bytes)
            .unwrap()
            .into_graph_input();
        let ids: Vec<editor_crdt::Dot> = decoded.iter().map(|cs| cs.ops[0].id).collect();
        assert_eq!(
            ids, expected_ids,
            "leading/consecutive/trailing carriers must keep window order"
        );

        let parts = editor_codec::split_bundle_bytes(&bytes).unwrap();
        assert_eq!(parts[0], p1);
        assert_eq!(parts[1], p2);
        assert_eq!(parts[4], p3);
    }

    #[test]
    fn ffi_send_window_emits_stashed_carrier_bytes_verbatim() {
        use editor_core::{InsertionOp, Message};

        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let pre_heads = editor.current_heads().unwrap();

        editor
            .enqueue(Message::Insertion {
                op: InsertionOp::Text { text: "a".into() },
            })
            .unwrap();
        let _ = editor.tick().unwrap();

        let parents = editor_codec::decode_dots(&editor.current_heads().unwrap()).unwrap();
        let bundle = synth_bundle_with_unknown_op(900, &parents);
        let stashed = editor_codec::split_bundle_bytes(&bundle).unwrap().remove(0);

        editor.receive_remote_changeset(bundle).unwrap();
        let _ = editor.tick().unwrap();
        assert!(
            editor
                .changeset_ids()
                .unwrap()
                .contains(&"900:0".to_string()),
            "carrier must enter the graph through the real receive path"
        );

        let out: MissingChangesets = editor
            .missing_changesets_tolerant(pre_heads.clone())
            .unwrap()
            .from_ffi()
            .unwrap();
        assert_eq!(out.withheld, 0);

        let parts = editor_codec::split_bundle_bytes(&out.bytes).unwrap();
        let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&out.bytes)
                .unwrap()
                .into_graph_input();
        assert_eq!(css.len(), parts.len());
        let carrier_idx = css
            .iter()
            .position(|cs| cs.ops[0].id == editor_crdt::Dot::new(900, 0))
            .expect("carrier must be in the send window");
        assert_eq!(
            parts[carrier_idx], stashed,
            "carrier bytes must round-trip verbatim, not value-reencoded"
        );

        let heads_set: hashbrown::HashSet<editor_crdt::Dot> = editor_codec::decode_dots(&pre_heads)
            .unwrap()
            .into_iter()
            .collect();
        let oracle = editor
            .with_inner(|inner| Ok(inner.editor.missing_changesets_tolerant(&heads_set)))
            .unwrap();
        assert_eq!(
            css, oracle,
            "decoded payload must equal the graph window value-for-value"
        );
    }

    #[test]
    fn ffi_send_window_round_trips_record_tail_carrier_bytes_verbatim() {
        use editor_core::{InsertionOp, Message};

        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let pre_heads = editor.current_heads().unwrap();
        let step = |text: &str| {
            editor
                .enqueue(Message::Insertion {
                    op: InsertionOp::Text { text: text.into() },
                })
                .unwrap();
            let _ = editor.tick().unwrap();
        };

        step("a");
        let carrier_dot = editor_crdt::Dot::new(930, 0);
        let parents = editor_codec::decode_dots(&editor.current_heads().unwrap()).unwrap();
        let bundle = synth_bundle_with_record_tail(930, &parents, 1);
        let carrier_envelope = editor_codec::split_bundle_bytes(&bundle).unwrap().remove(0);

        let decoded = editor_codec::decode_changeset_stream(&bundle)
            .unwrap()
            .into_graph_input();
        assert!(
            matches!(
                decoded[0].ops[0].payload,
                editor_model::EditOp::Seq(editor_crdt::ListOp::Ins { .. })
            ),
            "carrier op must decode to a known shape"
        );
        assert!(
            !editor_codec::changesets_contain_unknown(&decoded),
            "a record-tail carrier must look clean at the value level"
        );
        assert!(editor_codec::bundle_contains_unknown(&bundle).unwrap());

        editor.receive_remote_changeset(bundle).unwrap();
        let _ = editor.tick().unwrap();
        assert!(
            editor
                .changeset_ids()
                .unwrap()
                .contains(&"930:0".to_string()),
            "carrier must enter the graph through the real receive path"
        );

        step("b");

        let out: MissingChangesets = editor
            .missing_changesets_tolerant(pre_heads)
            .unwrap()
            .from_ffi()
            .unwrap();
        assert_eq!(out.withheld, 0);

        let parts = editor_codec::split_bundle_bytes(&out.bytes).unwrap();
        let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&out.bytes)
                .unwrap()
                .into_graph_input();
        assert_eq!(css.len(), parts.len());
        let carrier_idx = css
            .iter()
            .position(|cs| cs.ops[0].id == carrier_dot)
            .expect("carrier must be in the send window");
        assert_eq!(
            parts[carrier_idx], carrier_envelope,
            "record tail must survive the send round-trip byte-identically"
        );

        assert!(
            editor
                .inner
                .lock()
                .unwrap()
                .carrier_bytes
                .contains_key(&carrier_dot),
            "receive must stash the record-tail carrier"
        );
    }

    #[test]
    fn ffi_send_window_preserves_interleaved_carrier_order() {
        use editor_core::{InsertionOp, Message};

        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let pre_heads = editor.current_heads().unwrap();
        let step = |text: &str| {
            editor
                .enqueue(Message::Insertion {
                    op: InsertionOp::Text { text: text.into() },
                })
                .unwrap();
            let _ = editor.tick().unwrap();
        };

        step("a");
        let parents = editor_codec::decode_dots(&editor.current_heads().unwrap()).unwrap();
        editor
            .receive_remote_changeset(synth_bundle_with_unknown_op(901, &parents))
            .unwrap();
        let _ = editor.tick().unwrap();
        step("b");

        let heads_set: hashbrown::HashSet<editor_crdt::Dot> = editor_codec::decode_dots(&pre_heads)
            .unwrap()
            .into_iter()
            .collect();
        let oracle_ids: Vec<editor_crdt::Dot> = editor
            .with_inner(|inner| Ok(inner.editor.missing_changesets_tolerant(&heads_set)))
            .unwrap()
            .iter()
            .map(|cs| cs.ops[0].id)
            .collect();
        let carrier_pos = oracle_ids
            .iter()
            .position(|id| *id == editor_crdt::Dot::new(901, 0))
            .expect("carrier must be in the send window");
        assert!(
            carrier_pos > 0 && carrier_pos + 1 < oracle_ids.len(),
            "scenario must sandwich the carrier between clean changesets"
        );

        let out: MissingChangesets = editor
            .missing_changesets_tolerant(pre_heads)
            .unwrap()
            .from_ffi()
            .unwrap();
        assert_eq!(out.withheld, 0);
        let emitted_ids: Vec<editor_crdt::Dot> = editor_codec::decode_changeset_stream(&out.bytes)
            .unwrap()
            .into_graph_input()
            .iter()
            .map(|cs| cs.ops[0].id)
            .collect();
        assert_eq!(
            emitted_ids, oracle_ids,
            "payload must preserve the window's causal order"
        );
    }

    #[test]
    fn ffi_send_window_stash_miss_truncates_to_causal_prefix() {
        use editor_core::{InsertionOp, Message};

        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let pre_heads = editor.current_heads().unwrap();
        let step = |text: &str| {
            editor
                .enqueue(Message::Insertion {
                    op: InsertionOp::Text { text: text.into() },
                })
                .unwrap();
            let _ = editor.tick().unwrap();
        };

        step("a");
        let parents = editor_codec::decode_dots(&editor.current_heads().unwrap()).unwrap();
        editor
            .receive_remote_changeset(synth_bundle_with_unknown_op(902, &parents))
            .unwrap();
        let _ = editor.tick().unwrap();
        step("b");

        editor.inner.lock().unwrap().carrier_bytes.clear();

        let heads_set: hashbrown::HashSet<editor_crdt::Dot> = editor_codec::decode_dots(&pre_heads)
            .unwrap()
            .into_iter()
            .collect();
        let oracle = editor
            .with_inner(|inner| Ok(inner.editor.missing_changesets_tolerant(&heads_set)))
            .unwrap();
        let carrier_pos = oracle
            .iter()
            .position(|cs| cs.ops[0].id == editor_crdt::Dot::new(902, 0))
            .expect("carrier must be in the send window");
        assert!(
            carrier_pos + 1 < oracle.len(),
            "scenario must have a clean changeset after the carrier"
        );

        let out: MissingChangesets = editor
            .missing_changesets_tolerant(pre_heads)
            .unwrap()
            .from_ffi()
            .unwrap();
        assert_eq!(out.withheld as usize, oracle.len() - carrier_pos);
        assert!(out.withheld >= 2);

        let emitted = editor_codec::decode_changeset_stream(&out.bytes)
            .unwrap()
            .into_graph_input();
        assert_eq!(
            emitted.as_slice(),
            &oracle[..carrier_pos],
            "only the causal prefix before the miss may be emitted"
        );
    }

    #[test]
    fn ffi_send_window_without_carriers_matches_single_encode_bytes() {
        use editor_core::{InsertionOp, Message};

        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let pre_heads = editor.current_heads().unwrap();
        let step = |text: &str| {
            editor
                .enqueue(Message::Insertion {
                    op: InsertionOp::Text { text: text.into() },
                })
                .unwrap();
            let _ = editor.tick().unwrap();
        };
        step("a");
        step("b");

        let heads_set: hashbrown::HashSet<editor_crdt::Dot> = editor_codec::decode_dots(&pre_heads)
            .unwrap()
            .into_iter()
            .collect();
        let oracle = editor
            .with_inner(|inner| Ok(inner.editor.missing_changesets_tolerant(&heads_set)))
            .unwrap();
        assert!(!oracle.is_empty());
        let expected = editor_codec::encode_changesets(
            editor_codec::ReencodableChangesets::from_local_ops(oracle),
        )
        .unwrap();

        let out: MissingChangesets = editor
            .missing_changesets_tolerant(pre_heads)
            .unwrap()
            .from_ffi()
            .unwrap();
        assert_eq!(out.withheld, 0);
        assert_eq!(
            out.bytes, expected,
            "carrier-free windows must stay byte-identical to a single encode"
        );

        let current = editor.current_heads().unwrap();
        let empty: MissingChangesets = editor
            .missing_changesets_tolerant(current)
            .unwrap()
            .from_ffi()
            .unwrap();
        assert!(empty.bytes.is_empty());
        assert_eq!(empty.withheld, 0);
    }

    #[test]
    fn ffi_insert_template_fragment_rejects_unknown_bearing_template() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let bytes = synth_bundle_with_unknown_op(7, &[]);
        assert!(
            editor.insert_template_fragment(bytes).is_err(),
            "template carrying an op newer than this reader must be rejected at insert time"
        );
    }

    #[test]
    fn ffi_insert_template_fragment_accepts_empty_bytes_as_noop() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        assert!(editor.insert_template_fragment(Vec::new()).is_ok());
    }

    #[test]
    fn ffi_partition_remote_changesets_splits_ready_from_blocked_by_byte_index() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);

        // Independent genesis actor — no dependency on the editor's own graph or
        // on the other changeset in this payload, so unconditionally ready.
        let ready_op = editor_crdt::Op {
            id: editor_crdt::Dot::new(50, 0),
            parents: vec![],
            payload: editor_model::EditOp::Seq(editor_crdt::ListOp::Ins {
                pos: 0,
                item: editor_model::SeqItem::Char('r'),
            }),
        };
        // Parent dot is neither in the editor's graph nor produced by the other
        // changeset in this payload, so it stays blocked.
        let blocked_op = editor_crdt::Op {
            id: editor_crdt::Dot::new(51, 0),
            parents: vec![editor_crdt::Dot::new(999, 5)],
            payload: editor_model::EditOp::Seq(editor_crdt::ListOp::Ins {
                pos: 0,
                item: editor_model::SeqItem::Char('b'),
            }),
        };
        let ready_cs = editor_crdt::Changeset {
            ops: vec![ready_op.clone()],
        };
        let blocked_cs = editor_crdt::Changeset {
            ops: vec![blocked_op.clone()],
        };

        // Blocked changeset first in the payload so the byte-index selection is
        // pinned against source order, not merely "first entry is ready".
        let payload =
            editor_codec::encode_changesets(editor_codec::ReencodableChangesets::from_local_ops(
                vec![blocked_cs.clone(), ready_cs.clone()],
            ))
            .unwrap();

        let result = editor.partition_remote_changesets(payload).unwrap();
        let result: PartitionedChangesets = result.from_ffi().unwrap();

        let ready_decoded = editor_codec::decode_changeset_stream(&result.ready)
            .unwrap()
            .into_graph_input();
        let blocked_decoded = editor_codec::decode_changeset_stream(&result.blocked)
            .unwrap()
            .into_graph_input();

        assert_eq!(ready_decoded.len(), 1);
        assert_eq!(ready_decoded[0].ops[0].id, ready_op.id);
        assert_eq!(blocked_decoded.len(), 1);
        assert_eq!(blocked_decoded[0].ops[0].id, blocked_op.id);
    }

    // Regression: the website pusher captures `missing_changesets_tolerant(captured)`
    // into IndexedDB and advances `captured = current_heads()` after every cycle. A
    // capture landing between an undo and the next edit must not lose the undo
    // changeset — a reload from (server graph, captured records) has to reproduce
    // the live document instead of dropping everything after the undo as orphans.
    #[test]
    fn ffi_reload_with_pending_survives_capture_between_undo_and_next_edit() {
        use editor_core::{HistoryOp, InsertionOp, Message};

        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let server_graph =
            editor_codec::encode_changesets(editor_codec::ReencodableChangesets::from_local_ops(
                initial.projected.graph().changesets_as_vec(),
            ))
            .unwrap();
        let server_heads = editor_codec::encode_dots(
            &initial
                .projected
                .graph()
                .current_heads()
                .copied()
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let editor = make_ffi_editor(initial);

        let mut captured = server_heads;
        let mut records: Vec<(String, Vec<u8>)> = Vec::new();
        let capture = |captured: &mut Vec<u8>, records: &mut Vec<(String, Vec<u8>)>| {
            let fresh: MissingChangesets = editor
                .missing_changesets_tolerant(captured.clone())
                .unwrap()
                .from_ffi()
                .unwrap();
            if !fresh.bytes.is_empty() {
                for entry in editor.split_changesets(fresh.bytes).unwrap() {
                    let entry = entry.from_ffi().unwrap();
                    match records.iter_mut().find(|(id, _)| *id == entry.id) {
                        Some((_, bytes)) => *bytes = entry.bytes,
                        None => records.push((entry.id, entry.bytes)),
                    }
                }
            }
            *captured = editor.current_heads().unwrap();
        };
        let step = |msg: Message| {
            editor.enqueue(msg).unwrap();
            let _ = editor.tick().unwrap();
        };

        step(Message::Insertion {
            op: InsertionOp::Text {
                text: "keep ".into(),
            },
        });
        capture(&mut captured, &mut records);
        step(Message::Insertion {
            op: InsertionOp::Text {
                text: "oops".into(),
            },
        });
        capture(&mut captured, &mut records);
        step(Message::History {
            op: HistoryOp::Undo,
        });
        capture(&mut captured, &mut records);
        step(Message::Insertion {
            op: InsertionOp::Text {
                text: "final".into(),
            },
        });
        capture(&mut captured, &mut records);

        let (reloaded_state, _) = crate::graph::state_from_changesets_with_pending(
            server_graph,
            records.into_iter().map(|(_, bytes)| bytes).collect(),
        )
        .unwrap();
        let reloaded = Editor::new(
            editor_core::Editor::new_test(reloaded_state),
            CarrierStash::default(),
        );

        assert_eq!(reloaded.prose_text().unwrap(), editor.prose_text().unwrap());
        let mut live_ids = editor.changeset_ids().unwrap();
        let mut reloaded_ids = reloaded.changeset_ids().unwrap();
        live_ids.sort();
        reloaded_ids.sort();
        assert_eq!(reloaded_ids, live_ids);
    }

    #[test]
    fn ffi_mixed_version_window_survives_capture_fold_and_reload() {
        use editor_core::{InsertionOp, Message};

        use crate::server::{BundleStatus, CollectResult, EditorServer};

        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let server_graph =
            editor_codec::encode_changesets(editor_codec::ReencodableChangesets::from_local_ops(
                initial.projected.graph().changesets_as_vec(),
            ))
            .unwrap();
        let server_heads = editor_codec::encode_dots(
            &initial
                .projected
                .graph()
                .current_heads()
                .copied()
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let editor = make_ffi_editor(initial);

        let mut captured = server_heads.clone();
        let mut records: Vec<(String, Vec<u8>)> = Vec::new();
        let capture = |captured: &mut Vec<u8>, records: &mut Vec<(String, Vec<u8>)>| {
            let fresh: MissingChangesets = editor
                .missing_changesets_tolerant(captured.clone())
                .unwrap()
                .from_ffi()
                .unwrap();
            assert_eq!(
                fresh.withheld, 0,
                "capture must never withhold while the stash is intact"
            );
            if !fresh.bytes.is_empty() {
                for entry in editor.split_changesets(fresh.bytes).unwrap() {
                    let entry = entry.from_ffi().unwrap();
                    match records.iter_mut().find(|(id, _)| *id == entry.id) {
                        Some((_, bytes)) => *bytes = entry.bytes,
                        None => records.push((entry.id, entry.bytes)),
                    }
                }
            }
            *captured = editor.current_heads().unwrap();
        };
        let step = |text: &str| {
            editor
                .enqueue(Message::Insertion {
                    op: InsertionOp::Text { text: text.into() },
                })
                .unwrap();
            let _ = editor.tick().unwrap();
        };

        step("one ");
        capture(&mut captured, &mut records);

        let carrier_dot = editor_crdt::Dot::new(910, 0);
        let parents = editor_codec::decode_dots(&editor.current_heads().unwrap()).unwrap();
        let bundle = synth_bundle_with_unknown_op(910, &parents);
        let carrier_envelope = editor_codec::split_bundle_bytes(&bundle).unwrap().remove(0);
        editor.receive_remote_changeset(bundle).unwrap();
        let _ = editor.tick().unwrap();
        assert!(
            editor
                .changeset_ids()
                .unwrap()
                .contains(&"910:0".to_string()),
            "carrier must enter the graph through the real receive path"
        );

        step("two");

        capture(&mut captured, &mut records);
        let carrier_record = records
            .iter()
            .find(|(id, _)| id == "910:0")
            .expect("capture must persist the carrier as its own record");
        assert_eq!(
            carrier_record.1, carrier_envelope,
            "captured carrier record must be the original envelope bytes"
        );

        let drain: MissingChangesets = editor
            .missing_changesets_tolerant(server_heads.clone())
            .unwrap()
            .from_ffi()
            .unwrap();
        assert_eq!(drain.withheld, 0);
        let drained = editor_codec::decode_changeset_stream(&drain.bytes)
            .unwrap()
            .into_graph_input();
        let drain_parts = editor_codec::split_bundle_bytes(&drain.bytes).unwrap();
        let carrier_idx = drained
            .iter()
            .position(|cs| cs.ops[0].id == carrier_dot)
            .expect("carrier must be in the drain payload");
        assert_eq!(
            drain_parts[carrier_idx], carrier_envelope,
            "the server must receive the carrier's original envelope bytes"
        );
        assert!(
            drained
                .iter()
                .any(|cs| cs.ops.iter().any(|op| op.parents.contains(&carrier_dot))),
            "the second edit must be a causal descendant of the carrier"
        );

        let server = EditorServer;
        let mut packed = Vec::new();
        packed.extend_from_slice(&1u32.to_le_bytes());
        packed.extend_from_slice(&(drain.bytes.len() as u32).to_le_bytes());
        packed.extend_from_slice(&drain.bytes);
        let fold: CollectResult = server
            .collect_fold(server_graph.clone(), packed)
            .unwrap()
            .from_ffi()
            .unwrap();
        assert_eq!(fold.statuses, vec![BundleStatus::Applied]);

        let mut live_heads = editor_codec::decode_dots(&editor.current_heads().unwrap()).unwrap();
        live_heads.sort();
        let mut fold_heads = editor_codec::decode_dots(&fold.heads).unwrap();
        fold_heads.sort();
        assert_eq!(
            fold_heads, live_heads,
            "folded server heads must converge to the live client frontier"
        );
        assert!(
            !fold_heads.contains(&carrier_dot),
            "carrier must be covered by the folded frontier, not sit on it"
        );
        let mut incremental_heads = editor_codec::decode_dots(
            &server
                .update_heads(server_heads.clone(), drain.bytes.clone())
                .unwrap(),
        )
        .unwrap();
        incremental_heads.sort();
        assert_eq!(incremental_heads, fold_heads);

        let (reloaded_state, stash) = crate::graph::state_from_changesets_with_pending(
            server_graph,
            records.into_iter().map(|(_, bytes)| bytes).collect(),
        )
        .unwrap();
        assert!(
            stash.contains_key(&carrier_dot),
            "load must rehydrate the carrier stash from the pending records"
        );
        let reloaded = Editor::new(editor_core::Editor::new_test(reloaded_state), stash);
        assert_eq!(reloaded.prose_text().unwrap(), editor.prose_text().unwrap());

        let reassembled: MissingChangesets = reloaded
            .missing_changesets_tolerant(server_heads)
            .unwrap()
            .from_ffi()
            .unwrap();
        assert_eq!(reassembled.withheld, 0);
        let reassembled_css = editor_codec::decode_changeset_stream(&reassembled.bytes)
            .unwrap()
            .into_graph_input();
        let reassembled_parts = editor_codec::split_bundle_bytes(&reassembled.bytes).unwrap();
        let reassembled_idx = reassembled_css
            .iter()
            .position(|cs| cs.ops[0].id == carrier_dot)
            .expect("carrier must be in the reloaded send window");
        assert_eq!(
            reassembled_parts[reassembled_idx], carrier_envelope,
            "reloaded send window must emit the carrier verbatim"
        );
    }

    #[test]
    fn ffi_reload_rehydrates_server_graph_carrier_into_send_window() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let base_heads = editor_codec::encode_dots(
            &initial
                .projected
                .graph()
                .current_heads()
                .copied()
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let mut server_graph =
            editor_codec::encode_changesets(editor_codec::ReencodableChangesets::from_local_ops(
                initial.projected.graph().changesets_as_vec(),
            ))
            .unwrap();

        let carrier_dot = editor_crdt::Dot::new(920, 0);
        let parents = editor_codec::decode_dots(&base_heads).unwrap();
        let bundle = synth_bundle_with_unknown_op(920, &parents);
        let carrier_envelope = editor_codec::split_bundle_bytes(&bundle).unwrap().remove(0);
        server_graph.extend_from_slice(&bundle);

        let assert_carrier_sendable = |state: editor_state::State, stash: CarrierStash| {
            assert!(
                stash.contains_key(&carrier_dot),
                "load must rehydrate the carrier stash from the server graph"
            );
            let reloaded = Editor::new(editor_core::Editor::new_test(state), stash);
            let out: MissingChangesets = reloaded
                .missing_changesets_tolerant(base_heads.clone())
                .unwrap()
                .from_ffi()
                .unwrap();
            assert_eq!(
                out.withheld, 0,
                "a server-history carrier re-entering the send window must not be withheld"
            );
            let css = editor_codec::decode_changeset_stream(&out.bytes)
                .unwrap()
                .into_graph_input();
            let parts = editor_codec::split_bundle_bytes(&out.bytes).unwrap();
            let idx = css
                .iter()
                .position(|cs| cs.ops[0].id == carrier_dot)
                .expect("carrier must be in the send window");
            assert_eq!(
                parts[idx], carrier_envelope,
                "send window must emit the server-graph carrier verbatim"
            );
        };

        let (state, stash) = crate::graph::state_from_changesets(server_graph.clone()).unwrap();
        assert_carrier_sendable(state, stash);

        let (state, stash) =
            crate::graph::state_from_changesets_with_pending(server_graph, Vec::new()).unwrap();
        assert_carrier_sendable(state, stash);
    }

    #[test]
    fn ffi_cursor_hit_test_resolves_and_forwards() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let editor = make_ffi_editor(initial);
        let cursor = editor
            .cursor()
            .expect("ffi call returns Ok")
            .expect("collapsed cursor has metrics")
            .from_ffi()
            .expect("cursor metrics decode");
        let probe_x = cursor.caret.x;
        let probe_y = cursor.line.y + cursor.line.height * 0.5;
        let hit = editor
            .cursor_hit_test(0, probe_x, probe_y)
            .expect("ffi call returns Ok");

        assert!(
            hit,
            "probe resolving to current cursor must register as hit through FFI"
        );
    }

    #[test]
    fn ffi_selection_returns_none_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.selection().expect("ffi call returns Ok");
        assert!(
            result.is_none(),
            "selection FFI must return None when state.selection is None"
        );
    }

    #[test]
    fn ffi_cursor_returns_none_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.cursor().expect("ffi call returns Ok");
        assert!(result.is_none());
    }

    #[test]
    fn ffi_copy_selection_returns_none_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.copy_selection().expect("ffi call returns Ok");
        assert!(result.is_none());
    }

    #[test]
    fn ffi_selection_endpoints_returns_none_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.selection_endpoints().expect("ffi call returns Ok");
        assert!(result.is_none());
    }

    #[test]
    fn ffi_modifier_state_returns_none_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.modifier_state().expect("ffi call returns Ok");
        assert!(result.is_none());
    }

    #[test]
    fn ffi_root_modifiers_returns_root_default_modifiers() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600), block_gap(120)] { p: paragraph { text("hello") } }
            }
            selection: (p, 0)
        };
        let editor = make_ffi_editor(initial);

        let result = editor.root_modifiers().expect("ffi call returns Ok");

        assert!(result.contains(&editor_model::Modifier::FontSize { value: 1600 }));
        assert!(result.contains(&editor_model::Modifier::BlockGap { value: 120 }));
    }

    #[test]
    fn ffi_block_state_returns_none_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.block_state().expect("ffi call returns Ok");
        assert!(result.is_none());
    }

    #[test]
    fn ffi_last_history_tag_reports_repaste_as_text_availability() {
        let (source, ..) = state! {
            doc { root { p1: paragraph { text("hello") [bold] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let payload = editor_clipboard::Slice::extract(&source)
            .unwrap()
            .to_payload(&editor_resource::Resource::new_test());

        let (initial, ..) = state! {
            doc { root { p2: paragraph { text("Hi") } } }
            selection: (p2, 1)
        };
        let editor = make_ffi_editor(initial);
        let expected_text = payload.text.clone();
        assert!(
            editor
                .last_history_tag()
                .expect("ffi call returns Ok")
                .is_none()
        );

        editor
            .enqueue(editor_core::Message::Clipboard {
                op: editor_core::ClipboardOp::Paste {
                    html: Some(payload.html),
                    text: payload.text,
                },
            })
            .expect("enqueue paste");
        let _ = editor.tick().expect("tick");

        assert!(matches!(
            editor.last_history_tag().expect("ffi call returns Ok"),
            Some(editor_common::HistoryTag::PasteHtml { plain_text, .. }) if plain_text == expected_text
        ));
    }

    #[test]
    fn ffi_external_elements_returns_empty_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.external_elements().expect("ffi call returns Ok");
        assert!(result.is_empty());
    }

    #[test]
    fn ffi_external_elements_lists_image_when_selection_is_none() {
        let (initial, ..) = state! {
            doc { root { image paragraph { text("hi") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let result = editor.external_elements().expect("ffi call returns Ok");
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn ffi_selection_hit_test_returns_false_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let hit = editor
            .selection_hit_test(0, 10.0, 10.0)
            .expect("ffi call returns Ok");
        assert!(
            !hit,
            "selection_hit_test must return false when state.selection is None"
        );
    }

    #[test]
    fn ffi_cursor_hit_test_returns_false_for_no_selection_state() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: none
        };
        let editor = make_ffi_editor(initial);
        let hit = editor
            .cursor_hit_test(0, 10.0, 10.0)
            .expect("ffi call returns Ok");
        assert!(
            !hit,
            "cursor_hit_test must return false when state.selection is None"
        );
    }

    #[test]
    fn ffi_selection_unset_then_set_roundtrip() {
        use editor_core::{Message, SelectionOp};
        use editor_state::{Position, Selection};

        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
        };
        let editor = make_ffi_editor(initial);

        editor
            .enqueue(Message::Selection {
                op: SelectionOp::Unset,
            })
            .expect("enqueue unset");
        let _ = editor.tick().expect("tick");
        assert!(
            editor.selection().expect("ffi ok").is_none(),
            "Unset must clear selection through FFI",
        );

        let new_sel = Selection::collapsed(Position::new(p1, 1));
        editor
            .enqueue(Message::Selection {
                op: SelectionOp::Set { selection: new_sel },
            })
            .expect("enqueue set");
        let _ = editor.tick().expect("tick");
        let after_set = editor.selection().expect("ffi ok");
        assert!(
            after_set.is_some(),
            "Set must restore selection through FFI"
        );
    }

    #[test]
    fn ffi_enter_in_synthetic_trailing_paragraph_after_image() {
        let (initial, ..) = state! {
            doc {
                root {
                    image
                }
            }
            selection: none
        };
        let synth_p = {
            let view = initial.view();
            let root = view.root().unwrap();
            root.child_blocks()
                .find(|b| b.node_type() == editor_model::NodeType::Paragraph)
                .map(|b| b.id())
                .expect("synthetic trailing paragraph")
        };
        assert!(
            synth_p.is_synthetic(),
            "trailing paragraph must be synthetic"
        );

        let editor = make_ffi_editor(initial);
        let selection = editor_state::Selection::collapsed(editor_state::Position {
            node: synth_p,
            offset: 0,
            affinity: editor_state::Affinity::Downstream,
        });
        editor
            .enqueue(
                editor_core::Message::Selection {
                    op: editor_core::SelectionOp::Set { selection },
                }
                .into_ffi()
                .unwrap(),
            )
            .unwrap();
        editor.tick().unwrap();

        let msg = editor_core::Message::Key {
            event: editor_core::KeyEvent {
                key: editor_core::Key::Enter,
                modifiers: Default::default(),
            },
        };
        editor.enqueue(msg.into_ffi().unwrap()).unwrap();
        editor.tick().unwrap();
    }

    #[test]
    fn ffi_freeze_selection_returns_stable_selection() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 1) -> (p1, 8)
        };
        let editor = make_ffi_editor(initial.clone());
        let sel = initial.selection.unwrap();

        let result = editor.freeze_selection(sel);
        assert!(
            result.is_ok(),
            "freeze_selection must Ok for valid selection"
        );

        let stable = result
            .expect("freeze_selection must Ok for valid selection")
            .expect("freeze_selection must return Some for valid selection");
        editor
            .enqueue(editor_core::Message::TrackedRange {
                op: editor_core::TrackedRangeOp::AddFrozen {
                    id: "r".into(),
                    group: "g".into(),
                    selection: stable,
                    metadata: String::new(),
                },
            })
            .expect("enqueue");
        let _ = editor.tick().expect("tick");

        let ranges = editor.tracked_ranges(None).expect("ffi ok");
        let r = ranges.iter().find(|x| x.id == "r").expect("range present");
        assert_eq!(r.anchor.offset, 1);
        assert_eq!(r.head.offset, 8);
    }

    #[test]
    fn ffi_tracked_ranges_exports_resolved_endpoints_after_history_remap() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("ㅁㄴㅇㅁㅁㅁㅁㄴㅁㅇ") } } }
            selection: (p1, 4) -> (p1, 6)
        };
        let editor = make_ffi_editor(initial.clone());
        let stable = editor
            .freeze_selection(initial.selection.unwrap())
            .expect("freeze")
            .expect("valid selection");

        editor
            .enqueue(
                editor_core::Message::TrackedRange {
                    op: editor_core::TrackedRangeOp::AddFrozen {
                        id: "r".into(),
                        group: "comment".into(),
                        selection: stable,
                        metadata: String::new(),
                    },
                }
                .into_ffi()
                .unwrap(),
            )
            .expect("enqueue add");
        let _ = editor.tick().expect("tick add");

        editor
            .enqueue(
                editor_core::Message::Selection {
                    op: editor_core::SelectionOp::Set {
                        selection: editor_state::Selection::new(
                            editor_state::Position::new(p1, 0),
                            editor_state::Position::new(p1, 7),
                        ),
                    },
                }
                .into_ffi()
                .unwrap(),
            )
            .expect("enqueue selection");
        editor
            .enqueue(
                editor_core::Message::Deletion {
                    op: editor_core::DeletionOp::Selection,
                }
                .into_ffi()
                .unwrap(),
            )
            .expect("enqueue delete");
        let _ = editor.tick().expect("tick delete");

        editor
            .enqueue(
                editor_core::Message::History {
                    op: editor_core::HistoryOp::Undo,
                }
                .into_ffi()
                .unwrap(),
            )
            .expect("enqueue undo");
        let _ = editor.tick().expect("tick undo");

        editor
            .enqueue(
                editor_core::Message::Selection {
                    op: editor_core::SelectionOp::Set {
                        selection: editor_state::Selection::collapsed(editor_state::Position::new(
                            p1, 6,
                        )),
                    },
                }
                .into_ffi()
                .unwrap(),
            )
            .expect("enqueue cursor");
        editor
            .enqueue(
                editor_core::Message::Deletion {
                    op: editor_core::DeletionOp::Move {
                        movement: editor_core::Movement::Grapheme {
                            direction: editor_core::Direction::Backward,
                        },
                    },
                }
                .into_ffi()
                .unwrap(),
            )
            .expect("enqueue backspace");
        let _ = editor.tick().expect("tick backspace");

        let ranges = editor.tracked_ranges(None).expect("ffi ok");
        let r = ranges.iter().find(|x| x.id == "r").expect("range present");
        assert_eq!(r.text, "ㅁ");
        assert_eq!(r.anchor.offset, 4);
        assert_eq!(r.head.offset, 5);
    }

    #[test]
    fn ffi_tracked_ranges_omits_unresolved_ranges() {
        let (state_a, _p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let editor_a = make_ffi_editor(state_a.clone());
        let stable = editor_a
            .freeze_selection(state_a.selection.unwrap())
            .expect("freeze")
            .expect("valid selection");

        let (state_b, ..) = state! {
            doc { root { p2: paragraph { text("") } } }
            selection: (p2, 0)
        };
        let editor_b = make_ffi_editor(state_b);
        editor_b
            .enqueue(
                editor_core::Message::TrackedRange {
                    op: editor_core::TrackedRangeOp::AddFrozen {
                        id: "r".into(),
                        group: "comment".into(),
                        selection: stable,
                        metadata: String::new(),
                    },
                }
                .into_ffi()
                .unwrap(),
            )
            .expect("enqueue add");
        let _ = editor_b.tick().expect("tick add");

        let ranges = editor_b.tracked_ranges(None).expect("ffi ok");
        assert!(ranges.is_empty());
    }

    #[test]
    fn ffi_freeze_selection_returns_none_for_unresolvable() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);

        let bogus = editor_crdt::Dot::new(9999, 0);
        let bogus_sel = editor_state::Selection::new(
            editor_state::Position::new(bogus, 0),
            editor_state::Position::new(bogus, 0),
        );

        let result = editor.freeze_selection(bogus_sel);
        assert!(
            result
                .expect("freeze_selection must Ok for unresolvable selection")
                .is_none(),
            "freeze_selection must return None for unresolvable selection"
        );
    }

    #[test]
    fn ffi_dnd_over_returns_events() {
        use editor_core::{DndOp, EditorEvent, ExternalDndPayloadKind, InputModifiers, Message};

        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let editor = make_ffi_editor(initial);
        let cursor = editor
            .cursor()
            .expect("ffi call returns Ok")
            .expect("collapsed cursor has metrics");

        editor
            .enqueue(Message::Dnd {
                op: DndOp::EnterExternal {
                    payload: ExternalDndPayloadKind::Text,
                },
            })
            .expect("ffi enqueue returns Ok");
        editor
            .enqueue(Message::Dnd {
                op: DndOp::Over {
                    page: 0,
                    x: cursor.caret.x,
                    y: cursor.line.y + cursor.line.height * 0.5,
                    modifiers: InputModifiers::default(),
                },
            })
            .expect("ffi enqueue returns Ok");
        let events = editor.tick().expect("ffi tick returns Ok");

        assert!(
            events
                .iter()
                .any(|e| matches!(e, EditorEvent::RenderInvalidated)),
            "immediate dnd over must return render invalidation events",
        );
    }

    #[test]
    fn ffi_tracked_ranges_at_returns_hits() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let editor = make_ffi_editor(initial.clone());
        let sel = initial.selection.unwrap();

        editor
            .enqueue(editor_core::Message::TrackedRange {
                op: editor_core::TrackedRangeOp::Add {
                    id: "thread-a".into(),
                    group: "comment".into(),
                    selection: sel,
                    metadata: String::new(),
                },
            })
            .expect("enqueue tracked-range add");
        let _ = editor.tick().expect("tick");

        let endpoints = editor
            .selection_endpoints()
            .expect("ffi ok")
            .expect("selection produces endpoints");
        let cx = endpoints.from.rect.x + 2.0;
        let cy = endpoints.from.rect.y + endpoints.from.rect.height * 0.5;

        let page = endpoints.from.page_idx as u32;
        let hits = editor
            .tracked_ranges_at(page, cx, cy, Some("comment".into()))
            .expect("ffi ok");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "thread-a");
        assert_eq!(hits[0].group, "comment");
        assert!(
            !hits[0].rects.is_empty(),
            "FFI wrapper must forward range rects (hit point itself proves at least one)"
        );
        assert_eq!(
            hits[0].rects[0].page_idx, endpoints.from.page_idx,
            "rects must be page-local"
        );
    }

    #[test]
    fn ffi_spellcheck_tracked_range_text_matches_context() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("않녕하세요") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);

        let sel = editor.prose_to_selection(0, 5).expect("ok").expect("some");

        editor
            .enqueue(editor_core::Message::TrackedRange {
                op: editor_core::TrackedRangeOp::Add {
                    id: "err-1".into(),
                    group: "spellcheck".into(),
                    selection: sel,
                    metadata: String::new(),
                },
            })
            .expect("enqueue tracked-range add");
        let _ = editor.tick().expect("tick");

        let ranges = editor.tracked_ranges(None).expect("ffi ok");
        assert_eq!(ranges.len(), 1, "range must be present");
        assert_eq!(
            ranges[0].text, "않녕하세요",
            "tracked range text must equal server context so staleness filter keeps it"
        );

        let _ = p1;
    }

    #[test]
    fn ffi_export_page_vector_returns_nonempty_bytes() {
        // export 결과가 비어있지 않은 바이너리여야 호스트가 파일로 저장할 수 있다.
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let bytes = editor
            .export_page_vector(0, 1.0)
            .expect("export_page_vector must return Ok");
        assert!(!bytes.is_empty());
    }

    #[test]
    fn ffi_export_page_vector_starts_with_magic() {
        // TVE1(0x3156_4554) magic으로 시작해야 외부 변환기가 포맷을 식별할 수 있다.
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let bytes = editor
            .export_page_vector(0, 1.0)
            .expect("export_page_vector must return Ok");
        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        assert_eq!(magic, 0x3156_4554u32);
    }

    #[test]
    fn ffi_prose_to_selection_within_single_paragraph() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);

        let sel = editor.prose_to_selection(0, 5).expect("ok").expect("some");
        assert_eq!(
            sel.anchor.node, sel.head.node,
            "single-block range must share nodeId"
        );
        assert_eq!(
            sel.head.offset - sel.anchor.offset,
            5,
            "offset delta should be codepoint count"
        );
        assert!(matches!(
            sel.anchor.affinity,
            editor_state::Affinity::Downstream
        ));
        assert!(matches!(
            sel.head.affinity,
            editor_state::Affinity::Upstream
        ));

        // Regardless of the canonical Upstream head at the block end, the mapped
        // selection must cover the full prose range when its text is collected.
        editor
            .with_inner(|inner| {
                let view = inner.editor.state().view();
                let resolved = sel.resolve(&view).expect("resolve");
                assert_eq!(
                    resolved.collect_text(),
                    "hello",
                    "mapped selection must cover exactly the prose range"
                );
                Ok(())
            })
            .expect("with_inner ok");

        let _ = p1;
    }

    #[test]
    fn ffi_prose_to_selection_handles_multibyte_codepoints() {
        // "한글" → 2 codepoints, "ñ" → 1 codepoint, total 3.
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("한글ñ") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);

        let sel = editor.prose_to_selection(0, 3).expect("ok").expect("some");
        assert_eq!(sel.head.offset - sel.anchor.offset, 3, "3 codepoints");

        let _ = p1;
    }

    #[test]
    fn ffi_prose_to_selection_empty_range_is_collapsed() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);

        let sel = editor.prose_to_selection(2, 2).expect("ok").expect("some");
        assert!(
            sel.is_collapsed(),
            "empty range must produce collapsed selection (anchor==head incl. affinity)"
        );

        let _ = p1;
    }

    #[test]
    fn ffi_prose_to_selection_out_of_range_returns_none() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);

        let sel = editor.prose_to_selection(0, 5).expect("ok");
        assert!(sel.is_none(), "OOB range returns None");

        let _ = p1;
    }

    #[test]
    fn ffi_prose_to_selection_inverted_returns_none() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);

        let sel = editor.prose_to_selection(3, 1).expect("ok");
        assert!(sel.is_none(), "inverted range returns None");

        let _ = p1;
    }

    #[test]
    fn ffi_prose_to_selection_handles_emoji_surrogate_pair() {
        // "a😀b" — 3 codepoints, 6 UTF-8 bytes ('a'=1, '😀'=4, 'b'=1).
        // Position.offset is a codepoint index (Text::len() == chars().count()), so
        // the full range 0..3 spans 3 codepoints and the delta must be 3.
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("a😀b") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);

        let sel = editor.prose_to_selection(0, 3).expect("ok").expect("some");
        assert_eq!(
            sel.anchor.node, sel.head.node,
            "single-block range must share nodeId"
        );
        assert_eq!(
            sel.head.offset - sel.anchor.offset,
            3,
            "offset unit is codepoints: 'a'(1) + '😀'(1) + 'b'(1) = 3"
        );

        // 0..2 covers "a" and "😀" — 2 codepoints
        let sel2 = editor.prose_to_selection(0, 2).expect("ok").expect("some");
        assert_eq!(sel2.anchor.node, sel2.head.node);
        assert_eq!(
            sel2.head.offset - sel2.anchor.offset,
            2,
            "offset unit is codepoints: 'a'(1) + '😀'(1) = 2"
        );

        let _ = p1;
    }

    #[test]
    fn ffi_prose_to_selection_across_blocks() {
        // Two paragraphs: prose text is "a\n\nb" — 4 codepoints across two blocks.
        // anchor resolves to p1 (first paragraph text node) and head to p2 (second),
        // so anchor.node_id != head.node_id.
        let (initial, p1, _p2) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    p2: paragraph { text("b") }
                }
            }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);

        let sel = editor.prose_to_selection(0, 4).expect("ok").expect("some");
        assert_ne!(
            sel.anchor.node, sel.head.node,
            "cross-block range must have distinct anchor/head nodeIds"
        );

        let _ = p1;
    }

    #[test]
    fn ffi_prose_text_returns_doc_plain_text() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("안녕") }
                    p2: paragraph { text("Hello") }
                }
            }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);

        let text = editor.prose_text().expect("prose_text ok");
        assert!(text.contains("안녕"), "first paragraph text missing");
        assert!(text.contains("Hello"), "second paragraph text missing");
        assert_eq!(
            text.chars().count(),
            9,
            "expected 9 codepoints, got: {text:?}"
        );
    }

    fn op_count(bytes: &[u8]) -> u32 {
        u32::from_le_bytes(bytes[12..16].try_into().unwrap())
    }

    #[test]
    fn ffi_export_page_vector_has_valid_dimensions() {
        // width/height가 양수여야 외부 변환기가 페이지 크기를 올바르게 재현할 수 있다.
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let bytes = editor.export_page_vector(0, 1.0).expect("must return Ok");
        let width = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let height = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
        assert!(width > 0.0 && height > 0.0);
    }

    #[test]
    fn ffi_export_page_vector_shape_produces_ops() {
        // horizontal_rule이 FillPath/StrokePath op으로 수집되어야 외부 변환기가 재현 가능하다.
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("a") } horizontal_rule } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let bytes = editor.export_page_vector(0, 1.0).expect("must return Ok");
        assert!(
            op_count(&bytes) > 0,
            "horizontal_rule must produce at least one op"
        );
    }

    #[test]
    fn ffi_export_page_vector_table_border_produces_ops() {
        // table border가 벡터 op으로 수집되어야 외부 변환기가 재현 가능하다.
        let (initial, _p1) = state! {
            doc { root {
                p1: paragraph { text("a") }
                table { table_row { table_cell { paragraph } table_cell { paragraph } } }
            } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let bytes = editor.export_page_vector(0, 1.0).expect("must return Ok");
        assert!(
            op_count(&bytes) > 0,
            "table border must produce at least one op"
        );
    }

    #[test]
    fn ffi_export_page_vector_text_page_produces_valid_binary() {
        // 텍스트 페이지도 글리프 outline 기반 벡터 op와 유효한 헤더를 포함한 바이너리를 반환해야 한다.
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let bytes = editor.export_page_vector(0, 1.0).expect("must return Ok");
        assert!(
            bytes.len() >= 16,
            "must produce at least magic+dimensions+op_count"
        );
    }

    fn assert_no_image_ops(bytes: &[u8]) {
        let mut off = 12usize;
        let op_count = u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
        off += 4;
        for _ in 0..op_count {
            let tag = bytes[off];
            off += 1;
            assert!(
                tag == 0 || tag == 1,
                "unexpected op tag {tag} (image op leaked into headless export)"
            );
            let path_count = u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap()) as usize;
            off += 4;
            for _ in 0..path_count {
                let cmd = bytes[off];
                off += 1;
                off += match cmd {
                    0 | 1 => 8,
                    2 => 16,
                    3 => 24,
                    4 => 0,
                    _ => panic!("bad path cmd {cmd}"),
                };
            }
            off += 4;
            if tag == 0 {
                off += 1;
            } else {
                off += 4 + 1 + 1;
            }
        }
    }

    #[test]
    fn ffi_export_page_vector_image_node_emits_no_image_ops() {
        // image 노드는 headless export에서 픽셀 데이터가 없으므로 tag=2 Image op를 방출하지 않아야 한다.
        // horizontal_rule을 함께 배치해 op_count > 0 조건을 보장한다 (비-공허 검증).
        let (initial, _p1) = state! {
            doc { root {
                p1: paragraph { text("a") }
                image
                horizontal_rule
            } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let bytes = editor.export_page_vector(0, 1.0).expect("must return Ok");
        assert!(
            op_count(&bytes) > 0,
            "page with horizontal_rule must produce at least one op (non-vacuity)"
        );
        assert_no_image_ops(&bytes);
    }

    #[test]
    fn ffi_export_page_vector_emoji_text_emits_no_image_ops() {
        // 이모지 텍스트는 VectorExport 모드에서 draw_glyph_run으로 처리되며 tag=2 Image op를 방출하지 않아야 한다.
        // horizontal_rule을 함께 배치해 op_count > 0 조건을 보장한다 (비-공허 검증).
        let (initial, _p1) = state! {
            doc { root {
                p1: paragraph { text("Hello 😀 World 🎉") }
                horizontal_rule
            } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let bytes = editor.export_page_vector(0, 1.0).expect("must return Ok");
        assert!(
            op_count(&bytes) > 0,
            "page with horizontal_rule must produce at least one op (non-vacuity)"
        );
        assert_no_image_ops(&bytes);
    }

    #[test]
    fn ffi_export_page_vector_ruby_text_emits_no_image_ops() {
        // ruby 어노테이션이 있는 텍스트도 headless export에서 tag=2 Image op를 방출하지 않아야 한다.
        // horizontal_rule을 함께 배치해 op_count > 0 조건을 보장한다 (비-공허 검증).
        let (initial, _p1) = state! {
            doc { root {
                p1: paragraph { text("漢字") [ruby(text: "かんじ".into())] }
                horizontal_rule
            } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let bytes = editor.export_page_vector(0, 1.0).expect("must return Ok");
        assert!(
            op_count(&bytes) > 0,
            "page with horizontal_rule must produce at least one op (non-vacuity)"
        );
        assert_no_image_ops(&bytes);
    }

    #[test]
    fn clear_page_present_state_removes_prev_dl_and_sig() {
        let (initial, ..) =
            state! { doc { root { p1: paragraph { text("hi") } } } selection: (p1, 0) };
        let editor = make_ffi_editor(initial);
        {
            let mut g = editor.render.lock().unwrap();
            g.prev_dl
                .insert(0, editor_renderer::display_list::DisplayList::default());
            g.last_render_sig.insert(0, 123);
        }
        editor.render.lock().unwrap().clear_page_present_state(0);
        let g = editor.render.lock().unwrap();
        assert!(!g.prev_dl.contains_key(&0));
        assert!(!g.last_render_sig.contains_key(&0));
    }

    #[test]
    fn ffi_invalidate_surface_clears_present_state() {
        let (initial, ..) =
            state! { doc { root { p1: paragraph { text("hi") } } } selection: (p1, 0) };
        let editor = make_ffi_editor(initial);
        {
            let mut g = editor.render.lock().unwrap();
            g.prev_dl
                .insert(0, editor_renderer::display_list::DisplayList::default());
            g.last_render_sig.insert(0, 123);
            g.last_content_height.insert(0, 456);
        }
        editor.invalidate_surface(0).unwrap();
        let g = editor.render.lock().unwrap();
        assert!(!g.prev_dl.contains_key(&0));
        assert!(!g.last_render_sig.contains_key(&0));
        assert!(!g.last_content_height.contains_key(&0));
    }

    #[test]
    fn detach_surface_clears_prev_dl() {
        let (initial, ..) =
            state! { doc { root { p1: paragraph { text("hi") } } } selection: (p1, 0) };
        let editor = make_ffi_editor(initial);
        {
            let mut g = editor.render.lock().unwrap();
            g.prev_dl
                .insert(0, editor_renderer::display_list::DisplayList::default());
            g.last_render_sig.insert(0, 123);
        }
        editor.detach_surface(0).expect("detach must succeed");
        let g = editor.render.lock().unwrap();
        assert!(!g.prev_dl.contains_key(&0));
        assert!(!g.last_render_sig.contains_key(&0));
    }
}
