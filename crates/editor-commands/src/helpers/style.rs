use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{Modifier, ModifierType, PlainStyleEntry};
use editor_state::{Position, State, inline_leaf_dots_in_range, leaf_groups_in_range};
use editor_transaction::Transaction;

use crate::CommandError;

/// Collects textblock node ids whose subtree intersects the selection.
/// Includes textblocks that are only partially covered. For a collapsed
/// selection, returns the nearest textblock ancestor of the cursor.
pub(crate) fn collect_textblocks_in_selection(state: &State) -> Vec<Dot> {
    let Some(sel) = state.selection.as_ref() else {
        return Vec::new();
    };
    let view = state.view();

    if sel.anchor == sel.head {
        let Some(node) = view.node(sel.head.node) else {
            return Vec::new();
        };
        return node
            .ancestors()
            .find(|n| n.spec().is_textblock())
            .map(|n| vec![n.id()])
            .unwrap_or_default();
    }

    let Some(rs) = sel.resolve(&view) else {
        return Vec::new();
    };
    let lo = rs.from().position();
    let hi = rs.to().position();
    let (Some(lo_r), Some(hi_r)) = (lo.resolve(&view), hi.resolve(&view)) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    if let Some(root) = view.root() {
        let mut blocks = vec![root];
        for d in view.root().unwrap().descendants() {
            if let editor_model::ChildView::Block(b) = d {
                blocks.push(b);
            }
        }
        for block in blocks {
            if !block.spec().is_textblock() {
                continue;
            }
            let id = block.id();
            let count = block.children().count();
            let (Some(start), Some(end)) = (
                Position::new(id, 0).resolve(&view),
                Position::new(id, count).resolve(&view),
            ) else {
                continue;
            };
            if start <= hi_r && lo_r <= end {
                out.push(id);
            }
        }
    }
    out
}

/// Collects inline modifiers uniformly present across every inline leaf in the
/// selection (same type and value on all). For a collapsed selection, returns
/// the own (non-style) modifiers of the leaf immediately left of the caret.
pub(crate) fn collect_uniform_text_modifiers_in_selection(state: &State) -> Vec<Modifier> {
    let Some(sel) = state.selection.as_ref() else {
        return Vec::new();
    };
    let view = state.view();

    if sel.anchor == sel.head {
        let pos = &sel.head;
        let Some(node) = view.node(pos.node) else {
            return Vec::new();
        };
        let idx = pos.offset.checked_sub(1);
        let leaf_slot = match idx {
            Some(i) if node.child_at(i).is_some() => i,
            _ => pos.offset,
        };
        return match node.leaf_state_at(leaf_slot) {
            Some(st) => st
                .own
                .iter()
                .filter(|(_, o)| !o.from_style)
                .map(|(_, o)| o.value.clone())
                .collect(),
            None => Vec::new(),
        };
    }

    let Some(rs) = sel.resolve(&view) else {
        return Vec::new();
    };
    let groups = leaf_groups_in_range(&rs);
    if groups.is_empty() {
        return Vec::new();
    }

    enum Agg {
        Uniform(Modifier),
        Mixed,
    }
    let mut by_type: BTreeMap<ModifierType, Agg> = BTreeMap::new();
    let mut seen_count: BTreeMap<ModifierType, usize> = BTreeMap::new();
    let total: usize = groups.iter().map(|g| g.count).sum();

    for g in &groups {
        for (ty, own) in g.own {
            if own.from_style {
                continue;
            }
            *seen_count.entry(*ty).or_insert(0) += g.count;
            match by_type.get(ty) {
                None => {
                    by_type.insert(*ty, Agg::Uniform(own.value.clone()));
                }
                Some(Agg::Uniform(existing)) if *existing == own.value => {}
                Some(Agg::Uniform(_)) => {
                    by_type.insert(*ty, Agg::Mixed);
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

/// Removes inline modifiers whose type is in `types` from the inline leaves
/// within the current range selection. No-op for collapsed selections.
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
    if selection.anchor == selection.head {
        return Ok(false);
    }

    let to_remove: Vec<(Dot, Dot, Modifier)> = {
        let view = tr.state().view();
        let Some(resolved) = selection.resolve(&view) else {
            return Err(CommandError::Corrupted("cannot resolve selection".into()));
        };
        let mut acc = Vec::new();
        // Each group is uniform in its own modifiers, so a single range span op
        // clears the type across all of the group's leaves.
        for g in leaf_groups_in_range(&resolved) {
            for (ty, own) in g.own {
                if own.from_style {
                    continue;
                }
                if types.contains(ty) {
                    acc.push((g.first, g.last, own.value.clone()));
                }
            }
        }
        acc
    };

    if to_remove.is_empty() {
        return Ok(false);
    }

    let mut changed = false;
    for (first, last, modifier) in to_remove {
        tr.remove_span_modifier(first, last, modifier)?;
        changed = true;
    }
    Ok(changed)
}

/// Inline leaf dots intersecting the current range selection. Empty for a
/// collapsed selection.
pub(crate) fn collect_run_nodes_in_selection(
    tr: &mut Transaction,
) -> Result<Vec<Dot>, CommandError> {
    let Some(selection) = tr.selection() else {
        return Ok(Vec::new());
    };
    if selection.anchor == selection.head {
        return Ok(Vec::new());
    }
    let view = tr.state().view();
    let resolved = selection
        .resolve(&view)
        .ok_or_else(|| CommandError::Corrupted("cannot resolve selection".into()))?;
    let from = resolved.from().position();
    let to = resolved.to().position();
    Ok(inline_leaf_dots_in_range(&view, &from, &to))
}

/// Snapshot of a style entry's current values, or `None` if the style is not
/// registered.
pub(crate) fn capture_style_entry(state: &State, style_id: &str) -> Option<PlainStyleEntry> {
    let log = state.projected.styles();
    if !log.registered(style_id) {
        return None;
    }
    let entry = log.style_entry(style_id)?;
    Some(PlainStyleEntry {
        name: entry.name.get().clone(),
        modifiers: entry.modifiers.iter().cloned().collect(),
    })
}
