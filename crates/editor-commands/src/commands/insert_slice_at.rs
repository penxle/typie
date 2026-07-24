use editor_clipboard::Slice;
use editor_state::{Position, Selection};
use editor_transaction::Transaction;

use crate::CommandError;
use crate::judgments::insert_slice_at_position;
use crate::types::SliceProvenance;

pub fn insert_slice_at(
    tr: &mut Transaction,
    position: Position,
    slice: Slice,
    provenance: SliceProvenance,
) -> Result<Option<Selection>, CommandError> {
    insert_slice_at_position(tr, position, slice, provenance)
}

#[cfg(test)]
mod tests {
    use editor_clipboard::Slice;
    use editor_crdt::Dot;
    use editor_macros::state;
    use editor_model::{
        ChildView, DocView, Fragment, NodeType, PlainBulletListNode, PlainFileNode,
        PlainHorizontalRuleNode, PlainImageNode, PlainListItemNode, PlainNode, PlainPageBreakNode,
        PlainParagraphNode, PlainTextNode,
    };
    use editor_state::{Affinity, Position, Selection};
    use editor_transaction::Transaction;

    use super::*;
    use crate::test_utils::*;

    fn image_slice() -> Slice {
        Slice {
            content: vec![Fragment::leaf(PlainNode::Image(PlainImageNode::default()))],
            open_start: 0,
            open_end: 0,
        }
    }

    fn horizontal_rule_slice() -> Slice {
        Slice {
            content: vec![Fragment::leaf(PlainNode::HorizontalRule(
                PlainHorizontalRuleNode::default(),
            ))],
            open_start: 0,
            open_end: 0,
        }
    }

    fn empty_text_slice() -> Slice {
        Slice {
            content: vec![Fragment::leaf(PlainNode::Text(PlainTextNode {
                text: String::new(),
            }))],
            open_start: 0,
            open_end: 0,
        }
    }

    fn page_break_slice() -> Slice {
        Slice {
            content: vec![Fragment::leaf(PlainNode::PageBreak(
                PlainPageBreakNode::default(),
            ))],
            open_start: 0,
            open_end: 0,
        }
    }

    fn text_and_page_break_slice(text: &str) -> Slice {
        Slice {
            content: vec![
                Fragment::leaf(PlainNode::Text(PlainTextNode { text: text.into() })),
                Fragment::leaf(PlainNode::PageBreak(PlainPageBreakNode::default())),
            ],
            open_start: 0,
            open_end: 0,
        }
    }

