use editor_model::{NodeId, NodeType, PlainNode, PlainParagraphNode, Subtree};
use editor_state::{Affinity, GapCursor, Position, Selection};
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
    let selection = tr.selection();
    let doc = tr.doc();

    let (parent_id, index): (NodeId, usize) =
        match selection.resolve(&doc).and_then(|rs| rs.as_gap_cursor()) {
            None => return Ok(false),
            // Leading-unit gap: caret before the document's first child
            // unit. The slot is index 0 under the document root.
            Some(GapCursor::LeadingUnit { .. }) => (NodeId::ROOT, 0),
            // Between-monolithic gap: the slot is at `index` under
            // `parent`, between the two adjacent monolithic siblings.
            Some(GapCursor::BetweenMonolithic { parent, index, .. }) => (parent.id(), index),
        };

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
    tr.insert_subtree(parent_id, index, subtree)?;

    tr.set_selection(Selection::collapsed(Position {
        node_id: new_para_id,
        offset: 0,
        affinity: Affinity::Downstream,
    }))?;

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
            doc { root { paragraph { t: text("hi") } } }
            selection: (t, 1)
        };
        let (actual, ..) = transact_fail!(initial, |tr| materialize_gap_paragraph(&mut tr));
        let (expected, ..) = state! {
            doc { root { paragraph { t: text("hi") } } }
            selection: (t, 1)
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
}
