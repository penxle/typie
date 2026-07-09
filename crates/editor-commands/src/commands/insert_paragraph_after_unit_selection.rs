use editor_model::{ChildView, NodeType, PlainNode, PlainParagraphNode, Subtree};
use editor_state::{Affinity, Position, Selection, is_unit_node_selection};
use editor_transaction::Transaction;

use crate::helpers::is_block_container;
use crate::{CommandError, CommandResult};

pub fn insert_paragraph_after_unit_selection(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    // A unit-node selection brackets one child: both endpoints share the
    // container node, the offsets are adjacent, and the unit sits at the lower
    // offset. The slot right after the unit is therefore the higher offset.
    let parent_id = selection.anchor.node;
    let after_index = selection.anchor.offset.max(selection.head.offset);

    {
        let view = tr.state().view();
        if !is_unit_node_selection(&selection, &view) {
            return Ok(false);
        }
        let parent = view
            .node(parent_id)
            .ok_or(CommandError::NodeNotFound(parent_id))?;
        if !is_block_container(&parent) || !parent.spec().content.matches(NodeType::Paragraph) {
            return Ok(false);
        }
    }

    let subtree = Subtree::leaf(PlainNode::Paragraph(PlainParagraphNode::default()));
    tr.insert_subtree(parent_id, after_index, subtree)?;

    let new_para = {
        let view = tr.state().view();
        match view.node(parent_id).and_then(|p| p.child_at(after_index)) {
            Some(ChildView::Block(b)) => b.id(),
            _ => {
                return Err(CommandError::Corrupted(
                    "inserted paragraph not found after unit".into(),
                ));
            }
        }
    };

    tr.set_selection(Some(Selection::collapsed(Position {
        node: new_para,
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
    fn inserts_empty_paragraph_after_atom() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| insert_paragraph_after_unit_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                horizontal_rule
                p1: paragraph
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
        let (actual, ..) = transact!(initial, |tr| insert_paragraph_after_unit_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                horizontal_rule
                p1: paragraph
                paragraph { text("c") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn inserts_after_monolithic_block() {
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
        let (actual, ..) = transact!(initial, |tr| insert_paragraph_after_unit_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                fold {
                    fold_title { text("t") }
                    fold_content { paragraph { text("c") } }
                }
                p1: paragraph
                paragraph { text("b") }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn unit_at_end_appends_paragraph() {
        let (initial, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| insert_paragraph_after_unit_selection(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                horizontal_rule
                p1: paragraph
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn escaped_paragraph_after_unit_has_empty_carry() {
        let (initial, _r) = state! {
            doc { r: root {
                paragraph carry([bold]) { text("a") }
                horizontal_rule
                paragraph carry([italic]) { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let (actual, ..) = transact!(initial, |tr| insert_paragraph_after_unit_selection(&mut tr));
        let new_para = actual.selection.unwrap().head.node;
        assert!(
            actual.projected.carry_modifiers(new_para).is_empty(),
            "a paragraph inserted after a unit block starts with no carry, got {:?}",
            actual.projected.carry_modifiers(new_para)
        );
    }

    #[test]
    fn collapsed_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        transact_fail!(initial, |tr| insert_paragraph_after_unit_selection(&mut tr));
    }

    #[test]
    fn text_range_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 3)
        };
        transact_fail!(initial, |tr| insert_paragraph_after_unit_selection(&mut tr));
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
        transact_fail!(initial, |tr| insert_paragraph_after_unit_selection(&mut tr));
    }
}
