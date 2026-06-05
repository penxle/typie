use editor_clipboard::Slice;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::insert_slice_at_position;

pub fn insert_slice(tr: &mut Transaction, slice: Slice) -> CommandResult {
    // Mirror `insert_text` / `insert_hard_break`: callers compose
    // `delete_selection` ahead of this command when they want a non-collapsed
    // selection replaced.
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let Some(inserted) = insert_slice_at_position(tr, selection.head, slice)? else {
        return Ok(false);
    };
    if inserted.is_unit_node_selection(&tr.doc()) {
        tr.set_selection(Some(inserted))?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_clipboard::Slice;
    use editor_macros::state;
    use editor_model::{
        Fragment, PlainFoldTitleNode, PlainNode, PlainParagraphNode, PlainRootNode, PlainTextNode,
    };

    use super::*;
    use crate::test_utils::*;

    fn root_with_paragraph(text: &str) -> Slice {
        Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                style: None,
                children: vec![Fragment {
                    node: PlainNode::Paragraph(PlainParagraphNode::default()),
                    modifiers: vec![],
                    style: None,
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
            style: None,
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
                style: None,
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
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let empty = Slice {
            fragment: Fragment::leaf(PlainNode::Root(PlainRootNode::default())),
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact_fail!(initial.clone(), |tr| insert_slice(&mut tr, empty));
        assert_state_eq!(&actual, &initial);
    }

    #[test]
    fn insert_open_single_paragraph_into_paragraph_middle_merges_both_edges() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let slice = root_with_paragraph("XY");
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("HeXYllo") } } }
            selection: (t1, 4)
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
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                paragraph { t2: text("X") }
                paragraph { text("b") }
            } }
            selection: (t2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_open_paragraph_text_into_fold_title_uses_open_inline_content() {
        let (source, ..) = state! {
            doc { root { paragraph { t1: text("body") } } }
            selection: (t1, 0) -> (t1, 4)
        };
        let slice = Slice::extract(&source).expect("non-collapsed");

        let (initial, ..) = state! {
            doc { root { fold {
                ft: fold_title {}
                fold_content { paragraph {} }
            } } }
            selection: (ft, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
        let (expected, ..) = state! {
            doc { root { fold {
                fold_title { t: text("body") }
                fold_content { paragraph {} }
            } } }
            selection: (t, 4)
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
            open_fold_title_slice("title")
        ));
        let (expected, ..) = state! {
            doc { root { paragraph { t: text("title") } } }
            selection: (t, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_block_slice_into_paragraph_preserves_block_structure() {
        use editor_model::{PlainBulletListNode, PlainListItemNode};
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                style: None,
                children: vec![Fragment {
                    node: PlainNode::BulletList(PlainBulletListNode::default()),
                    modifiers: vec![],
                    style: None,
                    children: vec![Fragment {
                        node: PlainNode::ListItem(PlainListItemNode::default()),
                        modifiers: vec![],
                        style: None,
                        children: vec![paragraph_fragment("X")],
                    }],
                }],
            },
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
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
        use editor_model::Node;
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let slice = Slice::from_text("a\tb");
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
        let root = actual.doc.root().expect("root exists");
        let para = root.first_child().expect("paragraph exists");
        let children: Vec<_> = para.children().collect();
        assert_eq!(children.len(), 3, "paragraph must have 3 inline children");
        assert!(
            matches!(children[0].node(), Node::Text(t) if t.text.to_string() == "a"),
            "first child must be Text(\"a\")"
        );
        assert!(
            matches!(children[1].node(), Node::Tab(_)),
            "second child must be Tab"
        );
        assert!(
            matches!(children[2].node(), Node::Text(t) if t.text.to_string() == "b"),
            "third child must be Text(\"b\")"
        );
    }

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 1) -> (t1, 4)
        };
        let slice = Slice::from_text("X");
        transact_fail!(initial, |tr| insert_slice(&mut tr, slice));
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
                style: None,
                children: vec![paragraph_fragment("X"), paragraph_fragment("Y")],
            },
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                paragraph { text("X") }
                paragraph { t3: text("Y") }
                paragraph { text("b") }
            } }
            selection: (t3, 1)
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
                style: None,
                children: vec![
                    Fragment {
                        node: PlainNode::Callout(PlainCalloutNode::default()),
                        modifiers: vec![],
                        style: None,
                        children: vec![paragraph_fragment("1")],
                    },
                    Fragment {
                        node: PlainNode::Paragraph(PlainParagraphNode::default()),
                        modifiers: vec![],
                        style: None,
                        children: vec![],
                    },
                ],
            },
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
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
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 5)
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                style: None,
                children: vec![paragraph_fragment("first"), paragraph_fragment("second")],
            },
            open_start: 2,
            open_end: 2,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("Hellofirst") }
                paragraph { t2: text("second World") }
            } }
            selection: (t2, 6)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_image_at_text_middle_splits_paragraph_and_inserts() {
        use editor_model::PlainImageNode;
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                style: None,
                children: vec![Fragment::leaf(PlainNode::Image(PlainImageNode::default()))],
            },
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
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
                style: None,
                children: vec![Fragment::leaf(PlainNode::Image(PlainImageNode::default()))],
            },
            open_start: 0,
            open_end: 0,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
        let (expected, ..) = state! {
            doc { r: root {
                image
                paragraph {}
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    fn paragraph_slice_with_style(text: &str, style_id: &str) -> Slice {
        Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                style: None,
                children: vec![Fragment {
                    node: PlainNode::Paragraph(PlainParagraphNode::default()),
                    modifiers: vec![],
                    style: Some(style_id.into()),
                    children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                        text: text.into(),
                    }))],
                }],
            },
            open_start: 2,
            open_end: 2,
        }
    }

    #[test]
    fn paste_block_boundary_applies_source_paragraph_style() {
        use editor_model::Modifier;

        use crate::commands::define_style;

        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                paragraph { text("b") }
            } }
            selection: (r, 1, >)
        };
        let (with_style, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "H1".into(),
            vec![Modifier::FontSize { value: 2400 }],
        ));

        let slice = paragraph_slice_with_style("XY", "h1");
        let (actual, ..) = transact!(with_style, |tr| insert_slice(&mut tr, slice));

        let root = actual.doc.root().unwrap();
        let inserted = root.children().nth(1).unwrap();
        assert_eq!(inserted.entry().style.get().as_deref(), Some("h1"));
    }

    #[test]
    fn paste_into_empty_paragraph_inherits_source_style() {
        use editor_model::Modifier;

        use crate::commands::define_style;

        let (initial, p_empty) = state! {
            doc { root { p_empty: paragraph {} } }
            selection: (p_empty, 0)
        };
        let (with_style, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "H1".into(),
            vec![Modifier::FontSize { value: 2400 }],
        ));

        let slice = paragraph_slice_with_style("XY", "h1");
        let (actual, ..) = transact!(with_style, |tr| insert_slice(&mut tr, slice));

        let entry = actual.doc.get_entry(p_empty).unwrap();
        assert_eq!(entry.style.get().as_deref(), Some("h1"));
    }

    #[test]
    fn extract_preserves_paragraph_style() {
        use editor_model::Modifier;

        use crate::commands::{apply_style, define_style};

        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (s1, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "H1".into(),
            vec![Modifier::FontSize { value: 2400 }]
        ));
        let (with_style, ..) = transact!(s1, |tr| apply_style(&mut tr, p1, "h1".into()));

        let slice = Slice::extract(&with_style).expect("non-collapsed");
        // Single-text-node selection from a styled paragraph: extract
        // synthesizes a Paragraph wrapper carrying the enclosing textblock's
        // style so the slice can transport it to the paste site.
        assert!(matches!(slice.fragment.node, PlainNode::Paragraph(_)));
        assert_eq!(slice.fragment.style.as_deref(), Some("h1"));
    }

    #[test]
    fn paste_into_non_empty_paragraph_keeps_destination_style() {
        use editor_model::Modifier;

        use crate::commands::{apply_style, define_style};

        let (initial, p1, _t1) = state! {
            doc { root { p1: paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let (s1, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "h1".into(),
            "H1".into(),
            vec![Modifier::FontSize { value: 2400 }]
        ));
        let (s2, ..) = transact!(s1, |tr| define_style(
            &mut tr,
            "body".into(),
            "Body".into(),
            vec![Modifier::FontSize { value: 1600 }]
        ));
        let (with_styles, ..) = transact!(s2, |tr| apply_style(&mut tr, p1, "body".into()));

        let slice = paragraph_slice_with_style("XY", "h1");
        let (actual, ..) = transact!(with_styles, |tr| insert_slice(&mut tr, slice));

        let entry = actual.doc.get_entry(p1).unwrap();
        assert_eq!(entry.style.get().as_deref(), Some("body"));
    }
}