    fn assert_rejected_slice_preserves_synthetic_target(
        initial: editor_state::State,
        target: Dot,
        slice: Slice,
    ) {
        assert!(target.is_synthetic(), "fixture target must be synthetic");
        let mut tr = Transaction::new(&initial);
        assert!(
            insert_slice_at(
                &mut tr,
                Position::new(target, 0),
                slice,
                SliceProvenance::Formatted,
            )
            .expect("invalid insertion is a no-op")
            .is_none()
        );
        let (actual, ..) = tr.commit();
        assert_state_eq!(&actual, &initial);
        assert!(actual.view().node(target).is_some());
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

    fn paragraph_with_page_break_fragment(text: &str) -> Fragment {
        Fragment {
            node: PlainNode::Paragraph(PlainParagraphNode::default()),
            modifiers: vec![],
            carry: vec![],
            children: if text.is_empty() {
                page_break_slice().content
            } else {
                text_and_page_break_slice(text).content
            },
        }
    }

    fn paragraph_with_page_break_slice(text: &str, open_start: u32, open_end: u32) -> Slice {
        Slice {
            content: vec![paragraph_with_page_break_fragment(text)],
            open_start,
            open_end,
        }
    }

    fn open_paragraphs_ending_in_page_break_slice() -> Slice {
        Slice {
            content: vec![
                paragraph_fragment("A"),
                paragraph_with_page_break_fragment("B"),
            ],
            open_start: 1,
            open_end: 1,
        }
    }

    fn open_page_break_paragraphs_slice() -> Slice {
        Slice {
            content: vec![
                paragraph_with_page_break_fragment("A"),
                paragraph_with_page_break_fragment("B"),
            ],
            open_start: 1,
            open_end: 1,
        }
    }

    fn open_bullet_list_slice(text: &str, open_start: u32, open_end: u32) -> Slice {
        Slice {
            content: vec![Fragment {
                node: PlainNode::BulletList(PlainBulletListNode::default()),
                modifiers: vec![],
                carry: vec![],
                children: vec![Fragment {
                    node: PlainNode::ListItem(PlainListItemNode::default()),
                    modifiers: vec![],
                    carry: vec![],
                    children: vec![paragraph_fragment(text)],
                }],
            }],
            open_start,
            open_end,
        }
    }

    fn kinds(view: &DocView, root: Dot) -> Vec<NodeType> {
        view.node(root)
            .unwrap()
            .children()
            .map(|c| match c {
                ChildView::Block(b) => b.node_type(),
                ChildView::Leaf(l) => l.node_type(),
            })
            .collect()
    }

    fn assert_trailing_paragraph_is_synthetic(state: &editor_state::State) {
        let view = state.view();
        let trailing = view.root().unwrap().child_blocks().last().unwrap();
        assert_eq!(trailing.node_type(), NodeType::Paragraph);
        assert!(trailing.id().is_synthetic());
    }

    fn block_texts(view: &DocView, root: Dot) -> Vec<String> {
        view.node(root)
            .unwrap()
            .child_blocks()
            .map(|b| b.inline_text())
            .collect()
    }

    fn list_item_texts(view: &DocView, list: Dot) -> Vec<String> {
        view.node(list)
            .unwrap()
            .child_blocks()
            .map(|item| {
                item.descendants()
                    .filter_map(|child| match child {
                        ChildView::Leaf(leaf) => leaf.as_char(),
                        ChildView::Block(_) => None,
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn insert_slice_at_block_position_before_unit_keeps_unit() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("before") }
                image
                paragraph { text("after") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let root = initial.view().root().unwrap().id();

        let mut tr = Transaction::new(&initial);
        let inserted_selection = insert_slice_at(
            &mut tr,
            Position::new(root, 1),
            Slice::from_text("dropped"),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("slice inserted");
        let (actual, ..) = tr.commit();

        let view = actual.view();
        assert_eq!(
            kinds(&view, root),
            vec![
                NodeType::Paragraph,
                NodeType::Paragraph,
                NodeType::Image,
                NodeType::Paragraph,
            ]
        );
        let inserted = view.node(root).unwrap().child_blocks().nth(1).unwrap();
        assert_eq!(inserted.inline_text(), "dropped");
        assert_eq!(
            inserted_selection,
            Selection::new(
                Position {
                    node: root,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node: root,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            )
        );
    }

    #[test]
    fn insert_slice_at_block_position_after_atom_preserves_multiple_atom_order() {
        let (initial, root, ..) = state! {
            doc { root: root {
                paragraph { text("before") }
                image
                paragraph { text("after") }
            } }
            selection: none
        };
        let slice = Slice::new(
            vec![
                Fragment::leaf(PlainNode::File(PlainFileNode { id: None })),
                Fragment::leaf(PlainNode::Image(PlainImageNode::default())),
            ],
            0,
            0,
        );

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(root, 2),
            slice,
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("slice inserted");
        let (actual, ..) = tr.commit();

        assert_eq!(
            kinds(&actual.view(), root),
            vec![
                NodeType::Paragraph,
                NodeType::Image,
                NodeType::File,
                NodeType::Image,
                NodeType::Paragraph,
            ]
        );
        assert_eq!(
            inserted,
            Selection::new(
                Position {
                    node: root,
                    offset: 2,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node: root,
                    offset: 4,
                    affinity: Affinity::Upstream,
                },
            )
        );
    }

    #[test]
    fn insert_slice_at_image_at_root_end_fulfills_trailing_paragraph() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("before") }
            } }
            selection: (r, 0)
        };
        let root = initial.view().root().unwrap().id();

        let mut tr = Transaction::new(&initial);
        let inserted_selection = insert_slice_at(
            &mut tr,
            Position::new(root, 1),
            image_slice(),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("slice inserted");
        let (actual, ..) = tr.commit();

        let view = actual.view();
        assert_eq!(
            kinds(&view, root),
            vec![NodeType::Paragraph, NodeType::Image, NodeType::Paragraph]
        );
        assert_eq!(
            inserted_selection,
            Selection::new(
                Position {
                    node: root,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node: root,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            )
        );
    }

    #[test]
    fn insert_slice_at_image_in_text_middle_returns_inserted_range() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let root = initial.view().root().unwrap().id();

        let mut tr = Transaction::new(&initial);
        let inserted_selection = insert_slice_at(
            &mut tr,
            Position::new(p1, 3),
            image_slice(),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("slice inserted");
        let (actual, ..) = tr.commit();

        let view = actual.view();
        assert_eq!(
            kinds(&view, root),
            vec![NodeType::Paragraph, NodeType::Image, NodeType::Paragraph]
        );
        assert_eq!(
            inserted_selection,
            Selection::new(
                Position {
                    node: root,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node: root,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            )
        );
    }

    #[test]
    fn insert_slice_at_closed_paragraphs_in_text_middle_returns_inserted_range() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let root = initial.view().root().unwrap().id();
        let slice = Slice {
            content: vec![paragraph_fragment("A"), paragraph_fragment("B")],
            open_start: 0,
            open_end: 0,
        };

        let mut tr = Transaction::new(&initial);
        let inserted_selection = insert_slice_at(
            &mut tr,
            Position::new(p1, 3),
            slice,
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("slice inserted");
        let (actual, ..) = tr.commit();

        let view = actual.view();
        assert_eq!(
            block_texts(&view, root),
            vec![
                "hel".to_string(),
                "A".to_string(),
                "B".to_string(),
                "lo".to_string(),
            ]
        );
        assert_eq!(
            inserted_selection,
            Selection::new(
                Position {
                    node: root,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node: root,
                    offset: 3,
                    affinity: Affinity::Upstream,
                },
            )
        );
    }

    #[test]
    fn insert_slice_at_open_paragraphs_in_text_middle_returns_inserted_range() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("Hello World") } } }
            selection: (p1, 0)
        };
        let root = initial.view().root().unwrap().id();
        let slice = Slice {
            content: vec![paragraph_fragment("first"), paragraph_fragment("second")],
            open_start: 1,
            open_end: 1,
        };

        let mut tr = Transaction::new(&initial);
        let inserted_selection = insert_slice_at(
            &mut tr,
            Position::new(p1, 5),
            slice,
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("slice inserted");
        let (actual, ..) = tr.commit();

        let view = actual.view();
        assert_eq!(
            block_texts(&view, root),
            vec!["Hellofirst".to_string(), "second World".to_string()]
        );
        let first_para = view.node(root).unwrap().child_blocks().next().unwrap();
        let second_para = view.node(root).unwrap().child_blocks().nth(1).unwrap();
        assert_eq!(
            inserted_selection,
            Selection::new(
                Position {
                    node: first_para.id(),
                    offset: 5,
                    affinity: Affinity::Upstream,
                },
                Position {
                    node: second_para.id(),
                    offset: 6,
                    affinity: Affinity::Upstream,
                },
            )
        );
    }

    #[test]
    fn insert_open_list_context_merges_items_at_list_boundary() {
        let (initial, list) = state! {
            doc { root {
                list: bullet_list {
                    list_item { paragraph { text("A") } }
                    list_item { paragraph { text("C") } }
                }
                paragraph {}
            } }
            selection: none
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(list, 1),
            open_bullet_list_slice("B", 1, 1),
            SliceProvenance::Formatted,
        )
        .expect("command succeeds");
        assert!(inserted.is_some());
        let (actual, ..) = tr.commit();

        let view = actual.view();
        let list = view.node(list).expect("list remains");
        assert_eq!(list.child_blocks().count(), 3);
        assert_eq!(list_item_texts(&view, list.id()), ["A", "B", "C"]);
    }

    #[test]
    fn insert_open_list_context_with_only_start_edge_open_merges_item() {
        let (initial, list) = state! {
            doc { root {
                list: bullet_list { list_item { paragraph { text("A") } } }
                paragraph {}
            } }
            selection: none
        };
        let mut tr = Transaction::new(&initial);

        assert!(
            insert_slice_at(
                &mut tr,
                Position::new(list, 1),
                open_bullet_list_slice("B", 3, 0),
                SliceProvenance::Formatted,
            )
            .expect("command succeeds")
            .is_some()
        );
        let (actual, ..) = tr.commit();

        assert_eq!(list_item_texts(&actual.view(), list), ["A", "B"]);
    }

    #[test]
    fn insert_open_list_context_with_only_end_edge_open_merges_item() {
        let (initial, list) = state! {
            doc { root {
                list: bullet_list { list_item { paragraph { text("A") } } }
                paragraph {}
            } }
            selection: none
        };
        let mut tr = Transaction::new(&initial);

        assert!(
            insert_slice_at(
                &mut tr,
                Position::new(list, 1),
                open_bullet_list_slice("B", 0, 3),
                SliceProvenance::Formatted,
            )
            .expect("command succeeds")
            .is_some()
        );
        let (actual, ..) = tr.commit();

        assert_eq!(list_item_texts(&actual.view(), list), ["A", "B"]);
    }

    #[test]
    fn block_boundary_rejects_closed_invalid_root_without_unwrapping() {
        let (initial, root) = state! {
            doc { root: root { paragraph {} } }
            selection: none
        };
        let slice = Slice {
            content: vec![Fragment {
                node: NodeType::FoldContent.into_node().to_plain(),
                modifiers: vec![],
                carry: vec![],
                children: vec![paragraph_fragment("inside")],
            }],
            open_start: 0,
            open_end: 0,
        };
        let mut tr = Transaction::new(&initial);

        assert!(
            insert_slice_at(
                &mut tr,
                Position::new(root, 0),
                slice,
                SliceProvenance::Formatted,
            )
            .expect("invalid insertion is a no-op")
            .is_none()
        );
        let (actual, ..) = tr.commit();
        assert_eq!(kinds(&actual.view(), root), vec![NodeType::Paragraph]);
    }

    #[test]
    fn rejected_slice_does_not_materialize_or_split_synthetic_textblock() {
        let (initial, ..) = state! {
            doc { root { blockquote paragraph {} } }
            selection: none
        };
        let target = {
            let view = initial.view();
            view.root()
                .unwrap()
                .child_blocks()
                .find(|block| block.node_type() == NodeType::Blockquote)
                .unwrap()
                .child_blocks()
                .next()
                .unwrap()
                .id()
        };
        assert_rejected_slice_preserves_synthetic_target(initial, target, horizontal_rule_slice());
    }

    #[test]
    fn rejected_empty_inline_slice_does_not_materialize_synthetic_textblock() {
        let (initial, ..) = state! {
            doc { root { image } }
            selection: none
        };
        let target = initial
            .view()
            .root()
            .unwrap()
            .child_blocks()
            .find(|block| block.node_type() == NodeType::Paragraph)
            .unwrap()
            .id();
        assert_rejected_slice_preserves_synthetic_target(initial, target, empty_text_slice());
    }

    #[test]
    fn insert_slice_at_image_into_empty_paragraph_returns_inserted_range() {
        let (initial, p) = state! {
            doc { root { p: paragraph {} } }
            selection: (p, 0)
        };
        let root = initial.view().root().unwrap().id();

        let mut tr = Transaction::new(&initial);
        let inserted_selection = insert_slice_at(
            &mut tr,
            Position::new(p, 0),
            image_slice(),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("slice inserted");
        let (actual, ..) = tr.commit();

        let view = actual.view();
        assert_eq!(
            kinds(&view, root),
            vec![NodeType::Image, NodeType::Paragraph]
        );
        assert_eq!(
            inserted_selection,
            Selection::new(
                Position {
                    node: root,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node: root,
                    offset: 1,
                    affinity: Affinity::Upstream,
                },
            )
        );
    }

    #[test]
    fn insert_slice_at_image_materializes_synthetic_trailing_paragraph() {
        let (initial, ..) = state! {
            doc { root { image } }
            selection: none
        };
        let root = initial.view().root().unwrap().id();
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

        let mut tr = Transaction::new(&initial);
        let inserted_selection = insert_slice_at(
            &mut tr,
            Position::new(synth_p, 0),
            image_slice(),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("slice inserted");
        let (actual, ..) = tr.commit();

        let view = actual.view();
        assert_eq!(
            kinds(&view, root),
            vec![NodeType::Image, NodeType::Image, NodeType::Paragraph]
        );
        assert_eq!(
            inserted_selection,
            Selection::new(
                Position {
                    node: root,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node: root,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            )
        );
    }

    #[test]
    fn insert_page_break_slice_in_root_paragraph_middle_splits_at_terminal() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("World") } } }
            selection: none
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 3),
            page_break_slice(),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("page break inserted");
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root {
                paragraph { text("Wor") page_break }
                p2: paragraph { text("ld") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert!(!inserted.is_collapsed());
    }

    #[test]
    fn insert_page_break_slice_at_root_paragraph_start_keeps_right_content_separate() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("World") } } }
            selection: none
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 0),
            page_break_slice(),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("page break inserted");
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root {
                paragraph { page_break }
                p2: paragraph { text("World") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert!(!inserted.is_collapsed());
    }

    #[test]
    fn insert_page_break_slice_into_empty_root_paragraph_uses_synthetic_following_paragraph() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph {} } }
            selection: none
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 0),
            page_break_slice(),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("page break inserted");
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root {
                paragraph { page_break }
                p2: paragraph {}
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_trailing_paragraph_is_synthetic(&actual);
        assert!(!inserted.is_collapsed());
    }

    #[test]
    fn insert_open_paragraphs_ending_in_page_break_in_middle_uses_right_half() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("xy") } } }
            selection: none
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 1),
            open_paragraphs_ending_in_page_break_slice(),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("paragraphs inserted");
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root {
                paragraph { text("xA") }
                paragraph { text("B") page_break }
                p2: paragraph { text("y") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert!(!inserted.is_collapsed());
    }

    #[test]
    fn insert_open_page_break_paragraphs_in_middle_keeps_full_inserted_range() {
        let (initial, root, p1) = state! {
            doc { root: root { p1: paragraph { text("xy") } } }
            selection: none
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 1),
            open_page_break_paragraphs_slice(),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("paragraphs inserted");
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root {
                paragraph { text("xA") page_break }
                paragraph { text("B") page_break }
                p2: paragraph { text("y") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_eq!(
            inserted.anchor,
            Position {
                node: p1,
                offset: 1,
                affinity: Affinity::Upstream,
            }
        );
        assert_eq!(
            inserted.head,
            Position {
                node: root,
                offset: 2,
                affinity: Affinity::Upstream,
            }
        );
    }

    #[test]
    fn insert_page_break_slice_at_root_paragraph_end_uses_synthetic_following_paragraph() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("World") } } }
            selection: none
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 5),
            page_break_slice(),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("page break inserted");
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root {
                paragraph { text("World") page_break }
                p2: paragraph {}
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_trailing_paragraph_is_synthetic(&actual);
        assert!(!inserted.is_collapsed());
    }

    #[test]
    fn open_textblock_after_terminal_inline_stays_in_its_wrapper() {
        let (initial, p1) = state! {
            doc { root {
                p1: paragraph { text("x") page_break }
                paragraph {}
            } }
            selection: none
        };
        let slice = Slice {
            content: vec![paragraph_fragment("A")],
            open_start: 1,
            open_end: 0,
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 2),
            slice,
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("paragraph inserted");
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root {
                paragraph { text("x") page_break }
                inserted: paragraph { text("A") }
                paragraph {}
            } }
            selection: (inserted, 1)
        };
        assert_state_eq!(&actual, &expected);
        assert!(!inserted.is_collapsed());
    }

    #[test]
    fn insert_open_start_page_break_slice_at_root_paragraph_end_keeps_following_paragraph() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("World") } } }
            selection: none
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 5),
            paragraph_with_page_break_slice("lo", 1, 0),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("page break inserted");
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root {
                paragraph { text("Worldlo") page_break }
                p2: paragraph {}
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_trailing_paragraph_is_synthetic(&actual);
        assert!(!inserted.is_collapsed());
    }

    #[test]
    fn insert_page_break_only_in_nested_paragraph_is_atomic_no_op() {
        let (initial, p1) = state! {
            doc { root { blockquote { p1: paragraph { text("Nested") } } paragraph {} } }
            selection: none
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 3),
            page_break_slice(),
            SliceProvenance::Formatted,
        )
        .expect("invalid page break is a no-op");
        assert!(inserted.is_none());
        let (actual, ..) = tr.commit();

        assert_state_eq!(&actual, &initial);
    }

    #[test]
    fn insert_page_break_only_paragraph_nested_is_atomic_no_op() {
        let (initial, p1) = state! {
            doc { root { blockquote { p1: paragraph { text("Nested") } } paragraph {} } }
            selection: none
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 3),
            paragraph_with_page_break_slice("", 0, 0),
            SliceProvenance::Formatted,
        )
        .expect("invalid page break is a no-op");
        assert!(inserted.is_none());
        let (actual, ..) = tr.commit();

        assert_state_eq!(&actual, &initial);
    }

    #[test]
    fn insert_page_break_at_root_block_gap_wraps_it_in_paragraph() {
        let (initial, root) = state! {
            doc { root: root { paragraph { text("before") } } }
            selection: none
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(root, 1),
            page_break_slice(),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("page break inserted");
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root {
                paragraph { text("before") }
                paragraph { page_break }
                p3: paragraph {}
            } }
            selection: (p3, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_trailing_paragraph_is_synthetic(&actual);
        assert_eq!(
            inserted,
            Selection::new(
                Position::new(root, 1),
                Position {
                    node: root,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            )
        );
    }

    #[test]
    fn insert_non_terminal_page_break_drops_it_and_keeps_following_text() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let slice = Slice {
            content: vec![
                Fragment::leaf(PlainNode::PageBreak(PlainPageBreakNode::default())),
                Fragment::leaf(PlainNode::Text(PlainTextNode { text: "x".into() })),
            ],
            open_start: 0,
            open_end: 0,
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 1),
            slice,
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("text inserted");
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("axb") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
        assert!(!inserted.is_collapsed());
    }

    #[test]
    fn insert_closed_page_break_paragraph_preserves_block_structure() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("World") } } }
            selection: none
        };
        let root = initial.view().root().unwrap().id();

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 3),
            paragraph_with_page_break_slice("lo", 0, 0),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("paragraph inserted");
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root {
                paragraph { text("Wor") }
                paragraph { text("lo") page_break }
                paragraph { text("ld") }
            } }
            selection: none
        };
        editor_state::assert_doc_eq!(&actual, &expected);
        assert_eq!(inserted.anchor, Position::new(root, 1));
        assert_eq!(inserted.head.node, root);
        assert_eq!(inserted.head.offset, 2);
    }

    #[test]
    fn insert_open_page_break_paragraph_keeps_page_break_terminal() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("World") } } }
            selection: none
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 3),
            paragraph_with_page_break_slice("lo", 1, 1),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("slice inserted");
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root {
                paragraph { text("Worlo") page_break }
                p2: paragraph { text("ld") }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert!(!inserted.is_collapsed());
    }

    #[test]
    fn insert_closed_page_break_paragraph_nested_drops_only_page_break() {
        let (initial, p1) = state! {
            doc { root { blockquote { p1: paragraph { text("Nested") } } paragraph {} } }
            selection: none
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 3),
            paragraph_with_page_break_slice("lo", 0, 0),
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("paragraph inserted");
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    paragraph { text("Nes") }
                    paragraph { text("lo") }
                    paragraph { text("ted") }
                }
                paragraph {}
            } }
            selection: none
        };
        editor_state::assert_doc_eq!(&actual, &expected);
        assert!(!inserted.is_collapsed());
    }

    #[test]
    fn removing_first_page_break_only_block_does_not_transfer_its_openness() {
        let (initial, p1) = state! {
            doc { root { blockquote { p1: paragraph { text("Nested") } } paragraph {} } }
            selection: none
        };
        let slice = Slice {
            content: vec![
                paragraph_with_page_break_fragment(""),
                paragraph_fragment("x"),
            ],
            open_start: 1,
            open_end: 1,
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 3),
            slice,
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("text inserted");
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    paragraph { text("Nes") }
                    paragraph { text("xted") }
                }
                paragraph {}
            } }
            selection: none
        };
        editor_state::assert_doc_eq!(&actual, &expected);
        assert!(!inserted.is_collapsed());
    }

    #[test]
    fn removing_last_page_break_only_block_does_not_transfer_its_openness() {
        let (initial, p1) = state! {
            doc { root { blockquote { p1: paragraph { text("Nested") } } paragraph {} } }
            selection: none
        };
        let slice = Slice {
            content: vec![
                paragraph_fragment("x"),
                paragraph_with_page_break_fragment(""),
            ],
            open_start: 1,
            open_end: 1,
        };

        let mut tr = Transaction::new(&initial);
        let inserted = insert_slice_at(
            &mut tr,
            Position::new(p1, 3),
            slice,
            SliceProvenance::Formatted,
        )
        .expect("insert succeeds")
        .expect("text inserted");
        let (actual, ..) = tr.commit();

        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    paragraph { text("Nesx") }
                    paragraph { text("ted") }
                }
                paragraph {}
            } }
            selection: none
        };
        editor_state::assert_doc_eq!(&actual, &expected);
        assert!(!inserted.is_collapsed());
    }
}
