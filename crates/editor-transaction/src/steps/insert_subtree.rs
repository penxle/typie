use editor_model::{NodeId, Subtree};
use editor_state::State;

use crate::transform::Conflict;
use crate::{Step, StepError, StepOutput, Validation};

pub(crate) fn apply(
    state: &State,
    parent_id: NodeId,
    index: usize,
    subtree: &Subtree,
) -> Result<StepOutput, StepError> {
    let parent = state
        .doc
        .get_entry(parent_id)
        .ok_or(StepError::NodeNotFound(parent_id))?;

    if index > parent.children.len() {
        return Err(StepError::IndexOutOfBounds {
            parent_id,
            index,
            len: parent.children.len(),
        });
    }

    let entries = subtree.clone().into_entries(parent_id);
    let mut doc = state.doc.clone();
    for (id, entry) in entries {
        doc = doc.insert_node(id, entry);
    }
    doc = doc.with_node_updated(parent_id, |mut parent| {
        let mut children = parent.children.clone();
        children.insert(index, subtree.id);
        parent.children = children;
        parent
    });

    let mut new_state = state.clone();
    new_state.doc = doc;

    Ok(StepOutput {
        state: new_state,
        validations: vec![Validation::Node(parent_id), Validation::Subtree(subtree.id)],
    })
}

pub(crate) fn inverse(parent_id: NodeId, index: usize, subtree: Subtree) -> Step {
    Step::RemoveSubtree {
        parent_id,
        index,
        subtree,
    }
}

