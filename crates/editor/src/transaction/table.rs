use crate::model::{
    Node, NodeId, NodeType, ParagraphNode, TableAlign, TableBorderStyle, TableCellNode, TableNode,
    TableRowNode, TableWidthModel,
};
use crate::runtime::Effect;
use crate::state::selection_helpers::StructureSelectionInfo;
use crate::state::table_helpers::find_table_cell;
use crate::state::{Position, Selection, leaf_block_end, leaf_block_start};
use crate::transaction::Transaction;
use crate::types::Affinity;
use anyhow::{Context, Result};

impl Transaction {
    pub fn insert_table(&mut self, rows: u32, cols: u32) -> Result<Option<NodeId>> {
        let selection = self.selection().clone();
        let pos = selection.anchor;

        let Some(parent_node) = self.node(pos.node_id) else {
            return Ok(None);
        };

        let Some(parent) = parent_node.parent() else {
            return Ok(None);
        };

        let parent_id = parent.node_id();
        let parent_spec = parent.spec().context("Parent spec not found")?;

        let table_type = NodeType::Table;
        if !parent_spec.content.matches(table_type) {
            return Ok(None);
        }

        let block_index = parent_node.index().context("Block has no index")?;

        let table_id = NodeId::new();
        let mut row_ids = Vec::new();
        let mut first_cell_id = None;

        let parent_mut = self.node_mut(parent_id).context("Parent not found")?;
        parent_mut.as_mut().insert_child_with_id(
            block_index + 1,
            table_id,
            Node::Table(TableNode::default()),
        )?;

        for row_idx in 0..rows {
            let row_id = NodeId::new();
            row_ids.push(row_id);

            let table_mut = self.node_mut(table_id).context("Table not found")?;
            table_mut.as_mut().insert_child_with_id(
                row_idx as usize,
                row_id,
                Node::TableRow(TableRowNode::default()),
            )?;

            for col_idx in 0..cols {
                let cell_id = NodeId::new();
                let para_id = NodeId::new();

                if first_cell_id.is_none() {
                    first_cell_id = Some(para_id);
                }

                let row_mut = self.node_mut(row_id).context("Row not found")?;
                row_mut.as_mut().insert_child_with_id(
                    col_idx as usize,
                    cell_id,
                    Node::TableCell(TableCellNode::default()),
                )?;

                let cell_mut = self.node_mut(cell_id).context("Cell not found")?;
                cell_mut.as_mut().insert_child_with_id(
                    0,
                    para_id,
                    Node::Paragraph(ParagraphNode::default()),
                )?;
            }
        }

        if let Some(para_id) = first_cell_id {
            self.set_selection(Selection::collapsed(Position::new(
                para_id,
                0,
                Affinity::Downstream,
            )));
        }

        self.mark_structure_mutation(table_id);

        Ok(Some(table_id))
    }

    pub fn set_column_widths(&mut self, table_id: NodeId, col_widths: Vec<f32>) -> Result<bool> {
        let table_node = self.node(table_id).context("Table not found")?;

        let first_row = table_node.children().next();
        let Some(first_row) = first_row else {
            return Ok(false);
        };

        let first_row_cells: Vec<_> = first_row.children().map(|c| c.node_id()).collect();

        if first_row_cells.len() != col_widths.len() {
            return Ok(false);
        }

        let Some(valid_widths) =
            TableWidthModel::validate_ratio_widths(&col_widths, first_row_cells.len())
        else {
            return Ok(false);
        };

        for (cell_id, width) in first_row_cells.iter().zip(valid_widths.into_iter()) {
            let cell_mut = self.node_mut(*cell_id).context("Cell not found")?;

            cell_mut.as_mut().update(|node| {
                if let Node::TableCell(cell_node) = node {
                    cell_node.col_width = Some(width);
                }
            })?;
        }

        self.mark_structure_mutation(table_id);
        self.push_effect(Effect::LayoutChanged);

        Ok(true)
    }

