use editor_model::Node;
use editor_transaction::{Transaction, compact};

use crate::{CommandError, CommandResult};

pub fn join_paragraph_forward(tr: &mut Transaction) -> CommandResult {
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
        Node::Text(text_node) => {
            let text_len = text_node.text.len();
            if pos.offset < text_len || node.next_sibling().is_some() {
                return Ok(false);
            }
            node.parent()
                .ok_or(CommandError::NoParent(pos.node_id))?
                .id()
        }
        Node::Paragraph(_) => {
            let children_len = node.entry().children.len();
            if pos.offset < children_len {
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

    let next = match paragraph.next_sibling() {
        Some(next) => next,
        None => return Ok(false),
    };

    if !matches!(next.node(), Node::Paragraph(_)) {
        return Ok(false);
    }

    let next_id = next.id();

    // Record cursor position before merge (stays at current position)
    let cursor_selection = tr.selection();

    tr.merge_node(next_id, paragraph_id)?;

    let doc = tr.doc();
    if let Some(p) = doc.node(paragraph_id) {
        tr.apply_steps(compact(&p))?;
    }

    tr.set_selection(cursor_selection)?;

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
            selection: (t1, 0) -> (t1, 3)
        };
        transact_fail!(initial, |tr| join_paragraph_forward(&mut tr));
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
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_forward(&mut tr));
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
    fn join_empty_next() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                    paragraph {}
                }
            }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_forward(&mut tr));
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
    fn join_empty_current() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {}
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { t1: text("Hello") }
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn join_both_empty() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {}
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_forward(&mut tr));
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
    fn no_next_sibling_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 5)
        };
        transact_fail!(initial, |tr| join_paragraph_forward(&mut tr));
    }

    #[test]
    fn next_not_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                    horizontal_rule
                }
            }
            selection: (t1, 5)
        };
        transact_fail!(initial, |tr| join_paragraph_forward(&mut tr));
    }

    #[test]
    fn not_at_paragraph_end_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                    paragraph { t2: text("World") }
                }
            }
            selection: (t1, 3)
        };
        transact_fail!(initial, |tr| join_paragraph_forward(&mut tr));
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
            selection: (t2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| join_paragraph_forward(&mut tr));
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
