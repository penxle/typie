use crate::model::NodeId;
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_insert_table(&mut self, rows: u32, cols: u32) -> Vec<Effect> {
        self.transact(|tr| {
            let table_id = tr.insert_table(rows, cols)?;
            Ok(table_id.is_some())
        })
    }

    pub(crate) fn handle_set_column_widths(
        &mut self,
        table_id: String,
        col_widths: Vec<f32>,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.set_column_widths(node_id, col_widths))
    }

    pub(crate) fn handle_add_table_row(
        &mut self,
        table_id: String,
        after_row: usize,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.add_table_row(node_id, after_row))
    }

    pub(crate) fn handle_add_table_column(
        &mut self,
        table_id: String,
        after_col: usize,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.add_table_column(node_id, after_col))
    }

    pub(crate) fn handle_delete_table_row(&mut self, table_id: String, row: usize) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.delete_table_row(node_id, row))
    }

    pub(crate) fn handle_delete_table_column(
        &mut self,
        table_id: String,
        col: usize,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.delete_table_column(node_id, col))
    }

    pub(crate) fn handle_set_table_border_style(
        &mut self,
        table_id: String,
        style: String,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.set_table_border_style(node_id, &style))
    }
}
