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
        let existing_cs: editor_crdt::Changesets<editor_model::DocOp> =
            minicbor::decode(&existing[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let new_cs: editor_crdt::Changesets<editor_model::DocOp> =
            minicbor::decode(&new[..]).map_err(|e| FfiError::Deserialization(e.to_string()))?;

        // Atomic boundaries make the first-op dot a stable changeset key, so
        // dedup and dot-reuse rejection only need to walk by first-op dot.
        let mut known: hashbrown::HashSet<editor_crdt::Dot> = existing_cs
            .0
            .iter()
            .flat_map(|cs| cs.ops.iter().map(|op| op.id))
            .collect();

        let mut out = existing_cs.0;
        for cs in new_cs.0 {
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

        let bytes = minicbor::to_vec(&editor_crdt::Changesets(out))
            .map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    pub fn missing_for(
        &self,
        all_changesets: Vec<u8>,
        remote_heads_payload: Vec<u8>,
    ) -> EditorResult<Vec<u8>> {
        let cs: editor_crdt::Changesets<editor_model::DocOp> =
            minicbor::decode(&all_changesets[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let heads: editor_crdt::Dots = minicbor::decode(&remote_heads_payload[..])
            .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let heads_set: hashbrown::HashSet<editor_crdt::Dot> = heads.0.into_iter().collect();

        let g = editor_crdt::OpGraph::from_changesets(cs.0)?;
        let missing = g.missing_changesets_for(&heads_set)?;

        if missing.is_empty() {
            return Ok(Vec::new());
        }

        let bytes = minicbor::to_vec(&editor_crdt::Changesets(missing))
            .map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    pub fn to_graph(&self, plain: Complex<editor_model::PlainDoc>) -> EditorResult<Vec<u8>> {
        let plain: editor_model::PlainDoc = plain.from_ffi()?;
        let (_, graph) = editor_model::Doc::from_plain(plain);
        let bytes = minicbor::to_vec(&editor_crdt::Changesets(graph.changesets().to_vec()))
            .map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    pub fn to_plain(
        &self,
        changeset_payloads: Vec<u8>,
    ) -> EditorResult<Complex<editor_model::PlainDoc>> {
        let cs: editor_crdt::Changesets<editor_model::DocOp> =
            minicbor::decode(&changeset_payloads[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let graph = editor_crdt::OpGraph::from_changesets(cs.0)?;
        let doc = editor_model::Doc::from_op_graph(&graph)?;
        Ok(doc.to_plain().into_ffi()?)
    }

    pub fn heads(&self, changeset_payloads: Vec<u8>) -> EditorResult<Vec<u8>> {
        let cs: editor_crdt::Changesets<editor_model::DocOp> =
            minicbor::decode(&changeset_payloads[..])
                .map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let g = editor_crdt::OpGraph::from_changesets(cs.0)?;
        let heads = editor_crdt::Dots(g.current_heads().copied().collect());
        let bytes = minicbor::to_vec(&heads).map_err(|e| FfiError::Serialization(e.to_string()))?;
        Ok(bytes)
    }

    /// Returns the total ops count in a Changesets bundle. Used by push light validation.
    pub fn peek_changeset_ops_count(&self, bundle: Vec<u8>) -> EditorResult<u32> {
        let cs: editor_crdt::Changesets<editor_model::DocOp> =
            minicbor::decode(&bundle[..]).map_err(|e| FfiError::Deserialization(e.to_string()))?;
        let count: u32 = cs.0.iter().map(|c| c.ops.len() as u32).sum();
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
    use editor_crdt::{Changeset, Changesets, Dot, Op};
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

    fn enc<T: minicbor::Encode<()>>(t: &T) -> Vec<u8> {
        minicbor::to_vec(t).unwrap()
    }
    fn dec<T: for<'a> minicbor::Decode<'a, ()>>(b: &[u8]) -> T {
        minicbor::decode(b).unwrap()
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
            .apply(
                enc(&Changesets(vec![cs_a.clone()])),
                enc(&Changesets(vec![cs_b.clone()])),
            )
            .unwrap();
        let merged: Changesets<DocOp> = dec(&merged_bytes);
        assert_eq!(merged.0, vec![cs_a, cs_b]);
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
            .apply(
                enc(&Changesets(vec![cs.clone()])),
                enc(&Changesets(vec![cs.clone()])),
            )
            .unwrap();
        let merged: Changesets<DocOp> = dec(&merged_bytes);
        assert_eq!(merged.0, vec![cs]);
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
            .apply(
                enc(&Changesets::<DocOp>(vec![])),
                enc(&Changesets(vec![cs.clone(), cs.clone()])),
            )
            .unwrap();
        let merged: Changesets<DocOp> = dec(&merged_bytes);
        assert_eq!(merged.0, vec![cs]);
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
        let result = server.apply(
            enc(&Changesets::<DocOp>(vec![])),
            enc(&Changesets(vec![child_cs, parent_cs])),
        );
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
                enc(&Changesets::<DocOp>(vec![])),
                enc(&Changesets(vec![parent_cs.clone(), child_cs.clone()])),
            )
            .unwrap();
        let merged: Changesets<DocOp> = dec(&merged_bytes);
        assert_eq!(merged.0, vec![parent_cs, child_cs]);
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
        let merged_bytes = server
            .apply(
                enc(&Changesets::<DocOp>(vec![])),
                enc(&Changesets(vec![cs.clone()])),
            )
            .unwrap();
        let merged: Changesets<DocOp> = dec(&merged_bytes);
        assert_eq!(merged.0, vec![cs]);
    }

    #[test]
    fn apply_rejects_intra_cs_child_before_parent() {
        let op_a = Op {
            id: Dot::new(8, 0),
            parents: vec![],
            payload: dummy_payload(),
        };
        let op_b = Op {
            id: Dot::new(8, 1),
            parents: vec![op_a.id],
            payload: dummy_payload(),
        };
        let cs = Changeset::<DocOp> {
            ops: vec![op_b, op_a],
        };
        let server = EditorServer;
        let result = server.apply(
            enc(&Changesets::<DocOp>(vec![])),
            enc(&Changesets(vec![cs])),
        );
        assert!(matches!(
            result,
            Err(EditorError::Ffi(FfiError::CausalOrderViolation { .. }))
        ));
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
        let result = server.apply(
            enc(&Changesets::<DocOp>(vec![])),
            enc(&Changesets(vec![cs])),
        );
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
        let result = server.apply(enc(&Changesets(vec![cs_a])), enc(&Changesets(vec![cs_bad])));
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
        let all = Changesets(vec![cs_a.clone(), cs_b.clone()]);
        let known_heads = editor_crdt::Dots(vec![Dot::new(1, 0)]);

        let server = EditorServer;
        let missing_bytes = server.missing_for(enc(&all), enc(&known_heads)).unwrap();
        let missing: Changesets<DocOp> = dec(&missing_bytes);
        assert_eq!(missing.0, vec![cs_b]);
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
        let heads_bytes = server.heads(enc(&Changesets(vec![cs_a.clone()]))).unwrap();
        let heads: editor_crdt::Dots = dec(&heads_bytes);
        assert_eq!(heads.0, vec![Dot::new(1, 0)]);
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
        let result = server.apply(enc(&Changesets(vec![cs_v1])), enc(&Changesets(vec![cs_v2])));
        assert!(matches!(
            result,
            Err(EditorError::Ffi(FfiError::CausalOrderViolation { .. }))
        ));
    }
}
