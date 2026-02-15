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

    pub(crate) fn handle_set_blockquote(&mut self, variant: BlockquoteVariant) -> Vec<Effect> {
        self.transact(|tr| tr.set_blockquote(variant))
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

    pub(crate) fn handle_set_default_attrs(&mut self, attrs: DefaultAttrs) -> Vec<Effect> {
        self.transact(|tr| tr.set_default_attrs(attrs))
    }

    pub(crate) fn handle_insert_horizontal_rule(
        &mut self,
        variant: HorizontalRuleVariant,
    ) -> Vec<Effect> {
        self.transact(|tr| tr.insert_horizontal_rule(variant))
    }

    pub(crate) fn handle_set_horizontal_rule(
        &mut self,
        variant: HorizontalRuleVariant,
    ) -> Vec<Effect> {
        self.transact(|tr| tr.set_horizontal_rule(variant))
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
                    let family = tr.doc().default_attrs().font_family().to_string();
                    let weight = tr.doc().default_attrs().font_weight();
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
            DefaultAttrs::default().font_family().to_string(),
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
    fn test_clear_formatting_collapsed_resets_to_document_defaults_despite_cascade_attrs() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert(
            DefaultAttrs::default().font_family().to_string(),
            vec![400, 700],
        );
        let _guard = ScopedFontRegistration::new(fonts);

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {}
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
            runtime
                .state()
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 400)),
            "expected font_weight: 400 after ClearFormatting, got: {:?}",
            runtime.state().pending_styles
        );
        assert!(
            !runtime
                .state()
                .pending_styles
                .iter()
                .any(|s| matches!(s, Style::FontWeight(fw) if fw.weight == 700)),
        );
    }

    #[test]
    fn test_clear_formatting_fully_selected_resets_cascade_attrs_to_document_defaults() {
        let mut p = id!();

        let mut fonts = std::collections::HashMap::new();
        fonts.insert(
            DefaultAttrs::default().font_family().to_string(),
            vec![400, 700],
        );
        let _guard = ScopedFontRegistration::new(fonts);

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        runtime.update(Message::ToggleBold);
        runtime.update(Message::Input {
            text: "hello".to_string(),
        });

        let node = runtime.state().doc.node(p).unwrap();
        assert!(
            node.cascade_attrs()
                .unwrap_or_default()
                .iter()
                .any(|a| matches!(a, Attr::Style(Style::FontWeight(fw)) if fw.weight == 700)),
            "cascade_attrs should contain font_weight: 700 before clear"
        );

        runtime.layout();
        runtime.update(Message::SelectAll);
        runtime.update(Message::ClearFormatting);

        let node = runtime.state().doc.node(p).unwrap();
        let has_bold = node
            .cascade_attrs()
            .unwrap_or_default()
            .iter()
            .any(|a| matches!(a, Attr::Style(Style::FontWeight(fw)) if fw.weight == 700));
        assert!(
            !has_bold,
            "cascade_attrs should not contain font_weight: 700 after ClearFormatting on fully selected paragraph"
        );
    }

    #[test]
    fn test_clear_formatting_fully_selected_resets_line_height_to_document_default() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph(line_height: 1.0,) {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let custom_defaults = DefaultAttrs::default();
        let custom_line_height: f32 = 2.0;
        let mut attrs: Vec<Attr> = custom_defaults
            .to_attrs()
            .into_iter()
            .filter(|a| !matches!(a, Attr::Paragraph(_)))
            .collect();
        attrs.push(Attr::Paragraph(ParagraphAttr {
            line_height: custom_line_height,
        }));
        let custom_defaults = DefaultAttrs::from_attrs(&attrs);

        runtime.update(Message::SetDefaultAttrs {
            attrs: custom_defaults,
        });

        runtime.update(Message::ClearFormatting);

        let node = runtime.state().doc.node(p).unwrap();
        if let Node::Paragraph(para) = node.node() {
            assert!(
                (para.line_height - custom_line_height).abs() < f32::EPSILON,
                "expected line_height {} (document default) after ClearFormatting, got {}",
                custom_line_height,
                para.line_height
            );
        } else {
            panic!("expected Paragraph node");
        }
    }

    #[test]
    fn test_clear_formatting_resets_empty_paragraph_line_height_to_document_default() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph(line_height: 1.0,) {
                    text { "hello" }
                }
                @p2 paragraph(line_height: 1.0,) {}
            }
            selection { (p1, 0) }
        };

        let custom_line_height: f32 = 2.0;
        let mut attrs: Vec<Attr> = DefaultAttrs::default()
            .to_attrs()
            .into_iter()
            .filter(|a| !matches!(a, Attr::Paragraph(_)))
            .collect();
        attrs.push(Attr::Paragraph(ParagraphAttr {
            line_height: custom_line_height,
        }));
        runtime.update(Message::SetDefaultAttrs {
            attrs: DefaultAttrs::from_attrs(&attrs),
        });

        runtime.layout();
        runtime.update(Message::SelectAll);
        runtime.update(Message::ClearFormatting);

        let node = runtime.state().doc.node(p2).unwrap();
        if let Node::Paragraph(para) = node.node() {
            assert!(
                (para.line_height - custom_line_height).abs() < f32::EPSILON,
                "empty paragraph line_height: expected {}, got {}",
                custom_line_height,
                para.line_height
            );
        } else {
            panic!("expected Paragraph node");
        }
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
