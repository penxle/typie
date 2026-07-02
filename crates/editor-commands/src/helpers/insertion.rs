use std::collections::BTreeMap;

use editor_common::StrExt;
use editor_crdt::Dot;
use editor_model::{
    ChildView, Modifier, ModifierType, NodeView, PlainHardBreakNode, PlainNode, PlainTabNode,
    PlainTextNode, Subtree,
};
use editor_state::{Affinity, PendingModifiers, Position, ProjectedState, Selection};
use editor_transaction::{Step, Transaction};

use crate::helpers::{
    carryable_modifiers_at, find_enclosing_paragraph_id, is_tab_metric_modifier,
    is_text_applicable, resolve_effective_modifiers,
};
use crate::{CommandError, CommandResult};

/// Capture a projected child (block or leaf atom/char) as a `Subtree`, mirroring
/// the substrate's internal capture so the removal is reversible.
fn capture_child_subtree(ps: &ProjectedState, child: &ChildView) -> Subtree {
    match child {
        ChildView::Block(b) => capture_block_subtree(ps, b),
        ChildView::Leaf(l) => {
            let plain = if let Some(ch) = l.as_char() {
                PlainNode::Text(PlainTextNode {
                    text: ch.to_string(),
                })
            } else if let Some(atom) = l.as_atom() {
                atom.clone().into_node().to_plain()
            } else {
                PlainNode::Text(PlainTextNode {
                    text: String::new(),
                })
            };
            Subtree::leaf(plain)
        }
    }
}

fn capture_block_subtree(ps: &ProjectedState, block: &NodeView) -> Subtree {
    let node = block.node().to_plain();
    let dot = block.dot();
    let modifiers: Vec<Modifier> = dot
        .map(|d| ps.block_modifiers().modifiers_of(d).into_values().collect())
        .unwrap_or_default();
    let style = dot.and_then(|d| ps.node_styles().value_of(d));
    let marker = dot.and_then(|d| ps.node_markers().value_of(d));
    let children = block
        .children()
        .map(|c| capture_child_subtree(ps, &c))
        .collect();
    Subtree {
        node,
        modifiers,
        style,
        marker,
        children,
    }
}

/// Remove the child at full child-slot `index` of `parent` (a block OR a
/// block-level/inline atom leaf). Unlike `Transaction::remove_subtree`, this
/// indexes by the full child list and can remove leaf atoms.
pub(crate) fn remove_child_at(
    tr: &mut Transaction,
    parent: Dot,
    index: usize,
) -> Result<(), CommandError> {
    let subtree = {
        let view = tr.state().view();
        let parent_node = view
            .node(parent)
            .ok_or(CommandError::NodeNotFound(parent))?;
        let child = parent_node
            .child_at(index)
            .ok_or_else(|| CommandError::Corrupted("child index out of range".into()))?;
        capture_child_subtree(&tr.state().projected, &child)
    };
    tr.apply_steps(vec![Step::RemoveSubtree {
        parent,
        index,
        subtree,
    }])?;
    Ok(())
}

/// Remove `block` if it has no real children (the projection may synthesize a
/// Derived placeholder, so emptiness is judged by real children), then cascade
/// to its parent if that becomes empty in turn. A no-op for blocks that still
/// hold real content, are structural, or for which empty is valid.
pub(crate) fn prune_empty_real(tr: &mut Transaction, block: Dot) -> Result<(), CommandError> {
    let mut current = block;
    loop {
        let parent_id = {
            let view = tr.state().view();
            let Some(node) = view.node(current) else {
                break;
            };
            let has_real_child = node
                .children()
                .any(|c| crate::helpers::child_elem_id(&c).as_op_dot().is_some());
            let spec = node.spec();
            let removable = !has_real_child
                && !spec.structural
                && spec.content.min_required() > 0
                && node.parent().is_some();
            if !removable {
                break;
            }
            node.parent().map(|p| p.id())
        };
        let Some(parent_id) = parent_id else {
            break;
        };
        tr.remove_subtree(current)?;
        current = parent_id;
    }
    Ok(())
}

