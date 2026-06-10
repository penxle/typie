use editor_common::Tri;
use editor_model::{
    Expand, Modifier, ModifierState, ModifierType, Node, NodeId, NodeRef, NodeType, Schema,
};
use strum::IntoEnumIterator;

use crate::pending_modifier::{PendingModifier, PendingModifiers};
use crate::position::Position;
use crate::resolved_selection::ResolvedSelection;
use crate::selection::Selection;
use crate::state::State;

/// 커서 위치에서 "지금 글자를 치면 적용될" effective modifier 집합.
/// = text node own modifiers (Expand 룰) ⨁ ancestor inherited ⨁ pending overlay.
pub fn resolve_effective_modifiers_at(state: &State, pos: &Position) -> Vec<Modifier> {
    let doc = &state.doc;
    let Some(node) = doc.node(pos.node_id) else {
        return vec![];
    };

    let base = resolve_base_modifiers(&node, pos.offset);
    let inherited = resolve_inherited_modifiers(&node);
    let merged = merge_with_inherited(base, &inherited);
    apply_pending_delta(merged, &state.pending_modifiers)
}

fn resolve_base_modifiers(node: &NodeRef, offset: usize) -> Vec<Modifier> {
    match node.node() {
        Node::Text(text_node) => {
            let node_len = text_node.text.len();
            let at_start = offset == 0 && node_len > 0;
            let at_end = offset == node_len && node_len > 0;

            if !at_start && !at_end {
                return node.own_modifiers().cloned().collect::<Vec<_>>();
            }

            let mut out: Vec<Modifier> = node
                .own_modifiers()
                .filter(|m| {
                    let expand = &m.spec().expand;
                    match expand {
                        Expand::After => at_end,
                        Expand::Before => at_start,
                        Expand::Both => true,
                        Expand::None => false,
                    }
                })
                .cloned()
                .collect();

            if at_start {
                fold_prev_sibling_carryover_own(node, &mut out);
            }

            out
        }
        Node::Paragraph(_) => {
            let mut seen: Vec<ModifierType> = Vec::new();
            let mut out: Vec<Modifier> = Vec::new();
            for m in node.modifiers() {
                let t = m.as_type();
                if !seen.contains(&t) {
                    seen.push(t);
                    out.push(m.clone());
                }
            }
            if node.first_child().is_none()
                && let Some(style_id) = node.entry().style.get().as_ref()
                && let Some(style) = node.doc().style_entry(style_id)
            {
                for m in style.modifiers.iter() {
                    let t = m.as_type();
                    let is_inline = Schema::modifier_spec(t)
                        .target
                        .rightmost_node_types()
                        .contains(&NodeType::Text);
                    if is_inline && !seen.contains(&t) {
                        seen.push(t);
                        out.push(m.clone());
                    }
                }
            }
            if node.first_child().is_none()
                && let Some(marker) = node.marker()
            {
                for m in marker.modifiers.iter() {
                    let t = m.as_type();
                    let is_inline = Schema::modifier_spec(t)
                        .target
                        .rightmost_node_types()
                        .contains(&NodeType::Text);
                    if is_inline && !seen.contains(&t) {
                        seen.push(t);
                        out.push(m.clone());
                    }
                }
                if let Some(style_id) = marker.style.as_ref()
                    && let Some(style) = node.doc().style_entry(style_id)
                {
                    for m in style.modifiers.iter() {
                        let t = m.as_type();
                        let is_inline = Schema::modifier_spec(t)
                            .target
                            .rightmost_node_types()
                            .contains(&NodeType::Text);
                        if is_inline && !seen.contains(&t) {
                            seen.push(t);
                            out.push(m.clone());
                        }
                    }
                }
            }
            out
        }
        _ => vec![],
    }
}

fn fold_prev_sibling_carryover_own(node: &NodeRef, out: &mut Vec<Modifier>) {
    let Some(prev) = node.prev_sibling() else {
        return;
    };
    if !matches!(prev.node(), Node::Text(_)) {
        return;
    }
    for m in prev.own_modifiers() {
        if !matches!(m.spec().expand, Expand::After | Expand::Both) {
            continue;
        }
        let t = m.as_type();
        if out.iter().any(|e| e.as_type() == t) {
            continue;
        }
        out.push(m.clone());
    }
}

fn resolve_inherited_modifiers(node: &NodeRef) -> Vec<Modifier> {
    ModifierType::iter()
        .filter_map(|ty| node.inherited_modifier(ty).cloned())
        .collect()
}

