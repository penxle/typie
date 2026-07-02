use editor_crdt::Dot;
use editor_model::{
    ChildView, DocView, Expand, LeafView, Modifier, ModifierType, NodeType, NodeView, Schema,
};
use editor_state::{PendingModifier, PendingModifiers, Position, ResolvedSelection};
use strum::IntoEnumIterator;

use crate::CommandError;

fn last_leaf_dot(block: &NodeView) -> Option<Dot> {
    block
        .descendants()
        .filter_map(|c| match c {
            ChildView::Leaf(l) => Some(l.dot()),
            ChildView::Block(_) => None,
        })
        .last()
}

/// The `(first, last)` leaf dots bounding the inline span for a resolved
/// selection. When an endpoint lands on a whole block, the block token precedes
/// its content in the flat sequence, so an `After` anchor on it stops short of
/// the block's text; the `to` side descends to the block's last leaf so the
/// span covers the whole block.
pub(crate) fn span_dots(view: &DocView, rs: &ResolvedSelection) -> Option<(Dot, Dot)> {
    let from = rs.from();
    let to = rs.to();

    let from_child = view.node(from.node())?.child_at(from.offset())?;
    let first = match from_child {
        ChildView::Leaf(l) => l.dot(),
        ChildView::Block(b) => b.dot()?,
    };

    let to_off = to.offset().checked_sub(1)?;
    let to_child = view.node(to.node())?.child_at(to_off)?;
    let last = match to_child {
        ChildView::Leaf(l) => l.dot(),
        ChildView::Block(b) => last_leaf_dot(&b).or_else(|| b.dot())?,
    };

    Some((first, last))
}

pub(crate) fn resolve_effective_modifiers(
    node: &NodeView,
    offset: usize,
    pending_modifiers: &PendingModifiers,
) -> Vec<Modifier> {
    let base_modifiers = resolve_base_modifiers(node, offset);
    apply_pending_delta(base_modifiers, pending_modifiers)
}

fn char_leaf_at<'a>(node: &NodeView<'a>, index: usize) -> Option<LeafView<'a>> {
    match node.child_at(index) {
        Some(ChildView::Leaf(l)) if l.as_char().is_some() => Some(l),
        _ => None,
    }
}

fn own_no_style(leaf: &LeafView) -> Vec<(ModifierType, Modifier)> {
    leaf.own_modifiers()
        .iter()
        .filter(|(_, o)| !o.from_style)
        .map(|(t, o)| (*t, o.value.clone()))
        .collect()
}

/// Modifiers a fresh char inserted at `offset` within `node` should carry.
/// Derived from the inline leaves adjacent to the caret: the left leaf
/// contributes its rightward-expanding modifiers, the right leaf its
/// leftward-expanding ones; strictly inside a uniform run all of the run's
/// modifiers carry. Empty paragraphs fall back to their block modifiers.
pub(crate) fn resolve_base_modifiers(node: &NodeView, offset: usize) -> Vec<Modifier> {
    let left = offset.checked_sub(1).and_then(|i| char_leaf_at(node, i));
    let right = char_leaf_at(node, offset);

    let mid = match (&left, &right) {
        (Some(l), Some(r)) => own_no_style(l) == own_no_style(r),
        _ => false,
    };

    let mut out: Vec<Modifier> = Vec::new();
    let push_unique = |m: Modifier, out: &mut Vec<Modifier>| {
        if !out.iter().any(|e| e.as_type() == m.as_type()) {
            out.push(m);
        }
    };

    if let Some(l) = &left {
        for (ty, value) in own_no_style(l) {
            let keep = if mid {
                true
            } else {
                matches!(
                    Schema::modifier_spec(ty).expand,
                    Expand::After | Expand::Both
                )
            };
            if keep {
                push_unique(value, &mut out);
            }
        }
    }
    if !mid && let Some(r) = &right {
        for (ty, value) in own_no_style(r) {
            if matches!(
                Schema::modifier_spec(ty).expand,
                Expand::Before | Expand::Both
            ) {
                push_unique(value, &mut out);
            }
        }
    }

    out
}

fn apply_pending_delta(mut modifiers: Vec<Modifier>, pending: &PendingModifiers) -> Vec<Modifier> {
    for pm in pending {
        match pm {
            PendingModifier::Set { modifier } => {
                modifiers.retain(|existing| existing.as_type() != modifier.as_type());
                modifiers.push(modifier.clone());
            }
            PendingModifier::Unset { ty } => {
                modifiers.retain(|existing| existing.as_type() != *ty);
            }
        }
    }
    modifiers
}