/// Collect the leaf ids of the children in `[offset, offset + len)` of `block`.
fn child_leaf_dots(tr: &Transaction, block: Dot, offset: usize, len: usize) -> Vec<Dot> {
    let view = tr.state().view();
    let Some(node) = view.node(block) else {
        return Vec::new();
    };
    (offset..offset + len)
        .filter_map(|i| match node.child_at(i) {
            Some(ChildView::Leaf(l)) => Some(l.dot()),
            _ => None,
        })
        .collect()
}

/// Reconcile inline (span) formatting on `dots` so the non-style own modifiers
/// match `desired`: add desired spans that are missing or differ, and strip
/// text-applicable spans that are not desired (so e.g. typing with bold unset
/// inside a bold run yields plain text).
fn apply_inline_modifiers(
    tr: &mut Transaction,
    dots: &[Dot],
    desired: &[Modifier],
) -> Result<(), CommandError> {
    let (Some(first), Some(last)) = (dots.first(), dots.last()) else {
        return Ok(());
    };
    let (first, last) = (*first, *last);

    let desired_map: BTreeMap<ModifierType, Modifier> =
        desired.iter().map(|m| (m.as_type(), m.clone())).collect();

    let actual: BTreeMap<ModifierType, Modifier> = {
        let view = tr.state().view();
        match view.leaf(first) {
            Some(l) => l
                .own_modifiers()
                .iter()
                .filter(|(_, o)| !o.from_style)
                .map(|(t, o)| (*t, o.value.clone()))
                .collect(),
            None => BTreeMap::new(),
        }
    };

    for (ty, m) in &desired_map {
        if actual.get(ty) != Some(m) {
            tr.add_span_modifier(first, last, m.clone())?;
        }
    }
    for (ty, m) in &actual {
        if !desired_map.contains_key(ty) && is_text_applicable(*ty) {
            tr.remove_span_modifier(first, last, m.clone())?;
        }
    }
    Ok(())
}

fn apply_node_style(
    tr: &mut Transaction,
    dots: &[Dot],
    style_id: &Option<String>,
) -> Result<(), CommandError> {
    if let Some(style_id) = style_id {
        for dot in dots {
            tr.set_node_style(*dot, Some(style_id.clone()))?;
        }
    }
    Ok(())
}

