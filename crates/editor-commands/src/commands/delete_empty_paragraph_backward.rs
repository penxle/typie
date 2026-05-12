use editor_model::Node;
use editor_state::{NodeRefCursorExt, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::{CommandError, CommandResult};

pub fn delete_empty_paragraph_backward(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection();
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    if pos.offset != 0 {
        return Ok(false);
    }

    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    if !matches!(node.node(), Node::Paragraph(_)) {
        return Ok(false);
    }
    if node.first_child().is_some() {
        return Ok(false);
    }

    let prev_id = match node.prev_sibling() {
        Some(prev) => prev.id(),
        None => return Ok(false),
    };

    let paragraph_id = pos.node_id;
    let parent_id = node
        .parent()
        .ok_or(CommandError::NoParent(paragraph_id))?
        .id();

    tr.batch::<_, CommandError>(|tr| {
        tr.remove_subtree(paragraph_id)?;
        let doc = tr.doc();
        if let Some(parent) = doc.node(parent_id) {
            tr.apply_steps(fulfill(&parent))?;
        }
        Ok(())
    })?;

    let doc = tr.doc();
    let prev = doc
        .node(prev_id)
        .ok_or(CommandError::NodeNotFound(prev_id))?;
    let cursor = prev.last_cursor_position().ok_or(CommandError::Corrupted(
        "no cursor position in prev sibling".into(),
    ))?;

    tr.set_selection(Selection::collapsed(cursor))?;

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
            doc { root {
                paragraph { t1: text("hello") }
                p2: paragraph {}
            } }
            selection: (t1, 0) -> (p2, 0)
        };
        transact_fail!(initial, |tr| delete_empty_paragraph_backward(&mut tr));
    }

    #[test]
    fn non_paragraph_node_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| delete_empty_paragraph_backward(&mut tr));
    }

    #[test]
    fn non_empty_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { text("a") }
                p2: paragraph { text("b") }
            } }
            selection: (p2, 0)
        };
        transact_fail!(initial, |tr| delete_empty_paragraph_backward(&mut tr));
    }

    #[test]
    fn no_prev_sibling_returns_false() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph {}
                paragraph { text("a") }
            } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| delete_empty_paragraph_backward(&mut tr));
    }

    #[test]
    fn delete_empty_paragraph_after_fold() {
        let (initial, ..) = state! {
            doc { root {
                paragraph {}
                fold {
                    fold_title { text("title") }
                    fold_content { paragraph { t1: text("content") } }
                }
                p2: paragraph {}
            } }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_empty_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph {}
                fold {
                    fold_title { text("title") }
                    fold_content { paragraph { t1: text("content") } }
                }
                paragraph {}
            } }
            selection: (t1, 7)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_empty_paragraph_after_table() {
        let (initial, ..) = state! {
            doc { root {
                paragraph {}
                table {
                    table_row {
                        table_cell { paragraph { t1: text("cell") } }
                    }
                }
                p2: paragraph {}
            } }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_empty_paragraph_backward(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph {}
                table {
                    table_row {
                        table_cell { paragraph { t1: text("cell") } }
                    }
                }
                paragraph {}
            } }
            selection: (t1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }
}
