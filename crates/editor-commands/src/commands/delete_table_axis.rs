use editor_common::Axis;
use editor_crdt::Dot;
use editor_state::Selection;
use editor_state::first_cursor_position;
use editor_transaction::Transaction;

use crate::helpers::{col_count_from_table, cursor_pos_in_table};
use crate::{CommandError, CommandResult};

pub fn delete_table_axis(
    tr: &mut Transaction,
    table_id: Dot,
    axis: Axis,
    index: usize,
) -> CommandResult {
    match axis {
        Axis::Horizontal => {
            let (row_count, n_cols) = {
                let view = tr.view();
                let table = view
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                (table.child_blocks().count(), col_count_from_table(&table)?)
            };
            // Invariant: table must keep at least one row.
            if row_count <= 1 {
                return Ok(false);
            }
            let col_hint = cursor_pos_in_table(tr, table_id)
                .map(|(_, c)| c)
                .unwrap_or(0);
            let row_id = {
                let view = tr.view();
                let table = view
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                table
                    .child_blocks()
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
                let view = tr.view();
                let table = view
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                let row_ids: Vec<Dot> = table.child_blocks().map(|r| r.id()).collect();
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
                    let view = tr.view();
                    let row = view
                        .node(*row_id)
                        .ok_or(CommandError::NodeNotFound(*row_id))?;
                    row.child_blocks()
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
    table_id: Dot,
    row_index: usize,
    col_index: usize,
) -> Result<(), CommandError> {
    let pos = {
        let view = tr.view();
        let table = view
            .node(table_id)
            .ok_or(CommandError::NodeNotFound(table_id))?;
        let row = table
            .child_blocks()
            .nth(row_index)
            .ok_or_else(|| CommandError::Corrupted("row index out of range".into()))?;
        let cell = row
            .child_blocks()
            .nth(col_index)
            .ok_or_else(|| CommandError::Corrupted("col index out of range".into()))?;
        first_cursor_position(&cell)
            .ok_or_else(|| CommandError::Corrupted("cell has no cursor position".into()))?
    };
    tr.set_selection(Some(Selection::collapsed(pos)))?;
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
        let v = actual.view();
        let table = v.node(tbl).unwrap();
        assert_eq!(table.child_blocks().count(), 1);
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
        let v = actual.view();
        let table = v.node(tbl).unwrap();
        for row in table.child_blocks() {
            assert_eq!(row.child_blocks().count(), 1);
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
        let v = actual.view();
        let r0c0_node = v.node(r0c0).unwrap();
        let expected_pos = first_cursor_position(&r0c0_node).unwrap();
        let expected = Selection::collapsed(expected_pos);
        let sel = actual.selection.unwrap();
        assert_eq!(sel, expected);
    }
}
