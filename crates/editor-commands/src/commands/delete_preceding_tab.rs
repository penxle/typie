use editor_model::{AtomLeaf, ChildView};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::remove_atom_leaf;
use crate::{CommandError, CommandResult};

pub fn delete_preceding_tab(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }
    let pos = selection.head;
    if pos.offset == 0 {
        return Ok(false);
    }

    let tab_index = pos.offset - 1;
    {
        let view = tr.view();
        let node = view
            .node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;
        let Some(ChildView::Leaf(prev)) = node.child_at(tab_index) else {
            return Ok(false);
        };
        if prev.as_atom() != Some(&AtomLeaf::Tab) {
            return Ok(false);
        }
    }

    let new_offset = pos.offset - 1;
    remove_atom_leaf(tr, pos.node, tab_index)?;
    tr.set_selection(Some(Selection::collapsed(Position {
        node: pos.node,
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
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_preceding_tab(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn no_op_when_prev_is_not_tab() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("a") hard_break text("b") } } }
            selection: (p1, 2)
        };
        transact_fail!(initial, |tr| delete_preceding_tab(&mut tr));
    }

    #[test]
    fn no_op_in_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("abc") } } }
            selection: (p1, 2)
        };
        transact_fail!(initial, |tr| delete_preceding_tab(&mut tr));
    }
}