fn merge_with_inherited(mut base: Vec<Modifier>, inherited: &[Modifier]) -> Vec<Modifier> {
    for m in inherited {
        let t = m.as_type();
        if !base.iter().any(|b| b.as_type() == t) {
            base.push(m.clone());
        }
    }
    base
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

/// 현재 selection 기준 `ModifierState` 계산. Collapsed면 effective 집합을 Uniform으로 lift,
/// range면 per-type aggregator에 위임.
pub fn resolve_modifier_state(state: &State) -> Option<ModifierState> {
    let sel = state.selection.as_ref()?;
    if sel.is_collapsed() {
        let pos = sel.head;
        let modifiers = resolve_effective_modifiers_at(state, &pos);
        let mut out = ModifierState::default();
        for m in &modifiers {
            out.set_uniform(m);
        }
        if is_effective_bold(&modifiers) {
            out.effective_bold = Tri::Uniform { value: () };
        }
        Some(out)
    } else {
        let from = sel.anchor;
        let to = sel.head;
        Some(resolve_modifier_state_in_range(state, &from, &to))
    }
}

/// Per-modifier `Tri` aggregate over the inclusive range `[from, to]`.
///
/// For each `ModifierType`, walks every node in the selection range whose path
/// matches the modifier's selection target, then folds their effective values
/// (own ⨁ inherited) into one of `Absent` / `Uniform` / `Mixed`.
pub fn resolve_modifier_state_in_range(
    state: &State,
    from: &Position,
    to: &Position,
) -> ModifierState {
    let mut out = ModifierState::default();
    let nodes = collect_nodes_in_range(state, from, to);

    for ty in ModifierType::iter() {
        let target = &Schema::modifier_spec(ty).target;
        // `target` is the explicit selection scope (a positive expression).
        // Prefilter by its leaf node types, then run the full `matches` check,
        // so non-target nodes never contribute Absent to the aggregate.
        let targets = target.rightmost_node_types();
        debug_assert!(
            !targets.is_empty(),
            "modifier {ty:?} has no resolvable target types from {target:?}"
        );
        let mut canonical: Option<Modifier> = None;
        let mut absent_seen = false;
        let mut mixed = false;
        // Links treat interleaved plain text as neutral so a single URL repeated
        // across a paragraph stays editable as one. Ruby is per-glyph annotation,
        // so any plain text / differing ruby in the selection makes it Mixed.
        let sparse_absence_is_neutral = matches!(ty, ModifierType::Link);

        for node in &nodes {
            if !targets.contains(&node.as_type()) {
                continue;
            }
            let path = root_to_node_type_path(node);
            if !target.matches(&path) {
                continue;
            }

            let value = effective_modifier_on_node(node, ty);
            match (value, &canonical) {
                (Some(m), Some(c)) if &m == c => {}
                (Some(_), Some(_)) => {
                    mixed = true;
                    break;
                }
                (Some(m), None) => {
                    if absent_seen && !sparse_absence_is_neutral {
                        mixed = true;
                        break;
                    }
                    canonical = Some(m);
                }
                (None, Some(_)) => {
                    if !sparse_absence_is_neutral {
                        mixed = true;
                        break;
                    }
                }
                (None, None) => {
                    absent_seen = true;
                }
            }
        }

        if mixed {
            out.set_mixed(ty);
        } else if let Some(m) = canonical {
            out.set_uniform(&m);
        }
        // else: only None-valued nodes saw the context, or none at all → Absent.
    }

    out.effective_bold = aggregate_effective_bold(&nodes);

    out
}

fn root_to_node_type_path(node: &NodeRef<'_>) -> Vec<NodeType> {
    let mut path: Vec<NodeType> = node.ancestors().map(|n| n.as_type()).collect();
    path.reverse();
    path
}

fn effective_modifier_on_node(node: &NodeRef<'_>, ty: ModifierType) -> Option<Modifier> {
    node.effective_modifier(ty).cloned()
}

pub fn is_node_bold(node: &NodeRef) -> bool {
    if node.modifiers().any(|m| matches!(m, Modifier::Bold)) {
        return true;
    }
    matches!(
        effective_modifier_on_node(node, ModifierType::FontWeight),
        Some(Modifier::FontWeight { value }) if value >= 700
    )
}

pub fn is_effective_bold(effective: &[Modifier]) -> bool {
    effective.iter().any(|m| {
        matches!(m, Modifier::Bold) || matches!(m, Modifier::FontWeight { value } if *value >= 700)
    })
}

fn aggregate_effective_bold(nodes: &[NodeRef<'_>]) -> Tri<()> {
    let target = &Schema::modifier_spec(ModifierType::Bold).target;
    let targets = target.rightmost_node_types();
    let mut any_applicable = false;
    let mut all_bold = true;
    let mut any_bold = false;
    for node in nodes {
        if !targets.contains(&node.as_type()) {
            continue;
        }
        let path = root_to_node_type_path(node);
        if !target.matches(&path) {
            continue;
        }
        any_applicable = true;
        if is_node_bold(node) {
            any_bold = true;
        } else {
            all_bold = false;
        }
    }
    if !any_applicable {
        Tri::Absent
    } else if all_bold {
        Tri::Uniform { value: () }
    } else if any_bold {
        Tri::Mixed
    } else {
        Tri::Absent
    }
}

pub fn resolve_modifier_span_at(
    state: &State,
    pos: &Position,
    modifier_type: ModifierType,
) -> Option<Vec<NodeId>> {
    let doc = &state.doc;
    let node = doc.node(pos.node_id)?;
    if !matches!(node.node(), Node::Text(_)) {
        return None;
    }
    let base = node
        .explicit_modifiers()
        .find(|m| m.as_type() == modifier_type)?
        .clone();

    let mut left_chain: Vec<NodeId> = Vec::new();
    let mut cur = node;
    while let Some(prev) = cur.prev_sibling() {
        if !matches!(prev.node(), Node::Text(_)) {
            break;
        }
        let m = prev
            .explicit_modifiers()
            .find(|m| m.as_type() == modifier_type);
        if m != Some(&base) {
            break;
        }
        left_chain.push(prev.id());
        cur = prev;
    }
    left_chain.reverse();

    let mut right_chain: Vec<NodeId> = Vec::new();
    let mut cur = node;
    while let Some(next) = cur.next_sibling() {
        if !matches!(next.node(), Node::Text(_)) {
            break;
        }
        let m = next
            .explicit_modifiers()
            .find(|m| m.as_type() == modifier_type);
        if m != Some(&base) {
            break;
        }
        right_chain.push(next.id());
        cur = next;
    }

    let mut span = left_chain;
    span.push(node.id());
    span.extend(right_chain);
    Some(span)
}

/// Selection that covers the whole modifier span containing `pos`.
///
/// Resolves the contiguous run of sibling text nodes sharing the same explicit
/// modifier value (via [`resolve_modifier_span_at`]), then returns a selection
/// from the start of the first node to the end of the last. Used when entering
/// link/ruby editing so the visible selection extends over the entire mark,
/// even if the caller only has a collapsed caret inside it.
///
/// Returns `None` when `pos` is not inside such a span.
pub fn resolve_modifier_span_selection(
    state: &State,
    pos: &Position,
    modifier_type: ModifierType,
) -> Option<Selection> {
    let span = resolve_modifier_span_at(state, pos, modifier_type)?;
    let doc = &state.doc;

    let first = *span.first()?;
    let last = *span.last()?;

    let last_len = match doc.node(last)?.node() {
        Node::Text(t) => t.text.len(),
        _ => return None,
    };

    Some(Selection::new(
        Position::new(first, 0),
        Position::new(last, last_len),
    ))
}

fn collect_nodes_in_range<'a>(
    state: &'a State,
    from: &Position,
    to: &Position,
) -> Vec<NodeRef<'a>> {
    let sel = Selection::new(*from, *to);
    let Some(rs) = sel.resolve(&state.doc) else {
        return Vec::new();
    };

    let mut out: Vec<NodeRef<'a>> = Vec::new();
    // Start from root rather than `rs.common_ancestor()` so Root-context modifiers
    // (BlockGap, ParagraphIndent) are visited even when the selection lies entirely
    // inside a paragraph. `intersects_subtree` prunes off-spine siblings so the
    // perf cost is negligible.
    let Some(root) = state.doc.root() else {
        return out;
    };
    walk_subtree_intersecting(&root, &rs, &mut out);
    out
}

