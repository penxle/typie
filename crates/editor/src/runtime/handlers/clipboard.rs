use super::super::{Effect, PasteMode, Runtime};
use crate::model::Fragment;

impl Runtime {
    pub(crate) fn handle_paste(
        &mut self,
        html: Option<String>,
        text: String,
        mode: PasteMode,
    ) -> Vec<Effect> {
        if let Some(html_str) = html {
            match Fragment::from_html(&html_str) {
                Ok(frag) => {
                    if mode == PasteMode::Text {
                        let plain = frag.to_plain_text();
                        return self.transact(|tr| {
                            tr.delete_selection()?;
                            tr.paste_text(plain)
                        });
                    }

                    return self.transact(|tr| {
                        tr.delete_selection()?;
                        tr.paste_fragment(frag)
                    });
                }
                Err(e) => {
                    error!("HTML parse error: {:?}", e);
                }
            }
        }

        self.transact(|tr| {
            tr.delete_selection()?;
            tr.paste_text(text)
        })
    }
}
