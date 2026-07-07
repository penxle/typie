use editor_crdt::Dot;
use editor_model::{ChildView, NodeType, PlainNode, PlainParagraphNode, Subtree};
use editor_state::{Affinity, Position, Selection};
use editor_state::{GapCursor, as_gap_cursor};
use editor_transaction::Transaction;

use crate::helpers::is_block_container;
use crate::{CommandError, CommandResult};

/// If the selection is a gap cursor, materialize a real empty paragraph
/// at the gap slot and move the caret into it. Returns `Ok(false)` for a
/// non-gap selection so it composes as the first arm of the existing
/// `first!(...)` insertion pattern (mirrors
/// `insert_paragraph_before_unit_selection`, keyed on `as_gap_cursor`
/// instead of `is_unit_node_selection`). The subsequent insertion
/// command then runs against the fresh paragraph; for a block fragment
/// the existing "insert block into empty paragraph replaces it" behavior
/// yields the block at the gap with no leftover paragraph.
pub fn materialize_gap_paragraph(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    let (parent_id, index): (Dot, usize) = {
        let view = tr.state().view();
        let Some(rs) = selection.resolve(&view) else {
            return Ok(false);
        };
        let (parent, index) = match as_gap_cursor(&rs) {
            None => return Ok(false),
            // Leading-unit gap: caret before the document's first child
            // unit. The slot is index 0 under the document root.
            Some(GapCursor::LeadingUnit { .. }) => (view.root().unwrap(), 0),
            // Between-monolithic gap: the slot is at `index` under
            // `parent`, between the two adjacent monolithic siblings.
            Some(GapCursor::BetweenMonolithic { parent, index, .. }) => (parent, index),
        };
        if !is_block_container(&parent) || !parent.spec().content.matches(NodeType::Paragraph) {
            return Ok(false);
        }
        (parent.id(), index)
    };

    let subtree = Subtree::leaf(PlainNode::Paragraph(PlainParagraphNode::default()));
    tr.insert_subtree(parent_id, index, subtree)?;

    let new_para = {
        let view = tr.state().view();
        match view.node(parent_id).and_then(|p| p.child_at(index)) {
            Some(ChildView::Block(b)) => b.id(),
            _ => {
                return Err(CommandError::Corrupted(
                    "materialized paragraph not found at gap".into(),
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
    fn non_gap_selection_is_a_noop_ok_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 1)
        };
        let (actual, ..) = transact_fail!(initial, |tr| materialize_gap_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn leading_unit_gap_inserts_empty_paragraph_at_index_0() {
        let (initial, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let (actual, ..) = transact!(initial, |tr| materialize_gap_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph image paragraph { text("b") } } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn between_two_folds_gap_inserts_paragraph_at_index_1() {
        let (initial, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 1)
        };
        let (actual, ..) = transact!(initial, |tr| materialize_gap_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                p1: paragraph
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn gap_materialized_paragraph_has_empty_carry() {
        let (initial, _r) = state! {
            doc { r: root { image paragraph carry([bold]) { text("b") } } }
            selection: (r, 0, <)
        };
        let (actual, ..) = transact!(initial, |tr| materialize_gap_paragraph(&mut tr));
        let new_para = actual.selection.unwrap().head.node;
        assert!(
            actual.projected.carry_modifiers(new_para).is_empty(),
            "a paragraph materialized at a gap starts with no carry even next to a carrying sibling, got {:?}",
            actual.projected.carry_modifiers(new_para)
        );
    }

    #[test]
    fn typing_at_gap_before_unit_yields_document_default() {
        let (initial, _r) = state! {
            doc { r: root { image paragraph carry([bold]) { text("b") } } }
            selection: (r, 0, <)
        };
        let mut tr = Transaction::new(&initial);
        assert!(materialize_gap_paragraph(&mut tr).unwrap());
        assert!(crate::commands::insert_text(&mut tr, "가").unwrap());
        let (actual, ..) = tr.commit();
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("가") }
                image
                paragraph carry([bold]) { text("b") }
            } }
            selection: (p1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }
}
