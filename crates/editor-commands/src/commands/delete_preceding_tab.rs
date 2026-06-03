use editor_model::Node;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn delete_preceding_tab(tr: &mut Transaction) -> CommandResult {
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

    let prev = match node.node() {
        Node::Text(_) => {
            if pos.offset > 0 {
                return Ok(false);
            }
            node.prev_sibling()
        }
        _ => {
            if pos.offset == 0 {
                return Ok(false);
            }
            node.children().nth(pos.offset - 1)
        }
    };

    let Some(prev) = prev else {
        return Ok(false);
    };
    if !matches!(prev.node(), Node::Tab(_)) {
        return Ok(false);
    }

    let target_id = prev.id();
    let is_text = matches!(node.node(), Node::Text(_));
    let new_offset = if is_text { 0 } else { pos.offset - 1 };
    tr.remove_subtree(target_id)?;
    tr.set_selection(Some(Selection::collapsed(Position {
        node_id: pos.node_id,
        offset: new_offset,
        affinity: Affinity::Downstream,
    })))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn deletes_tab_before_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("a") tab t2: text("b") } } }
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_preceding_tab(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("a") t2: text("b") } } }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn no_op_when_prev_is_not_tab() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("a") hard_break t2: text("b") } } }
            selection: (t2, 0)
        };
        transact_fail!(initial, |tr| delete_preceding_tab(&mut tr));
    }

    #[test]
    fn no_op_in_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("abc") } } }
            selection: (t1, 2)
        };
        transact_fail!(initial, |tr| delete_preceding_tab(&mut tr));
    }
}
