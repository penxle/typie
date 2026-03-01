use crate::model::Fragment;
use crate::runtime::Effect;
use crate::state::{Position, Selection};
use crate::transaction::Transaction;
use crate::types::Affinity;
use anyhow::Result;
use std::borrow::Cow;

fn normalize_line_endings(input: &str) -> Cow<'_, str> {
    let needs_normalization = input.as_bytes().contains(&b'\r')
        || input.contains('\u{2028}') // Line Separator
        || input.contains('\u{2029}') // Paragraph Separator
        || input.contains('\u{0085}'); // Next Line (NEL)

    if !needs_normalization {
        return Cow::Borrowed(input);
    }

    let mut normalized = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if matches!(chars.peek(), Some('\n')) {
                    chars.next();
                }
                normalized.push('\n');
            }
            '\u{2028}' | '\u{2029}' | '\u{0085}' => normalized.push('\n'),
            _ => normalized.push(ch),
        }
    }

    Cow::Owned(normalized)
}

impl Transaction {
    pub fn paste_text(&mut self, s: String) -> Result<bool> {
        if s.is_empty() {
            return Ok(false);
        }

        if !self.selection().is_collapsed() {
            return Ok(false);
        }

        let normalized = normalize_line_endings(&s);
        let selection = self.selection().head;
        let fragment = Fragment::from_text(normalized.as_ref(), &self.state.pending_styles);
        let result = self.insert_fragment(selection, fragment)?;
        if let Some(selection) = result.as_selection() {
            let selection = if selection.is_collapsed() {
                let head = selection.head;
                Selection::collapsed(Position::new(head.node_id, head.offset, Affinity::Upstream))
            } else {
                selection
            };
            self.set_selection(selection);
        }

        Ok(result.inserted())
    }

    pub fn paste_fragment(&mut self, fragment: Fragment, text: Option<String>) -> Result<bool> {
        if fragment.is_empty() {
            return Ok(false);
        }

        let styles = self.state.pending_styles.clone();
        let fill_styles: Vec<_> = styles
            .iter()
            .filter(|s| {
                !matches!(
                    s,
                    crate::model::Style::Bold(_)
                        | crate::model::Style::Italic(_)
                        | crate::model::Style::Strikethrough(_)
                        | crate::model::Style::Underline(_)
                )
            })
            .cloned()
            .collect();

        // Save the current paragraph's attrs before insert_fragment may overwrite them
        let paragraph_attrs = self
            .doc()
            .node(self.selection().head.node_id)
            .and_then(|n| match n.node() {
                Some(crate::model::Node::Paragraph(p)) => Some(p.clone()),
                _ => None,
            });

        let fragment = fragment
            .with_fresh_ids_for_doc(self.doc())
            .fill_missing_styles(&fill_styles);
        let result = self.insert_fragment(self.selection().head, fragment)?;
        if let Some(selection) = result.as_selection() {
            self.set_selection(selection);
        }

        if let Some(text) = text.filter(|t| !t.is_empty()) {
            if let Some(selection) = result
                .as_inline_range_selection(self.doc())
                .or_else(|| result.as_range_selection())
            {
                self.push_effect(Effect::HtmlPasted {
                    selection,
                    text,
                    styles,
                    paragraph_attrs,
                });
            }
        }

        Ok(result.inserted())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Element, LayoutNode};
    use crate::model::{Node, NodeId, TableBorderStyle, TextAlign};
    use crate::runtime::Message;
    use crate::runtime::slate::DIRTY_RENDER_REQUIRED;
    use crate::types::Affinity;

