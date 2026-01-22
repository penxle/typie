use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_insert_table(&mut self, rows: u32, cols: u32) -> Vec<Effect> {
        self.transact(|tr| {
            let table_id = tr.insert_table(rows, cols)?;
            Ok(table_id.is_some())
        })
    }
}
