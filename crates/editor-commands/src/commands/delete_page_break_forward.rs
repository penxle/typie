use editor_model::{ChildView, NodeType};
use editor_state::{Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::remove_atom_leaf;
use crate::{CommandError, CommandResult};

pub fn delete_page_break_forward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;

    let (children_count, last_is_page_break) = {
        let view = tr.state().view();
        let node = view
            .node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;
        let last_is_page_break = matches!(
            node.last_child(),
            Some(ChildView::Leaf(l)) if l.node_type() == NodeType::PageBreak
        );
        (node.children().count(), last_is_page_break)
    };

    if !last_is_page_break {
        return Ok(false);
    }
    let page_break_index = children_count - 1;

    if pos.offset + 1 == children_count {
        remove_atom_leaf(tr, pos.node, page_break_index)?;
        Ok(true)
    } else if pos.offset == children_count {
        let new_offset = pos.offset - 1;
        remove_atom_leaf(tr, pos.node, page_break_index)?;
        tr.set_selection(Some(Selection::collapsed(Position {
            node: pos.node,
            offset: new_offset,
            affinity: pos.affinity,
        })))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn at_text_end_before_page_break_removes_marker() {
        let (initial, _t1) = state! {
            doc {
                root {
                    t1: paragraph { text("a") page_break }
                    paragraph { text("b") }
                }
            }
            selection: (t1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| delete_page_break_forward(&mut tr));
        let (expected, _t1) = state! {
            doc {
                root {
                    t1: paragraph { text("a") }
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
        let (initial, _t1) = state! {
            doc {
                root {
                    t1: paragraph { text("a") }
                    paragraph { text("b") }
                }
            }
            selection: (t1, 1)
        };
        transact_fail!(initial, |tr| delete_page_break_forward(&mut tr));
    }

    #[test]
    fn at_text_end_followed_by_hard_break_returns_false() {
        let (initial, _t1) = state! {
            doc {
                root {
                    t1: paragraph { text("a") hard_break }
                }
            }
            selection: (t1, 1)
        };
        transact_fail!(initial, |tr| delete_page_break_forward(&mut tr));
    }

    #[test]
    fn at_paragraph_offset_past_page_break_removes_marker_and_shifts_cursor() {
        let (initial, _p1) = state! {
            doc {
                root {
                    p1: paragraph { text("a") page_break }
                    paragraph { text("b") }
                }
            }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_page_break_forward(&mut tr));
        let (expected, _t1) = state! {
            doc {
                root {
                    t1: paragraph { text("a") }
                    paragraph { text("b") }
                }
            }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, _t1) = state! {
            doc {
                root {
                    t1: paragraph { text("a") page_break }
                }
            }
            selection: (t1, 0) -> (t1, 1)
        };
        transact_fail!(initial, |tr| delete_page_break_forward(&mut tr));
    }
}
