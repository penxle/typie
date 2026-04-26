use editor_common::StrExt;
use editor_model::{Node, NodeEntry, NodeId, TextNode};
use editor_state::State;

use crate::{Mapping, Step, StepError, StepOutput, Validation};

pub(crate) fn apply(
    state: &State,
    node_id: NodeId,
    offset: usize,
    new_node_id: NodeId,
) -> Result<StepOutput, StepError> {
    let entry = state
        .doc
        .get_entry(node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;
    let parent_id = entry.parent.ok_or(StepError::NodeNotFound(node_id))?;

    match &entry.node {
        Node::Text(text_node) => {
            if offset > text_node.text.char_count() {
                return Err(StepError::OffsetOutOfBounds {
                    node_id,
                    offset,
                    len: text_node.text.char_count(),
                });
            }

            let byte_offset = text_node.text.nth_char_byte_offset(offset);
            let left_text = text_node.text[..byte_offset].to_string();
            let right_text = text_node.text[byte_offset..].to_string();

            let new_entry = NodeEntry {
                node: Node::Text(TextNode { text: right_text }),
                parent: Some(parent_id),
                children: editor_model::imbl::Vector::new(),
                modifiers: entry.modifiers.clone(),
            };

            let left = TextNode { text: left_text };

            let doc = state
                .doc
                .with_node_updated(node_id, |mut e| {
                    e.node = Node::Text(left);
                    e
                })
                .insert_node(new_node_id, new_entry)
                .with_node_updated(parent_id, |mut e| {
                    let idx = e.children.iter().position(|&id| id == node_id).unwrap();
                    let mut children = e.children.clone();
                    children.insert(idx + 1, new_node_id);
                    e.children = children;
                    e
                });

            let mut new_state = state.clone();
            new_state.doc = doc;

            Ok(StepOutput {
                state: new_state,
                mapping: Mapping::identity(),
                validations: vec![Validation::Node(parent_id)],
            })
        }
        _ => {
            if offset > entry.children.len() {
                return Err(StepError::IndexOutOfBounds {
                    parent_id: node_id,
                    index: offset,
                    len: entry.children.len(),
                });
            }

            let left_children: editor_model::imbl::Vector<NodeId> =
                entry.children.iter().copied().take(offset).collect();
            let right_children: editor_model::imbl::Vector<NodeId> =
                entry.children.iter().copied().skip(offset).collect();

            let new_entry = NodeEntry {
                node: entry.node.clone(),
                parent: Some(parent_id),
                children: right_children.clone(),
                modifiers: entry.modifiers.clone(),
            };

            let mut doc = state
                .doc
                .with_node_updated(node_id, |mut e| {
                    e.children = left_children;
                    e
                })
                .insert_node(new_node_id, new_entry)
                .with_node_updated(parent_id, |mut e| {
                    let idx = e.children.iter().position(|&id| id == node_id).unwrap();
                    let mut children = e.children.clone();
                    children.insert(idx + 1, new_node_id);
                    e.children = children;
                    e
                });

            for child_id in &right_children {
                doc = doc.with_node_updated(*child_id, |mut e| {
                    e.parent = Some(new_node_id);
                    e
                });
            }

            let mut new_state = state.clone();
            new_state.doc = doc;

            Ok(StepOutput {
                state: new_state,
                mapping: Mapping::identity(),
                validations: vec![
                    Validation::Node(node_id),
                    Validation::Node(new_node_id),
                    Validation::Node(parent_id),
                ],
            })
        }
    }
}

pub(crate) fn inverse(node_id: NodeId, offset: usize, new_node_id: NodeId) -> Step {
    Step::MergeNode {
        node_id: new_node_id,
        target_id: node_id,
        offset,
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
    fn split_text_node() {
        let (state, p1, t1) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("Hello World") [bold]
                    }
                }
            }
            selection: (t1, 0)
        };

        let t2 = NodeId::new();

        let step = Step::SplitNode {
            node_id: t1,
            offset: 5,
            new_node_id: t2,
        };
        let output = step.apply(&state).unwrap();
        let new_state = output.state;

        assert_eq!(new_state.text(t1).text, "Hello");
        assert_eq!(new_state.text(t2).text, " World");
        assert_eq!(
            new_state.doc.get_entry(t2).unwrap().modifiers,
            vec![Modifier::Bold]
        );

        assert_eq!(new_state.node(p1).children().len(), 2);
        assert_eq!(new_state.node(p1).entry().children[0], t1);
        assert_eq!(new_state.node(p1).entry().children[1], t2);
    }

    #[test]
    fn split_element_node() {
        let (state, p1, t1, t2, t3) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("Hello World") [bold]
                        t2: text("A")
                        t3: text("B")
                    }
                }
            }
            selection: (t1, 0)
        };

        let p2 = NodeId::new();
        let step = Step::SplitNode {
            node_id: p1,
            offset: 1,
            new_node_id: p2,
        };
        let output = step.apply(&state).unwrap();
        let new_state = output.state;

        assert_eq!(new_state.node(p1).children().len(), 1);
        assert_eq!(new_state.node(p1).entry().children[0], t1);

        assert_eq!(new_state.node(p2).children().len(), 2);
        assert_eq!(new_state.node(p2).entry().children[0], t2);
        assert_eq!(new_state.node(p2).entry().children[1], t3);
        assert_eq!(new_state.node(t2).entry().parent, Some(p2));
        assert_eq!(new_state.node(t3).entry().parent, Some(p2));

        assert_eq!(new_state.node(NodeId::ROOT).children().len(), 2);
    }

    #[test]
    fn split_fold_title_content_violation() {
        let (state, ft1, ..) = state! {
            doc {
                root {
                    fold {
                        ft1: fold_title {
                            t1: text("Title")
                        }
                        fold_content {
                            paragraph
                        }
                    }
                }
            }
            selection: (t1, 0)
        };

        let new_fold_title_id = NodeId::new();
        let mut tr = Transaction::new(&state);
        let result = tr.split_node(ft1, 0, new_fold_title_id);

        assert!(result.is_err());
    }

    #[test]
    fn split_then_merge_text_roundtrip() {
        let (state, _, t1) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("Hello World") [bold]
                    }
                }
            }
            selection: (t1, 0)
        };

        let t2 = NodeId::new();

        let step = Step::SplitNode {
            node_id: t1,
            offset: 5,
            new_node_id: t2,
        };
        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;

        assert_eq!(state3.text(t1).text, "Hello World");
        assert!(!state3.has_node(t2));
    }
}
