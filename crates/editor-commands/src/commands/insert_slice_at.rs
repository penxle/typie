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
    use editor_model::{Node, NodeId};
    use editor_state::{Affinity, Position, Selection};
    use editor_transaction::Transaction;

    use super::*;

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
}
