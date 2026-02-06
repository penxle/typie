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

        // NOTE: delete 후 transaction commit 단계에서 trailing paragraph 생성 후 normalize_selection이 되어야 하기 때문에 transact 나눠서 실행
        effects.extend(self.transact(|tr| tr.delete_selection()));
        effects.extend(self.transact(|tr| tr.insert_text(text)));

        if let Some(replacement_effects) = self.try_text_replacement() {
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
