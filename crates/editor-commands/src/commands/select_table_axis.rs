use editor_common::Axis;
use editor_crdt::Dot;
use editor_state::cell_rect_selection;
use editor_transaction::Transaction;

use crate::helpers::cursor_pos_in_table;
use crate::{CommandError, CommandResult};

pub fn select_table_axis(
    tr: &mut Transaction,
    table_id: Dot,
    axis: Option<Axis>,
    index: Option<usize>,
) -> CommandResult {
    let (anchor_cell_id, head_cell_id) = match axis {
        None => {
            let view = tr.view();
            let table = view
                .node(table_id)
                .ok_or(CommandError::NodeNotFound(table_id))?;
            let first_row = table
                .child_blocks()
                .next()
                .ok_or_else(|| CommandError::Corrupted("table has no rows".into()))?;
            let anchor = first_row
                .child_blocks()
                .next()
                .ok_or_else(|| CommandError::Corrupted("row has no cells".into()))?
                .id();
            let last_row = table
                .child_blocks()
                .last()
                .ok_or_else(|| CommandError::Corrupted("table has no rows".into()))?;
            let head = last_row
                .child_blocks()
                .last()
                .ok_or_else(|| CommandError::Corrupted("row has no cells".into()))?
                .id();
            (anchor, head)
        }
        Some(Axis::Horizontal) => {
            let (cursor_row, _) = cursor_pos_in_table(tr, table_id).unwrap_or((0, 0));
            let row_idx = index.unwrap_or(cursor_row);
            let view = tr.view();
            let table = view
                .node(table_id)
                .ok_or(CommandError::NodeNotFound(table_id))?;
            let row = table
                .child_blocks()
                .nth(row_idx)
                .ok_or_else(|| CommandError::Corrupted("row index out of range".into()))?;
            let anchor = row
                .child_blocks()
                .next()
                .ok_or_else(|| CommandError::Corrupted("row has no cells".into()))?
                .id();
            let head = row
                .child_blocks()
                .last()
                .ok_or_else(|| CommandError::Corrupted("row has no cells".into()))?
                .id();
            (anchor, head)
        }
        Some(Axis::Vertical) => {
            let (_, cursor_col) = cursor_pos_in_table(tr, table_id).unwrap_or((0, 0));
            let col_idx = index.unwrap_or(cursor_col);
            let view = tr.view();
            let table = view
                .node(table_id)
                .ok_or(CommandError::NodeNotFound(table_id))?;
            let first_row = table
                .child_blocks()
                .next()
                .ok_or_else(|| CommandError::Corrupted("table has no rows".into()))?;
            let anchor = first_row
                .child_blocks()
                .nth(col_idx)
                .ok_or_else(|| CommandError::Corrupted("col index out of range".into()))?
                .id();
            let last_row = table
                .child_blocks()
                .last()
                .ok_or_else(|| CommandError::Corrupted("table has no rows".into()))?;
            let head = last_row
                .child_blocks()
                .nth(col_idx)
                .ok_or_else(|| CommandError::Corrupted("col index out of range".into()))?
                .id();
            (anchor, head)
        }
    };

    let selection = {
        let view = tr.view();
        cell_rect_selection(anchor_cell_id, head_cell_id, &view)
            .ok_or_else(|| CommandError::Corrupted("cannot build cell rect selection".into()))?
    };
    tr.set_selection(Some(selection))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::CellRect;
    use editor_state::{Affinity, Position, Selection};

    use super::*;
    use crate::test_utils::*;

    fn as_cell_rect<'a>(view: &'a editor_model::DocView<'a>, sel: &Selection) -> CellRect<'a> {
        sel.resolve(view)
            .unwrap()
            .as_cell_rect()
            .expect("expected CellRect selection")
    }

    #[test]
    fn select_full_table() {
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
        let (actual, ..) = transact!(initial, |tr| select_table_axis(&mut tr, tbl, None, None));
        let root_id = actual.view().root().unwrap().id();
        assert_eq!(
            actual.selection,
            Some(Selection::new(
                Position {
                    node: root_id,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node: root_id,
                    offset: 1,
                    affinity: Affinity::Upstream,
                },
            ))
        );
    }

    #[test]
    fn select_row() {
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
        let (actual, ..) = transact!(initial, |tr| select_table_axis(
            &mut tr,
            tbl,
            Some(Axis::Horizontal),
            None,
        ));
        let view = actual.view();
        let rect = as_cell_rect(&view, actual.selection.as_ref().unwrap());
        assert!(rect.is_full_row());
    }

    #[test]
    fn select_col() {
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
        let (actual, ..) = transact!(initial, |tr| select_table_axis(
            &mut tr,
            tbl,
            Some(Axis::Vertical),
            None,
        ));
        let view = actual.view();
        let rect = as_cell_rect(&view, actual.selection.as_ref().unwrap());
        assert!(rect.is_full_column());
    }
}
