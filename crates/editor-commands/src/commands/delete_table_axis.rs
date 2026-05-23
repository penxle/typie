use editor_common::Axis;
use editor_model::NodeId;
use editor_state::{NodeRefCursorExt, Selection};
use editor_transaction::Transaction;

use crate::helpers::{col_count_from_table, cursor_pos_in_table};
use crate::{CommandError, CommandResult};

pub fn delete_table_axis(
    tr: &mut Transaction,
    table_id: NodeId,
    axis: Axis,
    index: usize,
) -> CommandResult {
    match axis {
        Axis::Horizontal => {
            let (row_count, n_cols) = {
                let doc = tr.doc();
                let table = doc
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                (table.children().count(), col_count_from_table(&table)?)
            };
            // Invariant: table must keep at least one row.
            if row_count <= 1 {
                return Ok(false);
            }
            let col_hint = cursor_pos_in_table(tr, table_id)
                .map(|(_, c)| c)
                .unwrap_or(0);
            let row_id = {
                let doc = tr.doc();
                let table = doc
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                table
                    .children()
                    .nth(index)
                    .ok_or_else(|| CommandError::Corrupted("row index out of range".into()))?
                    .id()
            };
            tr.remove_subtree(row_id)?;
            let land = index.min(row_count - 2);
            restore_selection_to_cell(tr, table_id, land, col_hint.min(n_cols - 1))?;
        }
        Axis::Vertical => {
            let (row_ids, n_cols, row_count) = {
                let doc = tr.doc();
                let table = doc
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                let row_ids: Vec<NodeId> = table.children().map(|r| r.id()).collect();
                let row_count = row_ids.len();
                let n_cols = col_count_from_table(&table)?;
                (row_ids, n_cols, row_count)
            };
            // Invariant: table must keep at least one column.
            if n_cols <= 1 {
                return Ok(false);
            }
            let row_hint = cursor_pos_in_table(tr, table_id)
                .map(|(r, _)| r)
                .unwrap_or(0);
            for row_id in &row_ids {
                let cell_id = {
                    let doc = tr.doc();
                    let row = doc
                        .node(*row_id)
                        .ok_or(CommandError::NodeNotFound(*row_id))?;
                    row.children()
                        .nth(index)
                        .ok_or_else(|| CommandError::Corrupted("col index out of range".into()))?
                        .id()
                };
                tr.remove_subtree(cell_id)?;
            }
            let land = index.min(n_cols - 2);
            restore_selection_to_cell(tr, table_id, row_hint.min(row_count - 1), land)?;
        }
    }
    Ok(true)
}

fn restore_selection_to_cell(
    tr: &mut Transaction,
    table_id: NodeId,
    row_index: usize,
    col_index: usize,
) -> Result<(), CommandError> {
    let pos = {
        let doc = tr.doc();
        let table = doc
            .node(table_id)
            .ok_or(CommandError::NodeNotFound(table_id))?;
        let row = table
            .children()
            .nth(row_index)
            .ok_or_else(|| CommandError::Corrupted("row index out of range".into()))?;
        let cell = row
            .children()
            .nth(col_index)
            .ok_or_else(|| CommandError::Corrupted("col index out of range".into()))?;
        cell.first_cursor_position()
            .ok_or_else(|| CommandError::Corrupted("cell has no cursor position".into()))?
    };
    tr.set_selection(Selection::collapsed(pos))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn delete_row_in_2x2() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        r0c0: table_cell { paragraph { text("A") } }
                        table_cell { paragraph { text("B") } }
                    }
                    table_row {
                        table_cell { paragraph { text("C") } }
                        table_cell { paragraph { text("D") } }
                    }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_table_axis(
            &mut tr,
            tbl,
            Axis::Horizontal,
            0
        ));
        let doc = actual.doc;
        let table = doc.node(tbl).unwrap();
        assert_eq!(table.children().count(), 1);
    }

    #[test]
    fn delete_last_row_rejected() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row { r0c0: table_cell { paragraph { text("A") } } }
                }
            } }
            selection: (r0c0, 0)
        };
        transact_fail!(initial, |tr| delete_table_axis(
            &mut tr,
            tbl,
            Axis::Horizontal,
            0
        ));
    }

    #[test]
    fn delete_col_in_2x2() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        r0c0: table_cell { paragraph { text("A") } }
                        table_cell { paragraph { text("B") } }
                    }
                    table_row {
                        table_cell { paragraph { text("C") } }
                        table_cell { paragraph { text("D") } }
                    }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_table_axis(
            &mut tr,
            tbl,
            Axis::Vertical,
            0
        ));
        let doc = actual.doc;
        let table = doc.node(tbl).unwrap();
        for row in table.children() {
            assert_eq!(row.children().count(), 1);
        }
    }

    #[test]
    fn delete_last_col_rejected() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row { r0c0: table_cell { paragraph { text("A") } } }
                }
            } }
            selection: (r0c0, 0)
        };
        transact_fail!(initial, |tr| delete_table_axis(
            &mut tr,
            tbl,
            Axis::Vertical,
            0
        ));
    }

    #[test]
    fn delete_row_restores_selection_to_previous_row() {
        let (initial, tbl, r0c0, ..) = state! {
            doc { root {
                tbl: table {
                    table_row { r0c0: table_cell { paragraph { text("A") } } }
                    table_row { r1c0: table_cell { paragraph { text("B") } } }
                }
            } }
            selection: (r1c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| delete_table_axis(
            &mut tr,
            tbl,
            Axis::Horizontal,
            1
        ));
        let sel = actual.selection;
        let doc = actual.doc;
        let r0c0_node = doc.node(r0c0).unwrap();
        let expected_pos = r0c0_node.first_cursor_position().unwrap();
        let expected = Selection::collapsed(expected_pos);
        assert_eq!(sel, expected);
    }
}
