use crate::model::Fragment;
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_paste_html(&mut self, html: String, text: String) -> Vec<Effect> {
        match Fragment::from_html(&html) {
            Ok(frag) => self.transact(|tr| {
                tr.delete_selection()?;
                tr.normalize()?;
                tr.paste_fragment(frag, Some(text))
            }),
            Err(_e) => self.transact(|tr| {
                tr.delete_selection()?;
                tr.normalize()?;
                tr.paste_text(text)
            }),
        }
    }

    pub(crate) fn handle_paste_text(&mut self, text: String) -> Vec<Effect> {
        self.transact(|tr| {
            tr.delete_selection()?;
            tr.normalize()?;
            tr.paste_text(text)
        })
    }

    pub(crate) fn handle_repaste_as_text(&mut self) -> Vec<Effect> {
        let Some((selection, text, styles)) = self.repaste_text.take() else {
            return vec![];
        };

        let Ok((from, to)) = selection.as_sorted(&self.state.doc) else {
            return vec![];
        };

        self.transact(|tr| tr.replace_range(from, to, Fragment::from_text(&text, &styles)))
    }
}
