use editor_model::{Node, NodeId, PlainHardBreakNode, PlainNode, Subtree};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn insert_hard_break(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection();
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    let doc = tr.doc();

    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let break_id = NodeId::new();
    let break_subtree = Subtree::leaf(
        break_id,
        PlainNode::HardBreak(PlainHardBreakNode::default()),
    );

    match node.node() {
        Node::Text(text_node) => {
            let parent = node.parent().ok_or(CommandError::NoParent(pos.node_id))?;
            let node_index = node
                .index()
                .ok_or(CommandError::orphan_child(pos.node_id, parent.id()))?;
            let text_len = text_node.text.len();

            if pos.offset == 0 {
                // Case B: cursor at start of text → insert hard break before
                tr.insert_subtree(parent.id(), node_index, break_subtree)?;
                tr.set_selection(Selection::collapsed(Position {
                    node_id: pos.node_id,
                    offset: 0,
                    affinity: Affinity::Downstream,
                }))?;
            } else if pos.offset == text_len {
                // Case C: cursor at end of text → insert hard break after
                tr.insert_subtree(parent.id(), node_index + 1, break_subtree)?;

                let doc = tr.doc();
                let break_node = doc
                    .node(break_id)
                    .ok_or(CommandError::NodeNotFound(break_id))?;

                if let Some(next) = break_node.next_sibling() {
                    if matches!(next.node(), Node::Text(_)) {
                        tr.set_selection(Selection::collapsed(Position {
                            node_id: next.id(),
                            offset: 0,
                            affinity: Affinity::Downstream,
                        }))?;
                    } else {
                        let idx = next
                            .index()
                            .ok_or(CommandError::orphan_child(next.id(), parent.id()))?;
                        tr.set_selection(Selection::collapsed(Position {
                            node_id: parent.id(),
                            offset: idx,
                            affinity: Affinity::Downstream,
                        }))?;
                    }
                } else {
                    let break_idx = break_node
                        .index()
                        .ok_or(CommandError::orphan_child(break_id, parent.id()))?;
                    tr.set_selection(Selection::collapsed(Position {
                        node_id: parent.id(),
                        offset: break_idx + 1,
                        affinity: Affinity::Downstream,
                    }))?;
                }
            } else {
                // Case A: cursor in middle of text → split, insert hard break between
                let split_id = NodeId::new();
                tr.split_node(pos.node_id, pos.offset, split_id)?;
                tr.insert_subtree(parent.id(), node_index + 1, break_subtree)?;
                tr.set_selection(Selection::collapsed(Position {
                    node_id: split_id,
                    offset: 0,
                    affinity: Affinity::Downstream,
                }))?;
            }
        }
        _ => {
            // Case D: non-text node (empty paragraph, etc.)
            tr.insert_subtree(pos.node_id, pos.offset, break_subtree)?;
            tr.set_selection(Selection::collapsed(Position {
                node_id: pos.node_id,
                offset: pos.offset + 1,
                affinity: Affinity::Downstream,
            }))?;
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 3)
        };
        transact_fail!(initial, |tr| insert_hard_break(&mut tr));
    }

    #[test]
    fn insert_at_start_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        hard_break
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_in_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("He")
                        hard_break
                        t2: text("llo")
                    }
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("Hello")
                        hard_break
                    }
                }
            }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_in_empty_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        hard_break
                    }
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_with_next_text_sibling() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t2: text("World")
                    }
                }
            }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        hard_break
                        t2: text("World")
                    }
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn pending_modifiers_preserved() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        assert!(!actual.pending_modifiers.is_empty());
    }
}
