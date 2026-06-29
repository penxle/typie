use editor_common::Axis;
use editor_crdt::Dot;
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn move_table_axis(
    tr: &mut Transaction,
    table_id: Dot,
    axis: Axis,
    from: usize,
    to: usize,
) -> CommandResult {
    if from == to {
        return Ok(false);
    }
    match axis {
        Axis::Horizontal => {
            let row_id = {
                let view = tr.view();
                let table = view
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                table
                    .child_blocks()
                    .nth(from)
                    .ok_or_else(|| CommandError::Corrupted("row index out of range".into()))?
                    .id()
            };
            tr.move_node(row_id, table_id, to)?;
        }
        Axis::Vertical => {
            let row_ids: Vec<Dot> = {
                let view = tr.view();
                let table = view
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                table.child_blocks().map(|r| r.id()).collect()
            };
            for row_id in &row_ids {
                let cell_id = {
                    let view = tr.view();
                    let row = view
                        .node(*row_id)
                        .ok_or(CommandError::NodeNotFound(*row_id))?;
                    row.child_blocks()
                        .nth(from)
                        .ok_or_else(|| CommandError::Corrupted("col index out of range".into()))?
                        .id()
                };
                tr.move_node(cell_id, *row_id, to)?;
            }
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
    fn move_row_same_index_is_noop() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row { r0c0: table_cell { paragraph { text("A") } } }
                    table_row { r1c0: table_cell { paragraph { text("B") } } }
                }
            } }
            selection: (r0c0, 0)
        };
        transact_fail!(initial, |tr| move_table_axis(
            &mut tr,
            tbl,
            Axis::Horizontal,
            0,
            0
        ));
    }

    #[test]
    fn move_row_down() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row { t1: table_cell { paragraph { text("A") } } }
                    table_row { table_cell { paragraph { text("B") } } }
                }
            } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| move_table_axis(
            &mut tr,
            tbl,
            Axis::Horizontal,
            0,
            1
        ));
        let view = actual.view();
        let table = view.node(tbl).unwrap();
        // move_node re-mints block ids, so assert row order by cell content.
        let texts: Vec<String> = table
            .child_blocks()
            .map(|row| {
                row.child_blocks()
                    .next()
                    .and_then(|cell| cell.child_blocks().next())
                    .map(|para| para.inline_text())
                    .unwrap_or_default()
            })
            .collect();
        assert_eq!(texts, vec!["B".to_string(), "A".to_string()]);
    }

    #[test]
    fn move_col_right() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        r0c0: table_cell { paragraph { text("A") } }
                        r0c1: table_cell { paragraph { text("B") } }
                        r0c2: table_cell { paragraph { text("C") } }
                    }
                    table_row {
                        r1c0: table_cell { paragraph { text("D") } }
                        r1c1: table_cell { paragraph { text("E") } }
                        r1c2: table_cell { paragraph { text("F") } }
                    }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| move_table_axis(
            &mut tr,
            tbl,
            Axis::Vertical,
            0,
            2
        ));
        let view = actual.view();
        let table = view.node(tbl).unwrap();
        for row in table.child_blocks() {
            assert_eq!(row.child_blocks().count(), 3);
        }
    }
}
