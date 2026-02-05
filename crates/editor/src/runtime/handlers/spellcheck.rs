use super::super::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_select_spellcheck_error(&mut self, error_id: String) -> Vec<Effect> {
        if self.active_spellcheck_error_id.as_deref() == Some(&error_id) {
            return vec![];
        }

        self.active_spellcheck_error_id = Some(error_id);
        self.pending.spellcheck_overlays = true;

        vec![]
    }
}
