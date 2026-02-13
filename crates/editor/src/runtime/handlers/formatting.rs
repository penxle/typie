use crate::model::*;
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_toggle_bold(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.toggle_bold_style())
    }

    pub(crate) fn handle_toggle_style(&mut self, style: Style) -> Vec<Effect> {
        self.transact(|tr| match &style {
            Style::Italic(_) | Style::Strikethrough(_) | Style::Underline(_) => {
                tr.toggle_style(style)
            }
            _ => tr.set_style(style),
        })
    }

    pub(crate) fn handle_toggle_blockquote(&mut self, variant: BlockquoteVariant) -> Vec<Effect> {
        self.transact(|tr| tr.toggle_blockquote(variant))
    }

    pub(crate) fn handle_toggle_callout(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.toggle_callout())
    }

    pub(crate) fn handle_toggle_bullet_list(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.toggle_bullet_list())
    }

    pub(crate) fn handle_toggle_ordered_list(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.toggle_ordered_list())
    }

    pub(crate) fn handle_set_line_height(&mut self, height: f32) -> Vec<Effect> {
        self.transact(|tr| tr.set_line_height(height))
    }

    pub(crate) fn handle_set_text_align(&mut self, align: TextAlign) -> Vec<Effect> {
        self.transact(|tr| tr.set_text_align(align))
    }

    pub(crate) fn handle_set_block_gap(&mut self, gap: f32) -> Vec<Effect> {
        self.transact(|tr| tr.set_block_gap(gap))
    }

    pub(crate) fn handle_set_paragraph_indent(&mut self, indent: f32) -> Vec<Effect> {
        self.transact(|tr| tr.set_paragraph_indent(indent))
    }

    pub(crate) fn handle_insert_horizontal_rule(
        &mut self,
        variant: HorizontalRuleVariant,
    ) -> Vec<Effect> {
        self.transact(|tr| tr.insert_horizontal_rule(variant))
    }

    pub(crate) fn handle_clear_formatting(&mut self) -> Vec<Effect> {
        self.transact(|tr| {
            let selection = tr.selection().clone();
            if selection.is_collapsed() {
                tr.reset_all_styles()
            } else {
                let mut changed = false;

                if tr.reset_all_styles()? {
                    changed = true;

                    let codepoints = tr.selection_codepoints();
                    let family = tr.doc().default_styles().font_family().to_string();
                    let weight = tr.doc().default_styles().font_weight();
                    tr.push_effect(Effect::FontDetected {
                        family,
                        weight,
                        codepoints,
                    });
                }

                if tr.reset_fully_selected_paragraphs()? {
                    changed = true;
                }

                Ok(changed)
            }
        })
    }

    pub(crate) fn handle_add_annotation(&mut self, annotation: Annotation) -> Vec<Effect> {
        self.transact(|tr| {
            let selection = tr.selection().clone();
            if selection.is_collapsed() {
                return Ok(false);
            }
            tr.add_annotation(annotation)?;
            Ok(true)
        })
    }

    pub(crate) fn handle_update_annotation(&mut self, annotation: Annotation) -> Vec<Effect> {
        let ann_type = annotation.as_type();
        self.transact(|tr| tr.update_annotation(ann_type, annotation))
    }

    pub(crate) fn handle_remove_annotation(
        &mut self,
        annotation_type: AnnotationType,
    ) -> Vec<Effect> {
        self.transact(|tr| tr.remove_annotation(annotation_type))
    }

    pub(crate) fn handle_indent(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.sink_list_item())
    }

    pub(crate) fn handle_outdent(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.lift_list_item())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Message;
    use crate::test_utils::ScopedFontRegistration;

    #[test]
    fn test_clear_formatting_collapsed_clears_pending_styles() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert(
            DefaultStyles::default().font_family().to_string(),
            vec![400, 700],
        );
        let _guard = ScopedFontRegistration::new(fonts);

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) }
        };

        runtime.update(Message::ToggleBold);
        assert!(
            runtime
                .state()
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );

        runtime.update(Message::ClearFormatting);
        assert!(
            !runtime
                .state()
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );
    }

    #[test]
    fn test_clear_formatting_range_clears_styles() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text(styles: [font_weight(700), italic()]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        runtime.update(Message::ClearFormatting);

        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(runtime.state(), expected);
    }

    #[test]
    fn test_clear_formatting_full_paragraph_resets_alignment() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph(align: TextAlign::Center,) {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        runtime.update(Message::ClearFormatting);

        let expected = state! {
            doc {
                @p paragraph(align: TextAlign::Left,) {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(runtime.state(), expected);
    }

    #[test]
    fn test_clear_formatting_partial_paragraph_does_not_reset_alignment() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph(align: TextAlign::Center,) {
                    text { "hello world" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        runtime.update(Message::ClearFormatting);

        let expected = state! {
            doc {
                @p paragraph(align: TextAlign::Center,) {
                    text { "hello world" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(runtime.state(), expected);
    }

    #[test]
    fn test_clear_formatting_range_clears_styles_and_alignment_if_full() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph(align: TextAlign::Right,) {
                    text(styles: [font_weight(700)]) { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        runtime.update(Message::ClearFormatting);

        let expected = state! {
            doc {
                @p paragraph(align: TextAlign::Left,) {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        assert_state_eq!(runtime.state(), expected);
    }

    #[test]
    fn test_shortcut_during_composition_commits_preedit_and_applies_styles() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert("Pretendard".to_string(), vec![400, 700]);
        let _guard = ScopedFontRegistration::new(fonts);

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "start " }
                }
            }
            selection { (p, 6) }
        };

        runtime.update(Message::CompositionUpdate {
            text: "abc".to_string(),
        });
        assert_eq!(runtime.state().preedit.as_ref().unwrap().text, "abc");
        assert!(runtime.state().doc.to_plain_text().ends_with("start "));

        runtime.update(Message::CommitPreedit);
        runtime.update(Message::ToggleBold);

        assert!(runtime.state().doc.to_plain_text().ends_with("start abc"));
        assert!(runtime.state().preedit.is_none());

        assert!(
            runtime
                .state()
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(FontWeightStyle { weight: 700 })))
        );

        runtime.update(Message::Input {
            text: "d".to_string(),
        });
        assert!(runtime.state().doc.to_plain_text().ends_with("start abcd"));

        let doc = &runtime.state().doc;
        let p_node = doc.node(p).unwrap();
        let mut found_bold_d = false;
        for child in p_node.children() {
            if let Node::Text(text_node) = child.node() {
                for seg in text_node.text.get_segments() {
                    if seg.text.contains('d')
                        && seg.styles.iter().any(|s| {
                            matches!(s, Style::FontWeight(FontWeightStyle { weight: 700 }))
                        })
                    {
                        found_bold_d = true;
                    }
                }
            }
        }
        assert!(found_bold_d);
    }
}
