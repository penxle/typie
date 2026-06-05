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
    use editor_macros::state;
    use editor_model::{
        Fragment, Node, NodeId, PlainImageNode, PlainNode, PlainParagraphNode, PlainRootNode,
        PlainTextNode,
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

    fn text_of_first_child(node: editor_model::NodeRef<'_>) -> Option<String> {
        node.first_child().and_then(|child| match child.node() {
            Node::Text(text) => Some(text.text.to_string()),
            _ => None,
        })
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
        let mut tr = Transaction::new(&initial);
        let inserted_selection = insert_slice_at(
            &mut tr,
            Position::new(NodeId::ROOT, 1),
            Slice::from_text("dropped"),
        )
        .expect("insert succeeds")
        .expect("slice inserted");
        let (actual, ..) = tr.commit();

        let root = actual.doc.node(NodeId::ROOT).expect("root exists");
        let children: Vec<_> = root.children().map(|c| c.node()).collect();
        assert!(matches!(
            children.as_slice(),
            [
                Node::Paragraph(_),
                Node::Paragraph(_),
                Node::Image(_),
                Node::Paragraph(_),
            ]
        ));
        let inserted = root.children().nth(1).expect("inserted paragraph");
        let inserted_text = inserted.first_child().and_then(|n| match n.node() {
            Node::Text(t) => Some(t.text.to_string()),
            _ => None,
        });
        assert_eq!(inserted_text.as_deref(), Some("dropped"));
        assert_eq!(
            inserted_selection,
            Selection::new(
                Position {
                    node_id: NodeId::ROOT,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: NodeId::ROOT,
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

        let mut tr = Transaction::new(&initial);
        let inserted_selection =
            insert_slice_at(&mut tr, Position::new(NodeId::ROOT, 1), image_slice())
                .expect("insert succeeds")
                .expect("slice inserted");
        let (actual, ..) = tr.commit();

        let root = actual.doc.node(NodeId::ROOT).expect("root exists");
        let children: Vec<_> = root.children().map(|c| c.node()).collect();
        assert!(matches!(
            children.as_slice(),
            [Node::Paragraph(_), Node::Image(_), Node::Paragraph(_)]
        ));
        assert_eq!(
            inserted_selection,
            Selection::new(
                Position {
                    node_id: NodeId::ROOT,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: NodeId::ROOT,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            )
        );
    }

    #[test]
    fn insert_slice_at_image_in_text_middle_returns_inserted_range() {
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };

        let mut tr = Transaction::new(&initial);
        let inserted_selection = insert_slice_at(&mut tr, Position::new(t, 3), image_slice())
            .expect("insert succeeds")
            .expect("slice inserted");
        let (actual, ..) = tr.commit();

        let root = actual.doc.node(NodeId::ROOT).expect("root exists");
        let children: Vec<_> = root.children().map(|c| c.node()).collect();
        assert!(matches!(
            children.as_slice(),
            [Node::Paragraph(_), Node::Image(_), Node::Paragraph(_)]
        ));
        assert_eq!(
            inserted_selection,
            Selection::new(
                Position {
                    node_id: NodeId::ROOT,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: NodeId::ROOT,
                    offset: 2,
                    affinity: Affinity::Upstream,
                },
            )
        );
    }

    #[test]
    fn insert_slice_at_closed_paragraphs_in_text_middle_returns_inserted_range() {
        let (initial, t) = state! {
            doc { root { paragraph { t: text("hello") } } }
            selection: (t, 0)
        };
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
        let inserted_selection = insert_slice_at(&mut tr, Position::new(t, 3), slice)
            .expect("insert succeeds")
            .expect("slice inserted");
        let (actual, ..) = tr.commit();

        let root = actual.doc.node(NodeId::ROOT).expect("root exists");
        let children: Vec<_> = root.children().collect();
        assert_eq!(
            children
                .iter()
                .map(|child| text_of_first_child(*child))
                .collect::<Vec<_>>(),
            vec![
                Some("hel".into()),
                Some("A".into()),
                Some("B".into()),
                Some("lo".into()),
            ]
        );
        assert_eq!(
            inserted_selection,
            Selection::new(
                Position {
                    node_id: NodeId::ROOT,
                    offset: 1,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: NodeId::ROOT,
                    offset: 3,
                    affinity: Affinity::Upstream,
                },
            )
        );
    }

    #[test]
    fn insert_slice_at_open_paragraphs_in_text_middle_returns_inserted_range() {
        let (initial, t) = state! {
            doc { root { paragraph { t: text("Hello World") } } }
            selection: (t, 0)
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

        let mut tr = Transaction::new(&initial);
        let inserted_selection = insert_slice_at(&mut tr, Position::new(t, 5), slice)
            .expect("insert succeeds")
            .expect("slice inserted");
        let (actual, ..) = tr.commit();

        let root = actual.doc.node(NodeId::ROOT).expect("root exists");
        let children: Vec<_> = root.children().collect();
        assert_eq!(
            children
                .iter()
                .map(|child| text_of_first_child(*child))
                .collect::<Vec<_>>(),
            vec![Some("Hellofirst".into()), Some("second World".into())]
        );
        let first_text = children[0].first_child().expect("first text");
        let second_text = children[1].first_child().expect("second text");
        assert_eq!(
            inserted_selection,
            Selection::new(
                Position {
                    node_id: first_text.id(),
                    offset: 5,
                    affinity: Affinity::Upstream,
                },
                Position {
                    node_id: second_text.id(),
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

        let mut tr = Transaction::new(&initial);
        let inserted_selection = insert_slice_at(&mut tr, Position::new(p, 0), image_slice())
            .expect("insert succeeds")
            .expect("slice inserted");
        let (actual, ..) = tr.commit();

        let root = actual.doc.node(NodeId::ROOT).expect("root exists");
        let children: Vec<_> = root.children().map(|c| c.node()).collect();
        assert!(matches!(
            children.as_slice(),
            [Node::Image(_), Node::Paragraph(_)]
        ));
        assert_eq!(
            inserted_selection,
            Selection::new(
                Position {
                    node_id: NodeId::ROOT,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
                Position {
                    node_id: NodeId::ROOT,
                    offset: 1,
                    affinity: Affinity::Upstream,
                },
            )
        );
    }
}
