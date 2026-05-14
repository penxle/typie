use editor_model::Node;
use editor_state::{Position, Selection};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn delete_page_break_forward(tr: &mut Transaction) -> CommandResult {
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
            if pos.offset < text_node.text.len() {
                return Ok(false);
            }
            let next = match node.next_sibling() {
                Some(next) => next,
                None => return Ok(false),
            };
            if !matches!(next.node(), Node::PageBreak(_)) {
                return Ok(false);
            }
            tr.remove_subtree(next.id())?;
            Ok(true)
        }
        Node::Paragraph(_) => {
            let last = match node.last_child() {
                Some(last) => last,
                None => return Ok(false),
            };
            if !matches!(last.node(), Node::PageBreak(_)) {
                return Ok(false);
            }
            let children_count = node.entry().children.len();
            let last_id = last.id();
            if pos.offset + 1 == children_count {
                tr.remove_subtree(last_id)?;
                Ok(true)
            } else if pos.offset == children_count {
                let new_offset = pos.offset - 1;
                tr.remove_subtree(last_id)?;
                tr.set_selection(Selection::collapsed(Position {
                    node_id: pos.node_id,
                    offset: new_offset,
                    affinity: pos.affinity,
                }))?;
                Ok(true)
            } else {
                Ok(false)
            }
        }
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn at_text_end_before_page_break_removes_marker() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("a") page_break }
                    paragraph { text("b") }
                }
            }
            selection: (t1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_page_break_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("a") }
                    paragraph { text("b") }
                }
            }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn at_paragraph_offset_before_page_break_removes_marker() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") page_break }
                    paragraph { text("b") }
                }
            }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_page_break_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    paragraph { text("b") }
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn page_break_only_paragraph_at_offset_0_removes_marker() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { page_break }
                    paragraph { text("a") }
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_page_break_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {}
                    paragraph { text("a") }
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn at_text_end_without_page_break_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("a") }
                    paragraph { text("b") }
                }
            }
            selection: (t1, 1)
        };
        transact_fail!(initial, |tr| delete_page_break_forward(&mut tr));
    }

    #[test]
    fn at_text_end_followed_by_hard_break_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("a") hard_break }
                }
            }
            selection: (t1, 1)
        };
        transact_fail!(initial, |tr| delete_page_break_forward(&mut tr));
    }

    #[test]
    fn at_paragraph_offset_past_page_break_removes_marker_and_shifts_cursor() {
        // Removing the child shrinks `children.len()`, so an offset equal to the
        // old length becomes out of bounds and must be decremented in the same step.
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") page_break }
                    paragraph { text("b") }
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_page_break_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("a") }
                    paragraph { text("b") }
                }
            }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("a") page_break }
                }
            }
            selection: (t1, 0) -> (t1, 1)
        };
        transact_fail!(initial, |tr| delete_page_break_forward(&mut tr));
    }
}
