use editor_model::Node;
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn delete_page_break_backward(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection();
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let paragraph_id = match node.node() {
        Node::Text(_) => {
            if pos.offset > 0 || node.prev_sibling().is_some() {
                return Ok(false);
            }
            node.parent()
                .ok_or(CommandError::NoParent(pos.node_id))?
                .id()
        }
        Node::Paragraph(_) => {
            if pos.offset > 0 {
                return Ok(false);
            }
            pos.node_id
        }
        _ => return Ok(false),
    };

    let doc = tr.doc();
    let paragraph = doc
        .node(paragraph_id)
        .ok_or(CommandError::NodeNotFound(paragraph_id))?;

    let prev = match paragraph.prev_sibling() {
        Some(prev) => prev,
        None => return Ok(false),
    };

    if !matches!(prev.node(), Node::Paragraph(_)) {
        return Ok(false);
    }

    let last = match prev.last_child() {
        Some(last) => last,
        None => return Ok(false),
    };

    if !matches!(last.node(), Node::PageBreak(_)) {
        return Ok(false);
    }

    tr.remove_subtree(last.id())?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn at_paragraph_start_after_page_break_paragraph_removes_marker() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { page_break }
                    paragraph { t1: text("1234") }
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_page_break_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {}
                    paragraph { t1: text("1234") }
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn at_paragraph_start_after_text_only_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("Hello") }
                    paragraph { t1: text("World") }
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| delete_page_break_backward(&mut tr));
    }

    #[test]
    fn at_paragraph_start_after_hard_break_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("Hello") hard_break }
                    paragraph { t1: text("World") }
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| delete_page_break_backward(&mut tr));
    }

    #[test]
    fn at_text_with_prev_sibling_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {
                        text("Hello")
                        t1: text("World")
                    }
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| delete_page_break_backward(&mut tr));
    }

    #[test]
    fn not_at_paragraph_start_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { page_break }
                    paragraph { t1: text("hello") }
                }
            }
            selection: (t1, 3)
        };
        transact_fail!(initial, |tr| delete_page_break_backward(&mut tr));
    }

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { page_break }
                    paragraph { t1: text("1234") }
                }
            }
            selection: (t1, 0) -> (t1, 2)
        };
        transact_fail!(initial, |tr| delete_page_break_backward(&mut tr));
    }

    #[test]
    fn no_prev_sibling_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("hello") }
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| delete_page_break_backward(&mut tr));
    }

    #[test]
    fn prev_not_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    horizontal_rule
                    paragraph { t1: text("hello") }
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| delete_page_break_backward(&mut tr));
    }
}
