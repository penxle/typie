use crate::model::Fragment;
use crate::runtime::Effect;
use crate::transaction::Transaction;
use anyhow::Result;

impl Transaction {
    pub fn paste_text(&mut self, s: String) -> Result<bool> {
        if s.is_empty() {
            return Ok(false);
        }

        let mut changed = false;
        let lines: Vec<&str> = s.split('\n').collect();

        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                self.split_paragraph()?;
                changed = true;
            }
            if !line.is_empty() {
                if self.insert_text(line)? {
                    changed = true;
                }
            }
        }

        Ok(changed)
    }

    pub fn paste_fragment(&mut self, fragment: Fragment, text: Option<String>) -> Result<bool> {
        if fragment.is_empty() {
            return Ok(false);
        }

        let fragment = fragment.with_fresh_ids_for_doc(self.doc());
        let result = self.insert_fragment(self.selection().head, fragment)?;
        if let Some(selection) = result.as_selection() {
            self.set_selection(selection);
        }

        if let Some(text) = text.filter(|t| !t.is_empty()) {
            if let Some(range) = result.as_range_selection() {
                let (from, to) = range.as_sorted(self.doc())?;
                self.push_effect(Effect::HtmlPasted { text, from, to });
            }
        }

        Ok(result.inserted())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Element, LayoutNode};
    use crate::model::{Node, NodeId};
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
        assert!(matches!(children[0].node(), Node::Paragraph(_)));
        assert!(matches!(children[1].node(), Node::HorizontalRule(_)));
        assert!(matches!(children[2].node(), Node::Paragraph(_)));

        let first_text = children[0]
            .first_child()
            .expect("first para should have child");
        if let Node::Text(t) = first_text.node() {
            assert_eq!(t.text.to_string(), "A");
        }

        let last_text = children[2]
            .first_child()
            .expect("last para should have child");
        if let Node::Text(t) = last_text.node() {
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
            matches!(para.node(), Node::Paragraph(_)),
            "child should be a paragraph"
        );

        let text_child = para.first_child();
        assert!(text_child.is_some(), "paragraph should have a text child");

        if let Node::Text(t) = text_child.unwrap().node() {
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
                    if let Node::Text(t) = tc.node() {
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
}
