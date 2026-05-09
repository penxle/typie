use editor_model::{DocOp, NodeId, PlainNode};
use editor_state::BatchedState;

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(node_id: NodeId, old_node: PlainNode, new_node: PlainNode) -> Step {
    Step::SetNode {
        node_id,
        old_node: new_node,
        new_node: old_node,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    validations: &mut Vec<Validation>,
    node_id: NodeId,
    old_node: &PlainNode,
    new_node: &PlainNode,
) -> Result<(), StepError> {
    if old_node.as_type() != new_node.as_type() {
        return Err(StepError::ContentViolation {
            node_id,
            detail: format!(
                "SetNode is attr-only; type change ({:?} -> {:?}) requires a separate remove/insert sequence",
                old_node.as_type(),
                new_node.as_type(),
            ),
        });
    }
    if matches!(old_node, PlainNode::Text(_)) && matches!(new_node, PlainNode::Text(_)) {
        return Err(StepError::ContentViolation {
            node_id,
            detail: "SetNode is attr-only; TextNode content changes require InsertText/RemoveText"
                .into(),
        });
    }
    for attr in new_node.to_attrs() {
        batched.apply(DocOp::Attr { node_id, op: attr })?;
    }
    validations.push(Validation::Node(node_id));
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::{
        CalloutVariant, Node, NodeId, PlainCalloutNode, PlainImageNode, PlainNode,
        PlainTableRowNode,
    };

    use crate::test_utils::DocTestExt;
    use crate::{Step, Transaction};

    #[test]
    fn set_node_apply() {
        let (state, c1) = state! {
            doc {
                root {
                    c1: callout {
                        paragraph { text("Hello") }
                    }
                }
            }
            selection: (c1, 0)
        };

        let step = Step::SetNode {
            node_id: c1,
            old_node: PlainNode::Callout(PlainCalloutNode::default()),
            new_node: PlainNode::Callout(PlainCalloutNode {
                variant: CalloutVariant::Warning,
            }),
        };
        let new_state = step.apply(&state).unwrap().state;

        let Node::Callout(cn) = new_state.node(c1).node() else {
            panic!("expected Callout")
        };
        assert_eq!(*cn.variant.get(), CalloutVariant::Warning);
    }

    #[test]
    fn set_node_type_change_content_violation() {
        let (state, p1, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        assert!(
            tr.set_node(p1, PlainNode::TableRow(PlainTableRowNode {}))
                .is_err()
        );
    }

    #[test]
    fn set_node_children_incompatible() {
        let (state, p1, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        assert!(
            tr.set_node(p1, PlainNode::Image(PlainImageNode::default()))
                .is_err()
        );
    }

    #[test]
    fn set_node_not_found() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello")
                    }
                }
            }
            selection: (p1, 0)
        };

        let missing = NodeId::new();
        let step = Step::SetNode {
            node_id: missing,
            old_node: PlainNode::Callout(PlainCalloutNode::default()),
            new_node: PlainNode::Callout(PlainCalloutNode {
                variant: CalloutVariant::Warning,
            }),
        };
        assert!(step.apply(&state).is_err());
    }
}
