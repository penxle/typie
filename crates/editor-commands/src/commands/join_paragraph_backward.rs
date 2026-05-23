use editor_model::Node;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, compact};

use crate::{CommandError, CommandResult};

pub fn join_paragraph_backward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
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

    let prev_id = prev.id();

    // Calculate join point cursor before merge
    let prev_was_empty = prev.entry().children.is_empty();
    let join_cursor = if let Some(last_child) = prev.last_child() {
        match last_child.node() {
            Node::Text(t) => Some((last_child.id(), t.text.len())),
            _ => None,
        }
    } else {
        None
    };
    let prev_children_count = prev.entry().children.len();

    tr.merge_node(paragraph_id, prev_id)?;

    let doc = tr.doc();
    if let Some(p) = doc.node(prev_id) {
        tr.apply_steps(compact(&p))?;
    }

    let new_selection = if let Some((cursor_node, cursor_offset)) = join_cursor {
        Selection::collapsed(Position {
            node_id: cursor_node,
            offset: cursor_offset,
            affinity: Affinity::Upstream,
        })
    } else if prev_was_empty {
        // prev was empty — check if merged children start with text
        let doc = tr.doc();
        let prev = doc
            .node(prev_id)
            .ok_or(CommandError::NodeNotFound(prev_id))?;
        match prev.first_child() {
            Some(child) if matches!(child.node(), Node::Text(_)) => {
                Selection::collapsed(Position {
                    node_id: child.id(),
                    offset: 0,
                    affinity: Affinity::Downstream,
                })
            }
            _ => Selection::collapsed(Position {
                node_id: prev_id,
                offset: 0,
                affinity: Affinity::Downstream,
            }),
        }
    } else {
        Selection::collapsed(Position {
            node_id: prev_id,
            offset: prev_children_count,
            affinity: Affinity::Downstream,
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
                    paragraph { t1: text("Hello") }
                    paragraph { t2: text("World") }
                }
            }
            selection: (t2, 0) -> (t2, 3)
        };
        transact_fail!(initial, |tr| join_paragraph_backward(&mut tr));
    }

    #[test]
    fn join_two_text_paragraphs() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                    paragraph { t2: text("World") }
                }
            }
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("HelloWorld")
                    }
                }
            }
            selection: (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn join_into_empty_prev() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {}
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn join_empty_into_prev() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                    p2: paragraph {}
                }
            }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 5)
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
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| join_paragraph_backward(&mut tr));
    }

    #[test]
    fn prev_not_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    horizontal_rule
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| join_paragraph_backward(&mut tr));
    }

    #[test]
    fn not_at_paragraph_start_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                    paragraph { t2: text("World") }
                }
            }
            selection: (t2, 3)
        };
        transact_fail!(initial, |tr| join_paragraph_backward(&mut tr));
    }

    #[test]
    fn join_preserves_all_children() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("A")
                        t2: text("B") [bold]
                    }
                    paragraph {
                        t3: text("C")
                        t4: text("D")
                    }
                }
            }
            selection: (t3, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("A")
                        t2: text("B") [bold]
                        t3: text("CD")
                    }
                }
            }
            selection: (t2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }
}
