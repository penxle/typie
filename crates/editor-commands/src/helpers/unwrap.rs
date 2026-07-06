use editor_crdt::Dot;
use editor_model::NodeType;
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::child_node_type;
use crate::{CommandError, CommandResult};

pub(crate) fn unwrap_block_wrapper(tr: &mut Transaction, node_id: Dot) -> CommandResult {
    let (parent_id, node_index, children) = {
        let view = tr.state().view();
        let node = view
            .node(node_id)
            .ok_or(CommandError::NodeNotFound(node_id))?;
        let parent = node.parent().ok_or(CommandError::NoParent(node_id))?;
        let parent_id = parent.id();
        let parent_spec = parent.spec();
        let node_index = node
            .index()
            .ok_or_else(|| CommandError::Corrupted("wrapper has no index".into()))?;

        let children: Vec<(Dot, NodeType)> = node
            .child_blocks()
            .map(|c| (c.id(), c.node_type()))
            .collect();
        if children.is_empty() {
            return Ok(false);
        }

        let mut new_seq: Vec<NodeType> = parent.children().map(|c| child_node_type(&c)).collect();
        new_seq.remove(node_index);
        for (i, (_, t)) in children.iter().enumerate() {
            new_seq.insert(node_index + i, *t);
        }
        if !parent_spec.content.matches_sequence(&new_seq) {
            return Ok(false);
        }
        (parent_id, node_index, children)
    };

    tr.batch::<_, CommandError>(|tr| {
        for (i, (child_id, _)) in children.iter().enumerate() {
            tr.move_node(*child_id, parent_id, node_index + 1 + i)?;
        }
        tr.remove_subtree(node_id)?;
        Ok(())
    })?;

    let first_lifted = {
        let view = tr.state().view();
        view.node(parent_id)
            .and_then(|parent| match parent.child_at(node_index) {
                Some(editor_model::ChildView::Block(block)) => Some(block.id()),
                _ => None,
            })
    };
    if let Some(block_id) = first_lifted {
        place_caret_at_block_start(tr, block_id)?;
    }
    Ok(true)
}

pub(crate) fn place_caret_at_block_start(
    tr: &mut Transaction,
    block_id: Dot,
) -> Result<(), CommandError> {
    let view = tr.state().view();
    if view.node(block_id).is_none() {
        return Ok(());
    }
    let pos = Position {
        node: block_id,
        offset: 0,
        affinity: Affinity::Downstream,
    };
    tr.set_selection(Some(Selection::collapsed(pos)))?;
    Ok(())
}
