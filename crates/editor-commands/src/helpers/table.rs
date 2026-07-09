use editor_common::Axis;
use editor_crdt::Dot;
use editor_model::{
    Fragment, NodeView, PlainNode, PlainParagraphNode, PlainTableCellNode, PlainTableRowNode,
    Subtree,
};
use editor_state::{Selection, cell_rect_selection, enclosing_table_cell};
use editor_transaction::{Transaction, fulfill};

use crate::CommandError;

const FRACTIONAL_COLUMN_WIDTH_WEIGHT_SCALE: f64 = 10_000.0;

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

pub(crate) fn table_axis_selection(
    tr: &Transaction,
    table_id: Dot,
    axis: Option<Axis>,
    index: Option<usize>,
) -> Result<Selection, CommandError> {
    let (anchor_cell_id, head_cell_id) = match axis {
        None => {
            let view = tr.state().view();
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
            let view = tr.state().view();
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
            let view = tr.state().view();
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
    let view = tr.state().view();
    cell_rect_selection(anchor_cell_id, head_cell_id, &view)
        .ok_or_else(|| CommandError::Corrupted("cannot build cell rect selection".into()))
}

pub(crate) fn column_width_weights(widths: &[f32]) -> Vec<Option<u32>> {
    let scale = if widths
        .iter()
        .any(|width| width.is_finite() && *width > 0.0 && *width < 1.0)
    {
        FRACTIONAL_COLUMN_WIDTH_WEIGHT_SCALE
    } else {
        1.0
    };
    widths
        .iter()
        .map(|width| {
            if !width.is_finite() || *width <= 0.0 {
                None
            } else {
                Some(column_width_weight((*width as f64) * scale))
            }
        })
        .collect()
}

pub(crate) fn first_row_col_width_weights(
    tr: &Transaction,
    table_id: Dot,
) -> Result<Option<Vec<u32>>, CommandError> {
    let view = tr.state().view();
    let table = view
        .node(table_id)
        .ok_or(CommandError::NodeNotFound(table_id))?;
    let Some(first_row) = table.child_blocks().next() else {
        return Ok(None);
    };
    let mut widths = Vec::new();
    for cell in first_row.child_blocks() {
        let editor_model::Node::TableCell(cell_node) = cell.node() else {
            return Ok(None);
        };
        let Some(width) = *cell_node.col_width.get() else {
            return Ok(None);
        };
        if width == 0 {
            return Ok(None);
        }
        widths.push(width);
    }
    Ok((!widths.is_empty()).then_some(widths))
}

pub(crate) fn inserted_col_width_weights(
    existing_widths: &[u32],
    insert_index: usize,
) -> Vec<Option<u32>> {
    if existing_widths.is_empty() {
        return vec![Some(1)];
    }

    let new_col_count = existing_widths.len() + 1;
    let inserted_share = 1.0 / new_col_count as f64;
    let existing_scale = 1.0 - inserted_share;
    let existing_total: f64 = existing_widths.iter().map(|width| *width as f64).sum();
    let insert_at = insert_index.min(existing_widths.len());
    let mut next_widths: Vec<Option<u32>> = existing_widths
        .iter()
        .map(|width| Some(column_width_weight(*width as f64 * existing_scale)))
        .collect();
    next_widths.insert(
        insert_at,
        Some(column_width_weight(existing_total * inserted_share)),
    );
    next_widths
}

pub(crate) fn apply_col_width_weights_to_first_row(
    tr: &mut Transaction,
    table_id: Dot,
    widths: &[Option<u32>],
) -> Result<(), CommandError> {
    let cell_ids: Vec<Dot> = {
        let view = tr.state().view();
        let table = view
            .node(table_id)
            .ok_or(CommandError::NodeNotFound(table_id))?;
        let first_row = table
            .child_blocks()
            .next()
            .ok_or_else(|| CommandError::Corrupted("table has no rows".into()))?;
        first_row.child_blocks().map(|cell| cell.id()).collect()
    };
    if cell_ids.len() != widths.len() {
        return Ok(());
    }
    for (cell_id, width) in cell_ids.into_iter().zip(widths.iter().copied()) {
        set_table_cell_col_width(tr, cell_id, width)?;
    }
    Ok(())
}

pub(crate) fn apply_col_width_weights_to_all_rows(
    tr: &mut Transaction,
    table_id: Dot,
    widths: &[Option<u32>],
) -> Result<(), CommandError> {
    let row_ids: Vec<Dot> = {
        let view = tr.state().view();
        let table = view
            .node(table_id)
            .ok_or(CommandError::NodeNotFound(table_id))?;
        table.child_blocks().map(|row| row.id()).collect()
    };
    for row_id in row_ids {
        let cell_ids: Vec<Dot> = {
            let view = tr.state().view();
            let row = view
                .node(row_id)
                .ok_or(CommandError::NodeNotFound(row_id))?;
            row.child_blocks()
                .take(widths.len())
                .map(|cell| cell.id())
                .collect()
        };
        for (cell_id, width) in cell_ids.into_iter().zip(widths.iter().copied()) {
            set_table_cell_col_width(tr, cell_id, width)?;
        }
    }
    Ok(())
}

fn column_width_weight(width: f64) -> u32 {
    if !width.is_finite() {
        return 1;
    }
    width.round().clamp(1.0, u32::MAX as f64) as u32
}

fn set_table_cell_col_width(
    tr: &mut Transaction,
    cell_id: Dot,
    col_width: Option<u32>,
) -> Result<(), CommandError> {
    tr.set_node(
        cell_id,
        PlainNode::TableCell(PlainTableCellNode {
            col_width,
            background_color: None,
        }),
    )?;
    Ok(())
}

pub(crate) fn make_empty_table_cell(col_width: Option<u32>) -> Subtree {
    Subtree::leaf(PlainNode::TableCell(PlainTableCellNode {
        col_width,
        background_color: None,
    }))
    .with_children(vec![Subtree::leaf(PlainNode::Paragraph(
        PlainParagraphNode {},
    ))])
}

fn make_empty_table_row(n_cols: usize, col_widths: Option<&[u32]>) -> Subtree {
    Subtree::leaf(PlainNode::TableRow(PlainTableRowNode {})).with_children(
        (0..n_cols)
            .map(|idx| {
                make_empty_table_cell(col_widths.and_then(|widths| widths.get(idx)).copied())
            })
            .collect(),
    )
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
    let col_widths = first_row_col_width_weights(tr, table_id)?;
    tr.insert_subtree(
        table_id,
        index,
        make_empty_table_row(n_cols, col_widths.as_deref()),
    )?;
    Ok(())
}

/// Insert a fresh empty cell at column `index` in every row of the table.
pub(crate) fn insert_empty_table_column(
    tr: &mut Transaction,
    table_id: Dot,
    index: usize,
) -> Result<(), CommandError> {
    let next_widths = first_row_col_width_weights(tr, table_id)?
        .map(|widths| inserted_col_width_weights(&widths, index));
    let row_ids: Vec<Dot> = {
        let view = tr.state().view();
        let table = view
            .node(table_id)
            .ok_or(CommandError::NodeNotFound(table_id))?;
        table.child_blocks().map(|r| r.id()).collect()
    };
    for row_id in row_ids {
        tr.insert_subtree(row_id, index, make_empty_table_cell(None))?;
    }
    if let Some(widths) = next_widths {
        apply_col_width_weights_to_all_rows(tr, table_id, &widths)?;
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
