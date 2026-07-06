use editor_model::{ChildView, NodeType};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn join_paragraph_backward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;

    let (prev_id, prev_was_empty, prev_child_count, prev_last_is_char) = {
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
        let parent = node.parent().ok_or(CommandError::NoParent(pos.node))?;
        let index = parent
            .child_blocks()
            .position(|b| b.id() == pos.node)
            .ok_or_else(|| CommandError::orphan_child(pos.node, parent.id()))?;
        if index == 0 {
            return Ok(false);
        }
        let prev = parent
            .child_blocks()
            .nth(index - 1)
            .ok_or(CommandError::NodeNotFound(pos.node))?;
        if prev.node_type() != NodeType::Paragraph {
            return Ok(false);
        }
        let prev_id = prev.id();
        let prev_child_count = prev.children().count();
        let prev_was_empty = prev_child_count == 0;
        let prev_last_is_char = matches!(
            prev.last_child(),
            Some(ChildView::Leaf(l)) if l.as_char().is_some()
        );
        (prev_id, prev_was_empty, prev_child_count, prev_last_is_char)
    };

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
}
