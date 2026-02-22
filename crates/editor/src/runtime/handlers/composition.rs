use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_composition_update(&mut self, text: &str) -> Vec<Effect> {
        self.transact(|tr| {
            tr.delete_selection()?;
            tr.set_preedit(text.to_string())
        })
    }

    pub(crate) fn handle_composition_end(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.complete_preedit())
    }

    pub(crate) fn handle_commit_preedit(&mut self) -> Vec<Effect> {
        let input_byte_len = self.state.preedit.as_ref().map_or(0, |p| p.text.len());
        self.transact(|tr| {
            tr.commit_preedit()?;
            tr.try_text_replacement(input_byte_len)?;
            Ok(true)
        })
    }
}
