use editor_common::StrExt;
use editor_model::Node;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn delete_node_forward(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection();
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    match node.node() {
        Node::Text(text_node) => {
            let text_len = text_node.text.char_count();
            if pos.offset < text_len {
                return Ok(false);
            }

            let next = match node.next_sibling() {
                Some(next) => next,
                None => return Ok(false),
            };

            if matches!(next.node(), Node::Text(_)) {
                return Ok(false);
            }

            tr.remove_subtree(next.id())?;
            tr.set_selection(Selection::collapsed(Position {
                node_id: pos.node_id,
                offset: pos.offset,
                affinity: Affinity::Upstream,
            }))?;
        }
        _ => {
            let children_len = node.entry().children.len();
            if pos.offset >= children_len {
                return Ok(false);
            }

            let child_id =
                *node
                    .entry()
                    .children
                    .get(pos.offset)
                    .ok_or(CommandError::Corrupted(format!(
                        "child at index {} not found in {:?}",
                        pos.offset, pos.node_id
                    )))?;

            let child = doc
                .node(child_id)
                .ok_or(CommandError::NodeNotFound(child_id))?;

            if matches!(child.node(), Node::Text(_)) {
                return Ok(false);
            }

            tr.remove_subtree(child_id)?;
            tr.set_selection(Selection::collapsed(Position {
                node_id: pos.node_id,
                offset: pos.offset,
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
        transact_fail!(initial, |tr| delete_node_forward(&mut tr));
    }

    #[test]
    fn delete_hard_break_after_text() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        hard_break
                        t2: text("World")
                    }
                }
            }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| delete_node_forward(&mut tr));
        let (expected, ..) = state! {
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
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_hard_break_at_paragraph_offset() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        hard_break
                        t1: text("Hello")
                    }
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_node_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn no_next_sibling_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        transact_fail!(initial, |tr| delete_node_forward(&mut tr));
    }

    #[test]
    fn next_is_text_returns_false() {
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
        transact_fail!(initial, |tr| delete_node_forward(&mut tr));
    }

    #[test]
    fn in_middle_of_text_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 3)
        };
        transact_fail!(initial, |tr| delete_node_forward(&mut tr));
    }

    #[test]
    fn at_paragraph_end_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hello") } } }
            selection: (p1, 1)
        };
        transact_fail!(initial, |tr| delete_node_forward(&mut tr));
    }
}
