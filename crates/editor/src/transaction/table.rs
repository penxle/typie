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
}
