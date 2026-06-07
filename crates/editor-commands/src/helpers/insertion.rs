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

    let host_paragraph_id = find_enclosing_paragraph_id(&doc, pos.node_id);
    let host_marker: Option<editor_model::Marker> = host_paragraph_id
        .and_then(|id| doc.node(id))
        .and_then(|p| p.marker().cloned());
    let host_is_empty = host_paragraph_id
        .and_then(|id| doc.node(id))
        .map(|p| !p.children().any(|c| matches!(c.node(), Node::Text(_))))
        .unwrap_or(false);
    let marker_style: Option<String> = if host_is_empty {
        host_marker.as_ref().and_then(|m| m.style.clone())
    } else {
        None
    };

    let pending_style_explicit = tr.pending_style().is_some();
    let pending_style: Option<String> = match tr.pending_style() {
        Some(editor_state::PendingStyle::Set { style_id }) => Some(style_id.clone()),
        Some(editor_state::PendingStyle::Unset) => None,
        None => doc
            .node(pos.node_id)
            .and_then(|n| n.entry().style.get().clone())
            .or_else(|| marker_style.clone()),
    };

    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let mut effective_mods = resolve_effective_modifiers(&node, pos.offset, tr.pending_modifiers());
    effective_mods.retain(|m| is_text_applicable(m.as_type()));
    let insert_len = text.char_count();

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

    if let Some(marker) = &host_marker {
        for m in &marker.modifiers {
            if !is_text_applicable(m.as_type()) {
                continue;
            }
            if !effective_mods.iter().any(|e| e.as_type() == m.as_type()) {
                effective_mods.push(m.clone());
            }
        }
    }

    match node.node() {
        Node::Text(text_node) => {
            let mut node_mods: Vec<Modifier> = node.modifiers().cloned().collect();
            node_mods.sort_by_key(|m| m.as_type());
            let mut effective_sorted = effective_mods.clone();
            effective_sorted.sort_by_key(|m| m.as_type());
            if effective_sorted == node_mods
                && doc
                    .node(pos.node_id)
                    .and_then(|n| n.entry().style.get().clone())
                    == pending_style
            {
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

                if let Some(style_id) = pending_style.clone() {
                    tr.set_node_style(new_id, Some(style_id))?;
                }

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

            if let Some(style_id) = pending_style.clone() {
                tr.set_node_style(new_id, Some(style_id))?;
            }

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

    if pending_style_explicit {
        tr.set_pending_style(None)?;
    }

    if let Some(p_id) = host_paragraph_id
        && host_marker.is_some()
    {
        tr.set_marker(p_id, None)?;
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
        let marker = editor_model::Marker {
            modifiers: carryable,
            style: None,
        };
        if !marker.is_empty() {
            tr.set_marker(p_id, Some(marker))?;
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

    let host_paragraph_id = find_enclosing_paragraph_id(&doc, pos.node_id);
    let host_marker: Option<editor_model::Marker> = host_paragraph_id
        .and_then(|id| doc.node(id))
        .filter(|p| !p.children().any(|c| matches!(c.node(), Node::Text(_))))
        .and_then(|p| p.marker().cloned());
    let marker_style: Option<String> = host_marker.as_ref().and_then(|m| m.style.clone());

    let pending_style_explicit = tr.pending_style().is_some();
    let pending_style: Option<String> = match tr.pending_style() {
        Some(editor_state::PendingStyle::Set { style_id }) => Some(style_id.clone()),
        Some(editor_state::PendingStyle::Unset) => None,
        None => doc
            .node(pos.node_id)
            .and_then(|n| n.entry().style.get().clone())
            .or_else(|| marker_style.clone()),
    };

    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let mut metric_mods = resolve_effective_modifiers(&node, pos.offset, tr.pending_modifiers());
    metric_mods.retain(|m| is_tab_metric_modifier(m.as_type()));

    // An empty host paragraph's marker holds the next-input formatting; fold its tab-metric
    // modifiers into the tab (the resolver no longer surfaces the marker post de-overload).
    if let Some(marker) = &host_marker {
        for m in &marker.modifiers {
            if is_tab_metric_modifier(m.as_type())
                && !metric_mods.iter().any(|e| e.as_type() == m.as_type())
            {
                metric_mods.push(m.clone());
            }
        }
    }

    let carryable = carryable_modifiers_at(&doc, pos, tr.pending_modifiers());

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

    if let Some(style_id) = pending_style {
        tr.set_node_style(tab_id, Some(style_id))?;
    }

    if pending_style_explicit {
        tr.set_pending_style(None)?;
    }

    if let Some(p_id) = host_paragraph_id
        && host_marker.is_some()
    {
        tr.set_marker(p_id, None)?;
    }

    if let Some(p_id) = host_paragraph_id
        && host_marker.is_none()
    {
        let marker = editor_model::Marker {
            modifiers: carryable,
            style: None,
        };
        if !marker.is_empty() {
            tr.set_marker(p_id, Some(marker))?;
        }
    }

    Ok(true)
}
