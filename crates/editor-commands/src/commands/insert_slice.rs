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
    use editor_model::{Fragment, PlainNode, PlainParagraphNode, PlainRootNode, PlainTextNode};

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
    fn insert_inline_only_into_paragraph_middle() {
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
    fn insert_inline_at_block_boundary_wraps_paragraph() {
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
                children: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                    text: "X".into(),
                }))],
            },
            open_start: 1,
            open_end: 1,
        };
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
    fn insert_disallowed_block_unwraps_children() {
        use editor_model::{PlainBulletListNode, PlainListItemNode};
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
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
            doc { root { paragraph { t: text("HelloX") } } }
            selection: (t, 6)
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
            doc { root { paragraph { t1: text("Hello World") } } }
            selection: (t1, 5)
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
