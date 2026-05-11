use editor_model::Node;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::{CommandError, CommandResult};

pub fn select_node_backward(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection();
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    if pos.offset != 0 {
        return Ok(false);
    }

    let doc = tr.doc();
    let start = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let mut current = start;
    let prev = loop {
        if let Some(prev) = current.prev_sibling() {
            break prev;
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return Ok(false),
        }
    };

    if !prev.spec().is_leaf() || matches!(prev.node(), Node::Text(_)) {
        return Ok(false);
    }

    let parent_id = prev.parent().ok_or(CommandError::NoParent(prev.id()))?.id();
    let prev_idx = prev
        .index()
        .ok_or_else(|| CommandError::orphan_child(prev.id(), parent_id))?;

    let remove_start = !start.spec().is_leaf() && start.entry().children.is_empty();
    let start_id = start.id();
    let start_parent_id = start.parent().map(|p| p.id());

    if remove_start {
        tr.batch::<_, CommandError>(|tr| {
            tr.remove_subtree(start_id)?;
            if let Some(pid) = start_parent_id {
                let doc = tr.doc();
                if let Some(parent) = doc.node(pid) {
                    tr.apply_steps(fulfill(&parent))?;
                }
            }
            Ok(())
        })?;
    }

    tr.set_selection(Selection::new(
        Position {
            node_id: parent_id,
            offset: prev_idx,
            affinity: Affinity::Downstream,
        },
        Position {
            node_id: parent_id,
            offset: prev_idx + 1,
            affinity: Affinity::Upstream,
        },
    ))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn rejects_range_selection() {
        let (initial, ..) = state! {
            doc { root { horizontal_rule paragraph { t: text("Hello") } } }
            selection: (t, 0) -> (t, 1)
        };
        transact_fail!(initial, |tr| select_node_backward(&mut tr));
    }

    #[test]
    fn rejects_if_not_on_first_position() {
        let (initial, ..) = state! {
            doc { root { horizontal_rule paragraph { t: text("hello") } } }
            selection: (t, 1)
        };
        transact_fail!(initial, |tr| select_node_backward(&mut tr));
    }

    #[test]
    fn rejects_if_prev_sibling_is_not_leaf() {
        let (initial, ..) = state! {
            doc { root { paragraph paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
        transact_fail!(initial, |tr| select_node_backward(&mut tr));
    }

    #[test]
    fn select_node_backward_on_first_position() {
        let (initial, ..) = state! {
            doc { root { horizontal_rule paragraph { t: text("hello") } } }
            selection: (t, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_backward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root { horizontal_rule paragraph { text("hello") } } }
            selection: (r, 0, >) -> (r, 1, <)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_backward_removes_empty_paragraph() {
        let (initial, ..) = state! {
            doc { root { horizontal_rule p: paragraph paragraph { text("hello") } } }
            selection: (p, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_backward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root { horizontal_rule paragraph { text("hello") } } }
            selection: (r, 0, >) -> (r, 1, <)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_backward_removes_empty_paragraph_but_keeps_trailing_paragraph() {
        let (initial, ..) = state! {
            doc { root { horizontal_rule p: paragraph } }
            selection: (p, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_backward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root { horizontal_rule paragraph } }
            selection: (r, 0, >) -> (r, 1, <)
        };

        assert_state_eq!(actual, expected);
    }
}
