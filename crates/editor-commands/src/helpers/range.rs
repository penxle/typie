use editor_model::{Doc, Node, NodeId};
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
    let from_path = doc
        .node(from_start_id)
        .ok_or(CommandError::NodeNotFound(from_start_id))?
        .path();
    let to_path = doc
        .node(to_end_id)
        .ok_or(CommandError::NodeNotFound(to_end_id))?
        .path();

    let mut result = Vec::new();
    for desc in doc.root().expect("root must exist").descendants() {
        if !matches!(desc.node(), Node::Text(_)) {
            continue;
        }
        let path = desc.path();
        if path >= from_path && path <= to_path {
            result.push(desc.id());
        }
    }

    Ok(result)
}

/// Compact affected textblocks and restore selection from absolute text offsets.
pub(crate) fn compact_and_restore_selection(
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

    let sel_offsets = selection_offsets_in_textblocks(&doc, node_ids);

    for tb_id in &textblock_ids {
        let doc = tr.doc();
        if let Some(tb) = doc.node(*tb_id) {
            tr.apply_steps(compact(&tb))?;
        }
    }

    if let Some((from_tb, from_abs, to_tb, to_abs)) = sel_offsets {
        let doc = tr.doc();
        if let (Some(from_pos), Some(to_pos)) = (
            position_from_text_offset(&doc, from_tb, from_abs, false),
            position_from_text_offset(&doc, to_tb, to_abs, true),
        ) {
            tr.set_selection(Selection::new(from_pos, to_pos))?;
        }
    }

    Ok(())
}

fn selection_offsets_in_textblocks(
    doc: &Doc,
    node_ids: &[NodeId],
) -> Option<(NodeId, usize, NodeId, usize)> {
    let first_id = *node_ids.first()?;
    let last_id = *node_ids.last()?;
    let from_tb = find_ancestor_textblock(doc, first_id)?;
    let to_tb = find_ancestor_textblock(doc, last_id)?;
    let from_abs = text_offset_in_textblock(doc, from_tb, first_id, 0)?;
    let node_len = match doc.node(last_id)?.node() {
        Node::Text(t) => t.text.len(),
        _ => 0,
    };
    let to_abs = text_offset_in_textblock(doc, to_tb, last_id, node_len)?;
    Some((from_tb, from_abs, to_tb, to_abs))
}

fn text_offset_in_textblock(
    doc: &Doc,
    tb_id: NodeId,
    node_id: NodeId,
    local_offset: usize,
) -> Option<usize> {
    let tb = doc.node(tb_id)?;
    let mut abs = 0;
    for child in tb.children() {
        if child.id() == node_id {
            return Some(abs + local_offset);
        }
        if let Node::Text(t) = child.node() {
            abs += t.text.len();
        }
    }
    None
}

fn position_from_text_offset(
    doc: &Doc,
    tb_id: NodeId,
    abs_offset: usize,
    end_bias: bool,
) -> Option<Position> {
    let tb = doc.node(tb_id)?;
    let mut remaining = abs_offset;
    for child in tb.children() {
        if let Node::Text(t) = child.node() {
            let len = t.text.len();
            let fits = if end_bias {
                remaining <= len
            } else {
                remaining < len
            };
            if fits {
                return Some(Position::new(child.id(), remaining));
            }
            remaining -= len;
        }
    }
    tb.children().last().map(|child| {
        let len = match child.node() {
            Node::Text(t) => t.text.len(),
            _ => 0,
        };
        Position::new(child.id(), len)
    })
}
