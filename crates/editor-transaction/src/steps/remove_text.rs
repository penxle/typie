use editor_common::StrExt;
use editor_model::{Node, NodeId, TextNode};
use editor_state::State;

use crate::{Step, StepError, StepOutput};

pub(crate) fn apply(
    state: &State,
    node_id: NodeId,
    offset: usize,
    text: &str,
) -> Result<StepOutput, StepError> {
    let entry = state
        .doc
        .get_entry(node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;
    let text_node = match &entry.node {
        Node::Text(t) => t,
        _ => return Err(StepError::ExpectedTextNode(node_id)),
    };

    let len = text.char_count();
    if offset + len > text_node.text.char_count() {
        return Err(StepError::OffsetOutOfBounds {
            node_id,
            offset: offset + len,
            len: text_node.text.char_count(),
        });
    }

    let byte_start = text_node.text.nth_char_byte_offset(offset);
    let byte_end = text_node.text.nth_char_byte_offset(offset + len);
    let mut new_text = text_node.text.clone();
    new_text.replace_range(byte_start..byte_end, "");

    let doc = state.doc.with_node_updated(node_id, |mut entry| {
        entry.node = Node::Text(TextNode { text: new_text });
        entry
    });

    let mut new_state = state.clone();
    new_state.doc = doc;

    Ok(StepOutput {
        state: new_state,
        validations: vec![],
    })
}

pub(crate) fn inverse(node_id: NodeId, offset: usize, text: String) -> Step {
    Step::InsertText {
        node_id,
        offset,
        text,
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use crate::test_utils::DocTestExt;
    use crate::*;

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

        assert_eq!(output.state.text(t1).text, "Hello");
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

        assert_eq!(state3.text(t1).text, "Hello World");
    }
}