/// Inheritable modifiers provided by ancestors (self excluded), per type.
pub(crate) fn resolve_inherited_modifiers(node: &NodeView) -> Vec<Modifier> {
    let Some(parent) = node.parent() else {
        return Vec::new();
    };
    let parent_eff = parent.effective();
    ModifierType::iter()
        .filter(|&ty| Schema::modifier_spec(ty).inheritable)
        .filter_map(|ty| parent_eff.get(&ty).cloned())
        .collect()
}

pub(crate) fn is_tab_metric_modifier(modifier_type: ModifierType) -> bool {
    matches!(
        modifier_type,
        ModifierType::FontFamily
            | ModifierType::FontWeight
            | ModifierType::FontSize
            | ModifierType::LetterSpacing
    )
}

pub(crate) fn is_text_applicable(modifier_type: ModifierType) -> bool {
    Schema::modifier_spec(modifier_type)
        .target
        .rightmost_node_types()
        .contains(&NodeType::Text)
}

pub(crate) fn resolve_applicable_target_collapsed(
    view: &DocView,
    cursor_node_id: Dot,
    modifier_type: ModifierType,
) -> Option<Dot> {
    let target = &Schema::modifier_spec(modifier_type).target;
    let targets = target.rightmost_node_types();

    let cursor = view.node(cursor_node_id)?;
    for n in cursor.ancestors() {
        if !targets.contains(&n.node_type()) {
            continue;
        }
        let mut path: Vec<NodeType> = n.ancestors().map(|a| a.node_type()).collect();
        path.reverse();
        if target.matches(&path) {
            return Some(n.id());
        }
    }
    None
}

pub(crate) fn collect_applicable_targets_in_range(
    view: &DocView,
    resolved: &ResolvedSelection,
    modifier_type: ModifierType,
) -> Vec<Dot> {
    let target = &Schema::modifier_spec(modifier_type).target;
    let targets = target.rightmost_node_types();
    let mut out = Vec::new();
    let Some(root) = view.root() else {
        return out;
    };
    let (Some(lo_r), Some(hi_r)) = (
        resolved.from().position().resolve(view),
        resolved.to().position().resolve(view),
    ) else {
        return out;
    };

    let mut blocks = vec![root];
    if let Some(root) = view.root() {
        for d in root.descendants() {
            if let ChildView::Block(b) = d {
                blocks.push(b);
            }
        }
    }

    for node in blocks {
        let id = node.id();
        let count = node.children().count();
        let (Some(start), Some(end)) = (
            Position::new(id, 0).resolve(view),
            Position::new(id, count).resolve(view),
        ) else {
            continue;
        };
        if !(start <= hi_r && lo_r <= end) {
            continue;
        }
        if targets.contains(&node.node_type()) {
            let mut path: Vec<NodeType> = node.ancestors().map(|a| a.node_type()).collect();
            path.reverse();
            if target.matches(&path) {
                out.push(id);
            }
        }
    }
    out
}

pub(crate) fn is_unit_variant(modifier: &Modifier) -> bool {
    matches!(
        modifier,
        Modifier::Bold | Modifier::Italic | Modifier::Underline | Modifier::Strikethrough
    )
}

pub(crate) fn apply_modifier_to_node(
    tr: &mut editor_transaction::Transaction,
    target_id: Dot,
    modifier: &Modifier,
) -> Result<(), CommandError> {
    let modifier_type = modifier.as_type();
    let (existing, inherited_value) = {
        let view = tr.state().view();
        let target = view
            .node(target_id)
            .ok_or(CommandError::NodeNotFound(target_id))?;
        let existing = target.block_modifier(modifier_type).cloned();
        let inherited = resolve_inherited_modifiers(&target);
        let inherited_value = inherited.into_iter().find(|m| m.as_type() == modifier_type);
        (existing, inherited_value)
    };

    if let Some(existing) = existing {
        tr.remove_modifier(target_id, existing)?;
    }

    if inherited_value.as_ref() != Some(modifier) {
        tr.add_modifier(target_id, modifier.clone())?;
    }

    Ok(())
}

pub(crate) fn carryable_modifiers_at(
    view: &DocView,
    pos: Position,
    pending: &PendingModifiers,
) -> Vec<Modifier> {
    let Some(node) = view.node(pos.node) else {
        return vec![];
    };
    let effective = resolve_effective_modifiers(&node, pos.offset, pending);
    effective
        .into_iter()
        .filter(|m| {
            matches!(
                Schema::modifier_spec(m.as_type()).expand,
                Expand::After | Expand::Both
            )
        })
        .collect()
}

pub(crate) fn find_enclosing_paragraph_id(view: &DocView, node: Dot) -> Option<Dot> {
    view.node(node)?
        .ancestors()
        .find(|n| n.node_type() == NodeType::Paragraph)
        .map(|n| n.id())
}
