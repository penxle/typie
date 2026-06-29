use editor_crdt::Dot;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{delete_selection_range, selection_for_node};

pub fn delete_node(tr: &mut Transaction, node: Dot) -> CommandResult {
    let selection = {
        let view = tr.state().view();
        match selection_for_node(&view, node)? {
            Some(selection) => selection,
            None => return Ok(false),
        }
    };
    delete_selection_range(tr, selection)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn deletes_block_node_and_moves_selection_to_next_text_position() {
        let (initial, _root, _p1, img, ..) = state! {
            doc { r: root {
                p1: paragraph { text("Before") }
                img: image
                p2: paragraph { text("After") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| delete_node(&mut tr, img));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("Before") }
                p2: paragraph { text("After") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn fulfills_empty_parent_after_delete() {
        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { text("Only") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_node(&mut tr, p1));
        let (expected, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn deletes_table_and_moves_selection_to_following_paragraph() {
        let (initial, table, ..) = state! {
            doc { root {
                table: table {
                    table_row {
                        table_cell { paragraph { text("cell") } }
                    }
                }
                p1: paragraph { text("After") }
            } }
            selection: (table, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_node(&mut tr, table));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("After") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
