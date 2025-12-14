use crate::state::Preedit;
use crate::transaction::Transaction;
use anyhow::Result;

impl Transaction {
    pub fn set_preedit(&mut self, text: String) -> Result<bool> {
        let selection = self.selection().clone();
        if !selection.is_collapsed() {
            return Ok(false);
        }

        self.state.preedit = Some(Preedit {
            node_id: selection.head.node_id,
            offset: selection.head.offset,
            text,
            marks: self.state.pending_marks.clone(),
        });

        Ok(true)
    }

    pub fn complete_preedit(&mut self) -> Result<bool> {
        self.state.preedit = None;
        self.state.pending_marks = None;
        Ok(true)
    }
}
