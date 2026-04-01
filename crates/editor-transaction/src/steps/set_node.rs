use editor_model::{Node, NodeId};
use editor_state::State;

use crate::{Step, StepError, StepOutput, Validation};

pub(crate) fn apply(
    state: &State,
    node_id: NodeId,
    new_node: &Node,
) -> Result<StepOutput, StepError> {
    let entry = state
        .doc
        .get_entry(node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;

    let doc = state.doc.with_node_updated(node_id, |mut e| {
        e.node = new_node.clone();
        e
    });

    let mut new_state = state.clone();
    new_state.doc = doc;

    let old_type = entry.node.as_type();
    let new_type = new_node.as_type();

    let validations = if old_type != new_type {
        let mut v = Vec::new();
        if let Some(parent_id) = entry.parent {
            v.push(Validation::Node(parent_id));
        }
        v.push(Validation::Subtree(node_id));
        v
    } else {
        vec![]
    };

    Ok(StepOutput {
        state: new_state,
        validations,
    })
}

pub(crate) fn inverse(node_id: NodeId, old_node: Node, new_node: Node) -> Step {
    Step::SetNode {
        node_id,
        old_node: new_node,
        new_node: old_node,
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
    fn set_node_apply() {
        let (state, p1) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello")
                    }
                }
            }
            selection: (p1, 0)
        };

        let old_node = Node::Paragraph(ParagraphNode::default());
        let new_node = Node::Paragraph(ParagraphNode {
            align: TextAlign::Center,
        });

        let step = Step::SetNode {
            node_id: p1,
            old_node,
            new_node: new_node.clone(),
        };

        let output = step.apply(&state).unwrap();
        let new_state = output.state;

        assert_eq!(*new_state.node(p1).node(), new_node);
    }

    #[test]
    fn set_node_inverse_roundtrip() {
        let (state, p1) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello")
                    }
                }
            }
            selection: (p1, 0)
        };

        let original_node = state.node(p1).node().clone();

        let step = Step::SetNode {
            node_id: p1,
            old_node: Node::Paragraph(ParagraphNode::default()),
            new_node: Node::Paragraph(ParagraphNode {
                align: TextAlign::Center,
            }),
        };

        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;

        assert_eq!(*state3.node(p1).node(), original_node);
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
        let result = tr.set_node(p1, Node::TableRow(TableRowNode {}));

        assert!(result.is_err());
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
        let result = tr.set_node(p1, Node::Image(ImageNode::default()));

        assert!(result.is_err());
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
            old_node: Node::Paragraph(ParagraphNode::default()),
            new_node: Node::Paragraph(ParagraphNode::default()),
        };

        assert!(step.apply(&state).is_err());
    }
}
