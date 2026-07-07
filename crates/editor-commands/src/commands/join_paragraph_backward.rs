use editor_model::{ChildView, NodeType};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::{prev_sibling, remove_atom_leaf};
use crate::{CommandError, CommandResult};

pub fn join_paragraph_backward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;

    let (prev_id, page_break_index, prev_was_empty, prev_child_count, prev_last_is_char) = {
        let view = tr.state().view();
        let node = view
            .node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;
        if node.node_type() != NodeType::Paragraph {
            return Ok(false);
        }
        if node.dot().is_none() {
            return Ok(false);
        }
        if pos.offset > 0 {
            return Ok(false);
        }
        node.parent().ok_or(CommandError::NoParent(pos.node))?;
        let prev = match prev_sibling(&node) {
            Some(ChildView::Block(prev)) => prev,
            _ => return Ok(false),
        };
        if prev.node_type() != NodeType::Paragraph {
            return Ok(false);
        }
        let prev_id = prev.id();
        let raw_child_count = prev.children().count();
        let has_trailing_page_break = matches!(
            prev.last_child(),
            Some(ChildView::Leaf(l)) if l.node_type() == NodeType::PageBreak
        );
        let page_break_index = has_trailing_page_break.then(|| raw_child_count - 1);
        let prev_child_count = raw_child_count - usize::from(has_trailing_page_break);
        let prev_was_empty = prev_child_count == 0;
        let prev_last_is_char = prev_child_count > 0
            && matches!(
                prev.child_at(prev_child_count - 1),
                Some(ChildView::Leaf(l)) if l.as_char().is_some()
            );
        (
            prev_id,
            page_break_index,
            prev_was_empty,
            prev_child_count,
            prev_last_is_char,
        )
    };

    if let Some(index) = page_break_index {
        remove_atom_leaf(tr, prev_id, index)?;
    }

    tr.merge_node(prev_id)?;

    let new_selection = if prev_was_empty {
        Selection::collapsed(Position {
            node: prev_id,
            offset: 0,
            affinity: Affinity::Downstream,
        })
    } else {
        let affinity = if prev_last_is_char {
            Affinity::Upstream
        } else {
            Affinity::Downstream
        };
        Selection::collapsed(Position {
            node: prev_id,
            offset: prev_child_count,
            affinity,
        })
    };
    tr.set_selection(Some(new_selection))?;

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
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    p2: paragraph { text("World") }
                }
            }
            selection: (p2, 0) -> (p2, 3)
        };
        transact_fail!(initial, |tr| join_paragraph_backward(&mut tr));
    }

    #[test]
    fn join_two_text_paragraphs() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    p2: paragraph { text("World") }
                }
            }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("HelloWorld")
                    }
                }
            }
            selection: (p1, 5, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn join_into_empty_prev() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {}
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn join_empty_into_prev() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    p2: paragraph {}
                }
            }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 5, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn join_both_empty() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {}
                    p2: paragraph {}
                }
            }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn no_prev_sibling_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| join_paragraph_backward(&mut tr));
    }

    #[test]
    fn prev_not_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    horizontal_rule
                    p1: paragraph { text("Hello") }
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| join_paragraph_backward(&mut tr));
    }

    #[test]
    fn not_at_paragraph_start_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("Hello") }
                    p2: paragraph { text("World") }
                }
            }
            selection: (p2, 3)
        };
        transact_fail!(initial, |tr| join_paragraph_backward(&mut tr));
    }

    #[test]
    fn join_preserves_all_children() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {
                        text("A")
                        text("B") [bold]
                    }
                    cur: paragraph {
                        text("CD")
                    }
                }
            }
            selection: (cur, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    m: paragraph {
                        text("A")
                        text("B") [bold]
                        text("CD")
                    }
                }
            }
            selection: (m, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn join_preserves_tail_bold() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {
                        text("AB")
                    }
                    cur: paragraph {
                        text("CD") [bold]
                    }
                }
            }
            selection: (cur, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    m: paragraph {
                        text("AB")
                        text("CD") [bold]
                    }
                }
            }
            selection: (m, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn structural_merge_drops_front_trailing_page_break() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("AB") page_break }
                    cur: paragraph { text("CD") }
                }
            }
            selection: (cur, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    m: paragraph {
                        text("AB")
                        text("CD")
                    }
                }
            }
            selection: (m, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn structural_merge_drops_empty_front_trailing_page_break() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { page_break }
                    cur: paragraph { text("CD") }
                }
            }
            selection: (cur, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    m: paragraph {
                        text("CD")
                    }
                }
            }
            selection: (m, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn image_between_paragraphs_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("Hello") }
                    image
                    p1: paragraph { text("World") }
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| join_paragraph_backward(&mut tr));
    }

    #[test]
    fn backspace_merge_keeps_front_paragraph_carry() {
        let (initial, p1, _p2) = state! {
            doc {
                root {
                    p1: paragraph carry([bold]) { text("Hello") }
                    p2: paragraph carry([italic]) { text("World") }
                }
            }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_backward(&mut tr));
        let carry = actual.projected.carry_modifiers(p1);
        assert!(
            carry
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Bold)),
            "merged block keeps the front paragraph's carry, got {carry:?}"
        );
        assert!(
            !carry
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Italic)),
            "the trailing paragraph's carry is discarded on merge, got {carry:?}"
        );
    }
}
