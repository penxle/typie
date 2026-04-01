use crate::model::Fragment;
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub fn handle_paste_html(&mut self, html: String, text: String) -> Vec<Effect> {
        if let Ok(frag) = Fragment::from_html(&html) {
            if !frag.is_empty() {
                return self.transact(|tr| {
                    tr.delete_selection()?;
                    tr.normalize()?;
                    tr.paste_fragment(frag, None)
                });
            }
        }

        self.transact(|tr| {
            tr.delete_selection()?;
            tr.normalize()?;
            tr.paste_text(text)
        })
    }

    pub fn handle_paste_html_as_text(&mut self, html: String, text: String) -> Vec<Effect> {
        let plain = Fragment::from_html(&html)
            .ok()
            .filter(|f| !f.is_empty())
            .map(|f| f.to_plain_text())
            .unwrap_or(text);

        let paragraph_attrs = self
            .state
            .doc
            .node(self.state.selection.head.node_id)
            .and_then(|n| match n.node() {
                Some(crate::model::Node::Paragraph(p)) => Some(p.clone()),
                _ => None,
            });

        self.transact(|tr| {
            tr.delete_selection()?;
            tr.normalize()?;
            let mut fragment = Fragment::from_text(&plain, &tr.pending_styles());
            if let Some(attrs) = paragraph_attrs {
                fragment = fragment.with_paragraph_attrs(attrs);
            }
            tr.paste_text_fragment(fragment)
        })
    }

    pub fn handle_paste_text(&mut self, text: String) -> Vec<Effect> {
        self.transact(|tr| {
            tr.delete_selection()?;
            tr.normalize()?;
            tr.paste_text(text)
        })
    }

    pub fn handle_repaste_as_text(&mut self) -> Vec<Effect> {
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
    use crate::model::{Node, NodeId};
    use crate::runtime::Message;
    use crate::types::Affinity;

    #[test]
    fn paste_html_as_text_preserves_current_paragraph_line_height() {
        let mut p = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph(line_height: 220,) {
                    text { "Hello" }
                }
            }
            selection { (p, 5) }
        };

        let html = r#"<p>AAA</p><p>BBB</p><p>CCC</p>"#;
        rt.update(Message::PasteHtmlAsText {
            html: html.to_string(),
            text: "AAA\nBBB\nCCC".to_string(),
        });

        let doc = &rt.state().doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        for child in root.children() {
            if let Some(Node::Paragraph(para)) = child.node() {
                assert_eq!(
                    para.line_height, 220,
                    "PasteHtmlAsText should preserve current paragraph's line_height (220), got {}",
                    para.line_height
                );
            }
        }
    }

    #[test]
    fn paste_html_as_text_into_empty_paragraph_preserves_line_height() {
        let mut p = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph(line_height: 220,) {}
            }
            selection { (p, 0) }
        };

        let html = r#"<p>AAA</p><p>BBB</p>"#;
        rt.update(Message::PasteHtmlAsText {
            html: html.to_string(),
            text: "AAA\nBBB".to_string(),
        });

        let doc = &rt.state().doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        for child in root.children() {
            if let Some(Node::Paragraph(para)) = child.node() {
                assert_eq!(
                    para.line_height, 220,
                    "PasteHtmlAsText into empty paragraph should preserve line_height (220), got {}",
                    para.line_height
                );
            }
        }
    }

    #[test]
    fn paste_html_as_text_falls_back_to_text_param() {
        let mut p = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        // Invalid HTML that fails to parse
        rt.update(Message::PasteHtmlAsText {
            html: String::new(),
            text: "fallback text".to_string(),
        });

        assert!(rt.state().doc.to_plain_text().contains("fallback text"));
    }

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
