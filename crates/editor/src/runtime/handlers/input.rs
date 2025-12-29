use super::super::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_input(&mut self, text: &str) -> Vec<Effect> {
        if let Some(effects) = self.try_auto_surround(text) {
            return effects;
        }

        self.transact(|tr| {
            tr.delete_selection()?;
            tr.insert_text(text)
        })
    }
}
