use crate::model::Fragment;
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_paste_html(&mut self, html: String, text: String) -> Vec<Effect> {
        if let Ok(frag) = Fragment::from_html(&html) {
            if !frag.is_empty() {
                return self.transact(|tr| {
                    tr.delete_selection()?;
                    tr.normalize()?;
                    tr.paste_fragment(frag, Some(text))
                });
            }
        }

        self.transact(|tr| {
            tr.delete_selection()?;
            tr.normalize()?;
            tr.paste_text(text)
        })
    }

    pub(crate) fn handle_paste_text(&mut self, text: String) -> Vec<Effect> {
        self.transact(|tr| {
            tr.delete_selection()?;
            tr.normalize()?;
            tr.paste_text(text)
        })
    }

    pub(crate) fn handle_repaste_as_text(&mut self) -> Vec<Effect> {
        let Some((selection, text, styles, paragraph_attrs)) = self.repaste_text.take() else {
            return vec![];
        };

        let Ok((from, to)) = selection.as_sorted(&self.state.doc) else {
            return vec![];
        };

        let fragment = Fragment::from_text(&text, &styles);
        let fragment = if let Some(attrs) = paragraph_attrs {
            fragment.with_paragraph_attrs(attrs)
        } else {
            fragment
        };

        self.transact(|tr| tr.replace_range(from, to, fragment))
    }
}

#[cfg(test)]
mod tests {
    use crate::runtime::Message;
    use crate::types::Affinity;

    #[test]
    fn paste_html_falls_back_to_plain_text_when_fragment_is_empty() {
        let mut p = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        rt.update(Message::PasteHtml {
            html: r#"<meta name="typ-frag" data-open-start="0" data-open-end="0">"#.to_string(),
            text: "fallback".to_string(),
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text { "fallback" }
                }
            }
            selection { (p, 8, Affinity::Upstream) }
        };

        assert_state_eq!(rt.state(), expected);
    }
}
