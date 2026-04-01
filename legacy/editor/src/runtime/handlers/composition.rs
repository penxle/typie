use crate::runtime::{Effect, Runtime};
use crate::state::Position;

impl Runtime {
    pub fn handle_composition_update(
        &mut self,
        text: &str,
        replace_length: Option<usize>,
    ) -> Vec<Effect> {
        self.transact(|tr| {
            tr.delete_selection()?;
            if let Some(len) = replace_length.filter(|&l| l > 0) {
                let head = tr.selection().head;
                let from =
                    Position::new(head.node_id, head.offset.saturating_sub(len), head.affinity);
                tr.delete_range(from, head)?;
            }
            tr.set_preedit(text.to_string())
        })
    }

    pub fn handle_composition_end(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.complete_preedit())
    }

    pub fn handle_commit_preedit(&mut self) -> Vec<Effect> {
        let input_byte_len = self.state.preedit.as_ref().map_or(0, |p| p.text.len());
        self.transact(|tr| {
            tr.commit_preedit()?;
            tr.try_text_replacement(input_byte_len)?;
            Ok(true)
        })
    }
}
