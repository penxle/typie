use crate::model::*;
use crate::runtime::{Effect, Runtime};

impl Runtime {
    pub(crate) fn handle_toggle_bold(&mut self) -> Vec<Effect> {
        self.transact(|tr| tr.toggle_bold_style())
    }

    pub(crate) fn handle_toggle_style(&mut self, style: Style) -> Vec<Effect> {
        self.transact(|tr| match &style {
            Style::Bold(_) | Style::Italic(_) | Style::Strikethrough(_) | Style::Underline(_) => {
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

    pub(crate) fn handle_set_line_height(&mut self, height: u32) -> Vec<Effect> {
        self.transact(|tr| tr.set_line_height(height))
    }

    pub(crate) fn handle_set_text_align(&mut self, align: TextAlign) -> Vec<Effect> {
        self.transact(|tr| tr.set_text_align(align))
    }

    pub(crate) fn handle_set_block_gap(&mut self, gap: u32) -> Vec<Effect> {
        self.transact(|tr| tr.set_block_gap(gap))
    }

    pub(crate) fn handle_set_paragraph_indent(&mut self, indent: u32) -> Vec<Effect> {
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
                @p paragraph(line_height: 100,) {
                    text { "hello" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let custom_defaults = DefaultAttrs::default();
        let custom_line_height: u32 = 200;
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
        if let Some(Node::Paragraph(para)) = node.node() {
            assert_eq!(
                para.line_height, custom_line_height,
                "expected line_height {} (document default) after ClearFormatting, got {}",
                custom_line_height, para.line_height
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
                @p1 paragraph(line_height: 100,) {
                    text { "hello" }
                }
                @p2 paragraph(line_height: 100,) {}
            }
            selection { (p1, 0) }
        };

        let custom_line_height: u32 = 200;
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
        if let Some(Node::Paragraph(para)) = node.node() {
            assert_eq!(
                para.line_height, custom_line_height,
                "empty paragraph line_height: expected {}, got {}",
                custom_line_height, para.line_height
            );
        } else {
            panic!("expected Paragraph node");
        }
    }

    #[test]
    fn test_clear_formatting_range_in_fold_title_does_not_apply_disallowed_styles() {
        let mut ft = id!();

        let initial = state! {
            doc {
                fold {
                    @ft fold_title { text { "title" } }
                    fold_content {
                        paragraph { text { "content" } }
                    }
                }
            }
            selection { (ft, 0) -> (ft, 5) }
        };

        let actual = transact!(initial.clone(), |tr| {
            tr.reset_all_styles()
                .expect("reset_all_styles should not error for FoldTitle");
        });

        assert_state_eq!(actual, initial);
    }

    #[test]
    fn test_clear_formatting_collapsed_in_fold_title_does_not_set_disallowed_pending_styles() {
        let mut ft = id!();

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                fold {
                    @ft fold_title { text { "title" } }
                    fold_content {
                        paragraph { text { "content" } }
                    }
                }
            }
            selection { (ft, 0) }
        };

        runtime.update(Message::ClearFormatting);

        assert!(
            runtime.state().pending_styles.is_empty(),
            "FoldTitle should not have any pending styles after ClearFormatting, got: {:?}",
            runtime.state().pending_styles
        );
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
            if let Some(Node::Text(text_node)) = child.node() {
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

    #[test]
    fn test_update_annotation_collapsed_updates_only_cursor_annotation() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph {
                    text { "hello " , "world" @[link("http://a.com")] }
                }
                @p2 paragraph {
                    text { "foo " , "bar" @[link("http://b.com")] }
                }
            }
            selection { (p1, 7) }
        };

        // Cursor is inside "world" link (offset 7 = 'o' of "world")
        runtime.update(Message::UpdateAnnotation {
            annotation: Annotation::Link(LinkAnnotation {
                href: "http://updated.com".to_string(),
            }),
        });

        // p1's link annotation should be updated
        let p1_node = runtime.state().doc.node(p1).unwrap();
        let mut found_updated = false;
        for child in p1_node.children() {
            if let Some(Node::Text(text_node)) = child.node() {
                for seg in text_node.text.get_segments() {
                    for ann in &seg.annotations {
                        if let Annotation::Link(link) = ann {
                            assert_eq!(link.href, "http://updated.com");
                            found_updated = true;
                        }
                    }
                }
            }
        }
        assert!(
            found_updated,
            "p1's link should be updated to http://updated.com"
        );

        // p2's link annotation should remain unchanged
        let p2_node = runtime.state().doc.node(p2).unwrap();
        let mut found_original = false;
        for child in p2_node.children() {
            if let Some(Node::Text(text_node)) = child.node() {
                for seg in text_node.text.get_segments() {
                    for ann in &seg.annotations {
                        if let Annotation::Link(link) = ann {
                            assert_eq!(
                                link.href, "http://b.com",
                                "p2's link should remain http://b.com, but got {}",
                                link.href
                            );
                            found_original = true;
                        }
                    }
                }
            }
        }
        assert!(
            found_original,
            "p2 should still have its original link annotation"
        );
    }

    #[test]
    fn test_add_annotation_rectangular_selection_applies_only_selected_cells() {
        let mut p11 = id!();
        let mut p12 = id!();
        let mut p21 = id!();
        let mut p22 = id!();

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                table {
                    table_row {
                        table_cell { @p11 paragraph { text { "a" } } }
                        table_cell { @p12 paragraph { text { "b" } } }
                    }
                    table_row {
                        table_cell { @p21 paragraph { text { "c" } } }
                        table_cell { @p22 paragraph { text { "d" } } }
                    }
                }
            }
            selection { (p11, 0) -> (p21, 1) }
        };

        runtime.update(Message::AddAnnotation {
            annotation: Annotation::Link(LinkAnnotation {
                href: "https://example.com".to_string(),
            }),
        });

        let has_link = |runtime: &Runtime, para_id: NodeId| -> bool {
            let para = runtime.state().doc.node(para_id).unwrap();
            para.children().any(|child| {
                if let Some(Node::Text(text_node)) = child.node() {
                    text_node.text.get_segments().iter().any(|seg| {
                        seg.annotations
                            .iter()
                            .any(|ann| matches!(ann, Annotation::Link(_)))
                    })
                } else {
                    false
                }
            })
        };

        assert!(
            has_link(&runtime, p11),
            "selected cell p11 should have link"
        );
        assert!(
            has_link(&runtime, p21),
            "selected cell p21 should have link"
        );
        assert!(
            !has_link(&runtime, p12),
            "unselected cell p12 should not have link"
        );
        assert!(
            !has_link(&runtime, p22),
            "unselected cell p22 should not have link"
        );
    }

    #[test]
    fn test_remove_annotation_collapsed_removes_only_cursor_annotation() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph {
                    text { "hello " , "world" @[link("http://a.com")] }
                }
                @p2 paragraph {
                    text { "foo " , "bar" @[link("http://b.com")] }
                }
            }
            selection { (p1, 7) }
        };

        // Cursor is inside "world" link (offset 7 = 'o' of "world")
        runtime.update(Message::RemoveAnnotation {
            annotation_type: AnnotationType::Link,
        });

        // p1's link annotation should be removed
        let p1_node = runtime.state().doc.node(p1).unwrap();
        for child in p1_node.children() {
            if let Some(Node::Text(text_node)) = child.node() {
                for seg in text_node.text.get_segments() {
                    assert!(
                        seg.annotations.is_empty(),
                        "p1 should have no annotations after RemoveAnnotation, but found: {:?}",
                        seg.annotations
                    );
                }
            }
        }

        // p2's link annotation should still be present
        let p2_node = runtime.state().doc.node(p2).unwrap();
        let mut found_link = false;
        for child in p2_node.children() {
            if let Some(Node::Text(text_node)) = child.node() {
                for seg in text_node.text.get_segments() {
                    if seg
                        .annotations
                        .iter()
                        .any(|a| a.as_type() == AnnotationType::Link)
                    {
                        found_link = true;
                    }
                }
            }
        }
        assert!(
            found_link,
            "p2 should still have link annotation, but it was removed"
        );
    }

    #[test]
    fn test_remove_annotation_collapsed_no_annotation_at_cursor_does_nothing() {
        let mut p = id!();

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "hello " , "world" @[link("http://a.com")] }
                }
            }
            selection { (p, 2) }
        };

        // Cursor at offset 2 = 'l' of "hello ", no annotation here
        runtime.update(Message::RemoveAnnotation {
            annotation_type: AnnotationType::Link,
        });

        // Link on "world" should still be present
        let p_node = runtime.state().doc.node(p).unwrap();
        let mut found_link = false;
        for child in p_node.children() {
            if let Some(Node::Text(text_node)) = child.node() {
                for seg in text_node.text.get_segments() {
                    if seg
                        .annotations
                        .iter()
                        .any(|a| a.as_type() == AnnotationType::Link)
                    {
                        found_link = true;
                    }
                }
            }
        }
        assert!(
            found_link,
            "link annotation should remain when cursor is outside it"
        );
    }

    #[test]
    fn test_remove_annotation_collapsed_two_links_in_same_paragraph_removes_only_cursor_one() {
        let mut p = id!();

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "aaa" @[link("http://first.com")], " ", "bbb" @[link("http://second.com")] }
                }
            }
            selection { (p, 5) }
        };

        // "aaa" = offsets 0..3, " " = offset 3, "bbb" = offsets 4..7
        // Cursor at offset 5 = second 'b' of "bbb" (inside second link)
        runtime.update(Message::RemoveAnnotation {
            annotation_type: AnnotationType::Link,
        });

        let p_node = runtime.state().doc.node(p).unwrap();
        let mut first_link_present = false;
        let mut second_link_present = false;
        for child in p_node.children() {
            if let Some(Node::Text(text_node)) = child.node() {
                for seg in text_node.text.get_segments() {
                    for ann in &seg.annotations {
                        if let Annotation::Link(link) = ann {
                            if link.href == "http://first.com" {
                                first_link_present = true;
                            }
                            if link.href == "http://second.com" {
                                second_link_present = true;
                            }
                        }
                    }
                }
            }
        }
        assert!(first_link_present, "first link should remain");
        assert!(
            !second_link_present,
            "second link at cursor should be removed"
        );
    }

    #[test]
    fn test_remove_annotation_collapsed_at_annotation_start_boundary() {
        let mut p = id!();

        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph {
                    text { "hello " , "world" @[link("http://a.com")] }
                }
            }
            selection { (p, 6) }
        };

        // Cursor at offset 6 = 'w' of "world" (start of annotation)
        runtime.update(Message::RemoveAnnotation {
            annotation_type: AnnotationType::Link,
        });

        let p_node = runtime.state().doc.node(p).unwrap();
        for child in p_node.children() {
            if let Some(Node::Text(text_node)) = child.node() {
                for seg in text_node.text.get_segments() {
                    assert!(
                        seg.annotations.is_empty(),
                        "annotation should be removed when cursor is at its start boundary"
                    );
                }
            }
        }
    }
}
