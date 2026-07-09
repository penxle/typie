use editor_common::Axis;
use editor_crdt::Dot;
use editor_state::Selection;
use editor_state::first_cursor_position;
use editor_transaction::Transaction;

use crate::helpers::{insert_empty_table_column, insert_empty_table_row};
use crate::{CommandError, CommandResult};

pub fn insert_table_axis(
    tr: &mut Transaction,
    table_id: Dot,
    axis: Axis,
    index: usize,
    before: bool,
) -> CommandResult {
    let insertion_index = if before { index } else { index + 1 };
    match axis {
        Axis::Horizontal => {
            insert_empty_table_row(tr, table_id, insertion_index)?;

            let pos = {
                let view = tr.view();
                let table = view
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                let row = table
                    .child_blocks()
                    .nth(insertion_index)
                    .ok_or_else(|| CommandError::Corrupted("inserted row not found".into()))?;
                let cell = row
                    .child_blocks()
                    .next()
                    .ok_or_else(|| CommandError::Corrupted("new row has no cells".into()))?;
                first_cursor_position(&cell)
                    .ok_or_else(|| CommandError::Corrupted("cell has no cursor position".into()))?
            };
            tr.set_selection(Some(Selection::collapsed(pos)))?;
        }
        Axis::Vertical => {
            insert_empty_table_column(tr, table_id, insertion_index)?;

            let pos = {
                let view = tr.view();
                let table = view
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                let row = table
                    .child_blocks()
                    .next()
                    .ok_or_else(|| CommandError::Corrupted("table has no rows".into()))?;
                let cell = row
                    .child_blocks()
                    .nth(insertion_index)
                    .ok_or_else(|| CommandError::Corrupted("new cell not found".into()))?;
                first_cursor_position(&cell)
                    .ok_or_else(|| CommandError::Corrupted("cell has no cursor position".into()))?
            };
            tr.set_selection(Some(Selection::collapsed(pos)))?;
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
    fn insert_row_before_first() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row { r0c0: table_cell { paragraph { text("A") } } }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_table_axis(
            &mut tr,
            tbl,
            Axis::Horizontal,
            0,
            true
        ));
        let v = actual.view();
        let table = v.node(tbl).unwrap();
        assert_eq!(table.child_blocks().count(), 2);
        let new_row = table.child_blocks().next().unwrap();
        for cell in new_row.child_blocks() {
            assert_empty_cell(&cell);
        }
    }

    #[test]
    fn insert_row_after_first() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row { r0c0: table_cell { paragraph { text("A") } } }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_table_axis(
            &mut tr,
            tbl,
            Axis::Horizontal,
            0,
            false
        ));
        let v = actual.view();
        let table = v.node(tbl).unwrap();
        assert_eq!(table.child_blocks().count(), 2);
        let new_row = table.child_blocks().nth(1).unwrap();
        for cell in new_row.child_blocks() {
            assert_empty_cell(&cell);
        }
    }

    #[test]
    fn insert_col_before_first() {
        let (initial, tbl, ..) = state! {
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
        let (actual, ..) = transact!(initial, |tr| insert_table_axis(
            &mut tr,
            tbl,
            Axis::Vertical,
            0,
            true
        ));
        let v = actual.view();
        let table = v.node(tbl).unwrap();
        let row = table.child_blocks().next().unwrap();
        assert_eq!(row.child_blocks().count(), 3);
        assert_empty_cell(&row.child_blocks().next().unwrap());
    }

    #[test]
    fn insert_col_after_last() {
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
        let (actual, ..) = transact!(initial, |tr| insert_table_axis(
            &mut tr,
            tbl,
            Axis::Vertical,
            1,
            false
        ));
        let v = actual.view();
        let table = v.node(tbl).unwrap();
        for row in table.child_blocks() {
            assert_eq!(row.child_blocks().count(), 3);
            assert_empty_cell(&row.child_blocks().nth(2).unwrap());
        }
    }

    #[test]
    fn insert_col_rebalances_widths_on_all_rows() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        r0c0: table_cell(col_width: Some(300u32)) { paragraph { text("A") } }
                        table_cell(col_width: Some(100u32)) { paragraph { text("B") } }
                        table_cell(col_width: Some(100u32)) { paragraph { text("C") } }
                    }
                    table_row {
                        table_cell(col_width: Some(300u32)) { paragraph { text("D") } }
                        table_cell(col_width: Some(100u32)) { paragraph { text("E") } }
                        table_cell(col_width: Some(100u32)) { paragraph { text("F") } }
                    }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_table_axis(
            &mut tr,
            tbl,
            Axis::Vertical,
            0,
            false
        ));
        let v = actual.view();
        let table = v.node(tbl).unwrap();
        for row in table.child_blocks() {
            let widths: Vec<Option<u32>> = row
                .child_blocks()
                .map(|cell| match cell.node() {
                    editor_model::Node::TableCell(n) => *n.col_width.get(),
                    _ => panic!("expected a table cell"),
                })
                .collect();
            assert_eq!(
                widths,
                vec![Some(225), Some(125), Some(75), Some(75)],
                "existing column weights are rebalanced and the inserted column gets its own weight"
            );
        }
    }

    #[test]
    fn insert_row_carries_existing_width_weights() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        r0c0: table_cell(col_width: Some(300u32)) { paragraph { text("A") } }
                        table_cell(col_width: Some(100u32)) { paragraph { text("B") } }
                    }
                }
            } }
            selection: (r0c0, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_table_axis(
            &mut tr,
            tbl,
            Axis::Horizontal,
            0,
            true
        ));
        let v = actual.view();
        let table = v.node(tbl).unwrap();
        for row in table.child_blocks() {
            let widths: Vec<Option<u32>> = row
                .child_blocks()
                .map(|cell| match cell.node() {
                    editor_model::Node::TableCell(n) => *n.col_width.get(),
                    _ => panic!("expected a table cell"),
                })
                .collect();
            assert_eq!(widths, vec![Some(300), Some(100)]);
        }
    }

    fn assert_empty_cell(cell: &editor_model::NodeView<'_>) {
        assert_eq!(cell.child_blocks().count(), 1, "cell should have one child");
        let para = cell.child_blocks().next().unwrap();
        assert_eq!(para.node_type(), editor_model::NodeType::Paragraph);
        assert_eq!(para.children().count(), 0, "paragraph should be empty");
    }
}
