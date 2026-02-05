use super::super::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_select_ai_feedback_item(&mut self, item_id: String) -> Vec<Effect> {
        if self.active_ai_feedback_item_id.as_deref() == Some(&item_id) {
            return vec![];
        }

        self.active_ai_feedback_item_id = Some(item_id);
        self.pending.ai_feedback_overlays = true;

        vec![]
    }
}
