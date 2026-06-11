use editor_model::{Doc, Modifier, ModifierType, Node, NodeId, PlainStyleEntry};
use editor_state::{Position, State};
use editor_transaction::Transaction;

use crate::CommandError;
use crate::helpers::{collect_text_nodes_in_range, compact_textblocks_for_nodes};

/// Collects textblock node ids whose subtree intersects the selection.
/// Includes textblocks that are only partially covered. For a collapsed
/// selection, returns the nearest textblock ancestor of the cursor.
pub(crate) fn collect_textblocks_in_selection(state: &State) -> Vec<NodeId> {
    let Some(sel) = state.selection.as_ref() else {
        return Vec::new();
    };

    if sel.is_collapsed() {
        let Some(node) = state.doc.node(sel.head.node_id) else {
            return Vec::new();
        };
        return node
            .ancestors()
            .find(|n| n.spec().is_textblock())
            .map(|n| vec![n.id()])
            .unwrap_or_default();
    }

    let Some(rs) = sel.resolve(&state.doc) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    rs.visit_intersecting_nodes(|node| {
        if node.spec().is_textblock() {
            out.push(node.id());
            return false;
        }
        true
    });
    out
}

/// Collects inline modifiers from text nodes intersecting the selection,
/// returning only those that are uniformly present across every text node.
/// A modifier type is uniform iff every text node in the selection has an
/// explicit modifier of that type with the same value. Mixed or partially
/// present modifiers are dropped. For a collapsed selection, returns the
/// explicit modifiers of the text node at the caret.
pub(crate) fn collect_uniform_text_modifiers_in_selection(state: &State) -> Vec<Modifier> {
    use std::collections::BTreeMap;

    let Some(sel) = state.selection.as_ref() else {
        return Vec::new();
    };

    if sel.is_collapsed() {
        let Some(node) = state.doc.node(sel.head.node_id) else {
            return Vec::new();
        };
        if !matches!(node.node(), Node::Text(_)) {
            return Vec::new();
        }
        return node.explicit_modifiers().cloned().collect();
    }

    let Some(rs) = sel.resolve(&state.doc) else {
        return Vec::new();
    };
    let mut text_nodes = Vec::new();
    rs.for_each_text_node(|node, _span| text_nodes.push(node));
    if text_nodes.is_empty() {
        return Vec::new();
    }

    enum Agg {
        Uniform(Modifier),
        Mixed,
    }
    let mut by_type: BTreeMap<ModifierType, Agg> = BTreeMap::new();
    let mut seen_count: BTreeMap<ModifierType, usize> = BTreeMap::new();
    let total = text_nodes.len();

    for node in &text_nodes {
        for m in node.explicit_modifiers() {
            let ty = m.as_type();
            *seen_count.entry(ty).or_insert(0) += 1;
            match by_type.get(&ty) {
                None => {
                    by_type.insert(ty, Agg::Uniform(m.clone()));
                }
                Some(Agg::Uniform(existing)) if existing == m => {}
                Some(Agg::Uniform(_)) => {
                    by_type.insert(ty, Agg::Mixed);
                }
                Some(Agg::Mixed) => {}
            }
        }
    }

    by_type
        .into_iter()
        .filter_map(|(ty, agg)| {
            if *seen_count.get(&ty).unwrap_or(&0) < total {
                return None;
            }
            match agg {
                Agg::Uniform(m) => Some(m),
                Agg::Mixed => None,
            }
        })
        .collect()
}

/// Removes inline modifiers whose type is in `types` from text nodes within
/// the current range selection. No-op for collapsed selections.
pub(crate) fn clear_inline_modifier_types_in_selection(
    tr: &mut Transaction,
    types: &[ModifierType],
) -> Result<bool, CommandError> {
    if types.is_empty() {
        return Ok(false);
    }
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.is_collapsed() {
        return Ok(false);
    }

    let (from, to) = {
        let doc = tr.doc();
        let resolved = selection
            .resolve(&doc)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        (
            Position::from(resolved.from()),
            Position::from(resolved.to()),
        )
    };

    let node_ids = collect_text_nodes_in_range(tr, &from, &to)?;
    if node_ids.is_empty() {
        return Ok(false);
    }

    let mut changed = false;
    for &node_id in &node_ids {
        let to_remove: Vec<Modifier> = {
            let doc = tr.doc();
            let node = doc
                .node(node_id)
                .ok_or(CommandError::NodeNotFound(node_id))?;
            node.explicit_modifiers()
                .filter(|m| types.contains(&m.as_type()))
                .cloned()
                .collect()
        };
        for modifier in to_remove {
            tr.remove_modifier(node_id, modifier)?;
            changed = true;
        }
    }

    compact_textblocks_for_nodes(tr, &node_ids)?;
    Ok(changed)
}

/// Inline run (text/tab) node ids intersecting the current range selection,
/// with boundary splits applied. Empty for a collapsed selection.
pub(crate) fn collect_run_nodes_in_selection(
    tr: &mut Transaction,
) -> Result<Vec<NodeId>, CommandError> {
    let Some(selection) = tr.selection() else {
        return Ok(Vec::new());
    };
    if selection.is_collapsed() {
        return Ok(Vec::new());
    }
    let (from, to) = {
        let doc = tr.doc();
        let resolved = selection
            .resolve(&doc)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        (
            Position::from(resolved.from()),
            Position::from(resolved.to()),
        )
    };
    collect_text_nodes_in_range(tr, &from, &to)
}

/// Snapshot of a style entry's current values, or `None` if the style is not
/// present (no entry in `Doc.styles`). Useful as the `old` capture for
/// `Step::SetStyle` and as the read path for command-level edits.
pub(crate) fn capture_style_entry(doc: &Doc, style_id: &str) -> Option<PlainStyleEntry> {
    if !doc.style_present(style_id) {
        return None;
    }
    let entry = doc.style_entry(style_id)?;
    Some(PlainStyleEntry {
        name: entry.name.get().clone(),
        modifiers: entry.modifiers.iter().cloned().collect(),
    })
}
