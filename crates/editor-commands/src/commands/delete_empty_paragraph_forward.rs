use editor_model::Node;
use editor_state::Selection;
use editor_state::first_cursor_position;
use editor_transaction::{Transaction, fulfill};

use crate::{CommandError, CommandResult};

pub fn delete_empty_paragraph_forward(tr: &mut Transaction) -> CommandResult {
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

    let (next_id, parent_id) = {
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

        let parent = node.parent().ok_or(CommandError::NoParent(pos.node))?;
        let idx = parent
            .child_blocks()
            .position(|b| b.id() == node.id())
            .ok_or_else(|| CommandError::orphan_child(pos.node, parent.id()))?;
        let next = match parent.child_blocks().nth(idx + 1) {
            Some(next) => next,
            None => return Ok(false),
        };
        (next.id(), parent.id())
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
        let next = view
            .node(next_id)
            .ok_or(CommandError::NodeNotFound(next_id))?;
        first_cursor_position(&next).ok_or(CommandError::Corrupted(
            "no cursor position in next sibling".into(),
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
                p1: paragraph {}
                p2: paragraph { text("hello") }
            } }
            selection: (p1, 0) -> (p2, 0)
        };
        transact_fail!(initial, |tr| delete_empty_paragraph_forward(&mut tr));
    }

    #[test]
    fn non_paragraph_node_returns_false() {
        let (initial, ..) = state! {
            doc { root { bq: blockquote { paragraph { text("hello") } } } }
            selection: (bq, 0)
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
}
