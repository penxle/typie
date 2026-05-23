use editor_model::{Node, NodeId, PlainNode, PlainPageBreakNode, Subtree};
use editor_transaction::Transaction;

use crate::helpers::find_ancestor_textblock;
use crate::{CommandError, CommandResult};

pub fn insert_page_break_into_prev_paragraph(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() {
        return Ok(false);
    }
    let pos = selection.head;

    let doc = tr.doc();
    let Some(paragraph_id) = find_ancestor_textblock(&doc, pos.node_id) else {
        return Ok(false);
    };
    let paragraph = doc
        .node(paragraph_id)
        .ok_or(CommandError::NodeNotFound(paragraph_id))?;

    if !matches!(paragraph.node(), Node::Paragraph(_)) {
        return Ok(false);
    }
    if paragraph
        .parent()
        .is_none_or(|parent| parent.id() != NodeId::ROOT)
    {
        return Ok(false);
    }

    let Some(prev) = paragraph.prev_sibling() else {
        return Ok(false);
    };
    if !matches!(prev.node(), Node::Paragraph(_)) {
        return Ok(false);
    }
    if prev
        .children()
        .any(|child| matches!(child.node(), Node::PageBreak(_)))
    {
        return Ok(false);
    }

    let prev_id = prev.id();
    let insert_index = prev.entry().children.len();

    tr.insert_subtree(
        prev_id,
        insert_index,
        Subtree::leaf(
            NodeId::new(),
            PlainNode::PageBreak(PlainPageBreakNode::default()),
        ),
    )?;

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
                    paragraph { text("hello") }
                    paragraph { t2: text("world") }
                }
            }
            selection: (t2, 0) -> (t2, 3)
        };
        transact_fail!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
    }

    #[test]
    fn no_prev_sibling_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
    }

    #[test]
    fn prev_sibling_not_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote { paragraph { text("a") } }
                    paragraph { t1: text("hello") }
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
    }

    #[test]
    fn current_paragraph_not_root_child_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { text("a") }
                        paragraph { t1: text("b") }
                    }
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
    }

    #[test]
    fn prev_already_has_page_break_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("hello") page_break {} }
                    paragraph { t1: text("world") }
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
    }

    #[test]
    fn current_textblock_not_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("a") }
                    fold {
                        fold_title { t1: text("title") }
                        fold_content { paragraph {} }
                    }
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
    }

    #[test]
    fn inserts_into_prev_paragraph_with_text() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("hello") }
                    paragraph { t1: text("world") }
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { text("hello") page_break {} }
                    paragraph { t1: text("world") }
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn inserts_into_empty_prev_paragraph() {
        let (initial, ..) = state! {
            doc { root { paragraph {} p2: paragraph {} } }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { page_break {} } p2: paragraph {} } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn inserts_when_cursor_in_middle_of_current_paragraph() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("hello") }
                    paragraph { t1: text("world") }
                }
            }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { text("hello") page_break {} }
                    paragraph { t1: text("world") }
                }
            }
            selection: (t1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn pending_modifiers_preserved() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("hello") }
                    paragraph { t1: text("world") }
                }
            }
            selection: (t1, 0)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_page_break_into_prev_paragraph(&mut tr));
        assert!(!actual.pending_modifiers.is_empty());
    }
}
