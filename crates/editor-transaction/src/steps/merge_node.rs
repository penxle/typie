use editor_model::{Node, NodeId, TextNode};
use editor_state::State;

use crate::transform::Conflict;
use crate::{Step, StepError, StepOutput, Validation};

pub(crate) fn apply(
    state: &State,
    node_id: NodeId,
    target_id: NodeId,
) -> Result<StepOutput, StepError> {
    let node_entry = state
        .doc
        .get_entry(node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;
    let target_entry = state
        .doc
        .get_entry(target_id)
        .ok_or(StepError::NodeNotFound(target_id))?;
    let parent_id = node_entry.parent.ok_or(StepError::NodeNotFound(node_id))?;

    match (&target_entry.node, &node_entry.node) {
        (Node::Text(target_text), Node::Text(node_text)) => {
            let merged_text = format!("{}{}", target_text.text, node_text.text);

            let doc = state
                .doc
                .with_node_updated(target_id, |mut e| {
                    e.node = Node::Text(TextNode { text: merged_text });
                    e
                })
                .remove_node(node_id)
                .with_node_updated(parent_id, |mut e| {
                    let idx = e.children.iter().position(|&id| id == node_id).unwrap();
                    let mut children = e.children.clone();
                    children.remove(idx);
                    e.children = children;
                    e
                });

            let mut new_state = state.clone();
            new_state.doc = doc;

            Ok(StepOutput {
                state: new_state,
                validations: vec![Validation::Node(parent_id)],
            })
        }
        _ => {
            let moved_children = node_entry.children.clone();

            let mut doc = state
                .doc
                .with_node_updated(target_id, |mut e| {
                    let mut children = e.children.clone();
                    for child_id in &moved_children {
                        children.push_back(*child_id);
                    }
                    e.children = children;
                    e
                })
                .remove_node(node_id)
                .with_node_updated(parent_id, |mut e| {
                    let idx = e.children.iter().position(|&id| id == node_id).unwrap();
                    let mut children = e.children.clone();
                    children.remove(idx);
                    e.children = children;
                    e
                });

            for child_id in &moved_children {
                doc = doc.with_node_updated(*child_id, |mut e| {
                    e.parent = Some(target_id);
                    e
                });
            }

            let mut new_state = state.clone();
            new_state.doc = doc;

            Ok(StepOutput {
                state: new_state,
                validations: vec![Validation::Node(target_id), Validation::Node(parent_id)],
            })
        }
    }
}

pub(crate) fn inverse(node_id: NodeId, target_id: NodeId, offset: usize) -> Step {
    Step::SplitNode {
        node_id: target_id,
        offset,
        new_node_id: node_id,
    }
}

pub(crate) fn transform_against(
    local_node_id: NodeId,
    local_target_id: NodeId,
    local_offset: usize,
    against: &Step,
) -> Result<Vec<Step>, Conflict> {
    crate::transform::transform_default(
        Step::MergeNode {
            node_id: local_node_id,
            target_id: local_target_id,
            offset: local_offset,
        },
        against,
    )
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use crate::Transaction;
    use crate::test_utils::DocTestExt;
    use crate::*;

    #[test]
    fn merge_text_nodes() {
        let (state, p1, t1, t2) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("Hello") [bold]
                        t2: text(" World") [bold]
                    }
                }
            }
            selection: (t1, 0)
        };

        let step = Step::MergeNode {
            node_id: t2,
            target_id: t1,
            offset: 5,
        };

        let output = step.apply(&state).unwrap();
        let new_state = output.state;
        let merged = new_state.text(t1);

        assert_eq!(merged.text, "Hello World");
        assert!(!new_state.has_node(t2));
        assert_eq!(new_state.node(p1).children().len(), 1);
    }

    #[test]
    fn merge_element_nodes() {
        let (state, p1, t1, p2, t2, t3) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("A")
                    }
                    p2: paragraph {
                        t2: text("B")
                        t3: text("C")
                    }
                }
            }
            selection: (t1, 0)
        };

        let step = Step::MergeNode {
            node_id: p2,
            target_id: p1,
            offset: 1,
        };

        let output = step.apply(&state).unwrap();
        let new_state = output.state;
        let p1_ref = new_state.node(p1);
        let p1_children = &p1_ref.entry().children;

        assert_eq!(p1_children.len(), 3);
        assert_eq!(p1_children[0], t1);
        assert_eq!(p1_children[1], t2);
        assert_eq!(p1_children[2], t3);
        assert_eq!(new_state.node(t2).entry().parent, Some(p1));
        assert!(!new_state.has_node(p2));
    }

    #[test]
    fn merge_paragraph_into_bullet_list_content_violation() {
        let (state, bl1, _, p1) = state! {
            doc {
                root {
                    bl1: bullet_list {
                        list_item {
                            paragraph {
                                t1: text("A")
                            }
                        }
                    }
                    p1: paragraph {
                        text("B")
                    }
                }
            }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        let result = tr.merge_node(p1, bl1);

        assert!(result.is_err());
    }

    #[test]
    fn merge_then_split_roundtrip() {
        let (state, p1, t1, t2) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("Hello")
                        t2: text(" World")
                    }
                }
            }
            selection: (t1, 0)
        };

        let step = Step::MergeNode {
            node_id: t2,
            target_id: t1,
            offset: 5,
        };
        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;

        assert!(state3.has_node(t2));
        assert_eq!(state3.node(p1).children().len(), 2);
    }
}
