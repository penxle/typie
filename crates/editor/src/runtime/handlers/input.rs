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

    pub(crate) fn handle_replace_backward(&mut self, length: usize, text: &str) -> Vec<Effect> {
        self.transact(|tr| {
            for _ in 0..length {
                tr.delete_text_backward()?;
            }
            tr.insert_text(text)
        })
    }
}