/// If a collapsed caret sits inside a projection-synthesized scaffold block (one
/// with no authored op — e.g. the mandatory trailing paragraph the Root schema
/// derives after a block-level unit like a horizontal rule), that block has no
/// CRDT identity to parent inserted content to, so an insert against it fails
/// with `OffsetOutOfBounds`. Materialize the synthetic block — and any synthetic
/// ancestors up to the nearest real (or root) container — into real blocks and
/// move the caret into the new real block, so the subsequent insert has a real
/// target. No-op for a real caret block, the root, or a non-collapsed selection.
fn materialize_caret_block(tr: &mut Transaction) -> Result<(), CommandError> {
    let Some(selection) = tr.selection() else {
        return Ok(());
    };
    if selection.anchor != selection.head {
        return Ok(());
    }
    let caret = selection.head.node;
    if caret.as_op_dot().is_some() || caret == Dot::ROOT {
        return Ok(());
    }

    // Walk up to the nearest real (or root) ancestor, recording the synthetic
    // chain (deepest-first) so it can be rebuilt as real nested blocks.
    let (anchor_id, anchor_slot, chain) = {
        let view = tr.state().view();
        let mut node = view.node(caret).ok_or(CommandError::NodeNotFound(caret))?;
        let mut chain = vec![node.node().to_plain()];
        loop {
            let parent = node.parent().ok_or(CommandError::NoParent(node.id()))?;
            let slot = node
                .index()
                .ok_or_else(|| CommandError::orphan_child(node.id(), parent.id()))?;
            if parent.id().as_op_dot().is_some() || parent.id() == Dot::ROOT {
                break (parent.id(), slot, chain);
            }
            chain.push(parent.node().to_plain());
            node = parent;
        }
    };

    let mut subtree: Option<Subtree> = None;
    for node in chain.iter().cloned() {
        let mut st = Subtree::leaf(node);
        if let Some(child) = subtree.take() {
            st = st.with_children(vec![child]);
        }
        subtree = Some(st);
    }
    let subtree = subtree.expect("synthetic chain is non-empty");

    tr.insert_subtree(anchor_id, anchor_slot, subtree)?;

    let new_caret = {
        let view = tr.state().view();
        let mut cur = match view.node(anchor_id).and_then(|p| p.child_at(anchor_slot)) {
            Some(ChildView::Block(b)) => b.id(),
            _ => return Err(CommandError::Corrupted("materialized block missing".into())),
        };
        for _ in 1..chain.len() {
            cur = match view.node(cur).and_then(|n| n.first_child()) {
                Some(ChildView::Block(b)) => b.id(),
                _ => return Err(CommandError::Corrupted("materialized child missing".into())),
            };
        }
        cur
    };

    tr.set_selection(Some(Selection::collapsed(Position {
        node: new_caret,
        offset: 0,
        affinity: Affinity::Downstream,
    })))?;

    Ok(())
}

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

    materialize_caret_block(tr)?;

    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }
    let pos = selection.head;
    let block = pos.node;

    let pending_style_explicit = tr.pending_style().is_some();
    let pending_modifiers = tr.pending_modifiers().clone();

    let (mut effective_mods, pending_style, host_paragraph_id, host_has_marker) = {
        let view = tr.state().view();
        let node = view.node(block).ok_or(CommandError::NodeNotFound(block))?;

        let host_paragraph_id = find_enclosing_paragraph_id(&view, block);
        let host_marker =
            host_paragraph_id.and_then(|id| tr.state().projected.node_markers().value_of(id));
        let host_is_empty = host_paragraph_id
            .and_then(|id| view.node(id))
            .map(|p| {
                !p.children()
                    .any(|c| matches!(c, ChildView::Leaf(l) if l.as_char().is_some()))
            })
            .unwrap_or(false);
        let marker_style = if host_is_empty {
            host_marker.as_ref().and_then(|m| m.style.clone())
        } else {
            None
        };

        let left_style = pos
            .offset
            .checked_sub(1)
            .and_then(|i| match node.child_at(i) {
                Some(ChildView::Leaf(l)) => Some(l.dot()),
                _ => None,
            })
            .and_then(|d| tr.state().projected.node_styles().value_of(d));

        let pending_style: Option<String> = match tr.pending_style() {
            Some(editor_state::PendingStyle::Set { style_id }) => Some(style_id.clone()),
            Some(editor_state::PendingStyle::Unset) => None,
            None => left_style.or(marker_style.clone()),
        };

        let mut effective_mods = resolve_effective_modifiers(&node, pos.offset, &pending_modifiers);
        effective_mods.retain(|m| is_text_applicable(m.as_type()));

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

        (
            effective_mods,
            pending_style,
            host_paragraph_id,
            host_marker.is_some(),
        )
    };
    effective_mods.retain(|m| is_text_applicable(m.as_type()));

    let insert_len = text.char_count();
    tr.insert_text(block, pos.offset, text)?;

    let new_dots = child_leaf_dots(tr, block, pos.offset, insert_len);
    apply_inline_modifiers(tr, &new_dots, &effective_mods)?;
    apply_node_style(tr, &new_dots, &pending_style)?;

    tr.set_selection(Some(Selection::collapsed(Position {
        node: block,
        offset: pos.offset + insert_len,
        affinity: Affinity::Upstream,
    })))?;

    if !tr.pending_modifiers().is_empty() {
        tr.set_pending_modifiers(PendingModifiers::new())?;
    }
    if pending_style_explicit {
        tr.set_pending_style(None)?;
    }
    if let Some(p_id) = host_paragraph_id
        && host_has_marker
    {
        tr.set_marker(p_id, None)?;
    }

    Ok(true)
}

