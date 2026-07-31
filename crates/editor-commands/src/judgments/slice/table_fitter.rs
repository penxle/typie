//! Pure planning for two-dimensional table Slice semantics.

use editor_clipboard::Slice;
use editor_model::{DocView, Fragment, NodeType, PlainNode};
use editor_state::{Position, Selection, enclosing_table_cell};

use super::fit_fragment_forest;
use crate::CommandError;
use crate::helpers::{PlannedEndpoint, node_type_path};

pub(crate) struct TableGridPlan {
    pub(crate) source_rows: Vec<Vec<Fragment>>,
    pub(crate) table: PlannedEndpoint,
    pub(crate) target_cells: Vec<Vec<PlannedEndpoint>>,
    pub(crate) anchor_row: usize,
    pub(crate) anchor_col: usize,
    pub(crate) original_rows: usize,
    pub(crate) original_cols: usize,
    pub(crate) append_rows: usize,
    pub(crate) append_cols: usize,
    pub(crate) final_selection: TableFinalSelection,
}

pub(crate) struct CellFillPlan {
    pub(crate) cells: Vec<PlannedEndpoint>,
    pub(crate) original_child_types: Vec<Vec<NodeType>>,
    pub(crate) blocks: Vec<Fragment>,
    pub(crate) final_selection: TableFinalSelection,
}

#[derive(Clone, Copy)]
pub(crate) enum TableFinalSelection {
    /// Resolve after cell replacement so newly authored paragraph ids need not
    /// be guessed during planning.
    FirstTextInAnchorCell,
}

pub(super) fn selection_is_wholly_in_one_cell(view: &DocView, selection: Selection) -> bool {
    let Some(resolved) = selection.resolve(view) else {
        return false;
    };
    let anchor = enclosing_table_cell(view, resolved.anchor().node());
    let head = enclosing_table_cell(view, resolved.head().node());
    anchor.is_some() && anchor == head
}

pub(super) fn fit_table_grid(
    view: &DocView,
    selection: Selection,
    slice: &Slice,
) -> Result<Option<TableGridPlan>, CommandError> {
    let Some(source_rows) = source_grid(slice) else {
        return Ok(None);
    };
    let source_row_count = source_rows.len();
    let source_col_count = source_rows.first().map(Vec::len).unwrap_or(0);
    if source_row_count == 0 || source_col_count == 0 {
        return Ok(None);
    }
    let Some(resolved) = selection.resolve(view) else {
        return Ok(None);
    };
    let (table_id, anchor_row, anchor_col) = if let Some(rect) = resolved.as_cell_rect() {
        if rect.cells().is_empty() {
            return Err(CommandError::Corrupted(
                "table-grid selection has no anchor cell".into(),
            ));
        }
        (rect.table.id(), *rect.rows.start(), *rect.cols.start())
    } else {
        let Some(cell_id) = enclosing_table_cell(view, resolved.head().node()) else {
            return Ok(None);
        };
        if enclosing_table_cell(view, resolved.anchor().node()) != Some(cell_id) {
            return Ok(None);
        }
        let cell = view
            .node(cell_id)
            .ok_or(CommandError::NodeNotFound(cell_id))?;
        let row = cell.parent().ok_or(CommandError::NoParent(cell_id))?;
        let table = row.parent().ok_or(CommandError::NoParent(row.id()))?;
        let row_index = row
            .index()
            .ok_or_else(|| CommandError::orphan_child(row.id(), table.id()))?;
        let col_index = cell
            .index()
            .ok_or_else(|| CommandError::orphan_child(cell_id, row.id()))?;
        (table.id(), row_index, col_index)
    };

    let table = PlannedEndpoint::capture(view, Position::new(table_id, 0))
        .ok_or_else(|| CommandError::Corrupted("cannot capture table-grid Slice target".into()))?;
    let table_node = view
        .node(table_id)
        .ok_or(CommandError::NodeNotFound(table_id))?;
    let original_rows = table_node.child_blocks().count();
    let original_cols = table_node
        .child_blocks()
        .next()
        .map(|row| row.child_blocks().count())
        .unwrap_or(0);
    if original_rows == 0 || original_cols == 0 {
        return Ok(None);
    }
    let target_row_count = source_row_count.min(original_rows.saturating_sub(anchor_row));
    let target_col_count = source_col_count.min(original_cols.saturating_sub(anchor_col));
    let target_cells = (0..target_row_count)
        .map(|row_offset| {
            (0..target_col_count)
                .map(|col_offset| {
                    let cell_id = table_node
                        .child_blocks()
                        .nth(anchor_row + row_offset)
                        .and_then(|row| row.child_blocks().nth(anchor_col + col_offset))
                        .map(|cell| cell.id())
                        .ok_or_else(|| {
                            CommandError::Corrupted("cannot capture table-grid target cell".into())
                        })?;
                    PlannedEndpoint::capture(view, Position::new(cell_id, 0)).ok_or_else(|| {
                        CommandError::Corrupted("cannot capture table-grid target cell slot".into())
                    })
                })
                .collect::<Result<Vec<_>, CommandError>>()
        })
        .collect::<Result<Vec<_>, CommandError>>()?;
    if target_cells.first().and_then(|row| row.first()).is_none() {
        return Err(CommandError::Corrupted(
            "table-grid plan has no target anchor slot".into(),
        ));
    }
    let append_rows = (anchor_row + source_row_count).saturating_sub(original_rows);
    let append_cols = (anchor_col + source_col_count).saturating_sub(original_cols);
    Ok(Some(TableGridPlan {
        source_rows,
        table,
        target_cells,
        anchor_row,
        anchor_col,
        original_rows,
        original_cols,
        append_rows,
        append_cols,
        final_selection: TableFinalSelection::FirstTextInAnchorCell,
    }))
}