    #[test]
    fn paste_text_keeps_following_paragraphs() {
        let mut p = id!();

        let initial = state! {
            doc {
                paragraph {
                    text { "Hello" }
                }
                @p paragraph {
                    text { "World" }
                }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.paste_text("Foo".to_string()).unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "Hello" }
                }
                @p paragraph {
                    text { "FooWorld" }
                }
            }
            selection { (p, 3, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_text_handles_multiple_lines() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "World" }
                }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr
            .paste_text("Hello\nBar".to_string())
            .unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "Hello" }
                }
                @p paragraph {
                    text { "BarWorld" }
                }
            }
            selection { (p, 3, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_text_normalizes_crlf_line_endings() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.paste_text("A\r\nB".to_string()).unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "A" }
                }
                @p paragraph {
                    text { "B" }
                }
            }
            selection { (p, 1, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_text_normalizes_cr_only_line_endings() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.paste_text("A\rB".to_string()).unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "A" }
                }
                @p paragraph {
                    text { "B" }
                }
            }
            selection { (p, 1, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_text_normalizes_unicode_line_separators() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr
            .paste_text("A\u{2028}B\u{2029}C\u{0085}D".to_string())
            .unwrap());

        let expected = state! {
            doc {
                paragraph { text { "A" } }
                paragraph { text { "B" } }
                paragraph { text { "C" } }
                @p paragraph { text { "D" } }
            }
            selection { (p, 1, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_text_middle_of_paragraph_splits_correctly() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "StartEnd" }
                }
            }
            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .paste_text("Line1\nLine2".to_string())
            .unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "StartLine1" }
                }
                @p paragraph {
                    text { "Line2End" }
                }
            }
            selection { (p, 5, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_text_with_empty_lines() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "StartEnd" }
                }
            }
            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .paste_text("Line1\n\nLine2".to_string())
            .unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "StartLine1" }
                }
                paragraph {}
                @p paragraph {
                    text { "Line2End" }
                }
            }
            selection { (p, 5, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_text_at_end_of_paragraph() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "Start" }
                }
            }
            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr
            .paste_text("Line1\nLine2".to_string())
            .unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "StartLine1" }
                }
                @p paragraph {
                    text { "Line2" }
                }
            }
            selection { (p, 5, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_text_at_start_of_paragraph() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "End" }
                }
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr
            .paste_text("Line1\nLine2".to_string())
            .unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "Line1" }
                }
                @p paragraph {
                    text { "Line2End" }
                }
            }
            selection { (p, 5, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_text_into_empty_paragraph() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr
            .paste_text("Line1\nLine2".to_string())
            .unwrap());

        let expected = state! {
            doc {
                paragraph {
                    text { "Line1" }
                }
                @p paragraph {
                    text { "Line2" }
                }
            }
            selection { (p, 5, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }
    #[test]
    fn paste_single_line_text_into_empty_paragraph() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let actual = transact!(initial, |tr| tr.paste_text("Hello".to_string()).unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "Hello" }
                }
            }
            selection { (p, 5, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_single_line_at_end_of_text() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "Hello" }
                }
            }
            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.paste_text("World".to_string()).unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "HelloWorld" }
                }
            }
            selection { (p, 10, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }
    #[test]
    fn paste_single_line_in_middle_of_text() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "StartEnd" }
                }
            }
            selection { (p, 5) }
        };

        let actual = transact!(initial, |tr| tr.paste_text("Middle".to_string()).unwrap());

        let expected = state! {
            doc {
                @p paragraph {
                    text { "StartMiddleEnd" }
                }
            }
            selection { (p, 11, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn test_paste_single_line_rendering() {
        let mut p = id!();
        let mut runtime = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph { }
            }
            selection { (p, 0) }
        };

        runtime.layout();

        runtime.update(Message::PasteText {
            text: "Hello".to_string(),
        });

        runtime.tick();

        assert!(
            runtime.slate.dirty & DIRTY_RENDER_REQUIRED != 0,
            "Should emit RenderRequired"
        );

        let page = &runtime.pages()[0];
        let root = &page.root;

        fn find_text_in_layout(node: &LayoutNode, target: &str) -> bool {
            if let Some(Element::Line(line)) = &node.element {
                if line.text.contains(target) {
                    return true;
                }
            }
            if let Some(children) = &node.children {
                for child in children {
                    if find_text_in_layout(&child.node, target) {
                        return true;
                    }
                }
            }
            false
        }

        assert!(
            find_text_in_layout(&root.node, "Hello"),
            "Layout should contain pasted text 'Hello'"
        );
    }

    #[test]
    fn paste_fragment_with_hard_break() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "AB" }
                }
            }
            selection { (p, 1) }
        };

        let fragment = fragment! {
            open_start: 1, open_end: 1,
            paragraph {
                text { "X" }
                hard_break {}
                text { "Y" }
            }
        };

        let actual = transact!(initial, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text { "AX" }
                    hard_break {}
                    text { "YB" }
                }
            }
            selection { (p, 4) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_fragment_with_hard_break_in_middle() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "Hello" }
                    hard_break {}
                    text { "World" }
                }
            }
            selection { (p, 3) }
        };

        let fragment = fragment! {
            open_start: 1, open_end: 1,
            paragraph {
                text { "X" }
            }
        };

        let actual = transact!(initial, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text { "HelXlo" }
                    hard_break {}
                    text { "World" }
                }
            }
            selection { (p, 4) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_multiple_paragraphs() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "AB" }
                }
            }
            selection { (p, 1) }
        };

        let fragment = fragment! {
            open_start: 1, open_end: 1,
            paragraph {
                text { "First" }
            }
            paragraph {
                text { "Second" }
            }
        };

        let actual = transact!(initial, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let expected = state! {
            doc {
                paragraph {
                    text { "AFirst" }
                }
                @p paragraph {
                    text { "SecondB" }
                }
            }
            selection { (p, 6) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_multiple_paragraphs_preserves_all() {
        let mut p1 = id!();
        let mut p2 = id!();
        let mut p3 = id!();

        let initial = state! {
            doc {
                @p1 paragraph {
                    text { "First" }
                }
                @p2 paragraph {
                    text { "Second" }
                }
                @p3 paragraph {
                    text { "Target" }
                }
            }
            selection { (p1, 0) -> (p2, 6) }
        };

        let fragment = initial.selection.extract_fragment(&initial.doc).unwrap();

        let state_after_collapse = state! {
            doc {
                @p1 paragraph {
                    text { "First" }
                }
                @p2 paragraph {
                    text { "Second" }
                }
                @p3 paragraph {
                    text { "Target" }
                }
            }
            selection { (p3, 0) }
        };

        let actual = transact!(state_after_collapse, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let expected = state! {
            doc {
                paragraph {
                    text { "First" }
                }
                paragraph {
                    text { "Second" }
                }
                paragraph {
                    text { "First" }
                }
                @p3 paragraph {
                    text { "SecondTarget" }
                }
            }
            selection { (p3, 6) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_hr_into_empty_paragraph_after_hr() {
        let mut p = id!();

        let initial = state! {
            doc {
                horizontal_rule {}
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let fragment = fragment! {
            open_start: 0, open_end: 0,
            horizontal_rule {}
        };

        let actual = transact!(initial, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let expected = state! {
            doc {
                horizontal_rule {}
                horizontal_rule {}
                paragraph {}
            }
            selection { (NodeId::ROOT, 1) -> (NodeId::ROOT, 2, Affinity::Upstream) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_fragment_with_horizontal_rule() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "AB" }
                }
            }
            selection { (p, 1) }
        };

        let fragment = fragment! {
            open_start: 0, open_end: 0,
            horizontal_rule {}
        };

        let actual = transact!(initial, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let root = actual.doc.node(NodeId::ROOT).expect("root");
        let children: Vec<_> = root.children().collect();
        assert_eq!(children.len(), 3, "should have 3 children");
        assert!(matches!(children[0].node(), Some(Node::Paragraph(_))));
        assert!(matches!(children[1].node(), Some(Node::HorizontalRule(_))));
        assert!(matches!(children[2].node(), Some(Node::Paragraph(_))));

        let first_text = children[0]
            .first_child()
            .expect("first para should have child");
        if let Some(Node::Text(t)) = first_text.node() {
            assert_eq!(t.text.to_string(), "A");
        }

        let last_text = children[2]
            .first_child()
            .expect("last para should have child");
        if let Some(Node::Text(t)) = last_text.node() {
            assert_eq!(t.text.to_string(), "B");
        }
    }

    #[test]
    fn paste_fragment_with_hr_between_paragraphs() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "AB" }
                }
            }
            selection { (p, 1) }
        };

        let fragment = fragment! {
            open_start: 1, open_end: 1,
            paragraph {
                text { "First" }
            }
            horizontal_rule {}
            paragraph {
                text { "Second" }
            }
        };

        let actual = transact!(initial, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let expected = state! {
            doc {
                paragraph {
                    text { "AFirst" }
                }
                horizontal_rule {}
                @p paragraph {
                    text { "SecondB" }
                }
            }
            selection { (p, 6) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_text_and_hr_selection() {
        let mut n1 = id!();

        let initial = state! {
            doc {
                @n1 paragraph {
                    text { "Hello world" }
                }
                horizontal_rule {}
                paragraph {
                    text { "After" }
                }
            }
            selection { (n1, 6) -> (NodeId::ROOT, 2, Affinity::Upstream) }
        };

        let fragment = initial.selection.extract_fragment(&initial.doc).unwrap();

        let paste_target = state! {
            doc {
                @n1 paragraph {
                    text { "Target" }
                }
            }
            selection { (n1, 3) }
        };

        let actual = transact!(paste_target, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let expected = state! {
            doc {
                paragraph {
                    text { "Tarworld" }
                }
                horizontal_rule {}
                @n1 paragraph {
                    text { "get" }
                }
            }
            selection { (n1, 3) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_text_preserves_styles() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "Hello " }
                    text(styles: [font_weight(700)]) { "World" }
                }
            }
            selection { (p, 0) -> (p, 11) }
        };

        let fragment = initial.selection.extract_fragment(&initial.doc).unwrap();

        let paste_target = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let actual = transact!(paste_target, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "Hello ",
                        "World" => [font_weight(700)]
                    }
                }
            }
            selection { (p, 11) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_text_preserves_styles_through_json() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "Hello " }
                    text(styles: [font_weight(700)]) { "World" }
                }
            }
            selection { (p, 0) -> (p, 11) }
        };

        let fragment = initial.selection.extract_fragment(&initial.doc).unwrap();
        let json = fragment.to_json().unwrap();
        let restored_fragment = Fragment::from_json(&json).unwrap();

        let paste_target = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let actual = transact!(paste_target, |tr| {
            tr.paste_fragment(restored_fragment, None).unwrap();
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "Hello ",
                        "World" => [font_weight(700)]
                    }
                }
            }
            selection { (p, 11) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_text_preserves_embolden_style_through_html_roundtrip() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text(styles: [bold()]) { "World" }
                }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let fragment = initial.selection.extract_fragment(&initial.doc).unwrap();
        let html = fragment.to_html();
        let restored_fragment = Fragment::from_html(&html).unwrap();

        let paste_target = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let actual = transact!(paste_target, |tr| {
            tr.paste_fragment(restored_fragment, None).unwrap();
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [bold()]) { "World" }
                }
            }
            selection { (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_blockquote_with_paragraph() {
        let mut bq_p = id!();
        let mut p2 = id!();
        let mut target = id!();

        let initial = state! {
            doc {
                blockquote {
                    @bq_p paragraph { text { "AA" } }
                }
                @p2 paragraph { text { "BB" } }
            }
            selection { (bq_p, 0) -> (p2, 2) }
        };

        let fragment = initial.selection.extract_fragment(&initial.doc).unwrap();

        let has_blockquote = fragment
            .iter()
            .any(|(_, n)| matches!(n.data(), Node::Blockquote(_)));
        assert!(has_blockquote, "Fragment should contain blockquote");

        let top_levels = fragment.top_level_node_ids();
        let first_top = fragment.node(top_levels[0]).unwrap();
        assert!(
            matches!(first_top.data(), Node::Blockquote(_)),
            "First top-level should be Blockquote, got {:?}",
            first_top.data().as_type()
        );

        let paste_target = state! {
            doc {
                @target paragraph { text { "Target" } }
            }
            selection { (target, 3) }
        };

        let actual = transact!(paste_target, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let expected = state! {
            doc {
                paragraph { text { "Tar" } }
                blockquote {
                    @bq_p paragraph { text { "AA" } }
                }
                @target paragraph { text { "BBget" } }
            }
            selection { (target, 2) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_blockquote_into_empty_paragraph() {
        let mut bq_p = id!();
        let mut p2 = id!();
        let mut target = id!();

        let initial = state! {
            doc {
                blockquote {
                    @bq_p paragraph { text { "AA" } }
                }
                @p2 paragraph { text { "BB" } }
            }
            selection { (bq_p, 0) -> (p2, 2) }
        };

        let fragment = initial.selection.extract_fragment(&initial.doc).unwrap();

        let paste_target = state! {
            doc {
                @target paragraph {}
            }
            selection { (target, 0) }
        };

        let actual = transact!(paste_target, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let expected = state! {
            doc {
                blockquote {
                    @bq_p paragraph { text { "AA" } }
                }
                @target paragraph { text { "BB" } }
            }
            selection { (target, 2) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_text_emits_codepoints_detected() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let (_, effects) = transact_with_effect!(initial, |tr| tr
            .paste_text("Hello 안녕 こんにちは 你好".to_string())
            .unwrap());

        let codepoints: Vec<u32> = effects
            .iter()
            .filter_map(|e| match e {
                Effect::FontDetected { codepoints, .. } => Some(codepoints.clone()),
                _ => None,
            })
            .flatten()
            .collect();

        assert!(
            codepoints.contains(&('H' as u32)),
            "paste_text should detect Latin codepoints"
        );
        assert!(
            codepoints.contains(&('안' as u32)),
            "paste_text should detect Korean codepoints"
        );
        assert!(
            codepoints.contains(&('こ' as u32)),
            "paste_text should detect Japanese codepoints"
        );
        assert!(
            codepoints.contains(&('你' as u32)),
            "paste_text should detect Chinese codepoints"
        );
    }

    #[test]
    fn paste_fragment_emits_codepoints_detected() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {
                    text { "Hello 안녕 こんにちは 你好" }
                }
            }
            selection { (p, 0) -> (p, 18) }
        };

        let fragment = initial.selection.extract_fragment(&initial.doc).unwrap();

        let paste_target = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let (_, effects) = transact_with_effect!(paste_target, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let codepoints: Vec<u32> = effects
            .iter()
            .filter_map(|e| match e {
                Effect::FontDetected { codepoints, .. } => Some(codepoints.clone()),
                _ => None,
            })
            .flatten()
            .collect();

        assert!(
            codepoints.contains(&('H' as u32)),
            "paste_fragment should detect Latin codepoints"
        );
        assert!(
            codepoints.contains(&('안' as u32)),
            "paste_fragment should detect Korean codepoints"
        );
        assert!(
            codepoints.contains(&('こ' as u32)),
            "paste_fragment should detect Japanese codepoints"
        );
        assert!(
            codepoints.contains(&('你' as u32)),
            "paste_fragment should detect Chinese codepoints"
        );
    }

    #[test]
    fn paste_fragment_preserves_ids_when_no_conflict() {
        let mut target_p = id!();

        let mut frag_p = id!();
        let fragment = fragment! {
            open_start: 0, open_end: 0,
            @frag_p paragraph { text { "Pasted" } }
        };

        let initial = state! {
            doc {
                @target_p paragraph { text { "Target" } }
            }
            selection { (target_p, 0) }
        };

        let actual = transact!(initial, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        assert!(
            actual.doc.node(frag_p).is_some(),
            "Non-conflicting pasted node ID should be preserved in the document"
        );
    }

    #[test]
    fn paste_fragment_remaps_ids_when_conflict_exists() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph { text { "Hello" } }
            }
            selection { (p, 0) -> (p, 5) }
        };

        let fragment = initial.selection.extract_fragment(&initial.doc).unwrap();
        let fragment_ids: Vec<NodeId> = fragment.collect_all_ids().into_iter().collect();

        let paste_target = state! {
            doc {
                @p paragraph { text { "Hello" } }
            }
            selection { (p, 5) }
        };

        let actual = transact!(paste_target, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        assert!(
            actual.doc.node(p).is_some(),
            "Original document node should still exist"
        );

        let doc_ids: Vec<NodeId> = actual
            .doc
            .node(NodeId::ROOT)
            .unwrap()
            .descendants()
            .map(|n| n.node_id())
            .collect();

        for fid in &fragment_ids {
            if initial.doc.node(*fid).is_some() {
                let count = doc_ids.iter().filter(|id| *id == fid).count();
                assert!(
                    count <= 1,
                    "Conflicting ID {fid:?} should not be duplicated (found {count} times)"
                );
            }
        }
    }

    #[test]
    fn select_all_then_paste_text() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p1 paragraph { text { "Hello" } }
                @p2 paragraph { text { "World" } }
            }
            selection { (p1, 0) }
        };

        rt.layout();
        rt.update(Message::SelectAll);
        rt.update(Message::PasteText {
            text: "New text".to_string(),
        });

        let doc = &rt.state().doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        let children: Vec<_> = root.children().collect();
        assert_eq!(children.len(), 1, "should have exactly one paragraph");

        let para = &children[0];
        assert!(
            matches!(para.node(), Some(Node::Paragraph(_))),
            "child should be a paragraph"
        );

        let text_child = para.first_child();
        assert!(text_child.is_some(), "paragraph should have a text child");

        if let Some(Node::Text(t)) = text_child.unwrap().node() {
            assert_eq!(t.text.to_string(), "New text");
        } else {
            panic!("Expected text node");
        }
    }

    #[test]
    fn select_all_then_paste_multiline_text() {
        let mut p = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph { text { "Hello" } }
            }
            selection { (p, 0) }
        };

        rt.layout();
        rt.update(Message::SelectAll);
        rt.update(Message::PasteText {
            text: "Line1\nLine2\nLine3".to_string(),
        });

        let doc = &rt.state().doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        let children: Vec<_> = root.children().collect();
        assert_eq!(children.len(), 3);

        let texts: Vec<String> = children
            .iter()
            .filter_map(|child| {
                child.first_child().and_then(|tc| {
                    if let Some(Node::Text(t)) = tc.node() {
                        Some(t.text.to_string())
                    } else {
                        None
                    }
                })
            })
            .collect();

        assert_eq!(texts, vec!["Line1", "Line2", "Line3"]);
    }

    #[test]
    fn paste_non_paragraph_fragment_at_empty_paragraph() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let fragment = fragment! {
            open_start: 2, open_end: 0,

            blockquote {
                paragraph {
                    text { "asd" }
                }
            }
            fold {
                fold_title {}
                fold_content {
                    paragraph {
                        text { "asdasd" }
                    }
                }
            }
        };

        let _ = transact!(initial, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });
    }

    #[test]
    fn paste_open_fold_content_fragment_with_table() {
        let mut target = id!();

        let initial = state! {
            doc {
                fold {
                    fold_title {}
                    fold_content {
                        @target paragraph {}
                    }
                }
            }
            selection { (target, 0) }
        };

        let mut source_para = id!();
        let mut source_content = id!();

        let source = state! {
            doc {
                fold {
                    fold_title {}
                    @source_content fold_content {
                        @source_para paragraph {
                            text { "outer" }
                        }
                        table(border_style: TableBorderStyle::Solid, proportion: 1.0,) {
                            table_row {
                                table_cell {
                                    horizontal_rule {}
                                    fold {
                                        fold_title {}
                                        fold_content {
                                            paragraph {
                                                text { "inner" }
                                            }
                                        }
                                    }
                                }
                                table_cell {
                                    paragraph {}
                                }
                            }
                            table_row {
                                table_cell {
                                    paragraph {}
                                }
                                table_cell {
                                    paragraph {}
                                }
                            }
                        }
                    }
                }
            }
            selection { (source_para, 0) -> (source_content, 2, Affinity::Upstream) }
        };

        let fragment = source.selection.extract_fragment(&source.doc).unwrap();
        assert_eq!(fragment.open_start(), 1);
        assert_eq!(fragment.open_end(), 0);

        let actual = transact!(initial, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let root = actual.doc.node(NodeId::ROOT).expect("root");
        let has_table = root
            .descendants()
            .any(|n| matches!(n.node(), Some(Node::Table(_))));
        let has_outer_text = root
            .descendants()
            .any(|n| matches!(n.node(), Some(Node::Text(t)) if t.text.as_str() == "outer"));

        assert!(has_table, "pasted fragment should contain a table");
        assert!(
            has_outer_text,
            "pasted fragment should contain outer paragraph text"
        );
    }

    #[test]
    fn paste_open_table_cell_fragment_into_fold_content() {
        let mut target = id!();

        let initial = state! {
            doc {
                fold {
                    fold_title {}
                    fold_content {
                        @target paragraph {}
                    }
                }
            }
            selection { (target, 0) }
        };

        let mut source_para = id!();
        let mut source_cell = id!();

        let source = state! {
            doc {
                table(border_style: TableBorderStyle::Solid, proportion: 1.0,) {
                    table_row {
                        @source_cell table_cell {
                            @source_para paragraph {
                                text { "cell-start" }
                            }
                            fold {
                                fold_title {}
                                fold_content {
                                    paragraph {
                                        text { "inner-fold" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            selection { (source_para, 0) -> (source_cell, 2, Affinity::Upstream) }
        };

        let fragment = source.selection.extract_fragment(&source.doc).unwrap();
        assert_eq!(fragment.open_start(), 1);
        assert_eq!(fragment.open_end(), 0);

        let actual = transact!(initial, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let root = actual.doc.node(NodeId::ROOT).expect("root");
        let has_fold = root
            .descendants()
            .any(|n| matches!(n.node(), Some(Node::Fold(_))));
        let has_source_text = root
            .descendants()
            .any(|n| matches!(n.node(), Some(Node::Text(t)) if t.text.as_str() == "cell-start"));
        let has_inner_fold_text = root
            .descendants()
            .any(|n| matches!(n.node(), Some(Node::Text(t)) if t.text.as_str() == "inner-fold"));

        assert!(has_fold, "pasted table_cell content should keep fold block");
        assert!(
            has_source_text,
            "pasted table_cell content should keep paragraph text"
        );
        assert!(
            has_inner_fold_text,
            "pasted table_cell content should keep fold content text"
        );
    }

    #[test]
    fn paste_open_list_item_fragment_into_callout() {
        let mut target = id!();

        let initial = state! {
            doc {
                callout {
                    @target paragraph {}
                }
            }
            selection { (target, 0) }
        };

        let mut source_para = id!();
        let mut source_item = id!();

        let source = state! {
            doc {
                bullet_list {
                    @source_item list_item {
                        @source_para paragraph {
                            text { "item" }
                        }
                        bullet_list {
                            list_item {
                                paragraph {
                                    text { "sub-item" }
                                }
                            }
                        }
                    }
                }
            }
            selection { (source_para, 0) -> (source_item, 2, Affinity::Upstream) }
        };

        let fragment = source.selection.extract_fragment(&source.doc).unwrap();
        assert_eq!(fragment.open_start(), 1);
        assert_eq!(fragment.open_end(), 0);

        let actual = transact!(initial, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let root = actual.doc.node(NodeId::ROOT).expect("root");
        let has_bullet_list = root
            .descendants()
            .any(|n| matches!(n.node(), Some(Node::BulletList(_))));
        let has_item_text = root
            .descendants()
            .any(|n| matches!(n.node(), Some(Node::Text(t)) if t.text.as_str() == "item"));
        let has_sub_item_text = root
            .descendants()
            .any(|n| matches!(n.node(), Some(Node::Text(t)) if t.text.as_str() == "sub-item"));

        assert!(
            has_bullet_list,
            "pasted list_item content should keep nested bullet list"
        );
        assert!(
            has_item_text,
            "pasted list_item content should keep paragraph text"
        );
        assert!(
            has_sub_item_text,
            "pasted list_item content should keep nested list text"
        );
    }

    #[test]
    fn paste_fold_title_text_into_paragraph_uses_pending_styles() {
        // FoldTitle 안의 텍스트를 복사하여 일반 Paragraph에 붙여넣으면,
        // 목적지(Paragraph)의 pending styles가 적용되어야 한다.
        let mut ft = id!();

        let source = state! {
            doc {
                fold {
                    @ft fold_title {
                        text { "접기 제목" }
                    }
                    fold_content {
                        paragraph {}
                    }
                }
            }
            selection { (ft, 0) -> (ft, 4) }
        };

        let fragment = source.selection.extract_fragment(&source.doc).unwrap();

        let mut target = id!();
        let paste_target = state! {
            doc {
                @target paragraph {}
            }
            selection { (target, 0) }
        };

        let (_, effects) = transact_with_effect!(paste_target, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let font_detected: Vec<(String, u16)> = effects
            .iter()
            .filter_map(|e| match e {
                Effect::FontDetected { family, weight, .. } => Some((family.clone(), *weight)),
                _ => None,
            })
            .collect();

        assert!(
            font_detected.iter().any(|(_, w)| *w == 400),
            "paste into Paragraph should emit FontDetected with default weight (400), but got: {:?}",
            font_detected,
        );
        assert!(
            !font_detected
                .iter()
                .any(|(_, w)| *w == crate::model::FOLD_TITLE_FONT_WEIGHT),
            "paste into Paragraph should NOT emit FontDetected with FoldTitle weight ({}), but got: {:?}",
            crate::model::FOLD_TITLE_FONT_WEIGHT,
            font_detected,
        );
    }

    #[test]
    fn paste_text_into_fold_title_detects_overridden_font_weight() {
        let mut ft = id!();

        let initial = state! {
            doc {
                fold {
                    @ft fold_title {}
                    fold_content {
                        paragraph {}
                    }
                }
            }
            selection { (ft, 0) }
        };

        let (_, effects) = transact_with_effect!(initial, |tr| {
            tr.paste_text("붙여넣기".to_string()).unwrap();
        });

        let font_detected: Vec<(String, u16)> = effects
            .iter()
            .filter_map(|e| match e {
                Effect::FontDetected { family, weight, .. } => Some((family.clone(), *weight)),
                _ => None,
            })
            .collect();

        let has_fold_title_weight = font_detected
            .iter()
            .any(|(_, w)| *w == crate::model::FOLD_TITLE_FONT_WEIGHT);

        assert!(
            has_fold_title_weight,
            "paste_text into FoldTitle should emit FontDetected with style_overrides weight ({}), but got: {:?}",
            crate::model::FOLD_TITLE_FONT_WEIGHT,
            font_detected,
        );
    }

    #[test]
    fn paste_fragment_into_fold_title_detects_overridden_font_weight() {
        // 일반 Paragraph 텍스트를 복사하여 FoldTitle에 붙여넣는 케이스.
        // 목적지 FoldTitle의 style_overrides(weight=500)에 해당하는 FontDetected가 발행되어야 한다.
        let mut p = id!();
        let mut ft = id!();

        let source = state! {
            doc {
                @p paragraph {
                    text { "본문 텍스트" }
                }
            }
            selection { (p, 0) -> (p, 6) }
        };

        let fragment = source.selection.extract_fragment(&source.doc).unwrap();

        let paste_target = state! {
            doc {
                fold {
                    @ft fold_title {}
                    fold_content {
                        paragraph {}
                    }
                }
            }
            selection { (ft, 0) }
        };

        let (_, effects) = transact_with_effect!(paste_target, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let font_detected: Vec<(String, u16)> = effects
            .iter()
            .filter_map(|e| match e {
                Effect::FontDetected { family, weight, .. } => Some((family.clone(), *weight)),
                _ => None,
            })
            .collect();

        let has_fold_title_weight = font_detected
            .iter()
            .any(|(_, w)| *w == crate::model::FOLD_TITLE_FONT_WEIGHT);

        assert!(
            has_fold_title_weight,
            "paste_fragment into FoldTitle should emit FontDetected with style_overrides weight ({}), but got: {:?}",
            crate::model::FOLD_TITLE_FONT_WEIGHT,
            font_detected,
        );

        assert!(
            !font_detected.iter().any(|(_, w)| *w == 400),
            "paste_fragment into FoldTitle should NOT emit FontDetected with weight 400 (text renders only at {}), but got: {:?}",
            crate::model::FOLD_TITLE_FONT_WEIGHT,
            font_detected,
        );
    }

    #[test]
    fn paste_fragment_into_fold_title_selection_at_end() {
        // Paragraph 텍스트를 복사하여 FoldTitle에 붙여넣으면,
        // 커서가 붙여넣어진 텍스트의 맨 끝에 위치해야 한다.
        let mut p = id!();
        let mut ft = id!();

        let source = state! {
            doc {
                @p paragraph {
                    text { "본문 텍스트" }
                }
            }
            selection { (p, 0) -> (p, 6) }
        };

        let fragment = source.selection.extract_fragment(&source.doc).unwrap();

        let paste_target = state! {
            doc {
                fold {
                    @ft fold_title {}
                    fold_content {
                        paragraph {}
                    }
                }
            }
            selection { (ft, 0) }
        };

        let actual = transact!(paste_target, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        assert_eq!(
            actual.selection.head.offset, 6,
            "cursor should be at end of pasted text (offset 6), but was at offset {}",
            actual.selection.head.offset,
        );
        assert!(
            actual.selection.is_collapsed(),
            "selection should be collapsed after paste, but was {:?}",
            actual.selection,
        );
    }

    #[test]
    fn paste_html_plain_text_fills_default_styles_from_pending() {
        let mut p = id!();

        let target = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let fragment = Fragment::from_html("hello").unwrap();

        let actual = transact!(target, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        // state! 매크로는 text 노드에 DefaultAttrs 6종을 자동 적용 → 동일해야 함
        let expected = state! {
            doc {
                @p paragraph {
                    text { "hello" }
                }
            }
            selection { (p, 5) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_html_bold_preserves_bold_and_fills_rest() {
        // <b>bold</b> 붙여넣기 → Bold 유지 + 나머지 기본값 보충
        let mut p = id!();

        let target = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let fragment = Fragment::from_html("<b>bold</b>").unwrap();

        let actual = transact!(target, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text(styles: [bold()]) { "bold" }
                }
            }
            selection { (p, 4) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_html_mixed_bold_and_plain_fills_both_correctly() {
        // <b>bold</b> plain → bold 세그먼트는 Bold 유지, plain 세그먼트는 기본값 보충
        let mut p = id!();

        let target = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let fragment = Fragment::from_html("<b>bold</b> plain").unwrap();

        let actual = transact!(target, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        let expected = state! {
            doc {
                @p paragraph {
                    text {
                        "bold" => [bold()],
                        " plain"
                    }
                }
            }
            selection { (p, 10) }
        };

        assert_state_eq!(actual, expected);
    }

    #[test]
    fn paste_open_fragment_does_not_apply_pending_bold_to_explicit_weight_400() {
        let mut target = id!();

        let initial = state! {
            doc {
                @target paragraph {}
            }
            selection { (target, 0) }
        };

        let fragment = fragment! {
            open_start: 1,
            open_end: 1,

            paragraph {
                text { "1" => [bg_color("none"), font_family("RIDIBatang"), font_size(1200), font_weight(400), letter_spacing(0), text_color("black")] }
            }
            paragraph {
                text { "2" => [bg_color("none"), bold(), font_family("RIDIBatang"), font_size(1200), font_weight(400), letter_spacing(0), text_color("black")] }
            }
        };

        let actual = transact!(initial, |tr| {
            tr.state
                .pending_styles
                .push(crate::model::Style::Bold(crate::model::BoldStyle {}));
            tr.paste_fragment(fragment, None).unwrap();
        });

        let root = actual.doc.node(NodeId::ROOT).expect("root");
        let mut one_styles = None;
        let mut two_styles = None;

        for node in root.descendants() {
            if let Some(Node::Text(text_node)) = node.node() {
                for seg in text_node.text.get_segments() {
                    if seg.text == "1" {
                        one_styles = Some(seg.styles.clone());
                    } else if seg.text == "2" {
                        two_styles = Some(seg.styles.clone());
                    }
                }
            }
        }

        let one_styles = one_styles.expect("segment '1' should exist");
        let two_styles = two_styles.expect("segment '2' should exist");

        assert!(
            one_styles
                .iter()
                .any(|s| matches!(s, crate::model::Style::FontWeight(fw) if fw.weight == 400)),
            "segment '1' should keep explicit FontWeight(400), got: {:?}",
            one_styles
        );
        assert!(
            !one_styles
                .iter()
                .any(|s| matches!(s, crate::model::Style::Bold(_))),
            "segment '1' should not become bold from pending style, got: {:?}",
            one_styles
        );
        assert!(
            two_styles
                .iter()
                .any(|s| matches!(s, crate::model::Style::Bold(_))),
            "segment '2' should preserve bold style, got: {:?}",
            two_styles
        );
    }

    #[test]
    fn paste_open_fragment_does_not_apply_pending_inline_decorations() {
        let mut target = id!();

        let initial = state! {
            doc {
                @target paragraph {}
            }
            selection { (target, 0) }
        };

        let fragment = fragment! {
            open_start: 1,
            open_end: 1,

            paragraph {
                text { "plain" => [bg_color("none"), font_family("RIDIBatang"), font_size(1200), font_weight(400), letter_spacing(0), text_color("black")] }
            }
            paragraph {
                text { "decorated" => [bg_color("none"), italic(), strikethrough(), underline(), font_family("RIDIBatang"), font_size(1200), font_weight(400), letter_spacing(0), text_color("black")] }
            }
        };

        let actual = transact!(initial, |tr| {
            tr.state
                .pending_styles
                .push(crate::model::Style::Italic(crate::model::ItalicStyle {}));
            tr.state
                .pending_styles
                .push(crate::model::Style::Strikethrough(
                    crate::model::StrikethroughStyle {},
                ));
            tr.state.pending_styles.push(crate::model::Style::Underline(
                crate::model::UnderlineStyle {},
            ));
            tr.paste_fragment(fragment, None).unwrap();
        });

        let root = actual.doc.node(NodeId::ROOT).expect("root");
        let mut plain_styles = None;
        let mut decorated_styles = None;

        for node in root.descendants() {
            if let Some(Node::Text(text_node)) = node.node() {
                for seg in text_node.text.get_segments() {
                    if seg.text == "plain" {
                        plain_styles = Some(seg.styles.clone());
                    } else if seg.text == "decorated" {
                        decorated_styles = Some(seg.styles.clone());
                    }
                }
            }
        }

        let plain_styles = plain_styles.expect("segment 'plain' should exist");
        let decorated_styles = decorated_styles.expect("segment 'decorated' should exist");

        assert!(
            !plain_styles
                .iter()
                .any(|s| matches!(s, crate::model::Style::Italic(_))),
            "segment 'plain' should not become italic from pending style, got: {:?}",
            plain_styles
        );
        assert!(
            !plain_styles
                .iter()
                .any(|s| matches!(s, crate::model::Style::Strikethrough(_))),
            "segment 'plain' should not become strikethrough from pending style, got: {:?}",
            plain_styles
        );
        assert!(
            !plain_styles
                .iter()
                .any(|s| matches!(s, crate::model::Style::Underline(_))),
            "segment 'plain' should not become underline from pending style, got: {:?}",
            plain_styles
        );

        assert!(
            decorated_styles
                .iter()
                .any(|s| matches!(s, crate::model::Style::Italic(_))),
            "segment 'decorated' should preserve italic style, got: {:?}",
            decorated_styles
        );
        assert!(
            decorated_styles
                .iter()
                .any(|s| matches!(s, crate::model::Style::Strikethrough(_))),
            "segment 'decorated' should preserve strikethrough style, got: {:?}",
            decorated_styles
        );
        assert!(
            decorated_styles
                .iter()
                .any(|s| matches!(s, crate::model::Style::Underline(_))),
            "segment 'decorated' should preserve underline style, got: {:?}",
            decorated_styles
        );
    }

    #[test]
    fn paste_deep_open_fragment_places_cursor_at_end_of_last_leaf_textblock() {
        let mut target = id!();
        let mut para = id!();

        let initial = state! {
            doc {
                @target paragraph {}
            }
            selection { (target, 0) }
        };

        let fragment = fragment! {
            open_start: 1,
            open_end: 3,

            paragraph {}
            bullet_list {
                list_item {
                    @para paragraph {
                        text { "ㅁㄴㅇ" }
                    }
                }
            }
        };

        let actual = transact!(initial, |tr| {
            tr.paste_fragment(fragment, None).unwrap();
        });

        assert!(
            actual.selection.is_collapsed(),
            "Selection should be collapsed after paste, but was {:?}",
            actual.selection
        );
        assert_eq!(
            actual.selection.head.node_id, para,
            "Cursor should be in the inserted paragraph"
        );
        assert_eq!(
            actual.selection.head.offset, 3,
            "Cursor should be at end of pasted text (offset 3), but was {}",
            actual.selection.head.offset
        );
    }

    #[test]
    fn repaste_as_text_after_pasting_deep_open_fragment() {
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph {}
            }
            selection { (p, 0) }
        };

        let fragment = fragment! {
            open_start: 1,
            open_end: 5,

            paragraph {}
            bullet_list {
                list_item {
                    paragraph {
                        text { "A" }
                    }
                    bullet_list {
                        list_item {
                            paragraph {
                                text { "B" }
                            }
                        }
                        list_item {
                            paragraph {
                                text { "C" }
                            }
                        }
                    }
                }
            }
        };

        let pasted_text = fragment.to_plain_text();
        let (after_paste, effects) = transact_with_effect!(initial, |tr| tr
            .paste_fragment(fragment, Some(pasted_text.clone()))
            .unwrap());

        let (selection, text, styles) = effects
            .into_iter()
            .find_map(|effect| match effect {
                Effect::HtmlPasted {
                    selection,
                    text,
                    styles,
                    ..
                } => Some((selection, text, styles)),
                _ => None,
            })
            .expect("paste_fragment should emit HtmlPasted");

        let (from, to) = selection
            .as_sorted(&after_paste.doc)
            .expect("HtmlPasted selection should be valid");

        let after_repaste = transact!(after_paste, |tr| tr
            .replace_range(from, to, Fragment::from_text(&text, &styles))
            .unwrap());

        assert!(
            after_repaste.doc.to_plain_text().contains("A"),
            "repaste-as-text should keep pasted plain text"
        );
    }

    // === Paragraph settings (line_height, align) preservation tests ===

    #[test]
    fn paste_fragment_into_empty_paragraph_uses_fragment_settings() {
        // When pasting into an empty paragraph, all paragraphs should use the fragment's settings.
        // e.g., current paragraph has line_height 220 but is empty,
        // pasting 3 paragraphs with line_height 160 should produce 3 paragraphs with line_height 160.
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph(line_height: 220,) {}
            }
            selection { (p, 0) }
        };

        let fragment = fragment! {
            open_start: 1, open_end: 1,
            paragraph(line_height: 160,) { text { "AAA" } }
            paragraph(line_height: 160,) { text { "BBB" } }
            paragraph(line_height: 160,) { text { "CCC" } }
        };

        let actual = transact!(initial, |tr| tr.paste_fragment(fragment, None).unwrap());

        // Verify paragraph settings via doc inspection
        let doc = &actual.doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        let paras: Vec<_> = root.children().collect();
        assert_eq!(paras.len(), 3);

        for (i, para) in paras.iter().enumerate() {
            let p = match para.node().unwrap() {
                Node::Paragraph(p) => p,
                _ => panic!("expected paragraph"),
            };
            assert_eq!(
                p.line_height, 160,
                "Paragraph {} should use fragment's line_height (160), got {}",
                i, p.line_height
            );
        }

        assert_eq!(doc.to_plain_text(), "AAA\nBBB\nCCC");
    }

    #[test]
    fn paste_fragment_into_nonempty_paragraph_first_uses_current_rest_uses_fragment() {
        // When pasting into a non-empty paragraph, the first paragraph uses the current
        // paragraph's settings, and the remaining paragraphs use the fragment's original settings.
        // e.g., current paragraph has line_height 220 with text,
        // pasting 3 paragraphs with line_height 160 should produce:
        //   line_height 220 (existing content + first pasted paragraph content),
        //   line_height 160, line_height 160
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph(line_height: 220,) {
                    text { "Hello" }
                }
            }
            selection { (p, 5) }
        };

        let fragment = fragment! {
            open_start: 1, open_end: 1,
            paragraph(line_height: 160,) { text { "AAA" } }
            paragraph(line_height: 160,) { text { "BBB" } }
            paragraph(line_height: 160,) { text { "CCC" } }
        };

        let actual = transact!(initial, |tr| tr.paste_fragment(fragment, None).unwrap());

        // Verify paragraph settings via doc inspection
        let doc = &actual.doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        let paras: Vec<_> = root.children().collect();
        assert_eq!(paras.len(), 3);

        let p0 = match paras[0].node().unwrap() {
            Node::Paragraph(p) => p,
            _ => panic!("expected paragraph"),
        };
        let p1 = match paras[1].node().unwrap() {
            Node::Paragraph(p) => p,
            _ => panic!("expected paragraph"),
        };
        let p2 = match paras[2].node().unwrap() {
            Node::Paragraph(p) => p,
            _ => panic!("expected paragraph"),
        };

        assert_eq!(
            p0.line_height, 220,
            "First paragraph should keep current paragraph's line_height"
        );
        assert_eq!(
            p1.line_height, 160,
            "Second paragraph should use fragment's line_height"
        );
        assert_eq!(
            p2.line_height, 160,
            "Third paragraph should use fragment's line_height"
        );

        assert_eq!(doc.to_plain_text(), "HelloAAA\nBBB\nCCC");
    }

    #[test]
    fn paste_fragment_into_nonempty_paragraph_preserves_align() {
        // Same as above but for align attribute.
        let mut p = id!();

        let initial = state! {
            doc {
                @p paragraph(align: TextAlign::Center,) {
                    text { "Hello" }
                }
            }
            selection { (p, 5) }
        };

        let fragment = fragment! {
            open_start: 1, open_end: 1,
            paragraph(align: TextAlign::Right,) { text { "AAA" } }
            paragraph(align: TextAlign::Right,) { text { "BBB" } }
        };

        let actual = transact!(initial, |tr| tr.paste_fragment(fragment, None).unwrap());

        let doc = &actual.doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        let paras: Vec<_> = root.children().collect();
        assert_eq!(paras.len(), 2);

        let p0 = match paras[0].node().unwrap() {
            Node::Paragraph(p) => p,
            _ => panic!("expected paragraph"),
        };
        let p1 = match paras[1].node().unwrap() {
            Node::Paragraph(p) => p,
            _ => panic!("expected paragraph"),
        };

        assert_eq!(
            p0.align,
            TextAlign::Center,
            "First paragraph should keep current paragraph's align"
        );
        assert_eq!(
            p1.align,
            TextAlign::Right,
            "Second paragraph should use fragment's align"
        );
    }

    #[test]
    fn repaste_nonempty_paragraph_uses_original_settings() {
        // Repaste into a non-empty paragraph should use the original paragraph's settings (220)
        // for all paragraphs, even though the pasted fragment had line_height 160.
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

        let fragment = fragment! {
            open_start: 1, open_end: 1,
            paragraph(line_height: 160,) { text { "AAA" } }
            paragraph(line_height: 160,) { text { "BBB" } }
            paragraph(line_height: 160,) { text { "CCC" } }
        };

        let pasted_text = fragment.to_plain_text();
        rt.update(Message::PasteHtml {
            html: fragment.to_html(),
            text: pasted_text,
        });

        rt.update(Message::RepasteAsText);

        let doc = &rt.state().doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        for child in root.children() {
            if let Some(Node::Paragraph(para)) = child.node() {
                assert_eq!(
                    para.line_height, 220,
                    "After repaste, all paragraphs should have the original paragraph's line_height (220), got {}",
                    para.line_height
                );
            }
        }
    }

    #[test]
    fn repaste_empty_paragraph_uses_original_settings() {
        // Even when pasting into an empty paragraph (which adopts the fragment's settings),
        // repaste should restore the ORIGINAL paragraph's settings (220).
        let mut p = id!();

        let mut rt = runtime! {
            viewport { 800, 600, 1.0 }
            doc {
                @p paragraph(line_height: 220,) {}
            }
            selection { (p, 0) }
        };

        let fragment = fragment! {
            open_start: 1, open_end: 1,
            paragraph(line_height: 160,) { text { "AAA" } }
            paragraph(line_height: 160,) { text { "BBB" } }
            paragraph(line_height: 160,) { text { "CCC" } }
        };

        let pasted_text = fragment.to_plain_text();
        rt.update(Message::PasteHtml {
            html: fragment.to_html(),
            text: pasted_text,
        });

        // After paste, paragraphs should have fragment's settings (160)
        {
            let doc = &rt.state().doc;
            let root = doc.node(NodeId::ROOT).unwrap();
            let first_para = root.children().next().unwrap();
            if let Some(Node::Paragraph(para)) = first_para.node() {
                assert_eq!(
                    para.line_height, 160,
                    "After paste into empty paragraph, should use fragment's settings"
                );
            }
        }

        rt.update(Message::RepasteAsText);

        // After repaste, all paragraphs should use the original paragraph's settings (220)
        let doc = &rt.state().doc;
        let root = doc.node(NodeId::ROOT).unwrap();
        for child in root.children() {
            if let Some(Node::Paragraph(para)) = child.node() {
                assert_eq!(
                    para.line_height, 220,
                    "After repaste, all paragraphs should have the original paragraph's line_height (220), got {}",
                    para.line_height
                );
            }
        }
    }
}
