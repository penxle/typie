use editor_common::Axis;
use editor_model::NodeId;
use editor_state::cell_rect_selection;
use editor_transaction::Transaction;

use crate::helpers::cursor_pos_in_table;
use crate::{CommandError, CommandResult};

pub fn select_table_axis(
    tr: &mut Transaction,
    table_id: NodeId,
    axis: Option<Axis>,
) -> CommandResult {
    let (anchor_cell_id, head_cell_id) = match axis {
        None => {
            let doc = tr.doc();
            let table = doc
                .node(table_id)
                .ok_or(CommandError::NodeNotFound(table_id))?;
            let first_row = table
                .children()
                .next()
                .ok_or_else(|| CommandError::Corrupted("table has no rows".into()))?;
            let anchor = first_row
                .children()
                .next()
                .ok_or_else(|| CommandError::Corrupted("row has no cells".into()))?
                .id();
            let last_row = table
                .children()
                .last()
                .ok_or_else(|| CommandError::Corrupted("table has no rows".into()))?;
            let head = last_row
                .children()
                .last()
                .ok_or_else(|| CommandError::Corrupted("row has no cells".into()))?
                .id();
            (anchor, head)
        }
        Some(Axis::Horizontal) => {
            let (row_idx, _) = cursor_pos_in_table(tr, table_id).unwrap_or((0, 0));
            let doc = tr.doc();
            let table = doc
                .node(table_id)
                .ok_or(CommandError::NodeNotFound(table_id))?;
            let row = table
                .children()
                .nth(row_idx)
                .ok_or_else(|| CommandError::Corrupted("row index out of range".into()))?;
            let anchor = row
                .children()
                .next()
                .ok_or_else(|| CommandError::Corrupted("row has no cells".into()))?
                .id();
            let head = row
                .children()
                .last()
                .ok_or_else(|| CommandError::Corrupted("row has no cells".into()))?
                .id();
            (anchor, head)
        }
        Some(Axis::Vertical) => {
            let (_, col_idx) = cursor_pos_in_table(tr, table_id).unwrap_or((0, 0));
            let doc = tr.doc();
            let table = doc
                .node(table_id)
                .ok_or(CommandError::NodeNotFound(table_id))?;
            let first_row = table
                .children()
                .next()
                .ok_or_else(|| CommandError::Corrupted("table has no rows".into()))?;
            let anchor = first_row
                .children()
                .nth(col_idx)
                .ok_or_else(|| CommandError::Corrupted("col index out of range".into()))?
                .id();
            let last_row = table
                .children()
                .last()
                .ok_or_else(|| CommandError::Corrupted("table has no rows".into()))?;
            let head = last_row
                .children()
                .nth(col_idx)
                .ok_or_else(|| CommandError::Corrupted("col index out of range".into()))?
                .id();
            (anchor, head)
        }
    };

    let doc = tr.doc();
    let selection = cell_rect_selection(&doc, anchor_cell_id, head_cell_id)
        .ok_or_else(|| CommandError::Corrupted("cannot build cell rect selection".into()))?;
    tr.set_selection(selection)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::CellRect;

    use super::*;
    use crate::test_utils::*;

    fn as_cell_rect(state: &editor_state::State) -> CellRect<'_> {
        state
            .selection
            .resolve(&state.doc)
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
        let (actual, ..) = transact!(initial, |tr| select_table_axis(&mut tr, tbl, None));
        let rect = as_cell_rect(&actual);
        assert!(rect.is_full_table());
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
            Some(Axis::Horizontal)
        ));
        let rect = as_cell_rect(&actual);
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
            Some(Axis::Vertical)
        ));
        let rect = as_cell_rect(&actual);
        assert!(rect.is_full_column());
    }
}
