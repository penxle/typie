use editor_clipboard::Slice;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::insert_slice_at_position;

pub fn insert_slice(tr: &mut Transaction, slice: Slice) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let Some(inserted) = insert_slice_at_position(tr, selection.head, slice)? else {
        return Ok(false);
    };
    let unit = is_unit_node_selection(&tr.view(), &inserted);
    if unit {
        tr.set_selection(Some(inserted))?;
    }
    Ok(true)
}

fn is_unit_node_selection(view: &editor_model::DocView, sel: &editor_state::Selection) -> bool {
    if sel.anchor.node != sel.head.node {
        return false;
    }
    let (lo, hi) = (
        sel.anchor.offset.min(sel.head.offset),
        sel.anchor.offset.max(sel.head.offset),
    );
    if lo + 1 != hi {
        return false;
    }
    match view.node(sel.anchor.node).and_then(|n| n.child_at(lo)) {
        Some(editor_model::ChildView::Block(b)) => b.spec().is_unit(),
        Some(editor_model::ChildView::Leaf(l)) => {
            l.as_atom().map(|a| a.is_block_level()).unwrap_or(false)
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use editor_clipboard::Slice;
    use editor_macros::state;
    use editor_model::{
        ChildView, Fragment, NodeType, PlainFoldTitleNode, PlainNode, PlainParagraphNode,
        PlainRootNode, PlainTextNode,
    };

    use super::*;
    use crate::test_utils::*;

    fn root_with_paragraph(text: &str) -> Slice {
        Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                children: vec![Fragment {
                    node: PlainNode::Paragraph(PlainParagraphNode::default()),
                    modifiers: vec![],
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
        let (actual, ..) = transact_fail!(initial.clone(), |tr| insert_slice(&mut tr, empty));
        assert_state_eq!(&actual, &initial);
    }

    #[test]
    fn insert_open_single_paragraph_into_paragraph_middle_merges_both_edges() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let slice = root_with_paragraph("XY");
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
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
            children: vec![],
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                children: vec![empty_paragraph(), empty_paragraph()],
            },
            open_start: 1,
            open_end: 1,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
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
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
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
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
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
            open_fold_title_slice("title")
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
                children: vec![Fragment {
                    node: PlainNode::BulletList(PlainBulletListNode::default()),
                    modifiers: vec![],
                    children: vec![Fragment {
                        node: PlainNode::ListItem(PlainListItemNode::default()),
                        modifiers: vec![],
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
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let slice = Slice::from_text("a\tb");
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
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
                children: vec![
                    Fragment {
                        node: PlainNode::Callout(PlainCalloutNode::default()),
                        modifiers: vec![],
                        children: vec![paragraph_fragment("1")],
                    },
                    Fragment {
                        node: PlainNode::Paragraph(PlainParagraphNode::default()),
                        modifiers: vec![],
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
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 5)
        };
        let slice = Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                children: vec![paragraph_fragment("first"), paragraph_fragment("second")],
            },
            open_start: 2,
            open_end: 2,
        };
        let (actual, ..) = transact!(initial, |tr| insert_slice(&mut tr, slice));
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
}
