use editor_crdt::{CrdtError, Dot, LwwRegOp, RgaOp};
use editor_model::{DocOp, NodeId};
use editor_state::BatchedState;

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(
    node_id: NodeId,
    old_parent: NodeId,
    old_index: usize,
    new_parent: NodeId,
    new_index: usize,
) -> Step {
    Step::MoveNode {
        node_id,
        old_parent: new_parent,
        old_index: new_index,
        new_parent: old_parent,
        new_index: old_index,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    validations: &mut Vec<Validation>,
    node_id: NodeId,
    old_parent: NodeId,
    _old_index: usize,
    new_parent: NodeId,
    new_index: usize,
) -> Result<(), StepError> {
    let old_parent_dots: Vec<Dot> = {
        let old_entry = batched
            .doc
            .get_entry(old_parent)
            .ok_or(StepError::NodeNotFound(old_parent))?;
        let dots: Vec<Dot> = old_entry
            .children
            .iter_with_dot()
            .filter(|&(_, &v)| v == node_id)
            .map(|(d, _)| d)
            .collect();
        if dots.is_empty() {
            return Err(StepError::NodeNotFound(node_id));
        }
        dots
    };

    for target in old_parent_dots {
        batched.apply(DocOp::Children {
            node_id: old_parent,
            op: RgaOp::Remove { observed: target },
        })?;
    }

    let new_anchor_dot: Option<Dot> = {
        let new_entry = batched
            .doc
            .get_entry(new_parent)
            .ok_or(StepError::NodeNotFound(new_parent))?;
        new_entry.children.dot_at(new_index).map_err(|e| match e {
            CrdtError::OffsetOutOfBounds { offset, len } => StepError::IndexOutOfBounds {
                parent_id: new_parent,
                index: offset,
                len,
            },
            other => panic!("dot_at unexpected: {other:?}"),
        })?
    };

    batched.apply(DocOp::Children {
        node_id: new_parent,
        op: RgaOp::Insert {
            after: new_anchor_dot,
            value: node_id,
        },
    })?;
    batched.apply(DocOp::Parent {
        node_id,
        op: LwwRegOp::Set {
            value: Some(new_parent),
        },
    })?;

    validations.push(Validation::Subtree(node_id));
    validations.push(Validation::Node(old_parent));
    validations.push(Validation::Node(new_parent));
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::NodeId;

    use crate::test_utils::DocTestExt;
    use crate::{Step, Transaction};

    #[test]
    fn move_node_between_parents() {
        let (state, p1, t1, p2) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("A")
                    }
                    p2: paragraph {
                        text("B")
                    }
                }
            }
            selection: (t1, 0)
        };

        let step = Step::MoveNode {
            node_id: t1,
            old_parent: p1,
            old_index: 0,
            new_parent: p2,
            new_index: 1,
        };
        let new_state = step.apply(&state).unwrap().state;

        assert_eq!(new_state.node(p1).children().count(), 0);
        assert_eq!(new_state.node(p2).children().count(), 2);
        assert_eq!(*new_state.node(t1).entry().parent.get(), Some(p2));
    }

    #[test]
    fn move_node_within_same_parent() {
        let (state, p1, _, p2) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("A")
                    }
                    p2: paragraph {
                        text("B")
                    }
                }
            }
            selection: (t1, 0)
        };

        let step = Step::MoveNode {
            node_id: p1,
            old_parent: NodeId::ROOT,
            old_index: 0,
            new_parent: NodeId::ROOT,
            new_index: 1,
        };
        let new_state = step.apply(&state).unwrap().state;
        let root_children: Vec<NodeId> = new_state
            .node(NodeId::ROOT)
            .entry()
            .children
            .iter()
            .copied()
            .collect();

        assert_eq!(root_children[0], p2);
        assert_eq!(root_children[1], p1);
    }

    #[test]
    fn move_node_content_violation() {
        let (state, t1) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("A")
                    }
                }
            }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        assert!(tr.move_node(t1, NodeId::ROOT, 1).is_err());
    }

    #[test]
    fn move_node_context_deep_violation() {
        let (state, tc1, _, tb2) = state! {
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
                    tb2: table
                }
            }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        assert!(tr.move_node(tb2, tc1, 1).is_err());
    }
}
