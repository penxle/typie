use editor_model::NodeId;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{delete_selection_range, selection_for_node};

pub fn delete_node(tr: &mut Transaction, node_id: NodeId) -> CommandResult {
    let doc = tr.doc();
    let Some(selection) = selection_for_node(&doc, node_id)? else {
        return Ok(false);
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
        let (initial, _root, _t1, img, ..) = state! {
            doc { r: root {
                paragraph { t1: text("Before") }
                img: image
                paragraph { t2: text("After") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| delete_node(&mut tr, img));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("Before") }
                paragraph { t2: text("After") }
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn fulfills_empty_parent_after_delete() {
        let (initial, paragraph, ..) = state! {
            doc { root { paragraph: paragraph { t1: text("Only") } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| delete_node(&mut tr, paragraph));
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
                paragraph { t1: text("After") }
            } }
            selection: (table, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_node(&mut tr, table));
        let (expected, ..) = state! {
            doc { root {
                paragraph { t1: text("After") }
            } }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