pub(super) fn is_pure_table_grid_slice(slice: &Slice) -> bool {
    slice.open_start == 0
        && slice.open_end == 0
        && slice.content.len() == 1
        && slice
            .content
            .first()
            .is_some_and(|table| rectangular_table_width(table).is_some())
}

pub(super) fn fit_cell_fill(
    view: &DocView,
    selection: Selection,
    slice: &Slice,
) -> Option<CellFillPlan> {
    let resolved = selection.resolve(view)?;
    let rect = resolved.as_cell_rect()?;
    let cell_ids: Vec<_> = rect.cells().iter().map(|cell| cell.id()).collect();
    let anchor_cell_id = *cell_ids.first()?;
    let cell_path = node_type_path(view, anchor_cell_id)?;
    let blocks = slice_to_cell_blocks(slice);
    if blocks.is_empty() {
        return None;
    }
    let blocks = fit_fragment_forest(blocks, &cell_path)?;
    let original_child_types = cell_ids
        .iter()
        .map(|cell_id| {
            view.node(*cell_id).map(|cell| {
                cell.children()
                    .map(|child| match child {
                        editor_model::ChildView::Block(block) => block.node_type(),
                        editor_model::ChildView::Leaf(leaf) => leaf.node_type(),
                    })
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let cells = cell_ids
        .iter()
        .map(|cell_id| PlannedEndpoint::capture(view, Position::new(*cell_id, 0)))
        .collect::<Option<Vec<_>>>()?;
    Some(CellFillPlan {
        cells,
        original_child_types,
        blocks,
        final_selection: TableFinalSelection::FirstTextInAnchorCell,
    })
}

fn source_grid(slice: &Slice) -> Option<Vec<Vec<Fragment>>> {
    if slice.open_start != 0 || slice.open_end != 0 || slice.content.len() != 1 {
        return None;
    }
    rectangular_table_width(slice.content.first()?)?;
    let mut fitted = fit_fragment_forest(slice.content.clone(), &[NodeType::Root])?;
    if fitted.len() != 1 {
        return None;
    }
    let table = fitted.pop()?;
    rectangular_table_width(&table)?;
    let (_, _, _, rows) = table.into_parts();
    Some(
        rows.into_iter()
            .map(|row| {
                let (_, _, _, cells) = row.into_parts();
                cells
            })
            .collect(),
    )
}

fn rectangular_table_width(table: &Fragment) -> Option<usize> {
    if !matches!(table.node, PlainNode::Table(_)) || table.children.is_empty() {
        return None;
    }
    let mut width = None;
    for row in &table.children {
        if !matches!(row.node, PlainNode::TableRow(_))
            || row.children.is_empty()
            || row
                .children
                .iter()
                .any(|cell| !matches!(cell.node, PlainNode::TableCell(_)))
        {
            return None;
        }
        match width {
            Some(width) if width != row.children.len() => return None,
            None => width = Some(row.children.len()),
            _ => {}
        }
    }
    width
}

fn slice_to_cell_blocks(slice: &Slice) -> Vec<Fragment> {
    let mut out = Vec::new();
    let mut inline_run = Vec::new();
    let flush = |run: &mut Vec<Fragment>, out: &mut Vec<Fragment>| {
        if !run.is_empty() {
            out.push(Fragment {
                node: PlainNode::Paragraph(editor_model::PlainParagraphNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: std::mem::take(run),
            });
        }
    };
    for child in &slice.content {
        match &child.node {
            PlainNode::Text(_) | PlainNode::HardBreak(_) | PlainNode::Tab(_) => {
                inline_run.push(child.clone());
            }
            _ => {
                flush(&mut inline_run, &mut out);
                out.push(child.clone());
            }
        }
    }
    flush(&mut inline_run, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orphan_table_row_is_not_a_pure_table_grid_slice() {
        let row = Fragment::leaf(PlainNode::TableRow(
            editor_model::PlainTableRowNode::default(),
        ))
        .with_children(vec![Fragment::leaf(PlainNode::TableCell(
            editor_model::PlainTableCellNode::default(),
        ))]);
        let slice = Slice::new(vec![row], 0, 0);

        assert!(!is_pure_table_grid_slice(&slice));
        assert!(source_grid(&slice).is_none());
    }
}
