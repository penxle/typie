use editor_crdt::{Dot, LwwRegOp, OrMapOp, RgaOp, TextOp};
use editor_model::{DocOp, Modifier, Node, NodeAttr, NodeId};
use editor_state::BatchedState;

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(node_id: NodeId, offset: usize, new_node_id: NodeId) -> Step {
    Step::MergeNode {
        node_id: new_node_id,
        target_id: node_id,
        offset,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    validations: &mut Vec<Validation>,
    node_id: NodeId,
    offset: usize,
    new_node_id: NodeId,
) -> Result<(), StepError> {
    let (kind, parent_id, attrs, modifiers, style, parent_anchor_dot, content_split) = {
        let entry = batched
            .doc
            .get_entry(node_id)
            .ok_or(StepError::NodeNotFound(node_id))?;
        let parent_id = (*entry.parent.get()).ok_or(StepError::NodeNotFound(node_id))?;
        let kind = entry.node.as_type();
        let attrs: Vec<NodeAttr> = entry.node.to_plain().to_attrs();
        let modifiers: Vec<Modifier> = entry.modifiers.iter().map(|(_, m)| m.clone()).collect();
        let style: Option<String> = entry.style.get().clone();

        let parent_entry = batched
            .doc
            .get_entry(parent_id)
            .ok_or(StepError::NodeNotFound(parent_id))?;
        let parent_anchor_dot: Dot = parent_entry
            .children
            .iter_with_dot()
            .find(|&(_, &v)| v == node_id)
            .map(|(d, _)| d)
            .ok_or(StepError::NodeNotFound(node_id))?;

        let content_split = match &entry.node {
            Node::Text(text_node) => {
                let len = text_node.text.len();
                if offset > len {
                    return Err(StepError::OffsetOutOfBounds {
                        node_id,
                        offset,
                        len,
                    });
                }
                let tail_chars: Vec<(Dot, char)> =
                    text_node.text.iter_with_dot().skip(offset).collect();
                ContentSplit::Text { tail_chars }
            }
            _ => {
                let children_count = entry.children.iter_with_dot().count();
                if offset > children_count {
                    return Err(StepError::IndexOutOfBounds {
                        parent_id: node_id,
                        index: offset,
                        len: children_count,
                    });
                }
                let tail_children: Vec<(Dot, NodeId)> = entry
                    .children
                    .iter_with_dot()
                    .skip(offset)
                    .map(|(d, &id)| (d, id))
                    .collect();
                ContentSplit::Children { tail_children }
            }
        };

        (
            kind,
            parent_id,
            attrs,
            modifiers,
            style,
            parent_anchor_dot,
            content_split,
        )
    };

    batched.apply(DocOp::Presence {
        node_id: new_node_id,
        op: OrMapOp::Set {
            key: new_node_id,
            value: kind,
        },
    })?;
    for modifier in &modifiers {
        batched.apply(DocOp::Modifier {
            node_id: new_node_id,
            op: OrMapOp::Set {
                key: modifier.as_type(),
                value: modifier.clone(),
            },
        })?;
    }
    if let Some(style_id) = style {
        batched.apply(DocOp::NodeStyle {
            node_id: new_node_id,
            op: LwwRegOp::Set {
                value: Some(style_id),
            },
        })?;
    }
    for attr in attrs {
        batched.apply(DocOp::Attr {
            node_id: new_node_id,
            op: attr,
        })?;
    }
    batched.apply(DocOp::Parent {
        node_id: new_node_id,
        op: LwwRegOp::Set {
            value: Some(parent_id),
        },
    })?;
    batched.apply(DocOp::Children {
        node_id: parent_id,
        op: RgaOp::Insert {
            after: Some(parent_anchor_dot),
            value: new_node_id,
        },
    })?;
    match content_split {
        ContentSplit::Text { tail_chars } => {
            for (target, _) in &tail_chars {
                batched.apply(DocOp::Text {
                    node_id,
                    op: TextOp::RemoveChar { observed: *target },
                })?;
            }
            let mut after: Option<Dot> = None;
            for (_, ch) in tail_chars {
                let op_id = batched
                    .apply(DocOp::Text {
                        node_id: new_node_id,
                        op: TextOp::InsertChar { ch, after },
                    })?
                    .id;
                after = Some(op_id);
            }
        }
        ContentSplit::Children { tail_children } => {
            for (target, _) in &tail_children {
                batched.apply(DocOp::Children {
                    node_id,
                    op: RgaOp::Remove { observed: *target },
                })?;
            }
            let mut after: Option<Dot> = None;
            for (_, child_id) in tail_children {
                let op_id = batched
                    .apply(DocOp::Children {
                        node_id: new_node_id,
                        op: RgaOp::Insert {
                            after,
                            value: child_id,
                        },
                    })?
                    .id;
                batched.apply(DocOp::Parent {
                    node_id: child_id,
                    op: LwwRegOp::Set {
                        value: Some(new_node_id),
                    },
                })?;
                after = Some(op_id);
            }
        }
    }

    validations.push(Validation::Node(node_id));
    validations.push(Validation::Node(new_node_id));
    validations.push(Validation::Node(parent_id));
    Ok(())
}

enum ContentSplit {
    Text { tail_chars: Vec<(Dot, char)> },
    Children { tail_children: Vec<(Dot, NodeId)> },
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::{Modifier, NodeId};

    use crate::test_utils::DocTestExt;
    use crate::{Step, Transaction};

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
        let new_state = step.apply(&state).unwrap().state;

        assert_eq!(new_state.text(t1).text.to_string(), "Hello");
        assert_eq!(new_state.text(t2).text.to_string(), " World");
        assert!(new_state.node(t2).modifiers().any(|m| *m == Modifier::Bold));
        assert_eq!(new_state.node(p1).children().count(), 2);
        let p1_children: Vec<NodeId> = new_state
            .node(p1)
            .entry()
            .children
            .iter()
            .copied()
            .collect();
        assert_eq!(p1_children[0], t1);
        assert_eq!(p1_children[1], t2);
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
        let new_state = step.apply(&state).unwrap().state;

        assert_eq!(new_state.node(p1).children().count(), 1);
        let p1_children: Vec<NodeId> = new_state
            .node(p1)
            .entry()
            .children
            .iter()
            .copied()
            .collect();
        assert_eq!(p1_children[0], t1);

        assert_eq!(new_state.node(p2).children().count(), 2);
        let p2_children: Vec<NodeId> = new_state
            .node(p2)
            .entry()
            .children
            .iter()
            .copied()
            .collect();
        assert_eq!(p2_children[0], t2);
        assert_eq!(p2_children[1], t3);
        assert_eq!(*new_state.node(t2).entry().parent.get(), Some(p2));
        assert_eq!(*new_state.node(t3).entry().parent.get(), Some(p2));

        assert_eq!(new_state.node(NodeId::ROOT).children().count(), 2);
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

        let new_id = NodeId::new();
        let mut tr = Transaction::new(&state);
        assert!(tr.split_node(ft1, 0, new_id).is_err());
    }

    #[test]
    fn split_copies_style_ref_to_new_node() {
        use editor_model::PlainStyleEntry;
        let (initial, t1) = state! {
            doc { root { paragraph { t1: text("HelloWorld") } } }
            selection: (t1, 0)
        };
        let mut tr = Transaction::new(&initial);
        tr.set_style(
            "s1".into(),
            Some(PlainStyleEntry {
                name: "s".into(),
                modifiers: Default::default(),
            }),
        )
        .unwrap();
        tr.set_node_style(t1, Some("s1".into())).unwrap();
        let new_id = NodeId::new();
        tr.split_node(t1, 5, new_id).unwrap();
        let (next, ..) = tr.commit();

        assert_eq!(
            next.doc.get_entry(new_id).unwrap().style.get().as_deref(),
            Some("s1"),
            "split-off node must inherit the style ref"
        );
    }

    #[test]
    fn split_then_merge_text_roundtrip() {
        let (state, t1) = state! {
            doc {
                root {
                    paragraph {
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

        assert_eq!(state3.text(t1).text.to_string(), "Hello World");
        assert!(!state3.has_node(t2));
    }
}
