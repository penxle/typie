use editor_model::{NodeId, NodeType, PlainNode, PlainParagraphNode, Subtree};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::is_block_container;
use crate::{CommandError, CommandResult};

pub fn insert_paragraph_before_unit_selection(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let doc = tr.doc();

    if !selection.is_unit_node_selection(&doc) {
        return Ok(false);
    }

    // A unit-node selection brackets one child: both endpoints share the
    // container node, the offsets are adjacent, and the unit sits at the lower
    // offset. Inserting at that lower offset places the new paragraph right
    // before the unit. Reading it via min keeps the command direction-agnostic,
    // matching `is_unit_node_selection`.
    let parent_id = selection.anchor.node_id;
    let before_index = selection.anchor.offset.min(selection.head.offset);

    let parent = doc
        .node(parent_id)
        .ok_or(CommandError::NodeNotFound(parent_id))?;
    if !is_block_container(parent.node()) || !parent.spec().content.matches(NodeType::Paragraph) {
        return Ok(false);
    }

    let new_para_id = NodeId::new();
    let subtree = Subtree::leaf(
        new_para_id,
        PlainNode::Paragraph(PlainParagraphNode::default()),
    );
    tr.insert_subtree(parent_id, before_index, subtree)?;

    tr.set_selection(Some(Selection::collapsed(Position {
        node_id: new_para_id,
        offset: 0,
        affinity: Affinity::Downstream,
    })))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn inserts_empty_paragraph_before_atom() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| insert_paragraph_before_unit_selection(
            &mut tr
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                p1: paragraph
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn direction_agnostic() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 2, <) -> (r, 1, >)
        };
        let (actual, ..) = transact!(initial, |tr| insert_paragraph_before_unit_selection(
            &mut tr
        ));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                p1: paragraph
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn inserts_before_monolithic_block() {
        let (initial, ..) = state! {
            doc { r: root {
                fold {
                    fold_title { text("t") }
                    fold_content { paragraph { text("c") } }
                }
                paragraph { text("b") }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| insert_paragraph_before_unit_selection(
            &mut tr
        ));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph
                fold {
                    fold_title { text("t") }
                    fold_content { paragraph { text("c") } }
                }
                paragraph { text("b") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn unit_at_start_prepends_paragraph() {
        let (initial, ..) = state! {
            doc { r: root {
                horizontal_rule
                paragraph { text("a") }
            } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| insert_paragraph_before_unit_selection(
            &mut tr
        ));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph
                horizontal_rule
                paragraph { text("a") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        transact_fail!(initial, |tr| insert_paragraph_before_unit_selection(
            &mut tr
        ));
    }

    #[test]
    fn text_range_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 3)
        };
        transact_fail!(initial, |tr| insert_paragraph_before_unit_selection(
            &mut tr
        ));
    }

    #[test]
    fn multi_leaf_returns_false() {
        let (initial, ..) = state! {
            doc { r: root {
                horizontal_rule
                horizontal_rule
            } }
            selection: (r, 0, >) -> (r, 2, <)
        };
        transact_fail!(initial, |tr| insert_paragraph_before_unit_selection(
            &mut tr
        ));
    }
}
