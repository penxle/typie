use editor_model::NodeType;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use super::{continuation_paint_at, materialize_caret_block};
use crate::{CommandError, CommandResult};

pub(crate) fn split_paragraph_at_selection(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    {
        let view = tr.state().view();
        let node = view
            .node(selection.head.node)
            .ok_or(CommandError::NodeNotFound(selection.head.node))?;
        if node.node_type() != NodeType::Paragraph {
            return Ok(false);
        }
    }

    materialize_caret_block(tr)?;

    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let pos = selection.head;

    let (parent_id, block_index, paint) = {
        let view = tr.state().view();
        let node = view
            .node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;
        if node.node_type() != NodeType::Paragraph {
            return Ok(false);
        }
        let parent = node.parent().ok_or(CommandError::NoParent(pos.node))?;
        let parent_id = parent.id();
        let block_index = parent
            .child_blocks()
            .position(|b| b.id() == pos.node)
            .ok_or_else(|| CommandError::orphan_child(pos.node, parent_id))?;
        let paint = continuation_paint_at(&tr.state().projected, pos);
        (parent_id, block_index, paint)
    };

    tr.split_node(pos.node, pos.offset)?;

    let new_paragraph_id = {
        let view = tr.state().view();
        let parent = view
            .node(parent_id)
            .ok_or(CommandError::NodeNotFound(parent_id))?;
        parent
            .child_blocks()
            .nth(block_index + 1)
            .map(|b| b.id())
            .ok_or_else(|| CommandError::Corrupted("split produced no new sibling".into()))?
    };

    tr.replace_carry(pos.node, paint.clone())?;
    tr.replace_carry(new_paragraph_id, paint)?;

    tr.set_selection(Some(Selection::collapsed(Position {
        node: new_paragraph_id,
        offset: 0,
        affinity: Affinity::Downstream,
    })))?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_crdt::Dot;
    use editor_macros::state;
    use editor_model::{EditOp, ModifierType};
    use editor_state::undo::RecordedOp;

    use super::*;
    use crate::test_utils::*;

    fn carry_ops(recorded: &[RecordedOp]) -> Vec<(Dot, ModifierType)> {
        recorded
            .iter()
            .filter_map(|r| match &r.op.payload {
                EditOp::NodeCarry(op) => Some(op.target_key()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn split_at_end_records_no_carry_op_for_either_block() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
        };
        let (actual, _, recorded, ..) =
            transact!(initial, |tr| split_paragraph_at_selection(&mut tr));
        let new_paragraph = actual
            .view()
            .node(Dot::ROOT)
            .expect("root")
            .child_blocks()
            .nth(1)
            .expect("split produced a sibling")
            .id();

        assert_eq!(carry_ops(&recorded), Vec::new());
        assert!(actual.projected.carry_modifiers(p1).is_empty());
        assert!(actual.projected.carry_modifiers(new_paragraph).is_empty());
    }

    #[test]
    fn split_at_end_records_carry_op_only_for_the_new_block() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph carry([bold]) { text("Hi") [bold] } } }
            selection: (p1, 2)
        };
        let (actual, _, recorded, ..) =
            transact!(initial, |tr| split_paragraph_at_selection(&mut tr));
        let new_paragraph = actual
            .view()
            .node(Dot::ROOT)
            .expect("root")
            .child_blocks()
            .nth(1)
            .expect("split produced a sibling")
            .id();

        assert_eq!(
            carry_ops(&recorded),
            vec![(new_paragraph, ModifierType::Bold)]
        );
        assert_eq!(
            actual
                .projected
                .carry_modifiers(p1)
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![ModifierType::Bold]
        );
        assert_eq!(
            actual
                .projected
                .carry_modifiers(new_paragraph)
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![ModifierType::Bold]
        );
    }
}
