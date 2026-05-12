use editor_model::Node;
use editor_state::{NodeRefCursorExt, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::{CommandError, CommandResult};

pub fn delete_empty_paragraph_forward(tr: &mut Transaction) -> CommandResult {
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

    let next_id = match node.next_sibling() {
        Some(next) => next.id(),
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
    let next = doc
        .node(next_id)
        .ok_or(CommandError::NodeNotFound(next_id))?;
    let cursor = next.first_cursor_position().ok_or(CommandError::Corrupted(
        "no cursor position in next sibling".into(),
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
                p1: paragraph {}
                paragraph { t1: text("hello") }
            } }
            selection: (p1, 0) -> (t1, 0)
        };
        transact_fail!(initial, |tr| delete_empty_paragraph_forward(&mut tr));
    }

    #[test]
    fn non_paragraph_node_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| delete_empty_paragraph_forward(&mut tr));
    }

    #[test]
    fn non_empty_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("a") }
                paragraph { text("b") }
            } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| delete_empty_paragraph_forward(&mut tr));
    }

    #[test]
    fn no_next_sibling_returns_false() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { text("a") }
                p1: paragraph {}
            } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| delete_empty_paragraph_forward(&mut tr));
    }

    #[test]
    fn delete_empty_paragraph_before_fold() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph {}
                fold {
                    fold_title { t1: text("title") }
                    fold_content { paragraph { text("content") } }
                }
                paragraph {}
            } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_empty_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("title") }
                    fold_content { paragraph { text("content") } }
                }
                paragraph {}
            } }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn delete_empty_paragraph_before_table() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph {}
                table {
                    table_row {
                        table_cell { paragraph { t1: text("cell") } }
                    }
                }
                paragraph {}
            } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_empty_paragraph_forward(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                table {
                    table_row {
                        table_cell { paragraph { t1: text("cell") } }
                    }
                }
                paragraph {}
            } }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
