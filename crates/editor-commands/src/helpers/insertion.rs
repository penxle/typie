use editor_common::StrExt;
use editor_model::{
    Modifier, Node, NodeId, PlainHardBreakNode, PlainNode, PlainTabNode, PlainTextNode, Subtree,
};
use editor_state::{Affinity, PendingModifiers, Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::{
    carryable_modifiers_at, find_enclosing_paragraph_id, is_tab_metric_modifier,
    is_text_applicable, resolve_effective_modifiers,
};
use crate::{CommandError, CommandResult};

pub(crate) fn insert_text_at_caret(tr: &mut Transaction, text: &str) -> CommandResult {
    if text.is_empty() {
        return Err(CommandError::InvalidArgument(
            "text must not be empty".into(),
        ));
    }

    if text.contains(['\n', '\r']) {
        return Err(CommandError::InvalidArgument(
            "text must not contain newlines".into(),
        ));
    }

    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    let doc = tr.doc();

    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let mut effective_mods = resolve_effective_modifiers(&node, pos.offset, tr.pending_modifiers());
    effective_mods.retain(|m| is_text_applicable(m.as_type()));
    let insert_len = text.char_count();

    let host_paragraph_id = find_enclosing_paragraph_id(&doc, pos.node_id);

    if let Some(p_id) = host_paragraph_id
        && p_id != pos.node_id
        && let Some(p_node) = doc.node(p_id)
    {
        for m in p_node.modifiers() {
            if !is_text_applicable(m.as_type()) {
                continue;
            }
            if !effective_mods.iter().any(|e| e.as_type() == m.as_type()) {
                effective_mods.push(m.clone());
            }
        }
    }

    let marker_to_clear: Vec<Modifier> = host_paragraph_id
        .and_then(|id| doc.node(id))
        .map(|p| {
            p.modifiers()
                .filter(|m| is_text_applicable(m.as_type()))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    match node.node() {
        Node::Text(text_node) => {
            let mut node_mods: Vec<Modifier> = node.modifiers().cloned().collect();
            node_mods.sort_by_key(|m| m.as_type());
            let mut effective_sorted = effective_mods.clone();
            effective_sorted.sort_by_key(|m| m.as_type());
            if effective_sorted == node_mods {
                tr.insert_text(pos.node_id, pos.offset, text)?;
                tr.set_selection(Some(Selection::collapsed(Position {
                    node_id: pos.node_id,
                    offset: pos.offset + insert_len,
                    affinity: Affinity::Upstream,
                })))?;
            } else {
                let parent = node.parent().ok_or(CommandError::NoParent(pos.node_id))?;
                let node_index = node
                    .index()
                    .ok_or(CommandError::orphan_child(pos.node_id, parent.id()))?;

                let new_id = NodeId::new();
                let subtree = Subtree::leaf(new_id, PlainNode::Text(PlainTextNode::default()))
                    .with_modifiers(effective_mods);

                if pos.offset == 0 {
                    tr.insert_subtree(parent.id(), node_index, subtree)?;
                } else if pos.offset == text_node.text.len() {
                    tr.insert_subtree(parent.id(), node_index + 1, subtree)?;
                } else {
                    let split_id = NodeId::new();
                    tr.split_node(pos.node_id, pos.offset, split_id)?;
                    tr.insert_subtree(parent.id(), node_index + 1, subtree)?;
                }
                tr.insert_text(new_id, 0, text)?;

                tr.set_selection(Some(Selection::collapsed(Position {
                    node_id: new_id,
                    offset: insert_len,
                    affinity: Affinity::Upstream,
                })))?;
            }
        }
        _ => {
            // Case 3: non-text node (empty paragraph, etc.)
            let new_id = NodeId::new();
            let subtree = Subtree::leaf(new_id, PlainNode::Text(PlainTextNode::default()))
                .with_modifiers(effective_mods);

            tr.insert_subtree(pos.node_id, pos.offset, subtree)?;
            tr.insert_text(new_id, 0, text)?;
            tr.set_selection(Some(Selection::collapsed(Position {
                node_id: new_id,
                offset: insert_len,
                affinity: Affinity::Upstream,
            })))?;
        }
    }

    if !tr.pending_modifiers().is_empty() {
        tr.set_pending_modifiers(PendingModifiers::new())?;
    }

    if let Some(p_id) = host_paragraph_id {
        for m in marker_to_clear {
            tr.remove_modifier(p_id, m)?;
        }
    }

    Ok(true)
}

pub(crate) fn insert_hard_break_at_caret(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    let doc = tr.doc();

    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let carryable = carryable_modifiers_at(&doc, pos, tr.pending_modifiers());
    let host_paragraph_id = find_enclosing_paragraph_id(&doc, pos.node_id);

    let break_id = NodeId::new();
    let break_subtree = Subtree::leaf(
        break_id,
        PlainNode::HardBreak(PlainHardBreakNode::default()),
    );

    match node.node() {
        Node::Text(text_node) => {
            let parent = node.parent().ok_or(CommandError::NoParent(pos.node_id))?;
            let node_index = node
                .index()
                .ok_or(CommandError::orphan_child(pos.node_id, parent.id()))?;
            let text_len = text_node.text.len();

            if pos.offset == 0 {
                // Case B: cursor at start of text → insert hard break before
                tr.insert_subtree(parent.id(), node_index, break_subtree)?;
                tr.set_selection(Some(Selection::collapsed(Position {
                    node_id: pos.node_id,
                    offset: 0,
                    affinity: Affinity::Downstream,
                })))?;
            } else if pos.offset == text_len {
                // Case C: cursor at end of text → insert hard break after
                tr.insert_subtree(parent.id(), node_index + 1, break_subtree)?;

                let doc = tr.doc();
                let break_node = doc
                    .node(break_id)
                    .ok_or(CommandError::NodeNotFound(break_id))?;

                if let Some(next) = break_node.next_sibling() {
                    if matches!(next.node(), Node::Text(_)) {
                        tr.set_selection(Some(Selection::collapsed(Position {
                            node_id: next.id(),
                            offset: 0,
                            affinity: Affinity::Downstream,
                        })))?;
                    } else {
                        let idx = next
                            .index()
                            .ok_or(CommandError::orphan_child(next.id(), parent.id()))?;
                        tr.set_selection(Some(Selection::collapsed(Position {
                            node_id: parent.id(),
                            offset: idx,
                            affinity: Affinity::Downstream,
                        })))?;
                    }
                } else {
                    let break_idx = break_node
                        .index()
                        .ok_or(CommandError::orphan_child(break_id, parent.id()))?;
                    tr.set_selection(Some(Selection::collapsed(Position {
                        node_id: parent.id(),
                        offset: break_idx + 1,
                        affinity: Affinity::Downstream,
                    })))?;
                }
            } else {
                // Case A: cursor in middle of text → split, insert hard break between
                let split_id = NodeId::new();
                tr.split_node(pos.node_id, pos.offset, split_id)?;
                tr.insert_subtree(parent.id(), node_index + 1, break_subtree)?;
                tr.set_selection(Some(Selection::collapsed(Position {
                    node_id: split_id,
                    offset: 0,
                    affinity: Affinity::Downstream,
                })))?;
            }
        }
        _ => {
            // Case D: non-text node (empty paragraph, etc.)
            tr.insert_subtree(pos.node_id, pos.offset, break_subtree)?;
            tr.set_selection(Some(Selection::collapsed(Position {
                node_id: pos.node_id,
                offset: pos.offset + 1,
                affinity: Affinity::Downstream,
            })))?;
        }
    }

    if let Some(p_id) = host_paragraph_id {
        for m in carryable {
            tr.add_modifier(p_id, m)?;
        }
    }

    Ok(true)
}

pub(crate) fn insert_tab_at_caret(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    let doc = tr.doc();

    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let mut metric_mods = resolve_effective_modifiers(&node, pos.offset, tr.pending_modifiers());
    metric_mods.retain(|m| is_tab_metric_modifier(m.as_type()));

    let carryable = carryable_modifiers_at(&doc, pos, tr.pending_modifiers());
    let host_paragraph_id = find_enclosing_paragraph_id(&doc, pos.node_id);

    let tab_id = NodeId::new();
    let tab_subtree =
        Subtree::leaf(tab_id, PlainNode::Tab(PlainTabNode::default())).with_modifiers(metric_mods);

    match node.node() {
        Node::Text(text_node) => {
            let parent = node.parent().ok_or(CommandError::NoParent(pos.node_id))?;
            let node_index = node
                .index()
                .ok_or(CommandError::orphan_child(pos.node_id, parent.id()))?;
            let text_len = text_node.text.len();

            if pos.offset == 0 {
                tr.insert_subtree(parent.id(), node_index, tab_subtree)?;
                tr.set_selection(Some(Selection::collapsed(Position {
                    node_id: pos.node_id,
                    offset: 0,
                    affinity: Affinity::Downstream,
                })))?;
            } else if pos.offset == text_len {
                tr.insert_subtree(parent.id(), node_index + 1, tab_subtree)?;
                let doc = tr.doc();
                let tab_node = doc.node(tab_id).ok_or(CommandError::NodeNotFound(tab_id))?;
                if let Some(next) = tab_node.next_sibling() {
                    if matches!(next.node(), Node::Text(_)) {
                        tr.set_selection(Some(Selection::collapsed(Position {
                            node_id: next.id(),
                            offset: 0,
                            affinity: Affinity::Downstream,
                        })))?;
                    } else {
                        let idx = next
                            .index()
                            .ok_or(CommandError::orphan_child(next.id(), parent.id()))?;
                        tr.set_selection(Some(Selection::collapsed(Position {
                            node_id: parent.id(),
                            offset: idx,
                            affinity: Affinity::Downstream,
                        })))?;
                    }
                } else {
                    let tab_idx = tab_node
                        .index()
                        .ok_or(CommandError::orphan_child(tab_id, parent.id()))?;
                    tr.set_selection(Some(Selection::collapsed(Position {
                        node_id: parent.id(),
                        offset: tab_idx + 1,
                        affinity: Affinity::Downstream,
                    })))?;
                }
            } else {
                let split_id = NodeId::new();
                tr.split_node(pos.node_id, pos.offset, split_id)?;
                tr.insert_subtree(parent.id(), node_index + 1, tab_subtree)?;
                tr.set_selection(Some(Selection::collapsed(Position {
                    node_id: split_id,
                    offset: 0,
                    affinity: Affinity::Downstream,
                })))?;
            }
        }
        _ => {
            tr.insert_subtree(pos.node_id, pos.offset, tab_subtree)?;
            tr.set_selection(Some(Selection::collapsed(Position {
                node_id: pos.node_id,
                offset: pos.offset + 1,
                affinity: Affinity::Downstream,
            })))?;
        }
    }

    if let Some(p_id) = host_paragraph_id {
        for m in carryable {
            tr.add_modifier(p_id, m)?;
        }
    }

    Ok(true)
}