    pub fn add_table_row(&mut self, table_id: NodeId, row: usize, before: bool) -> Result<bool> {
        let table_node = self.node(table_id).context("Table not found")?;

        let first_row = table_node.children().next();
        let col_count = first_row.map(|r| r.children().count()).unwrap_or(0);
        if col_count == 0 {
            return Ok(false);
        }

        let row_count = table_node.children().count();
        let insert_index = if before {
            row.min(row_count)
        } else {
            row.saturating_add(1).min(row_count)
        };

        let row_id = NodeId::new();
        let table_mut = self.node_mut(table_id).context("Table not found")?;
        table_mut.as_mut().insert_child_with_id(
            insert_index,
            row_id,
            Node::TableRow(TableRowNode::default()),
        )?;

        for col_idx in 0..col_count {
            let cell_id = NodeId::new();
            let para_id = NodeId::new();

            let row_mut = self.node_mut(row_id).context("Row not found")?;
            row_mut.as_mut().insert_child_with_id(
                col_idx,
                cell_id,
                Node::TableCell(TableCellNode::default()),
            )?;

            let cell_mut = self.node_mut(cell_id).context("Cell not found")?;
            cell_mut.as_mut().insert_child_with_id(
                0,
                para_id,
                Node::Paragraph(ParagraphNode::default()),
            )?;
        }

        self.mark_structure_mutation(table_id);
        self.push_effect(Effect::LayoutChanged);

        Ok(true)
    }

    pub fn add_table_column(&mut self, table_id: NodeId, col: usize, before: bool) -> Result<bool> {
        let table_node = self.node(table_id).context("Table not found")?;

        let row_ids: Vec<_> = table_node.children().map(|r| r.node_id()).collect();
        if row_ids.is_empty() {
            return Ok(false);
        }

        let first_row = self.node(row_ids[0]).context("First row not found")?;
        let col_count = first_row.children().count();
        let existing_first_row_widths: Option<Vec<f32>> = {
            let widths: Vec<Option<f32>> = first_row
                .children()
                .map(|cell| {
                    if let Some(Node::TableCell(cell_node)) = cell.node() {
                        cell_node.col_width
                    } else {
                        None
                    }
                })
                .collect();
            if widths.iter().all(|width| width.is_some()) {
                Some(widths.into_iter().map(|width| width.unwrap()).collect())
            } else {
                None
            }
        };

        let insert_index = if before {
            col.min(col_count)
        } else {
            col.saturating_add(1).min(col_count)
        };

        for row_id in &row_ids {
            let cell_id = NodeId::new();
            let para_id = NodeId::new();

            let row_mut = self.node_mut(*row_id).context("Row not found")?;
            row_mut.as_mut().insert_child_with_id(
                insert_index,
                cell_id,
                Node::TableCell(TableCellNode::default()),
            )?;

            let cell_mut = self.node_mut(cell_id).context("Cell not found")?;
            cell_mut.as_mut().insert_child_with_id(
                0,
                para_id,
                Node::Paragraph(ParagraphNode::default()),
            )?;
        }

        if let Some(existing_widths) = existing_first_row_widths {
            let next_widths =
                TableWidthModel::inserted_ratio_widths(&existing_widths, insert_index);

            let first_row_cell_ids: Vec<_> = {
                let first_row = self.node(row_ids[0]).context("First row not found")?;
                first_row.children().map(|cell| cell.node_id()).collect()
            };

            for (cell_id, width) in first_row_cell_ids.into_iter().zip(next_widths.into_iter()) {
                let cell_mut = self.node_mut(cell_id).context("Cell not found")?;
                cell_mut.as_mut().update(|node| {
                    if let Node::TableCell(cell_node) = node {
                        cell_node.col_width = Some(width);
                    }
                })?;
            }
        }

        self.mark_structure_mutation(table_id);
        self.push_effect(Effect::LayoutChanged);

        Ok(true)
    }

