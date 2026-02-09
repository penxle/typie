use super::super::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_input(&mut self, text: &str) -> Vec<Effect> {
        let mut effects = Vec::new();

        if self.state.preedit.is_some() {
            effects.extend(self.transact(|tr| tr.complete_preedit()));
        }

        if let Some(surround_effects) = self.try_auto_surround(text) {
            effects.extend(surround_effects);
            return effects;
        }

        effects.extend(self.transact(|tr| {
            tr.delete_selection()?;
            tr.normalize()?;
            tr.insert_text(text)
        }));

        if let Some(replacement_effects) = self.try_text_replacement(text.len()) {
            effects.extend(replacement_effects);
        }

        effects
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