fn insert_atom_at_caret(
    tr: &mut Transaction,
    plain: PlainNode,
    metric_only: bool,
) -> CommandResult {
    materialize_caret_block(tr)?;

    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }
    let pos = selection.head;
    let block = pos.node;
    let pending_modifiers = tr.pending_modifiers().clone();

    let (metric_mods, pending_style, carryable, host_paragraph_id, host_has_marker) = {
        let view = tr.state().view();
        let node = view.node(block).ok_or(CommandError::NodeNotFound(block))?;

        let host_paragraph_id = find_enclosing_paragraph_id(&view, block);
        let host_marker = host_paragraph_id
            .filter(|&id| {
                view.node(id)
                    .map(|p| {
                        !p.children()
                            .any(|c| matches!(c, ChildView::Leaf(l) if l.as_char().is_some()))
                    })
                    .unwrap_or(false)
            })
            .and_then(|id| tr.state().projected.node_markers().value_of(id));
        let marker_style = host_marker.as_ref().and_then(|m| m.style.clone());

        let left_style = pos
            .offset
            .checked_sub(1)
            .and_then(|i| match node.child_at(i) {
                Some(ChildView::Leaf(l)) => Some(l.dot()),
                _ => None,
            })
            .and_then(|d| tr.state().projected.node_styles().value_of(d));

        let pending_style: Option<String> = match tr.pending_style() {
            Some(editor_state::PendingStyle::Set { style_id }) => Some(style_id.clone()),
            Some(editor_state::PendingStyle::Unset) => None,
            None => left_style.or(marker_style.clone()),
        };

        let mut metric_mods = resolve_effective_modifiers(&node, pos.offset, &pending_modifiers);
        if metric_only {
            metric_mods.retain(|m| is_tab_metric_modifier(m.as_type()));
            if let Some(marker) = &host_marker {
                for m in &marker.modifiers {
                    if is_tab_metric_modifier(m.as_type())
                        && !metric_mods.iter().any(|e| e.as_type() == m.as_type())
                    {
                        metric_mods.push(m.clone());
                    }
                }
            }
        } else {
            metric_mods.clear();
        }

        let carryable = carryable_modifiers_at(&view, pos, &pending_modifiers);

        (
            metric_mods,
            pending_style,
            carryable,
            host_paragraph_id,
            host_marker.is_some(),
        )
    };

    tr.insert_subtree(block, pos.offset, Subtree::leaf(plain))?;

    let new_dots = child_leaf_dots(tr, block, pos.offset, 1);
    apply_inline_modifiers(tr, &new_dots, &metric_mods)?;
    apply_node_style(tr, &new_dots, &pending_style)?;

    tr.set_selection(Some(Selection::collapsed(Position {
        node: block,
        offset: pos.offset + 1,
        affinity: Affinity::Downstream,
    })))?;

    if tr.pending_style().is_some() {
        tr.set_pending_style(None)?;
    }

    if let Some(p_id) = host_paragraph_id {
        if host_has_marker {
            tr.set_marker(p_id, None)?;
        } else {
            let marker = editor_model::Marker {
                modifiers: carryable,
                style: None,
            };
            if !marker.is_empty() {
                tr.set_marker(p_id, Some(marker))?;
            }
        }
    }

    Ok(true)
}

pub(crate) fn insert_hard_break_at_caret(tr: &mut Transaction) -> CommandResult {
    insert_atom_at_caret(
        tr,
        PlainNode::HardBreak(PlainHardBreakNode::default()),
        false,
    )
}

pub(crate) fn insert_tab_at_caret(tr: &mut Transaction) -> CommandResult {
    insert_atom_at_caret(tr, PlainNode::Tab(PlainTabNode::default()), true)
}
