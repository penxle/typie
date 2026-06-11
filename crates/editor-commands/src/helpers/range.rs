use editor_model::{Node, NodeId};
use editor_state::{Position, Selection};
use editor_transaction::{Transaction, compact};

use crate::CommandError;
use crate::helpers::find_ancestor_textblock;

/// Collect text leaf nodes in range [from, to]. Splits boundary nodes if needed.
/// Returns node IDs of text nodes fully within the range after splitting.
pub(crate) fn collect_text_nodes_in_range(
    tr: &mut Transaction,
    from: &Position,
    to: &Position,
) -> Result<Vec<NodeId>, CommandError> {
    let doc = tr.doc();

    // Split 'to' node first (splitting 'from' first would shift 'to' position)
    let to_node = doc
        .node(to.node_id)
        .ok_or(CommandError::NodeNotFound(to.node_id))?;
    let to_node_len = match to_node.node() {
        Node::Text(t) => t.text.len(),
        _ => 0,
    };
    let to_needs_split =
        matches!(to_node.node(), Node::Text(_)) && to.offset > 0 && to.offset < to_node_len;

    if to_needs_split {
        let split_id = NodeId::new();
        tr.split_node(to.node_id, to.offset, split_id)?;
    }

    let doc = tr.doc();
    let from_node = doc
        .node(from.node_id)
        .ok_or(CommandError::NodeNotFound(from.node_id))?;
    let from_node_len = match from_node.node() {
        Node::Text(t) => t.text.len(),
        _ => 0,
    };
    let from_needs_split =
        matches!(from_node.node(), Node::Text(_)) && from.offset > 0 && from.offset < from_node_len;

    let (from_start_id, to_end_id) = if from_needs_split {
        let split_id = NodeId::new();
        tr.split_node(from.node_id, from.offset, split_id)?;
        // If from and to pointed to the same node, the tail after split is now split_id
        let to_id = if to.node_id == from.node_id {
            split_id
        } else {
            to.node_id
        };
        (split_id, to_id)
    } else {
        (from.node_id, to.node_id)
    };

    let doc = tr.doc();
    let from_pos = if from_needs_split {
        Position::new(from_start_id, 0)
    } else {
        *from
    };
    let to_end_node = doc
        .node(to_end_id)
        .ok_or(CommandError::NodeNotFound(to_end_id))?;
    let to_pos = match to_end_node.node() {
        Node::Text(t) => {
            let offset = if to_needs_split {
                t.text.len()
            } else {
                to.offset.min(t.text.len())
            };
            Position::new(to_end_id, offset)
        }
        _ => Position::new(to_end_id, to.offset),
    };

    let resolved = Selection::new(from_pos, to_pos)
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
    let mut result = Vec::new();
    resolved.visit_intersecting_nodes(|node| match node.node() {
        Node::Text(_) => {
            if resolved.text_span_for_node(&node).is_some() {
                result.push(node.id());
            }
            false
        }
        Node::Tab(_) => {
            if resolved.contains_subtree(&node) {
                result.push(node.id());
            }
            false
        }
        _ => true,
    });
    Ok(result)
}

pub(crate) fn compact_textblock_at_position(
    tr: &mut Transaction,
    pos: Position,
) -> Result<(), CommandError> {
    let doc = tr.doc();
    let Some(node) = doc.node(pos.node_id) else {
        return Ok(());
    };
    let Node::Text(_) = node.node() else {
        return Ok(());
    };
    let Some(tb_id) = find_ancestor_textblock(&doc, pos.node_id) else {
        return Ok(());
    };

    let doc = tr.doc();
    let tb = doc.node(tb_id).ok_or(CommandError::NodeNotFound(tb_id))?;
    tr.apply_steps(compact(&tb))?;
    Ok(())
}

pub(crate) fn compact_textblocks_for_nodes(
    tr: &mut Transaction,
    node_ids: &[NodeId],
) -> Result<(), CommandError> {
    let doc = tr.doc();
    let mut textblock_ids = Vec::new();
    for &node_id in node_ids {
        if let Some(tb_id) = find_ancestor_textblock(&doc, node_id)
            && !textblock_ids.contains(&tb_id)
        {
            textblock_ids.push(tb_id);
        }
    }

    for tb_id in &textblock_ids {
        let doc = tr.doc();
        if let Some(tb) = doc.node(*tb_id) {
            tr.apply_steps(compact(&tb))?;
        }
    }

    Ok(())
}
