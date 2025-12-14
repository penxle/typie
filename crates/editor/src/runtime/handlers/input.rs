use super::super::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_input(&mut self, text: &str) -> Vec<Effect> {
        self.transact(|tr| {
            tr.delete_selection()?;
            tr.insert_text(text)
        })
    }
}
