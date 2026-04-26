use editor_common::StrExt;
use editor_model::{Node, NodeId, TextNode};
use editor_state::State;

use crate::transform::Conflict;
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

    if offset > text_node.text.char_count() {
        return Err(StepError::OffsetOutOfBounds {
            node_id,
            offset,
            len: text_node.text.char_count(),
        });
    }

    let byte_offset = text_node.text.nth_char_byte_offset(offset);
    let mut new_text = text_node.text.clone();
    new_text.insert_str(byte_offset, text);

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
    Step::RemoveText {
        node_id,
        offset,
        text,
    }
}

pub(crate) fn transform_against(
    local_node_id: NodeId,
    local_offset: usize,
    local_text: &str,
    against: &Step,
) -> Result<Vec<Step>, Conflict> {
    match against {
        Step::InsertText {
            node_id,
            offset: q,
            text: u,
        } if *node_id == local_node_id => {
            let p = local_offset;
            let new_offset = if *q < p {
                p + u.char_count()
            } else if *q == p && local_text > u.as_str() {
                p + u.char_count()
            } else {
                p
            };
            Ok(vec![Step::InsertText {
                node_id: local_node_id,
                offset: new_offset,
                text: local_text.to_string(),
            }])
        }
        Step::RemoveText {
            node_id,
            offset: q,
            text: u,
        } if *node_id == local_node_id => {
            let p = local_offset;
            let ul = u.char_count();
            let new_offset = if q + ul <= p {
                p - ul
            } else if *q >= p {
                p
            } else {
                *q
            };
            Ok(vec![Step::InsertText {
                node_id: local_node_id,
                offset: new_offset,
                text: local_text.to_string(),
            }])
        }
        _ => crate::transform::transform_default(
            Step::InsertText {
                node_id: local_node_id,
                offset: local_offset,
                text: local_text.to_string(),
            },
            against,
        ),
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::NodeId;

    use crate::test_utils::DocTestExt;
    use crate::*;

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

        assert_eq!(output.state.text(t1).text, "Hello World");
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

        assert!(step.apply(&state).is_err());
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

        assert_eq!(state3.text(t1).text, "Hello");
    }

    #[test]
    fn transform_insert_against_insert_before_shifts_offset() {
        let n = NodeId::new();
        let local = Step::InsertText {
            node_id: n,
            offset: 5,
            text: "ab".into(),
        };
        let against = Step::InsertText {
            node_id: n,
            offset: 3,
            text: "X".into(),
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            vec![Step::InsertText {
                node_id: n,
                offset: 6,
                text: "ab".into()
            }],
        );
    }

    #[test]
    fn transform_insert_against_insert_after_unchanged() {
        let n = NodeId::new();
        let local = Step::InsertText {
            node_id: n,
            offset: 3,
            text: "ab".into(),
        };
        let against = Step::InsertText {
            node_id: n,
            offset: 5,
            text: "X".into(),
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            vec![local.clone()],
        );
    }

    #[test]
    fn transform_insert_against_insert_different_node_unchanged() {
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        let local = Step::InsertText {
            node_id: n1,
            offset: 3,
            text: "ab".into(),
        };
        let against = Step::InsertText {
            node_id: n2,
            offset: 0,
            text: "X".into(),
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            vec![local.clone()],
        );
    }

    #[test]
    fn transform_insert_against_remove_before_shifts_back() {
        let n = NodeId::new();
        let local = Step::InsertText {
            node_id: n,
            offset: 7,
            text: "ab".into(),
        };
        let against = Step::RemoveText {
            node_id: n,
            offset: 2,
            text: "abc".into(),
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            vec![Step::InsertText {
                node_id: n,
                offset: 4,
                text: "ab".into()
            }],
        );
    }

    #[test]
    fn transform_insert_against_remove_inside_clamps_to_start() {
        let n = NodeId::new();
        let local = Step::InsertText {
            node_id: n,
            offset: 5,
            text: "ab".into(),
        };
        let against = Step::RemoveText {
            node_id: n,
            offset: 3,
            text: "abcde".into(),
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            vec![Step::InsertText {
                node_id: n,
                offset: 3,
                text: "ab".into()
            }],
        );
    }

    #[test]
    fn transform_insert_against_remove_after_unchanged() {
        let n = NodeId::new();
        let local = Step::InsertText {
            node_id: n,
            offset: 3,
            text: "ab".into(),
        };
        let against = Step::RemoveText {
            node_id: n,
            offset: 5,
            text: "x".into(),
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            vec![local.clone()],
        );
    }
}
