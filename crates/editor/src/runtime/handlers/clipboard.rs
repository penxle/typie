use super::super::{Effect, Runtime};
use crate::model::Fragment;

impl Runtime {
    pub(crate) fn handle_paste(
        &mut self,
        fragment: Option<String>,
        html: Option<String>,
        text: String,
    ) -> Vec<Effect> {
        if let Some(json) = fragment {
            match Fragment::from_json(&json) {
                Ok(frag) => {
                    return self.transact(|tr| {
                        tr.delete_selection()?;
                        tr.paste_fragment(frag)
                    });
                }
                Err(e) => {
                    error!("Fragment parse error: {:?}", e);
                }
            }
        }

        if let Some(html_str) = html {
            match Fragment::from_html(&html_str) {
                Ok(frag) => {
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
