use editor_crdt::{CrdtError, Dot, LwwRegOp, OrMapOp, RgaOp, TextOp};
use editor_model::{DocOp, NodeId, PlainNode, Subtree};
use editor_state::BatchedState;

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(parent_id: NodeId, index: usize, subtree: Subtree) -> Step {
    Step::RemoveSubtree {
        parent_id,
        index,
        subtree,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    validations: &mut Vec<Validation>,
    parent_id: NodeId,
    index: usize,
    subtree: &Subtree,
) -> Result<(), StepError> {
    let anchor_dot: Option<Dot> = {
        let parent_entry = batched
            .doc
            .get_entry(parent_id)
            .ok_or(StepError::NodeNotFound(parent_id))?;
        parent_entry.children.dot_at(index).map_err(|e| match e {
            CrdtError::OffsetOutOfBounds { offset, len } => StepError::IndexOutOfBounds {
                parent_id,
                index: offset,
                len,
            },
            other => panic!("dot_at unexpected: {other:?}"),
        })?
    };

    emit_pass1(batched, subtree)?;
    emit_pass2(batched, subtree, parent_id, anchor_dot)?;

    validations.push(Validation::Subtree(subtree.id));
    validations.push(Validation::Node(parent_id));
    Ok(())
}

fn emit_pass1(batched: &mut BatchedState, subtree: &Subtree) -> Result<(), StepError> {
    batched.apply(DocOp::Presence {
        node_id: subtree.id,
        op: OrMapOp::Set {
            key: subtree.id,
            value: subtree.node.as_type(),
        },
    })?;
    for modifier in &subtree.modifiers {
        batched.apply(DocOp::Modifier {
            node_id: subtree.id,
            op: OrMapOp::Set {
                key: modifier.as_type(),
                value: modifier.clone(),
            },
        })?;
    }
    for attr in subtree.node.to_attrs() {
        batched.apply(DocOp::Attr {
            node_id: subtree.id,
            op: attr,
        })?;
    }
    if let PlainNode::Text(text_node) = &subtree.node {
        let mut after: Option<Dot> = None;
        for ch in text_node.text.chars() {
            let op_id = batched.apply(DocOp::Text {
                node_id: subtree.id,
                op: TextOp::InsertChar { ch, after },
            })?;
            after = Some(op_id);
        }
    }
    for child in &subtree.children {
        emit_pass1(batched, child)?;
    }
    Ok(())
}

fn emit_pass2(
    batched: &mut BatchedState,
    subtree: &Subtree,
    parent_id: NodeId,
    anchor: Option<Dot>,
) -> Result<Dot, StepError> {
    batched.apply(DocOp::Parent {
        node_id: subtree.id,
        op: LwwRegOp::Set {
            value: Some(parent_id),
        },
    })?;
    let emit_dot = batched.apply(DocOp::Children {
        node_id: parent_id,
        op: RgaOp::Insert {
            after: anchor,
            value: subtree.id,
        },
    })?;
    let mut sibling_after: Option<Dot> = None;
    for child in &subtree.children {
        let child_emit = emit_pass2(batched, child, subtree.id, sibling_after)?;
        sibling_after = Some(child_emit);
    }
    Ok(emit_dot)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::{
        NodeId, PlainBulletListNode, PlainListItemNode, PlainNode, PlainParagraphNode,
        PlainTableNode, Subtree,
    };

    use crate::test_utils::DocTestExt;
    use crate::{Step, Transaction};

    #[test]
    fn insert_subtree_apply() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let new_id = NodeId::new();
        let subtree = Subtree::leaf(new_id, PlainNode::Paragraph(PlainParagraphNode::default()));
        let step = Step::InsertSubtree {
            parent_id: NodeId::ROOT,
            index: 1,
            subtree,
        };
        let new_state = step.apply(&state).unwrap().state;

        assert!(new_state.has_node(new_id));
        assert_eq!(new_state.node(NodeId::ROOT).children().count(), 2);
        assert_eq!(
            *new_state.node(new_id).entry().parent.get(),
            Some(NodeId::ROOT)
        );
    }

    #[test]
    fn insert_subtree_index_out_of_bounds() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let new_id = NodeId::new();
        let step = Step::InsertSubtree {
            parent_id: NodeId::ROOT,
            index: 99,
            subtree: Subtree::leaf(new_id, PlainNode::Paragraph(PlainParagraphNode::default())),
        };
        assert!(step.apply(&state).is_err());
    }

    #[test]
    fn insert_subtree_content_violation() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let new_id = NodeId::new();
        let subtree = Subtree::leaf(new_id, PlainNode::Text(Default::default()));
        let mut tr = Transaction::new(&state);
        assert!(tr.insert_subtree(NodeId::ROOT, 0, subtree).is_err());
    }

    #[test]
    fn insert_subtree_context_violation() {
        let (state, tc1, ..) = state! {
            doc {
                root {
                    table {
                        table_row {
                            tc1: table_cell {
                                paragraph {
                                    t1: text("Hi")
                                }
                            }
                        }
                    }
                }
            }
            selection: (t1, 0)
        };

        let new_table_id = NodeId::new();
        let subtree = Subtree::leaf(new_table_id, PlainNode::Table(PlainTableNode::default()));
        let mut tr = Transaction::new(&state);
        assert!(tr.insert_subtree(tc1, 0, subtree).is_err());
    }

    #[test]
    fn insert_empty_container_fails_content_validation() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let list_id = NodeId::new();
        let subtree = Subtree::leaf(list_id, PlainNode::BulletList(PlainBulletListNode {}));
        let mut tr = Transaction::new(&state);
        assert!(tr.insert_subtree(NodeId::ROOT, 1, subtree).is_err());
    }

    #[test]
    fn insert_valid_subtree_succeeds() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let list_id = NodeId::new();
        let item_id = NodeId::new();
        let para_id = NodeId::new();
        let subtree = Subtree::leaf(list_id, PlainNode::BulletList(PlainBulletListNode {}))
            .with_children(vec![
                Subtree::leaf(item_id, PlainNode::ListItem(PlainListItemNode {})).with_children(
                    vec![Subtree::leaf(
                        para_id,
                        PlainNode::Paragraph(PlainParagraphNode::default()),
                    )],
                ),
            ]);
        let step = Step::InsertSubtree {
            parent_id: NodeId::ROOT,
            index: 0,
            subtree,
        };
        let new_state = step.apply(&state).unwrap().state;

        assert!(new_state.has_node(list_id));
        assert!(new_state.has_node(item_id));
        assert!(new_state.has_node(para_id));
        assert_eq!(new_state.node(NodeId::ROOT).children().count(), 2);
    }
}
