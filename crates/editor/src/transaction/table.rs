use crate::model::{Node, NodeId, ParagraphNode, TableCellNode, TableNode, TableRowNode};
use crate::runtime::Effect;
use crate::state::selection_helpers::CellSelectionInfo;
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

    pub fn delete_cell_selection(&mut self, info: &CellSelectionInfo) -> Result<bool> {
        match info {
            CellSelectionInfo::None => Ok(false),
            CellSelectionInfo::FullTables(_) => Ok(false),
            CellSelectionInfo::Rectangular { table_id, range } => {
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

                self.push_effect(Effect::SubtreeChanged { node_id: *table_id });

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
    use crate::{model::NodeId, runtime::Message};

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
                }
            }
            selection { (p1, 0) -> (p2, 5) }
        };

        let actual = transact!(initial, |tr| {
            tr.delete_selection().unwrap();
        });

        let doc = actual.doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        let table = root.first_child().unwrap();
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
        use crate::model::{Node, TableBorderStyle};
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
        if let Node::Paragraph(_p) = n3_node.node() {
            let text_child = n3_node.first_child().expect("n3 should have text child");
            if let Node::Text(t) = text_child.node() {
                assert_eq!(t.text.to_string(), "56", "Expected '4' to be deleted");
            } else {
                panic!("n3 child should be text");
            }
        } else {
            panic!("n3 should be paragraph");
        }
    }
}
