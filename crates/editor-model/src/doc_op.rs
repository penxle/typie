use editor_crdt::{LwwRegOp, Op, OrMapOp, RgaOp, TextOp};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::{
    AnchorKind, Doc, ModelError, Modifier, ModifierType, Node, NodeAttr, NodeEntry, NodeId,
    NodeType,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocOp {
    #[n(0)]
    Presence {
        #[n(0)]
        node_id: NodeId,
        #[n(1)]
        op: OrMapOp<NodeId, NodeType>,
    },
    #[n(1)]
    Parent {
        #[n(0)]
        node_id: NodeId,
        #[n(1)]
        op: LwwRegOp<Option<NodeId>>,
    },
    #[n(2)]
    Children {
        #[n(0)]
        node_id: NodeId,
        #[n(1)]
        op: RgaOp<NodeId>,
    },
    #[n(3)]
    Text {
        #[n(0)]
        node_id: NodeId,
        #[n(1)]
        op: TextOp,
    },
    #[n(4)]
    Modifier {
        #[n(0)]
        node_id: NodeId,
        #[n(1)]
        op: OrMapOp<ModifierType, Modifier>,
    },
    #[n(5)]
    Attr {
        #[n(0)]
        node_id: NodeId,
        #[n(1)]
        op: NodeAttr,
    },
}

pub fn apply_doc_op(mut doc: Doc, op: &Op<DocOp>) -> Result<Doc, ModelError> {
    match &op.payload {
        DocOp::Presence {
            node_id,
            op: presence_op,
        } => {
            if let OrMapOp::Set { key, .. } = presence_op {
                if *node_id != *key {
                    return Err(ModelError::PresenceKeyMismatch {
                        node_id: *node_id,
                        key: *key,
                    });
                }
            }

            // Same-kind re-apply is idempotent.
            if let OrMapOp::Set {
                key,
                value: incoming,
            } = presence_op
            {
                if let Some(existing_entry) = doc.entries.get(key) {
                    let existing_kind = existing_entry.node.as_type();
                    if existing_kind != *incoming {
                        return Err(ModelError::PresenceKindConflict {
                            node_id: *key,
                            existing: existing_kind,
                            incoming: *incoming,
                        });
                    }
                }
            }

            doc.nodes = doc
                .nodes
                .apply(op.id, presence_op.clone())
                .expect("local apply");

            if let OrMapOp::Set { key, value } = presence_op {
                if !doc.entries.contains_key(key) {
                    doc.entries.insert(*key, NodeEntry::new(value.into_node()));
                }
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
            {
                if !entry.children.contains_dot(*anchor) {
                    return Err(ModelError::OrphanAnchor {
                        node_id: *node_id,
                        anchor: *anchor,
                        kind: AnchorKind::Children,
                    });
                }
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
    }
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::OpGraph;

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
}
