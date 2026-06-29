use editor_model::{ChildView, Node, NodeType};
use editor_transaction::Transaction;

use crate::helpers::remove_atom_leaf;
use crate::{CommandError, CommandResult};

pub fn delete_page_break_backward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;
    if pos.offset > 0 {
        return Ok(false);
    }

    let (prev_id, page_break_index) = {
        let view = tr.state().view();
        let node = view
            .node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;

        let parent = match node.parent() {
            Some(parent) => parent,
            None => return Ok(false),
        };
        let idx = match parent.child_blocks().position(|b| b.id() == node.id()) {
            Some(idx) => idx,
            None => return Ok(false),
        };
        if idx == 0 {
            return Ok(false);
        }
        let prev = match parent.child_blocks().nth(idx - 1) {
            Some(prev) => prev,
            None => return Ok(false),
        };

        if !matches!(prev.node(), Node::Paragraph(_)) {
            return Ok(false);
        }

        let last_is_page_break = matches!(
            prev.last_child(),
            Some(ChildView::Leaf(l)) if l.node_type() == NodeType::PageBreak
        );
        if !last_is_page_break {
            return Ok(false);
        }

        (prev.id(), prev.children().count() - 1)
    };

    remove_atom_leaf(tr, prev_id, page_break_index)?;

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
                    p1: paragraph { text("1234") }
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_page_break_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {}
                    p1: paragraph { text("1234") }
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn at_paragraph_start_after_text_only_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("Hello") }
                    p1: paragraph { text("World") }
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| delete_page_break_backward(&mut tr));
    }

    #[test]
    fn at_paragraph_start_after_hard_break_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("Hello") hard_break }
                    p1: paragraph { text("World") }
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| delete_page_break_backward(&mut tr));
    }

    #[test]
    fn not_at_paragraph_start_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { page_break }
                    p1: paragraph { text("hello") }
                }
            }
            selection: (p1, 3)
        };
        transact_fail!(initial, |tr| delete_page_break_backward(&mut tr));
    }

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { page_break }
                    p1: paragraph { text("1234") }
                }
            }
            selection: (p1, 0) -> (p1, 2)
        };
        transact_fail!(initial, |tr| delete_page_break_backward(&mut tr));
    }

    #[test]
    fn no_prev_sibling_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("hello") }
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| delete_page_break_backward(&mut tr));
    }

    #[test]
    fn prev_not_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    horizontal_rule
                    p1: paragraph { text("hello") }
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| delete_page_break_backward(&mut tr));
    }
}
