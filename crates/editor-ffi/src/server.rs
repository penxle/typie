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
pub struct Materialized {
    pub plain: editor_model::PlainDoc,
    pub text: String,
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
    pub base_char_count: u32,
    pub plain: editor_model::PlainDoc,
    pub text: String,
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

#[cfg_attr(feature = "wasm", wasm_bindgen::prelude::wasm_bindgen)]
pub struct EditorServer;

#[cfg_attr(feature = "wasm", editor_macros::ffi_export(wasm))]
#[allow(dead_code)]
impl EditorServer {
    pub fn create() -> Owned<Self> {
        into_owned(Self)
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

    pub fn extract_text(&self, doc: Complex<editor_model::PlainDoc>) -> EditorResult<String> {
        let plain: editor_model::PlainDoc = doc.from_ffi()?;
        let state = editor_state::State::from_plain(&plain).map_err(|e| EditorError::General {
            msg: format!("{e:?}"),
        })?;
        Ok(extract_text_from_view(&state.view()))
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
        let existing_cs: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&existing[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        let bundles = crate::graph::decode_length_prefixed(&packed_bundles)?;

        let mut state = editor_state::State::from_changesets(existing_cs, None)?;
        let base_char_count = count_characters(&extract_text_from_view(&state.view()));

        let mut statuses: Vec<BundleStatus> = Vec::with_capacity(bundles.len());
        let mut char_counts: Vec<u32> = Vec::with_capacity(bundles.len());
        let mut last = base_char_count;

        for bundle in bundles {
            let status = match editor_codec::decode_changeset_stream(&bundle[..]) {
                Ok(decoded) => match state.receive_remote_changesets(decoded.into_graph_input()) {
                    Ok((next, ops)) if !ops.is_empty() => {
                        state = next;
                        BundleStatus::Applied
                    }
                    Ok(_) => BundleStatus::Duplicate,
                    Err(_) => BundleStatus::Failed,
                },
                Err(_) => BundleStatus::Failed,
            };
            if status == BundleStatus::Applied {
                last = count_characters(&extract_text_from_view(&state.view()));
            }
            statuses.push(status);
            char_counts.push(last);
        }

        let heads: Vec<editor_crdt::Dot> = state.graph().current_heads().copied().collect();
        let heads = if heads.is_empty() {
            Vec::new()
        } else {
            editor_codec::encode_dots(&heads).map_err(|e| FfiError::Serialization(e.to_string()))?
        };
        let plain = state.to_plain();
        let text = extract_text_from_view(&state.view());

        Ok(CollectResult {
            heads,
            statuses,
            char_counts,
            base_char_count,
            plain,
            text,
        }
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

    pub fn to_plain(
        &self,
        changeset_payloads: Vec<u8>,
    ) -> EditorResult<Complex<editor_model::PlainDoc>> {
        let cs: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&changeset_payloads[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        let state = crate::graph::build_state_tolerant(cs)?;
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
        let state = crate::graph::build_state_tolerant(cs)?;
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

    pub fn revert(&self, graph: Vec<u8>, target_heads: Vec<u8>) -> EditorResult<Vec<u8>> {
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

        let state = crate::graph::build_state_tolerant(css)
            .map_err(|e| FfiError::RevertFailed(e.to_string()))?;
        let current_heads: hashbrown::HashSet<editor_crdt::Dot> =
            state.graph().current_heads().copied().collect();

        let target_state = state_at_heads(state.graph(), &target_set)?;

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
        let state = crate::graph::build_state_tolerant(cs)?;
        let plain = state.to_plain();
        let text = extract_text_from_view(&state.view());
        Ok(Materialized { plain, text }.into_ffi()?)
    }

    pub fn validate_and_extract_text(&self, changeset_payloads: Vec<u8>) -> EditorResult<String> {
        let cs: Vec<editor_crdt::Changeset<editor_model::EditOp>> =
            editor_codec::decode_changeset_stream(&changeset_payloads[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?
                .into_graph_input();
        let state = crate::graph::build_state_tolerant(cs)?;
        Ok(extract_text_from_view(&state.view()))
    }
}

/// Builds a `State` whose graph contains only the ops that are ancestors of
/// (or equal to) `heads`. Used by `revert` to project the document at a past
/// point without requiring a bespoke `from_op_graph_at` on the new model.
fn state_at_heads(
    graph: &editor_crdt::OpGraph<editor_model::EditOp>,
    heads: &hashbrown::HashSet<editor_crdt::Dot>,
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
    editor_state::State::from_changesets(css, None)
        .map_err(|e| FfiError::RevertFailed(e.to_string()))
}

fn extract_text_from_view(view: &editor_model::DocView<'_>) -> String {
    fn walk(nv: editor_model::NodeView<'_>, out: &mut String) {
        if nv.spec().is_leaf() {
            return;
        }
        for child in nv.children() {
            match child {
                editor_model::ChildView::Block(b) => walk(b, out),
                editor_model::ChildView::Leaf(l) => {
                    if let Some(ch) = l.as_char() {
                        out.push(ch);
                    }
                }
            }
        }
        out.push('\n');
    }
    let mut out = String::new();
    if let Some(root) = view.root() {
        walk(root, &mut out);
    }
    out.trim_end_matches('\n').to_string()
}

/// Mirror of the API's `countCharacters`: drop zero-width spaces, collapse
/// whitespace runs to a single space, trim, and count Unicode scalar values.
fn count_characters(text: &str) -> u32 {
    let mut collapsed = String::with_capacity(text.len());
    let mut prev_ws = false;
    for c in text.chars() {
        if c == '\u{200B}' {
            continue;
        }
        if c.is_whitespace() {
            if !prev_ws {
                collapsed.push(' ');
            }
            prev_ws = true;
        } else {
            collapsed.push(c);
            prev_ws = false;
        }
    }
    collapsed.trim().chars().count() as u32
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
        let server = EditorServer;
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
        let server = EditorServer;
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
        let server = EditorServer;
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
        let server = EditorServer;
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
        let server = EditorServer;
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
        let server = EditorServer;
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
        let server = EditorServer;
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
        let server = EditorServer;
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

        let server = EditorServer;
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
        let server = EditorServer;
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
    fn count_characters_matches_normalization() {
        assert_eq!(count_characters("hello"), 5);
        assert_eq!(count_characters("a\u{200B}b"), 2); // zero-width space stripped
        assert_eq!(count_characters("a  b"), 3); // whitespace run collapsed → "a b"
        assert_eq!(count_characters("  trim  "), 4); // trimmed → "trim"
        assert_eq!(count_characters("line1\nline2"), 11); // newline → space → "line1 line2"
        assert_eq!(count_characters(""), 0);
        assert_eq!(count_characters("   "), 0);
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
        let server = EditorServer;
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
        let server = EditorServer;
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
        let server = EditorServer;

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
        let server = EditorServer;
        let result = server.apply(enc_css(&[cs_v1]), enc_css(&[cs_v2]));
        assert!(matches!(
            result,
            Err(EditorError::Ffi(FfiError::CausalOrderViolation { .. }))
        ));
    }

    #[cfg(feature = "wasm-server")]
    #[test]
    fn outline_text_to_svg_forwards_svg_document() {
        let server = EditorServer;
        let svg = server
            .outline_text_to_svg(load_test_font(), "A".to_string())
            .unwrap();
        assert!(svg.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg""#));
        assert!(svg.contains("<path d=\""));
    }

    #[cfg(feature = "wasm-server")]
    #[test]
    fn outline_text_to_svg_rejects_invalid_font_data() {
        let server = EditorServer;
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
        let server = EditorServer;
        let result = server.extract_text(plain).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn to_plain_round_trip_via_graph() {
        let state = make_state_with_text("round trip");
        let plain = state.to_plain();

        let server = EditorServer;
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

        let server = EditorServer;
        let revert_bytes = server.revert(graph_bytes.clone(), target_bytes).unwrap();

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
        let heads_bytes = EditorServer.heads(graph_bytes.clone()).unwrap();

        let server = EditorServer;
        let revert_bytes = server.revert(graph_bytes, heads_bytes).unwrap();
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

        let server = EditorServer;
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

        let server = EditorServer;
        let revert_bytes = server.revert(graph_bytes.clone(), target_bytes).unwrap();

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
}