fn walk_subtree_intersecting<'a>(
    node: &NodeRef<'a>,
    rs: &ResolvedSelection<'a>,
    out: &mut Vec<NodeRef<'a>>,
) {
    if !rs.intersects_subtree(node) {
        return;
    }
    out.push(*node);
    for child in node.children() {
        walk_subtree_intersecting(&child, rs, out);
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::{
        Alignment, AlignmentValue, BlockGapValue, FontSizeValue, Modifier, ModifierType,
        ParagraphIndentValue,
    };

    use super::*;

    #[test]
    fn inherits_root_font_size_when_text_has_none() {
        let (state, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 2)
        };
        let head = state.selection.as_ref().unwrap().head;
        let result = resolve_effective_modifiers_at(&state, &head);
        assert!(
            result
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { value: 1600 }))
        );
    }

    #[test]
    fn text_own_modifier_wins_over_inherited() {
        let (state, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") [font_size(2400)] }
                }
            }
            selection: (t1, 2)
        };
        let result =
            resolve_effective_modifiers_at(&state, &state.selection.as_ref().unwrap().head);
        assert!(
            result
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { value: 2400 }))
        );
        assert!(
            !result
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { value: 1600 }))
        );
    }

    #[test]
    fn pending_overrides_inherited_and_own() {
        let (state, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") [font_size(2400)] }
                }
            }
            selection: (t1, 2)
            pending_modifiers: [font_size(3200)]
        };
        let result =
            resolve_effective_modifiers_at(&state, &state.selection.as_ref().unwrap().head);
        assert!(
            result
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { value: 3200 }))
        );
    }

    #[test]
    fn pending_unset_clears_inherited() {
        let (state, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 2)
            pending_modifiers: [!font_size]
        };
        let result =
            resolve_effective_modifiers_at(&state, &state.selection.as_ref().unwrap().head);
        assert!(!result.iter().any(|m| m.as_type() == ModifierType::FontSize));
    }

    #[test]
    fn middle_of_bold_text_inherits_bold() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 2)
        };
        let result =
            resolve_effective_modifiers_at(&state, &state.selection.as_ref().unwrap().head);
        assert!(result.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn end_of_bold_text_inherits_bold() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 5)
        };
        let result =
            resolve_effective_modifiers_at(&state, &state.selection.as_ref().unwrap().head);
        assert!(result.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn start_of_bold_text_does_not_inherit() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 0)
        };
        let result =
            resolve_effective_modifiers_at(&state, &state.selection.as_ref().unwrap().head);
        assert!(!result.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn end_of_link_does_not_inherit() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Click") [link(href: "https://example.com".to_string())] } } }
            selection: (t1, 5)
        };
        let result =
            resolve_effective_modifiers_at(&state, &state.selection.as_ref().unwrap().head);
        assert!(!result.iter().any(|m| matches!(m, Modifier::Link { .. })));
    }

    #[test]
    fn middle_of_link_inherits() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Click") [link(href: "https://example.com".to_string())] } } }
            selection: (t1, 2)
        };
        let result =
            resolve_effective_modifiers_at(&state, &state.selection.as_ref().unwrap().head);
        assert!(
            result
                .iter()
                .any(|m| matches!(m, Modifier::Link { href } if href == "https://example.com"))
        );
    }

    #[test]
    fn non_text_node_returns_only_pending() {
        let (state, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
            pending_modifiers: [bold]
        };
        let result =
            resolve_effective_modifiers_at(&state, &state.selection.as_ref().unwrap().head);
        assert!(result.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn collapsed_fold_title_surfaces_implicit_text_style() {
        let (state, ..) = state! {
            doc {
                root [font_size(1600)] {
                    fold {
                        fold_title { t1: text("Title") }
                        fold_content { paragraph { text("Body") } }
                    }
                }
            }
            selection: (t1, 2)
            pending_modifiers: [bold]
        };
        let result =
            resolve_effective_modifiers_at(&state, &state.selection.as_ref().unwrap().head);
        // FoldTitle's implicit text style reaches the cursor so the toolbar
        // shows the real size/weight/color. FoldTitle's own FontSize(1050)
        // wins over the root's inherited 1600.
        assert!(
            result
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 500 }))
        );
        assert!(
            result
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { value: 1050 }))
        );
        assert!(result.iter().any(|m| *m
            == Modifier::TextColor {
                value: "gray".to_string()
            }));
        assert!(result.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn collapsed_fold_title_modifier_state_reflects_implicit_text_style() {
        let (state, ..) = state! {
            doc {
                root [font_size(1600)] {
                    fold {
                        fold_title { t1: text("Title") }
                        fold_content { paragraph { text("Body") } }
                    }
                }
            }
            selection: (t1, 2)
            pending_modifiers: [bold]
        };
        let s = resolve_modifier_state(&state).unwrap();
        assert_ne!(s.bold, editor_common::Tri::Absent);
        assert_eq!(
            s.font_size,
            editor_common::Tri::Uniform {
                value: FontSizeValue { value: 1050 }
            }
        );
    }

    #[test]
    fn empty_text_node_inherits_all() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("") [bold] } } }
            selection: (t1, 0)
        };
        let result =
            resolve_effective_modifiers_at(&state, &state.selection.as_ref().unwrap().head);
        assert!(result.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn inherited_weight_from_parent_overrides_root() {
        let (state, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph [font_weight(700)] {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };
        let result =
            resolve_effective_modifiers_at(&state, &state.selection.as_ref().unwrap().head);
        assert!(
            result
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 700 }))
        );
        assert!(
            !result
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 400 }))
        );
    }

    #[test]
    fn range_all_bold_yields_uniform() {
        let (state, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello") [bold]
                t2: text("World") [bold]
            } } }
            selection: (t1, 0) -> (t2, 5)
        };
        let from = state.selection.as_ref().unwrap().anchor;
        let to = state.selection.as_ref().unwrap().head;
        let s = resolve_modifier_state_in_range(&state, &from, &to);
        assert_eq!(s.bold, editor_common::Tri::Uniform { value: () });
    }

    #[test]
    fn range_partial_bold_yields_mixed() {
        let (state, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello") [bold]
                t2: text("World")
            } } }
            selection: (t1, 0) -> (t2, 5)
        };
        let sel = state.selection.as_ref().unwrap();
        let s = resolve_modifier_state_in_range(&state, &sel.anchor, &sel.head);
        assert_eq!(s.bold, editor_common::Tri::Mixed);
    }

    #[test]
    fn range_no_bold_yields_absent() {
        let (state, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello")
                t2: text("World")
            } } }
            selection: (t1, 0) -> (t2, 5)
        };
        let sel = state.selection.as_ref().unwrap();
        let s = resolve_modifier_state_in_range(&state, &sel.anchor, &sel.head);
        assert_eq!(s.bold, editor_common::Tri::Absent);
    }

    #[test]
    fn range_font_size_uniform_with_inherited_root() {
        let (state, ..) = state! {
            doc { root [font_size(1600)] { paragraph {
                t1: text("Hello")
                t2: text("World")
            } } }
            selection: (t1, 0) -> (t2, 5)
        };
        let sel = state.selection.as_ref().unwrap();
        let s = resolve_modifier_state_in_range(&state, &sel.anchor, &sel.head);
        assert_eq!(
            s.font_size,
            editor_common::Tri::Uniform {
                value: FontSizeValue { value: 1600 }
            }
        );
    }

    #[test]
    fn range_alignment_uniform_across_multiple_paragraphs() {
        let (state, ..) = state! {
            doc { root { blockquote {
                paragraph [alignment(Alignment::Right)] { t1: text("A") }
                paragraph [alignment(Alignment::Right)] { t2: text("B") }
            } } }
            selection: (t1, 0) -> (t2, 1)
        };
        let sel = state.selection.as_ref().unwrap();
        let s = resolve_modifier_state_in_range(&state, &sel.anchor, &sel.head);
        assert_eq!(
            s.alignment,
            editor_common::Tri::Uniform {
                value: AlignmentValue {
                    value: Alignment::Right
                }
            }
        );
    }

    #[test]
    fn range_alignment_mixed_for_paragraph_left_and_image_right() {
        let (state, ..) = state! {
            doc { root {
                paragraph [alignment(Alignment::Left)] { t1: text("A") }
                image [alignment(Alignment::Right)]
                paragraph { t2: text("B") }
            } }
            selection: (t1, 0) -> (t2, 1)
        };
        let sel = state.selection.as_ref().unwrap();
        let s = resolve_modifier_state_in_range(&state, &sel.anchor, &sel.head);
        assert_eq!(s.alignment, editor_common::Tri::Mixed);
    }

    #[test]
    fn facade_collapsed_lifts_to_uniform() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hi") [bold] } } }
            selection: (t1, 1)
        };
        let s = resolve_modifier_state(&state).unwrap();
        assert_eq!(s.bold, editor_common::Tri::Uniform { value: () });
    }

    #[test]
    fn facade_collapsed_absent_for_unset() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hi") } } }
            selection: (t1, 1)
        };
        let s = resolve_modifier_state(&state).unwrap();
        assert_eq!(s.bold, editor_common::Tri::Absent);
    }

    #[test]
    fn range_block_gap_uniform_from_root() {
        let (state, ..) = state! {
            doc { root [block_gap(150)] { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let sel = state.selection.as_ref().unwrap();
        let s = resolve_modifier_state_in_range(&state, &sel.anchor, &sel.head);
        assert_eq!(
            s.block_gap,
            editor_common::Tri::Uniform {
                value: BlockGapValue { value: 150 }
            }
        );
    }

    #[test]
    fn range_paragraph_indent_uniform_from_root() {
        let (state, ..) = state! {
            doc { root [paragraph_indent(200)] { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let sel = state.selection.as_ref().unwrap();
        let s = resolve_modifier_state_in_range(&state, &sel.anchor, &sel.head);
        assert_eq!(
            s.paragraph_indent,
            editor_common::Tri::Uniform {
                value: ParagraphIndentValue { value: 200 }
            }
        );
    }

    #[test]
    fn range_block_gap_absent_when_root_unset() {
        // Empty `[]` bypasses `default_modifiers()` so root has no BlockGap.
        let (state, ..) = state! {
            doc { root [] { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let sel = state.selection.as_ref().unwrap();
        let s = resolve_modifier_state_in_range(&state, &sel.anchor, &sel.head);
        assert_eq!(s.block_gap, editor_common::Tri::Absent);
    }

    #[test]
    fn facade_range_uses_aggregator() {
        let (state, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello") [bold]
                t2: text("World")
            } } }
            selection: (t1, 0) -> (t2, 5)
        };
        let s = resolve_modifier_state(&state).unwrap();
        assert_eq!(s.bold, editor_common::Tri::Mixed);
    }

    #[test]
    fn resolve_span_at_caret_inside_uniform_link_returns_node() {
        let (state, t1, ..) = state! {
            doc { root { paragraph { t1: text("Click") [link(href: "https://a.com".to_string())] } } }
            selection: (t1, 2)
        };
        let span = resolve_modifier_span_at(
            &state,
            &state.selection.as_ref().unwrap().head,
            ModifierType::Link,
        );
        assert_eq!(span, Some(vec![t1]));
    }

    #[test]
    fn resolve_span_extends_across_adjacent_same_href() {
        let (state, t1, t2, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello") [link(href: "https://a.com".to_string())]
                t2: text("World") [link(href: "https://a.com".to_string())]
            } } }
            selection: (t1, 2)
        };
        let span = resolve_modifier_span_at(
            &state,
            &state.selection.as_ref().unwrap().head,
            ModifierType::Link,
        );
        assert_eq!(span, Some(vec![t1, t2]));
    }

    #[test]
    fn resolve_span_stops_at_different_href() {
        let (state, _t1, t2, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello") [link(href: "https://a.com".to_string())]
                t2: text("World") [link(href: "https://b.com".to_string())]
            } } }
            selection: (t2, 2)
        };
        let span = resolve_modifier_span_at(
            &state,
            &state.selection.as_ref().unwrap().head,
            ModifierType::Link,
        );
        assert_eq!(span, Some(vec![t2]));
    }

    #[test]
    fn span_selection_covers_whole_link_from_collapsed_caret() {
        let (state, t1, t2, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello") [link(href: "https://a.com".to_string())]
                t2: text("World") [link(href: "https://a.com".to_string())]
            } } }
            selection: (t1, 2)
        };
        let sel = resolve_modifier_span_selection(
            &state,
            &state.selection.as_ref().unwrap().head,
            ModifierType::Link,
        );
        assert_eq!(
            sel,
            Some(Selection::new(Position::new(t1, 0), Position::new(t2, 5)))
        );
    }

    #[test]
    fn span_selection_is_none_outside_link() {
        let (state, _t1, ..) = state! {
            doc { root { paragraph { t1: text("plain") } } }
            selection: (t1, 2)
        };
        let sel = resolve_modifier_span_selection(
            &state,
            &state.selection.as_ref().unwrap().head,
            ModifierType::Link,
        );
        assert_eq!(sel, None);
    }

    #[test]
    fn resolve_span_stops_at_non_modifier_text() {
        let (state, _t0, t1, _t2, ..) = state! {
            doc { root { paragraph {
                t0: text("pre")
                t1: text("link") [link(href: "https://a.com".to_string())]
                t2: text("post")
            } } }
            selection: (t1, 2)
        };
        let span = resolve_modifier_span_at(
            &state,
            &state.selection.as_ref().unwrap().head,
            ModifierType::Link,
        );
        assert_eq!(span, Some(vec![t1]));
    }

    #[test]
    fn resolve_span_returns_none_outside_any_link() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("plain") } } }
            selection: (t1, 2)
        };
        let span = resolve_modifier_span_at(
            &state,
            &state.selection.as_ref().unwrap().head,
            ModifierType::Link,
        );
        assert_eq!(span, None);
    }

    #[test]
    fn resolve_span_does_not_cross_paragraph_boundary() {
        let (state, t1, ..) = state! {
            doc { root {
                paragraph { t1: text("a") [link(href: "https://a.com".to_string())] }
                paragraph { t2: text("b") [link(href: "https://a.com".to_string())] }
            } }
            selection: (t1, 0)
        };
        let span = resolve_modifier_span_at(
            &state,
            &state.selection.as_ref().unwrap().head,
            ModifierType::Link,
        );
        assert_eq!(span, Some(vec![t1]));
    }

    #[test]
    fn range_link_uniform_ignores_plain_text_inside_selection() {
        let (state, _p1, ..) = state! {
            doc { root { p1: paragraph {
                t0: text("pre")
                t1: text("Hello") [link(href: "https://a.com".to_string())]
                t2: text("mid")
                t3: text("World") [link(href: "https://a.com".to_string())]
                t4: text("post")
            } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let s = resolve_modifier_state(&state).unwrap();
        assert_eq!(
            s.link,
            editor_common::Tri::Uniform {
                value: editor_model::LinkValue {
                    href: "https://a.com".to_string(),
                },
            }
        );
    }

    #[test]
    fn range_link_mixed_when_selection_contains_different_hrefs_among_plain_text() {
        let (state, _p1, ..) = state! {
            doc { root { p1: paragraph {
                t0: text("pre")
                t1: text("Hello") [link(href: "https://a.com".to_string())]
                t2: text("mid")
                t3: text("World") [link(href: "https://b.com".to_string())]
                t4: text("post")
            } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let s = resolve_modifier_state(&state).unwrap();
        assert_eq!(s.link, editor_common::Tri::Mixed);
    }

    #[test]
    fn range_ruby_uniform_when_selection_is_a_single_ruby_run() {
        let (state, _p1, ..) = state! {
            doc { root { p1: paragraph {
                t1: text("Hello") [ruby(text: "헬로".to_string())]
                t2: text("World") [ruby(text: "헬로".to_string())]
            } } }
            selection: (p1, 0) -> (p1, 2)
        };
        let s = resolve_modifier_state(&state).unwrap();
        assert_eq!(
            s.ruby,
            editor_common::Tri::Uniform {
                value: editor_model::RubyValue {
                    text: "헬로".to_string(),
                },
            }
        );
    }

    #[test]
    fn range_ruby_mixed_when_plain_text_is_in_selection() {
        // Unlike links, ruby does not treat interleaved plain text as neutral:
        // mixing ruby with plain text is Mixed, which disables the ruby button.
        let (state, _p1, ..) = state! {
            doc { root { p1: paragraph {
                t0: text("pre")
                t1: text("Hello") [ruby(text: "헬로".to_string())]
                t2: text("post")
            } } }
            selection: (p1, 0) -> (p1, 3)
        };
        let s = resolve_modifier_state(&state).unwrap();
        assert_eq!(s.ruby, editor_common::Tri::Mixed);
    }

    #[test]
    fn range_ruby_mixed_when_selection_contains_different_text() {
        let (state, _p1, ..) = state! {
            doc { root { p1: paragraph {
                t1: text("Hello") [ruby(text: "헬로".to_string())]
                t2: text("World") [ruby(text: "월드".to_string())]
            } } }
            selection: (p1, 0) -> (p1, 2)
        };
        let s = resolve_modifier_state(&state).unwrap();
        assert_eq!(s.ruby, editor_common::Tri::Mixed);
    }

    #[test]
    fn paragraph_marker_is_effective_at_paragraph_cursor() {
        let (state, ..) = state! {
            doc { root { p1: paragraph [bold] {} } }
            selection: (p1, 0)
        };
        let head = state.selection.as_ref().unwrap().head;
        let result = resolve_effective_modifiers_at(&state, &head);
        assert!(result.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn paragraph_marker_inherits_into_child_text_at_offset_zero() {
        let (state, ..) = state! {
            doc { root { p1: paragraph [bold] { t1: text("llo") } } }
            selection: (t1, 0)
        };
        let head = state.selection.as_ref().unwrap().head;
        let result = resolve_effective_modifiers_at(&state, &head);
        assert!(result.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn paragraph_marker_does_not_double_count_with_text_modifier_in_middle() {
        let (state, ..) = state! {
            doc { root { p1: paragraph [bold] { t1: text("Hi") [bold] } } }
            selection: (t1, 1)
        };
        let head = state.selection.as_ref().unwrap().head;
        let result = resolve_effective_modifiers_at(&state, &head);
        let bolds = result
            .iter()
            .filter(|m| matches!(m, Modifier::Bold))
            .count();
        assert_eq!(bolds, 1);
    }

    #[test]
    fn effective_bold_uniform_for_heavy_weight_without_mark() {
        let (state, ..) = state! {
            doc { root [font_weight(400)] { paragraph {
                t1: text("Hello") [font_weight(700)]
            } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let sel = state.selection.as_ref().unwrap();
        let s = resolve_modifier_state_in_range(&state, &sel.anchor, &sel.head);
        assert_eq!(s.bold, editor_common::Tri::Absent);
        assert_eq!(s.effective_bold, editor_common::Tri::Uniform { value: () });
    }

    #[test]
    fn effective_bold_uniform_for_mixed_mark_and_weight() {
        let (state, ..) = state! {
            doc { root [font_weight(400)] { paragraph {
                t1: text("Hello") [bold, font_weight(400)]
                t2: text("World") [font_weight(800)]
            } } }
            selection: (t1, 0) -> (t2, 5)
        };
        let sel = state.selection.as_ref().unwrap();
        let s = resolve_modifier_state_in_range(&state, &sel.anchor, &sel.head);
        assert_eq!(s.bold, editor_common::Tri::Mixed);
        assert_eq!(s.effective_bold, editor_common::Tri::Uniform { value: () });
    }

    #[test]
    fn effective_bold_mixed_when_partial() {
        let (state, ..) = state! {
            doc { root [font_weight(400)] { paragraph {
                t1: text("Hello") [font_weight(700)]
                t2: text("World") [font_weight(400)]
            } } }
            selection: (t1, 0) -> (t2, 5)
        };
        let sel = state.selection.as_ref().unwrap();
        let s = resolve_modifier_state_in_range(&state, &sel.anchor, &sel.head);
        assert_eq!(s.effective_bold, editor_common::Tri::Mixed);
    }

    #[test]
    fn effective_bold_absent_when_no_bold() {
        let (state, ..) = state! {
            doc { root [font_weight(400)] { paragraph {
                t1: text("Hello") [font_weight(400)]
            } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let sel = state.selection.as_ref().unwrap();
        let s = resolve_modifier_state_in_range(&state, &sel.anchor, &sel.head);
        assert_eq!(s.effective_bold, editor_common::Tri::Absent);
    }

    #[test]
    fn effective_bold_collapsed_heavy_weight_uniform() {
        let (state, ..) = state! {
            doc { root [font_weight(400)] { paragraph {
                t1: text("Hello") [font_weight(700)]
            } } }
            selection: (t1, 3)
        };
        let s = resolve_modifier_state(&state).unwrap();
        assert_eq!(s.bold, editor_common::Tri::Absent);
        assert_eq!(s.effective_bold, editor_common::Tri::Uniform { value: () });
    }

    #[test]
    fn effective_bold_collapsed_light_weight_absent() {
        let (state, ..) = state! {
            doc { root [font_weight(400)] { paragraph {
                t1: text("Hello") [font_weight(400)]
            } } }
            selection: (t1, 3)
        };
        let s = resolve_modifier_state(&state).unwrap();
        assert_eq!(s.effective_bold, editor_common::Tri::Absent);
    }

    #[test]
    fn collapsed_effective_includes_run_style_inline_modifier() {
        use editor_model::PlainStyleEntry;

        let (initial, t1, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };

        let mut plain = initial.doc.to_plain();
        plain.styles.insert(
            "s1".into(),
            PlainStyleEntry {
                name: "s".into(),
                modifiers: std::iter::once(Modifier::Bold).collect(),
            },
        );
        plain.nodes.entry(t1).and_modify(|e| {
            e.style = Some("s1".into());
        });

        let (doc, graph) = editor_model::Doc::from_plain(plain);
        let next = crate::state::State::new(doc, graph, initial.selection);

        let pos = next.selection.as_ref().unwrap().head;
        let eff = resolve_effective_modifiers_at(&next, &pos);
        assert!(
            eff.contains(&Modifier::Bold),
            "run style's inline modifier must be effective at caret"
        );
    }

    #[test]
    fn inherits_font_size_from_root_base_style() {
        use editor_model::Modifier;
        let (state, _p, _t1) = state! {
            doc {
                styles { base: "기본" [font_size(1600)] }
                root @base [] { p: paragraph { t1: text("Hello") } }
            }
            selection: (t1, 2)
        };
        let pos = state.selection.as_ref().unwrap().head;
        let eff = resolve_effective_modifiers_at(&state, &pos);
        assert!(
            eff.iter()
                .any(|m| matches!(m, Modifier::FontSize { value: 1600 })),
            "text must inherit FontSize from the root's base style"
        );
    }

    #[test]
    fn caret_inherits_base_font_via_resolver() {
        let (state, _p, _t1) = state! {
            doc {
                styles { base: "기본" [font_size(1600)] }
                root @base [] { p: paragraph { t1: text("Hello") } }
            }
            selection: (t1, 2)
        };
        let pos = state.selection.as_ref().unwrap().head;
        assert!(
            resolve_effective_modifiers_at(&state, &pos)
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { value: 1600 }))
        );
    }

    #[test]
    fn empty_paragraph_marker_surfaces_at_caret() {
        let (state, ..) = state! {
            doc { root { p1: paragraph marker([bold]) {} } }
            selection: (p1, 0)
        };
        let head = state.selection.as_ref().unwrap().head;
        let result = resolve_effective_modifiers_at(&state, &head);
        assert!(
            result.iter().any(|m| matches!(m, Modifier::Bold)),
            "empty-paragraph marker's inline modifier must surface at caret"
        );
    }

    #[test]
    fn empty_paragraph_marker_style_surfaces_inline_modifiers() {
        use editor_model::PlainStyleEntry;

        let (initial, p1, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };

        let mut plain = initial.doc.to_plain();
        plain.styles.insert(
            "s1".into(),
            PlainStyleEntry {
                name: "s1".into(),
                modifiers: [Modifier::Bold, Modifier::LineHeight { value: 200 }]
                    .into_iter()
                    .collect(),
            },
        );
        plain.nodes.entry(p1).and_modify(|e| {
            e.style = Some("s1".into());
        });

        let (doc, graph) = editor_model::Doc::from_plain(plain);
        let next = crate::state::State::new(doc, graph, initial.selection);

        let pos = next.selection.as_ref().unwrap().head;
        let eff = resolve_effective_modifiers_at(&next, &pos);
        assert!(
            eff.contains(&Modifier::Bold),
            "marker style inline modifier must surface at empty-paragraph caret"
        );
        assert!(
            !eff.iter()
                .any(|m| matches!(m, Modifier::LineHeight { value: 200 })),
            "block modifier from style must not surface"
        );
    }

    #[test]
    fn boundary_at_start_of_next_text_inherits_preceding_font_size() {
        let (state, ..) = state! {
            doc { root { paragraph {
                t1: text("hello") [font_size(3000)]
                t2: text("world") [font_size(1200)]
            } } }
            selection: (t2, 0)
        };
        let head = state.selection.as_ref().unwrap().head;
        let result = resolve_effective_modifiers_at(&state, &head);
        assert!(
            result
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { value: 3000 })),
            "preceding sibling's font_size must carry to caret at start of next text"
        );
        assert!(
            !result
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { value: 1200 })),
            "current node's Expand::After modifier must not appear at offset 0"
        );
    }

    #[test]
    fn boundary_at_start_does_not_double_count_existing_modifier_type() {
        let (state, ..) = state! {
            doc { root { paragraph {
                t1: text("hello") [bold]
                t2: text("world") [bold]
            } } }
            selection: (t2, 0)
        };
        let head = state.selection.as_ref().unwrap().head;
        let result = resolve_effective_modifiers_at(&state, &head);
        let bolds = result
            .iter()
            .filter(|m| matches!(m, Modifier::Bold))
            .count();
        assert_eq!(bolds, 1, "prev sibling carryover must not duplicate types");
    }

    #[test]
    fn boundary_at_start_excludes_link_from_preceding_sibling() {
        let (state, ..) = state! {
            doc { root { paragraph {
                t1: text("Click") [link(href: "https://example.com".to_string())]
                t2: text("World")
            } } }
            selection: (t2, 0)
        };
        let head = state.selection.as_ref().unwrap().head;
        let result = resolve_effective_modifiers_at(&state, &head);
        assert!(
            !result.iter().any(|m| matches!(m, Modifier::Link { .. })),
            "Expand::None modifiers must not bleed across sibling boundary"
        );
    }

    #[test]
    fn boundary_skips_fold_when_prev_sibling_is_not_text() {
        let (state, ..) = state! {
            doc { root { paragraph {
                hard_break {}
                t2: text("world") [font_size(3000)]
            } } }
            selection: (t2, 0)
        };
        let head = state.selection.as_ref().unwrap().head;
        let result = resolve_effective_modifiers_at(&state, &head);
        assert!(
            !result
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { value: 3000 })),
            "non-text prev sibling must not trigger carryover"
        );
    }
}
