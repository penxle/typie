use editor_macros::{ffi, ffi_export};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::prelude::*;

#[ffi]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ChunkCodepoints {
    pub chunks: Vec<Vec<u32>>,
}

#[wasm_bindgen]
pub struct EditorServer;

#[ffi_export(wasm)]
impl EditorServer {
    pub fn create() -> Owned<Self> {
        into_owned(Self)
    }

    pub fn get_font_metadata(
        &self,
        data: Vec<u8>,
    ) -> EditorResult<Complex<editor_server::font::FontMetadata>> {
        editor_server::font::get_font_metadata(&data)?
            .into_ffi()
            .map_err(Into::into)
    }

    pub fn get_font_codepoints(&self, ttf_data: Vec<u8>) -> EditorResult<Vec<u32>> {
        Ok(editor_server::font::get_font_codepoints(&ttf_data)?)
    }

    pub fn outline_text_to_svg(&self, font_data: Vec<u8>, text: String) -> EditorResult<String> {
        Ok(editor_server::font::outline_text_to_svg(&font_data, &text)?)
    }

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
        let (doc, _) = editor_model::Doc::from_plain(plain);
        Ok(doc.extract_text())
    }

    pub fn default_doc_with_preset(
        &self,
        root: Complex<editor_model::PlainRootNode>,
        modifiers: Vec<Complex<editor_model::Modifier>>,
    ) -> EditorResult<Complex<editor_model::PlainDoc>> {
        let root = root.from_ffi()?;
        let modifiers: Vec<editor_model::Modifier> = modifiers.from_ffi()?;
        let modifier_map = modifiers.into_iter().map(|m| (m.as_type(), m)).collect();

        let paragraph_id = editor_model::NodeId::new();
        let mut nodes = std::collections::BTreeMap::new();
        nodes.insert(
            editor_model::NodeId::ROOT,
            editor_model::PlainNodeEntry {
                parent: None,
                children: vec![paragraph_id],
                modifiers: modifier_map,
                node: editor_model::PlainNode::Root(root),
            },
        );
        nodes.insert(
            paragraph_id,
            editor_model::PlainNodeEntry {
                parent: Some(editor_model::NodeId::ROOT),
                children: vec![],
                modifiers: Default::default(),
                node: editor_model::PlainNode::Paragraph(editor_model::PlainParagraphNode {}),
            },
        );
        Ok(editor_model::PlainDoc { nodes }.into_ffi()?)
    }

    pub fn apply(&self, existing: Vec<u8>, new: Vec<u8>) -> EditorResult<Vec<u8>> {
        let existing_cs: Vec<editor_crdt::Changeset<editor_model::DocOp>> =
            editor_crdt::wire::decode(&existing[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let new_cs: Vec<editor_crdt::Changeset<editor_model::DocOp>> =
            editor_crdt::wire::decode(&new[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;

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

        let bytes =
            editor_crdt::wire::encode(&out).map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    pub fn missing_for(
        &self,
        all_changesets: Vec<u8>,
        remote_heads_payload: Vec<u8>,
    ) -> EditorResult<Vec<u8>> {
        let cs: Vec<editor_crdt::Changeset<editor_model::DocOp>> =
            editor_crdt::wire::decode(&all_changesets[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let heads_vec = editor_crdt::wire::decode_dots(&remote_heads_payload[..])
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let heads_set: hashbrown::HashSet<editor_crdt::Dot> = heads_vec.into_iter().collect();

        let g = editor_crdt::OpGraph::from_changesets(cs)?;
        let missing = g.missing_changesets_for(&heads_set)?;

        let bytes = editor_crdt::wire::encode(&missing)
            .map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    pub fn to_graph(&self, plain: Complex<editor_model::PlainDoc>) -> EditorResult<Vec<u8>> {
        let plain: editor_model::PlainDoc = plain.from_ffi()?;
        let (_, graph) = editor_model::Doc::from_plain(plain);
        let bytes = editor_crdt::wire::encode(&graph.changesets_as_vec())
            .map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    pub fn to_plain(
        &self,
        changeset_payloads: Vec<u8>,
    ) -> EditorResult<Complex<editor_model::PlainDoc>> {
        let cs: Vec<editor_crdt::Changeset<editor_model::DocOp>> =
            editor_crdt::wire::decode(&changeset_payloads[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let graph = editor_crdt::OpGraph::from_changesets(cs)?;
        let doc = editor_model::Doc::from_op_graph(&graph)?;
        Ok(doc.to_plain().into_ffi()?)
    }

    pub fn heads(&self, changeset_payloads: Vec<u8>) -> EditorResult<Vec<u8>> {
        let cs: Vec<editor_crdt::Changeset<editor_model::DocOp>> =
            editor_crdt::wire::decode(&changeset_payloads[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let g = editor_crdt::OpGraph::from_changesets(cs)?;
        let heads: Vec<editor_crdt::Dot> = g.current_heads().copied().collect();
        let bytes = editor_crdt::wire::encode_dots(&heads)
            .map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    /// Returns the total ops count in a Changesets bundle. Used by push light validation.
    pub fn peek_changeset_ops_count(&self, bundle: Vec<u8>) -> EditorResult<u32> {
        let cs: Vec<editor_crdt::Changeset<editor_model::DocOp>> =
            editor_crdt::wire::decode(&bundle[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let count: u32 = cs.iter().map(|c| c.ops.len() as u32).sum();
        Ok(count)
    }

    /// Verifies a PlainDoc's structural invariants (root uniqueness, tree reciprocity).
    pub fn verify_plain(&self, plain: Complex<editor_model::PlainDoc>) -> EditorResult<()> {
        let plain: editor_model::PlainDoc = plain.from_ffi()?;
        let (doc, _) = editor_model::Doc::from_plain(plain);
        doc.verify().map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::{Changeset, Dot, Op};
    use editor_model::{DocOp, NodeId, NodeType};

    use super::*;
    use crate::error::EditorError;

    fn dummy_payload() -> DocOp {
        let id = NodeId::new();
        DocOp::Presence {
            node_id: id,
            op: editor_crdt::OrMapOp::Set {
                key: id,
                value: NodeType::Paragraph,
            },
        }
    }

    fn enc_css(css: &[Changeset<DocOp>]) -> Vec<u8> {
        editor_crdt::wire::encode(css).unwrap()
    }
    fn dec_css(b: &[u8]) -> Vec<Changeset<DocOp>> {
        editor_crdt::wire::decode(b).unwrap()
    }
    fn enc_dots(dots: &[Dot]) -> Vec<u8> {
        editor_crdt::wire::encode_dots(dots).unwrap()
    }
    fn dec_dots(b: &[u8]) -> Vec<Dot> {
        editor_crdt::wire::decode_dots(b).unwrap()
    }

    fn load_test_font() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/Pretendard-Regular.ttf"
        ))
        .expect("test font not found")
    }

    #[test]
    fn apply_concatenates_distinct_changesets() {
        let cs_a = Changeset::<DocOp> {
            ops: vec![Op {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let cs_b = Changeset::<DocOp> {
            ops: vec![Op {
                id: Dot::new(2, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let server = EditorServer;
        let merged_bytes = server
            .apply(enc_css(&[cs_a.clone()]), enc_css(&[cs_b.clone()]))
            .unwrap();
        let merged = dec_css(&merged_bytes);
        assert_eq!(merged, vec![cs_a, cs_b]);
    }

    #[test]
    fn apply_skips_full_duplicate_changesets() {
        let cs = Changeset::<DocOp> {
            ops: vec![Op {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let server = EditorServer;
        let merged_bytes = server
            .apply(enc_css(&[cs.clone()]), enc_css(&[cs.clone()]))
            .unwrap();
        let merged = dec_css(&merged_bytes);
        assert_eq!(merged, vec![cs]);
    }

    #[test]
    fn apply_dedups_duplicates_within_new_payload() {
        let cs = Changeset::<DocOp> {
            ops: vec![Op {
                id: Dot::new(7, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let server = EditorServer;
        let merged_bytes = server
            .apply(enc_css(&[]), enc_css(&[cs.clone(), cs.clone()]))
            .unwrap();
        let merged = dec_css(&merged_bytes);
        assert_eq!(merged, vec![cs]);
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
        let parent_cs = Changeset::<DocOp> { ops: vec![parent] };
        let child_cs = Changeset::<DocOp> { ops: vec![child] };
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
        let parent_cs = Changeset::<DocOp> { ops: vec![parent] };
        let child_cs = Changeset::<DocOp> { ops: vec![child] };
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
        let cs = Changeset::<DocOp> {
            ops: vec![op1, op2],
        };
        let server = EditorServer;
        let merged_bytes = server.apply(enc_css(&[]), enc_css(&[cs.clone()])).unwrap();
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
        let cs = Changeset::<DocOp> { ops: vec![bad] };
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
        let cs_a = Changeset::<DocOp> {
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
        let cs_bad = Changeset::<DocOp> {
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
        let cs_a = Changeset::<DocOp> {
            ops: vec![Op {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let cs_b = Changeset::<DocOp> {
            ops: vec![Op {
                id: Dot::new(2, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let known_heads = vec![Dot::new(1, 0)];

        let server = EditorServer;
        let missing_bytes = server
            .missing_for(
                enc_css(&[cs_a.clone(), cs_b.clone()]),
                enc_dots(&known_heads),
            )
            .unwrap();
        let missing = dec_css(&missing_bytes);
        assert_eq!(missing, vec![cs_b]);
    }

    #[test]
    fn heads_returns_dot_set() {
        let cs_a = Changeset::<DocOp> {
            ops: vec![Op {
                id: Dot::new(1, 0),
                parents: vec![],
                payload: dummy_payload(),
            }],
        };
        let server = EditorServer;
        let heads_bytes = server.heads(enc_css(&[cs_a.clone()])).unwrap();
        let heads = dec_dots(&heads_bytes);
        assert_eq!(heads, vec![Dot::new(1, 0)]);
    }

    #[test]
    fn apply_rejects_same_dot_different_content() {
        let dot = Dot::new(11, 0);
        let id = NodeId::default();
        let payload_a = DocOp::Presence {
            node_id: id,
            op: editor_crdt::OrMapOp::Set {
                key: id,
                value: NodeType::Paragraph,
            },
        };
        let payload_b = DocOp::Presence {
            node_id: id,
            op: editor_crdt::OrMapOp::Set {
                key: id,
                value: NodeType::Text,
            },
        };
        let cs_v1 = Changeset::<DocOp> {
            ops: vec![Op {
                id: dot,
                parents: vec![],
                payload: payload_a,
            }],
        };
        let cs_v2 = Changeset::<DocOp> {
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

    #[test]
    fn outline_text_to_svg_forwards_svg_document() {
        // FFI 래퍼가 outline_text_to_svg 결과를 그대로 전달해 SVG 문서를 반환하는지 확인한다.
        let server = EditorServer;
        let svg = server
            .outline_text_to_svg(load_test_font(), "A".to_string())
            .unwrap();
        assert!(svg.starts_with(r#"<svg xmlns="http://www.w3.org/2000/svg""#));
        assert!(svg.contains("<path d=\""));
    }

    #[test]
    fn outline_text_to_svg_rejects_invalid_font_data() {
        // 잘못된 폰트 바이트를 넘기면 FFI 래퍼도 에러를 그대로 반환해야 한다.
        let server = EditorServer;
        let result = server.outline_text_to_svg(vec![0, 1, 2, 3], "A".to_string());
        assert!(result.is_err());
    }
}
