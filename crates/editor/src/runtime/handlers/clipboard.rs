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
                    web_sys::console::error_1(&format!("Fragment parse error: {:?}", e).into());
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
                    web_sys::console::error_1(&format!("HTML parse error: {:?}", e).into());
                }
            }
        }

        self.transact(|tr| {
            tr.delete_selection()?;
            tr.paste_text(text)
        })
    }
}
