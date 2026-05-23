use editor_model::{NodeId, NodeRef};
use editor_state::enclosing_table_cell;
use editor_transaction::Transaction;

use crate::CommandError;

pub(crate) fn col_count_from_table(table: &NodeRef<'_>) -> Result<usize, CommandError> {
    let first_row = table
        .children()
        .next()
        .ok_or_else(|| CommandError::Corrupted("table has no rows".into()))?;
    Ok(first_row.children().count())
}

pub(crate) fn cursor_pos_in_table(tr: &Transaction, table_id: NodeId) -> Option<(usize, usize)> {
    let doc = tr.doc();
    let cell_id = enclosing_table_cell(&doc, tr.selection()?.head.node_id)?;
    let table = doc.node(table_id)?;
    for (row_idx, row) in table.children().enumerate() {
        for (col_idx, cell) in row.children().enumerate() {
            if cell.id() == cell_id {
                return Some((row_idx, col_idx));
            }
        }
    }
    None
}
