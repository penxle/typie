use editor_crdt::{Dot, LwwRegOp, OrMapOp, RgaOp, TextOp};
use editor_model::{DocOp, Node, NodeId};
use editor_state::BatchedState;

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(node_id: NodeId, target_id: NodeId, offset: usize) -> Step {
    Step::SplitNode {
        node_id: target_id,
        offset,
        new_node_id: node_id,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    validations: &mut Vec<Validation>,
    node_id: NodeId,
    target_id: NodeId,
    _offset: usize,
) -> Result<(), StepError> {
    let (parent_id, parent_anchor_dots, source_presence_dots, content_to_move, target_end_dot) = {
        let source_entry = batched
            .doc
            .get_entry(node_id)
            .ok_or(StepError::NodeNotFound(node_id))?;
        let parent_id = (*source_entry.parent.get()).ok_or(StepError::NodeNotFound(node_id))?;

        let mut source_presence_dots: Vec<Dot> =
            batched.doc.nodes_tags_for(&node_id).copied().collect();
        source_presence_dots.sort_unstable();
        source_presence_dots.dedup();

        let parent_entry = batched
            .doc
            .get_entry(parent_id)
            .ok_or(StepError::NodeNotFound(parent_id))?;
        let parent_anchor_dots: Vec<Dot> = parent_entry
            .children
            .iter_with_dot()
            .filter(|&(_, &v)| v == node_id)
            .map(|(d, _)| d)
            .collect();

        let target_entry = batched
            .doc
            .get_entry(target_id)
            .ok_or(StepError::NodeNotFound(target_id))?;

        let target_end_dot: Option<Dot> = match &target_entry.node {
            Node::Text(t) => t.text.iter_with_dot().last().map(|(d, _)| d),
            _ => target_entry.children.iter_with_dot().last().map(|(d, _)| d),
        };

        let content_to_move = match &source_entry.node {
            Node::Text(t) => {
                let chars: Vec<(Dot, char)> = t.text.iter_with_dot().collect();
                ContentMove::Text { chars }
            }
            _ => {
                let children: Vec<(Dot, NodeId)> = source_entry
                    .children
                    .iter_with_dot()
                    .map(|(d, &id)| (d, id))
                    .collect();
                ContentMove::Children { children }
            }
        };

        (
            parent_id,
            parent_anchor_dots,
            source_presence_dots,
            content_to_move,
            target_end_dot,
        )
    };

    match content_to_move {
        ContentMove::Text { chars } => {
            let mut after = target_end_dot;
            for (_, ch) in &chars {
                let op_id = batched.apply(DocOp::Text {
                    node_id: target_id,
                    op: TextOp::InsertChar { ch: *ch, after },
                })?;
                after = Some(op_id);
            }
            for (target, _) in chars {
                batched.apply(DocOp::Text {
                    node_id,
                    op: TextOp::RemoveChar { observed: target },
                })?;
            }
        }
        ContentMove::Children { children } => {
            let mut after = target_end_dot;
            for (_, child_id) in &children {
                let op_id = batched.apply(DocOp::Children {
                    node_id: target_id,
                    op: RgaOp::Insert {
                        after,
                        value: *child_id,
                    },
                })?;
                batched.apply(DocOp::Parent {
                    node_id: *child_id,
                    op: LwwRegOp::Set {
                        value: Some(target_id),
                    },
                })?;
                after = Some(op_id);
            }
            for (target, _) in children {
                batched.apply(DocOp::Children {
                    node_id,
                    op: RgaOp::Remove { observed: target },
                })?;
            }
        }
    }
    if !source_presence_dots.is_empty() {
        batched.apply(DocOp::Presence {
            node_id,
            op: OrMapOp::Unset {
                observed: source_presence_dots,
            },
        })?;
    }
    for target in parent_anchor_dots {
        batched.apply(DocOp::Children {
            node_id: parent_id,
            op: RgaOp::Remove { observed: target },
        })?;
    }

    validations.push(Validation::Subtree(target_id));
    validations.push(Validation::Node(parent_id));
    Ok(())
}

enum ContentMove {
    Text { chars: Vec<(Dot, char)> },
    Children { children: Vec<(Dot, NodeId)> },
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use crate::test_utils::DocTestExt;
    use crate::{Step, Transaction};

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
        let new_state = step.apply(&state).unwrap().state;

        assert_eq!(new_state.text(t1).text.to_string(), "Hello World");
        assert!(!new_state.has_node(t2));
        assert_eq!(new_state.node(p1).children().count(), 1);
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
        let new_state = step.apply(&state).unwrap().state;

        assert_eq!(new_state.node(p1).children().count(), 3);
        let p1_children: Vec<_> = new_state
            .node(p1)
            .entry()
            .children
            .iter()
            .copied()
            .collect();
        assert_eq!(p1_children[0], t1);
        assert_eq!(p1_children[1], t2);
        assert_eq!(p1_children[2], t3);
        assert_eq!(*new_state.node(t2).entry().parent.get(), Some(p1));
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
        assert!(tr.merge_node(p1, bl1).is_err());
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
        assert_eq!(state3.node(p1).children().count(), 2);
    }
}
