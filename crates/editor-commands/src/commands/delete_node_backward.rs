use editor_model::Node;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn delete_node_backward(tr: &mut Transaction) -> CommandResult {
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
        Node::Text(_) => {
            if pos.offset > 0 {
                return Ok(false);
            }

            let prev = match node.prev_sibling() {
                Some(prev) => prev,
                None => return Ok(false),
            };

            if matches!(prev.node(), Node::Text(_)) {
                return Ok(false);
            }

            tr.remove_subtree(prev.id())?;
            tr.set_selection(Selection::collapsed(Position {
                node_id: pos.node_id,
                offset: 0,
                affinity: Affinity::Downstream,
            }))?;
        }
        _ => {
            if pos.offset == 0 {
                return Ok(false);
            }

            let child_id = *node.entry().children.iter().nth(pos.offset - 1).ok_or(
                CommandError::Corrupted(format!(
                    "child at index {} not found in {:?}",
                    pos.offset - 1,
                    pos.node_id
                )),
            )?;

            let child = doc
                .node(child_id)
                .ok_or(CommandError::NodeNotFound(child_id))?;

            if matches!(child.node(), Node::Text(_)) {
                return Ok(false);
            }

            tr.remove_subtree(child_id)?;
            tr.set_selection(Selection::collapsed(Position {
                node_id: pos.node_id,
                offset: pos.offset - 1,
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
        transact_fail!(initial, |tr| delete_node_backward(&mut tr));
    }

    #[test]
    fn delete_hard_break_before_text() {
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
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_node_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t2: text("World")
                    }
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_hard_break_at_paragraph_offset() {
        let (initial, ..) = state! {
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
        let (actual, ..) = transact!(initial, |tr| delete_node_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_no_prev_sibling_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| delete_node_backward(&mut tr));
    }

    #[test]
    fn delete_prev_text_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t2: text("World")
                    }
                }
            }
            selection: (t2, 0)
        };
        transact_fail!(initial, |tr| delete_node_backward(&mut tr));
    }

    #[test]
    fn delete_in_middle_of_text_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 3)
        };
        transact_fail!(initial, |tr| delete_node_backward(&mut tr));
    }

    #[test]
    fn delete_at_paragraph_start_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| delete_node_backward(&mut tr));
    }
}
