use editor_model::{Node, NodeId, NodeType};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::Transaction;

use crate::{CommandError, CommandResult};

pub(crate) fn unwrap_block_wrapper(tr: &mut Transaction, node_id: NodeId) -> CommandResult {
    let doc = tr.doc();
    let node = doc
        .node(node_id)
        .ok_or(CommandError::NodeNotFound(node_id))?;
    let parent = node.parent().ok_or(CommandError::NoParent(node_id))?;
    let parent_id = parent.id();
    let parent_spec = parent.spec();
    let node_index = node
        .index()
        .ok_or(CommandError::Corrupted("wrapper has no index".into()))?;

    let children: Vec<(NodeId, NodeType)> =
        node.children().map(|c| (c.id(), c.as_type())).collect();
    if children.is_empty() {
        return Ok(false);
    }

    let mut new_seq: Vec<NodeType> = parent.children().map(|c| c.as_type()).collect();
    new_seq.remove(node_index);
    for (i, (_, t)) in children.iter().enumerate() {
        new_seq.insert(node_index + i, *t);
    }
    if !parent_spec.content.matches_sequence(&new_seq) {
        return Ok(false);
    }

    let first_child_id = children[0].0;

    tr.batch::<_, CommandError>(|tr| {
        for (i, (child_id, _)) in children.iter().enumerate() {
            tr.move_node(*child_id, parent_id, node_index + 1 + i)?;
        }
        tr.remove_subtree(node_id)?;
        Ok(())
    })?;

    place_caret_at_block_start(tr, first_child_id)?;
    Ok(true)
}

pub(crate) fn place_caret_at_block_start(
    tr: &mut Transaction,
    block_id: NodeId,
) -> Result<(), CommandError> {
    let doc = tr.doc();
    let Some(block) = doc.node(block_id) else {
        return Ok(());
    };
    let pos = match block.first_child() {
        Some(child) if matches!(child.node(), Node::Text(_)) => Position {
            node_id: child.id(),
            offset: 0,
            affinity: Affinity::Downstream,
        },
        _ => Position {
            node_id: block_id,
            offset: 0,
            affinity: Affinity::Downstream,
        },
    };
    tr.set_selection(Some(Selection::collapsed(pos)))?;
    Ok(())
}
