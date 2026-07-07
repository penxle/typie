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
    fn move_row_preserves_cell_bold() {
        use editor_model::Modifier;
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row { c0: table_cell { paragraph { text("A") [bold] } } }
                    table_row { table_cell { paragraph { text("B") } } }
                }
            } }
            selection: (c0, 0)
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
        let rows: Vec<_> = table.child_blocks().collect();
        let cell = rows[1].child_blocks().next().unwrap();
        let para = cell.child_blocks().next().unwrap();
        assert_eq!(para.inline_text(), "A");
        assert!(
            para.leaf_own_modifiers_at(0).contains(&Modifier::Bold),
            "the moved row's cell keeps its inline bold char paint"
        );
    }

    #[test]
    fn move_row_preserves_cell_background() {
        use editor_model::{Modifier, ModifierType};
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        c0: table_cell [background_color("#ffff00".to_string())] { paragraph { text("A") } }
                    }
                    table_row { table_cell { paragraph { text("B") } } }
                }
            } }
            selection: (c0, 0)
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
        let rows: Vec<_> = table.child_blocks().collect();
        let cell = rows[1].child_blocks().next().unwrap();
        assert_eq!(
            cell.block_modifier(ModifierType::BackgroundColor),
            Some(&Modifier::BackgroundColor {
                value: "#ffff00".to_string()
            }),
            "the moved row's cell keeps its background color"
        );
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

    #[test]
    fn move_col_preserves_col_width() {
        let (initial, tbl, ..) = state! {
            doc { root {
                tbl: table {
                    table_row {
                        r0c0: table_cell(col_width: Some(300u32)) { paragraph { text("A") } }
                        table_cell(col_width: Some(100u32)) { paragraph { text("B") } }
                        table_cell(col_width: Some(200u32)) { paragraph { text("C") } }
                    }
                    table_row {
                        table_cell(col_width: Some(300u32)) { paragraph { text("D") } }
                        table_cell(col_width: Some(100u32)) { paragraph { text("E") } }
                        table_cell(col_width: Some(200u32)) { paragraph { text("F") } }
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
        let rows: Vec<_> = table.child_blocks().collect();
        let cols: Vec<(String, Option<u32>)> = rows[0]
            .child_blocks()
            .map(|cell| {
                let text = cell
                    .child_blocks()
                    .next()
                    .map(|p| p.inline_text())
                    .unwrap_or_default();
                let width = match cell.node() {
                    editor_model::Node::TableCell(n) => *n.col_width.get(),
                    _ => panic!("expected a table cell"),
                };
                (text, width)
            })
            .collect();
        assert_eq!(
            cols,
            vec![
                ("B".to_string(), Some(100)),
                ("C".to_string(), Some(200)),
                ("A".to_string(), Some(300)),
            ],
            "the moved column carries its col_width to the new position"
        );
    }
}
