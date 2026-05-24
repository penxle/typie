use editor_model::{NodeId, PlainNode, PlainTableCellNode};
use editor_transaction::Transaction;

use crate::helpers::col_count_from_table;
use crate::{CommandError, CommandResult};

pub fn set_table_column_widths(
    tr: &mut Transaction,
    table_id: NodeId,
    widths: Vec<f32>,
) -> CommandResult {
    if widths.is_empty() {
        return Ok(false);
    }
    let (row_ids, n_cols) = {
        let doc = tr.doc();
        let table = doc
            .node(table_id)
            .ok_or(CommandError::NodeNotFound(table_id))?;
        let n_cols = col_count_from_table(&table)?;
        let row_ids: Vec<NodeId> = table.children().map(|r| r.id()).collect();
        (row_ids, n_cols)
    };
    let update_count = widths.len().min(n_cols);
    for row_id in &row_ids {
        for (col_idx, &width) in widths.iter().enumerate().take(update_count) {
            let cell_id = {
                let doc = tr.doc();
                let row = doc
                    .node(*row_id)
                    .ok_or(CommandError::NodeNotFound(*row_id))?;
                let cell = row
                    .children()
                    .nth(col_idx)
                    .ok_or_else(|| CommandError::Corrupted("col index out of range".into()))?;
                cell.id()
            };
            let new_width = if width > 0.0 {
                Some(width as u32)
            } else {
                None
            };
            tr.set_node(
                cell_id,
                PlainNode::TableCell(PlainTableCellNode {
                    col_width: new_width,
                    background_color: None,
                }),
            )?;
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn sets_col_widths_on_all_rows() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        r0c0: table_cell { paragraph { text("A") } }
                        r0c1: table_cell { paragraph { text("B") } }
                    }
                    table_row {
                        r1c0: table_cell { paragraph { text("C") } }
                        r1c1: table_cell { paragraph { text("D") } }
                    }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_table_column_widths(
            &mut tr,
            tbl,
            vec![100.0, 200.0]
        ));
        let doc = actual.doc;
        let table = doc.node(tbl).unwrap();
        for row in table.children() {
            let cells: Vec<_> = row.children().collect();
            assert_eq!(cells.len(), 2);
            let c0 = cells[0].node();
            let c1 = cells[1].node();
            if let editor_model::Node::TableCell(n) = c0 {
                assert_eq!(*n.col_width.get(), Some(100));
            }
            if let editor_model::Node::TableCell(n) = c1 {
                assert_eq!(*n.col_width.get(), Some(200));
            }
        }
    }

    #[test]
    fn empty_widths_is_noop() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row { r0c0: table_cell { paragraph { text("A") } } }
                }
            } }
            selection: (r0c0, 0)
        };
        transact_fail!(initial, |tr| set_table_column_widths(&mut tr, tbl, vec![]));
    }

    #[test]
    fn partial_widths_only_updates_specified_cols() {
        let (initial, tbl, _r0c0, r0c1, ..) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        r0c0: table_cell { paragraph { text("A") } }
                        r0c1: table_cell { paragraph { text("B") } }
                    }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| set_table_column_widths(
            &mut tr,
            tbl,
            vec![150.0]
        ));
        let doc = actual.doc;
        let c1 = doc.node(r0c1).unwrap();
        if let editor_model::Node::TableCell(n) = c1.node() {
            assert_eq!(*n.col_width.get(), None);
        }
    }
}
