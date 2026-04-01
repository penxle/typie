use editor_model::{NodeId, Subtree};
use editor_state::State;

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

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::*;
    use editor_state::*;

    use crate::Transaction;

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
}
