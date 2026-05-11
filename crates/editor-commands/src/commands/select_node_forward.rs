use editor_model::Node;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::{CommandError, CommandResult};

pub fn select_node_forward(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection();
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    let doc = tr.doc();
    let start = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let at_end = match start.node() {
        Node::Text(t) => pos.offset == t.text.len(),
        _ => pos.offset == start.entry().children.len(),
    };
    if !at_end {
        return Ok(false);
    }

    let mut current = start;
    let next = loop {
        if let Some(next) = current.next_sibling() {
            break next;
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return Ok(false),
        }
    };

    if !next.spec().is_leaf() || matches!(next.node(), Node::Text(_)) {
        return Ok(false);
    }

    let next_id = next.id();
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

    let doc = tr.doc();
    let next = doc
        .node(next_id)
        .ok_or(CommandError::NodeNotFound(next_id))?;
    let parent_id = next.parent().ok_or(CommandError::NoParent(next_id))?.id();
    let next_idx = next
        .index()
        .ok_or_else(|| CommandError::orphan_child(next_id, parent_id))?;

    tr.set_selection(Selection::new(
        Position {
            node_id: parent_id,
            offset: next_idx + 1,
            affinity: Affinity::Upstream,
        },
        Position {
            node_id: parent_id,
            offset: next_idx,
            affinity: Affinity::Downstream,
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
            doc { root { paragraph { t: text("Hello") } horizontal_rule paragraph } }
            selection: (t, 0) -> (t, 1)
        };
        transact_fail!(initial, |tr| select_node_forward(&mut tr));
    }

    #[test]
    fn rejects_if_not_on_last_position() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello") } horizontal_rule paragraph } }
            selection: (t, 3)
        };
        transact_fail!(initial, |tr| select_node_forward(&mut tr));
    }

    #[test]
    fn rejects_if_next_sibling_is_not_leaf() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello") } paragraph } }
            selection: (t, 5)
        };
        transact_fail!(initial, |tr| select_node_forward(&mut tr));
    }

    #[test]
    fn select_node_forward_on_last_position() {
        let (initial, ..) = state! {
            doc { root { paragraph { t: text("hello") } horizontal_rule paragraph } }
            selection: (t, 5)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_forward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root { paragraph { text("hello") } horizontal_rule paragraph } }
            selection: (r, 2, <) -> (r, 1, >)
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn select_node_forward_removes_empty_paragraph() {
        let (initial, ..) = state! {
            doc { root { paragraph { text("hello") } p: paragraph horizontal_rule paragraph } }
            selection: (p, 0)
        };

        let (actual, ..) = transact!(initial, |tr| select_node_forward(&mut tr));

        let (expected, ..) = state! {
            doc { r: root { paragraph { text("hello") } horizontal_rule paragraph } }
            selection: (r, 2, <) -> (r, 1, >)
        };

        assert_state_eq!(actual, expected);
    }
}
