use editor_model::{ChildView, Node};
use editor_state::Selection;
use editor_state::last_cursor_position;
use editor_transaction::{Transaction, fulfill};

use crate::helpers::prev_sibling;
use crate::{CommandError, CommandResult};

pub fn delete_empty_paragraph_backward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;
    if pos.offset != 0 {
        return Ok(false);
    }

    let (prev_id, parent_id) = {
        let view = tr.state().view();
        let node = view
            .node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;

        if !matches!(node.node(), Node::Paragraph(_)) {
            return Ok(false);
        }
        if node.first_child().is_some() {
            return Ok(false);
        }

        let parent_id = node.parent().ok_or(CommandError::NoParent(pos.node))?.id();
        let prev = match prev_sibling(&node) {
            Some(ChildView::Block(prev)) => prev,
            _ => return Ok(false),
        };
        (prev.id(), parent_id)
    };

    let paragraph_id = pos.node;

    tr.batch::<_, CommandError>(|tr| {
        tr.remove_subtree(paragraph_id)?;
        let steps = {
            let view = tr.state().view();
            view.node(parent_id).map(|parent| fulfill(&parent))
        };
        if let Some(steps) = steps {
            tr.apply_steps(steps)?;
        }
        Ok(())
    })?;

    let cursor = {
        let view = tr.state().view();
        let prev = view
            .node(prev_id)
            .ok_or(CommandError::NodeNotFound(prev_id))?;
        last_cursor_position(&prev).ok_or(CommandError::Corrupted(
            "no cursor position in prev sibling".into(),
        ))?
    };

    tr.set_selection(Some(Selection::collapsed(cursor)))?;

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
                p1: paragraph { text("hello") }
                p2: paragraph {}
            } }
            selection: (p1, 0) -> (p2, 0)
        };
        transact_fail!(initial, |tr| delete_empty_paragraph_backward(&mut tr));
    }

    #[test]
    fn non_paragraph_node_returns_false() {
        let (initial, ..) = state! {
            doc { root { bq: blockquote { paragraph { text("hello") } } } }
            selection: (bq, 0)
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
    fn image_prev_sibling_returns_false() {
        let (initial, ..) = state! {
            doc { root {
                paragraph { text("a") }
                image
                p1: paragraph {}
            } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| delete_empty_paragraph_backward(&mut tr));
    }
}
