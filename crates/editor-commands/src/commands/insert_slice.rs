use editor_clipboard::Slice;
use editor_state::is_unit_node_selection;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::insert_slice_at_position;
use crate::types::SliceProvenance;

pub fn insert_slice(
    tr: &mut Transaction,
    slice: Slice,
    provenance: SliceProvenance,
) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let Some(inserted) = insert_slice_at_position(tr, selection.head, slice, provenance)? else {
        return Ok(false);
    };
    let unit = is_unit_node_selection(&inserted, &tr.view());
    if unit {
        tr.set_selection(Some(inserted))?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_clipboard::Slice;
    use editor_crdt::Dot;
    use editor_macros::state;
    use editor_model::{
        Alignment, ChildView, Fragment, Modifier, NodeType, PlainFoldTitleNode, PlainNode,
        PlainParagraphNode, PlainRootNode, PlainTextNode,
    };
    use editor_resource::Resource;
    use editor_state::{Position, Selection, State};

    use super::*;
    use crate::test_utils::*;

    fn root_child_dots(state: &State) -> Vec<Dot> {
        let view = state.view();
        view.root()
            .expect("root exists")
            .children()
            .map(|c| match c {
                ChildView::Block(b) => b.id(),
                ChildView::Leaf(l) => l.dot(),
            })
            .collect()
    }

    fn carry_of(state: &State, dot: Dot) -> Vec<Modifier> {
        state.projected.carry_modifiers(dot).into_values().collect()
    }

    fn block_modifiers_of(state: &State, dot: Dot) -> Vec<Modifier> {
        state
            .projected
            .block_modifiers()
            .modifiers_of(dot)
            .into_values()
            .collect()
    }

    fn root_with_paragraph(text: &str) -> Slice {
        Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![Fragment {
                    node: PlainNode::Paragraph(PlainParagraphNode::default()),
                    modifiers: vec![],
                    carry: vec![],
                    children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                        text: text.into(),
                    }))],
                }],
            },
            open_start: 2,
            open_end: 2,
        }
    }

    fn paragraph_fragment(text: &str) -> Fragment {
        Fragment {
            node: PlainNode::Paragraph(PlainParagraphNode::default()),
            modifiers: vec![],
            carry: vec![],
            children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: text.into(),
            }))],
        }
    }

    fn open_fold_title_slice(text: &str) -> Slice {
        Slice {
            fragment: Fragment {
                node: PlainNode::FoldTitle(PlainFoldTitleNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                    text: text.into(),
                }))],
            },
            open_start: 1,
            open_end: 1,
        }
    }

    #[test]
    fn insert_empty_slice_no_op() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let empty = Slice {
            fragment: Fragment::leaf(PlainNode::Root(PlainRootNode::default())),
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact_fail!(initial.clone(), |tr| insert_slice(
            &mut tr,
            empty,
            SliceProvenance::Formatted
        ));
        assert_state_eq!(&actual, &initial);
    }

    #[test]
    fn insert_open_single_paragraph_into_paragraph_middle_merges_both_edges() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let slice = root_with_paragraph("XY");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("HeXYllo") } } }
            selection: (p1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_paragraph_break_slice_into_paragraph_middle_splits_once() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("asd") } } }
            selection: (p1, 1)
        };
        let empty_paragraph = || Fragment {
            node: PlainNode::Paragraph(PlainParagraphNode::default()),
            modifiers: vec![],
            carry: vec![],
            children: vec![],
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![empty_paragraph(), empty_paragraph()],
            },
            open_start: 1,
            open_end: 1,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("a") }
                p2: paragraph { text("sd") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_open_paragraph_at_block_boundary_inserts_paragraph() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                paragraph { text("b") }
            } }
            selection: (r, 1, >)
        };
        let slice = root_with_paragraph("X");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                p2: paragraph { text("X") }
                paragraph { text("b") }
            } }
            selection: (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_open_paragraph_text_into_fold_title_uses_open_inline_content() {
        let (source, ..) = state! {
            doc { root { p1: paragraph { text("body") } } }
            selection: (p1, 0) -> (p1, 4)
        };
        let slice = Slice::extract(&source).expect("non-collapsed");

        let (initial, ..) = state! {
            doc { root { fold {
                ft: fold_title {}
                fold_content { paragraph {} }
            } } }
            selection: (ft, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root { fold {
                ft1: fold_title { text("body") }
                fold_content { paragraph {} }
            } } }
            selection: (ft1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_open_fold_title_text_into_paragraph_uses_open_inline_content() {
        let (initial, ..) = state! {
            doc { root { p: paragraph {} } }
            selection: (p, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            open_fold_title_slice("title"),
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("title") } } }
            selection: (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_block_slice_into_paragraph_preserves_block_structure() {
        use editor_model::{PlainBulletListNode, PlainListItemNode};
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![Fragment {
                    node: PlainNode::BulletList(PlainBulletListNode::default()),
                    modifiers: vec![],
                    carry: vec![],
                    children: vec![Fragment {
                        node: PlainNode::ListItem(PlainListItemNode::default()),
                        modifiers: vec![],
                        carry: vec![],
                        children: vec![paragraph_fragment("X")],
                    }],
                }],
            },
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("Hello") }
                bl: bullet_list { list_item { paragraph { text("X") } } }
                paragraph {}
            } }
            selection: (bl, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn pasting_text_with_tab_yields_inline_tab_node() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let slice = Slice::from_text("a\tb");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let view = actual.view();
        let para = view
            .root()
            .expect("root exists")
            .child_blocks()
            .next()
            .expect("paragraph exists");
        let children: Vec<ChildView> = para.children().collect();
        assert_eq!(children.len(), 3, "paragraph must have 3 inline children");
        match &children[0] {
            ChildView::Leaf(l) => assert_eq!(l.as_char(), Some('a'), "first child must be 'a'"),
            _ => panic!("first child must be a char leaf"),
        }
        match &children[1] {
            ChildView::Leaf(l) => assert_eq!(
                l.node_type(),
                NodeType::Tab,
                "second child must be a Tab atom"
            ),
            _ => panic!("second child must be a tab leaf"),
        }
        match &children[2] {
            ChildView::Leaf(l) => assert_eq!(l.as_char(), Some('b'), "third child must be 'b'"),
            _ => panic!("third child must be a char leaf"),
        }
    }

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 1) -> (p1, 4)
        };
        let slice = Slice::from_text("X");
        transact_fail!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
    }

    #[test]
    fn insert_blocks_at_block_boundary() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                paragraph { text("b") }
            } }
            selection: (r, 1, >)
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![paragraph_fragment("X"), paragraph_fragment("Y")],
            },
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                paragraph { text("X") }
                p3: paragraph { text("Y") }
                paragraph { text("b") }
            } }
            selection: (p3, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_blocks_into_empty_paragraph_replaces_without_extra_empties() {
        use editor_model::PlainCalloutNode;
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![
                    Fragment {
                        node: PlainNode::Callout(PlainCalloutNode::default()),
                        modifiers: vec![],
                        carry: vec![],
                        children: vec![paragraph_fragment("1")],
                    },
                    Fragment {
                        node: PlainNode::Paragraph(PlainParagraphNode::default()),
                        modifiers: vec![],
                        carry: vec![],
                        children: vec![],
                    },
                ],
            },
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                callout { paragraph { text("1") } }
                p2: paragraph {}
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_blocks_into_paragraph_middle_splits_and_merges() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 5)
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![paragraph_fragment("first"), paragraph_fragment("second")],
            },
            open_start: 2,
            open_end: 2,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("Hellofirst") }
                p2: paragraph { text("second World") }
            } }
            selection: (p2, 6)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_image_at_text_middle_splits_paragraph_and_inserts() {
        use editor_model::PlainImageNode;
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![Fragment::leaf(PlainNode::Image(PlainImageNode::default()))],
            },
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { r: root {
                paragraph { text("hel") }
                image
                paragraph { text("lo") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_image_into_empty_paragraph_replaces_it() {
        use editor_model::PlainImageNode;
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![Fragment::leaf(PlainNode::Image(PlainImageNode::default()))],
            },
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { r: root {
                image
                paragraph {}
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_image_slice_materializes_synthetic_trailing_paragraph() {
        use editor_model::PlainImageNode;
        use editor_state::{Position, Selection};

        let (initial, ..) = state! {
            doc { root { image } }
            selection: none
        };
        let synth_p = {
            let view = initial.view();
            let root = view.root().unwrap();
            root.child_blocks()
                .find(|b| b.node_type() == NodeType::Paragraph)
                .map(|b| b.id())
                .expect("synthetic trailing paragraph")
        };
        assert!(
            synth_p.is_synthetic(),
            "trailing paragraph must be synthetic"
        );

        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![Fragment::leaf(PlainNode::Image(PlainImageNode::default()))],
            },
            open_start: 0,
            open_end: 0,
        };
        let mut tr = Transaction::new(&initial);
        tr.set_selection(Some(Selection::collapsed(Position::new(synth_p, 0))))
            .unwrap();

        assert!(insert_slice(&mut tr, slice, SliceProvenance::Formatted).unwrap());
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { r: root {
                image
                image
                paragraph {}
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn formatted_slice_insert_preserves_pending_modifiers() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
            pending_modifiers: [bold]
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![Fragment {
                    node: PlainNode::Paragraph(PlainParagraphNode::default()),
                    modifiers: vec![],
                    carry: vec![],
                    children: vec![Fragment {
                        node: PlainNode::Text(PlainTextNode { text: "XY".into() }),
                        modifiers: vec![Modifier::Italic],
                        carry: vec![],
                        children: vec![],
                    }],
                }],
            },
            open_start: 2,
            open_end: 2,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("He")
                        text("XY") [italic]
                        text("llo")
                    }
                }
            }
            selection: (p1, 4)
            pending_modifiers: [bold]
        };
        assert_state_eq!(&actual, &expected);
        assert!(!actual.pending_modifiers.is_empty());
    }

    #[test]
    fn round_trip_paint_block_format_and_carry_survive_full_copy_paste() {
        let (source, ..) = state! {
            doc { r: root {
                s1: paragraph { text("A") [bold] }
                s2: paragraph { text("B") [link(href: "https://e.com".to_string())] }
                s3: paragraph { text("C") [font_size(2000)] }
                s4: paragraph carry([bold]) { text("D") }
            } }
            selection: (r, 0, >) -> (r, 4, <)
        };
        let original = Slice::extract(&source).expect("non-collapsed");
        assert!(
            original.fragment.children[3]
                .carry
                .iter()
                .any(|m| matches!(m, Modifier::Bold)),
            "sanity: extracted carry paragraph carries bold"
        );

        let (initial, ..) = state! {
            doc { t: root { anchor: paragraph { text("Z") } } }
            selection: (t, 1, >)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            original.clone(),
            SliceProvenance::Formatted
        ));

        let root = actual.view().root().expect("root exists").id();
        let reextracted = {
            let sel = Selection::new(Position::new(root, 1), Position::new(root, 5));
            let pasted = State {
                selection: Some(sel),
                ..actual
            };
            Slice::extract(&pasted).expect("re-extract pasted blocks")
        };
        assert_eq!(
            reextracted.fragment.children, original.fragment.children,
            "paint, block format, and carry all survive the copy-paste round trip"
        );
    }

    #[test]
    fn round_trip_center_aligned_carry_paragraph_preserves_alignment_and_carry() {
        let (source, ..) = state! {
            doc { r: root {
                s1: paragraph [alignment(Alignment::Center)] carry([bold]) { text("X") }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let original = Slice::extract(&source).expect("non-collapsed");
        assert!(
            original.fragment.children[0]
                .modifiers
                .iter()
                .any(|m| matches!(
                    m,
                    Modifier::Alignment {
                        value: Alignment::Center
                    }
                )),
            "sanity: extracted paragraph is center-aligned"
        );
        assert!(
            original.fragment.children[0]
                .carry
                .iter()
                .any(|m| matches!(m, Modifier::Bold)),
            "sanity: extracted paragraph carries bold"
        );

        let (initial, ..) = state! {
            doc { t: root { anchor: paragraph { text("Z") } } }
            selection: (t, 1, >)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            original,
            SliceProvenance::Formatted
        ));

        let pasted = root_child_dots(&actual)[1];
        assert!(
            carry_of(&actual, pasted)
                .iter()
                .any(|m| matches!(m, Modifier::Bold)),
            "carry survives paste, got {:?}",
            carry_of(&actual, pasted)
        );
        assert!(
            block_modifiers_of(&actual, pasted).iter().any(|m| matches!(
                m,
                Modifier::Alignment {
                    value: Alignment::Center
                }
            )),
            "alignment (block format) survives paste, got {:?}",
            block_modifiers_of(&actual, pasted)
        );
    }

    #[test]
    fn round_trip_aligned_unit_image_via_payload_preserves_alignment() {
        let (source, ..) = state! {
            doc { r: root { img: image [alignment(Alignment::Center)] paragraph {} } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let original = Slice::extract(&source).expect("non-collapsed");
        let payload = original.to_payload();
        let parsed = Slice::from_payload(Some(&payload.html), &payload.text, &Resource::new_test());
        assert!(
            matches!(parsed.fragment.children[0].node, PlainNode::Image(_)),
            "sanity: payload carries the image"
        );

        let (initial, ..) = state! {
            doc { t: root { anchor: paragraph { text("Z") } } }
            selection: (t, 1, >)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            parsed,
            SliceProvenance::Formatted
        ));

        let img_dot = {
            let view = actual.view();
            view.root()
                .expect("root exists")
                .children()
                .find_map(|c| match c {
                    ChildView::Leaf(l) if l.node_type() == NodeType::Image => Some(l.dot()),
                    _ => None,
                })
                .expect("pasted image present")
        };
        assert!(
            block_modifiers_of(&actual, img_dot)
                .iter()
                .any(|m| matches!(
                    m,
                    Modifier::Alignment {
                        value: Alignment::Center
                    }
                )),
            "the pasted unit image keeps its alignment (block format), got {:?}",
            block_modifiers_of(&actual, img_dot)
        );
    }

    #[test]
    fn paste_open_fragment_leaves_target_carry_untouched() {
        let (src, ..) = state! {
            doc { root { sp: paragraph { text("XY") } } }
            selection: (sp, 0) -> (sp, 2)
        };
        let open = Slice::extract(&src).expect("non-collapsed");
        assert!(
            open.fragment.carry.is_empty(),
            "sanity: an open (inline) fragment carries no carry"
        );

        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph carry([italic]) { text("Hello") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            open,
            SliceProvenance::Formatted
        ));
        assert_eq!(
            carry_of(&actual, p1),
            vec![Modifier::Italic],
            "pasting a carry-less open fragment must not disturb the target's carry"
        );
    }

    #[test]
    fn paste_open_bold_fragment_into_italic_para_keeps_paint_and_target_block_format() {
        let (src, ..) = state! {
            doc { root { sp: paragraph [alignment(Alignment::Right)] { text("XY") [bold] } } }
            selection: (sp, 0) -> (sp, 2)
        };
        let open = Slice::extract(&src).expect("non-collapsed");

        let (initial, ..) = state! {
            doc { root { p1: paragraph [alignment(Alignment::Center)] { text("ab") [italic] } } }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            open,
            SliceProvenance::Formatted
        ));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph [alignment(Alignment::Center)] {
                    text("a") [italic]
                    text("XY") [bold]
                    text("b") [italic]
                }
            } }
            selection: (p1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    fn inline_all_have(view: &editor_model::DocView, block: Dot, modifier: &Modifier) -> bool {
        let Some(node) = view.node(block) else {
            return false;
        };
        let mut count = 0;
        for (i, c) in node.children().enumerate() {
            if matches!(c, ChildView::Leaf(_)) {
                count += 1;
                if !node.leaf_own_modifiers_at(i).iter().any(|m| m == modifier) {
                    return false;
                }
            }
        }
        count > 0
    }

    #[test]
    fn plain_paste_two_lines_paints_all_and_carries_new_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("가") [bold] } } }
            selection: (p1, 1)
        };
        let slice = Slice::from_text("a\nb");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        let view = actual.view();
        let paras: Vec<Dot> = view
            .root()
            .unwrap()
            .child_blocks()
            .map(|b| b.id())
            .collect();
        assert_eq!(paras.len(), 2);
        assert_eq!(view.node(paras[0]).unwrap().inline_text(), "가a");
        assert_eq!(view.node(paras[1]).unwrap().inline_text(), "b");
        assert!(inline_all_have(&view, paras[0], &Modifier::Bold));
        assert!(inline_all_have(&view, paras[1], &Modifier::Bold));
        assert!(
            carry_of(&actual, paras[1])
                .iter()
                .any(|m| matches!(m, Modifier::Bold)),
            "the new paragraph records bold carry, got {:?}",
            carry_of(&actual, paras[1])
        );
        assert!(
            carry_of(&actual, paras[0]).is_empty(),
            "the original left paragraph keeps its untouched carry"
        );
    }

    #[test]
    fn plain_paste_blank_line_carries_empty_middle_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("가") [bold] } } }
            selection: (p1, 1)
        };
        let slice = Slice::from_text("a\n\nb");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        let view = actual.view();
        let paras: Vec<Dot> = view
            .root()
            .unwrap()
            .child_blocks()
            .map(|b| b.id())
            .collect();
        assert_eq!(paras.len(), 3);
        assert_eq!(view.node(paras[0]).unwrap().inline_text(), "가a");
        assert_eq!(view.node(paras[1]).unwrap().inline_text(), "");
        assert_eq!(view.node(paras[2]).unwrap().inline_text(), "b");
        assert!(
            carry_of(&actual, paras[1])
                .iter()
                .any(|m| matches!(m, Modifier::Bold)),
            "the empty middle paragraph records bold carry, got {:?}",
            carry_of(&actual, paras[1])
        );
        assert!(
            carry_of(&actual, paras[2])
                .iter()
                .any(|m| matches!(m, Modifier::Bold))
        );
        assert!(inline_all_have(&view, paras[2], &Modifier::Bold));
    }

    #[test]
    fn plain_paste_with_tab_paints_tab_uniformly() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("가") [bold] } } }
            selection: (p1, 1)
        };
        let slice = Slice::from_text("a\tb");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        let view = actual.view();
        let paras: Vec<Dot> = view
            .root()
            .unwrap()
            .child_blocks()
            .map(|b| b.id())
            .collect();
        assert_eq!(
            paras.len(),
            1,
            "single-line tab paste creates no new paragraph"
        );
        let p = view.node(paras[0]).unwrap();
        assert!(
            p.children()
                .any(|c| matches!(c, ChildView::Leaf(l) if l.node_type() == NodeType::Tab)),
            "the pasted tab is a Tab atom"
        );
        assert!(
            inline_all_have(&view, paras[0], &Modifier::Bold),
            "every inline leaf including the Tab is painted bold"
        );
    }

    #[test]
    fn plain_paste_at_block_boundary_uses_document_default() {
        let (initial, r) = state! {
            doc { r: root {
                paragraph { text("a") [bold] }
                paragraph { text("b") [bold] }
            } }
            selection: (r, 1, >)
        };
        let slice = Slice::from_text("x\ny");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        let view = actual.view();
        let paras: Vec<Dot> = view
            .root()
            .unwrap()
            .child_blocks()
            .map(|b| b.id())
            .collect();
        let texts: Vec<String> = paras
            .iter()
            .map(|id| view.node(*id).unwrap().inline_text())
            .collect();
        assert_eq!(texts, vec!["a", "x", "y", "b"]);
        assert!(
            view.node(paras[1])
                .unwrap()
                .leaf_own_modifiers_at(0)
                .is_empty()
        );
        assert!(
            view.node(paras[2])
                .unwrap()
                .leaf_own_modifiers_at(0)
                .is_empty()
        );
        assert!(carry_of(&actual, paras[1]).is_empty());
        assert!(carry_of(&actual, paras[2]).is_empty());
        let _ = r;
    }

    #[test]
    fn plain_paste_in_link_middle_copies_link() {
        let href = "https://e.com".to_string();
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("ab") [link(href: href.clone())] } } }
            selection: (p1, 1)
        };
        let slice = Slice::from_text("X");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        let view = actual.view();
        let p = view.node(p1).unwrap();
        assert_eq!(p.inline_text(), "aXb");
        assert!(
            p.leaf_own_modifiers_at(1)
                .iter()
                .any(|m| matches!(m, Modifier::Link { .. })),
            "plain paste in the middle of a link copies the link onto the pasted char, got {:?}",
            p.leaf_own_modifiers_at(1)
        );
    }

    #[test]
    fn plain_paste_consumes_pending_once() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 1)
            pending_modifiers: [bold]
        };
        let slice = Slice::from_text("X");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Plain
        ));
        assert!(
            actual.pending_modifiers.is_empty(),
            "plain paste consumes the pending format once"
        );
        let view = actual.view();
        let p = view.node(p1).unwrap();
        assert_eq!(p.inline_text(), "hXi");
        assert!(
            p.leaf_own_modifiers_at(1)
                .iter()
                .any(|m| matches!(m, Modifier::Bold)),
            "the pasted char inherits the consumed pending bold"
        );
        assert!(
            !p.leaf_own_modifiers_at(0)
                .iter()
                .any(|m| matches!(m, Modifier::Bold))
        );
        assert!(
            !p.leaf_own_modifiers_at(2)
                .iter()
                .any(|m| matches!(m, Modifier::Bold))
        );
    }

    #[test]
    fn formatted_slice_unpainted_run_ignores_pending_and_continuation() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("가") [bold] } } }
            selection: (p1, 1)
            pending_modifiers: [italic]
        };
        let slice = root_with_paragraph("XY");
        let (actual, ..) = transact!(initial, |tr| insert_slice(
            &mut tr,
            slice,
            SliceProvenance::Formatted
        ));
        assert!(
            !actual.pending_modifiers.is_empty(),
            "a formatted paste never consumes the pending format"
        );
        let view = actual.view();
        let p = view.node(p1).unwrap();
        assert_eq!(p.inline_text(), "가XY");
        assert!(
            p.leaf_own_modifiers_at(0)
                .iter()
                .any(|m| matches!(m, Modifier::Bold))
        );
        for slot in [1usize, 2] {
            assert!(
                p.leaf_own_modifiers_at(slot).is_empty(),
                "an unpainted formatted run must not inherit caret pending/continuation, slot {slot}: {:?}",
                p.leaf_own_modifiers_at(slot)
            );
        }
    }
}
