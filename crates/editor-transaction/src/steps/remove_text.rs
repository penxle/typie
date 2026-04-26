use editor_common::StrExt;
use editor_model::{Node, NodeId, TextNode};
use editor_state::State;

use crate::{MapAction, Mapping, Step, StepError, StepOutput};

pub(crate) fn build_mapping(node_id: NodeId, offset: usize, text: &str) -> Mapping {
    Mapping::single(MapAction::TextRemove {
        node: node_id,
        offset,
        len: text.char_count(),
    })
}

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
        mapping: build_mapping(node_id, offset, text),
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

pub(crate) fn rebase_against(
    node_id: NodeId,
    offset: usize,
    text: &str,
    mapping: &Mapping,
) -> Vec<Step> {
    #[derive(Clone)]
    struct Range {
        offset: usize,
        text: String,
    }

    let mut ranges = vec![Range {
        offset,
        text: text.to_string(),
    }];

    'actions: for action in mapping.actions() {
        match action {
            MapAction::NodeDeleted { node } if *node == node_id => {
                ranges.clear();
                break 'actions;
            }
            MapAction::TextInsert {
                node,
                offset: q,
                len,
                ..
            } if *node == node_id => {
                let q = *q;
                let len = *len;
                let mut next: Vec<Range> = Vec::with_capacity(ranges.len() * 2);
                for r in ranges.drain(..) {
                    let r_len = r.text.char_count();
                    let r_end = r.offset + r_len;
                    if q <= r.offset {
                        next.push(Range {
                            offset: r.offset + len,
                            text: r.text,
                        });
                    } else if q >= r_end {
                        next.push(r);
                    } else {
                        let split_at = q - r.offset;
                        let left_text: String = r.text.chars().take(split_at).collect();
                        let right_text: String = r.text.chars().skip(split_at).collect();
                        if !left_text.is_empty() {
                            next.push(Range {
                                offset: r.offset,
                                text: left_text,
                            });
                        }
                        if !right_text.is_empty() {
                            next.push(Range {
                                offset: r.offset + len,
                                text: right_text,
                            });
                        }
                    }
                }
                ranges = next;
            }
            MapAction::TextRemove {
                node,
                offset: q,
                len,
            } if *node == node_id => {
                let q = *q;
                let len = *len;
                let mut next: Vec<Range> = Vec::with_capacity(ranges.len());
                for r in ranges.drain(..) {
                    let r_len = r.text.char_count();
                    let r_end = r.offset + r_len;
                    let q_end = q + len;
                    if q_end <= r.offset {
                        next.push(Range {
                            offset: r.offset - len,
                            text: r.text,
                        });
                    } else if q >= r_end {
                        next.push(r);
                    } else if q <= r.offset && q_end >= r_end {
                        // against fully covers r — drop.
                    } else if q > r.offset && q_end < r_end {
                        let drop_start = q - r.offset;
                        let left: String = r.text.chars().take(drop_start).collect();
                        let right: String = r.text.chars().skip(drop_start + len).collect();
                        next.push(Range {
                            offset: r.offset,
                            text: format!("{left}{right}"),
                        });
                    } else if q <= r.offset {
                        let trimmed: String = r.text.chars().skip(q_end - r.offset).collect();
                        next.push(Range {
                            offset: q,
                            text: trimmed,
                        });
                    } else {
                        let kept: String = r.text.chars().take(q - r.offset).collect();
                        next.push(Range {
                            offset: r.offset,
                            text: kept,
                        });
                    }
                }
                ranges = next;
            }
            _ => {}
        }
    }

    ranges
        .into_iter()
        .filter(|r| !r.text.is_empty())
        .map(|r| Step::RemoveText {
            node_id,
            offset: r.offset,
            text: r.text,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::DocTestExt;

    #[test]
    fn build_mapping_yields_text_remove_with_char_count() {
        let n = NodeId::new();
        let m = build_mapping(n, 2, "한a");
        assert_eq!(
            m.actions(),
            &[MapAction::TextRemove {
                node: n,
                offset: 2,
                len: 2,
            }]
        );
    }

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
    fn rebase_swallowed_by_node_deleted() {
        let n = NodeId::new();
        let mapping = Mapping::single(MapAction::NodeDeleted { node: n });
        let result = rebase_against(n, 0, "abc", &mapping);
        assert!(result.is_empty());
    }

    #[test]
    fn rebase_text_insert_inside_remove_range_splits() {
        let n = NodeId::new();
        let mapping = Mapping::single(MapAction::TextInsert {
            node: n,
            offset: 4,
            len: 2,
            text: "xy".into(),
        });
        let result = rebase_against(n, 2, "ABCDE", &mapping);
        assert_eq!(
            result,
            vec![
                Step::RemoveText {
                    node_id: n,
                    offset: 2,
                    text: "AB".into(),
                },
                Step::RemoveText {
                    node_id: n,
                    offset: 4,
                    text: "CDE".into(),
                },
            ]
        );
    }

    #[test]
    fn rebase_text_remove_overlapping_subset() {
        let n = NodeId::new();
        let mapping = Mapping::single(MapAction::TextRemove {
            node: n,
            offset: 3,
            len: 2,
        });
        let result = rebase_against(n, 2, "ABCDE", &mapping);
        assert_eq!(
            result,
            vec![Step::RemoveText {
                node_id: n,
                offset: 2,
                text: "ADE".into(),
            }]
        );
    }

    #[test]
    fn rebase_text_remove_fully_contains_local() {
        let n = NodeId::new();
        let mapping = Mapping::single(MapAction::TextRemove {
            node: n,
            offset: 1,
            len: 10,
        });
        let result = rebase_against(n, 2, "abc", &mapping);
        assert!(result.is_empty());
    }

    #[test]
    fn rebase_text_remove_disjoint_before_shifts() {
        let n = NodeId::new();
        let mapping = Mapping::single(MapAction::TextRemove {
            node: n,
            offset: 0,
            len: 2,
        });
        let result = rebase_against(n, 5, "ab", &mapping);
        assert_eq!(
            result,
            vec![Step::RemoveText {
                node_id: n,
                offset: 3,
                text: "ab".into(),
            }]
        );
    }

    #[test]
    fn rebase_text_insert_before_shifts() {
        let n = NodeId::new();
        let mapping = Mapping::single(MapAction::TextInsert {
            node: n,
            offset: 0,
            len: 3,
            text: "xyz".into(),
        });
        let result = rebase_against(n, 2, "abc", &mapping);
        assert_eq!(
            result,
            vec![Step::RemoveText {
                node_id: n,
                offset: 5,
                text: "abc".into(),
            }]
        );
    }

    #[test]
    fn rebase_remove_split_twice_by_consecutive_inserts() {
        let n = NodeId::new();
        let mapping = Mapping::identity()
            .compose(&Mapping::single(MapAction::TextInsert {
                node: n,
                offset: 4,
                len: 2,
                text: "XY".into(),
            }))
            .compose(&Mapping::single(MapAction::TextInsert {
                node: n,
                offset: 6,
                len: 2,
                text: "ZZ".into(),
            }));
        let result = rebase_against(n, 2, "ABCDE", &mapping);
        assert_eq!(
            result,
            vec![
                Step::RemoveText {
                    node_id: n,
                    offset: 2,
                    text: "AB".into(),
                },
                Step::RemoveText {
                    node_id: n,
                    offset: 4,
                    text: "CD".into(),
                },
                Step::RemoveText {
                    node_id: n,
                    offset: 6,
                    text: "E".into(),
                },
            ]
        );
    }
}
