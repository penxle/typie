use editor_crdt::Dot;
use editor_model::{
    Fragment, NodeView, PlainNode, PlainParagraphNode, PlainTableCellNode, PlainTableRowNode,
    Subtree,
};
use editor_state::enclosing_table_cell;
use editor_transaction::{Transaction, fulfill};

use crate::CommandError;

pub(crate) fn col_count_from_table(table: &NodeView<'_>) -> Result<usize, CommandError> {
    let first_row = table
        .child_blocks()
        .next()
        .ok_or_else(|| CommandError::Corrupted("table has no rows".into()))?;
    Ok(first_row.child_blocks().count())
}

pub(crate) fn cursor_pos_in_table(tr: &Transaction, table_id: Dot) -> Option<(usize, usize)> {
    let view = tr.state().view();
    let cell_id = enclosing_table_cell(&view, tr.selection()?.head.node)?;
    let table = view.node(table_id)?;
    for (row_idx, row) in table.child_blocks().enumerate() {
        for (col_idx, cell) in row.child_blocks().enumerate() {
            if cell.id() == cell_id {
                return Some((row_idx, col_idx));
            }
        }
    }
    None
}

pub(crate) fn table_row_count(tr: &Transaction, table_id: Dot) -> Result<usize, CommandError> {
    let view = tr.state().view();
    let table = view
        .node(table_id)
        .ok_or(CommandError::NodeNotFound(table_id))?;
    Ok(table.child_blocks().count())
}

pub(crate) fn table_col_count(tr: &Transaction, table_id: Dot) -> Result<usize, CommandError> {
    let view = tr.state().view();
    let table = view
        .node(table_id)
        .ok_or(CommandError::NodeNotFound(table_id))?;
    Ok(table
        .child_blocks()
        .next()
        .map(|r| r.child_blocks().count())
        .unwrap_or(0))
}

pub(crate) fn nth_table_cell(
    tr: &Transaction,
    table_id: Dot,
    row: usize,
    col: usize,
) -> Result<Dot, CommandError> {
    let view = tr.state().view();
    let table = view
        .node(table_id)
        .ok_or(CommandError::NodeNotFound(table_id))?;
    let row_ref = table
        .child_blocks()
        .nth(row)
        .ok_or_else(|| CommandError::Corrupted(format!("row {row} missing")))?;
    let cell = row_ref
        .child_blocks()
        .nth(col)
        .ok_or_else(|| CommandError::Corrupted(format!("cell {row},{col} missing")))?;
    Ok(cell.id())
}

pub(crate) fn make_empty_table_cell() -> Subtree {
    Subtree::leaf(PlainNode::TableCell(PlainTableCellNode {
        col_width: None,
        background_color: None,
    }))
    .with_children(vec![Subtree::leaf(PlainNode::Paragraph(
        PlainParagraphNode {},
    ))])
}

fn make_empty_table_row(n_cols: usize) -> Subtree {
    Subtree::leaf(PlainNode::TableRow(PlainTableRowNode {}))
        .with_children((0..n_cols).map(|_| make_empty_table_cell()).collect())
}

/// Insert a fresh empty row at `index` in the table. The new row gets as many
/// empty cells as the table's first row.
pub(crate) fn insert_empty_table_row(
    tr: &mut Transaction,
    table_id: Dot,
    index: usize,
) -> Result<(), CommandError> {
    let n_cols = {
        let view = tr.state().view();
        let table = view
            .node(table_id)
            .ok_or(CommandError::NodeNotFound(table_id))?;
        col_count_from_table(&table)?
    };
    tr.insert_subtree(table_id, index, make_empty_table_row(n_cols))?;
    Ok(())
}

/// Insert a fresh empty cell at column `index` in every row of the table.
pub(crate) fn insert_empty_table_column(
    tr: &mut Transaction,
    table_id: Dot,
    index: usize,
) -> Result<(), CommandError> {
    let row_ids: Vec<Dot> = {
        let view = tr.state().view();
        let table = view
            .node(table_id)
            .ok_or(CommandError::NodeNotFound(table_id))?;
        table.child_blocks().map(|r| r.id()).collect()
    };
    for row_id in row_ids {
        tr.insert_subtree(row_id, index, make_empty_table_cell())?;
    }
    Ok(())
}

/// Replace a cell's children with the given block fragments and re-fulfill so
/// the cell stays schema-valid (e.g. an emptied cell regains one paragraph).
pub(crate) fn replace_cell_children(
    tr: &mut Transaction,
    cell_id: Dot,
    blocks: &[Fragment],
) -> Result<(), CommandError> {
    let child_ids: Vec<Dot> = {
        let view = tr.state().view();
        view.node(cell_id)
            .ok_or(CommandError::NodeNotFound(cell_id))?
            .child_blocks()
            .map(|c| c.id())
            .collect()
    };
    for child_id in child_ids.into_iter().rev() {
        tr.remove_subtree(child_id)?;
    }
    for (idx, block) in blocks.iter().enumerate() {
        let subtree = block.clone().into_subtree();
        tr.insert_subtree(cell_id, idx, subtree)?;
    }
    let steps = {
        let view = tr.state().view();
        view.node(cell_id)
            .map(|node| fulfill(&node))
            .unwrap_or_default()
    };
    tr.apply_steps(steps)?;
    Ok(())
}
