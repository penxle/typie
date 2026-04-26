use editor_common::StrExt;
use editor_model::{Node, NodeId, TextNode};
use editor_state::State;

use crate::{MapAction, Mapping, Step, StepError, StepOutput};

pub(crate) fn build_mapping(node_id: NodeId, offset: usize, text: &str) -> Mapping {
    Mapping::single(MapAction::TextInsert {
        node: node_id,
        offset,
        len: text.char_count(),
        text: text.to_string(),
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
        mapping: build_mapping(node_id, offset, text),
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

pub(crate) fn rebase_against(
    node_id: NodeId,
    mut offset: usize,
    text: &str,
    mapping: &Mapping,
) -> Vec<Step> {
    for action in mapping.actions() {
        match action {
            MapAction::NodeDeleted { node } if *node == node_id => return vec![],
            MapAction::TextInsert {
                node,
                offset: q,
                len,
                text: against_text,
            } if *node == node_id => {
                if *q < offset {
                    offset += *len;
                } else if *q == offset && against_text.as_str() < text {
                    offset += *len;
                }
            }
            MapAction::TextRemove {
                node,
                offset: q,
                len,
            } if *node == node_id => {
                if *q + *len <= offset {
                    offset -= *len;
                } else if *q < offset {
                    offset = *q;
                }
            }
            _ => {}
        }
    }
    vec![Step::InsertText {
        node_id,
        offset,
        text: text.into(),
    }]
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::DocTestExt;

    #[test]
    fn build_mapping_yields_text_insert_with_char_count() {
        let n = NodeId::new();
        let m = build_mapping(n, 3, "한글ab");
        assert_eq!(
            m.actions(),
            &[MapAction::TextInsert {
                node: n,
                offset: 3,
                len: 4,
                text: "한글ab".into(),
            }]
        );
    }

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
    fn rebase_swallowed_by_node_deleted() {
        let n = NodeId::new();
        let mapping = Mapping::single(MapAction::NodeDeleted { node: n });
        let result = rebase_against(n, 0, "x", &mapping);
        assert!(result.is_empty());
    }

    #[test]
    fn rebase_unrelated_node_passthrough() {
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        let mapping = Mapping::single(MapAction::TextInsert {
            node: n2,
            offset: 0,
            len: 3,
            text: "abc".into(),
        });
        let result = rebase_against(n1, 5, "x", &mapping);
        assert_eq!(
            result,
            vec![Step::InsertText {
                node_id: n1,
                offset: 5,
                text: "x".into(),
            }]
        );
    }

    #[test]
    fn rebase_text_insert_before_shifts_offset() {
        let n = NodeId::new();
        let mapping = Mapping::single(MapAction::TextInsert {
            node: n,
            offset: 2,
            len: 3,
            text: "abc".into(),
        });
        let result = rebase_against(n, 5, "x", &mapping);
        assert_eq!(
            result,
            vec![Step::InsertText {
                node_id: n,
                offset: 8,
                text: "x".into(),
            }]
        );
    }

    #[test]
    fn rebase_text_insert_after_no_shift() {
        let n = NodeId::new();
        let mapping = Mapping::single(MapAction::TextInsert {
            node: n,
            offset: 8,
            len: 3,
            text: "yyy".into(),
        });
        let result = rebase_against(n, 5, "x", &mapping);
        assert_eq!(
            result,
            vec![Step::InsertText {
                node_id: n,
                offset: 5,
                text: "x".into(),
            }]
        );
    }

    #[test]
    fn rebase_text_insert_same_offset_lex_greater_against_no_shift() {
        let n = NodeId::new();
        let mapping = Mapping::single(MapAction::TextInsert {
            node: n,
            offset: 5,
            len: 1,
            text: "y".into(),
        });
        let result = rebase_against(n, 5, "x", &mapping);
        assert_eq!(
            result,
            vec![Step::InsertText {
                node_id: n,
                offset: 5,
                text: "x".into(),
            }]
        );
    }

    #[test]
    fn rebase_text_insert_same_offset_lex_smaller_against_shifts() {
        let n = NodeId::new();
        let mapping = Mapping::single(MapAction::TextInsert {
            node: n,
            offset: 5,
            len: 1,
            text: "a".into(),
        });
        let result = rebase_against(n, 5, "x", &mapping);
        assert_eq!(
            result,
            vec![Step::InsertText {
                node_id: n,
                offset: 6,
                text: "x".into(),
            }]
        );
    }

    #[test]
    fn rebase_text_remove_before_shifts_back() {
        let n = NodeId::new();
        let mapping = Mapping::single(MapAction::TextRemove {
            node: n,
            offset: 1,
            len: 2,
        });
        let result = rebase_against(n, 5, "x", &mapping);
        assert_eq!(
            result,
            vec![Step::InsertText {
                node_id: n,
                offset: 3,
                text: "x".into(),
            }]
        );
    }

    #[test]
    fn rebase_text_remove_overlapping_clamps_to_start() {
        let n = NodeId::new();
        let mapping = Mapping::single(MapAction::TextRemove {
            node: n,
            offset: 2,
            len: 3,
        });
        let result = rebase_against(n, 4, "x", &mapping);
        assert_eq!(
            result,
            vec![Step::InsertText {
                node_id: n,
                offset: 2,
                text: "x".into(),
            }]
        );
    }
}
