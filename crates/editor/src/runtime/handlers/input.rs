use super::super::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_input(&mut self, text: &str) -> Vec<Effect> {
        if let Some(effects) = self.try_auto_surround(text) {
            return effects;
        }

        // NOTE: delete 후 transaction commit 단계에서 trailing paragraph 생성 후 normalize_selection이 되어야 하기 때문에 transact 나눠서 실행
        let delete_effects = self.transact(|tr| tr.delete_selection());
        let insert_effects = self.transact(|tr| tr.insert_text(text));

        let mut effects = delete_effects;
        effects.extend(insert_effects);
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