pub(crate) fn transform_against(
    local_parent_id: NodeId,
    local_index: usize,
    local_subtree: &Subtree,
    against: &Step,
) -> Result<Vec<Step>, Conflict> {
    match against {
        Step::InsertSubtree {
            parent_id,
            index: j,
            ..
        } if *parent_id == local_parent_id => {
            let new_index = if *j <= local_index {
                local_index + 1
            } else {
                local_index
            };
            Ok(vec![Step::InsertSubtree {
                parent_id: local_parent_id,
                index: new_index,
                subtree: local_subtree.clone(),
            }])
        }
        Step::RemoveSubtree {
            parent_id,
            index: j,
            ..
        } if *parent_id == local_parent_id => {
            let new_index = if *j < local_index {
                local_index - 1
            } else {
                local_index
            };
            Ok(vec![Step::InsertSubtree {
                parent_id: local_parent_id,
                index: new_index,
                subtree: local_subtree.clone(),
            }])
        }
        _ => crate::transform::transform_default(
            Step::InsertSubtree {
                parent_id: local_parent_id,
                index: local_index,
                subtree: local_subtree.clone(),
            },
            against,
        ),
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::*;

    use crate::Transaction;
    use crate::test_utils::DocTestExt;
    use crate::*;

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
        let subtree = Subtree::leaf(new_id, Node::Paragraph(ParagraphNode::default()));

        let step = Step::InsertSubtree {
            parent_id: NodeId::ROOT,
            index: 1,
            subtree,
        };

        let output = step.apply(&state).unwrap();
        let new_state = output.state;

        assert!(new_state.has_node(new_id));
        assert_eq!(new_state.node(NodeId::ROOT).children().len(), 2);
        assert_eq!(new_state.node(NodeId::ROOT).entry().children[1], new_id);
        assert_eq!(new_state.node(new_id).entry().parent, Some(NodeId::ROOT));
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
            subtree: Subtree::leaf(new_id, Node::Paragraph(ParagraphNode::default())),
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
        let subtree = Subtree::leaf(new_id, Node::Text(TextNode { text: "Bad".into() }));

        let mut tr = Transaction::new(&state);
        let result = tr.insert_subtree(NodeId::ROOT, 0, subtree);

        assert!(result.is_err());
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
        let subtree = Subtree::leaf(new_table_id, Node::Table(TableNode::default()));

        let mut tr = Transaction::new(&state);
        let result = tr.insert_subtree(tc1, 0, subtree);

        assert!(result.is_err());
    }

    #[test]
    fn insert_then_remove_roundtrip() {
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
        let subtree = Subtree::leaf(new_id, Node::Paragraph(ParagraphNode::default()));
        let step = Step::InsertSubtree {
            parent_id: NodeId::ROOT,
            index: 1,
            subtree,
        };
        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;
        assert!(!state3.has_node(new_id));
        assert_eq!(state3.node(NodeId::ROOT).children().len(), 1);
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
        // BulletList requires at least one ListItem child — empty should fail
        let subtree = Subtree::leaf(list_id, Node::BulletList(BulletListNode {}));

        let mut tr = Transaction::new(&state);
        let result = tr.insert_subtree(NodeId::ROOT, 1, subtree);

        assert!(result.is_err());
    }

    fn paragraph_subtree(id: NodeId) -> Subtree {
        Subtree::leaf(id, Node::Paragraph(ParagraphNode::default()))
    }

    #[test]
    fn transform_insert_against_insert_before_shifts_index() {
        let parent = NodeId::new();
        let local = Step::InsertSubtree {
            parent_id: parent,
            index: 3,
            subtree: paragraph_subtree(NodeId::new()),
        };
        let against = Step::InsertSubtree {
            parent_id: parent,
            index: 1,
            subtree: paragraph_subtree(NodeId::new()),
        };
        let out = crate::transform::transform(&local, &against).unwrap();
        assert_eq!(out.len(), 1);
        if let Step::InsertSubtree { index, .. } = &out[0] {
            assert_eq!(*index, 4);
        } else {
            panic!("expected InsertSubtree, got {:?}", out[0]);
        }
    }

    #[test]
    fn transform_insert_against_remove_before_shifts_back() {
        let parent = NodeId::new();
        let local = Step::InsertSubtree {
            parent_id: parent,
            index: 5,
            subtree: paragraph_subtree(NodeId::new()),
        };
        let against = Step::RemoveSubtree {
            parent_id: parent,
            index: 2,
            subtree: paragraph_subtree(NodeId::new()),
        };
        let out = crate::transform::transform(&local, &against).unwrap();
        if let Step::InsertSubtree { index, .. } = &out[0] {
            assert_eq!(*index, 4);
        } else {
            panic!("expected InsertSubtree, got {:?}", out[0]);
        }
    }

    #[test]
    fn transform_insert_against_remove_after_unchanged() {
        let parent = NodeId::new();
        let local = Step::InsertSubtree {
            parent_id: parent,
            index: 1,
            subtree: paragraph_subtree(NodeId::new()),
        };
        let against = Step::RemoveSubtree {
            parent_id: parent,
            index: 5,
            subtree: paragraph_subtree(NodeId::new()),
        };
        let out = crate::transform::transform(&local, &against).unwrap();
        if let Step::InsertSubtree { index, .. } = &out[0] {
            assert_eq!(*index, 1);
        } else {
            panic!("expected InsertSubtree, got {:?}", out[0]);
        }
    }

    #[test]
    fn transform_insert_different_parent_unchanged() {
        let parent_a = NodeId::new();
        let parent_b = NodeId::new();
        let local = Step::InsertSubtree {
            parent_id: parent_a,
            index: 3,
            subtree: paragraph_subtree(NodeId::new()),
        };
        let against = Step::InsertSubtree {
            parent_id: parent_b,
            index: 0,
            subtree: paragraph_subtree(NodeId::new()),
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            vec![local.clone()],
        );
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
        let subtree =
            Subtree::leaf(list_id, Node::BulletList(BulletListNode {})).with_children(vec![
                Subtree::leaf(item_id, Node::ListItem(ListItemNode {})).with_children(vec![
                    Subtree::leaf(para_id, Node::Paragraph(ParagraphNode::default())),
                ]),
            ]);

        // Insert before existing Paragraph so the trailing Paragraph requirement is satisfied
        let step = Step::InsertSubtree {
            parent_id: NodeId::ROOT,
            index: 0,
            subtree,
        };

        let output = step.apply(&state).unwrap();
        let new_state = output.state;
        assert!(new_state.has_node(list_id));
        assert!(new_state.has_node(item_id));
        assert!(new_state.has_node(para_id));
        assert_eq!(new_state.node(NodeId::ROOT).children().len(), 2);
    }
}
