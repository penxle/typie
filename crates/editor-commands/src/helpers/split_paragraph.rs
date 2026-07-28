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
