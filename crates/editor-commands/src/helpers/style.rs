#![allow(dead_code)]

use editor_model::{Doc, Modifier, ModifierType, Node, NodeId, NodeRef, PlainStyleEntry};
use editor_state::{Position, ResolvedSelection, State};
use editor_transaction::Transaction;

use crate::CommandError;
use crate::helpers::{collect_text_nodes_in_range, compact_and_restore_selection};

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
    let Some(root) = state.doc.root() else {
        return Vec::new();
    };

    let mut out = Vec::new();
    walk_textblocks(&root, &rs, &mut out);
    out
}

fn walk_textblocks<'a>(node: &NodeRef<'a>, rs: &ResolvedSelection<'a>, out: &mut Vec<NodeId>) {
    if !rs.intersects_subtree(node) {
        return;
    }
    if node.spec().is_textblock() {
        out.push(node.id());
        return;
    }
    for child in node.children() {
        walk_textblocks(&child, rs, out);
    }
}

fn walk_text_nodes<'a>(node: &NodeRef<'a>, rs: &ResolvedSelection<'a>, out: &mut Vec<NodeRef<'a>>) {
    if !rs.intersects_subtree(node) {
        return;
    }
    if matches!(node.node(), Node::Text(_)) {
        out.push(*node);
        return;
    }
    for child in node.children() {
        walk_text_nodes(&child, rs, out);
    }
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
    let Some(root) = state.doc.root() else {
        return Vec::new();
    };

    let mut text_nodes: Vec<NodeRef> = Vec::new();
    walk_text_nodes(&root, &rs, &mut text_nodes);
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

    compact_and_restore_selection(tr, &node_ids)?;
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

/// Returns the style ids defined on the document, in lexicographic order.
/// A style is defined iff `Doc.styles` contains it (presence).
pub(crate) fn defined_style_ids(doc: &Doc) -> Vec<String> {
    let mut ids: Vec<String> = doc.styles_iter().map(|(k, _)| k.clone()).collect();
    ids.sort_unstable();
    ids
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

/// Effective modifiers contributed by the style applied to `node`, or empty
/// when no style is applied or the referenced style id is dangling (not in
/// `Doc.styles` or `style_entries`).
pub(crate) fn style_modifiers_for(node: &NodeRef<'_>) -> Vec<Modifier> {
    let doc = node.doc();
    let Some(id) = node.entry().style.get().as_ref() else {
        return Vec::new();
    };
    if !doc.style_present(id) {
        return Vec::new();
    }
    let Some(style) = doc.style_entry(id) else {
        return Vec::new();
    };
    style.modifiers.iter().cloned().collect()
}

/// Effective modifiers for a node combining its own (direct) modifiers with the
/// modifiers contributed by its applied style. Direct modifiers override
/// style-derived modifiers of the same `ModifierType`.
pub(crate) fn effective_modifiers_with_styles(node: &NodeRef<'_>) -> Vec<Modifier> {
    let mut by_type: Vec<(ModifierType, Modifier)> = style_modifiers_for(node)
        .into_iter()
        .map(|m| (m.as_type(), m))
        .collect();
    for m in node.explicit_modifiers() {
        let ty = m.as_type();
        if let Some(pos) = by_type.iter().position(|(t, _)| *t == ty) {
            by_type[pos].1 = m.clone();
        } else {
            by_type.push((ty, m.clone()));
        }
    }
    by_type.into_iter().map(|(_, m)| m).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::state;
    use editor_model::Modifier;

    use crate::commands::define_style;
    use crate::test_utils::*;

    #[test]
    fn style_modifiers_resolve_when_applied() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (with_style, ..) = transact!(state, |tr| define_style(
            &mut tr,
            "heading-1".into(),
            "제목 1".into(),
            vec![Modifier::Bold, Modifier::FontSize { value: 2400 }],
        ));
        let (applied, ..) = transact!(with_style, |tr| tr
            .set_node_style(p1, Some("heading-1".into()))
            .map(|_| true)
            .map_err(crate::CommandError::Step));

        let node = applied.doc.node(p1).unwrap();
        let mods = style_modifiers_for(&node);
        assert!(mods.contains(&Modifier::Bold));
        assert!(mods.contains(&Modifier::FontSize { value: 2400 }));
    }

    #[test]
    fn direct_modifier_overrides_style_modifier() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (with_style, ..) = transact!(state, |tr| define_style(
            &mut tr,
            "heading-1".into(),
            "제목 1".into(),
            vec![Modifier::FontSize { value: 2400 }],
        ));
        let (applied, ..) = transact!(with_style, |tr| tr
            .set_node_style(p1, Some("heading-1".into()))
            .map(|_| true)
            .map_err(crate::CommandError::Step));
        let (after, ..) = transact!(applied, |tr| crate::commands::set_node_modifier(
            &mut tr,
            p1,
            Modifier::FontSize { value: 1600 }
        ));

        let node = after.doc.node(p1).unwrap();
        let mods = effective_modifiers_with_styles(&node);
        assert!(mods.contains(&Modifier::FontSize { value: 1600 }));
        assert!(!mods.contains(&Modifier::FontSize { value: 2400 }));
    }

    #[test]
    fn dangling_style_ref_is_ignored() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (applied, ..) = transact!(state, |tr| tr
            .set_node_style(p1, Some("missing".into()))
            .map(|_| true)
            .map_err(crate::CommandError::Step));
        let node = applied.doc.node(p1).unwrap();
        assert!(style_modifiers_for(&node).is_empty());
    }

    #[test]
    fn defined_style_ids_returns_sorted() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let (defined1, ..) = transact!(state, |tr| define_style(
            &mut tr,
            "heading-2".into(),
            "h2".into(),
            vec![],
        ));
        let (defined2, ..) = transact!(defined1, |tr| define_style(
            &mut tr,
            "heading-1".into(),
            "h1".into(),
            vec![],
        ));
        let ids = defined_style_ids(&defined2.doc);
        assert_eq!(ids, vec!["heading-1".to_string(), "heading-2".to_string()]);
    }
}
