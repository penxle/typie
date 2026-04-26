use editor_model::NodeId;
use editor_state::State;

use crate::{Mapping, Step, StepError, StepOutput, Validation};

pub(crate) fn apply(
    state: &State,
    node_id: NodeId,
    old_parent: NodeId,
    old_index: usize,
    new_parent: NodeId,
    new_index: usize,
) -> Result<StepOutput, StepError> {
    state
        .doc
        .get_entry(node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;
    let old_p = state
        .doc
        .get_entry(old_parent)
        .ok_or(StepError::NodeNotFound(old_parent))?;

    if old_index >= old_p.children.len() {
        return Err(StepError::IndexOutOfBounds {
            parent_id: old_parent,
            index: old_index,
            len: old_p.children.len(),
        });
    }

    let doc = state.doc.with_node_updated(old_parent, |mut entry| {
        let mut children = entry.children.clone();
        children.remove(old_index);
        entry.children = children;
        entry
    });

    let new_p = doc
        .get_entry(new_parent)
        .ok_or(StepError::NodeNotFound(new_parent))?;
    if new_index > new_p.children.len() {
        return Err(StepError::IndexOutOfBounds {
            parent_id: new_parent,
            index: new_index,
            len: new_p.children.len(),
        });
    }

    let doc = doc.with_node_updated(new_parent, |mut entry| {
        let mut children = entry.children.clone();
        children.insert(new_index, node_id);
        entry.children = children;
        entry
    });

    let doc = doc.with_node_updated(node_id, |mut entry| {
        entry.parent = Some(new_parent);
        entry
    });

    let mut new_state = state.clone();
    new_state.doc = doc;

    let mut validations = vec![Validation::Node(old_parent), Validation::Subtree(node_id)];
    if new_parent != old_parent {
        validations.push(Validation::Node(new_parent));
    }

    Ok(StepOutput {
        state: new_state,
        mapping: Mapping::identity(),
        validations,
    })
}

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

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::*;

    use crate::Transaction;
    use crate::test_utils::DocTestExt;
    use crate::*;

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

        let output = step.apply(&state).unwrap();
        let new_state = output.state;

        assert_eq!(new_state.node(p1).children().len(), 0);
        assert_eq!(new_state.node(p2).children().len(), 2);
        assert_eq!(new_state.node(t1).entry().parent, Some(p2));
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

        let output = step.apply(&state).unwrap();
        let new_state = output.state;
        let root_ref = new_state.node(NodeId::ROOT);
        let root_children = &root_ref.entry().children;

        assert_eq!(root_children[0], p2);
        assert_eq!(root_children[1], p1);
    }

    #[test]
    fn move_node_content_violation() {
        let (state, _p1, t1) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("A")
                    }
                }
            }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        let result = tr.move_node(t1, NodeId::ROOT, 1);

        assert!(result.is_err());
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
        let result = tr.move_node(tb2, tc1, 1);

        assert!(result.is_err());
    }

    #[test]
    fn move_node_inverse_roundtrip() {
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

        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;

        assert_eq!(state3.node(p1).children().len(), 1);
        assert_eq!(state3.node(t1).entry().parent, Some(p1));
    }
}
