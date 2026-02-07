use crate::runtime::Effect;
use crate::state::Preedit;
use crate::transaction::Transaction;
use crate::utils::collect_codepoints;
use anyhow::Result;

impl Transaction {
    pub fn set_preedit(&mut self, text: String) -> Result<bool> {
        let selection = self.selection().clone();
        if !selection.is_collapsed() {
            return Ok(false);
        }

        let codepoints = collect_codepoints(&text);
        if !codepoints.is_empty() {
            let (family, weight) = self.current_font();
            self.push_effect(Effect::FontDetected {
                family,
                weight,
                codepoints: codepoints.clone(),
            });
            self.push_effect(Effect::CodepointDetected { codepoints });
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
        if let Some(preedit) = self.state.preedit.take() {
            self.state.pending_marks = preedit.marks;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn commit_preedit(&mut self) -> Result<bool> {
        if let Some(preedit) = self.state.preedit.take() {
            let text = preedit.text.clone();
            let marks = preedit.marks.clone();

            self.set_selection(crate::state::Selection::collapsed(
                crate::state::Position::new(
                    preedit.node_id,
                    preedit.offset,
                    crate::types::Affinity::Downstream,
                ),
            ));

            self.state.pending_marks = marks;
            self.insert_text(&text)?;

            Ok(true)
        } else {
            Ok(false)
        }
    }
}
