use editor_crdt::{LwwRegOp, Op, OrMapOp, OrSetOp, RgaOp, TextOp};
use serde::{Deserialize, Serialize};

use crate::{
    AnchorKind, Doc, ModelError, Modifier, ModifierType, Node, NodeAttr, NodeEntry, NodeId,
    NodeType, StyleEntry,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, editor_macros::Wire)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocOp {
    #[wire(n(0))]
    Presence {
        #[wire(n(0))]
        node_id: NodeId,
        #[wire(n(1))]
        op: OrMapOp<NodeId, NodeType>,
    },
    #[wire(n(1))]
    Parent {
        #[wire(n(0))]
        node_id: NodeId,
        #[wire(n(1))]
        op: LwwRegOp<Option<NodeId>>,
    },
    #[wire(n(2))]
    Children {
        #[wire(n(0))]
        node_id: NodeId,
        #[wire(n(1))]
        op: RgaOp<NodeId>,
    },
    #[wire(n(3))]
    Text {
        #[wire(n(0))]
        node_id: NodeId,
        #[wire(n(1))]
        op: TextOp,
    },
    #[wire(n(4))]
    Modifier {
        #[wire(n(0))]
        node_id: NodeId,
        #[wire(n(1))]
        op: OrMapOp<ModifierType, Modifier>,
    },
    #[wire(n(5))]
    Attr {
        #[wire(n(0))]
        node_id: NodeId,
        #[wire(n(1))]
        op: NodeAttr,
    },
    #[wire(n(6))]
    NodeStyle {
        #[wire(n(0))]
        node_id: NodeId,
        #[wire(n(1))]
        op: LwwRegOp<Option<String>>,
    },
    #[wire(n(7))]
    Style {
        #[wire(n(0))]
        style_id: String,
        #[wire(n(1))]
        op: StyleOp,
    },
    #[wire(n(8))]
    NodeMarker {
        #[wire(n(0))]
        node_id: NodeId,
        #[wire(n(1))]
        op: LwwRegOp<Option<crate::marker::Marker>>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, editor_macros::Wire)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StyleOp {
    #[wire(n(0))]
    Name(#[wire(n(0))] LwwRegOp<String>),
    #[wire(n(1))]
    Modifiers(#[wire(n(0))] OrSetOp<Modifier>),
    #[wire(n(2))]
    Presence(#[wire(n(0))] OrMapOp<String, ()>),
}

pub fn apply_doc_op(mut doc: Doc, op: &Op<DocOp>) -> Result<Doc, ModelError> {
    match &op.payload {
        DocOp::Presence {
            node_id,
            op: presence_op,
        } => {
            if let OrMapOp::Set { key, .. } = presence_op
                && *node_id != *key
            {
                return Err(ModelError::PresenceKeyMismatch {
                    node_id: *node_id,
                    key: *key,
                });
            }

            // Same-kind re-apply is idempotent.
            if let OrMapOp::Set {
                key,
                value: incoming,
            } = presence_op
                && let Some(existing_entry) = doc.entries.get(key)
            {
                let existing_kind = existing_entry.node.as_type();
                if existing_kind != *incoming {
                    return Err(ModelError::PresenceKindConflict {
                        node_id: *key,
                        existing: existing_kind,
                        incoming: *incoming,
                    });
                }
            }

            doc.nodes = doc
                .nodes
                .apply(op.id, presence_op.clone())
                .expect("local apply");

            if let OrMapOp::Set { key, value } = presence_op
                && !doc.entries.contains_key(key)
            {
                doc.entries.insert(*key, NodeEntry::new(value.into_node()));
            }
        }
        DocOp::Parent {
            node_id,
            op: lww_op,
        } => {
            let entry = doc
                .entries
                .get_mut(node_id)
                .ok_or(ModelError::NodeNotFound(*node_id))?;
            entry.parent = entry
                .parent
                .apply(op.id, lww_op.clone())
                .expect("local apply");
        }
        DocOp::Children {
            node_id,
            op: rga_op,
        } => {
            let entry = doc
                .entries
                .get_mut(node_id)
                .ok_or(ModelError::NodeNotFound(*node_id))?;
            if let RgaOp::Insert {
                after: Some(anchor),
                ..
            } = rga_op
                && !entry.children.contains_dot(*anchor)
            {
                return Err(ModelError::OrphanAnchor {
                    node_id: *node_id,
                    anchor: *anchor,
                    kind: AnchorKind::Children,
                });
            }
            entry.children = entry
                .children
                .apply(op.id, rga_op.clone())
                .expect("local apply");
        }
        DocOp::Text {
            node_id,
            op: text_op,
        } => {
            let entry = doc
                .entries
                .get_mut(node_id)
                .ok_or(ModelError::NodeNotFound(*node_id))?;
            if let TextOp::InsertChar {
                after: Some(anchor),
                ..
            } = text_op
            {
                let Node::Text(t) = &entry.node else {
                    return Err(ModelError::AttrNodeKindMismatch);
                };
                if !t.text.contains_dot(*anchor) {
                    return Err(ModelError::OrphanAnchor {
                        node_id: *node_id,
                        anchor: *anchor,
                        kind: AnchorKind::Text,
                    });
                }
            }
            let Node::Text(t) = &mut entry.node else {
                return Err(ModelError::AttrNodeKindMismatch);
            };
            t.text = t.text.apply(op.id, text_op.clone()).expect("local apply");
        }
        DocOp::Modifier {
            node_id,
            op: ormap_op,
        } => {
            if let OrMapOp::Set { key, value } = ormap_op {
                let value_type = value.as_type();
                if *key != value_type {
                    return Err(ModelError::ModifierKeyMismatch {
                        node_id: *node_id,
                        key: *key,
                        value_type,
                    });
                }
            }
            let entry = doc
                .entries
                .get_mut(node_id)
                .ok_or(ModelError::NodeNotFound(*node_id))?;
            entry.modifiers = entry
                .modifiers
                .apply(op.id, ormap_op.clone())
                .expect("local apply");
        }
        DocOp::Attr {
            node_id,
            op: node_attr,
        } => {
            let entry = doc
                .entries
                .get_mut(node_id)
                .ok_or(ModelError::NodeNotFound(*node_id))?;
            entry.node.apply_attr(op.id, node_attr)?;
        }
        DocOp::NodeStyle {
            node_id,
            op: lww_op,
        } => {
            let entry = doc
                .entries
                .get_mut(node_id)
                .ok_or(ModelError::NodeNotFound(*node_id))?;
            entry.style = entry
                .style
                .apply(op.id, lww_op.clone())
                .expect("local apply");
        }
        DocOp::NodeMarker {
            node_id,
            op: lww_op,
        } => {
            let entry = doc
                .entries
                .get_mut(node_id)
                .ok_or(ModelError::NodeNotFound(*node_id))?;
            entry.marker = entry
                .marker
                .apply(op.id, lww_op.clone())
                .expect("local apply");
        }
        DocOp::Style {
            style_id,
            op: style_op,
        } => {
            if let StyleOp::Presence(OrMapOp::Set { key, .. }) = style_op
                && key != style_id
            {
                return Err(ModelError::StylePresenceKeyMismatch {
                    style_id: style_id.clone(),
                    key: key.clone(),
                });
            }

            match style_op {
                StyleOp::Presence(presence_op) => {
                    doc.styles = doc
                        .styles
                        .apply(op.id, presence_op.clone())
                        .expect("local apply");
                }
                StyleOp::Name(lww_op) => {
                    let entry = doc
                        .style_entries
                        .entry(style_id.clone())
                        .or_insert_with(StyleEntry::new);
                    entry.name = entry
                        .name
                        .apply(op.id, lww_op.clone())
                        .expect("local apply");
                }
                StyleOp::Modifiers(orset_op) => {
                    let entry = doc
                        .style_entries
                        .entry(style_id.clone())
                        .or_insert_with(StyleEntry::new);
                    entry.modifiers = entry
                        .modifiers
                        .apply(op.id, orset_op.clone())
                        .expect("local apply");
                }
            }
        }
    }
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{OpGraph, TextOp};

    fn first_op<P: Clone>(graph: &OpGraph<P>, payload: P) -> (OpGraph<P>, Op<P>) {
        graph.clone().add(payload).unwrap()
    }

    #[test]
    fn apply_presence_creates_entry() {
        let graph = OpGraph::<DocOp>::new();
        let id = NodeId::new();
        let (_g, op) = first_op(
            &graph,
            DocOp::Presence {
                node_id: id,
                op: OrMapOp::Set {
                    key: id,
                    value: NodeType::Paragraph,
                },
            },
        );
        let doc = apply_doc_op(Doc::empty(), &op).unwrap();
        assert!(doc.get_entry(id).is_some());
    }

    #[test]
    fn presence_kind_conflict_rejected() {
        let mut graph = OpGraph::<DocOp>::new();
        let id = NodeId::new();
        let (g, op1) = first_op(
            &graph,
            DocOp::Presence {
                node_id: id,
                op: OrMapOp::Set {
                    key: id,
                    value: NodeType::Paragraph,
                },
            },
        );
        graph = g;
        let doc = apply_doc_op(Doc::empty(), &op1).unwrap();

        let (_, op2) = first_op(
            &graph,
            DocOp::Presence {
                node_id: id,
                op: OrMapOp::Set {
                    key: id,
                    value: NodeType::Text,
                },
            },
        );
        let result = apply_doc_op(doc, &op2);
        assert!(matches!(
            result,
            Err(ModelError::PresenceKindConflict {
                existing: NodeType::Paragraph,
                incoming: NodeType::Text,
                ..
            })
        ));
    }

    #[test]
    fn presence_same_kind_idempotent() {
        let mut graph = OpGraph::<DocOp>::new();
        let id = NodeId::new();
        let (g, op1) = first_op(
            &graph,
            DocOp::Presence {
                node_id: id,
                op: OrMapOp::Set {
                    key: id,
                    value: NodeType::Paragraph,
                },
            },
        );
        graph = g;
        let doc = apply_doc_op(Doc::empty(), &op1).unwrap();

        let (_, op2) = first_op(
            &graph,
            DocOp::Presence {
                node_id: id,
                op: OrMapOp::Set {
                    key: id,
                    value: NodeType::Paragraph,
                },
            },
        );
        let doc2 = apply_doc_op(doc, &op2).unwrap();
        assert!(doc2.get_entry(id).is_some());
    }

    #[test]
    fn modifier_key_must_match_value_discriminant() {
        let mut graph = OpGraph::<DocOp>::new();
        let id = NodeId::new();
        let (g, presence) = first_op(
            &graph,
            DocOp::Presence {
                node_id: id,
                op: OrMapOp::Set {
                    key: id,
                    value: NodeType::Paragraph,
                },
            },
        );
        graph = g;
        let doc = apply_doc_op(Doc::empty(), &presence).unwrap();

        let (_, mod_op) = first_op(
            &graph,
            DocOp::Modifier {
                node_id: id,
                op: OrMapOp::Set {
                    key: ModifierType::Bold,
                    value: Modifier::Italic,
                },
            },
        );
        let result = apply_doc_op(doc, &mod_op);
        assert!(matches!(
            result,
            Err(ModelError::ModifierKeyMismatch {
                key: ModifierType::Bold,
                value_type: ModifierType::Italic,
                ..
            })
        ));
    }

    #[test]
    fn modifier_with_correct_key_succeeds() {
        let mut graph = OpGraph::<DocOp>::new();
        let id = NodeId::new();
        let (g, presence) = first_op(
            &graph,
            DocOp::Presence {
                node_id: id,
                op: OrMapOp::Set {
                    key: id,
                    value: NodeType::Paragraph,
                },
            },
        );
        graph = g;
        let doc = apply_doc_op(Doc::empty(), &presence).unwrap();

        let (_, mod_op) = first_op(
            &graph,
            DocOp::Modifier {
                node_id: id,
                op: OrMapOp::Set {
                    key: ModifierType::Bold,
                    value: Modifier::Bold,
                },
            },
        );
        let doc2 = apply_doc_op(doc, &mod_op).unwrap();
        assert!(
            doc2.get_entry(id)
                .unwrap()
                .modifiers
                .contains_key(&ModifierType::Bold)
        );
    }

    #[test]
    fn presence_key_mismatch_rejected() {
        let graph = OpGraph::<DocOp>::new();
        let id = NodeId::new();
        let other_id = NodeId::new();
        let (_, op) = first_op(
            &graph,
            DocOp::Presence {
                node_id: id,
                op: OrMapOp::Set {
                    key: other_id,
                    value: NodeType::Paragraph,
                },
            },
        );
        let result = apply_doc_op(Doc::empty(), &op);
        assert!(matches!(
            result,
            Err(ModelError::PresenceKeyMismatch { .. })
        ));
    }

    #[test]
    fn parent_node_not_found_returns_error() {
        let graph = OpGraph::<DocOp>::new();
        let id = NodeId::new();
        let parent_id = NodeId::new();
        let (_, op) = first_op(
            &graph,
            DocOp::Parent {
                node_id: id,
                op: LwwRegOp::Set {
                    value: Some(parent_id),
                },
            },
        );
        let result = apply_doc_op(Doc::empty(), &op);
        assert!(matches!(result, Err(ModelError::NodeNotFound(_))));
    }

    #[test]
    fn attr_dispatch_via_apply_doc_op() {
        let mut graph = OpGraph::<DocOp>::new();
        let id = NodeId::new();
        let (g, presence) = first_op(
            &graph,
            DocOp::Presence {
                node_id: id,
                op: OrMapOp::Set {
                    key: id,
                    value: NodeType::Callout,
                },
            },
        );
        graph = g;
        let doc = apply_doc_op(Doc::empty(), &presence).unwrap();

        use crate::{CalloutNodeAttr, CalloutVariant};
        let (_, attr_op) = first_op(
            &graph,
            DocOp::Attr {
                node_id: id,
                op: NodeAttr::Callout {
                    attr: CalloutNodeAttr::Variant(CalloutVariant::Warning),
                },
            },
        );
        let doc2 = apply_doc_op(doc, &attr_op).unwrap();
        if let Node::Callout(n) = &doc2.get_entry(id).unwrap().node {
            assert_eq!(*n.variant.get(), CalloutVariant::Warning);
        } else {
            panic!("expected Callout node");
        }
    }

    #[test]
    fn children_with_orphan_anchor_returns_error() {
        let mut graph = OpGraph::<DocOp>::new();
        let root_id = NodeId::new();
        let child_id = NodeId::new();

        let (g, presence_root) = first_op(
            &graph,
            DocOp::Presence {
                node_id: root_id,
                op: OrMapOp::Set {
                    key: root_id,
                    value: NodeType::Root,
                },
            },
        );
        graph = g;
        let doc = apply_doc_op(Doc::empty(), &presence_root).unwrap();

        let (g, presence_child) = first_op(
            &graph,
            DocOp::Presence {
                node_id: child_id,
                op: OrMapOp::Set {
                    key: child_id,
                    value: NodeType::Paragraph,
                },
            },
        );
        graph = g;
        let doc = apply_doc_op(doc, &presence_child).unwrap();

        let bad_anchor = editor_crdt::Dot::new(999, 999);
        let (_, bad_children_op) = first_op(
            &graph,
            DocOp::Children {
                node_id: root_id,
                op: RgaOp::Insert {
                    after: Some(bad_anchor),
                    value: child_id,
                },
            },
        );
        let result = apply_doc_op(doc, &bad_children_op);
        assert!(matches!(
            result,
            Err(ModelError::OrphanAnchor {
                kind: AnchorKind::Children,
                ..
            })
        ));
    }

    #[test]
    fn text_with_orphan_anchor_returns_error() {
        let mut graph = OpGraph::<DocOp>::new();
        let id = NodeId::new();

        let (g, presence) = first_op(
            &graph,
            DocOp::Presence {
                node_id: id,
                op: OrMapOp::Set {
                    key: id,
                    value: NodeType::Text,
                },
            },
        );
        graph = g;
        let doc = apply_doc_op(Doc::empty(), &presence).unwrap();

        let bad_anchor = editor_crdt::Dot::new(999, 999);
        let (_, bad_text_op) = first_op(
            &graph,
            DocOp::Text {
                node_id: id,
                op: TextOp::InsertChar {
                    after: Some(bad_anchor),
                    ch: 'x',
                },
            },
        );
        let result = apply_doc_op(doc, &bad_text_op);
        assert!(matches!(
            result,
            Err(ModelError::OrphanAnchor {
                kind: AnchorKind::Text,
                ..
            })
        ));
    }

    #[test]
    fn text_apply_inserts_chars() {
        let mut graph = OpGraph::<DocOp>::new();
        let id = NodeId::new();
        let (g, presence) = first_op(
            &graph,
            DocOp::Presence {
                node_id: id,
                op: OrMapOp::Set {
                    key: id,
                    value: NodeType::Text,
                },
            },
        );
        graph = g;
        let doc = apply_doc_op(Doc::empty(), &presence).unwrap();

        let (g, op1) = first_op(
            &graph,
            DocOp::Text {
                node_id: id,
                op: TextOp::InsertChar {
                    after: None,
                    ch: 'h',
                },
            },
        );
        graph = g;
        let doc = apply_doc_op(doc, &op1).unwrap();
        let dot1 = op1.id;

        let (_, op2) = first_op(
            &graph,
            DocOp::Text {
                node_id: id,
                op: TextOp::InsertChar {
                    after: Some(dot1),
                    ch: 'i',
                },
            },
        );
        let doc = apply_doc_op(doc, &op2).unwrap();

        if let Node::Text(t) = &doc.get_entry(id).unwrap().node {
            assert_eq!(t.text.to_string(), "hi");
        } else {
            panic!("expected Text node");
        }
    }

    #[test]
    fn wire_round_trip_text_op() {
        use editor_crdt::Dot;
        use editor_crdt::wire::{CollectCtx, DecCtx, EncCtx, Wire};
        let dot = Dot::new(7, 5);
        let op = DocOp::Text {
            node_id: NodeId::new(),
            op: editor_crdt::TextOp::InsertChar {
                after: Some(dot),
                ch: '가',
            },
        };
        let mut cc = CollectCtx::new();
        <DocOp as Wire>::collect(&op, &mut cc);
        let (table, baselines) = cc.finalize();
        let ec = EncCtx::from_table(&table, baselines.clone());
        let dc = DecCtx {
            actor_table: table,
            baselines,
        };
        let mut buf = Vec::new();
        <DocOp as Wire>::encode(&op, &ec, &mut buf).unwrap();
        let mut slice = &buf[..];
        let got = <DocOp as Wire>::decode(&dc, &mut slice).unwrap();
        assert_eq!(got, op);
    }

    fn build_typing(text: &str) -> Vec<editor_crdt::Changeset<DocOp>> {
        let mut g = OpGraph::with_actor(1);
        let para = NodeId::new();
        let mut prev = None;
        for ch in text.chars() {
            let payload = DocOp::Text {
                node_id: para,
                op: TextOp::InsertChar { after: prev, ch },
            };
            let (ng, op) = g.add(payload).unwrap();
            prev = Some(op.id);
            g = ng;
        }
        g.commit().changesets_as_vec()
    }

    #[test]
    fn run_grouping_round_trip() {
        let css = build_typing("hello, world!");
        let bytes = editor_crdt::wire::encode(&css).unwrap();
        let decoded: Vec<editor_crdt::Changeset<DocOp>> =
            editor_crdt::wire::decode(&bytes).unwrap();
        assert_eq!(decoded, css);
    }

    #[test]
    fn run_grouping_collapses_typing_into_one_entry() {
        let css = build_typing("hello, world!");
        assert_eq!(css.len(), 1);
        let bytes = editor_crdt::wire::encode(&css).unwrap();
        let body = editor_crdt::wire::envelope::unwrap(&bytes).unwrap();
        let mut input = &body[..];
        let _dc = editor_crdt::wire::preamble::decode_preamble(&mut input).unwrap();
        let cs_count = editor_crdt::wire::varint::read_varint(&mut input).unwrap();
        assert_eq!(cs_count, 1);
        let parent_count = editor_crdt::wire::varint::read_varint(&mut input).unwrap();
        for _ in 0..parent_count {
            let _ = editor_crdt::wire::varint::read_varint(&mut input).unwrap();
            let _ = editor_crdt::wire::varint::read_varint(&mut input).unwrap();
        }
        let entry_count = editor_crdt::wire::varint::read_varint(&mut input).unwrap();
        assert_eq!(
            entry_count, 1,
            "13-char typing should fold into one run entry"
        );
    }

    #[test]
    fn hangul_round_trip() {
        let css = build_typing("안녕하세요!");
        let bytes = editor_crdt::wire::encode(&css).unwrap();
        let decoded: Vec<editor_crdt::Changeset<DocOp>> =
            editor_crdt::wire::decode(&bytes).unwrap();
        assert_eq!(decoded, css);
    }

    #[test]
    fn single_op_round_trip() {
        let css = build_typing("a");
        let bytes = editor_crdt::wire::encode(&css).unwrap();
        let decoded: Vec<editor_crdt::Changeset<DocOp>> =
            editor_crdt::wire::decode(&bytes).unwrap();
        assert_eq!(decoded, css);
    }

    fn build_changeset(payloads: Vec<DocOp>) -> Vec<editor_crdt::Changeset<DocOp>> {
        let mut g = OpGraph::with_actor(1);
        for p in payloads {
            let (ng, _) = g.add(p).unwrap();
            g = ng;
        }
        g.commit().changesets_as_vec()
    }

    fn assert_round_trip(payloads: Vec<DocOp>) {
        let css = build_changeset(payloads);
        let bytes = editor_crdt::wire::encode(&css).unwrap();
        let decoded: Vec<editor_crdt::Changeset<DocOp>> =
            editor_crdt::wire::decode(&bytes).unwrap();
        assert_eq!(decoded, css);
    }

    fn style_op_variants() -> Vec<StyleOp> {
        use editor_crdt::{LwwRegOp, OrMapOp, OrSetOp};
        vec![
            StyleOp::Name(LwwRegOp::Set {
                value: "heading".to_string(),
            }),
            StyleOp::Modifiers(OrSetOp::Add {
                elem: Modifier::Bold,
            }),
            StyleOp::Modifiers(OrSetOp::Remove {
                observed: editor_crdt::Dot::new(3, 9),
            }),
            StyleOp::Presence(OrMapOp::Set {
                key: "sid".to_string(),
                value: (),
            }),
            StyleOp::Presence(OrMapOp::Unset {
                observed: vec![editor_crdt::Dot::new(3, 9)],
            }),
        ]
    }

    #[test]
    fn style_escape_round_trip_explicit_node_id() {
        for op in style_op_variants() {
            let id = NodeId::new();
            assert_round_trip(vec![
                DocOp::Presence {
                    node_id: id,
                    op: editor_crdt::OrMapOp::Set {
                        key: id,
                        value: NodeType::Paragraph,
                    },
                },
                DocOp::Style {
                    style_id: "sid".to_string(),
                    op,
                },
            ]);
        }
    }

    #[test]
    fn style_escape_round_trip_implicit_node_id() {
        for op in style_op_variants() {
            assert_round_trip(vec![
                DocOp::Style {
                    style_id: "sid".to_string(),
                    op: StyleOp::Name(editor_crdt::LwwRegOp::Set {
                        value: "first".to_string(),
                    }),
                },
                DocOp::Style {
                    style_id: "sid".to_string(),
                    op,
                },
            ]);
        }
    }

    #[test]
    fn non_escape_variant_round_trip() {
        let id = NodeId::new();
        assert_round_trip(vec![
            DocOp::Presence {
                node_id: id,
                op: editor_crdt::OrMapOp::Set {
                    key: id,
                    value: NodeType::Paragraph,
                },
            },
            DocOp::Modifier {
                node_id: id,
                op: editor_crdt::OrMapOp::Set {
                    key: ModifierType::Bold,
                    value: Modifier::Bold,
                },
            },
            DocOp::NodeStyle {
                node_id: id,
                op: editor_crdt::LwwRegOp::Set {
                    value: Some("style".to_string()),
                },
            },
        ]);
    }

    #[test]
    fn node_marker_op_wire_round_trip() {
        use crate::marker::Marker;
        let some_marker = || DocOp::NodeMarker {
            node_id: NodeId::ROOT,
            op: editor_crdt::LwwRegOp::Set {
                value: Some(Marker {
                    modifiers: vec![Modifier::Bold],
                    style: Some("s1".to_string()),
                }),
            },
        };
        let none_marker = || DocOp::NodeMarker {
            node_id: NodeId::ROOT,
            op: editor_crdt::LwwRegOp::Set { value: None },
        };

        let id = NodeId::new();
        assert_round_trip(vec![
            DocOp::Presence {
                node_id: id,
                op: editor_crdt::OrMapOp::Set {
                    key: id,
                    value: NodeType::Paragraph,
                },
            },
            DocOp::NodeMarker {
                node_id: id,
                op: editor_crdt::LwwRegOp::Set {
                    value: Some(Marker {
                        modifiers: vec![Modifier::Bold],
                        style: Some("s1".to_string()),
                    }),
                },
            },
            DocOp::NodeMarker {
                node_id: id,
                op: editor_crdt::LwwRegOp::Set { value: None },
            },
        ]);

        assert_round_trip(vec![some_marker(), none_marker()]);
    }

    #[test]
    fn empty_bundle_round_trip() {
        let bytes = editor_crdt::wire::encode::<DocOp>(&[]).unwrap();
        assert!(bytes.is_empty());
        let decoded: Vec<editor_crdt::Changeset<DocOp>> =
            editor_crdt::wire::decode(&bytes).unwrap();
        assert!(decoded.is_empty());
    }
}

use editor_crdt::Dot;
use editor_crdt::wire::{CollectCtx, DecCtx, EncCtx, WireChangeset, WireError, WireResult, varint};

const VARIANT_PRESENCE: u8 = 0;
const VARIANT_PARENT: u8 = 1;
const VARIANT_CHILDREN: u8 = 2;
const VARIANT_TEXT: u8 = 3;
const VARIANT_MODIFIER: u8 = 4;
const VARIANT_ATTR: u8 = 5;
const VARIANT_NODE_STYLE: u8 = 6;

const VARIANT_ESCAPE: u8 = 7;
const EXT_STYLE: u8 = 0;
const EXT_MARKER: u8 = 1;

const VARIANT_STYLE: u8 = 8 + EXT_STYLE;
const VARIANT_NODE_MARKER: u8 = 8 + EXT_MARKER;

const ENTRY_TAG_RUN_BIT: u8 = 0b1000_0000;
const ENTRY_TAG_NODE_ID_EXPLICIT: u8 = 0b0100_0000;
const ENTRY_TAG_VARIANT_MASK: u8 = 0b0011_1000;
const ENTRY_TAG_VARIANT_SHIFT: u32 = 3;
const ENTRY_TAG_SUBFLAG_MASK: u8 = 0b0000_0111;

/// Per-bundle state carried across changesets so the first entry of cs[i>0] can omit
/// its `node_id` when it matches the last entry of cs[i-1] (cross-cs implicit node_id).
#[derive(Default)]
pub struct DocOpBundleState {
    prev_node_id: Option<NodeId>,
}

impl WireChangeset for DocOp {
    type BundleState = DocOpBundleState;

    fn collect_changeset(ops: &[Op<Self>], ctx: &mut CollectCtx) {
        for op in ops {
            ctx.observe(&op.id);
            for p in &op.parents {
                ctx.observe(p);
            }
            <DocOp as editor_crdt::wire::Wire>::collect(&op.payload, ctx);
        }
    }

    fn encode_changeset(
        ops: &[Op<Self>],
        state: &mut Self::BundleState,
        ctx: &EncCtx,
        out: &mut Vec<u8>,
    ) -> WireResult<u32> {
        if ops.is_empty() {
            return Err(WireError::EmptyChangesetOps);
        }
        for (i, op) in ops.iter().enumerate().skip(1) {
            let expected = ops[i - 1].id;
            if op.parents.len() != 1 || op.parents[0] != expected {
                return Err(WireError::ParentChainViolation {
                    cs_idx: 0,
                    op_idx: i,
                    parents: op.parents.clone(),
                    expected,
                });
            }
        }

        let mut prev_node_id = state.prev_node_id;
        let mut entries: Vec<Entry> = Vec::new();
        let mut i = 0;
        while i < ops.len() {
            if let Some(run_len) = try_match_text_run(ops, i) {
                entries.push(Entry::Run {
                    start: i,
                    len: run_len,
                });
                i += run_len;
            } else {
                entries.push(Entry::Single { idx: i });
                i += 1;
            }
        }

        for entry in &entries {
            match entry {
                Entry::Single { idx } => {
                    encode_single_op_entry(&ops[*idx], &mut prev_node_id, ctx, out)?;
                }
                Entry::Run { start, len } => {
                    encode_text_run_entry(
                        &ops[*start..*start + *len],
                        &mut prev_node_id,
                        ctx,
                        out,
                    )?;
                }
            }
        }
        state.prev_node_id = prev_node_id;
        Ok(entries.len() as u32)
    }

    fn decode_changeset(
        state: &mut Self::BundleState,
        ctx: &DecCtx,
        first_op_parents: Vec<Dot>,
        entry_count: u32,
        input: &mut &[u8],
    ) -> WireResult<Vec<Op<Self>>> {
        if entry_count == 0 {
            return Err(WireError::EmptyChangesetEntries);
        }
        let mut ops: Vec<Op<DocOp>> = Vec::new();
        let mut prev_node_id = state.prev_node_id;
        let mut prev_id: Option<Dot> = None;

        for entry_idx in 0..entry_count {
            let tag = <u8 as editor_crdt::wire::Wire>::decode(ctx, input)?;
            let is_run = (tag & ENTRY_TAG_RUN_BIT) != 0;
            let node_id_explicit = (tag & ENTRY_TAG_NODE_ID_EXPLICIT) != 0;
            // Implicit only allowed when the bundle has carried over a node_id from a prior cs
            // or a prior entry within this cs. The very first entry of the bundle (state.prev_node_id == None)
            // and entry 0 of this cs without prior prev_node_id must be explicit.
            if !node_id_explicit && prev_node_id.is_none() {
                return Err(WireError::FirstEntryImplicitNodeId);
            }
            if is_run && (tag & 0b0011_1111) != 0 {
                return Err(WireError::RunTagBitsNonZero { tag });
            }

            let variant = if is_run {
                0
            } else {
                let tag_variant = (tag & ENTRY_TAG_VARIANT_MASK) >> ENTRY_TAG_VARIANT_SHIFT;
                if tag_variant == VARIANT_ESCAPE {
                    let ext = <u8 as editor_crdt::wire::Wire>::decode(ctx, input)?;
                    match ext {
                        EXT_STYLE => VARIANT_STYLE,
                        EXT_MARKER => VARIANT_NODE_MARKER,
                        n => return Err(WireError::UnknownPayloadVariant { tag: n }),
                    }
                } else {
                    tag_variant
                }
            };

            let node_id = if node_id_explicit {
                let nid = <NodeId as editor_crdt::wire::Wire>::decode(ctx, input)?;
                prev_node_id = Some(nid);
                nid
            } else {
                prev_node_id.unwrap()
            };

            if is_run {
                let run_ops = decode_text_run_entry(
                    ctx,
                    node_id,
                    prev_id,
                    if entry_idx == 0 {
                        Some(&first_op_parents)
                    } else {
                        None
                    },
                    input,
                )?;
                if let Some(last) = run_ops.last() {
                    prev_id = Some(last.id);
                }
                ops.extend(run_ops);
            } else {
                let subflag = tag & ENTRY_TAG_SUBFLAG_MASK;
                let op = decode_single_op_entry(
                    ctx,
                    variant,
                    subflag,
                    node_id,
                    if entry_idx == 0 {
                        Some(&first_op_parents)
                    } else {
                        None
                    },
                    prev_id,
                    input,
                )?;
                prev_id = Some(op.id);
                ops.push(op);
            }
        }

        state.prev_node_id = prev_node_id;
        Ok(ops)
    }
}

enum Entry {
    Single { idx: usize },
    Run { start: usize, len: usize },
}

fn try_match_text_run(ops: &[Op<DocOp>], start: usize) -> Option<usize> {
    let first = match &ops[start].payload {
        DocOp::Text {
            node_id,
            op: editor_crdt::TextOp::InsertChar { after, .. },
        } => (*node_id, *after, ops[start].id),
        _ => return None,
    };
    let actor = first.2.actor;
    let base_clock = first.2.clock;
    let mut len = 1;
    let mut prev_id = first.2;
    while start + len < ops.len() {
        let op = &ops[start + len];
        let DocOp::Text {
            node_id,
            op: editor_crdt::TextOp::InsertChar { after, .. },
        } = &op.payload
        else {
            break;
        };
        if *node_id != first.0 {
            break;
        }
        let expected_clock = match base_clock.checked_add(len as u64) {
            Some(c) => c,
            None => break,
        };
        if op.id.actor != actor || op.id.clock != expected_clock {
            break;
        }
        if *after != Some(prev_id) {
            break;
        }
        prev_id = op.id;
        len += 1;
    }
    if len >= 2 { Some(len) } else { None }
}

fn encode_single_op_entry(
    op: &Op<DocOp>,
    prev_node_id: &mut Option<NodeId>,
    ctx: &EncCtx,
    out: &mut Vec<u8>,
) -> WireResult<()> {
    let (variant, subflag, node_id) = describe_doc_op(&op.payload);
    let (tag_variant, ext) = match variant {
        VARIANT_STYLE => (VARIANT_ESCAPE, Some(EXT_STYLE)),
        VARIANT_NODE_MARKER => (VARIANT_ESCAPE, Some(EXT_MARKER)),
        v => (v, None),
    };
    let node_id_explicit = !matches!(prev_node_id, Some(prev) if *prev == node_id);
    let mut tag: u8 = 0;
    if node_id_explicit {
        tag |= ENTRY_TAG_NODE_ID_EXPLICIT;
    }
    tag |= (tag_variant << ENTRY_TAG_VARIANT_SHIFT) & ENTRY_TAG_VARIANT_MASK;
    tag |= subflag & ENTRY_TAG_SUBFLAG_MASK;
    out.push(tag);
    if let Some(ext) = ext {
        out.push(ext);
    }
    if node_id_explicit {
        <NodeId as editor_crdt::wire::Wire>::encode(&node_id, ctx, out)?;
        *prev_node_id = Some(node_id);
    }
    <Dot as editor_crdt::wire::Wire>::encode(&op.id, ctx, out)?;
    encode_doc_op_payload(&op.payload, ctx, out, node_id)?;
    Ok(())
}

fn encode_text_run_entry(
    run: &[Op<DocOp>],
    prev_node_id: &mut Option<NodeId>,
    ctx: &EncCtx,
    out: &mut Vec<u8>,
) -> WireResult<()> {
    let (node_id, first_after) = match &run[0].payload {
        DocOp::Text {
            node_id,
            op: editor_crdt::TextOp::InsertChar { after, .. },
        } => (*node_id, *after),
        _ => unreachable!("try_match_text_run guarantees Text/InsertChar"),
    };
    let mut chars = String::new();
    for op in run {
        let DocOp::Text {
            op: editor_crdt::TextOp::InsertChar { ch, .. },
            ..
        } = &op.payload
        else {
            unreachable!("try_match_text_run guarantees Text/InsertChar")
        };
        chars.push(*ch);
    }
    let node_id_explicit = !matches!(prev_node_id, Some(prev) if *prev == node_id);
    let mut tag: u8 = ENTRY_TAG_RUN_BIT;
    if node_id_explicit {
        tag |= ENTRY_TAG_NODE_ID_EXPLICIT;
    }
    out.push(tag);
    if node_id_explicit {
        <NodeId as editor_crdt::wire::Wire>::encode(&node_id, ctx, out)?;
        *prev_node_id = Some(node_id);
    }
    <Dot as editor_crdt::wire::Wire>::encode(&run[0].id, ctx, out)?;
    varint::write_varint(run.len() as u64, out);
    match first_after {
        None => out.push(0),
        Some(d) => {
            out.push(1);
            <Dot as editor_crdt::wire::Wire>::encode(&d, ctx, out)?;
        }
    }
    out.extend_from_slice(chars.as_bytes());
    Ok(())
}

fn describe_doc_op(p: &DocOp) -> (u8, u8, NodeId) {
    match p {
        DocOp::Presence { node_id, op } => {
            let sub = match op {
                editor_crdt::OrMapOp::Set { .. } => 0,
                editor_crdt::OrMapOp::Unset { .. } => 1,
            };
            (VARIANT_PRESENCE, sub, *node_id)
        }
        DocOp::Parent { node_id, op } => {
            let editor_crdt::LwwRegOp::Set { value } = op;
            let sub = if value.is_some() { 1 } else { 0 };
            (VARIANT_PARENT, sub, *node_id)
        }
        DocOp::Children { node_id, op } => {
            let sub = match op {
                editor_crdt::RgaOp::Insert { after, .. } => {
                    if after.is_some() {
                        0b010
                    } else {
                        0
                    }
                }
                editor_crdt::RgaOp::Remove { .. } => 1,
            };
            (VARIANT_CHILDREN, sub, *node_id)
        }
        DocOp::Text { node_id, op } => {
            let sub = match op {
                editor_crdt::TextOp::InsertChar { after, .. } => {
                    if after.is_some() {
                        0b010
                    } else {
                        0
                    }
                }
                editor_crdt::TextOp::RemoveChar { .. } => 1,
            };
            (VARIANT_TEXT, sub, *node_id)
        }
        DocOp::Modifier { node_id, op } => {
            let sub = match op {
                editor_crdt::OrMapOp::Set { .. } => 0,
                editor_crdt::OrMapOp::Unset { .. } => 1,
            };
            (VARIANT_MODIFIER, sub, *node_id)
        }
        DocOp::Attr { node_id, .. } => (VARIANT_ATTR, 0, *node_id),
        DocOp::NodeStyle { node_id, op } => {
            let editor_crdt::LwwRegOp::Set { value } = op;
            let sub = if value.is_some() { 0 } else { 1 };
            (VARIANT_NODE_STYLE, sub, *node_id)
        }
        DocOp::NodeMarker { node_id, op } => {
            let editor_crdt::LwwRegOp::Set { value } = op;
            let sub = if value.is_some() { 0 } else { 1 };
            (VARIANT_NODE_MARKER, sub, *node_id)
        }
        DocOp::Style { op, .. } => {
            // Style variant carries its own style_id (String) inside the payload,
            // so the entry-level node_id slot is unused — use NodeId::ROOT as sentinel.
            let sub = match op {
                StyleOp::Name(_) => 0,
                StyleOp::Modifiers(editor_crdt::OrSetOp::Add { .. }) => 1,
                StyleOp::Modifiers(editor_crdt::OrSetOp::Remove { .. }) => 2,
                StyleOp::Presence(editor_crdt::OrMapOp::Set { .. }) => 3,
                StyleOp::Presence(editor_crdt::OrMapOp::Unset { .. }) => 4,
            };
            (VARIANT_STYLE, sub, NodeId::ROOT)
        }
    }
}

fn encode_doc_op_payload(
    p: &DocOp,
    ctx: &EncCtx,
    out: &mut Vec<u8>,
    outer_node_id: NodeId,
) -> WireResult<()> {
    use editor_crdt::wire::Wire;
    match p {
        DocOp::Presence { op, .. } => match op {
            editor_crdt::OrMapOp::Set { key, value } => {
                if *key != outer_node_id {
                    return Err(WireError::PresenceKeyMismatch {
                        key: key.raw(),
                        node_id: outer_node_id.raw(),
                    });
                }
                <NodeType as Wire>::encode(value, ctx, out)?;
            }
            editor_crdt::OrMapOp::Unset { observed } => {
                <Vec<Dot> as Wire>::encode(observed, ctx, out)?;
            }
        },
        DocOp::Parent { op, .. } => {
            let editor_crdt::LwwRegOp::Set { value } = op;
            if let Some(v) = value {
                <NodeId as Wire>::encode(v, ctx, out)?;
            }
        }
        DocOp::Children { op, .. } => match op {
            editor_crdt::RgaOp::Insert { after, value } => {
                if let Some(d) = after {
                    <Dot as Wire>::encode(d, ctx, out)?;
                }
                <NodeId as Wire>::encode(value, ctx, out)?;
            }
            editor_crdt::RgaOp::Remove { observed } => {
                <Dot as Wire>::encode(observed, ctx, out)?;
            }
        },
        DocOp::Text { op, .. } => match op {
            editor_crdt::TextOp::InsertChar { after, ch } => {
                if let Some(d) = after {
                    <Dot as Wire>::encode(d, ctx, out)?;
                }
                <char as Wire>::encode(ch, ctx, out)?;
            }
            editor_crdt::TextOp::RemoveChar { observed } => {
                <Dot as Wire>::encode(observed, ctx, out)?;
            }
        },
        DocOp::Modifier { op, .. } => match op {
            editor_crdt::OrMapOp::Set { key, value } => {
                <ModifierType as Wire>::encode(key, ctx, out)?;
                <Modifier as Wire>::encode(value, ctx, out)?;
            }
            editor_crdt::OrMapOp::Unset { observed } => {
                <Vec<Dot> as Wire>::encode(observed, ctx, out)?;
            }
        },
        DocOp::Attr { op, .. } => {
            <NodeAttr as Wire>::encode(op, ctx, out)?;
        }
        DocOp::NodeStyle { op, .. } => {
            let editor_crdt::LwwRegOp::Set { value } = op;
            if let Some(v) = value {
                <String as Wire>::encode(v, ctx, out)?;
            }
        }
        DocOp::NodeMarker { op, .. } => {
            let editor_crdt::LwwRegOp::Set { value } = op;
            if let Some(v) = value {
                <crate::marker::Marker as Wire>::encode(v, ctx, out)?;
            }
        }
        DocOp::Style { style_id, op } => {
            <String as Wire>::encode(style_id, ctx, out)?;
            match op {
                StyleOp::Name(editor_crdt::LwwRegOp::Set { value }) => {
                    <String as Wire>::encode(value, ctx, out)?;
                }
                StyleOp::Modifiers(editor_crdt::OrSetOp::Add { elem }) => {
                    <Modifier as Wire>::encode(elem, ctx, out)?;
                }
                StyleOp::Modifiers(editor_crdt::OrSetOp::Remove { observed }) => {
                    <Dot as Wire>::encode(observed, ctx, out)?;
                }
                StyleOp::Presence(editor_crdt::OrMapOp::Set { key, .. }) => {
                    if key != style_id {
                        return Err(WireError::StylePresenceKeyMismatch {
                            style_id: style_id.clone(),
                            key: key.clone(),
                        });
                    }
                }
                StyleOp::Presence(editor_crdt::OrMapOp::Unset { observed }) => {
                    <Vec<Dot> as Wire>::encode(observed, ctx, out)?;
                }
            }
        }
    }
    Ok(())
}

fn decode_single_op_entry(
    ctx: &DecCtx,
    variant: u8,
    subflag: u8,
    node_id: NodeId,
    first_op_parents: Option<&[Dot]>,
    prev_id: Option<Dot>,
    input: &mut &[u8],
) -> WireResult<Op<DocOp>> {
    let id = <Dot as editor_crdt::wire::Wire>::decode(ctx, input)?;
    let parents = match first_op_parents {
        Some(p) => p.to_vec(),
        None => vec![prev_id.expect("non-first op must have prev_id")],
    };
    let payload = decode_doc_op_payload(ctx, variant, subflag, node_id, input)?;
    Ok(Op {
        id,
        parents,
        payload,
    })
}

fn decode_doc_op_payload(
    ctx: &DecCtx,
    variant: u8,
    subflag: u8,
    node_id: NodeId,
    input: &mut &[u8],
) -> WireResult<DocOp> {
    use editor_crdt::wire::Wire;
    match variant {
        VARIANT_PRESENCE => {
            let op = if subflag & 1 == 0 {
                let value = <NodeType as Wire>::decode(ctx, input)?;
                editor_crdt::OrMapOp::Set {
                    key: node_id,
                    value,
                }
            } else {
                let observed = <Vec<Dot> as Wire>::decode(ctx, input)?;
                editor_crdt::OrMapOp::Unset { observed }
            };
            Ok(DocOp::Presence { node_id, op })
        }
        VARIANT_PARENT => {
            let value = if subflag & 1 != 0 {
                Some(<NodeId as Wire>::decode(ctx, input)?)
            } else {
                None
            };
            Ok(DocOp::Parent {
                node_id,
                op: editor_crdt::LwwRegOp::Set { value },
            })
        }
        VARIANT_CHILDREN => {
            if subflag & 1 == 0 {
                let after = if subflag & 0b010 != 0 {
                    Some(<Dot as Wire>::decode(ctx, input)?)
                } else {
                    None
                };
                let value = <NodeId as Wire>::decode(ctx, input)?;
                Ok(DocOp::Children {
                    node_id,
                    op: editor_crdt::RgaOp::Insert { after, value },
                })
            } else {
                let observed = <Dot as Wire>::decode(ctx, input)?;
                Ok(DocOp::Children {
                    node_id,
                    op: editor_crdt::RgaOp::Remove { observed },
                })
            }
        }
        VARIANT_TEXT => {
            if subflag & 1 == 0 {
                let after = if subflag & 0b010 != 0 {
                    Some(<Dot as Wire>::decode(ctx, input)?)
                } else {
                    None
                };
                let ch = <char as Wire>::decode(ctx, input)?;
                Ok(DocOp::Text {
                    node_id,
                    op: editor_crdt::TextOp::InsertChar { after, ch },
                })
            } else {
                let observed = <Dot as Wire>::decode(ctx, input)?;
                Ok(DocOp::Text {
                    node_id,
                    op: editor_crdt::TextOp::RemoveChar { observed },
                })
            }
        }
        VARIANT_MODIFIER => {
            let op = if subflag & 1 == 0 {
                let key = <ModifierType as Wire>::decode(ctx, input)?;
                let value = <Modifier as Wire>::decode(ctx, input)?;
                editor_crdt::OrMapOp::Set { key, value }
            } else {
                let observed = <Vec<Dot> as Wire>::decode(ctx, input)?;
                editor_crdt::OrMapOp::Unset { observed }
            };
            Ok(DocOp::Modifier { node_id, op })
        }
        VARIANT_ATTR => {
            let attr = <NodeAttr as Wire>::decode(ctx, input)?;
            Ok(DocOp::Attr { node_id, op: attr })
        }
        VARIANT_NODE_STYLE => {
            let value = if subflag & 1 == 0 {
                Some(<String as Wire>::decode(ctx, input)?)
            } else {
                None
            };
            Ok(DocOp::NodeStyle {
                node_id,
                op: editor_crdt::LwwRegOp::Set { value },
            })
        }
        VARIANT_NODE_MARKER => {
            let value = if subflag & 1 == 0 {
                Some(<crate::marker::Marker as Wire>::decode(ctx, input)?)
            } else {
                None
            };
            Ok(DocOp::NodeMarker {
                node_id,
                op: editor_crdt::LwwRegOp::Set { value },
            })
        }
        VARIANT_STYLE => {
            let style_id = <String as Wire>::decode(ctx, input)?;
            let op = match subflag {
                0 => {
                    let value = <String as Wire>::decode(ctx, input)?;
                    StyleOp::Name(editor_crdt::LwwRegOp::Set { value })
                }
                1 => {
                    let elem = <Modifier as Wire>::decode(ctx, input)?;
                    StyleOp::Modifiers(editor_crdt::OrSetOp::Add { elem })
                }
                2 => {
                    let observed = <Dot as Wire>::decode(ctx, input)?;
                    StyleOp::Modifiers(editor_crdt::OrSetOp::Remove { observed })
                }
                3 => StyleOp::Presence(editor_crdt::OrMapOp::Set {
                    key: style_id.clone(),
                    value: (),
                }),
                4 => {
                    let observed = <Vec<Dot> as Wire>::decode(ctx, input)?;
                    StyleOp::Presence(editor_crdt::OrMapOp::Unset { observed })
                }
                _ => return Err(WireError::UnknownPayloadVariant { tag: variant }),
            };
            Ok(DocOp::Style { style_id, op })
        }
        n => Err(WireError::UnknownPayloadVariant { tag: n }),
    }
}

fn decode_text_run_entry(
    ctx: &DecCtx,
    node_id: NodeId,
    prev_id_outer: Option<Dot>,
    first_op_parents: Option<&[Dot]>,
    input: &mut &[u8],
) -> WireResult<Vec<Op<DocOp>>> {
    let first_id = <Dot as editor_crdt::wire::Wire>::decode(ctx, input)?;
    let run_len = varint::read_varint(input)?;
    if run_len < 2 {
        return Err(WireError::InvalidRunLength { got: run_len });
    }
    let after_tag = <u8 as editor_crdt::wire::Wire>::decode(ctx, input)?;
    let first_after = match after_tag {
        0 => None,
        1 => Some(<Dot as editor_crdt::wire::Wire>::decode(ctx, input)?),
        n => {
            return Err(WireError::UnknownVariant {
                ty: "RunAfter",
                tag: n,
            });
        }
    };
    let mut chars = Vec::with_capacity(run_len as usize);
    for _ in 0..run_len {
        let ch = <char as editor_crdt::wire::Wire>::decode(ctx, input)?;
        chars.push(ch);
    }
    let mut ops = Vec::with_capacity(run_len as usize);
    let mut prev_id_inner: Option<Dot> = None;
    for (i, ch) in chars.iter().enumerate() {
        let i_u64 = i as u64;
        let clock = first_id
            .clock
            .checked_add(i_u64)
            .ok_or(WireError::ClockOverflow {
                context: "Text run expansion",
                base: first_id.clock,
                delta: i_u64,
            })?;
        let id = Dot::new(first_id.actor, clock);
        let after = if i == 0 {
            first_after
        } else {
            Some(prev_id_inner.unwrap())
        };
        let parents = if i == 0 {
            match first_op_parents {
                Some(p) => p.to_vec(),
                None => vec![prev_id_outer.unwrap()],
            }
        } else {
            vec![prev_id_inner.unwrap()]
        };
        ops.push(Op {
            id,
            parents,
            payload: DocOp::Text {
                node_id,
                op: editor_crdt::TextOp::InsertChar { after, ch: *ch },
            },
        });
        prev_id_inner = Some(id);
    }
    Ok(ops)
}
