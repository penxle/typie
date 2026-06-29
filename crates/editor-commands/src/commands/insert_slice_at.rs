use editor_clipboard::Slice;
use editor_state::{Position, Selection};
use editor_transaction::Transaction;

use crate::CommandError;
use crate::helpers::insert_slice_at_position;

pub fn insert_slice_at(
    tr: &mut Transaction,
    position: Position,
    slice: Slice,
) -> Result<Option<Selection>, CommandError> {
    insert_slice_at_position(tr, position, slice)
}

#[cfg(test)]
mod tests {
    use editor_clipboard::Slice;
    use editor_crdt::Dot;
    use editor_macros::state;
    use editor_model::{
        ChildView, DocView, Fragment, NodeType, PlainImageNode, PlainNode, PlainParagraphNode,
        PlainRootNode, PlainTextNode,
    };
    use editor_state::{Affinity, Position, Selection};
    use editor_transaction::Transaction;

    use super::*;

    fn image_slice() -> Slice {
        Slice {
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                style: None,
                children: vec![Fragment::leaf(PlainNode::Image(PlainImageNode::default()))],
            },
            open_start: 0,
            open_end: 0,
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
        let inserted_selection =
            insert_slice_at(&mut tr, Position::new(root, 1), Slice::from_text("dropped"))
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
        let inserted_selection = insert_slice_at(&mut tr, Position::new(root, 1), image_slice())
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
        let inserted_selection = insert_slice_at(&mut tr, Position::new(p1, 3), image_slice())
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
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                style: None,
                children: vec![paragraph_fragment("A"), paragraph_fragment("B")],
            },
            open_start: 0,
            open_end: 0,
        };

        let mut tr = Transaction::new(&initial);
        let inserted_selection = insert_slice_at(&mut tr, Position::new(p1, 3), slice)
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
            fragment: Fragment {
                node: PlainNode::Root(PlainRootNode::default()),
                modifiers: vec![],
                style: None,
                children: vec![paragraph_fragment("first"), paragraph_fragment("second")],
            },
            open_start: 2,
            open_end: 2,
        };

        let mut tr = Transaction::new(&initial);
        let inserted_selection = insert_slice_at(&mut tr, Position::new(p1, 5), slice)
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
    fn insert_slice_at_image_into_empty_paragraph_returns_inserted_range() {
        let (initial, p) = state! {
            doc { root { p: paragraph {} } }
            selection: (p, 0)
        };
        let root = initial.view().root().unwrap().id();

        let mut tr = Transaction::new(&initial);
        let inserted_selection = insert_slice_at(&mut tr, Position::new(p, 0), image_slice())
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
}
