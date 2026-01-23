use crate::model::{Node, NodeId, ParagraphNode, TableCellNode, TableNode, TableRowNode};
use crate::runtime::Effect;
use crate::state::{Position, Selection};
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
        let parent_spec = parent.spec();

        let table_type = crate::model::NodeType::Table;
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

        self.push_effect(Effect::NodeChanged { node_id: table_id });
        self.push_effect(Effect::StructureChanged);

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

        for (cell_id, &width) in first_row_cells.iter().zip(col_widths.iter()) {
            let cell_mut = self.node_mut(*cell_id).context("Cell not found")?;

            cell_mut.as_mut().update(|node| {
                if let Node::TableCell(cell_node) = node {
                    cell_node.col_width = Some(width);
                }
            })?;
        }

        self.push_effect(Effect::SubtreeChanged { node_id: table_id });
        self.push_effect(Effect::LayoutChanged);

        Ok(true)
    }

    pub fn add_table_row(&mut self, table_id: NodeId, after_row: usize) -> Result<bool> {
        let table_node = self.node(table_id).context("Table not found")?;

        let first_row = table_node.children().next();
        let col_count = first_row.map(|r| r.children().count()).unwrap_or(0);
        if col_count == 0 {
            return Ok(false);
        }

        let row_count = table_node.children().count();
        let insert_index = (after_row + 1).min(row_count);

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

        self.push_effect(Effect::SubtreeChanged { node_id: table_id });
        self.push_effect(Effect::StructureChanged);
        self.push_effect(Effect::LayoutChanged);

        Ok(true)
    }

    pub fn add_table_column(&mut self, table_id: NodeId, after_col: usize) -> Result<bool> {
        let table_node = self.node(table_id).context("Table not found")?;

        let row_ids: Vec<_> = table_node.children().map(|r| r.node_id()).collect();
        if row_ids.is_empty() {
            return Ok(false);
        }

        let first_row = self.node(row_ids[0]).context("First row not found")?;
        let col_count = first_row.children().count();
        let insert_index = (after_col + 1).min(col_count);

        let mut first_row_new_cell_id: Option<NodeId> = None;
        for (row_idx, row_id) in row_ids.iter().enumerate() {
            let cell_id = NodeId::new();
            let para_id = NodeId::new();

            if row_idx == 0 {
                first_row_new_cell_id = Some(cell_id);
            }

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

        if let Some(cell_id) = first_row_new_cell_id {
            use crate::model::DEFAULT_CELL_WIDTH;
            let cell_mut = self.node_mut(cell_id).context("Cell not found")?;
            cell_mut.as_mut().update(|node| {
                if let Node::TableCell(cell_node) = node {
                    cell_node.col_width = Some(DEFAULT_CELL_WIDTH);
                }
            })?;
        }

        self.push_effect(Effect::SubtreeChanged { node_id: table_id });
        self.push_effect(Effect::StructureChanged);
        self.push_effect(Effect::LayoutChanged);

        Ok(true)
    }

    pub fn delete_table_row(&mut self, table_id: NodeId, row: usize) -> Result<bool> {
        let table_node = self.node(table_id).context("Table not found")?;

        let row_count = table_node.children().count();
        if row >= row_count || row_count <= 1 {
            return Ok(false);
        }

        let row_id = table_node.children().nth(row).map(|r| r.node_id());
        let Some(row_id) = row_id else {
            return Ok(false);
        };

        self.delete_node_recursive(row_id)?;

        self.push_effect(Effect::SubtreeChanged { node_id: table_id });
        self.push_effect(Effect::StructureChanged);

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

        for row_id in row_ids {
            let row_node = self.node(row_id).context("Row not found")?;
            let cell_id = row_node.children().nth(col).map(|c| c.node_id());

            if let Some(cell_id) = cell_id {
                self.delete_node_recursive(cell_id)?;
            }
        }

        self.push_effect(Effect::SubtreeChanged { node_id: table_id });
        self.push_effect(Effect::StructureChanged);

        Ok(true)
    }

    pub fn set_table_border_style(&mut self, table_id: NodeId, style: &str) -> Result<bool> {
        use crate::model::TableBorderStyle;

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

        self.push_effect(Effect::NodeChanged { node_id: table_id });
        self.push_effect(Effect::LayoutChanged);

        Ok(true)
    }
}
