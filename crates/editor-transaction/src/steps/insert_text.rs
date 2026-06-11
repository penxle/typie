use editor_crdt::{CrdtError, PlacementId, TextOp};
use editor_model::{DocOp, Node, NodeId};
use editor_state::BatchedState;

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(node_id: NodeId, offset: usize, text: String) -> Step {
    Step::RemoveText {
        node_id,
        offset,
        text,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    validations: &mut Vec<Validation>,
    node_id: NodeId,
    offset: usize,
    text: &str,
) -> Result<(), StepError> {
    // Read: anchor dot at offset
    let mut after: Option<PlacementId> = {
        let entry = batched
            .doc
            .get_entry(node_id)
            .ok_or(StepError::NodeNotFound(node_id))?;
        let Node::Text(text_node) = &entry.node else {
            return Err(StepError::ExpectedTextNode(node_id));
        };
        text_node
            .text
            .placement_before_offset(offset)
            .map_err(|e| match e {
                CrdtError::OffsetOutOfBounds { offset, len } => StepError::OffsetOutOfBounds {
                    node_id,
                    offset,
                    len,
                },
                other => panic!("placement_before_offset unexpected error: {other:?}"),
            })?
    };

    // Sequential apply, chaining each emitted dot as the next `after`
    for ch in text.chars() {
        let op_id = batched
            .apply(DocOp::Text {
                node_id,
                op: TextOp::InsertChar { ch, after },
            })?
            .id;
        after = Some(PlacementId(op_id));
    }

    validations.push(Validation::Node(node_id));
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use crate::test_utils::DocTestExt;
    use crate::{Step, StepError};

    #[test]
    fn insert_text_apply() {
        let (state, t1) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let step = Step::InsertText {
            node_id: t1,
            offset: 5,
            text: " World".to_string(),
        };
        let output = step.apply(&state).unwrap();

        assert_eq!(output.state.text(t1).text.to_string(), "Hello World");
    }

    #[test]
    fn insert_text_not_text_node() {
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

        let step = Step::InsertText {
            node_id: p1,
            offset: 0,
            text: "X".to_string(),
        };
        assert!(matches!(
            step.apply(&state),
            Err(StepError::ExpectedTextNode(_))
        ));
    }

    #[test]
    fn insert_then_remove_roundtrip() {
        let (state, t1) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let step = Step::InsertText {
            node_id: t1,
            offset: 5,
            text: " World".to_string(),
        };
        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;

        assert_eq!(state3.text(t1).text.to_string(), "Hello");
    }
}