    pub fn delete_table_row(&mut self, table_id: NodeId, row: usize) -> Result<bool> {
        let table_node = self.node(table_id).context("Table not found")?;

        let row_count = table_node.children().count();
        if row >= row_count || row_count <= 1 {
            return Ok(false);
        }

        let row_child = table_node.children().nth(row);
        let Some(row_child) = row_child else {
            return Ok(false);
        };
        let row_id = row_child.node_id();

        let selection = self.selection();
        let is_selected = {
            let doc = self.doc();
            let check = |node_id| {
                if let Some((_, t_id, r_idx, _)) = find_table_cell(doc, node_id) {
                    t_id == table_id && r_idx == row
                } else {
                    false
                }
            };
            check(selection.anchor.node_id) || check(selection.head.node_id)
        };

        if is_selected {
            let target_row_idx = if row + 1 < row_count {
                row + 1
            } else if row > 0 {
                row - 1
            } else {
                unreachable!("row_count <= 1 should be handled early return")
            };

            let target_pos = {
                let table_node = self.node(table_id).context("Table not found")?;
                if let Some(target_row) = table_node.children().nth(target_row_idx) {
                    if let Some(first_cell) = target_row.first_child() {
                        leaf_block_start(&first_cell)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some(pos) = target_pos {
                self.set_selection(Selection::collapsed(pos));
            }
        }

        self.delete_node_recursive(row_id)?;

        self.mark_structure_mutation(table_id);

        Ok(true)
    }

    pub fn delete_table_column(&mut self, table_id: NodeId, col: usize) -> Result<bool> {
        let table_node = self.node(table_id).context("Table not found")?;

        let row_ids: Vec<_> = table_node.children().map(|r| r.node_id()).collect();
        if row_ids.is_empty() {
            return Ok(false);
        }

        let first_row = self.node(row_ids[0]).context("First row not found")?;
        let col_count = first_row.children().count();
        if col >= col_count || col_count <= 1 {
            return Ok(false);
        }
        let existing_first_row_widths: Option<Vec<f32>> = {
            let widths: Vec<Option<f32>> = first_row
                .children()
                .map(|cell| {
                    if let Some(Node::TableCell(cell_node)) = cell.node() {
                        cell_node.col_width
                    } else {
                        None
                    }
                })
                .collect();
            if widths.iter().all(|width| width.is_some()) {
                Some(widths.into_iter().map(|width| width.unwrap()).collect())
            } else {
                None
            }
        };

        let selection = self.selection();
        let is_selected = {
            let doc = self.doc();
            let check = |node_id| {
                if let Some((_, t_id, _, c_idx)) = find_table_cell(doc, node_id) {
                    t_id == table_id && c_idx == col
                } else {
                    false
                }
            };
            check(selection.anchor.node_id) || check(selection.head.node_id)
        };

        if is_selected {
            let target_col_idx = if col + 1 < col_count {
                col + 1
            } else if col > 0 {
                col - 1
            } else {
                unreachable!("col_count <= 1 should be handled early return")
            };

            if let Some(first_row_id) = row_ids.first() {
                if let Some(first_row) = self.node(*first_row_id) {
                    if let Some(target_cell) = first_row.children().nth(target_col_idx) {
                        if let Some(pos) = leaf_block_start(&target_cell) {
                            self.set_selection(Selection::collapsed(pos));
                        }
                    }
                }
            }
        }

        for row_id in row_ids.iter().copied() {
            let row_node = self.node(row_id).context("Row not found")?;
            let cell_id = row_node.children().nth(col).map(|c| c.node_id());

            if let Some(cell_id) = cell_id {
                self.delete_node_recursive(cell_id)?;
            }
        }

        if let Some(existing_widths) = existing_first_row_widths {
            let next_widths = TableWidthModel::removed_ratio_widths(&existing_widths, col);

            if let Some(first_row_id) = row_ids.first() {
                let first_row_cell_ids: Vec<_> = {
                    let first_row = self.node(*first_row_id).context("First row not found")?;
                    first_row.children().map(|cell| cell.node_id()).collect()
                };

                for (cell_id, width) in first_row_cell_ids.into_iter().zip(next_widths.into_iter())
                {
                    let cell_mut = self.node_mut(cell_id).context("Cell not found")?;
                    cell_mut.as_mut().update(|node| {
                        if let Node::TableCell(cell_node) = node {
                            cell_node.col_width = Some(width);
                        }
                    })?;
                }
            }
        }

        self.mark_structure_mutation(table_id);

        Ok(true)
    }

    pub fn set_table_border_style(&mut self, table_id: NodeId, style: &str) -> Result<bool> {
        let border_style = match style {
            "solid" => TableBorderStyle::Solid,
            "dashed" => TableBorderStyle::Dashed,
            "dotted" => TableBorderStyle::Dotted,
            "none" => TableBorderStyle::None,
            _ => return Ok(false),
        };

        let table_mut = self.node_mut(table_id).context("Table not found")?;

        table_mut.as_mut().update(|node| {
            if let Node::Table(table_node) = node {
                table_node.border_style = border_style;
            }
        })?;

        self.mark_attr_mutation(table_id);
        self.push_effect(Effect::LayoutChanged);

        Ok(true)
    }

    pub fn set_table_align(&mut self, table_id: NodeId, align: TableAlign) -> Result<bool> {
        let table_mut = self.node_mut(table_id).context("Table not found")?;

        table_mut.as_mut().update(|node| {
            if let Node::Table(table_node) = node {
                table_node.align = align;
            }
        })?;

        self.mark_attr_mutation(table_id);
        self.push_effect(Effect::LayoutChanged);

        Ok(true)
    }

    pub fn set_table_width(
        &mut self,
        table_id: NodeId,
        width: f32,
        content_width: f32,
    ) -> Result<bool> {
        if !width.is_finite() || width < 0.0 || !content_width.is_finite() || content_width <= 0.0 {
            return Ok(false);
        }

        let ratio_widths = self.resolved_first_row_ratio_widths(table_id)?;
        if ratio_widths.is_empty() {
            return Ok(false);
        }

        let width_model = TableWidthModel::new(ratio_widths.len(), content_width);
        let proportion = width_model.proportion_for_actual_table_width(width);
        self.set_table_proportion(table_id, proportion)
    }

    pub fn set_table_proportion(&mut self, table_id: NodeId, proportion: f32) -> Result<bool> {
        if !proportion.is_finite() || !(0.0..=1.0).contains(&proportion) {
            return Ok(false);
        }

        let table_mut = self.node_mut(table_id).context("Table not found")?;

        table_mut.as_mut().update(|node| {
            if let Node::Table(table_node) = node {
                table_node.proportion = proportion;
            }
        })?;

        self.mark_structure_mutation(table_id);
        self.push_effect(Effect::LayoutChanged);

        Ok(true)
    }

    pub fn select_table(&mut self, table_id: NodeId) -> Result<bool> {
        let table_node = self.node(table_id).context("Table not found")?;
        if !matches!(table_node.node(), Some(Node::Table(_))) {
            return Ok(false);
        }

        let parent = table_node.parent().context("Table parent not found")?;
        let table_index = table_node.index().context("Table index not found")?;

        let anchor = Position::new(parent.node_id(), table_index, Affinity::Downstream);
        let head = Position::new(parent.node_id(), table_index + 1, Affinity::Upstream);
        self.set_selection(Selection::new(anchor, head));

        Ok(true)
    }

    pub fn select_table_row(&mut self, table_id: NodeId, row: usize) -> Result<bool> {
        let table_node = self.node(table_id).context("Table not found")?;

        let row_node = table_node.children().nth(row);
        let Some(row_node) = row_node else {
            return Ok(false);
        };

        let start_pos = leaf_block_start(&row_node).context("Cannot find start of row")?;
        let end_pos = leaf_block_end(&row_node).context("Cannot find end of row")?;

        self.set_selection(Selection::new(start_pos, end_pos));

        Ok(true)
    }

    pub fn select_table_column(&mut self, table_id: NodeId, col: usize) -> Result<bool> {
        let table_node = self.node(table_id).context("Table not found")?;

        let first_row = table_node.first_child();
        let last_row = table_node.children().last();

        let (Some(first_row), Some(last_row)) = (first_row, last_row) else {
            return Ok(false);
        };

        let first_cell = first_row.children().nth(col);
        let last_cell = last_row.children().nth(col);

        let (Some(first_cell), Some(last_cell)) = (first_cell, last_cell) else {
            return Ok(false);
        };

        let start_pos = leaf_block_start(&first_cell).context("Cannot find start of first cell")?;
        let end_pos = leaf_block_end(&last_cell).context("Cannot find end of last cell")?;

        self.set_selection(Selection::new(start_pos, end_pos));

        Ok(true)
    }

    pub fn move_table_row(
        &mut self,
        table_id: NodeId,
        from_row: usize,
        to_row: usize,
    ) -> Result<bool> {
        if from_row == to_row {
            return Ok(false);
        }

        let table_node = self.node(table_id).context("Table not found")?;
        let row_count = table_node.children().count();

        if from_row >= row_count || to_row >= row_count {
            return Ok(false);
        }

        let preserve_first_row_col_widths = from_row == 0 || to_row == 0;
        let preserved_col_widths = if preserve_first_row_col_widths {
            Some(self.first_row_col_widths(table_id)?)
        } else {
            None
        };

        let row_id = table_node
            .children()
            .nth(from_row)
            .map(|r| r.node_id())
            .context("Row not found")?;

        let row_node = self.node_mut(row_id).context("Row not found")?;
        row_node.as_mut().move_to(table_id, to_row)?;

        if let Some(col_widths) = preserved_col_widths {
            self.apply_first_row_col_widths(table_id, &col_widths)?;
        }

        self.mark_structure_mutation(table_id);
        self.push_effect(Effect::LayoutChanged);

        Ok(true)
    }

    fn first_row_col_widths(&self, table_id: NodeId) -> Result<Vec<Option<f32>>> {
        let table_node = self.node(table_id).context("Table not found")?;
        let first_row = table_node
            .children()
            .next()
            .context("First row not found")?;

        Ok(first_row
            .children()
            .map(|cell| {
                if let Some(Node::TableCell(cell_node)) = cell.node() {
                    cell_node.col_width
                } else {
                    None
                }
            })
            .collect())
    }

    fn resolved_first_row_ratio_widths(&self, table_id: NodeId) -> Result<Vec<f32>> {
        let col_widths = self.first_row_col_widths(table_id)?;
        let col_count = col_widths.len();
        if col_count == 0 {
            return Ok(Vec::new());
        }

        if col_widths.iter().all(|width| width.is_some()) {
            let raw_widths = col_widths
                .into_iter()
                .map(|width| width.unwrap_or(0.0))
                .collect::<Vec<_>>();
            if let Some(validated) = TableWidthModel::validate_ratio_widths(&raw_widths, col_count)
            {
                return Ok(validated);
            }
        }

        Ok(vec![1.0 / col_count as f32; col_count])
    }

    fn apply_first_row_col_widths(
        &mut self,
        table_id: NodeId,
        col_widths: &[Option<f32>],
    ) -> Result<()> {
        let first_row_cell_ids: Vec<_> = {
            let table_node = self.node(table_id).context("Table not found")?;
            let first_row = table_node
                .children()
                .next()
                .context("First row not found")?;
            first_row.children().map(|cell| cell.node_id()).collect()
        };

        if first_row_cell_ids.len() != col_widths.len() {
            return Ok(());
        }

        for (cell_id, col_width) in first_row_cell_ids
            .into_iter()
            .zip(col_widths.iter().copied())
        {
            let cell_mut = self.node_mut(cell_id).context("Cell not found")?;
            cell_mut.as_mut().update(|node| {
                if let Node::TableCell(cell_node) = node {
                    cell_node.col_width = col_width;
                }
            })?;
        }

        Ok(())
    }

    pub fn move_table_column(
        &mut self,
        table_id: NodeId,
        from_col: usize,
        to_col: usize,
    ) -> Result<bool> {
        if from_col == to_col {
            return Ok(false);
        }

        let table_node = self.node(table_id).context("Table not found")?;
        let row_ids: Vec<_> = table_node.children().map(|r| r.node_id()).collect();

        if row_ids.is_empty() {
            return Ok(false);
        }

        let first_row = self.node(row_ids[0]).context("First row not found")?;
        let col_count = first_row.children().count();

        if from_col >= col_count || to_col >= col_count {
            return Ok(false);
        }

        for row_id in &row_ids {
            let row_node = self.node(*row_id).context("Row not found")?;
            let cell_id = row_node
                .children()
                .nth(from_col)
                .map(|c| c.node_id())
                .context("Cell not found")?;

            let cell_node = self.node_mut(cell_id).context("Cell not found")?;
            cell_node.as_mut().move_to(*row_id, to_col)?;
        }

        self.mark_structure_mutation(table_id);
        self.push_effect(Effect::LayoutChanged);

        Ok(true)
    }

    pub fn delete_structure_selection(&mut self, info: &StructureSelectionInfo) -> Result<bool> {
        match info {
            StructureSelectionInfo::None => Ok(false),
            StructureSelectionInfo::Structural(_) => Ok(false),
            StructureSelectionInfo::Rectangular { table_id, range } => {
                let table_node = self.node(*table_id).context("Table not found")?;
                let row_ids: Vec<_> = table_node.children().map(|c| c.node_id()).collect();

                let ((r_start, r_end), (c_start, c_end)) = range;
                let mut first_new_para_id = None;

                for r in *r_start..=*r_end {
                    if let Some(row_id) = row_ids.get(r) {
                        let row_node = self.node(*row_id).context("Row not found")?;
                        let cell_ids: Vec<_> = row_node.children().map(|c| c.node_id()).collect();

                        for c in *c_start..=*c_end {
                            if let Some(cell_id) = cell_ids.get(c) {
                                let cell = self.node(*cell_id).context("Cell not found")?;
                                let children: Vec<_> =
                                    cell.children().map(|c| c.node_id()).collect();

                                for child_id in children {
                                    self.delete_node_recursive(child_id)?;
                                }

                                let para_id = NodeId::new();
                                let cell_mut = self.node_mut(*cell_id).context("Cell not found")?;
                                cell_mut.as_mut().insert_child_with_id(
                                    0,
                                    para_id,
                                    Node::Paragraph(ParagraphNode::default()),
                                )?;

                                if first_new_para_id.is_none() {
                                    first_new_para_id = Some(para_id);
                                }
                            }
                        }
                    }
                }

                self.mark_structure_mutation(*table_id);

                if let Some(para_id) = first_new_para_id {
                    self.set_selection(Selection::collapsed(Position::new(
                        para_id,
                        0,
                        Affinity::Downstream,
                    )));
                }

                Ok(true)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Message;

    #[test]
    fn insert_table_appends_trailing_paragraph_when_inserted_at_end() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph { text { "start" } }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| {
            let inserted = tr.insert_table(1, 1).unwrap();
            assert!(inserted.is_some());
        });

        let root = actual.doc.node(NodeId::ROOT).unwrap();
        let child_types: Vec<NodeType> = root
            .children()
            .filter_map(|child| child.node_type())
            .collect();

        assert_eq!(
            child_types,
            vec![NodeType::Paragraph, NodeType::Table, NodeType::Paragraph],
            "insert_table should preserve root trailing paragraph invariant"
        );
    }

    #[test]
    fn test_delete_cell_selection_rectangular() {
        let mut p1 = id!();
        let mut p2 = id!();

        let initial = state! {
            doc {
                table {
                    table_row {
                        table_cell { @p1 paragraph { text { "cell1" } } }
                        table_cell { @p2 paragraph { text { "cell2" } } }
                    }
                    table_row {
                        table_cell { paragraph { text { "cell3" } } }
                        table_cell { paragraph { text { "cell4" } } }
                    }
                }
            }
            selection { (p1, 0) -> (p2, 5) }
        };

        let actual = transact!(initial, |tr| {
            tr.delete_selection().unwrap();
        });

        let doc = actual.doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        let table = root
            .children()
            .find(|n| n.node_type() == Some(NodeType::Table))
            .unwrap();
        let row = table.first_child().unwrap();
        let cell1 = row.first_child().unwrap();
        let cell2 = row.children().nth(1).unwrap();

        assert_eq!(cell1.children().count(), 1);
        let p1_new = cell1.first_child().unwrap();
        assert_eq!(p1_new.children().count(), 0);

        assert_eq!(cell2.children().count(), 1);
        let p2_new = cell2.first_child().unwrap();
        assert_eq!(p2_new.children().count(), 0);
    }

    #[test]
    fn test_add_row_before_first_row() {
        let mut t = id!();
        let mut p00 = id!();

        let initial = state! {
            doc {
                @t table {
                    table_row {
                        table_cell { @p00 paragraph { text { "r0" } } }
                    }
                    table_row {
                        table_cell { paragraph { text { "r1" } } }
                    }
                }
            }
            selection { (p00, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.add_table_row(t, 0, true).unwrap();
        });

        let doc = actual.doc;
        let table = doc.node(t).unwrap();
        assert_eq!(table.children().count(), 3);

        let first_row = table.first_child().unwrap();
        let first_cell = first_row.first_child().unwrap();
        let first_para = first_cell.first_child().unwrap();
        assert_eq!(first_para.children().count(), 0);

        let second_row = table.children().nth(1).unwrap();
        let second_cell = second_row.first_child().unwrap();
        let second_para = second_cell.first_child().unwrap();
        assert_eq!(second_para.node_id(), p00);
    }

    #[test]
    fn test_add_column_before_first_column() {
        let mut t = id!();
        let mut p00 = id!();

        let initial = state! {
            doc {
                @t table {
                    table_row {
                        table_cell { @p00 paragraph { text { "r0c0" } } }
                        table_cell { paragraph { text { "r0c1" } } }
                    }
                    table_row {
                        table_cell { paragraph { text { "r1c0" } } }
                        table_cell { paragraph { text { "r1c1" } } }
                    }
                }
            }
            selection { (p00, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.add_table_column(t, 0, true).unwrap();
        });

        let doc = actual.doc;
        let table = doc.node(t).unwrap();

        let first_row = table.first_child().unwrap();
        assert_eq!(first_row.children().count(), 3);

        let first_cell = first_row.first_child().unwrap();
        let first_para = first_cell.first_child().unwrap();
        assert_eq!(first_para.children().count(), 0);

        let second_cell = first_row.children().nth(1).unwrap();
        let second_para = second_cell.first_child().unwrap();
        assert_eq!(second_para.node_id(), p00);
    }

    #[test]
    fn test_add_column_preserves_ratio_model() {
        let mut t = id!();
        let mut p00 = id!();

        let initial = state! {
            doc {
                @t table {
                    table_row {
                        table_cell { @p00 paragraph { text { "r0c0" } } }
                        table_cell { paragraph { text { "r0c1" } } }
                    }
                    table_row {
                        table_cell { paragraph { text { "r1c0" } } }
                        table_cell { paragraph { text { "r1c1" } } }
                    }
                }
            }
            selection { (p00, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.set_column_widths(t, vec![0.3, 0.7]).unwrap();
            tr.add_table_column(t, 1, false).unwrap();
        });

        let doc = actual.doc;
        let table = doc.node(t).unwrap();
        let first_row = table.first_child().unwrap();
        let widths: Vec<f32> = first_row
            .children()
            .map(|cell| {
                if let Some(Node::TableCell(cell_node)) = cell.node() {
                    cell_node.col_width.unwrap()
                } else {
                    panic!("Expected table cell");
                }
            })
            .collect();

        assert_eq!(widths.len(), 3);
        assert!((widths[0] - 0.2).abs() < 0.001);
        assert!((widths[1] - 0.46666667).abs() < 0.001);
        assert!((widths[2] - 0.33333334).abs() < 0.001);
        assert!((widths.iter().sum::<f32>() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_delete_cell_selection_full_table() {
        let mut t = id!();
        let mut p_before = id!();
        let mut p_after = id!();

        let mut rt = runtime! {
            doc {
                @p_before paragraph { text { "before" } }
                @t table {
                    table_row {
                        table_cell { paragraph { text { "cell" } } }
                    }
                }
                @p_after paragraph { text { "after" } }
            }
            selection { (p_before, 0) -> (p_after, 5) }
        };

        rt.update(Message::DeleteSelection);
        rt.flush();

        let doc = rt.doc();
        assert!(doc.node(t).is_none());
    }

    #[test]
    fn test_delete_cell_selection_internal_to_external() {
        let mut t = id!();
        let mut p_internal = id!();
        let mut p_external = id!();

        let initial = state! {
          doc {
            @t table {
              table_row {
                table_cell { @p_internal paragraph { text { "internal" } } }
              }
            }
            @p_external paragraph { text { "external" } }
          }
          selection { (p_internal, 0) -> (p_external, 3) }
        };

        let actual = transact!(initial, |tr| {
            tr.delete_selection().unwrap();
        });

        let doc = actual.doc;
        assert!(
            doc.node(t).is_none(),
            "Table should be deleted when selected internal-to-external"
        );
    }

    #[test]
    fn test_delete_mixed_table_selection() {
        let mut n2 = id!();
        let mut n3 = id!();

        let initial = state! {
            doc {
                paragraph {}
                table(border_style: TableBorderStyle::Solid,) {
                    table_row {
                        table_cell {
                            @n2 paragraph {
                                text { "123" }
                            }
                        }
                    }
                }
                @n3 paragraph {
                    text { "456" }
                }
            }
            selection { (n3, 1) -> (n2, 2) }
        };

        let actual = transact!(initial, |tr| {
            tr.delete_selection().unwrap();
        });

        let doc = actual.doc;

        let n3_node = doc.node(n3).expect("n3 should exist");
        if let Some(Node::Paragraph(_p)) = n3_node.node() {
            let text_child = n3_node.first_child().expect("n3 should have text child");
            if let Some(Node::Text(t)) = text_child.node() {
                assert_eq!(t.text.to_string(), "56", "Expected '4' to be deleted");
            } else {
                panic!("n3 child should be text");
            }
        } else {
            panic!("n3 should be paragraph");
        }
    }

    #[test]
    fn test_delete_row_with_selection() {
        let mut t = id!();
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let initial = state! {
            doc {
                @t table {
                    table_row {
                        table_cell { @p1 paragraph { text { "row1" } } }
                    }
                    table_row {
                        table_cell { @p2 paragraph { text { "row2" } } }
                    }
                    table_row {
                        table_cell { @p3 paragraph { text { "row3" } } }
                    }
                }
            }
            selection { (p2, 2) }
        };

        let actual = transact!(initial, |tr| {
            tr.delete_table_row(t, 1).unwrap();
        });

        let doc = actual.doc;
        let table = doc.node(t).unwrap();
        assert_eq!(table.children().count(), 2);

        assert!(
            doc.node(actual.selection.anchor.node_id).is_some(),
            "Selection anchor node should exist"
        );
    }

    #[test]
    fn test_delete_column_with_selection() {
        let mut t = id!();
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();
        let mut p4 = id!();

        let initial = state! {
            doc {
                @t table {
                    table_row {
                        table_cell { @p1 paragraph { text { "c1" } } }
                        table_cell { @p2 paragraph { text { "c2" } } }
                    }
                    table_row {
                        table_cell { @p3 paragraph { text { "c1" } } }
                        table_cell { @p4 paragraph { text { "c2" } } }
                    }
                }
            }
            selection { (p4, 1) }
        };

        let actual = transact!(initial, |tr| {
            tr.delete_table_column(t, 1).unwrap();
        });

        let doc = actual.doc;
        let table = doc.node(t).unwrap();
        let row1 = table.first_child().unwrap();
        assert_eq!(row1.children().count(), 1, "Row should have 1 cell left");

        let sel = actual.selection;
        assert!(
            doc.node(sel.anchor.node_id).is_some(),
            "Selection anchor node should exist"
        );

        assert_eq!(sel.anchor.node_id, p1, "Selection should move to p1");
    }

    #[test]
    fn test_delete_column_preserves_ratio_model() {
        let mut t = id!();
        let mut p00 = id!();

        let initial = state! {
            doc {
                @t table {
                    table_row {
                        table_cell { @p00 paragraph { text { "r0c0" } } }
                        table_cell { paragraph { text { "r0c1" } } }
                        table_cell { paragraph { text { "r0c2" } } }
                    }
                    table_row {
                        table_cell { paragraph { text { "r1c0" } } }
                        table_cell { paragraph { text { "r1c1" } } }
                        table_cell { paragraph { text { "r1c2" } } }
                    }
                }
            }
            selection { (p00, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.set_column_widths(t, vec![0.2, 0.3, 0.5]).unwrap();
            tr.delete_table_column(t, 1).unwrap();
        });

        let doc = actual.doc;
        let table = doc.node(t).unwrap();
        let first_row = table.first_child().unwrap();
        let widths: Vec<f32> = first_row
            .children()
            .map(|cell| {
                if let Some(Node::TableCell(cell_node)) = cell.node() {
                    cell_node.col_width.unwrap()
                } else {
                    panic!("Expected table cell");
                }
            })
            .collect();

        assert_eq!(widths.len(), 2);
        assert!((widths[0] - 0.2857143).abs() < 0.001);
        assert!((widths[1] - 0.71428573).abs() < 0.001);
        assert!((widths.iter().sum::<f32>() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_move_table_row_preserves_column_widths_when_first_row_changes() {
        let mut t = id!();
        let mut p00 = id!();
        let mut p10 = id!();

        let initial = state! {
            doc {
                @t table {
                    table_row {
                        table_cell { @p00 paragraph { text { "r0c0" } } }
                        table_cell { paragraph { text { "r0c1" } } }
                    }
                    table_row {
                        table_cell { @p10 paragraph { text { "r1c0" } } }
                        table_cell { paragraph { text { "r1c1" } } }
                    }
                }
            }
            selection { (p00, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.set_column_widths(t, vec![0.4, 0.6]).unwrap();
            tr.move_table_row(t, 0, 1).unwrap();
        });

        let doc = actual.doc;
        let table = doc.node(t).unwrap();
        let first_row = table.first_child().unwrap();

        let first_cell_after_move = first_row.first_child().unwrap();
        let first_para_after_move = first_cell_after_move.first_child().unwrap();
        assert_eq!(
            first_para_after_move.node_id(),
            p10,
            "First row should now be the previous second row"
        );

        let widths: Vec<_> = first_row
            .children()
            .map(|cell| {
                if let Some(Node::TableCell(cell_node)) = cell.node() {
                    cell_node.col_width
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(widths, vec![Some(0.4), Some(0.6)]);
    }

    #[test]
    fn test_set_table_proportion() {
        let mut t = id!();
        let mut p = id!();

        let initial = state! {
            doc {
                @t table {
                    table_row {
                        table_cell { @p paragraph { text { "cell" } } }
                    }
                }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.set_table_proportion(t, 0.8).unwrap();
        });

        let table = actual.doc.node(t).unwrap();
        if let Some(Node::Table(table_node)) = table.node() {
            assert_eq!(table_node.proportion, 0.8);
        } else {
            panic!("Expected table node");
        }
    }

    #[test]
    fn test_set_table_proportion_rejects_invalid_values() {
        let mut t = id!();
        let mut p = id!();

        let initial = state! {
            doc {
                @t table {
                    table_row {
                        table_cell { @p paragraph { text { "cell" } } }
                    }
                }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| {
            assert!(!tr.set_table_proportion(t, -0.1).unwrap());
            assert!(!tr.set_table_proportion(t, 1.1).unwrap());
            assert!(!tr.set_table_proportion(t, f32::NAN).unwrap());
        });

        let table = actual.doc.node(t).unwrap();
        if let Some(Node::Table(table_node)) = table.node() {
            assert_eq!(table_node.proportion, 1.0);
        } else {
            panic!("Expected table node");
        }
    }

    #[test]
    fn test_select_table_selects_table_node_range() {
        let mut t = id!();
        let mut p = id!();

        let initial = state! {
            doc {
                paragraph {}
                @t table {
                    table_row {
                        table_cell { @p paragraph { text { "cell" } } }
                    }
                }
                paragraph {}
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.select_table(t).unwrap();
        });

        let selection = actual.selection;
        assert_eq!(selection.anchor.node_id, NodeId::ROOT);
        assert_eq!(selection.head.node_id, NodeId::ROOT);
        assert_eq!(selection.anchor.offset, 1);
        assert_eq!(selection.head.offset, 2);
    }
}
