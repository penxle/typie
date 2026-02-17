use crate::model::{NodeId, TableAlign};
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
        row: usize,
        before: bool,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.add_table_row(node_id, row, before))
    }

    pub(crate) fn handle_add_table_column(
        &mut self,
        table_id: String,
        col: usize,
        before: bool,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.add_table_column(node_id, col, before))
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

    pub(crate) fn handle_set_table_align(
        &mut self,
        table_id: String,
        align: TableAlign,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.set_table_align(node_id, align))
    }

    pub(crate) fn handle_set_table_proportion(
        &mut self,
        table_id: String,
        proportion: f32,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.set_table_proportion(node_id, proportion))
    }

    pub(crate) fn handle_set_table_width(
        &mut self,
        table_id: String,
        width: f32,
        content_width: f32,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.set_table_width(node_id, width, content_width))
    }

    pub(crate) fn handle_select_table(&mut self, table_id: String) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.select_table(node_id))
    }

    pub(crate) fn handle_select_table_row(&mut self, table_id: String, row: usize) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.select_table_row(node_id, row))
    }

    pub(crate) fn handle_select_table_column(
        &mut self,
        table_id: String,
        col: usize,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.select_table_column(node_id, col))
    }

    pub(crate) fn handle_move_table_row(
        &mut self,
        table_id: String,
        from_row: usize,
        to_row: usize,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.move_table_row(node_id, from_row, to_row))
    }

    pub(crate) fn handle_move_table_column(
        &mut self,
        table_id: String,
        from_col: usize,
        to_col: usize,
    ) -> Vec<Effect> {
        let Some(node_id) = NodeId::from_string(&table_id) else {
            return vec![];
        };

        self.transact(|tr| tr.move_table_column(node_id, from_col, to_col))
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::Message;

    #[test]
    fn set_table_proportion_invalidates_table_descendant_layout_cache() {
        let mut t = id!();
        let mut r0 = id!();
        let mut p00 = id!();

        let mut runtime = runtime! {
            viewport {
                paginated { width: 800.0, height: 600.0, margin: 50.0 }
            }
            doc {
                @t table {
                    @r0 table_row {
                        table_cell { @p00 paragraph { text { "r0c0" } } }
                        table_cell { paragraph { text { "r0c1" } } }
                    }
                }
            }
            selection { (p00, 0) }
        };

        assert!(
            runtime.is_layout_cached(r0),
            "row should be cached after initial layout"
        );

        runtime.update(Message::SetTableProportion {
            table_id: t.to_string(),
            proportion: 0.8,
        });

        assert!(
            !runtime.is_layout_cached(r0),
            "row cache should be invalidated when table proportion changes"
        );
    }
}
