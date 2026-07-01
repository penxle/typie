#[cfg(not(feature = "wasm-server"))]
use hashbrown::HashMap;
use std::sync::Mutex;

use editor_macros::ffi;
use serde::{Deserialize, Serialize};

#[cfg(not(feature = "wasm-server"))]
use crate::platform::{PlatformHandle, SurfaceHandle};
use crate::prelude::*;

struct EditorInner {
    editor: editor_core::Editor,
    #[cfg(not(feature = "wasm-server"))]
    surfaces: HashMap<u32, SurfaceHandle>,
    // Last render signature presented for each page. `render_surface` skips
    // re-rasterizing a page whose signature is unchanged (the selection-drag hot
    // path, where only the page under the moving endpoint actually changes).
    // Cleared whenever the page's pixel buffer is (re)created: attach/detach/resize.
    #[cfg(not(feature = "wasm-server"))]
    last_render_sig: HashMap<u32, u64>,
}

#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub struct Editor {
    inner: Mutex<EditorInner>,
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

    pub fn can(&self, message: Complex<editor_core::Message>) -> EditorResult<bool> {
        self.with_inner(|inner| Ok(inner.editor.can(message.from_ffi()?)?))
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
            let payload = editor_clipboard::Slice::extract(inner.editor.state())
                .map(|slice| slice.to_payload());
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
            Ok(crate::root::base_style_modifiers(inner.editor.state()).into_ffi()?)
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

    pub fn style_entries(&self) -> EditorResult<Vec<Complex<editor_core::StyleInfo>>> {
        self.with_inner(|inner| Ok(inner.editor.style_entries().into_ffi()?))
    }

    pub fn applied_style(
        &self,
    ) -> EditorResult<Complex<editor_common::Tri<editor_core::StyleRefValue>>> {
        self.with_inner(|inner| Ok(inner.editor.applied_style().into_ffi()?))
    }

    pub fn style_divergence(&self) -> EditorResult<bool> {
        self.with_inner(|inner| Ok(inner.editor.style_divergence()))
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

    pub fn cursor_hit_test(&self, page: u32, x: f32, y: f32) -> EditorResult<bool> {
        self.with_inner(|inner| Ok(inner.editor.cursor_hit_test(page as usize, x, y)))
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
    ) -> EditorResult<Complex<editor_core::Ime>> {
        self.with_inner(|inner| Ok(inner.editor.ime(before_limit, after_limit)?.into_ffi()?))
    }

    pub fn receive_remote_changeset(&self, payload: Vec<u8>) -> EditorResult<()> {
        self.with_inner(|inner| {
            let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
                editor_crdt::wire::decode(&payload[..])
                    .map_err(|e| FfiError::Deserialization(e.to_string()))?;
            for changeset in css {
                inner.editor.receive_remote_changeset(changeset);
            }
            Ok(())
        })
    }

    pub fn local_changesets_since(&self, remote_heads_payload: Vec<u8>) -> EditorResult<Vec<u8>> {
        self.with_inner(|inner| {
            let heads_vec = editor_crdt::wire::decode_dots(&remote_heads_payload[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
            let heads_set: hashbrown::HashSet<editor_crdt::Dot> = heads_vec.into_iter().collect();
            let css = inner.editor.local_changesets_since(&heads_set)?;
            let bytes = editor_crdt::wire::encode(&css)
                .map_err(|e| FfiError::Serialization(e.to_string()))?;
            Ok(bytes)
        })
    }

    pub fn missing_changesets_tolerant(
        &self,
        remote_heads_payload: Vec<u8>,
    ) -> EditorResult<Vec<u8>> {
        self.with_inner(|inner| {
            let heads_vec = editor_crdt::wire::decode_dots(&remote_heads_payload[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
            let heads_set: hashbrown::HashSet<editor_crdt::Dot> = heads_vec.into_iter().collect();
            let css = inner.editor.missing_changesets_tolerant(&heads_set);
            let bytes = editor_crdt::wire::encode(&css)
                .map_err(|e| FfiError::Serialization(e.to_string()))?;
            Ok(bytes)
        })
    }

    pub fn partition_remote_changesets(
        &self,
        payload: Vec<u8>,
    ) -> EditorResult<Complex<PartitionedChangesets>> {
        self.with_inner(|inner| {
            let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
                editor_crdt::wire::decode(&payload[..])
                    .map_err(|e| FfiError::Deserialization(e.to_string()))?;
            let (ready, blocked) = inner.editor.partition_ready(css);
            let encode =
                |v: &[editor_crdt::Changeset<editor_model::EditOp>]| -> EditorResult<Vec<u8>> {
                    if v.is_empty() {
                        Ok(Vec::new())
                    } else {
                        editor_crdt::wire::encode(v)
                            .map_err(|e| FfiError::Serialization(e.to_string()).into())
                    }
                };
            Ok(PartitionedChangesets {
                ready: encode(&ready)?,
                blocked: encode(&blocked)?,
            }
            .into_ffi()?)
        })
    }

    pub fn split_changesets(&self, payload: Vec<u8>) -> EditorResult<Vec<Complex<ChangesetEntry>>> {
        let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_crdt::wire::decode(&payload[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let mut out = Vec::with_capacity(css.len());
        for cs in css {
            let first = cs
                .ops
                .first()
                .ok_or(FfiError::Deserialization("empty changeset".into()))?;
            let id = format!("{}:{}", first.id.actor, first.id.clock);
            let bytes = editor_crdt::wire::encode(std::slice::from_ref(&cs))
                .map_err(|e| FfiError::Serialization(e.to_string()))?;
            out.push(ChangesetEntry { id, bytes }.into_ffi()?);
        }
        Ok(out)
    }

    pub fn current_heads(&self) -> EditorResult<Vec<u8>> {
        self.with_inner(|inner| {
            let heads = inner.editor.current_heads();
            let bytes = editor_crdt::wire::encode_dots(&heads)
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
                .filter_map(|cs| {
                    cs.ops
                        .first()
                        .map(|op| format!("{}:{}", op.id.actor, op.id.clock))
                })
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
            let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
                editor_crdt::wire::decode(&changesets[..])
                    .map_err(|e| FfiError::Deserialization(e.to_string()))?;
            let template = editor_state::State::from_changesets(css, None)?;
            inner.editor.insert_template_fragment(template.to_plain())?;
            Ok(())
        })
    }

    pub fn materialize_at(&self, heads: Vec<u8>) -> EditorResult<Complex<editor_model::PlainDoc>> {
        self.with_inner(|inner| {
            let heads_vec = editor_crdt::wire::decode_dots(&heads[..])
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
            let projected = editor_state::ProjectedState::from_graph(subgraph).map_err(|e| {
                EditorError::General {
                    msg: format!("materialize projection failed: {e:?}"),
                }
            })?;
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
        use editor_state::{
            Affinity, Position, ResolvedPosition, ResolvedPositionFlatExt, Selection,
        };

        self.with_inner(|inner| {
            let view = inner.editor.state().view();
            let prose = editor_state::prose(&view);
            let Some(flat) = prose.to_flat_range((start as usize)..(end as usize)) else {
                return Ok(None);
            };
            let Some(anchor_rp) = ResolvedPosition::from_flat(&view, flat.start) else {
                return Ok(None);
            };
            let Some(head_rp) = ResolvedPosition::from_flat(&view, flat.end) else {
                return Ok(None);
            };

            let (anchor_aff, head_aff) = if start == end {
                (Affinity::Downstream, Affinity::Downstream)
            } else {
                (Affinity::Downstream, Affinity::Upstream)
            };

            let selection = Selection {
                anchor: Position {
                    node: anchor_rp.node(),
                    offset: anchor_rp.offset(),
                    affinity: anchor_aff,
                },
                head: Position {
                    node: head_rp.node(),
                    offset: head_rp.offset(),
                    affinity: head_aff,
                },
            };
            Ok(Some(selection).into_ffi()?)
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
        let surface = SurfaceHandle::new(handle, width, height, scale_factor)?;
        self.with_inner(|inner| {
            inner.surfaces.insert(page, surface);
            // Fresh buffer: drop any stale signature so the first paint always renders.
            inner.last_render_sig.remove(&page);
            Ok(())
        })
    }

    pub fn detach_surface(&self, page: u32) -> EditorResult<()> {
        self.with_inner(|inner| {
            inner.surfaces.remove(&page);
            inner.last_render_sig.remove(&page);
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
        self.with_inner(|inner| {
            if let Some(surface) = inner.surfaces.get_mut(&page) {
                surface.resize(width, height, scale_factor);
                // Buffer was recreated/cleared — its old signature no longer reflects pixels.
                inner.last_render_sig.remove(&page);
            }
            Ok(())
        })
    }

    pub fn render_surface(&self, page: u32) -> EditorResult<()> {
        self.with_inner(|inner| {
            // Skip the full re-raster + present when nothing this page draws has
            // changed since the last presented frame. During a selection drag only
            // the page under the moving endpoint changes; every other visible page
            // keeps a stable signature and is skipped here.
            let sig = inner.editor.page_render_signature(page);
            if inner.last_render_sig.get(&page) == Some(&sig) {
                return Ok(());
            }
            if let Some(surface) = inner.surfaces.get_mut(&page) {
                let scale_factor = surface.scale_factor() as f32;
                inner.editor.render_page(page, surface.sink(), scale_factor);
                surface.present();
                inner.last_render_sig.insert(page, sig);
            }
            Ok(())
        })
    }
}

#[cfg(feature = "wasm-server")]
#[wasm_bindgen::prelude::wasm_bindgen]
impl Editor {
    pub fn render_page_to_buffer(
        &self,
        page: u32,
        width: u32,
        height: u32,
    ) -> EditorResult<Vec<u8>> {
        self.with_inner(|inner| {
            let mut backend = editor_renderer::RenderBackend::new_cpu(width as u16, height as u16);
            inner.editor.render_page(page, backend.sink(), 1.0);

            let mut buf = vec![0u8; (width * height * 4) as usize];
            match &mut backend {
                editor_renderer::RenderBackend::Cpu(sink) => {
                    sink.flush_to(&mut buf);
                }
            }

            Ok(buf)
        })
    }
}

impl Editor {
    pub(crate) fn new(core: editor_core::Editor) -> Self {
        Self {
            inner: Mutex::new(EditorInner {
                editor: core,
                #[cfg(not(feature = "wasm-server"))]
                surfaces: HashMap::new(),
                #[cfg(not(feature = "wasm-server"))]
                last_render_sig: HashMap::new(),
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
        Editor::new(core)
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
        let empty_heads = editor_crdt::wire::encode_dots(&[]).unwrap();
        let bytes = editor.missing_changesets_tolerant(empty_heads).unwrap();
        let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_crdt::wire::decode(&bytes).unwrap();
        assert!(
            !css.is_empty(),
            "seed paragraph is unconfirmed vs empty heads"
        );

        let bogus = editor_crdt::wire::encode_dots(&[editor_crdt::Dot::new(424242, 7)]).unwrap();
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
        let bytes = editor
            .missing_changesets_tolerant(editor_crdt::wire::encode_dots(&[]).unwrap())
            .unwrap();
        let entries = editor.split_changesets(bytes).unwrap();
        assert!(!entries.is_empty());
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
    fn ffi_root_modifiers_returns_root_base_style_modifiers() {
        let (initial, ..) = state! {
            doc {
                styles { base: "기본" [font_size(1600), block_gap(120)] }
                root @base [] { p: paragraph { text("hello") } }
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
            .to_payload();

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
    fn ffi_can_returns_true_for_insertion_with_selection() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let msg = editor_core::Message::Insertion {
            op: editor_core::InsertionOp::Text { text: "x".into() },
        };
        let probed = editor.can(msg.into_ffi().unwrap()).unwrap();
        assert!(probed);
    }

    #[test]
    fn ffi_can_returns_false_for_undo_empty_history() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);
        let msg = editor_core::Message::History {
            op: editor_core::HistoryOp::Undo,
        };
        let probed = editor.can(msg.into_ffi().unwrap()).unwrap();
        assert!(!probed);
    }

    #[test]
    fn ffi_can_returns_false_for_same_selection_set() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let editor = make_ffi_editor(initial);
        let same = editor_state::Selection::collapsed(editor_state::Position::new(p1, 2));
        let msg = editor_core::Message::Selection {
            op: editor_core::SelectionOp::Set { selection: same },
        };
        let probed = editor.can(msg.into_ffi().unwrap()).unwrap();
        assert!(!probed);
    }

    #[test]
    fn ffi_can_does_not_mutate_observable_state() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let editor = make_ffi_editor(initial);

        let inspect_before = editor.inspect_state_as_macro().unwrap();
        let msg = editor_core::Message::Insertion {
            op: editor_core::InsertionOp::Text { text: "x".into() },
        };
        let _ = editor.can(msg.into_ffi().unwrap()).unwrap();
        let inspect_after = editor.inspect_state_as_macro().unwrap();

        assert_eq!(
            inspect_before, inspect_after,
            "can() must not mutate observable state visible through inspect_state_as_macro",
        );
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
}
