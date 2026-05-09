use editor_crdt::{CrdtError, Dot, TextOp};
use editor_model::{DocOp, Node, NodeId};
use editor_state::BatchedState;

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(node_id: NodeId, offset: usize, text: String) -> Step {
    Step::InsertText {
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
    let len = text.chars().count();

    // Read: collect target dots for offset..offset+len
    let target_dots: Vec<Dot> = {
        let entry = batched
            .doc
            .get_entry(node_id)
            .ok_or(StepError::NodeNotFound(node_id))?;
        let Node::Text(text_node) = &entry.node else {
            return Err(StepError::ExpectedTextNode(node_id));
        };
        let mut dots = Vec::with_capacity(len);
        for i in 0..len {
            let dot = text_node
                .text
                .dot_at(offset + i + 1)
                .map_err(|e| match e {
                    CrdtError::OffsetOutOfBounds { offset, len } => StepError::OffsetOutOfBounds {
                        node_id,
                        offset,
                        len,
                    },
                    other => panic!("dot_at unexpected: {other:?}"),
                })?
                .ok_or_else(|| StepError::OffsetOutOfBounds {
                    node_id,
                    offset: offset + i + 1,
                    len: text_node.text.len(),
                })?;
            dots.push(dot);
        }
        dots
    };

    // Sequential apply, one RemoveChar per collected dot
    for target in target_dots {
        batched.apply(DocOp::Text {
            node_id,
            op: TextOp::RemoveChar { observed: target },
        })?;
    }

    validations.push(Validation::Node(node_id));
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use crate::Step;
    use crate::test_utils::DocTestExt;

    #[test]
    fn remove_text_apply() {
        let (state, t1) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello World")
                    }
                }
            }
            selection: (t1, 0)
        };

        let step = Step::RemoveText {
            node_id: t1,
            offset: 5,
            text: " World".to_string(),
        };
        let output = step.apply(&state).unwrap();

        assert_eq!(output.state.text(t1).text.to_string(), "Hello");
    }

    #[test]
    fn remove_then_insert_roundtrip() {
        let (state, t1) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello World")
                    }
                }
            }
            selection: (t1, 0)
        };

        let step = Step::RemoveText {
            node_id: t1,
            offset: 5,
            text: " World".to_string(),
        };
        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;

        assert_eq!(state3.text(t1).text.to_string(), "Hello World");
    }
}
