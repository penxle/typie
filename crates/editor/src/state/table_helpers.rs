use crate::model::{Doc, NodeId, NodeType};
use crate::state::Selection;
use std::cmp::{max, min};

pub type TableCellRange = ((usize, usize), (usize, usize));

pub fn compute_table_selection(
    doc: &Doc,
    selection: &Selection,
) -> Option<(NodeId, TableCellRange)> {
    let anchor_info = find_table_cell(doc, selection.anchor.node_id);
    let head_info = find_table_cell(doc, selection.head.node_id);

    let (Some((_, t1, r1, c1)), Some((_, t2, r2, c2))) = (anchor_info, head_info) else {
        return None;
    };

    if t1 != t2 {
        return None;
    }

    if r1 == r2 && c1 == c2 {
        None
    } else {
        let start_row = min(r1, r2);
        let end_row = max(r1, r2);
        let start_col = min(c1, c2);
        let end_col = max(c1, c2);

        Some((t1, ((start_row, end_row), (start_col, end_col))))
    }
}

pub fn collect_cells_in_range(doc: &Doc, table_id: NodeId, range: TableCellRange) -> Vec<NodeId> {
    let mut cells = Vec::new();
    if let Some(table) = doc.node(table_id) {
        for (r_idx, row) in table.children().enumerate() {
            if r_idx < range.0.0 || r_idx > range.0.1 {
                continue;
            }
            for (c_idx, cell) in row.children().enumerate() {
                if c_idx < range.1.0 || c_idx > range.1.1 {
                    continue;
                }
                cells.push(cell.node_id());
            }
        }
    }
    cells
}

pub fn find_table_cell(doc: &Doc, node_id: NodeId) -> Option<(NodeId, NodeId, usize, usize)> {
    let mut current_id = node_id;

    loop {
        let Some(node) = doc.node(current_id) else {
            break;
        };

        if node.node_type() == Some(NodeType::TableCell) {
            let cell = node;
            let row = cell.parent()?;
            if row.node_type() != Some(NodeType::TableRow) {
                return None;
            }
            let table = row.parent()?;
            if table.node_type() != Some(NodeType::Table) {
                return None;
            }

            let row_idx = row.index()?;
            let col_idx = cell.index()?;

            return Some((cell.node_id(), table.node_id(), row_idx, col_idx));
        }

        if node.node_type() == Some(NodeType::Table) {
            break;
        }

        if let Some(parent) = node.parent() {
            current_id = parent.node_id();
        } else {
            break;
        }
    }
    None
}
