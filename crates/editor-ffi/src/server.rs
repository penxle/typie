use editor_macros::ffi;
use serde::{Deserialize, Serialize};

use crate::prelude::*;

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ChunkCodepoints {
    pub chunks: Vec<Vec<u32>>,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AnchorPaths {
    pub paths: Vec<Vec<u32>>,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Materialized {
    pub plain: editor_model::PlainDoc,
    pub text: String,
    pub projection_degraded: bool,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BundleStatus {
    Applied,
    Duplicate,
    Failed,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CollectResult {
    #[serde(with = "serde_bytes")]
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array"))]
    pub heads: Vec<u8>,
    // Per-bundle: whether it advanced the snapshot, was a no-op duplicate, or
    // was dead-lettered, plus the document's character count right after it
    // (for per-user attribution — unchanged for `duplicate`/`failed`).
    pub statuses: Vec<BundleStatus>,
    pub char_counts: Vec<u32>,
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array[]"))]
    pub entry_heads: Vec<serde_bytes::ByteBuf>,
    pub gross_insertions: Vec<u32>,
    pub gross_deletions: Vec<u32>,
    pub base_char_count: u32,
    pub plain: editor_model::PlainDoc,
    pub text: String,
    pub totality_violations: u32,
    pub projection_degraded: bool,
}

#[ffi]
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ConsolidateResult {
    #[serde(with = "serde_bytes")]
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array | null"))]
    pub payload: Option<Vec<u8>>,
    pub consumed: u32,
    pub consumed_bytes: u32,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ResolvedV1Selection {
    pub selection: editor_state::StableSelection,
    // `true` when the v1 anchor collapsed to its offset-0 fallback rather than
    // resolving exactly — the migration surfaces these for review.
    pub degraded: bool,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ProseRange {
    // 원고 텍스트(prose_text_annotated)의 UTF-16 코드 유닛 좌표 — JS 문자열 인덱스 그대로
    pub start: u32,
    pub end: u32,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProseRanges {
    pub ranges: Vec<ProseRange>,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ProseAnchor {
    // 입력 ranges의 인덱스 — 캡처에 실패한 range는 목록에서 빠진다
    pub index: u32,
    pub selection: editor_state::StableSelection,
    pub text: String,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProseAnchorCapture {
    // false면 heads 시점 원고가 expected_text와 다르다 — 좌표계가 다른 텍스트에 offset을 대지 않는다
    pub text_matches: bool,
    pub anchors: Vec<ProseAnchor>,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GraphWithAnchors {
    #[serde(with = "serde_bytes")]
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array"))]
    pub graph: Vec<u8>,
    pub anchors: Vec<editor_state::StableSelection>,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct XmlErrorInfo {
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub dot: Option<String>,
    pub detail: String,
    pub message: String,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct XmlVerdict {
    pub error: Option<XmlErrorInfo>,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct XmlRender {
    pub error: Option<XmlErrorInfo>,
    pub xml: String,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct XmlEditResult {
    pub error: Option<XmlErrorInfo>,
    #[serde(with = "serde_bytes")]
    #[cfg_attr(feature = "wasm", tsify(type = "Uint8Array"))]
    pub bundle: Vec<u8>,
    pub xml: String,
    pub blocks_inserted: u32,
    pub blocks_deleted: u32,
    pub blocks_moved: u32,
    pub blocks_updated: u32,
    pub chars_inserted: u32,
    pub chars_deleted: u32,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct XmlOutlineAttr {
    pub key: String,
    pub value: String,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct XmlOutlineRow {
    pub path: String,
    pub name: String,
    pub dot: Option<String>,
    pub attrs: Vec<XmlOutlineAttr>,
    pub preview: Option<String>,
    pub chars: Option<u32>,
    pub children: u32,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct XmlOutline {
    pub error: Option<XmlErrorInfo>,
    pub head: Option<XmlOutlineRow>,
    pub rows: Vec<XmlOutlineRow>,
    pub total: u32,
    pub xml: Option<String>,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct XmlOpErrorInfo {
    pub op: Option<u32>,
    pub address: Option<String>,
    pub info: XmlErrorInfo,
}

#[ffi]
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct XmlEdit {
    pub error: Option<XmlOpErrorInfo>,
    pub xml: String,
    pub affected: Vec<XmlOutline>,
}

fn xml_error_info(e: &editor_xml::XmlError) -> Result<XmlErrorInfo, FfiError> {
    let detail =
        serde_json::to_string(&e.detail).map_err(|err| FfiError::Serialization(err.to_string()))?;
    Ok(XmlErrorInfo {
        line: e.pos.map(|p| p.line),
        column: e.pos.map(|p| p.column),
        dot: e.dot.clone(),
        detail,
        message: e.message.clone(),
    })
}

fn outline_row_ffi(row: editor_xml::OutlineRow) -> XmlOutlineRow {
    XmlOutlineRow {
        path: row.path,
        name: row.name,
        dot: row.dot,
        attrs: row
            .attrs
            .into_iter()
            .map(|(key, value)| XmlOutlineAttr { key, value })
            .collect(),
        preview: row.preview,
        chars: row.chars,
        children: row.children,
    }
}

fn outline_ffi(result: editor_xml::OutlineResult) -> XmlOutline {
    XmlOutline {
        error: None,
        head: result.head.map(outline_row_ffi),
        rows: result.rows.into_iter().map(outline_row_ffi).collect(),
        total: result.total,
        xml: result.xml,
    }
}

#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub struct EditorServer {
    icu: editor_resource::IcuResources,
}

#[cfg_attr(feature = "wasm", editor_macros::ffi_export(wasm))]
#[allow(dead_code)]
impl EditorServer {
    pub fn create(icu_data: Vec<u8>) -> EditorResult<Owned<Self>> {
        let icu = editor_resource::IcuResources::from_icu_data(&icu_data)?;
        Ok(into_owned(Self { icu }))
    }

    pub fn count_characters(&self, text: String) -> u32 {
        editor_resource::count_text(
            &text,
            &self.icu.segmenters.grapheme,
            &self.icu.general_category,
        )
        .with_whitespace
    }

    #[cfg(feature = "wasm-server")]
    pub fn get_font_metadata(
        &self,
        data: Vec<u8>,
    ) -> EditorResult<Complex<editor_server::font::FontMetadata>> {
        editor_server::font::get_font_metadata(&data)?
            .into_ffi()
            .map_err(Into::into)
    }

    #[cfg(feature = "wasm-server")]
    pub fn get_font_codepoints(&self, ttf_data: Vec<u8>) -> EditorResult<Vec<u32>> {
        Ok(editor_server::font::get_font_codepoints(&ttf_data)?)
    }

    #[cfg(feature = "wasm-server")]
    pub fn outline_text_to_svg(&self, font_data: Vec<u8>, text: String) -> EditorResult<String> {
        Ok(editor_server::font::outline_text_to_svg(&font_data, &text)?)
    }

    /// Returns the compile error when the pattern is unusable, `None` when it is
    /// fine. Invalid input is an expected answer here, not a failure, so it does
    /// not go through `EditorResult`.
    #[cfg(feature = "wasm-server")]
    pub fn validate_regex(&self, pattern: String) -> Option<String> {
        editor_resource::validate_regex(&pattern).err()
    }

    #[cfg(feature = "wasm-server")]
    pub fn build_font(
        &self,
        ttf_data: Vec<u8>,
        chunk_codepoints: Complex<ChunkCodepoints>,
    ) -> EditorResult<Complex<editor_server::font::BuiltFont>> {
        let chunk_codepoints = chunk_codepoints.from_ffi()?;
        editor_server::font::build_font(&ttf_data, &chunk_codepoints.chunks)?
            .into_ffi()
            .map_err(Into::into)
    }

    #[cfg(feature = "wasm-server")]
    pub fn build_font_manifest(
        &self,
        coverages: Complex<ChunkCodepoints>,
    ) -> EditorResult<Vec<u8>> {
        let coverages = coverages.from_ffi()?;
        Ok(editor_server::font::build_font_manifest(&coverages.chunks)?)
    }

    pub fn extract_text(&self, doc: Complex<editor_model::PlainDoc>) -> EditorResult<String> {
        let plain: editor_model::PlainDoc = doc.from_ffi()?;
        let state = editor_state::State::from_plain(&plain).map_err(|e| EditorError::General {
            msg: format!("{e:?}"),
        })?;
        Ok(editor_state::doc_plain_text(&state.view()))
    }

    pub fn default_doc_with_preset(
        &self,
        root: Complex<editor_model::PlainRootNode>,
        modifiers: Vec<Complex<editor_model::Modifier>>,
    ) -> EditorResult<Complex<editor_model::PlainDoc>> {
        let root = root.from_ffi()?;
        let modifiers: Vec<editor_model::Modifier> = modifiers.from_ffi()?;
        Ok(build_default_doc(root, modifiers).into_ffi()?)
    }

    pub fn apply(&self, existing: Vec<u8>, new: Vec<u8>) -> EditorResult<Vec<u8>> {
        let existing_cs: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&existing[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_reencodable()
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .as_slice()
                .to_vec();
        let new_cs: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&new[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_reencodable()
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .as_slice()
                .to_vec();

        // Atomic boundaries make the first-op dot a stable changeset key, so
        // dedup and dot-reuse rejection only need to walk by first-op dot.
        let mut known: hashbrown::HashSet<editor_crdt::Dot> = existing_cs
            .iter()
            .flat_map(|cs| cs.ops.iter().map(|op| op.id))
            .collect();

        let mut out = existing_cs;
        for cs in new_cs {
            let Some(first) = cs.ops.first() else {
                continue;
            };
            // Same first-op dot under a divergent body means the original
            // boundary contract has been broken upstream — atomicity fixes
            // a dot's boundary on first arrival, so the divergent body must
            // not persist alongside the original.
            if let Some(prev) = out
                .iter()
                .find(|c| c.ops.first().map(|op| op.id) == Some(first.id))
            {
                if prev == &cs {
                    continue;
                }
                return Err(FfiError::CausalOrderViolation { first_op: first.id }.into());
            }
            // `seen` insert is intentionally last: at parent-check time it
            // does not yet contain `op.id`, so an op with `parents = [op.id]`
            // (self-reference) fails the `known ∪ seen` membership test. The
            // `known.contains(op.id)` guard above catches non-first dot
            // reuse that would otherwise survive the first-op dedup and
            // surface as `DotConflict` on the receiver.
            let mut seen: hashbrown::HashSet<editor_crdt::Dot> = hashbrown::HashSet::new();
            let mut parents_ok = true;
            for op in &cs.ops {
                if known.contains(&op.id) {
                    parents_ok = false;
                    break;
                }
                if !op
                    .parents
                    .iter()
                    .all(|p| known.contains(p) || seen.contains(p))
                {
                    parents_ok = false;
                    break;
                }
                if !seen.insert(op.id) {
                    parents_ok = false;
                    break;
                }
            }
            if !parents_ok {
                return Err(FfiError::CausalOrderViolation { first_op: first.id }.into());
            }
            // Extend `known` live so a later cs in the same `new` payload
            // can legally depend on an earlier cs accepted this iteration.
            known.extend(seen);
            out.push(cs);
        }

        if out.is_empty() {
            return Ok(Vec::new());
        }
        let bytes = editor_codec::encode_changesets(
            editor_codec::ReencodableChangesets::from_verified(out),
        )
        .map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    /// Fold a batch onto `existing` for the collect job while attributing
    /// per-bundle character counts — with the expensive `State` build amortized.
    /// The old collect ran `validate_and_extract_text` (a full `from_changesets`
    /// build) per entry (`O(tail × build)`); here the `State` is built once and
    /// each bundle is projected incrementally (`receive_remote_changesets`), then
    /// only the text is re-read per entry (`O(tail × extract)`, far cheaper than
    /// rebuilding). `char_counts[i]` is the document's character count right after
    /// bundle `i`; `statuses[i]` is `Applied`, `Duplicate` (verbatim re-delivery —
    /// advance the cursor, no dead-letter), or `Failed` (dead-letter).
    pub fn collect_fold(
        &self,
        existing: Vec<u8>,
        packed_bundles: Vec<u8>,
    ) -> EditorResult<Complex<CollectResult>> {
        Ok(self
            .collect_fold_inner(existing, packed_bundles)?
            .into_ffi()?)
    }

    pub fn consolidate(&self, stream: Vec<u8>) -> EditorResult<Complex<ConsolidateResult>> {
        let result = editor_codec::consolidate_stream(&stream).map_err(|e| match e {
            editor_codec::CodecError::Encode(_) => FfiError::Serialization(e.to_string()),
            editor_codec::CodecError::Corruption(_) | editor_codec::CodecError::Fenced(_) => {
                FfiError::Deserialization(e.to_string())
            }
        })?;
        let out = match result {
            Some(c) => ConsolidateResult {
                payload: Some(c.payload),
                consumed: c.consumed as u32,
                consumed_bytes: c.consumed_bytes as u32,
            },
            None => ConsolidateResult {
                payload: None,
                consumed: 0,
                consumed_bytes: 0,
            },
        };
        Ok(out.into_ffi()?)
    }

    pub fn missing_for(
        &self,
        all_changesets: Vec<u8>,
        remote_heads_payload: Vec<u8>,
    ) -> EditorResult<Vec<u8>> {
        let cs: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&all_changesets[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_reencodable()
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .as_slice()
                .to_vec();
        let heads_vec = editor_codec::decode_dots(&remote_heads_payload[..])
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let heads_set: hashbrown::HashSet<editor_crdt::Dot> = heads_vec.into_iter().collect();

        let g = editor_crdt::OpGraph::from_changesets(cs)?;
        let missing = g.missing_changesets_tolerant(&heads_set);
        if missing.is_empty() {
            return Ok(Vec::new());
        }

        let bytes = editor_codec::encode_changesets(
            editor_codec::ReencodableChangesets::from_verified(missing),
        )
        .map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    pub fn to_graph(&self, plain: Complex<editor_model::PlainDoc>) -> EditorResult<Vec<u8>> {
        let plain: editor_model::PlainDoc = plain.from_ffi()?;
        let state = editor_state::State::from_plain(&plain).map_err(|e| EditorError::General {
            msg: format!("{e:?}"),
        })?;
        let changesets = state.graph().changesets_as_vec();
        if changesets.is_empty() {
            return Ok(Vec::new());
        }
        let bytes = editor_codec::encode_changesets(
            editor_codec::ReencodableChangesets::from_local_ops(changesets),
        )
        .map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    pub fn to_graph_with_anchors(
        &self,
        plain: Complex<editor_model::PlainDoc>,
        anchor_paths: Complex<AnchorPaths>,
    ) -> EditorResult<Complex<GraphWithAnchors>> {
        let plain: editor_model::PlainDoc = plain.from_ffi()?;
        let paths: AnchorPaths = anchor_paths.from_ffi()?;
        let (graph, anchors) = crate::anchors::graph_with_anchors(&plain, &paths.paths)
            .map_err(|e| EditorError::General { msg: e.to_string() })?;
        Ok(GraphWithAnchors { graph, anchors }.into_ffi()?)
    }

    pub fn to_plain(
        &self,
        changeset_payloads: Vec<u8>,
    ) -> EditorResult<Complex<editor_model::PlainDoc>> {
        let cs: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&changeset_payloads[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        let state = crate::graph::build_state_tolerant(cs, &[])?;
        Ok(state.to_plain().into_ffi()?)
    }

    pub fn to_plain_resolved(
        &self,
        changeset_payloads: Vec<u8>,
    ) -> EditorResult<Complex<editor_model::PlainDoc>> {
        let cs: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&changeset_payloads[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        let state = crate::graph::build_state_tolerant(cs, &[])?;
        Ok(state.to_plain().into_ffi()?)
    }

    pub fn heads(&self, changeset_payloads: Vec<u8>) -> EditorResult<Vec<u8>> {
        let cs: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&changeset_payloads[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        // Frontier scan, not a full `from_changesets` build: heads is just
        // `all ids − referenced parent ids`, and every heads/durableHeads
        // caller on the server was paying a whole-graph rebuild for it.
        let heads = editor_crdt::OpGraph::<editor_model::EditOp>::heads_of(&cs);
        if heads.is_empty() {
            return Ok(Vec::new());
        }
        let bytes = editor_codec::encode_dots(&heads)
            .map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    /// Advance a cached frontier by one push bundle without touching the
    /// graph: a dot is a head iff no op references it as a parent, so
    /// `F' = (F ∪ ids(bundle)) − parents(bundle)` — `O(bundle)`, while
    /// rebuilding the frontier from the merged graph is `O(history)` (the
    /// 8MB-document push paid a full decode + merge + re-encode per push).
    /// Set arithmetic makes it idempotent under duplicate redelivery and
    /// order-independent across concurrent pushes.
    pub fn update_heads(&self, prev_heads: Vec<u8>, bundle: Vec<u8>) -> EditorResult<Vec<u8>> {
        let prev = editor_codec::decode_dots(&prev_heads[..])
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        // Structural read only (op id/parents) to update the frontier set — no
        // changeset value is reencoded, so a v-next-bearing bundle is fine here.
        let cs: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&bundle[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();

        let mut heads: hashbrown::HashSet<editor_crdt::Dot> = prev.into_iter().collect();
        for cs in &cs {
            for op in &cs.ops {
                heads.insert(op.id);
            }
        }
        for cs in &cs {
            for op in &cs.ops {
                for p in &op.parents {
                    heads.remove(p);
                }
            }
        }
        let mut heads: Vec<editor_crdt::Dot> = heads.into_iter().collect();
        heads.sort();
        if heads.is_empty() {
            return Ok(Vec::new());
        }
        let bytes = editor_codec::encode_dots(&heads)
            .map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    pub fn revert(
        &self,
        graph: Vec<u8>,
        target_heads: Vec<u8>,
        sweep_tombstones: Vec<String>,
    ) -> EditorResult<Vec<u8>> {
        // Input graph is used only to build state (`into_graph_input`); the only
        // thing ever reencoded is the revert transaction's own new local
        // changesets below (`from_local_ops`) — the input graph is never
        // value-reencoded.
        let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&graph[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        let target_vec = editor_codec::decode_dots(&target_heads[..])
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let target_set: hashbrown::HashSet<editor_crdt::Dot> = target_vec.into_iter().collect();
        let overlay = crate::graph::parse_sweep_tombstones(&sweep_tombstones);

        // The overlay hides swept dots for reading; a projection whose diff
        // becomes persisted ops must not hide them.
        let state = crate::graph::build_state_tolerant(css, &[])
            .map_err(|e| FfiError::RevertFailed(e.to_string()))?;
        let current_heads: hashbrown::HashSet<editor_crdt::Dot> =
            state.graph().current_heads().copied().collect();

        let target_state = state_at_heads(state.graph(), &target_set, &overlay)?;
        if target_state.projection_degraded() {
            return Err(FfiError::RevertFailed("target projection is degraded".to_string()).into());
        }

        let tr = editor_transaction::build_revert_transaction(&state, &target_state)
            .map_err(|e| FfiError::RevertFailed(e.to_string()))?;
        let (new_state, ..) = tr.commit();

        let revert_css = new_state.graph().local_changesets_since(&current_heads)?;
        if revert_css.is_empty() {
            return Ok(Vec::new());
        }

        let bytes = editor_codec::encode_changesets(
            editor_codec::ReencodableChangesets::from_local_ops(revert_css),
        )
        .map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    /// The file the model reads. The state is built the way `edit_from_xml`
    /// builds it — sweep tombstones included — so that the dots it writes are
    /// the dots the edit will look for.
    pub fn to_xml(
        &self,
        graph: Vec<u8>,
        sweep_tombstones: Vec<String>,
    ) -> EditorResult<Complex<XmlRender>> {
        fn failed(error: XmlErrorInfo) -> XmlRender {
            XmlRender {
                error: Some(error),
                xml: String::new(),
            }
        }

        let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&graph[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        let overlay = crate::graph::parse_sweep_tombstones(&sweep_tombstones);
        let state = crate::graph::build_state_tolerant(css, &overlay)?;
        if state.projection_degraded() {
            let e = editor_xml::XmlError::new(editor_xml::XmlErrorDetail::ProjectionDegraded);
            return Ok(failed(xml_error_info(&e)?).into_ffi()?);
        }
        let mut heads: Vec<editor_crdt::Dot> = state.graph().current_heads().copied().collect();
        heads.sort();
        match editor_xml::to_xml(&state, &heads) {
            Ok(xml) => Ok(XmlRender { error: None, xml }.into_ffi()?),
            Err(e) => Ok(failed(xml_error_info(&e)?).into_ffi()?),
        }
    }

    pub fn verify_xml(&self, xml: String) -> EditorResult<Complex<XmlVerdict>> {
        let error = match editor_xml::from_xml(&xml) {
            Ok(_) => None,
            Err(e) => Some(xml_error_info(&e)?),
        };
        Ok(XmlVerdict { error }.into_ffi()?)
    }

    pub fn outline_xml(
        &self,
        xml: String,
        under: String,
        depth: u32,
        offset: u32,
        limit: u32,
        full: bool,
    ) -> EditorResult<Complex<XmlOutline>> {
        fn failed(error: XmlErrorInfo) -> XmlOutline {
            XmlOutline {
                error: Some(error),
                head: None,
                rows: Vec::new(),
                total: 0,
                xml: None,
            }
        }

        let tree = match editor_xml::from_xml(&xml) {
            Ok(t) => t,
            Err(e) => return Ok(failed(xml_error_info(&e)?).into_ffi()?),
        };
        let under: editor_xml::Address = match under.parse() {
            Ok(a) => a,
            Err(detail) => {
                return Ok(failed(xml_error_info(&editor_xml::XmlError::new(detail))?).into_ffi()?);
            }
        };
        let scope = editor_xml::OutlineScope {
            under,
            depth,
            offset,
            limit,
            full,
        };
        match editor_xml::outline(&tree, &scope) {
            Ok(result) => Ok(outline_ffi(result).into_ffi()?),
            Err(e) => Ok(failed(xml_error_info(&e)?).into_ffi()?),
        }
    }

    pub fn edit_xml(&self, xml: String, ops_json: String) -> EditorResult<Complex<XmlEdit>> {
        fn failed(error: XmlOpErrorInfo) -> XmlEdit {
            XmlEdit {
                error: Some(error),
                xml: String::new(),
                affected: Vec::new(),
            }
        }

        let ops: Vec<editor_xml::Op> = match serde_json::from_str(&ops_json) {
            Ok(ops) => ops,
            Err(e) => {
                let info = xml_error_info(&editor_xml::XmlError::internal(format!("ops: {e}")))?;
                return Ok(failed(XmlOpErrorInfo {
                    op: None,
                    address: None,
                    info,
                })
                .into_ffi()?);
            }
        };
        match editor_xml::edit_file(&xml, &ops) {
            Ok(edited) => Ok(XmlEdit {
                error: None,
                xml: edited.xml,
                affected: edited.affected.into_iter().map(outline_ffi).collect(),
            }
            .into_ffi()?),
            Err(e) => Ok(failed(XmlOpErrorInfo {
                op: e.op.map(|i| i as u32),
                address: e.address,
                info: xml_error_info(&e.error)?,
            })
            .into_ffi()?),
        }
    }

    pub fn edit_from_xml(
        &self,
        graph: Vec<u8>,
        sweep_tombstones: Vec<String>,
        xml: String,
    ) -> EditorResult<Complex<XmlEditResult>> {
        fn failed(error: XmlErrorInfo) -> XmlEditResult {
            XmlEditResult {
                error: Some(error),
                bundle: Vec::new(),
                xml: String::new(),
                blocks_inserted: 0,
                blocks_deleted: 0,
                blocks_moved: 0,
                blocks_updated: 0,
                chars_inserted: 0,
                chars_deleted: 0,
            }
        }

        let tree = match editor_xml::from_xml(&xml) {
            Ok(t) => t,
            Err(e) => return Ok(failed(xml_error_info(&e)?).into_ffi()?),
        };
        let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&graph[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        let overlay = crate::graph::parse_sweep_tombstones(&sweep_tombstones);
        let full = crate::graph::build_state_tolerant(css, &overlay)?;
        let base_set: hashbrown::HashSet<editor_crdt::Dot> = tree.base.iter().copied().collect();
        let live: hashbrown::HashSet<editor_crdt::Dot> =
            full.graph().current_heads().copied().collect();
        // A file whose base is behind the live heads is refused rather than merged:
        // a move is an alias plus a re-insertion of the block's characters, so
        // merging it over a concurrent rewrite of the same block leaves the block
        // twice. The caller re-opens and edits the current document instead.
        if base_set != live {
            let e = editor_xml::XmlError::new(editor_xml::XmlErrorDetail::BaseNotInHistory);
            return Ok(failed(xml_error_info(&e)?).into_ffi()?);
        }
        let base_state = full;
        if base_state.projection_degraded() {
            let e = editor_xml::XmlError::new(editor_xml::XmlErrorDetail::ProjectionDegraded);
            return Ok(failed(xml_error_info(&e)?).into_ffi()?);
        }
        let outcome = match editor_xml::edit(base_state, &tree) {
            Ok(o) => o,
            Err(e) => return Ok(failed(xml_error_info(&e)?).into_ffi()?),
        };
        let new_css = outcome.state.graph().local_changesets_since(&base_set)?;
        let c = outcome.changed;
        // The saved file is written on the merge of what landed while it was open
        // with the edit, so a stale base never forks the file off the live branch.
        let (bundle, xml) = if new_css.is_empty() {
            (Vec::new(), String::new())
        } else {
            let post = outcome.state;
            let mut heads: Vec<editor_crdt::Dot> = post.graph().current_heads().copied().collect();
            heads.sort();
            let xml = match editor_xml::to_xml(&post, &heads) {
                Ok(xml) => xml,
                Err(e) => return Ok(failed(xml_error_info(&e)?).into_ffi()?),
            };
            let bundle = editor_codec::encode_changesets(
                editor_codec::ReencodableChangesets::from_local_ops(new_css),
            )
            .map_err(|e| FfiError::Serialization(e.to_string()))?;
            (bundle, xml)
        };
        Ok(XmlEditResult {
            error: None,
            bundle,
            xml,
            blocks_inserted: c.blocks_inserted,
            blocks_deleted: c.blocks_deleted,
            blocks_moved: c.blocks_moved,
            blocks_updated: c.blocks_updated,
            chars_inserted: c.chars_inserted,
            chars_deleted: c.chars_deleted,
        }
        .into_ffi()?)
    }

    pub fn zombie_dots(&self, graph: Vec<u8>) -> EditorResult<Vec<String>> {
        let css = editor_codec::decode_changeset_stream(&graph[..])
            .map_err(|e| FfiError::Deserialization(e.to_string()))?
            .into_graph_input();
        let state = crate::graph::build_state_tolerant(css, &[])
            .map_err(|e| FfiError::SweepFailed(e.to_string()))?;
        Ok(collect_zombie_dots(&state)
            .into_iter()
            .map(|d| d.to_string())
            .collect())
    }

    pub fn sweep(&self, graph: Vec<u8>) -> EditorResult<Vec<u8>> {
        let css = editor_codec::decode_changeset_stream(&graph[..])
            .map_err(|e| FfiError::Deserialization(e.to_string()))?
            .into_graph_input();
        let sweep_css = sweep_impl(css)?;
        if sweep_css.is_empty() {
            return Ok(Vec::new());
        }
        let bytes = editor_codec::encode_changesets(
            editor_codec::ReencodableChangesets::from_local_ops(sweep_css),
        )
        .map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    /// Returns the total ops count in a Changesets bundle. Used by push light validation.
    pub fn peek_changeset_ops_count(&self, bundle: Vec<u8>) -> EditorResult<u32> {
        let cs: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&bundle[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        let count: u32 = cs.iter().map(|c| c.ops.len() as u32).sum();
        Ok(count)
    }

    /// Verifies a PlainDoc's structural invariants by attempting to load it.
    pub fn verify_plain(&self, plain: Complex<editor_model::PlainDoc>) -> EditorResult<()> {
        let plain: editor_model::PlainDoc = plain.from_ffi()?;
        editor_state::State::from_plain(&plain)
            .map(|_| ())
            .map_err(|e| EditorError::General {
                msg: format!("{e:?}"),
            })
    }

    pub fn materialize(&self, changeset_payloads: Vec<u8>) -> EditorResult<Complex<Materialized>> {
        let cs: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&changeset_payloads[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        let state = crate::graph::build_state_tolerant(cs, &[])?;
        let plain = state.to_plain();
        let text = editor_state::doc_plain_text(&state.view());
        let projection_degraded = state.projection_degraded();
        Ok(Materialized {
            plain,
            text,
            projection_degraded,
        }
        .into_ffi()?)
    }

    pub fn validate_and_extract_text(&self, changeset_payloads: Vec<u8>) -> EditorResult<String> {
        let cs: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&changeset_payloads[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        let state = crate::graph::build_state_tolerant(cs, &[])?;
        Ok(editor_state::doc_plain_text(&state.view()))
    }

    /// Migration entry point: resolve a normalized v1 comment anchor against the
    /// document's current graph exactly as the shipping v1 reader would, then
    /// re-capture it as a v2 `StableSelection`. `StablePosition::resolve` is
    /// private, so the one-shot comment-anchor migration reaches it through here.
    /// `degraded` is `true` when either endpoint fell back to the offset-0 collapse.
    pub fn resolve_v1_selection(
        &self,
        graph: Vec<u8>,
        normalized_v1_json: String,
    ) -> EditorResult<Complex<ResolvedV1Selection>> {
        let v1: editor_state::StableSelectionV1 = serde_json::from_str(&normalized_v1_json)
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let css = editor_codec::decode_changeset_stream(&graph[..])
            .map_err(|e| FfiError::Deserialization(e.to_string()))?
            .into_graph_input();
        let state = crate::graph::build_state_tolerant(css, &[])?;
        let (selection, degraded) = editor_state::resolve_v1_selection(&state, &v1)
            .map_err(|msg| EditorError::General { msg })?;
        Ok(ResolvedV1Selection {
            selection,
            degraded,
        }
        .into_ffi()?)
    }

    /// 스냅샷 heads 시점 상태를 재구성해 원고 텍스트 좌표를 `StableSelection`으로 굳힌다.
    /// `expected_text`는 그 시점에 추출한 `prose_text_annotated()` — 다르면 좌표계가 어긋난 것이라 캡처하지 않는다.
    pub fn capture_prose_anchors(
        &self,
        graph: Vec<u8>,
        heads: Vec<u8>,
        expected_text: String,
        ranges: Complex<ProseRanges>,
    ) -> EditorResult<Complex<ProseAnchorCapture>> {
        let ranges: ProseRanges = ranges.from_ffi()?;
        let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&graph[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        let target: hashbrown::HashSet<editor_crdt::Dot> = editor_codec::decode_dots(&heads[..])
            .map_err(|e| FfiError::Deserialization(e.to_string()))?
            .into_iter()
            .collect();

        let state = if css.is_empty() {
            crate::graph::build_state_tolerant(css, &[])?
        } else {
            let graph = crate::graph::graph_tolerant(css);
            let current: hashbrown::HashSet<editor_crdt::Dot> =
                graph.current_heads().copied().collect();
            if current == target {
                let projected = editor_state::ProjectedState::from_graph_with_overlay(graph, &[])
                    .map_err(|e| EditorError::General {
                    msg: format!("{e:?}"),
                })?;
                editor_state::State::new(projected, None)
            } else {
                state_at_heads(&graph, &target, &[])?
            }
        };
        if state.projection_degraded() {
            return Err(EditorError::General {
                msg: "target projection is degraded".to_string(),
            });
        }

        let view = state.view();
        let prose = editor_state::prose_annotated(&view);
        if prose.text() != expected_text {
            return Ok(ProseAnchorCapture {
                text_matches: false,
                anchors: Vec::new(),
            }
            .into_ffi()?);
        }

        let anchors = ranges
            .ranges
            .iter()
            .enumerate()
            .filter_map(|(index, range)| {
                let sel =
                    prose.to_selection_utf16(&view, range.start as usize..range.end as usize)?;
                let resolved = sel.resolve(&view)?;
                if resolved.is_collapsed() {
                    return None;
                }
                Some(ProseAnchor {
                    index: index as u32,
                    selection: editor_state::StableSelection::capture(&sel, &view),
                    text: resolved.collect_text(),
                })
            })
            .collect();

        Ok(ProseAnchorCapture {
            text_matches: true,
            anchors,
        }
        .into_ffi()?)
    }
}

impl EditorServer {
    fn collect_fold_inner(
        &self,
        existing: Vec<u8>,
        packed_bundles: Vec<u8>,
    ) -> EditorResult<CollectResult> {
        let existing_cs: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&existing[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        let bundles = crate::graph::decode_length_prefixed(&packed_bundles)?;

        let mut state = editor_state::State::from_changesets(existing_cs, None)?;
        let base_char_count = doc_count(&self.icu, &state.view());

        let mut statuses: Vec<BundleStatus> = Vec::with_capacity(bundles.len());
        let mut char_counts: Vec<u32> = Vec::with_capacity(bundles.len());
        let mut entry_heads: Vec<serde_bytes::ByteBuf> = Vec::with_capacity(bundles.len());
        let mut gross_insertions: Vec<u32> = Vec::with_capacity(bundles.len());
        let mut gross_deletions: Vec<u32> = Vec::with_capacity(bundles.len());
        let mut last = base_char_count;

        let encode_heads = |state: &editor_state::State| -> EditorResult<Vec<u8>> {
            let heads: Vec<editor_crdt::Dot> = state.graph().current_heads().copied().collect();
            if heads.is_empty() {
                return Ok(Vec::new());
            }
            let bytes = editor_codec::encode_dots(&heads)
                .map_err(|e| FfiError::Serialization(e.to_string()))?;
            Ok(bytes)
        };

        let mut last_heads = encode_heads(&state)?;

        for bundle in bundles {
            let mut ins = 0u32;
            let mut del = 0u32;
            let status = match editor_codec::decode_changeset_stream(&bundle[..]) {
                Ok(decoded) => match state.receive_remote_changesets(decoded.into_graph_input()) {
                    Ok((next, ops)) if !ops.is_empty() => {
                        state = next;
                        for op in &ops {
                            match &op.payload {
                                editor_model::EditOp::Seq(editor_crdt::ListOp::Ins {
                                    item: editor_model::SeqItem::Char(_),
                                    ..
                                }) => ins += 1,
                                editor_model::EditOp::Seq(editor_crdt::ListOp::Del {
                                    len, ..
                                }) => del += *len as u32,
                                editor_model::EditOp::Seq(editor_crdt::ListOp::Undel {
                                    del: target,
                                }) => {
                                    if let Some(target_op) = state.graph().get(target)
                                        && let editor_model::EditOp::Seq(editor_crdt::ListOp::Del {
                                            len,
                                            ..
                                        }) = &target_op.payload
                                    {
                                        ins += *len as u32;
                                    }
                                }
                                _ => {}
                            }
                        }
                        BundleStatus::Applied
                    }
                    Ok(_) => BundleStatus::Duplicate,
                    Err(_) => BundleStatus::Failed,
                },
                Err(_) => BundleStatus::Failed,
            };
            if status == BundleStatus::Applied {
                last = doc_count(&self.icu, &state.view());
                last_heads = encode_heads(&state)?;
            }
            statuses.push(status);
            char_counts.push(last);
            entry_heads.push(serde_bytes::ByteBuf::from(last_heads.clone()));
            gross_insertions.push(ins);
            gross_deletions.push(del);
        }

        let plain = state.to_plain();
        let text = editor_state::doc_plain_text(&state.view());
        let totality_violations = collect_zombie_dots(&state).len() as u32;
        let projection_degraded = state.projection_degraded();

        Ok(CollectResult {
            heads: last_heads,
            statuses,
            char_counts,
            entry_heads,
            gross_insertions,
            gross_deletions,
            base_char_count,
            plain,
            text,
            totality_violations,
            projection_degraded,
        })
    }
}

/// Builds a `State` whose graph contains only the ops that are ancestors of
/// (or equal to) `heads`. Used by `revert` to project the document at a past
/// point without requiring a bespoke `from_op_graph_at` on the new model.
fn state_at_heads(
    graph: &editor_crdt::OpGraph<editor_model::EditOp>,
    heads: &hashbrown::HashSet<editor_crdt::Dot>,
    overlay: &[editor_crdt::Dot],
) -> Result<editor_state::State, FfiError> {
    for h in heads {
        if !graph.contains(h) {
            return Err(FfiError::RevertFailed(format!(
                "unknown target head: {h:?}"
            )));
        }
    }
    let ancestry = graph.ancestry_of(heads);
    let ordered = graph.topo_sort(&ancestry);
    let css: Vec<editor_crdt::Changeset<editor_model::EditOp>> = ordered
        .into_iter()
        .map(|op| editor_crdt::Changeset { ops: vec![op] })
        .collect();
    editor_state::State::from_changesets_with_overlay(css, overlay, None)
        .map_err(|e| FfiError::RevertFailed(e.to_string()))
}

fn collect_zombie_dots(state: &editor_state::State) -> Vec<editor_crdt::Dot> {
    let ps = &state.projected;
    let reachable: hashbrown::HashSet<editor_crdt::Dot> = ps
        .subtree_real_dots(editor_crdt::Dot::ROOT)
        .into_iter()
        .collect();
    let mut zombies = Vec::new();
    for op in state.graph().iter_all() {
        if !matches!(
            op.payload,
            editor_model::EditOp::Seq(editor_crdt::ListOp::Ins { .. })
        ) {
            continue;
        }
        if reachable.contains(&op.id) {
            continue;
        }
        if ps.seq_visible_pos(op.id).is_some() {
            zombies.push(op.id);
        }
    }
    zombies
}

fn sweep_impl(
    css: Vec<editor_crdt::Changeset<editor_model::EditOp>>,
) -> Result<Vec<editor_crdt::Changeset<editor_model::EditOp>>, FfiError> {
    let state = crate::graph::build_state_tolerant(css, &[])
        .map_err(|e| FfiError::SweepFailed(e.to_string()))?;
    let current_heads: hashbrown::HashSet<editor_crdt::Dot> =
        state.graph().current_heads().copied().collect();
    let zombies = collect_zombie_dots(&state);
    if zombies.is_empty() {
        return Ok(Vec::new());
    }
    let ops = editor_transaction::delete_dots_ops(&state.projected, &zombies);
    let mut graph = state.graph().clone();
    for op in ops {
        graph
            .add_mut(op)
            .map_err(|e| FfiError::SweepFailed(e.to_string()))?;
    }
    graph.commit_mut();
    graph
        .local_changesets_since(&current_heads)
        .map_err(|e| FfiError::SweepFailed(e.to_string()))
}

fn doc_count(icu: &editor_resource::IcuResources, view: &editor_model::DocView<'_>) -> u32 {
    let text = editor_state::doc_plain_text(view);
    editor_resource::count_text(&text, &icu.segmenters.grapheme, &icu.general_category)
        .with_whitespace
}

#[cfg(test)]
impl EditorServer {
    pub(crate) fn new_test() -> Self {
        Self {
            icu: editor_resource::IcuResources::new_test(),
        }
    }
}

use crate::doc_builder::build_default_doc;

#[cfg(test)]
mod tests {
    use editor_crdt::{Changeset, Dot, ListOp, Op};
    use editor_model::{EditOp, SeqItem};

    use super::*;
    use crate::error::EditorError;

    fn dummy_payload() -> EditOp {
        EditOp::Seq(ListOp::Ins {
            pos: 0,
            item: SeqItem::Char('x'),
        })
    }

    fn enc_css(css: &[Changeset<EditOp>]) -> Vec<u8> {
        editor_codec::encode_changesets(editor_codec::ReencodableChangesets::from_local_ops(
            css.to_vec(),
        ))
        .unwrap()
    }
    fn dec_css(b: &[u8]) -> Vec<Changeset<EditOp>> {
        editor_codec::decode_changeset_stream(b)
            .unwrap()
            .into_graph_input()
    }
    fn enc_dots(dots: &[Dot]) -> Vec<u8> {
        editor_codec::encode_dots(dots).unwrap()
    }
    fn dec_dots(b: &[u8]) -> Vec<Dot> {
        editor_codec::decode_dots(b).unwrap()
    }

    #[cfg(feature = "wasm-server")]
    fn load_test_font() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/Pretendard-Regular.ttf"
        ))
        .expect("test font not found")
    }

    #[test]
    fn apply_concatenates_distinct_changesets() {
        // cs_b causally follows cs_a so the wire format's implicit-prev round-trips cleanly.
        let cs_a = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let cs_b = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(2, 0),
                parents: vec![Dot::new(1, 0)],
                payload: dummy_payload(),
            }],
        };
        let server = EditorServer::new_test();
        let merged_bytes = server
            .apply(
                enc_css(std::slice::from_ref(&cs_a)),
                enc_css(std::slice::from_ref(&cs_b)),
            )
            .unwrap();
        let merged = dec_css(&merged_bytes);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].ops[0].id, cs_a.ops[0].id);
        assert_eq!(merged[1].ops[0].id, cs_b.ops[0].id);
        assert_eq!(merged[1].ops[0].parents, vec![Dot::new(1, 0)]);
    }

    #[test]
    fn apply_skips_full_duplicate_changesets() {
        let cs = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let server = EditorServer::new_test();
        let merged_bytes = server
            .apply(
                enc_css(std::slice::from_ref(&cs)),
                enc_css(std::slice::from_ref(&cs)),
            )
            .unwrap();
        let merged = dec_css(&merged_bytes);
        assert_eq!(merged, vec![cs]);
    }

    #[test]
    fn apply_dedups_duplicates_within_new_payload() {
        // Encode the same cs twice as independent bundles so the wire format
        // doesn't inject implicit-prev parents on the second copy.
        let cs = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(7, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let server = EditorServer::new_test();
        // existing already has cs; new payload re-sends the same cs
        let merged_bytes = server
            .apply(
                enc_css(std::slice::from_ref(&cs)),
                enc_css(std::slice::from_ref(&cs)),
            )
            .unwrap();
        let merged = dec_css(&merged_bytes);
        assert_eq!(merged.len(), 1, "duplicate should be silently dropped");
        assert_eq!(merged[0].ops[0].id, Dot::new(7, 0));
    }

    #[test]
    fn apply_rejects_causally_broken_payload() {
        let parent = Op {
            id: Dot::new(1, 0),
            parents: vec![],
            payload: dummy_payload(),
        };
        let child = Op {
            id: Dot::new(1, 1),
            parents: vec![parent.id],
            payload: dummy_payload(),
        };
        let parent_cs = Changeset::<EditOp> { ops: vec![parent] };
        let child_cs = Changeset::<EditOp> { ops: vec![child] };
        let server = EditorServer::new_test();
        let result = server.apply(enc_css(&[]), enc_css(&[child_cs, parent_cs]));
        assert!(matches!(
            result,
            Err(EditorError::Ffi(FfiError::CausalOrderViolation { .. }))
        ));
    }

    #[test]
    fn apply_accepts_correctly_ordered_chain() {
        let parent = Op {
            id: Dot::new(1, 0),
            parents: vec![],
            payload: dummy_payload(),
        };
        let child = Op {
            id: Dot::new(1, 1),
            parents: vec![parent.id],
            payload: dummy_payload(),
        };
        let parent_cs = Changeset::<EditOp> { ops: vec![parent] };
        let child_cs = Changeset::<EditOp> { ops: vec![child] };
        let server = EditorServer::new_test();
        let merged_bytes = server
            .apply(
                enc_css(&[]),
                enc_css(&[parent_cs.clone(), child_cs.clone()]),
            )
            .unwrap();
        let merged = dec_css(&merged_bytes);
        assert_eq!(merged, vec![parent_cs, child_cs]);
    }

    #[test]
    fn apply_accepts_intra_cs_parent_chain() {
        let op1 = Op {
            id: Dot::new(5, 0),
            parents: vec![],
            payload: dummy_payload(),
        };
        let op2 = Op {
            id: Dot::new(5, 1),
            parents: vec![op1.id],
            payload: dummy_payload(),
        };
        let cs = Changeset::<EditOp> {
            ops: vec![op1, op2],
        };
        let server = EditorServer::new_test();
        let merged_bytes = server
            .apply(enc_css(&[]), enc_css(std::slice::from_ref(&cs)))
            .unwrap();
        let merged = dec_css(&merged_bytes);
        assert_eq!(merged, vec![cs]);
    }

    #[test]
    fn apply_rejects_self_referencing_op() {
        let dot = Dot::new(33, 0);
        let bad = Op {
            id: dot,
            parents: vec![dot],
            payload: dummy_payload(),
        };
        let cs = Changeset::<EditOp> { ops: vec![bad] };
        let server = EditorServer::new_test();
        let result = server.apply(enc_css(&[]), enc_css(&[cs]));
        assert!(matches!(
            result,
            Err(EditorError::Ffi(FfiError::CausalOrderViolation { .. }))
        ));
    }

    #[test]
    fn apply_rejects_non_first_dot_reuse() {
        let x = Dot::new(20, 0);
        let cs_a = Changeset::<EditOp> {
            ops: vec![Op {
                id: x,
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let new_first = Op {
            id: Dot::new(21, 0),
            parents: vec![x],
            payload: dummy_payload(),
        };
        let new_reuse = Op {
            id: x,
            parents: vec![new_first.id],
            payload: dummy_payload(),
        };
        let cs_bad = Changeset::<EditOp> {
            ops: vec![new_first, new_reuse],
        };
        let server = EditorServer::new_test();
        let result = server.apply(enc_css(&[cs_a]), enc_css(&[cs_bad]));
        assert!(matches!(
            result,
            Err(EditorError::Ffi(FfiError::CausalOrderViolation { .. }))
        ));
    }

    #[test]
    fn missing_for_returns_only_missing_changesets() {
        // cs_b causally follows cs_a so the wire format's implicit-prev round-trips cleanly.
        let cs_a = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let cs_b = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(2, 0),
                parents: vec![Dot::new(1, 0)],
                payload: dummy_payload(),
            }],
        };
        // Remote peer knows cs_a but not cs_b
        let known_heads = vec![Dot::new(1, 0)];

        let server = EditorServer::new_test();
        let missing_bytes = server
            .missing_for(
                enc_css(&[cs_a.clone(), cs_b.clone()]),
                enc_dots(&known_heads),
            )
            .unwrap();
        let missing = dec_css(&missing_bytes);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].ops[0].id, cs_b.ops[0].id);
        assert_eq!(missing[0].ops[0].parents, vec![Dot::new(1, 0)]);
    }

    #[test]
    fn heads_returns_dot_set() {
        let cs_a = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let server = EditorServer::new_test();
        let heads_bytes = server.heads(enc_css(std::slice::from_ref(&cs_a))).unwrap();
        let heads = dec_dots(&heads_bytes);
        assert_eq!(heads, vec![Dot::new(1, 0)]);
    }

    fn pack(blobs: &[Vec<u8>]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&(blobs.len() as u32).to_le_bytes());
        for b in blobs {
            out.extend_from_slice(&(b.len() as u32).to_le_bytes());
            out.extend_from_slice(b);
        }
        out
    }

    #[test]
    fn count_characters_literal_grapheme_semantics() {
        let server = EditorServer::new_test();
        assert_eq!(server.count_characters("a  b".into()), 4);
        assert_eq!(server.count_characters(" abc ".into()), 5);
        assert_eq!(server.count_characters("a\n\nb".into()), 4);
        assert_eq!(
            server.count_characters("👨\u{200D}👩\u{200D}👧\u{200D}👦".into()),
            1
        );
        assert_eq!(
            server.count_characters("\u{1112}\u{1161}\u{11AB}".into()),
            1
        );
    }

    #[test]
    fn collect_fold_classifies_applied_duplicate_and_failed() {
        let cs_a = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let cs_b = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(2, 0),
                parents: vec![Dot::new(1, 0)],
                payload: dummy_payload(),
            }],
        };
        let server = EditorServer::new_test();
        let existing = enc_css(std::slice::from_ref(&cs_a));
        let applied_bundle = enc_css(std::slice::from_ref(&cs_b));
        let malformed_bundle = vec![0xFF, 0x00, 0x01];

        let packed = pack(&[applied_bundle.clone(), applied_bundle, malformed_bundle]);
        let result = server.collect_fold(existing, packed).unwrap();

        assert_eq!(
            result.statuses,
            vec![
                BundleStatus::Applied,
                BundleStatus::Duplicate,
                BundleStatus::Failed,
            ]
        );
        assert!(
            result.char_counts[0] == result.char_counts[1],
            "duplicate must not change the character count"
        );
    }

    /// The "Failed" case above is a decode failure (garbage bytes) — the outer
    /// `decode_changeset_stream` match arm. This covers the *inner* failure
    /// path: a bundle that decodes fine but whose op depends on a dot that is
    /// nowhere in the graph (a real causal gap, not a malformed payload) —
    /// `receive_remote_changesets` must reject it and `collect_fold` must
    /// classify it as `Failed`, not silently apply a partial result.
    #[test]
    fn collect_fold_reports_failed_for_causally_broken_bundle() {
        let cs_a = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let orphan_bundle = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(3, 0),
                parents: vec![Dot::new(9, 99)],
                payload: dummy_payload(),
            }],
        };
        let server = EditorServer::new_test();
        let existing = enc_css(std::slice::from_ref(&cs_a));
        let packed = pack(&[enc_css(std::slice::from_ref(&orphan_bundle))]);

        let result = server.collect_fold(existing, packed).unwrap();
        assert_eq!(result.statuses, vec![BundleStatus::Failed]);
        assert_eq!(
            result.char_counts[0], result.base_char_count,
            "a rejected bundle must not move the character count"
        );
    }

    /// A bundle whose changeset shares its *first* dot with an already-known
    /// changeset but is not verbatim-identical (a new second op grouped onto
    /// it) is neither a clean apply nor an idempotent re-delivery — the graph
    /// rejects it as `PartialDuplicate`, and `collect_fold` must surface that
    /// as `Failed` rather than silently accepting the overlapping half.
    #[test]
    fn collect_fold_reports_failed_for_partial_duplicate_bundle() {
        let cs_a = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let partial_bundle = Changeset::<EditOp> {
            ops: vec![
                Op {
                    id: Dot::new(1, 0),
                    parents: vec![],
                    payload: dummy_payload(),
                },
                Op {
                    id: Dot::new(5, 0),
                    parents: vec![Dot::new(1, 0)],
                    payload: dummy_payload(),
                },
            ],
        };
        let server = EditorServer::new_test();
        let existing = enc_css(std::slice::from_ref(&cs_a));
        let packed = pack(&[enc_css(std::slice::from_ref(&partial_bundle))]);

        let result = server.collect_fold(existing, packed).unwrap();
        assert_eq!(result.statuses, vec![BundleStatus::Failed]);
        assert_eq!(
            result.char_counts[0], result.base_char_count,
            "a rejected bundle must not move the character count"
        );
    }

    fn seq_char(actor: u64, counter: u64, parents: &[Dot], pos: usize, ch: char) -> Op<EditOp> {
        Op {
            id: Dot::new(actor, counter),
            parents: parents.to_vec(),
            payload: EditOp::Seq(ListOp::Ins {
                pos,
                item: SeqItem::Char(ch),
            }),
        }
    }

    #[test]
    fn collect_fold_reports_per_entry_heads_and_gross() {
        let server = EditorServer::new_test();
        let para = Op {
            id: Dot::new(1, 0),
            parents: vec![],
            payload: EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::Block {
                    node_type: editor_model::NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }),
        };
        let a = seq_char(1, 1, &[para.id], 1, 'a');
        let b = seq_char(1, 2, &[a.id], 2, 'b');
        let b_id = b.id;
        let del_id = Dot::new(1, 3);
        let del = Op {
            id: del_id,
            parents: vec![b_id],
            payload: EditOp::Seq(ListOp::Del { pos: 1, len: 2 }),
        };
        let bundle1 = enc_css(&[Changeset {
            ops: vec![para, a, b],
        }]);
        let bundle2 = enc_css(&[Changeset { ops: vec![del] }]);
        let dup = bundle2.clone();

        let result = server
            .collect_fold_inner(Vec::new(), pack(&[bundle1, bundle2, dup]))
            .unwrap();

        assert_eq!(result.gross_insertions, vec![2, 0, 0]);
        assert_eq!(result.gross_deletions, vec![0, 2, 0]);
        assert_eq!(result.entry_heads.len(), 3);
        assert_eq!(dec_dots(&result.entry_heads[0]), vec![b_id]);
        assert_eq!(dec_dots(&result.entry_heads[1]), vec![del_id]);
        assert_eq!(result.entry_heads[2], result.entry_heads[1]);
        assert_eq!(result.entry_heads[2], result.heads);
    }

    #[test]
    fn collect_fold_counts_undel_restore_as_gross_insertion() {
        let server = EditorServer::new_test();
        let para = Op {
            id: Dot::new(1, 0),
            parents: vec![],
            payload: EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::Block {
                    node_type: editor_model::NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }),
        };
        let a = seq_char(1, 1, &[para.id], 1, 'a');
        let b = seq_char(1, 2, &[a.id], 2, 'b');
        let c = seq_char(1, 3, &[b.id], 3, 'c');
        let c_id = c.id;
        let del_id = Dot::new(1, 4);
        let del = Op {
            id: del_id,
            parents: vec![c_id],
            payload: EditOp::Seq(ListOp::Del { pos: 1, len: 2 }),
        };
        let undel = Op {
            id: Dot::new(1, 5),
            parents: vec![del_id],
            payload: EditOp::Seq(ListOp::Undel { del: del_id }),
        };
        let bundle1 = enc_css(&[Changeset {
            ops: vec![para, a, b, c],
        }]);
        let bundle2 = enc_css(&[Changeset { ops: vec![del] }]);
        let bundle3 = enc_css(&[Changeset { ops: vec![undel] }]);

        let result = server
            .collect_fold_inner(Vec::new(), pack(&[bundle1, bundle2, bundle3]))
            .unwrap();

        assert_eq!(
            result.statuses,
            vec![
                BundleStatus::Applied,
                BundleStatus::Applied,
                BundleStatus::Applied,
            ]
        );
        assert_eq!(result.gross_insertions, vec![3, 0, 2]);
        assert_eq!(result.gross_deletions, vec![0, 2, 0]);
        assert_eq!(result.char_counts, vec![3, 1, 3]);
    }

    #[test]
    fn collect_fold_counts_undel_paired_with_its_del_in_one_bundle() {
        let server = EditorServer::new_test();
        let para = Op {
            id: Dot::new(1, 0),
            parents: vec![],
            payload: EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::Block {
                    node_type: editor_model::NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }),
        };
        let a = seq_char(1, 1, &[para.id], 1, 'a');
        let b = seq_char(1, 2, &[a.id], 2, 'b');
        let c = seq_char(1, 3, &[b.id], 3, 'c');
        let c_id = c.id;
        let del_id = Dot::new(1, 4);
        let del = Op {
            id: del_id,
            parents: vec![c_id],
            payload: EditOp::Seq(ListOp::Del { pos: 1, len: 2 }),
        };
        let undel = Op {
            id: Dot::new(1, 5),
            parents: vec![del_id],
            payload: EditOp::Seq(ListOp::Undel { del: del_id }),
        };
        let bundle1 = enc_css(&[Changeset {
            ops: vec![para, a, b, c],
        }]);
        let paired = enc_css(&[Changeset {
            ops: vec![del, undel],
        }]);

        let result = server
            .collect_fold_inner(Vec::new(), pack(&[bundle1, paired]))
            .unwrap();

        assert_eq!(
            result.statuses,
            vec![BundleStatus::Applied, BundleStatus::Applied]
        );
        assert_eq!(result.gross_insertions, vec![3, 2]);
        assert_eq!(result.gross_deletions, vec![0, 2]);
        assert_eq!(result.char_counts, vec![3, 3]);
    }

    #[test]
    fn collect_fold_counts_no_totality_violations_when_dead_parent_marker_is_revived() {
        let server = EditorServer::new_test();
        let existing = enc_css(&zombie_css());
        let result = server.collect_fold(existing, pack(&[])).unwrap();
        assert_eq!(
            result.totality_violations, 0,
            "revival attaches the dead-parent marker under Root, so nothing is unreachable"
        );
    }

    /// A single flat `TableCell` under Root must be wrapped back into
    /// `Table > TableRow` to satisfy the schema — several deterministic repairs,
    /// enough to latch the cap once the budget is lowered to 1.
    fn degraded_prone_css() -> Vec<editor_crdt::Changeset<editor_model::EditOp>> {
        let d = |c| Dot::new(1, c);
        vec![Changeset {
            ops: vec![
                Op {
                    id: d(0),
                    parents: vec![],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 0,
                        item: SeqItem::Block {
                            node_type: editor_model::NodeType::TableCell,
                            parents: vec![Dot::ROOT],
                            attrs: vec![],
                        },
                    }),
                },
                Op {
                    id: d(1),
                    parents: vec![d(0)],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 1,
                        item: SeqItem::Char('a'),
                    }),
                },
            ],
        }]
    }

    #[test]
    fn collect_fold_reports_projection_degraded_when_the_budget_caps() {
        let server = EditorServer::new_test();
        let existing = enc_css(&degraded_prone_css());

        let clean = server.collect_fold(existing.clone(), pack(&[])).unwrap();
        assert!(
            !clean.projection_degraded,
            "the fixture repairs cleanly at full budget"
        );

        let degraded = {
            let _guard = editor_model::override_repair_budget(1);
            server.collect_fold(existing, pack(&[])).unwrap()
        };
        assert!(
            degraded.projection_degraded,
            "budget 1 latches the cap, so collect records the snapshot as degraded"
        );
    }

    #[test]
    fn materialize_reports_projection_degraded_when_the_budget_caps() {
        let server = EditorServer::new_test();
        let graph = enc_css(&degraded_prone_css());
        let materialized = {
            let _guard = editor_model::override_repair_budget(1);
            server.materialize(graph).unwrap()
        };
        assert!(materialized.projection_degraded);
    }

    #[test]
    fn revert_refuses_a_degraded_target() {
        let server = EditorServer::new_test();
        let graph = enc_css(&degraded_prone_css());
        let target = server.heads(graph.clone()).unwrap();

        let result = {
            let _guard = editor_model::override_repair_budget(1);
            server.revert(graph, target, Vec::new())
        };
        assert!(
            matches!(result, Err(EditorError::Ffi(FfiError::RevertFailed(_)))),
            "revert must refuse a target whose projection is degraded"
        );
    }

    #[test]
    fn collect_fold_reports_zero_totality_violations_for_clean_document() {
        let cs = Changeset::<EditOp> {
            ops: vec![
                Op {
                    id: Dot::new(4, 0),
                    parents: vec![],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 0,
                        item: SeqItem::Block {
                            node_type: editor_model::NodeType::Paragraph,
                            parents: vec![Dot::ROOT],
                            attrs: vec![],
                        },
                    }),
                },
                Op {
                    id: Dot::new(4, 1),
                    parents: vec![Dot::new(4, 0)],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 1,
                        item: SeqItem::Char('a'),
                    }),
                },
            ],
        };
        let server = EditorServer::new_test();
        let existing = enc_css(std::slice::from_ref(&cs));
        let result = server.collect_fold(existing, pack(&[])).unwrap();
        assert_eq!(result.totality_violations, 0);
    }

    #[test]
    fn collect_fold_counts_graphemes_of_the_extracted_document() {
        let ins = |id: Dot, parents: Vec<Dot>, pos: usize, item: SeqItem| Op {
            id,
            parents,
            payload: EditOp::Seq(ListOp::Ins { pos, item }),
        };
        let paragraph = || SeqItem::Block {
            node_type: editor_model::NodeType::Paragraph,
            parents: vec![Dot::ROOT],
            attrs: vec![],
        };
        let d = |c| Dot::new(6, c);
        let e = |c| Dot::new(7, c);

        let existing = enc_css(&[Changeset::<EditOp> {
            ops: vec![
                ins(d(0), vec![], 0, paragraph()),
                ins(d(1), vec![d(0)], 1, SeqItem::Char('a')),
                ins(d(2), vec![d(1)], 2, SeqItem::Char('👨')),
                ins(d(3), vec![d(2)], 3, SeqItem::Char('\u{200D}')),
                ins(d(4), vec![d(3)], 4, SeqItem::Char('👩')),
                ins(d(5), vec![d(4)], 5, SeqItem::Char('\u{200D}')),
                ins(d(6), vec![d(5)], 6, SeqItem::Char('👧')),
                ins(d(7), vec![d(6)], 7, SeqItem::Char('\u{200D}')),
                ins(d(8), vec![d(7)], 8, SeqItem::Char('👦')),
                ins(
                    d(9),
                    vec![d(8)],
                    9,
                    SeqItem::Atom(editor_model::AtomLeaf::Tab),
                ),
                ins(d(10), vec![d(9)], 10, SeqItem::Char('b')),
                ins(d(11), vec![d(10)], 11, paragraph()),
                ins(d(12), vec![d(11)], 12, SeqItem::Char('c')),
                ins(
                    d(13),
                    vec![d(12)],
                    13,
                    SeqItem::Atom(editor_model::AtomLeaf::HardBreak),
                ),
                ins(d(14), vec![d(13)], 14, SeqItem::Char('d')),
            ],
        }]);
        let bundle = enc_css(&[Changeset::<EditOp> {
            ops: vec![
                ins(e(0), vec![d(14)], 15, SeqItem::Char('\u{1112}')),
                ins(e(1), vec![e(0)], 16, SeqItem::Char('\u{1161}')),
                ins(e(2), vec![e(1)], 17, SeqItem::Char('\u{11AB}')),
            ],
        }]);

        let server = EditorServer::new_test();
        let result = server.collect_fold(existing, pack(&[bundle])).unwrap();

        assert_eq!(result.statuses, vec![BundleStatus::Applied]);
        assert_eq!(
            result.base_char_count, 8,
            "tab, hard break and the paragraph separator each count as one character, and the seven code points of the family emoji collapse into one"
        );
        assert_eq!(
            result.text, "a👨\u{200D}👩\u{200D}👧\u{200D}👦\tb\nc\nd\u{1112}\u{1161}\u{11AB}",
            "the counted text keeps tabs, hard breaks and paragraph separators"
        );
        assert_eq!(
            result.char_counts,
            vec![9],
            "the appended syllable is three code points but one grapheme"
        );
        assert_eq!(
            server.extract_text(result.plain.clone()).unwrap(),
            result.text
        );
    }

    #[test]
    fn consolidate_merges_stream_preserving_changesets_and_heads() {
        let cs_a = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let cs_b = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(2, 0),
                parents: vec![Dot::new(1, 0)],
                payload: dummy_payload(),
            }],
        };
        let cs_c = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(3, 0),
                parents: vec![Dot::new(2, 0)],
                payload: dummy_payload(),
            }],
        };
        let server = EditorServer::new_test();
        let stream = [
            enc_css(std::slice::from_ref(&cs_a)),
            enc_css(std::slice::from_ref(&cs_b)),
            enc_css(std::slice::from_ref(&cs_c)),
        ]
        .concat();

        let result = server.consolidate(stream.clone()).unwrap();
        let payload = result.payload.expect("3 bundles should be merged");
        assert_eq!(result.consumed, 3);
        assert_eq!(result.consumed_bytes as usize, stream.len());

        let merged = dec_css(&payload);
        let original = dec_css(&stream);
        assert_eq!(
            merged, original,
            "changeset count and contents must be preserved"
        );

        let base = enc_css(&[]);
        let via_original = server.apply(base.clone(), stream).unwrap();
        let via_consolidated = server.apply(base, payload).unwrap();
        assert_eq!(
            dec_css(&via_original),
            dec_css(&via_consolidated),
            "apply result must match"
        );
        assert_eq!(
            dec_dots(&server.heads(via_original).unwrap()),
            dec_dots(&server.heads(via_consolidated).unwrap()),
            "heads must match"
        );
    }

    #[test]
    fn update_heads_matches_merged_graph_frontier() {
        // Two concurrent branches off cs_a, then a merge op — exercises head
        // replacement, concurrent-head accumulation, and multi-parent removal.
        let cs_a = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let cs_b = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(2, 0),
                parents: vec![Dot::new(1, 0)],
                payload: dummy_payload(),
            }],
        };
        let cs_c = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(3, 0),
                parents: vec![Dot::new(1, 0)],
                payload: dummy_payload(),
            }],
        };
        let cs_d = Changeset::<EditOp> {
            ops: vec![Op {
                id: Dot::new(2, 1),
                parents: vec![Dot::new(2, 0), Dot::new(3, 0)],
                payload: dummy_payload(),
            }],
        };
        let server = EditorServer::new_test();

        let mut graph = enc_css(std::slice::from_ref(&cs_a));
        let mut live = server.heads(graph.clone()).unwrap();
        for cs in [&cs_b, &cs_c, &cs_d] {
            let bundle = enc_css(std::slice::from_ref(cs));
            live = server.update_heads(live, bundle.clone()).unwrap();
            graph = server.apply(graph, bundle).unwrap();
            let full = server.heads(graph.clone()).unwrap();
            assert_eq!(dec_dots(&live), dec_dots(&full));
        }

        // Duplicate redelivery is a no-op.
        let dup = enc_css(std::slice::from_ref(&cs_d));
        let redelivered = server.update_heads(live.clone(), dup).unwrap();
        assert_eq!(dec_dots(&redelivered), dec_dots(&live));
    }

    #[test]
    fn apply_rejects_same_dot_different_content() {
        let dot = Dot::new(11, 0);
        let payload_a = EditOp::Seq(ListOp::Ins {
            pos: 0,
            item: SeqItem::Char('a'),
        });
        let payload_b = EditOp::Seq(ListOp::Ins {
            pos: 0,
            item: SeqItem::Char('b'),
        });
        let cs_v1 = Changeset::<EditOp> {
            ops: vec![Op {
                id: dot,
                parents: vec![],
                payload: payload_a,
            }],
        };
        let cs_v2 = Changeset::<EditOp> {
            ops: vec![Op {
                id: dot,
                parents: vec![],
                payload: payload_b,
            }],
        };
        let server = EditorServer::new_test();
        let result = server.apply(enc_css(&[cs_v1]), enc_css(&[cs_v2]));
        assert!(matches!(
            result,
            Err(EditorError::Ffi(FfiError::CausalOrderViolation { .. }))
        ));
    }

    #[cfg(feature = "wasm-server")]
    #[test]
    fn outline_text_to_svg_forwards_svg_document() {
        let server = EditorServer::new_test();
        let svg = server
            .outline_text_to_svg(load_test_font(), "A".to_string())
            .unwrap();
        assert!(svg.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg""#));
        assert!(svg.contains("<path d=\""));
    }

    #[cfg(feature = "wasm-server")]
    #[test]
    fn outline_text_to_svg_rejects_invalid_font_data() {
        let server = EditorServer::new_test();
        let result = server.outline_text_to_svg(vec![0, 1, 2, 3], "A".to_string());
        assert!(result.is_err());
    }

    fn make_state_with_text(text: &str) -> editor_state::State {
        let mut state = editor_state::State::empty();
        for (i, ch) in text.chars().enumerate() {
            state
                .projected_mut()
                .apply(EditOp::Seq(ListOp::Ins {
                    pos: 1 + i,
                    item: SeqItem::Char(ch),
                }))
                .unwrap();
        }
        state.projected_mut().commit();
        state
    }

    #[test]
    fn extract_text_from_plain_doc() {
        let state = make_state_with_text("hello world");
        let plain = state.to_plain();
        let server = EditorServer::new_test();
        let result = server.extract_text(plain).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn extract_text_contract_for_migration() {
        use std::collections::BTreeMap;

        use editor_model::{
            PlainDoc, PlainHardBreakNode, PlainNode, PlainNodeEntry, PlainParagraphNode,
            PlainRootNode, PlainTabNode, PlainTextNode,
        };

        fn entry(node: PlainNode, children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
            PlainNodeEntry {
                node,
                modifiers: BTreeMap::new(),
                carry: Vec::new(),
                children,
            }
        }

        let plain = PlainDoc {
            root: entry(
                PlainNode::Root(PlainRootNode::default()),
                vec![
                    entry(
                        PlainNode::Paragraph(PlainParagraphNode {}),
                        vec![
                            entry(PlainNode::Text(PlainTextNode { text: "a".into() }), vec![]),
                            entry(PlainNode::Tab(PlainTabNode {}), vec![]),
                            entry(PlainNode::Text(PlainTextNode { text: "b".into() }), vec![]),
                        ],
                    ),
                    entry(
                        PlainNode::Paragraph(PlainParagraphNode {}),
                        vec![
                            entry(PlainNode::Text(PlainTextNode { text: "c".into() }), vec![]),
                            entry(PlainNode::HardBreak(PlainHardBreakNode {}), vec![]),
                            entry(PlainNode::Text(PlainTextNode { text: "d".into() }), vec![]),
                        ],
                    ),
                ],
            ),
        };

        let server = EditorServer::new_test();
        assert_eq!(server.extract_text(plain).unwrap(), "a\tb\nc\nd");
    }

    #[test]
    fn to_plain_round_trip_via_graph() {
        let state = make_state_with_text("round trip");
        let plain = state.to_plain();

        let server = EditorServer::new_test();
        let graph_bytes = server.to_graph(plain).unwrap();
        let recovered = server.to_plain(graph_bytes).unwrap();

        let state2 = editor_state::State::from_plain(&recovered).unwrap();
        let view = state2.view();
        let para_view = view.root().unwrap().child_blocks().next().unwrap();
        assert_eq!(para_view.inline_text(), "round trip");
    }

    #[test]
    fn revert_produces_changeset_that_restores_past_text() {
        use editor_state::{ProjectedState, State};

        let mut ps = ProjectedState::empty();
        ps.commit();

        ps.apply(EditOp::Seq(ListOp::Ins {
            pos: 1,
            item: SeqItem::Char('a'),
        }))
        .unwrap();
        ps.commit();

        let target_heads: Vec<Dot> = ps.graph().current_heads().copied().collect();

        ps.apply(EditOp::Seq(ListOp::Ins {
            pos: 2,
            item: SeqItem::Char('b'),
        }))
        .unwrap();
        ps.commit();

        let graph_bytes = editor_codec::encode_changesets(
            editor_codec::ReencodableChangesets::from_local_ops(ps.graph().changesets_as_vec()),
        )
        .unwrap();
        let target_bytes = editor_codec::encode_dots(&target_heads).unwrap();

        let server = EditorServer::new_test();
        let revert_bytes = server
            .revert(graph_bytes.clone(), target_bytes, Vec::new())
            .unwrap();

        let merged = server.apply(graph_bytes, revert_bytes).unwrap();
        let merged_css: Vec<Changeset<EditOp>> = editor_codec::decode_changeset_stream(&merged)
            .unwrap()
            .into_graph_input();
        let state = State::from_changesets(merged_css, None).unwrap();
        let view = state.view();
        let para = view.root().unwrap().child_blocks().next().unwrap();
        assert_eq!(para.inline_text(), "a");
    }

    #[test]
    fn revert_to_current_heads_is_empty_noop() {
        use editor_state::ProjectedState;

        let mut ps = ProjectedState::empty();
        ps.commit();
        ps.apply(EditOp::Seq(ListOp::Ins {
            pos: 1,
            item: SeqItem::Char('a'),
        }))
        .unwrap();
        ps.commit();

        let graph_bytes = editor_codec::encode_changesets(
            editor_codec::ReencodableChangesets::from_local_ops(ps.graph().changesets_as_vec()),
        )
        .unwrap();
        let heads_bytes = EditorServer::new_test().heads(graph_bytes.clone()).unwrap();

        let server = EditorServer::new_test();
        let revert_bytes = server.revert(graph_bytes, heads_bytes, Vec::new()).unwrap();
        assert!(
            revert_bytes.is_empty(),
            "revert to current heads must be exactly 0 bytes, not a minimal empty envelope"
        );
        let revert_cs: Vec<Changeset<EditOp>> =
            editor_codec::decode_changeset_stream(&revert_bytes)
                .unwrap()
                .into_graph_input();
        assert!(
            revert_cs.is_empty(),
            "revert to current heads must be empty (no-op)"
        );
    }

    #[test]
    fn apply_is_idempotent_for_verbatim_duplicate_changeset() {
        use editor_crdt::OpGraph;

        let base_graph = {
            let mut g = OpGraph::<EditOp>::with_actor(1);
            g.add_mut(EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::Block {
                    node_type: editor_model::NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }))
            .unwrap();
            g.commit_mut();
            editor_codec::encode_changesets(editor_codec::ReencodableChangesets::from_local_ops(
                g.changesets_as_vec(),
            ))
            .unwrap()
        };

        let new_cs_bytes = {
            let css: Vec<Changeset<EditOp>> = editor_codec::decode_changeset_stream(&base_graph)
                .unwrap()
                .into_graph_input();
            let mut g = OpGraph::<EditOp>::from_changesets(css).unwrap();
            g.add_mut(EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('a'),
            }))
            .unwrap();
            g.commit_mut();
            let all = g.changesets_as_vec();
            editor_codec::encode_changesets(editor_codec::ReencodableChangesets::from_local_ops(
                all[all.len() - 1..].to_vec(),
            ))
            .unwrap()
        };

        let server = EditorServer::new_test();
        let once = server
            .apply(base_graph.clone(), new_cs_bytes.clone())
            .unwrap();
        let twice = server.apply(once.clone(), new_cs_bytes.clone()).unwrap();

        assert_eq!(once, twice, "verbatim duplicate must be deduped, not error");
    }

    #[test]
    fn revert_restores_deleted_paragraph() {
        use editor_state::{ProjectedState, State};

        let mut ps = ProjectedState::empty();
        ps.commit();
        // seq: [Para1]

        // Insert Para2 as sibling of Para1 (both children of ROOT)
        ps.apply(EditOp::Seq(ListOp::Ins {
            pos: 1,
            item: SeqItem::Block {
                node_type: editor_model::NodeType::Paragraph,
                parents: vec![Dot::ROOT],
                attrs: vec![],
            },
        }))
        .unwrap();
        ps.commit();
        // seq: [Para1(0), Para2(1)]

        // 'a' goes between Para1 and Para2 → inside Para1
        ps.apply(EditOp::Seq(ListOp::Ins {
            pos: 1,
            item: SeqItem::Char('a'),
        }))
        .unwrap();
        // 'b' goes after Para2 (which shifted to pos 2) → inside Para2
        ps.apply(EditOp::Seq(ListOp::Ins {
            pos: 3,
            item: SeqItem::Char('b'),
        }))
        .unwrap();
        ps.commit();
        // seq: [Para1(0), 'a'(1), Para2(2), 'b'(3)]

        let target_heads: Vec<Dot> = ps.graph().current_heads().copied().collect();

        // Delete Para2 and its content (2 flat items: the Block item + 'b')
        ps.apply(EditOp::Seq(ListOp::Del { pos: 2, len: 2 }))
            .unwrap();
        ps.commit();

        let graph_bytes = editor_codec::encode_changesets(
            editor_codec::ReencodableChangesets::from_local_ops(ps.graph().changesets_as_vec()),
        )
        .unwrap();
        let target_bytes = editor_codec::encode_dots(&target_heads).unwrap();

        let server = EditorServer::new_test();
        let revert_bytes = server
            .revert(graph_bytes.clone(), target_bytes, Vec::new())
            .unwrap();

        let merged = server.apply(graph_bytes, revert_bytes).unwrap();
        let merged_css: Vec<Changeset<EditOp>> = editor_codec::decode_changeset_stream(&merged)
            .unwrap()
            .into_graph_input();
        let state = State::from_changesets(merged_css, None).unwrap();
        let view = state.view();
        let root = view.root().unwrap();
        let paras: Vec<_> = root.child_blocks().collect();
        assert_eq!(paras.len(), 2, "both paragraphs should be restored");
        assert_eq!(paras[0].inline_text(), "a");
        assert_eq!(paras[1].inline_text(), "b");
    }

    #[test]
    fn revert_with_sweep_tombstone_neither_revives_swept_dot_nor_pulls_latest() {
        use editor_state::{ProjectedState, State};

        let mut ps = ProjectedState::empty();
        ps.commit();

        ps.apply(EditOp::Seq(ListOp::Ins {
            pos: 1,
            item: SeqItem::Char('a'),
        }))
        .unwrap();
        ps.commit();
        let z = ps
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 2,
                item: SeqItem::Char('z'),
            }))
            .unwrap()
            .id;
        ps.commit();

        let target_heads: Vec<Dot> = ps.graph().current_heads().copied().collect();

        ps.apply(EditOp::Seq(ListOp::Ins {
            pos: 3,
            item: SeqItem::Char('b'),
        }))
        .unwrap();
        ps.commit();

        let graph_bytes = editor_codec::encode_changesets(
            editor_codec::ReencodableChangesets::from_local_ops(ps.graph().changesets_as_vec()),
        )
        .unwrap();
        let target_bytes = editor_codec::encode_dots(&target_heads).unwrap();

        let server = EditorServer::new_test();
        let revert_bytes = server
            .revert(graph_bytes.clone(), target_bytes, vec![z.to_string()])
            .unwrap();

        let merged = server.apply(graph_bytes, revert_bytes).unwrap();
        let merged_css: Vec<Changeset<EditOp>> = editor_codec::decode_changeset_stream(&merged)
            .unwrap()
            .into_graph_input();
        let state = State::from_changesets(merged_css, None).unwrap();
        let view = state.view();
        let para = view.root().unwrap().child_blocks().next().unwrap();
        assert_eq!(
            para.inline_text(),
            "a",
            "past recovery must drop the swept dot (no revive) and the post-target edit (no latest)"
        );
    }

    fn zombie_css() -> Vec<editor_crdt::Changeset<editor_model::EditOp>> {
        use editor_crdt::{Changeset, Dot, ListOp, Op};
        use editor_model::{EditOp, NodeType, SeqItem};
        let d = |c| Dot::new(1, c);
        let ops = vec![
            Op {
                id: d(0),
                parents: vec![],
                payload: EditOp::Seq(ListOp::Ins {
                    pos: 0,
                    item: SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![Dot::ROOT],
                        attrs: vec![],
                    },
                }),
            },
            Op {
                id: d(1),
                parents: vec![d(0)],
                payload: EditOp::Seq(ListOp::Ins {
                    pos: 1,
                    item: SeqItem::Char('a'),
                }),
            },
            Op {
                id: d(2),
                parents: vec![d(1)],
                payload: EditOp::Seq(ListOp::Ins {
                    pos: 2,
                    item: SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![Dot::ROOT, Dot::new(9, 999)],
                        attrs: vec![],
                    },
                }),
            },
            Op {
                id: d(3),
                parents: vec![d(2)],
                payload: EditOp::Seq(ListOp::Ins {
                    pos: 3,
                    item: SeqItem::Char('z'),
                }),
            },
        ];
        vec![Changeset { ops }]
    }

    #[test]
    fn revived_dead_parent_content_is_not_swept() {
        let css = zombie_css();
        let state = crate::graph::build_state_tolerant(css.clone(), &[]).unwrap();

        let view = state.view();
        let marker = view.node(editor_crdt::Dot::new(1, 2));
        assert!(
            marker.is_some(),
            "the dead-parent marker is revived under Root, not projection-hidden"
        );
        assert_eq!(
            marker.unwrap().inline_text(),
            "z",
            "the revived marker carries its content, so it is live text"
        );
        assert!(
            collect_zombie_dots(&state).is_empty(),
            "revived content is not a zombie"
        );

        let sweep_css = sweep_impl(css).unwrap();
        assert!(
            sweep_css.is_empty(),
            "sweep is a no-op once revival leaves nothing unreachable"
        );
    }

    #[test]
    fn sweep_preserves_modifiers_when_span_anchors_on_revived_content() {
        use editor_model::{Anchor, Modifier, SpanOp};
        let mut css = zombie_css();
        let span_op = editor_crdt::Op {
            id: editor_crdt::Dot::new(1, 10),
            parents: vec![editor_crdt::Dot::new(1, 3)],
            payload: editor_model::EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: editor_crdt::Dot::new(1, 1),
                    bias: editor_model::Bias::Before,
                },
                end: Anchor {
                    id: editor_crdt::Dot::new(1, 3),
                    bias: editor_model::Bias::After,
                },
                modifier: Modifier::Bold,
            }),
        };
        css.push(editor_crdt::Changeset { ops: vec![span_op] });

        let before = crate::graph::build_state_tolerant(css.clone(), &[]).unwrap();
        let before_doc = before.projected.projected().clone();
        let sweep_css = sweep_impl(css.clone()).unwrap();
        let mut merged = css;
        merged.extend(sweep_css);
        let after = crate::graph::build_state_tolerant(merged, &[]).unwrap();
        assert!(
            after.projected.projected() == &before_doc,
            "모디파이어 상태 포함 동등"
        );
    }

    #[test]
    fn sweep_of_clean_document_is_empty() {
        use editor_crdt::{Changeset, Dot, ListOp, Op};
        use editor_model::{EditOp, NodeType, SeqItem};
        let d = |c| Dot::new(2, c);
        let ops = vec![
            Op {
                id: d(0),
                parents: vec![],
                payload: EditOp::Seq(ListOp::Ins {
                    pos: 0,
                    item: SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![Dot::ROOT],
                        attrs: vec![],
                    },
                }),
            },
            Op {
                id: d(1),
                parents: vec![d(0)],
                payload: EditOp::Seq(ListOp::Ins {
                    pos: 1,
                    item: SeqItem::Char('a'),
                }),
            },
        ];
        let sweep_css = sweep_impl(vec![Changeset { ops }]).unwrap();
        assert!(sweep_css.is_empty());
    }

    fn paragraph_ab_graph() -> Vec<u8> {
        let para = Dot::new(1, 0);
        let a = Dot::new(1, 1);
        let b = Dot::new(1, 2);
        let cs = Changeset::<EditOp> {
            ops: vec![
                Op {
                    id: para,
                    parents: vec![],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 0,
                        item: SeqItem::Block {
                            node_type: editor_model::NodeType::Paragraph,
                            parents: vec![Dot::ROOT],
                            attrs: vec![],
                        },
                    }),
                },
                Op {
                    id: a,
                    parents: vec![para],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 1,
                        item: SeqItem::Char('a'),
                    }),
                },
                Op {
                    id: b,
                    parents: vec![a],
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: 2,
                        item: SeqItem::Char('b'),
                    }),
                },
            ],
        };
        enc_css(std::slice::from_ref(&cs))
    }

    #[test]
    fn resolve_v1_selection_reencodes_a_resolvable_anchor_as_v2() {
        let server = EditorServer::new_test();
        let a = Dot::new(1, 1);
        let v1 = format!(
            r#"{{"anchor":{{"chain":["{root}"],"child":{{"dot":"{a}","bind":"right"}},"affinity":"upstream"}},"head":{{"chain":["{root}"],"child":{{"dot":"{a}","bind":"right"}},"affinity":"upstream"}}}}"#,
            root = Dot::ROOT,
            a = a
        );
        let result = server
            .resolve_v1_selection(paragraph_ab_graph(), v1)
            .unwrap();
        assert!(!result.degraded);
        assert_eq!(result.selection.version, 2);
        assert!(!result.selection.anchor.chain.is_empty());
    }

    #[test]
    fn resolve_v1_selection_flags_an_unresolvable_anchor_as_degraded() {
        let server = EditorServer::new_test();
        // A child dot that appears nowhere in the sequence: the v1 reader collapses
        // it to the offset-0 fallback, which the migration must flag as degraded.
        let v1 = format!(
            r#"{{"anchor":{{"chain":["{root}"],"child":{{"dot":"9_9","bind":"left"}},"affinity":"downstream"}},"head":{{"chain":["{root}"],"child":{{"dot":"9_9","bind":"left"}},"affinity":"downstream"}}}}"#,
            root = Dot::ROOT
        );
        let result = server
            .resolve_v1_selection(paragraph_ab_graph(), v1)
            .unwrap();
        assert!(result.degraded);
        assert_eq!(result.selection.version, 2);
    }

    fn prose_graph(text: &str) -> (editor_state::State, Vec<u8>, Vec<u8>) {
        let state = make_state_with_text(text);
        let graph = enc_css(&state.graph().changesets_as_vec());
        let heads: Vec<Dot> = state.graph().current_heads().copied().collect();
        (state, graph, enc_dots(&heads))
    }

    fn resolve_captured_text(graph: &[u8], anchor: &ProseAnchor) -> String {
        let state = crate::graph::build_state_tolerant(dec_css(graph), &[]).unwrap();
        let view = state.view();
        let ctx = editor_state::StableResolveCtx::from_live(&view, state.projected.seq_checkout());
        anchor
            .selection
            .resolve(&ctx)
            .unwrap()
            .resolve(&view)
            .unwrap()
            .collect_text()
    }

    #[test]
    fn capture_prose_anchors_captures_at_past_heads_and_survives_later_edits() {
        let (mut state, before, heads) = prose_graph("hello world");
        // heads 뒤의 편집: 맨 앞 삽입 + 안쪽 삭제("wor" → "wr")
        state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('X'),
            }))
            .unwrap();
        state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Del { pos: 9, len: 1 }))
            .unwrap();
        state.projected_mut().commit();
        let after = enc_css(&state.graph().changesets_as_vec());

        let server = EditorServer::new_test();
        let captured = server
            .capture_prose_anchors(
                after.clone(),
                heads.clone(),
                "hello world".into(),
                ProseRanges {
                    ranges: vec![
                        ProseRange { start: 0, end: 5 },
                        ProseRange { start: 6, end: 11 },
                    ],
                },
            )
            .unwrap();
        assert!(captured.text_matches);
        assert_eq!(captured.anchors.len(), 2);
        assert_eq!(captured.anchors[0].index, 0);
        assert_eq!(captured.anchors[0].text, "hello");
        assert_eq!(captured.anchors[1].text, "world");
        // 앞 삽입은 밖에, 안쪽 삭제는 편집분만 반영된다
        assert_eq!(resolve_captured_text(&after, &captured.anchors[0]), "hello");
        assert_eq!(resolve_captured_text(&after, &captured.anchors[1]), "wrld");

        // heads == 현재 heads 경로와 조상 재구성 경로가 같은 앵커를 낸다
        let same = server
            .capture_prose_anchors(
                before,
                heads,
                "hello world".into(),
                ProseRanges {
                    ranges: vec![ProseRange { start: 0, end: 5 }],
                },
            )
            .unwrap();
        assert_eq!(same.anchors[0].selection, captured.anchors[0].selection);
    }

    #[test]
    fn capture_prose_anchors_gates_on_expected_text() {
        let (_, graph, heads) = prose_graph("hello world");
        let server = EditorServer::new_test();
        let captured = server
            .capture_prose_anchors(
                graph,
                heads,
                "hello there".into(),
                ProseRanges {
                    ranges: vec![ProseRange { start: 0, end: 5 }],
                },
            )
            .unwrap();
        assert!(!captured.text_matches);
        assert!(captured.anchors.is_empty());
    }

    #[test]
    fn capture_prose_anchors_rejects_unknown_heads() {
        let (_, graph, _) = prose_graph("hello world");
        // 별개 문서의 heads로는 이걸 못 만든다 — actor·clock이 결정적이라 그 dot이 이 그래프에도 실재한다.
        let unknown_heads = enc_dots(&[Dot::new(9, 9)]);
        let server = EditorServer::new_test();
        assert!(
            server
                .capture_prose_anchors(
                    graph,
                    unknown_heads,
                    "hello world".into(),
                    ProseRanges { ranges: vec![] },
                )
                .is_err()
        );
    }

    #[test]
    fn capture_prose_anchors_skips_unresolvable_ranges_only() {
        let (_, graph, heads) = prose_graph("a😀b");
        let server = EditorServer::new_test();
        let captured = server
            .capture_prose_anchors(
                graph,
                heads,
                "a😀b".into(),
                ProseRanges {
                    ranges: vec![
                        ProseRange { start: 1, end: 2 }, // 서로게이트 한가운데 — UTF-16 오프셋 변환 자체가 실패
                        ProseRange { start: 1, end: 1 }, // 빈 구간 — 변환은 되지만 셀렉션이 collapsed
                        ProseRange { start: 1, end: 3 },
                    ],
                },
            )
            .unwrap();
        assert!(captured.text_matches);
        assert_eq!(captured.anchors.len(), 1);
        assert_eq!(captured.anchors[0].index, 2);
        assert_eq!(captured.anchors[0].text, "😀");
    }

    fn xml_test_graph(text: &str) -> Vec<u8> {
        let state = make_state_with_text(text);
        EditorServer::new_test().to_graph(state.to_plain()).unwrap()
    }

    fn render(server: &EditorServer, graph: Vec<u8>) -> String {
        let out = server.to_xml(graph, Vec::new()).unwrap();
        assert!(out.error.is_none(), "{:?}", out.error);
        out.xml
    }

    #[test]
    fn outline_xml_lists_blocks_and_reports_a_bad_under() {
        let server = EditorServer::new_test();
        let xml = render(&server, xml_test_graph("hello"));
        let out = server
            .outline_xml(xml.clone(), "root".into(), 1, 0, 200, false)
            .unwrap();
        assert!(out.error.is_none());
        assert_eq!(out.rows.len(), 1);
        assert_eq!(out.rows[0].path, "1");
        assert_eq!(out.rows[0].preview.as_deref(), Some("hello"));
        assert_eq!(out.rows[0].chars, Some(5));
        let full = server
            .outline_xml(xml.clone(), "1".into(), 1, 0, 200, true)
            .unwrap();
        assert!(full.xml.unwrap().starts_with("<paragraph dot="));
        let bad = server
            .outline_xml(xml.clone(), "9_9".into(), 1, 0, 200, false)
            .unwrap();
        let info = bad.error.unwrap();
        assert!(info.detail.contains("address_unresolved"));
        let invalid = server
            .outline_xml(xml, "0".into(), 1, 0, 200, false)
            .unwrap();
        assert!(invalid.error.unwrap().detail.contains("address_invalid"));
    }

    #[test]
    fn edit_xml_applies_a_batch_and_names_the_failing_op() {
        let server = EditorServer::new_test();
        let xml = render(&server, xml_test_graph("hello"));
        let ok = server
            .edit_xml(
                xml.clone(),
                r#"[{"op":"insert","xml":"<paragraph>world</paragraph>","at":{"after":"1"}}]"#
                    .into(),
            )
            .unwrap();
        assert!(ok.error.is_none(), "{:?}", ok.error);
        assert!(ok.xml.contains("<paragraph>world</paragraph>"));
        assert_eq!(ok.affected.len(), 1);
        assert_eq!(ok.affected[0].rows.len(), 2);
        assert!(server.verify_xml(ok.xml.clone()).unwrap().error.is_none());

        let bad = server
            .edit_xml(
                xml.clone(),
                r#"[{"op":"delete","targets":["1"]},{"op":"delete","targets":["9_9"]}]"#.into(),
            )
            .unwrap();
        let err = bad.error.unwrap();
        assert_eq!(err.op, Some(1));
        assert!(err.info.detail.contains("address_unresolved"));
        assert_eq!(bad.xml, "");

        let malformed = server.edit_xml(xml, "not json".into()).unwrap();
        assert!(malformed.error.unwrap().info.detail.contains("internal"));
    }

    fn attr_of(xml: &str, tag: &str, attr: &str) -> String {
        let line = xml
            .lines()
            .find(|line| line.trim_start().starts_with(tag))
            .unwrap_or_else(|| panic!("no {tag} in {xml}"));
        line.split(&format!("{attr}=\""))
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("the attribute has a value")
            .to_string()
    }

    /// A thousand paragraphs and a hundred thousand characters, the same shape
    /// the crate-level smoke test uses.
    fn big_plain_doc() -> editor_model::PlainDoc {
        use std::collections::BTreeMap;

        let paragraph = |nth: usize| editor_model::PlainNodeEntry {
            node: editor_model::PlainNode::Paragraph(editor_model::PlainParagraphNode {}),
            modifiers: BTreeMap::new(),
            carry: Vec::new(),
            children: vec![editor_model::PlainNodeEntry {
                node: editor_model::PlainNode::Text(editor_model::PlainTextNode {
                    text: (0..100)
                        .map(|i| char::from(b'a' + ((nth + i) % 26) as u8))
                        .collect(),
                }),
                modifiers: BTreeMap::new(),
                carry: Vec::new(),
                children: Vec::new(),
            }],
        };
        editor_model::PlainDoc {
            root: editor_model::PlainNodeEntry {
                node: editor_model::PlainNode::Root(editor_model::PlainRootNode::default()),
                modifiers: BTreeMap::new(),
                carry: Vec::new(),
                children: (0..1000).map(paragraph).collect(),
            },
        }
    }

    #[test]
    #[ignore = "perf smoke; run with --release -- --ignored perf"]
    fn perf_smoke_of_the_xml_boundary_on_a_large_document() {
        let server = EditorServer::new_test();
        let graph = server.to_graph(big_plain_doc()).unwrap();
        println!("graph: {} bytes", graph.len());

        let css = editor_codec::decode_changeset_stream(&graph[..])
            .unwrap()
            .into_graph_input();
        let started = std::time::Instant::now();
        let built = crate::graph::build_state_tolerant(css, &[]).unwrap();
        println!(
            "one state build: {:?} ({} blocks)",
            started.elapsed(),
            built.to_plain().root.children.len()
        );

        let started = std::time::Instant::now();
        let rendered = server.to_xml(graph.clone(), Vec::new()).unwrap();
        println!(
            "to_xml: {:?} ({} bytes)",
            started.elapsed(),
            rendered.xml.len()
        );
        assert!(rendered.error.is_none(), "{:?}", rendered.error);

        let started = std::time::Instant::now();
        let result = server
            .edit_from_xml(graph.clone(), Vec::new(), rendered.xml.clone())
            .unwrap();
        println!("edit_from_xml (unchanged): {:?}", started.elapsed());
        assert!(result.error.is_none(), "{:?}", result.error);
        assert!(result.bundle.is_empty());

        let changed = rendered
            .xml
            .replace("</root>", "  <paragraph>zz</paragraph>\n</root>");
        let started = std::time::Instant::now();
        let result = server.edit_from_xml(graph, Vec::new(), changed).unwrap();
        println!(
            "edit_from_xml (changed): {:?} ({} bytes back)",
            started.elapsed(),
            result.xml.len()
        );
        assert!(result.error.is_none(), "{:?}", result.error);
        assert!(!result.bundle.is_empty());
        assert!(!result.xml.is_empty());
    }

    #[test]
    fn xml_render_carries_a_null_error_beside_the_document() {
        let rendered = serde_json::to_string(&XmlRender {
            error: None,
            xml: "<root/>".to_string(),
        })
        .unwrap();
        assert_eq!(rendered, r#"{"error":null,"xml":"<root/>"}"#);
    }

    #[test]
    fn to_xml_reads_the_same_swept_document_edit_from_xml_writes_into() {
        let server = EditorServer::new_test();
        let graph = xml_test_graph("alpha beta");
        let paragraph = attr_of(&render(&server, graph.clone()), "<paragraph", "dot");
        let swept = vec![paragraph.clone()];

        let rendered = server.to_xml(graph.clone(), swept.clone()).unwrap();
        assert!(rendered.error.is_none(), "{:?}", rendered.error);
        assert!(
            !rendered.xml.contains(&format!("dot=\"{paragraph}\"")),
            "the swept paragraph must not reach the file: {}",
            rendered.xml
        );

        let result = server.edit_from_xml(graph, swept, rendered.xml).unwrap();
        assert!(result.error.is_none(), "{:?}", result.error);
        assert!(result.bundle.is_empty());
    }

    #[test]
    fn edit_from_xml_returns_the_file_rewritten_on_the_new_base() {
        let server = EditorServer::new_test();
        let graph = xml_test_graph("가나");

        let opened = server.to_xml(graph.clone(), Vec::new()).unwrap();
        assert!(opened.error.is_none(), "{:?}", opened.error);
        let edited = opened.xml.replace("가나", "가나다");

        let saved = server
            .edit_from_xml(graph.clone(), Vec::new(), edited)
            .unwrap();
        assert!(saved.error.is_none(), "{:?}", saved.error);
        assert!(!saved.bundle.is_empty());
        assert!(saved.xml.contains("가나다"), "{}", saved.xml);
        assert_ne!(
            attr_of(&saved.xml, "<root", "base"),
            attr_of(&opened.xml, "<root", "base")
        );
        assert!(
            server
                .verify_xml(saved.xml.clone())
                .unwrap()
                .error
                .is_none()
        );

        let merged = server.apply(graph, saved.bundle).unwrap();
        let again = server.edit_from_xml(merged, Vec::new(), saved.xml).unwrap();
        assert!(again.error.is_none(), "{:?}", again.error);
        assert!(
            again.bundle.is_empty(),
            "the rewritten file must read back as no change"
        );
        assert_eq!(again.xml, "");
    }

    #[test]
    fn edit_from_xml_on_a_stale_base_is_refused_until_the_file_is_reopened() {
        let server = EditorServer::new_test();
        let graph = xml_test_graph("가나");
        let opened = render(&server, graph.clone());

        let concurrent = server
            .edit_from_xml(
                graph.clone(),
                Vec::new(),
                opened.replacen(
                    "  <paragraph",
                    "  <paragraph>동시</paragraph>\n  <paragraph",
                    1,
                ),
            )
            .unwrap();
        assert!(concurrent.error.is_none(), "{:?}", concurrent.error);
        assert!(!concurrent.bundle.is_empty());
        let live_graph = server.apply(graph, concurrent.bundle).unwrap();

        let stale = server
            .edit_from_xml(
                live_graph.clone(),
                Vec::new(),
                opened.replace("가나", "가나다"),
            )
            .unwrap();
        let error = stale
            .error
            .expect("a base behind the live heads must be refused");
        assert_eq!(error.detail, r#"{"type":"base_not_in_history"}"#);
        assert!(stale.bundle.is_empty());
        assert_eq!(stale.xml, "");

        let reopened = render(&server, live_graph.clone());
        assert!(reopened.contains(">동시<"), "{reopened}");
        let saved = server
            .edit_from_xml(
                live_graph.clone(),
                Vec::new(),
                reopened.replace("가나", "가나다"),
            )
            .unwrap();
        assert!(saved.error.is_none(), "{:?}", saved.error);
        assert!(
            saved.xml.contains(">가나다<") && saved.xml.contains(">동시<"),
            "{}",
            saved.xml
        );
        let merged = server.apply(live_graph, saved.bundle).unwrap();
        let again = server.edit_from_xml(merged, Vec::new(), saved.xml).unwrap();
        assert!(again.error.is_none(), "{:?}", again.error);
        assert!(
            again.bundle.is_empty(),
            "the rewritten file must read back as no change"
        );
    }

    #[test]
    fn edit_from_xml_reports_a_base_this_history_does_not_hold() {
        let server = EditorServer::new_test();
        let graph = xml_test_graph("alpha beta");
        let xml = render(&server, graph.clone());
        let base = attr_of(&xml, "<root", "base");
        let stranger = editor_xml::encode_base(&[Dot::new(9, 9)]).unwrap();

        let result = server
            .edit_from_xml(graph, Vec::new(), xml.replace(&base, &stranger))
            .unwrap();

        let error = result.error.expect("a base outside the history must fail");
        assert_eq!(error.detail, r#"{"type":"base_not_in_history"}"#);
        assert!(result.bundle.is_empty());
    }

    #[test]
    fn xml_refuses_a_degraded_projection_on_both_sides() {
        let server = EditorServer::new_test();
        let graph = enc_css(&degraded_prone_css());
        let xml = render(&server, graph.clone());

        let rendered = {
            let _guard = editor_model::override_repair_budget(1);
            server.to_xml(graph.clone(), Vec::new()).unwrap()
        };
        let error = rendered
            .error
            .expect("a degraded projection must be refused");
        assert_eq!(error.detail, r#"{"type":"projection_degraded"}"#);
        assert!(rendered.xml.is_empty());

        let result = {
            let _guard = editor_model::override_repair_budget(1);
            server.edit_from_xml(graph, Vec::new(), xml).unwrap()
        };
        let error = result.error.expect("a degraded base must be refused");
        assert_eq!(error.detail, r#"{"type":"projection_degraded"}"#);
        assert!(result.bundle.is_empty());
    }

    #[test]
    fn edit_from_xml_applies_a_text_replacement_back_into_the_graph() {
        let server = EditorServer::new_test();
        let graph = xml_test_graph("alpha beta");

        let xml = render(&server, graph.clone());
        assert!(xml.contains(">alpha beta<"), "{xml}");
        let edited = xml.replace(">alpha beta<", ">alpha gamma<");

        let result = server
            .edit_from_xml(graph.clone(), Vec::new(), edited)
            .unwrap();
        assert!(result.error.is_none(), "{:?}", result.error);
        assert!(!result.bundle.is_empty());

        let merged = server.apply(graph, result.bundle).unwrap();
        assert_eq!(server.materialize(merged).unwrap().text, "alpha gamma");
    }

    #[test]
    fn edit_from_xml_returns_an_empty_bundle_when_the_xml_is_unchanged() {
        let server = EditorServer::new_test();
        let graph = xml_test_graph("alpha beta");
        let xml = render(&server, graph.clone());

        let result = server.edit_from_xml(graph, Vec::new(), xml).unwrap();
        assert!(result.error.is_none(), "{:?}", result.error);
        assert!(result.bundle.is_empty());
        assert_eq!(result.blocks_inserted, 0);
        assert_eq!(result.blocks_deleted, 0);
        assert_eq!(result.blocks_moved, 0);
        assert_eq!(result.blocks_updated, 0);
        assert_eq!(result.chars_inserted, 0);
        assert_eq!(result.chars_deleted, 0);
    }

    #[test]
    fn edit_from_xml_reports_a_syntax_error_with_a_position_and_structured_detail() {
        let server = EditorServer::new_test();
        let graph = xml_test_graph("alpha beta");

        let result = server
            .edit_from_xml(
                graph,
                Vec::new(),
                "<root><paragraph bad=x>alpha beta</paragraph></root>".to_string(),
            )
            .unwrap();
        let error = result.error.expect("malformed xml must produce an error");
        assert_eq!(error.detail, r#"{"type":"attr_unquoted","attr":"bad"}"#);
        assert!(error.line.is_some());
        assert!(error.column.is_some());
        assert!(result.bundle.is_empty());
    }

    #[test]
    fn verify_xml_agrees_with_edit_from_xml_on_well_formed_and_malformed_input() {
        let server = EditorServer::new_test();
        let graph = xml_test_graph("alpha beta");
        let xml = render(&server, graph);

        assert!(server.verify_xml(xml).unwrap().error.is_none());

        let verdict = server
            .verify_xml("<root><paragraph bad=x>alpha beta</paragraph></root>".to_string())
            .unwrap();
        assert_eq!(
            verdict.error.expect("malformed xml").detail,
            r#"{"type":"attr_unquoted","attr":"bad"}"#
        );
    }

    #[test]
    fn xml_error_detail_is_serialized_as_a_type_tagged_object() {
        let out_of_range = xml_error_info(&editor_xml::XmlError::new(
            editor_xml::XmlErrorDetail::ValueOutOfRange {
                modifier: "font_size".to_string(),
                value: "99".to_string(),
            },
        ))
        .unwrap();
        assert_eq!(
            out_of_range.detail,
            r#"{"type":"value_out_of_range","modifier":"font_size","value":"99"}"#
        );

        let newline = xml_error_info(&editor_xml::XmlError::new(
            editor_xml::XmlErrorDetail::NewlineInText,
        ))
        .unwrap();
        assert_eq!(newline.detail, r#"{"type":"newline_in_text"}"#);

        let missing_dot = xml_error_info(&editor_xml::XmlError::new(
            editor_xml::XmlErrorDetail::DotNotInDocument {
                dot: "1_9".to_string(),
            },
        ))
        .unwrap();
        assert_eq!(
            missing_dot.detail,
            r#"{"type":"dot_not_in_document","dot":"1_9"}"#
        );
    }

    fn labeled_plain_doc(labels: &[&str]) -> editor_model::PlainDoc {
        use std::collections::BTreeMap;
        let paragraph = |text: &str| editor_model::PlainNodeEntry {
            node: editor_model::PlainNode::Paragraph(editor_model::PlainParagraphNode {}),
            modifiers: BTreeMap::new(),
            carry: Vec::new(),
            children: vec![editor_model::PlainNodeEntry {
                node: editor_model::PlainNode::Text(editor_model::PlainTextNode {
                    text: text.into(),
                }),
                modifiers: BTreeMap::new(),
                carry: Vec::new(),
                children: Vec::new(),
            }],
        };
        editor_model::PlainDoc {
            root: editor_model::PlainNodeEntry {
                node: editor_model::PlainNode::Root(editor_model::PlainRootNode::default()),
                modifiers: BTreeMap::new(),
                carry: Vec::new(),
                children: labels.iter().map(|l| paragraph(l)).collect(),
            },
        }
    }

    fn plain_paragraph_texts(doc: &editor_model::PlainDoc) -> Vec<String> {
        doc.root
            .children
            .iter()
            .filter(|e| matches!(e.node, editor_model::PlainNode::Paragraph(_)))
            .map(|e| {
                e.children
                    .iter()
                    .filter_map(|c| match &c.node {
                        editor_model::PlainNode::Text(t) => Some(t.text.clone()),
                        _ => None,
                    })
                    .collect::<String>()
            })
            .collect()
    }

    fn xml_paragraph_texts(xml: &str) -> Vec<String> {
        xml.lines()
            .filter(|l| l.trim_start().starts_with("<paragraph"))
            .map(|l| {
                let s = l.find('>').unwrap() + 1;
                match l.rfind("</paragraph>") {
                    Some(e) => l[s..e].to_string(),
                    None => String::new(),
                }
            })
            .collect()
    }

    #[test]
    fn a_second_edit_over_rewritten_move_dots_does_not_duplicate_the_moved_run() {
        let server = EditorServer::new_test();
        let labels: Vec<String> = (1..=16)
            .map(|i| format!("A{i:02}"))
            .chain((1..=16).map(|i| format!("B{i:02}")))
            .chain(std::iter::once("Z".to_string()))
            .collect();
        let refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let graph0 = server.to_graph(labeled_plain_doc(&refs)).unwrap();
        let opened = render(&server, graph0.clone());
        let lines: Vec<&str> = opened.lines().collect();
        assert_eq!(lines.len(), 35);

        let mut swapped: Vec<&str> = vec![lines[0]];
        swapped.extend_from_slice(&lines[17..33]);
        swapped.extend_from_slice(&lines[1..17]);
        swapped.extend_from_slice(&lines[33..]);
        let target1 = swapped.join("\n") + "\n";

        let saved1 = server
            .edit_from_xml(graph0.clone(), Vec::new(), target1)
            .unwrap();
        assert!(saved1.error.is_none(), "{:?}", saved1.error);
        let graph1 = server.apply(graph0, saved1.bundle.clone()).unwrap();
        let after1 = plain_paragraph_texts(&server.to_plain(graph1.clone()).unwrap());
        let want1: Vec<String> = (1..=16)
            .map(|i| format!("B{i:02}"))
            .chain((1..=16).map(|i| format!("A{i:02}")))
            .chain(std::iter::once("Z".to_string()))
            .collect();
        assert_eq!(after1, want1, "first save (swap) replayed through apply");
        assert_eq!(
            xml_paragraph_texts(&saved1.xml),
            want1,
            "rewritten file after first save"
        );

        let lines1: Vec<&str> = saved1.xml.lines().collect();
        assert_eq!(lines1.len(), 35, "{}", saved1.xml);
        let mut with_sections: Vec<String> = vec![lines1[0].to_string()];
        with_sections.push(
            "  <paragraph carry:bold=\"\"><font_weight value=\"700\">1</font_weight></paragraph>"
                .into(),
        );
        with_sections.push("  <paragraph></paragraph>".into());
        for l in &lines1[17..33] {
            with_sections.push((*l).to_string());
        }
        with_sections.push("  <paragraph></paragraph>".into());
        with_sections.push("  <horizontal_rule attr:variant=\"line\"/>".into());
        with_sections.push(
            "  <paragraph carry:bold=\"\"><font_weight value=\"700\">2</font_weight></paragraph>"
                .into(),
        );
        with_sections.push("  <paragraph></paragraph>".into());
        for l in &lines1[1..17] {
            with_sections.push((*l).to_string());
        }
        with_sections.push("  <paragraph></paragraph>".into());
        with_sections.push("  <horizontal_rule attr:variant=\"line\"/>".into());
        with_sections.push(
            "  <paragraph carry:bold=\"\"><font_weight value=\"700\">3</font_weight></paragraph>"
                .into(),
        );
        for l in &lines1[33..] {
            with_sections.push((*l).to_string());
        }
        let target2 = with_sections.join("\n") + "\n";

        let saved2 = server
            .edit_from_xml(graph1.clone(), Vec::new(), target2)
            .unwrap();
        assert!(saved2.error.is_none(), "{:?}", saved2.error);
        let graph2 = server.apply(graph1, saved2.bundle.clone()).unwrap();
        let after2 = plain_paragraph_texts(&server.to_plain(graph2.clone()).unwrap());
        for label in &labels {
            let n = after2.iter().filter(|t| *t == label).count();
            assert_eq!(
                n, 1,
                "{label} appears {n} times after the second save: {after2:?}"
            );
        }
        assert_eq!(
            xml_paragraph_texts(&saved2.xml)
                .iter()
                .filter(|t| t.starts_with('A'))
                .count(),
            16
        );
        assert_eq!(
            saved2.blocks_inserted,
            9,
            "{:?}",
            (
                saved2.blocks_inserted,
                saved2.blocks_deleted,
                saved2.blocks_moved
            )
        );
        assert_eq!(saved2.blocks_deleted, 0);
    }

    mod prose_anchor_equivalence {
        use std::collections::BTreeMap;

        use editor_model::{
            PlainDoc, PlainNode, PlainNodeEntry, PlainParagraphNode, PlainRootNode, PlainTextNode,
        };
        use proptest::prelude::*;

        use super::*;

        fn entry(node: PlainNode, children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
            PlainNodeEntry {
                node,
                modifiers: BTreeMap::new(),
                carry: Vec::new(),
                children,
            }
        }

        fn paragraph(text: &str) -> PlainNodeEntry {
            let children = if text.is_empty() {
                Vec::new()
            } else {
                vec![entry(
                    PlainNode::Text(PlainTextNode { text: text.into() }),
                    Vec::new(),
                )]
            };
            entry(PlainNode::Paragraph(PlainParagraphNode {}), children)
        }

        fn doc(paragraphs: &[String]) -> PlainDoc {
            PlainDoc {
                root: PlainNodeEntry {
                    node: PlainNode::Root(PlainRootNode::default()),
                    modifiers: BTreeMap::new(),
                    carry: Vec::new(),
                    children: paragraphs.iter().map(|p| paragraph(p)).collect(),
                },
            }
        }

        fn paragraphs() -> impl Strategy<Value = Vec<String>> {
            prop::collection::vec(
                prop::collection::vec(
                    prop::sample::select(vec!['a', 'b', '가', '나', ' ', '😀']),
                    0..6,
                )
                .prop_map(|chars| chars.into_iter().collect::<String>()),
                1..4,
            )
        }

        // Some(c)=마지막 문단 끝에 c 삽입, None=마지막 글자 삭제(있을 때만)
        fn edits() -> impl Strategy<Value = Vec<Option<char>>> {
            prop::collection::vec(
                prop::option::of(prop::sample::select(vec!['x', '다'])),
                0..5,
            )
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(64))]
            #[test]
            fn reprojection_at_heads_equals_snapshot_projection(paragraphs in paragraphs(), edits in edits()) {
                let mut state = editor_state::State::from_plain(&doc(&paragraphs)).unwrap();
                let snapshot_text = editor_state::prose_annotated(&state.view()).text().to_string();
                let before = enc_css(&state.graph().changesets_as_vec());
                let heads: hashbrown::HashSet<Dot> = state.graph().current_heads().copied().collect();

                // seq 위치 가정: 문단 블록 1 + 글자 수. 어긋나면 state.projected.seq()로 실제 배치를 확인한다.
                let mut total: usize = paragraphs.iter().map(|p| 1 + p.chars().count()).sum();
                let mut last_len = paragraphs.last().map(|p| p.chars().count()).unwrap_or(0);
                for edit in edits {
                    match edit {
                        Some(c) => {
                            state.projected_mut().apply(EditOp::Seq(ListOp::Ins { pos: total, item: SeqItem::Char(c) })).unwrap();
                            total += 1;
                            last_len += 1;
                        }
                        None if last_len > 0 => {
                            state.projected_mut().apply(EditOp::Seq(ListOp::Del { pos: total - 1, len: 1 })).unwrap();
                            total -= 1;
                            last_len -= 1;
                        }
                        None => {}
                    }
                }
                state.projected_mut().commit();
                let after = enc_css(&state.graph().changesets_as_vec());

                // ① 스냅샷 추출 경로(create_editor_from_graph → state_from_changesets)의 텍스트
                let (snapshot_state, _) = crate::graph::state_from_changesets(before.clone()).unwrap();
                let snapshot_prose = editor_state::prose_annotated(&snapshot_state.view());
                prop_assert_eq!(snapshot_prose.text(), snapshot_text.as_str());

                // ② heads 조상 재구성 경로의 텍스트
                let after_state = crate::graph::build_state_tolerant(dec_css(&after), &[]).unwrap();
                let at_heads = state_at_heads(after_state.graph(), &heads, &[]).unwrap();
                prop_assert!(!at_heads.projection_degraded());
                let at_heads_prose = editor_state::prose_annotated(&at_heads.view());
                prop_assert_eq!(at_heads_prose.text(), snapshot_text.as_str());

                // ③ FFI 게이트가 그 텍스트를 받아들인다
                let server = EditorServer::new_test();
                let capture = server
                    .capture_prose_anchors(after, enc_dots(&heads.iter().copied().collect::<Vec<_>>()), snapshot_text.clone(), ProseRanges { ranges: vec![] })
                    .unwrap();
                prop_assert!(capture.text_matches);
            }
        }
    }
}
