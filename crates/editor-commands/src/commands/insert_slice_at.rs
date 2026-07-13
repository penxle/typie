use editor_clipboard::Slice;
use editor_state::{Position, Selection};
use editor_transaction::Transaction;

use crate::CommandError;
use crate::helpers::insert_slice_at_position;
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
        ChildView, DocView, Fragment, NodeType, PlainBulletListNode, PlainImageNode,
        PlainListItemNode, PlainNode, PlainParagraphNode, PlainTextNode,
    };
    use editor_state::{Affinity, Position, Selection};
    use editor_transaction::Transaction;

    use super::*;

    fn image_slice() -> Slice {
        Slice {
            content: vec![Fragment::leaf(PlainNode::Image(PlainImageNode::default()))],
            open_start: 0,
            open_end: 0,
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
}
