use std::collections::BTreeMap;

use editor_common::Tri;
use editor_crdt::Dot;
use editor_model::{
    Alignment, ChildView, DEFAULT_FONT_FAMILY, DEFAULT_FONT_WEIGHT, DocView, Modifier,
    ModifierState, ModifierType, NodeType, NodeView, Schema,
};
use editor_resource::{Resource, find_bold_target, find_unbold_target, match_weight};
use editor_state::{
    PendingModifiers, Position, ProjectedState, ResolvedSelection, Selection, apply_pending,
    continuation_at, leaf_groups_in_range, leaf_spans_in_range, resolve_modifier_state_in_range,
};
use editor_transaction::Transaction;
use strum::IntoEnumIterator;

use crate::helpers::{
    block_accepts_carry_kind, companion_set, companion_unset, end_touched_textblocks,
};
use crate::{CommandError, CommandResult};

pub(crate) fn resolve_effective_modifiers(
    state: &ProjectedState,
    block: Dot,
    offset: usize,
    pending_modifiers: &PendingModifiers,
) -> Vec<Modifier> {
    let mut out = continuation_at(state, block, offset);
    apply_pending(&mut out, pending_modifiers);
    out.into_values().collect()
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
    if let Some(rect) = resolved.as_cell_rect() {
        for cell in rect.cells() {
            let blocks = std::iter::once(cell).chain(cell.descendants().filter_map(|d| match d {
                ChildView::Block(b) => Some(b),
                ChildView::Leaf(_) => None,
            }));
            for node in blocks {
                if !targets.contains(&node.node_type()) {
                    continue;
                }
                let mut path: Vec<NodeType> = node.ancestors().map(|a| a.node_type()).collect();
                path.reverse();
                if target.matches(&path) {
                    out.push(node.id());
                }
            }
        }
        return out;
    }
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

pub(crate) fn is_table_justify(view: &DocView, id: Dot, modifier: &Modifier) -> bool {
    matches!(
        modifier,
        Modifier::Alignment {
            value: Alignment::Justify
        }
    ) && view
        .node(id)
        .is_some_and(|n| n.node_type() == NodeType::Table)
}

pub(crate) fn matches_modifier_context(
    view: &DocView,
    id: Dot,
    modifier_type: ModifierType,
) -> bool {
    let Some(node) = view.node(id) else {
        return true;
    };
    let mut path: Vec<NodeType> = node.ancestors().map(|a| a.node_type()).collect();
    path.reverse();
    Schema::modifier_spec(modifier_type).context.matches(&path)
}

pub(crate) fn apply_modifier_to_node(
    tr: &mut Transaction,
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

pub(crate) fn continuation_paint_at(state: &ProjectedState, pos: Position) -> Vec<Modifier> {
    continuation_at(state, pos.node, pos.offset)
        .into_iter()
        .filter(|(ty, _)| ty.is_carry_kind())
        .map(|(_, m)| m)
        .collect()
}

pub(crate) fn placeholder_modifier(ty: ModifierType) -> Modifier {
    match ty {
        ModifierType::Bold => Modifier::Bold,
        ModifierType::Italic => Modifier::Italic,
        ModifierType::Underline => Modifier::Underline,
        ModifierType::Strikethrough => Modifier::Strikethrough,
        ModifierType::FontSize => Modifier::FontSize { value: 0 },
        ModifierType::FontFamily => Modifier::FontFamily {
            value: String::new(),
        },
        ModifierType::FontWeight => Modifier::FontWeight { value: 0 },
        ModifierType::TextColor => Modifier::TextColor {
            value: String::new(),
        },
        ModifierType::BackgroundColor => Modifier::BackgroundColor {
            value: String::new(),
        },
        ModifierType::LetterSpacing => Modifier::LetterSpacing { value: 0 },
        ModifierType::Link => Modifier::Link {
            href: String::new(),
        },
        ModifierType::Ruby => Modifier::Ruby {
            text: String::new(),
        },
        other => unreachable!("{other:?} is not a text-applicable inline modifier"),
    }
}

pub(crate) fn validate_edit(
    modifier_type: ModifierType,
    modifier: &Option<Modifier>,
) -> Result<bool, CommandError> {
    if let Some(m) = modifier
        && m.as_type() != modifier_type
    {
        return Err(CommandError::InvalidArgument(format!(
            "modifier type mismatch: op type {:?}, modifier {:?}",
            modifier_type,
            m.as_type()
        )));
    }
    if let Some(m) = modifier
        && !m.is_valid()
    {
        return Ok(false);
    }
    if !modifier_type.is_text_applicable() {
        return Err(CommandError::InvalidArgument(format!(
            "edit_modifier is only valid for text-applicable modifiers; got {:?}",
            modifier_type
        )));
    }
    Ok(true)
}

pub(crate) fn edit_modifier_range(
    tr: &mut Transaction,
    selection: Selection,
    modifier_type: ModifierType,
    modifier: Option<Modifier>,
) -> CommandResult {
    let (spans, present, end_touched) = {
        let view = tr.view();
        let rs = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let spans = leaf_spans_in_range(&rs);
        let present = spans.first().and_then(|&(first, _)| {
            view.leaf_state_by_dot_slow(first)
                .and_then(|st| st.eff.get(&modifier_type).cloned())
        });
        let end_touched: Vec<Dot> = end_touched_textblocks(&view, &rs)
            .into_iter()
            .filter(|&b| block_accepts_carry_kind(&view, b, modifier_type))
            .collect();
        (spans, present, end_touched)
    };

    for &(first, last) in &spans {
        match &modifier {
            Some(m) => {
                tr.add_span_modifier(first, last, m.clone())?;
            }
            None => {
                if let Some(present) = present.clone() {
                    tr.remove_span_modifier(first, last, present)?;
                }
            }
        }
    }

    match &modifier {
        Some(m) => companion_set(tr, &end_touched, m)?,
        None => companion_unset(tr, &end_touched, modifier_type)?,
    }

    if spans.is_empty() && end_touched.is_empty() {
        return Ok(false);
    }
    Ok(true)
}

pub(crate) fn font_weight(effective: &BTreeMap<ModifierType, Modifier>) -> u16 {
    match effective.get(&ModifierType::FontWeight) {
        Some(Modifier::FontWeight { value }) => *value,
        _ => DEFAULT_FONT_WEIGHT,
    }
}

fn has_bold(effective: &BTreeMap<ModifierType, Modifier>) -> bool {
    effective.contains_key(&ModifierType::Bold)
}

pub(crate) fn weight_and_bold_after_family_change(
    old_weight: u16,
    old_bold: bool,
    available_weights: &[u16],
) -> (u16, bool) {
    let matched = match_weight(available_weights, old_weight).unwrap_or(old_weight);
    if old_bold {
        return find_bold_target(matched, available_weights)
            .map(|target| (target, false))
            .unwrap_or((matched, true));
    }
    if old_weight >= 700 && matched < 700 {
        return find_bold_target(matched, available_weights)
            .map(|target| (target, false))
            .unwrap_or((matched, true));
    }
    (matched, false)
}

type GroupOp = (Dot, Dot, bool, u16, u16, bool);
type EndTouchedFam = (Dot, bool, BTreeMap<ModifierType, Modifier>, u16);

pub(crate) fn set_font_family_range(
    tr: &mut Transaction,
    selection: Selection,
    family: Modifier,
    available_weights: &[u16],
) -> CommandResult {
    // Weight/bold normalization is uniform within a leaf group (same effective,
    // same host), so both the scan and the emitted ops are per group — a
    // select-all family change costs O(groups), not O(leaves).
    let (groups, end_touched): (Vec<GroupOp>, Vec<EndTouchedFam>) = {
        let view = tr.view();
        let rs = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let family_eq = |block: Dot| {
            view.node(block)
                .and_then(|n| n.effective().get(&ModifierType::FontFamily).cloned())
                .as_ref()
                == Some(&family)
        };
        let groups = leaf_groups_in_range(&rs)
            .into_iter()
            .map(|g| {
                (
                    g.first,
                    g.last,
                    family_eq(g.host),
                    font_weight(g.effective),
                    view.node(g.host)
                        .map(|node| font_weight(node.effective()))
                        .unwrap_or(DEFAULT_FONT_WEIGHT),
                    has_bold(g.effective),
                )
            })
            .collect();
        let end_touched = end_touched_textblocks(&view, &rs)
            .into_iter()
            .filter(|&b| block_accepts_carry_kind(&view, b, ModifierType::FontFamily))
            .map(|b| {
                let block_eff = view
                    .node(b)
                    .map(|n| n.effective().clone())
                    .unwrap_or_default();
                let inherited_weight = font_weight(&block_eff);
                (b, family_eq(b), block_eff, inherited_weight)
            })
            .collect();
        (groups, end_touched)
    };

    for (g_first, g_last, fam_eq, old_weight, inherited_weight, old_bold) in &groups {
        if *fam_eq {
            tr.remove_span_modifier(*g_first, *g_last, family.clone())?;
        } else {
            tr.add_span_modifier(*g_first, *g_last, family.clone())?;
        }

        let (new_weight, new_bold) =
            weight_and_bold_after_family_change(*old_weight, *old_bold, available_weights);

        if *old_bold && !new_bold {
            tr.remove_span_modifier(*g_first, *g_last, Modifier::Bold)?;
        } else if !*old_bold && new_bold {
            tr.add_span_modifier(*g_first, *g_last, Modifier::Bold)?;
        }

        if new_weight != *old_weight {
            tr.remove_span_modifier(
                *g_first,
                *g_last,
                Modifier::FontWeight { value: *old_weight },
            )?;
            if new_weight != *inherited_weight {
                tr.add_span_modifier(
                    *g_first,
                    *g_last,
                    Modifier::FontWeight { value: new_weight },
                )?;
            }
        }
    }

    for (block, fam_eq, block_eff, inherited_weight) in &end_touched {
        let carry = tr.state().projected.carry_modifiers(*block);
        let mut virt = block_eff.clone();
        for (ty, m) in &carry {
            virt.insert(*ty, m.clone());
        }
        let (new_weight, new_bold) = weight_and_bold_after_family_change(
            font_weight(&virt),
            has_bold(&virt),
            available_weights,
        );

        let cur_fam = carry.get(&ModifierType::FontFamily);
        if *fam_eq {
            if cur_fam.is_some() {
                tr.remove_carry_modifier(*block, ModifierType::FontFamily)?;
            }
        } else if cur_fam != Some(&family) {
            tr.set_carry_modifier(*block, family.clone())?;
        }

        let cur_bold = carry.contains_key(&ModifierType::Bold);
        if new_bold && !cur_bold {
            tr.set_carry_modifier(*block, Modifier::Bold)?;
        } else if !new_bold && cur_bold {
            tr.remove_carry_modifier(*block, ModifierType::Bold)?;
        }

        let cur_weight = match carry.get(&ModifierType::FontWeight) {
            Some(Modifier::FontWeight { value }) => Some(*value),
            _ => None,
        };
        let target_weight = (new_weight != *inherited_weight).then_some(new_weight);
        match target_weight {
            Some(w) if cur_weight != Some(w) => {
                tr.set_carry_modifier(*block, Modifier::FontWeight { value: w })?;
            }
            None if cur_weight.is_some() => {
                tr.remove_carry_modifier(*block, ModifierType::FontWeight)?;
            }
            _ => {}
        }
    }

    if groups.is_empty() && end_touched.is_empty() {
        return Ok(false);
    }
    Ok(true)
}

pub(crate) fn set_modifier_range_text(
    tr: &mut Transaction,
    selection: Selection,
    modifier: &Modifier,
) -> CommandResult {
    let modifier_type = modifier.as_type();

    let (groups, end_touched) = {
        let view = tr.view();
        let rs = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let inherited_eq = |block: Dot| {
            view.node(block)
                .and_then(|n| n.effective().get(&modifier_type).cloned())
                .as_ref()
                == Some(modifier)
        };
        let groups: Vec<(Dot, Dot, bool)> = leaf_groups_in_range(&rs)
            .iter()
            .map(|g| (g.first, g.last, inherited_eq(g.host)))
            .collect();
        let end_touched: Vec<(Dot, bool)> = end_touched_textblocks(&view, &rs)
            .into_iter()
            .filter(|&b| block_accepts_carry_kind(&view, b, modifier_type))
            .map(|b| (b, inherited_eq(b)))
            .collect();
        (groups, end_touched)
    };

    for (first, last, inherited_eq) in &groups {
        if *inherited_eq {
            tr.remove_span_modifier(*first, *last, modifier.clone())?;
        } else {
            tr.add_span_modifier(*first, *last, modifier.clone())?;
        }
    }

    let set_blocks: Vec<Dot> = end_touched
        .iter()
        .filter(|(_, eq)| !eq)
        .map(|(b, _)| *b)
        .collect();
    let unset_blocks: Vec<Dot> = end_touched
        .iter()
        .filter(|(_, eq)| *eq)
        .map(|(b, _)| *b)
        .collect();
    companion_set(tr, &set_blocks, modifier)?;
    companion_unset(tr, &unset_blocks, modifier_type)?;

    if groups.is_empty() && end_touched.is_empty() {
        return Ok(false);
    }
    Ok(true)
}

pub(crate) fn block_weight(view: &DocView, elem: Dot) -> Option<u16> {
    match view.node(elem)?.effective().get(&ModifierType::FontWeight) {
        Some(Modifier::FontWeight { value }) => Some(*value),
        _ => None,
    }
}

pub(crate) fn block_family(view: &DocView, elem: Dot) -> Option<String> {
    match view.node(elem)?.effective().get(&ModifierType::FontFamily) {
        Some(Modifier::FontFamily { value }) => Some(value.clone()),
        _ => None,
    }
}

pub(crate) fn range_has_heavy_weight(rs: &ResolvedSelection) -> bool {
    leaf_groups_in_range(rs).iter().any(|g| {
        matches!(
            g.effective.get(&ModifierType::FontWeight),
            Some(Modifier::FontWeight { value }) if *value >= 700
        )
    })
}

fn leaf_uniform_font_weight(rs: &ResolvedSelection) -> Option<u16> {
    let mut it = leaf_groups_in_range(rs)
        .into_iter()
        .map(|g| font_weight(g.effective));
    let first = it.next()?;
    it.all(|w| w == first).then_some(first)
}

fn leaf_uniform_font_family(rs: &ResolvedSelection) -> Option<String> {
    let family = |effective: &BTreeMap<ModifierType, Modifier>| match effective
        .get(&ModifierType::FontFamily)
    {
        Some(Modifier::FontFamily { value }) => value.clone(),
        _ => DEFAULT_FONT_FAMILY.to_string(),
    };
    let mut it = leaf_groups_in_range(rs)
        .into_iter()
        .map(|g| family(g.effective));
    let first = it.next()?;
    it.all(|f| f == first).then_some(first)
}

pub(crate) fn modifier_from_unit_type(
    modifier_type: ModifierType,
) -> Result<Modifier, CommandError> {
    match modifier_type {
        ModifierType::Italic => Ok(Modifier::Italic),
        ModifierType::Underline => Ok(Modifier::Underline),
        ModifierType::Strikethrough => Ok(Modifier::Strikethrough),
        other => Err(CommandError::InvalidArgument(format!(
            "{other:?} is not a unit modifier type"
        ))),
    }
}

pub(crate) fn range_has_modifier(ms: &ModifierState, ty: ModifierType) -> bool {
    matches!(
        match ty {
            ModifierType::Italic => &ms.italic,
            ModifierType::Underline => &ms.underline,
            ModifierType::Strikethrough => &ms.strikethrough,
            _ => return false,
        },
        Tri::Uniform { .. }
    )
}

pub(crate) fn toggle_bold_range(
    tr: &mut Transaction,
    selection: Selection,
    resource: &Resource,
) -> CommandResult {
    let (spans, is_bold, leaf_ctx, end_touched) = {
        let view = tr.view();
        let state = &tr.state().projected;
        let rs = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let spans = leaf_spans_in_range(&rs);
        let end_touched: Vec<Dot> = end_touched_textblocks(&view, &rs)
            .into_iter()
            .filter(|&b| block_accepts_carry_kind(&view, b, ModifierType::Bold))
            .collect();
        let is_bold = matches!(
            resolve_modifier_state_in_range(state, &rs).effective_bold,
            Tri::Uniform { .. }
        );
        let leaf_ctx = (!spans.is_empty()).then(|| {
            let from_block = rs.from().node();
            let inherited_weight = block_weight(&view, from_block).unwrap_or(DEFAULT_FONT_WEIGHT);
            let current_weight = leaf_uniform_font_weight(&rs).unwrap_or(inherited_weight);
            let font_family = leaf_uniform_font_family(&rs).unwrap_or_else(|| {
                block_family(&view, from_block).unwrap_or_else(|| DEFAULT_FONT_FAMILY.to_string())
            });
            let synthetic_bold = leaf_groups_in_range(&rs)
                .iter()
                .any(|g| has_bold(g.effective));
            let weight_bold = range_has_heavy_weight(&rs);
            (
                current_weight,
                font_family,
                inherited_weight,
                synthetic_bold,
                weight_bold,
            )
        });
        (spans, is_bold, leaf_ctx, end_touched)
    };

    if let Some((current_weight, font_family, inherited_weight, synthetic_bold, weight_bold)) =
        leaf_ctx
    {
        let available = resource.font_registry.weights(&font_family).unwrap_or(&[]);

        for &(first, last) in &spans {
            if is_bold {
                if synthetic_bold {
                    tr.remove_span_modifier(first, last, Modifier::Bold)?;
                }
                if weight_bold {
                    let unbold = find_unbold_target(current_weight, available);
                    tr.remove_span_modifier(
                        first,
                        last,
                        Modifier::FontWeight {
                            value: current_weight,
                        },
                    )?;
                    if unbold != inherited_weight {
                        tr.add_span_modifier(first, last, Modifier::FontWeight { value: unbold })?;
                    }
                } else {
                    tr.remove_span_modifier(
                        first,
                        last,
                        Modifier::FontWeight {
                            value: current_weight,
                        },
                    )?;
                }
            } else {
                match find_bold_target(current_weight, available) {
                    Some(target) => {
                        if tr.state().projected.span_of_type_overlaps(
                            first,
                            last,
                            ModifierType::FontWeight,
                        ) {
                            tr.remove_span_modifier(
                                first,
                                last,
                                Modifier::FontWeight {
                                    value: current_weight,
                                },
                            )?;
                        }
                        if target != inherited_weight {
                            tr.add_span_modifier(
                                first,
                                last,
                                Modifier::FontWeight { value: target },
                            )?;
                        }
                    }
                    None => {
                        tr.add_span_modifier(first, last, Modifier::Bold)?;
                    }
                }
            }
        }
    }

    if is_bold {
        for &b in &end_touched {
            let carry = tr.state().projected.carry_modifiers(b);
            if carry.contains_key(&ModifierType::Bold) {
                tr.remove_carry_modifier(b, ModifierType::Bold)?;
            }
            if matches!(
                carry.get(&ModifierType::FontWeight),
                Some(Modifier::FontWeight { value }) if *value >= 700
            ) {
                tr.remove_carry_modifier(b, ModifierType::FontWeight)?;
            }
        }
    } else {
        companion_set(tr, &end_touched, &Modifier::Bold)?;
    }

    if spans.is_empty() && end_touched.is_empty() {
        return Ok(false);
    }
    Ok(true)
}

pub(crate) fn toggle_modifier_range(
    tr: &mut Transaction,
    selection: Selection,
    modifier_type: ModifierType,
) -> CommandResult {
    let modifier = modifier_from_unit_type(modifier_type)?;

    let (spans, all_have, end_touched) = {
        let view = tr.view();
        let state = &tr.state().projected;
        let rs = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let spans = leaf_spans_in_range(&rs);
        let all_have =
            range_has_modifier(&resolve_modifier_state_in_range(state, &rs), modifier_type);
        let end_touched: Vec<_> = end_touched_textblocks(&view, &rs)
            .into_iter()
            .filter(|&b| block_accepts_carry_kind(&view, b, modifier_type))
            .collect();
        (spans, all_have, end_touched)
    };

    for &(first, last) in &spans {
        if all_have {
            tr.remove_span_modifier(first, last, modifier.clone())?;
        } else {
            tr.add_span_modifier(first, last, modifier.clone())?;
        }
    }

    if all_have {
        companion_unset(tr, &end_touched, modifier_type)?;
    } else {
        companion_set(tr, &end_touched, &modifier)?;
    }

    if spans.is_empty() && end_touched.is_empty() {
        return Ok(false);
    }
    Ok(true)
}

type CarryBlocksByKind = Vec<(ModifierType, Vec<Dot>)>;

pub(crate) fn clear_all_modifiers_range(
    tr: &mut Transaction,
    selection: Selection,
) -> CommandResult {
    let (spans, companion_by_kind): (Vec<(Dot, Dot)>, CarryBlocksByKind) = {
        let view = tr.view();
        let rs = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let et = end_touched_textblocks(&view, &rs);
        let companion_by_kind = ModifierType::iter()
            .filter(|t| t.is_carry_kind())
            .map(|ty| {
                let blocks: Vec<Dot> = et
                    .iter()
                    .copied()
                    .filter(|&b| block_accepts_carry_kind(&view, b, ty))
                    .collect();
                (ty, blocks)
            })
            .collect();
        (leaf_spans_in_range(&rs), companion_by_kind)
    };

    for &(first, last) in &spans {
        for ty in ModifierType::iter().filter(|&t| t.is_text_applicable()) {
            tr.remove_span_modifier(first, last, placeholder_modifier(ty))?;
        }
    }

    let mut touched_any_carry = false;
    for (ty, blocks) in &companion_by_kind {
        touched_any_carry |= !blocks.is_empty();
        companion_unset(tr, blocks, *ty)?;
    }

    if spans.is_empty() && !touched_any_carry {
        return Ok(false);
    }
    Ok(true)
}
