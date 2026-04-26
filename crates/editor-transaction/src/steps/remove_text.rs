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
            let lt = local_text.char_count();
            let ul = u.char_count();

            if *q <= p {
                Ok(vec![Step::RemoveText {
                    node_id: local_node_id,
                    offset: p + ul,
                    text: local_text.to_string(),
                }])
            } else if *q >= p + lt {
                Ok(vec![Step::RemoveText {
                    node_id: local_node_id,
                    offset: p,
                    text: local_text.to_string(),
                }])
            } else {
                let left_len = q - p;
                let left_text: String = local_text.chars().take(left_len).collect();
                let right_text: String = local_text.chars().skip(left_len).collect();
                Ok(vec![
                    Step::RemoveText {
                        node_id: local_node_id,
                        offset: p,
                        text: left_text,
                    },
                    Step::RemoveText {
                        node_id: local_node_id,
                        offset: p + ul,
                        text: right_text,
                    },
                ])
            }
        }
        Step::RemoveText {
            node_id,
            offset: q,
            text: u,
        } if *node_id == local_node_id => {
            let p = local_offset;
            let lt = local_text.char_count();
            let ul = u.char_count();
            let q = *q;

            let local_start = p;
            let local_end = p + lt;
            let against_start = q;
            let against_end = q + ul;

            if against_end <= local_start {
                return Ok(vec![Step::RemoveText {
                    node_id: local_node_id,
                    offset: local_start - ul,
                    text: local_text.to_string(),
                }]);
            }
            if against_start >= local_end {
                return Ok(vec![Step::RemoveText {
                    node_id: local_node_id,
                    offset: local_start,
                    text: local_text.to_string(),
                }]);
            }

            let against_covers_local_start = against_start <= local_start;
            let against_covers_local_end = against_end >= local_end;

            if against_covers_local_start && against_covers_local_end {
                return Ok(vec![]);
            }

            if !against_covers_local_start && !against_covers_local_end {
                let left_len = against_start - local_start;
                let left_text: String = local_text.chars().take(left_len).collect();
                let right_text: String =
                    local_text.chars().skip(against_end - local_start).collect();
                debug_assert_eq!(right_text.chars().count(), local_end - against_end);
                return Ok(vec![
                    Step::RemoveText {
                        node_id: local_node_id,
                        offset: local_start,
                        text: left_text,
                    },
                    Step::RemoveText {
                        node_id: local_node_id,
                        offset: local_start,
                        text: right_text,
                    },
                ]);
            }

            if against_covers_local_start {
                let kept_text: String =
                    local_text.chars().skip(against_end - local_start).collect();
                debug_assert_eq!(kept_text.chars().count(), local_end - against_end);
                return Ok(vec![Step::RemoveText {
                    node_id: local_node_id,
                    offset: against_start,
                    text: kept_text,
                }]);
            }

            let kept_len = against_start - local_start;
            let kept_text: String = local_text.chars().take(kept_len).collect();
            Ok(vec![Step::RemoveText {
                node_id: local_node_id,
                offset: local_start,
                text: kept_text,
            }])
        }
        _ => crate::transform::transform_default(
            Step::RemoveText {
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

    #[test]
    fn transform_remove_against_insert_before_shifts() {
        let n = NodeId::new();
        let local = Step::RemoveText {
            node_id: n,
            offset: 5,
            text: "abc".into(),
        };
        let against = Step::InsertText {
            node_id: n,
            offset: 2,
            text: "X".into(),
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            vec![Step::RemoveText {
                node_id: n,
                offset: 6,
                text: "abc".into()
            }],
        );
    }

    #[test]
    fn transform_remove_against_insert_after_unchanged() {
        let n = NodeId::new();
        let local = Step::RemoveText {
            node_id: n,
            offset: 2,
            text: "abc".into(),
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
    fn transform_remove_against_insert_inside_branches() {
        let n = NodeId::new();
        let local = Step::RemoveText {
            node_id: n,
            offset: 2,
            text: "abcde".into(),
        };
        let against = Step::InsertText {
            node_id: n,
            offset: 4,
            text: "XY".into(),
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            vec![
                Step::RemoveText {
                    node_id: n,
                    offset: 2,
                    text: "ab".into()
                },
                Step::RemoveText {
                    node_id: n,
                    offset: 4,
                    text: "cde".into()
                },
            ],
        );
    }

    #[test]
    fn transform_remove_against_remove_disjoint_before_shifts() {
        let n = NodeId::new();
        let local = Step::RemoveText {
            node_id: n,
            offset: 5,
            text: "ab".into(),
        };
        let against = Step::RemoveText {
            node_id: n,
            offset: 1,
            text: "xy".into(),
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            vec![Step::RemoveText {
                node_id: n,
                offset: 3,
                text: "ab".into()
            }],
        );
    }

    #[test]
    fn transform_remove_against_remove_fully_contained_eliminates() {
        let n = NodeId::new();
        let local = Step::RemoveText {
            node_id: n,
            offset: 3,
            text: "ab".into(),
        };
        let against = Step::RemoveText {
            node_id: n,
            offset: 1,
            text: "abcdef".into(),
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            Vec::<Step>::new(),
        );
    }

    #[test]
    fn transform_remove_against_remove_partial_overlap_at_start() {
        let n = NodeId::new();
        let local = Step::RemoveText {
            node_id: n,
            offset: 3,
            text: "abcde".into(),
        };
        let against = Step::RemoveText {
            node_id: n,
            offset: 1,
            text: "XYab".into(),
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            vec![Step::RemoveText {
                node_id: n,
                offset: 1,
                text: "cde".into()
            }],
        );
    }
}
