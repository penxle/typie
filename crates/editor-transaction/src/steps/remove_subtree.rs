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
    let node_id = subtree.id;
    state
        .doc
        .get_entry(node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;
    let parent = state
        .doc
        .get_entry(parent_id)
        .ok_or(StepError::NodeNotFound(parent_id))?;

    if index >= parent.children.len() {
        return Err(StepError::IndexOutOfBounds {
            parent_id,
            index,
            len: parent.children.len(),
        });
    }

    let node_ids = collect_ids(subtree);
    let mut doc = state.doc.clone();
    for id in node_ids {
        doc = doc.remove_node(id);
    }
    doc = doc.with_node_updated(parent_id, |mut parent| {
        let mut children = parent.children.clone();
        children.remove(index);
        parent.children = children;
        parent
    });

    let mut new_state = state.clone();
    new_state.doc = doc;

    Ok(StepOutput {
        state: new_state,
        validations: vec![Validation::Node(parent_id)],
    })
}

fn collect_ids(subtree: &Subtree) -> Vec<NodeId> {
    let mut ids = vec![subtree.id];
    for child in &subtree.children {
        ids.extend(collect_ids(child));
    }
    ids
}

pub(crate) fn inverse(parent_id: NodeId, index: usize, subtree: Subtree) -> Step {
    Step::InsertSubtree {
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
            Ok(vec![Step::RemoveSubtree {
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
            if *j == local_index {
                Ok(vec![])
            } else {
                let new_index = if *j < local_index {
                    local_index - 1
                } else {
                    local_index
                };
                Ok(vec![Step::RemoveSubtree {
                    parent_id: local_parent_id,
                    index: new_index,
                    subtree: local_subtree.clone(),
                }])
            }
        }
        _ => crate::transform::transform_default(
            Step::RemoveSubtree {
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
    use editor_state::*;

    use crate::{Step, Transaction};

    fn make_fold_state() -> (State, NodeId, NodeId, NodeId, NodeId, NodeId) {
        let (state, f1, ft1, t1, fc1, p1) = state! {
            doc {
                root {
                    f1: fold {
                        ft1: fold_title {
                            t1: text("Title")
                        }
                        fc1: fold_content {
                            p1: paragraph
                        }
                    }
                }
            }
            selection: (t1, 0)
        };
        (state, f1, ft1, fc1, t1, p1)
    }

    #[test]
    fn remove_fold_title_content_violation() {
        let (state, _, ft1, ..) = make_fold_state();

        let mut tr = Transaction::new(&state);
        let result = tr.remove_subtree(ft1);

        assert!(result.is_err());
    }

    #[test]
    fn remove_last_list_item_content_violation() {
        let (state, li1, ..) = state! {
            doc {
                root {
                    bullet_list {
                        li1: list_item {
                            paragraph {
                                t1: text("A")
                            }
                        }
                    }
                }
            }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        let result = tr.remove_subtree(li1);

        assert!(result.is_err());
    }

    fn paragraph_subtree(id: NodeId) -> Subtree {
        Subtree::leaf(id, Node::Paragraph(ParagraphNode::default()))
    }

    #[test]
    fn transform_remove_against_insert_before_shifts() {
        let parent = NodeId::new();
        let local = Step::RemoveSubtree {
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
        if let Step::RemoveSubtree { index, .. } = &out[0] {
            assert_eq!(*index, 4);
        } else {
            panic!("expected RemoveSubtree, got {:?}", out[0]);
        }
    }

    #[test]
    fn transform_remove_against_insert_at_unchanged() {
        let parent = NodeId::new();
        let local = Step::RemoveSubtree {
            parent_id: parent,
            index: 3,
            subtree: paragraph_subtree(NodeId::new()),
        };
        let against = Step::InsertSubtree {
            parent_id: parent,
            index: 5,
            subtree: paragraph_subtree(NodeId::new()),
        };
        let out = crate::transform::transform(&local, &against).unwrap();
        if let Step::RemoveSubtree { index, .. } = &out[0] {
            assert_eq!(*index, 3);
        } else {
            panic!("expected RemoveSubtree, got {:?}", out[0]);
        }
    }

    #[test]
    fn transform_remove_against_remove_before_shifts_back() {
        let parent = NodeId::new();
        let local = Step::RemoveSubtree {
            parent_id: parent,
            index: 3,
            subtree: paragraph_subtree(NodeId::new()),
        };
        let against = Step::RemoveSubtree {
            parent_id: parent,
            index: 1,
            subtree: paragraph_subtree(NodeId::new()),
        };
        let out = crate::transform::transform(&local, &against).unwrap();
        if let Step::RemoveSubtree { index, .. } = &out[0] {
            assert_eq!(*index, 2);
        } else {
            panic!("expected RemoveSubtree, got {:?}", out[0]);
        }
    }

    #[test]
    fn transform_remove_against_remove_same_index_eliminates() {
        let parent = NodeId::new();
        let id = NodeId::new();
        let subtree = paragraph_subtree(id);
        let local = Step::RemoveSubtree {
            parent_id: parent,
            index: 2,
            subtree: subtree.clone(),
        };
        let against = Step::RemoveSubtree {
            parent_id: parent,
            index: 2,
            subtree,
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            Vec::<Step>::new(),
        );
    }
}
