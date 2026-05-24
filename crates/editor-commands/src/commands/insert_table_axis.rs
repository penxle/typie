use editor_common::Axis;
use editor_model::{
    NodeId, PlainNode, PlainParagraphNode, PlainTableCellNode, PlainTableRowNode, PlainTextNode,
    Subtree,
};
use editor_state::{NodeRefCursorExt, Selection};
use editor_transaction::Transaction;

use crate::helpers::col_count_from_table;
use crate::{CommandError, CommandResult};

pub fn insert_table_axis(
    tr: &mut Transaction,
    table_id: NodeId,
    axis: Axis,
    index: usize,
    before: bool,
) -> CommandResult {
    match axis {
        Axis::Horizontal => {
            let n_cols = {
                let doc = tr.doc();
                let table = doc
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                col_count_from_table(&table)?
            };
            let insertion_index = if before { index } else { index + 1 };
            let row = make_empty_row(n_cols);
            tr.insert_subtree(table_id, insertion_index, row)?;

            let pos = {
                let doc = tr.doc();
                let table = doc
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                let row = table
                    .children()
                    .nth(insertion_index)
                    .ok_or_else(|| CommandError::Corrupted("inserted row not found".into()))?;
                let cell = row
                    .children()
                    .next()
                    .ok_or_else(|| CommandError::Corrupted("new row has no cells".into()))?;
                cell.first_cursor_position()
                    .ok_or_else(|| CommandError::Corrupted("cell has no cursor position".into()))?
            };
            tr.set_selection(Some(Selection::collapsed(pos)))?;
        }
        Axis::Vertical => {
            let row_ids: Vec<NodeId> = {
                let doc = tr.doc();
                let table = doc
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                table.children().map(|r| r.id()).collect()
            };
            let col_insertion_index = if before { index } else { index + 1 };
            for row_id in &row_ids {
                tr.insert_subtree(*row_id, col_insertion_index, make_empty_cell())?;
            }

            let pos = {
                let doc = tr.doc();
                let table = doc
                    .node(table_id)
                    .ok_or(CommandError::NodeNotFound(table_id))?;
                let row = table
                    .children()
                    .next()
                    .ok_or_else(|| CommandError::Corrupted("table has no rows".into()))?;
                let cell = row
                    .children()
                    .nth(col_insertion_index)
                    .ok_or_else(|| CommandError::Corrupted("new cell not found".into()))?;
                cell.first_cursor_position()
                    .ok_or_else(|| CommandError::Corrupted("cell has no cursor position".into()))?
            };
            tr.set_selection(Some(Selection::collapsed(pos)))?;
        }
    }
    Ok(true)
}

fn make_empty_cell() -> Subtree {
    let cell_id = NodeId::new();
    let para_id = NodeId::new();
    let text_id = NodeId::new();
    Subtree::leaf(
        cell_id,
        PlainNode::TableCell(PlainTableCellNode {
            col_width: None,
            background_color: None,
        }),
    )
    .with_children(vec![
        Subtree::leaf(para_id, PlainNode::Paragraph(PlainParagraphNode {})).with_children(vec![
            Subtree::leaf(
                text_id,
                PlainNode::Text(PlainTextNode {
                    text: String::new(),
                }),
            ),
        ]),
    ])
}

fn make_empty_row(n_cols: usize) -> Subtree {
    let row_id = NodeId::new();
    Subtree::leaf(row_id, PlainNode::TableRow(PlainTableRowNode {}))
        .with_children((0..n_cols).map(|_| make_empty_cell()).collect())
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
        let doc = actual.doc;
        let table = doc.node(tbl).unwrap();
        assert_eq!(table.children().count(), 2);
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
        let doc = actual.doc;
        let table = doc.node(tbl).unwrap();
        assert_eq!(table.children().count(), 2);
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
        let doc = actual.doc;
        let table = doc.node(tbl).unwrap();
        let row = table.children().next().unwrap();
        assert_eq!(row.children().count(), 3);
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
        let doc = actual.doc;
        let table = doc.node(tbl).unwrap();
        for row in table.children() {
            assert_eq!(row.children().count(), 3);
        }
    }
}
