use super::super::{Effect, Runtime};
use crate::state::{Position, Selection};
use crate::types::Affinity;

impl Runtime {
    pub(crate) fn handle_select_spellcheck_error(&mut self, error_id: String) -> Vec<Effect> {
        let Some(error) = self.spellcheck_errors.iter().find(|e| e.id == error_id) else {
            return vec![];
        };

        let Some((node_id, start_offset, end_offset)) = error.resolve_range(&self.state.doc) else {
            return vec![];
        };

        self.transact(move |tr| {
            tr.set_selection(Selection::new(
                Position::new(node_id, start_offset, Affinity::Downstream),
                Position::new(node_id, end_offset, Affinity::Upstream),
            ));
            Ok(true)
        })
    }
}
