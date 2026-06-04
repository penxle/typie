use editor_model::{
    Fragment, NodeId, NodeRef, PlainNode, PlainParagraphNode, PlainTableCellNode,
    PlainTableRowNode, Subtree,
};
use editor_state::enclosing_table_cell;
use editor_transaction::{Transaction, fulfill};

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

pub(crate) fn table_row_count(tr: &Transaction, table_id: NodeId) -> Result<usize, CommandError> {
    let doc = tr.doc();
    let table = doc
        .node(table_id)
        .ok_or(CommandError::NodeNotFound(table_id))?;
    Ok(table.children().count())
}

pub(crate) fn table_col_count(tr: &Transaction, table_id: NodeId) -> Result<usize, CommandError> {
    let doc = tr.doc();
    let table = doc
        .node(table_id)
        .ok_or(CommandError::NodeNotFound(table_id))?;
    Ok(table
        .children()
        .next()
        .map(|r| r.children().count())
        .unwrap_or(0))
}

pub(crate) fn nth_table_cell(
    tr: &Transaction,
    table_id: NodeId,
    row: usize,
    col: usize,
) -> Result<NodeId, CommandError> {
    let doc = tr.doc();
    let table = doc
        .node(table_id)
        .ok_or(CommandError::NodeNotFound(table_id))?;
    let row_ref = table
        .children()
        .nth(row)
        .ok_or_else(|| CommandError::Corrupted(format!("row {row} missing").into()))?;
    let cell = row_ref
        .children()
        .nth(col)
        .ok_or_else(|| CommandError::Corrupted(format!("cell {row},{col} missing").into()))?;
    Ok(cell.id())
}

pub(crate) fn make_empty_table_cell() -> Subtree {
    let cell_id = NodeId::new();
    let para_id = NodeId::new();
    Subtree::leaf(
        cell_id,
        PlainNode::TableCell(PlainTableCellNode {
            col_width: None,
            background_color: None,
        }),
    )
    .with_children(vec![Subtree::leaf(
        para_id,
        PlainNode::Paragraph(PlainParagraphNode {}),
    )])
}

fn make_empty_table_row(n_cols: usize) -> Subtree {
    let row_id = NodeId::new();
    Subtree::leaf(row_id, PlainNode::TableRow(PlainTableRowNode {}))
        .with_children((0..n_cols).map(|_| make_empty_table_cell()).collect())
}

/// Insert a fresh empty row at `index` in the table. The new row gets as many
/// empty cells as the table's first row.
pub(crate) fn insert_empty_table_row(
    tr: &mut Transaction,
    table_id: NodeId,
    index: usize,
) -> Result<(), CommandError> {
    let n_cols = {
        let doc = tr.doc();
        let table = doc
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
    table_id: NodeId,
    index: usize,
) -> Result<(), CommandError> {
    let row_ids: Vec<NodeId> = {
        let doc = tr.doc();
        let table = doc
            .node(table_id)
            .ok_or(CommandError::NodeNotFound(table_id))?;
        table.children().map(|r| r.id()).collect()
    };
    for row_id in &row_ids {
        tr.insert_subtree(*row_id, index, make_empty_table_cell())?;
    }
    Ok(())
}

/// Replace a cell's children with the given block fragments and re-fulfill so
/// the cell stays schema-valid (e.g. an emptied cell regains one paragraph).
pub(crate) fn replace_cell_children(
    tr: &mut Transaction,
    cell_id: NodeId,
    blocks: &[Fragment],
) -> Result<(), CommandError> {
    let child_ids: Vec<NodeId> = tr
        .doc()
        .node(cell_id)
        .ok_or(CommandError::NodeNotFound(cell_id))?
        .children()
        .map(|c| c.id())
        .collect();
    for child_id in child_ids.into_iter().rev() {
        tr.remove_subtree(child_id)?;
    }
    for (idx, block) in blocks.iter().enumerate() {
        let subtree = block.clone().into_subtree();
        tr.insert_subtree(cell_id, idx, subtree)?;
    }
    let doc = tr.doc();
    if let Some(node) = doc.node(cell_id) {
        let steps = fulfill(&node);
        drop(doc);
        tr.apply_steps(steps)?;
    }
    Ok(())
}
