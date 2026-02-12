use crate::model::*;
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_toggle_bold(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.toggle_bold_style())
    }

    pub(crate) fn handle_toggle_italic(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.toggle_style(Style::Italic(ItalicStyle)))
    }

    pub(crate) fn handle_toggle_strikethrough(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.toggle_style(Style::Strikethrough(StrikethroughStyle)))
    }

    pub(crate) fn handle_toggle_underline(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.toggle_style(Style::Underline(UnderlineStyle)))
    }

    pub(crate) fn handle_toggle_blockquote(
        &mut self,
        variant: crate::model::BlockquoteVariant,
    ) -> Vec<Effect> {
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

    pub(crate) fn handle_set_font_family(&mut self, family: String) -> Vec<Effect> {
        self.transact(|tr| tr.toggle_style(Style::FontFamily(FontFamilyStyle { family })))
    }

    pub(crate) fn handle_set_font_size(&mut self, size: f32) -> Vec<Effect> {
        self.transact(|tr| tr.toggle_style(Style::FontSize(FontSizeStyle { size })))
    }

    pub(crate) fn handle_set_font_weight(&mut self, weight: u16) -> Vec<Effect> {
        self.transact(|tr| tr.toggle_style(Style::FontWeight(FontWeightStyle { weight })))
    }

    pub(crate) fn handle_set_line_height(&mut self, height: f32) -> Vec<Effect> {
        self.transact(|tr| tr.set_line_height(height))
    }

    pub(crate) fn handle_set_letter_spacing(&mut self, spacing: f32) -> Vec<Effect> {
        self.transact(|tr| tr.toggle_style(Style::LetterSpacing(LetterSpacingStyle { spacing })))
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

    pub(crate) fn handle_toggle_text_color(&mut self, key: String) -> Vec<Effect> {
        self.transact(|tr| tr.set_style(Style::TextColor(TextColorStyle { color: key })))
    }

    pub(crate) fn handle_toggle_background_color(&mut self, key: Option<String>) -> Vec<Effect> {
        self.transact(|tr| {
            tr.toggle_style(Style::BackgroundColor(BackgroundColorStyle {
                color: key.unwrap_or_else(|| BackgroundColorStyle::NONE.to_string()),
            }))
        })
    }

    pub(crate) fn handle_insert_horizontal_rule(
        &mut self,
        variant: crate::model::HorizontalRuleVariant,
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
                    let family = FontFamilyStyle::default().family;
                    let weight = FontWeightStyle::default().weight;
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

    pub(crate) fn handle_add_annotation(
        &mut self,
        annotation: crate::model::Annotation,
    ) -> Vec<Effect> {
        self.transact(|tr| {
            let selection = tr.selection().clone();
            if selection.is_collapsed() {
                return Ok(false);
            }
            tr.add_annotation(annotation)?;
            Ok(true)
        })
    }

    pub(crate) fn handle_update_annotation(
        &mut self,
        id: String,
        annotation: crate::model::Annotation,
    ) -> Vec<Effect> {
        self.transact(|tr| {
            let Some(annotation_id) = crate::model::AnnotationId::from_string(&id) else {
                return Ok(false);
            };
            tr.update_annotation(annotation_id, annotation)
        })
    }

    pub(crate) fn handle_remove_annotation(&mut self, id: String) -> Vec<Effect> {
        self.transact(|tr| {
            let Some(annotation_id) = crate::model::AnnotationId::from_string(&id) else {
                return Ok(false);
            };
            tr.remove_annotation(annotation_id)
        })
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
    use crate::model::*;
    use crate::runtime::Message;

    #[test]
    fn test_clear_formatting_collapsed_clears_pending_styles() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert(FontFamilyStyle::default().family, vec![400, 700]);
        let _guard = crate::test_utils::ScopedFontRegistration::new(fonts);

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
        let _guard = crate::test_utils::ScopedFontRegistration::new(fonts);

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

        let snapshot = runtime.selection_snapshot_owned();
        let (uniform, _) = runtime.collect_selection_styles(snapshot);
        assert!(
            uniform
                .iter()
                .any(|s| matches!(s, Style::FontWeight(FontWeightStyle { weight: 700 })))
        );
    }
}
