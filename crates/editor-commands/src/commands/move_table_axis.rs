use editor_common::Axis;
use editor_model::NodeId;
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub fn move_table_axis(
    tr: &mut Transaction,
    table_id: NodeId,
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
                let doc = tr.doc();
                let table = doc
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                table
                    .children()
                    .nth(from)
                    .ok_or_else(|| CommandError::Corrupted("row index out of range".into()))?
                    .id()
            };
            tr.move_node(row_id, table_id, to)?;
        }
        Axis::Vertical => {
            let row_ids: Vec<NodeId> = {
                let doc = tr.doc();
                let table = doc
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                table.children().map(|r| r.id()).collect()
            };
            for row_id in &row_ids {
                let cell_id = {
                    let doc = tr.doc();
                    let row = doc
                        .node(*row_id)
                        .ok_or(CommandError::NodeNotFound(*row_id))?;
                    row.children()
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
        // depth-first extraction: tbl, row0, t1, row1
        let (initial, tbl, row0, _t1, row1, ..) = state! {
            doc { root {
                tbl: table {
                    row0: table_row { t1: table_cell { paragraph { text("A") } } }
                    row1: table_row { table_cell { paragraph { text("B") } } }
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
        let doc = actual.doc;
        let table = doc.node(tbl).unwrap();
        let children: Vec<NodeId> = table.children().map(|r| r.id()).collect();
        assert_eq!(children, vec![row1, row0]);
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
        let doc = actual.doc;
        let table = doc.node(tbl).unwrap();
        for row in table.children() {
            assert_eq!(row.children().count(), 3);
        }
    }
}
