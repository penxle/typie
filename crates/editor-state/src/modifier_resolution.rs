use editor_common::StrExt;
use editor_model::{
    Expand, Modifier, ModifierState, ModifierType, Node, NodeRef, NodeType, Schema,
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
    let candidate_paths = modifier_candidate_paths_at(&node);
    apply_pending_delta(merged, &state.pending_modifiers)
        .into_iter()
        .filter(|m| modifier_allowed_on_any_path(m.as_type(), &candidate_paths))
        .collect()
}

fn resolve_base_modifiers(node: &NodeRef, offset: usize) -> Vec<Modifier> {
    let Node::Text(text_node) = node.node() else {
        return vec![];
    };

    let node_len = text_node.text.char_count();
    let at_start = offset == 0 && node_len > 0;
    let at_end = offset == node_len && node_len > 0;

    if !at_start && !at_end {
        return node.modifiers().to_vec();
    }

    node.modifiers()
        .iter()
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
        .collect()
}

fn resolve_inherited_modifiers(node: &NodeRef) -> Vec<Modifier> {
    let mut found = Vec::new();
    for ancestor in node.ancestors().skip(1) {
        for modifier in ancestor.modifiers() {
            let t = modifier.as_type();
            if !found.iter().any(|m: &Modifier| m.as_type() == t) {
                found.push(modifier.clone());
            }
        }
    }
    found
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
pub fn resolve_modifier_state(state: &State) -> ModifierState {
    if state.selection.is_collapsed() {
        let pos = state.selection.head;
        let modifiers = resolve_effective_modifiers_at(state, &pos);
        let mut out = ModifierState::default();
        for m in &modifiers {
            out.set_uniform(m);
        }
        out
    } else {
        let from = state.selection.anchor;
        let to = state.selection.head;
        resolve_modifier_state_in_range(state, &from, &to)
    }
}

/// Per-modifier `Tri` aggregate over the inclusive range `[from, to]`.
///
/// For each `ModifierType`, walks every node in the selection range whose path
/// matches the modifier's schema context, then folds their effective values
/// (own ⨁ inherited) into one of `Absent` / `Uniform` / `Mixed`.
pub fn resolve_modifier_state_in_range(
    state: &State,
    from: &Position,
    to: &Position,
) -> ModifierState {
    let mut out = ModifierState::default();
    let nodes = collect_nodes_in_range(state, from, to);

    for ty in ModifierType::iter() {
        let context = &Schema::modifier_spec(ty).context;
        // Two-step filter: type prefilter via `ContextExpr::rightmost_node_types` + `ContextExpr::matches`.
        // The prefilter is necessary because `Not(...)` contexts (e.g. Bold's `!FoldTitle > Text`)
        // are permissive — `context.matches` succeeds for any path that doesn't fit the negated
        // pattern, including ancestor paths. Restricting evaluation to the modifier's actual
        // target types prevents absent values from non-targets corrupting the aggregate.
        let targets = context.rightmost_node_types();
        debug_assert!(
            !targets.is_empty(),
            "modifier {ty:?} has no resolvable target types from {context:?}"
        );
        let mut canonical: Option<Modifier> = None;
        let mut absent_seen = false;
        let mut mixed = false;

        for node in &nodes {
            if !targets.contains(&node.as_type()) {
                continue;
            }
            let path = root_to_node_type_path(node);
            if !context.matches(&path) {
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
                    if absent_seen {
                        mixed = true;
                        break;
                    }
                    canonical = Some(m);
                }
                (None, Some(_)) => {
                    mixed = true;
                    break;
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

    out
}

fn root_to_node_type_path(node: &NodeRef<'_>) -> Vec<NodeType> {
    let mut path: Vec<NodeType> = node.ancestors().map(|n| n.as_type()).collect();
    path.reverse();
    path
}

fn modifier_candidate_paths_at(node: &NodeRef<'_>) -> Vec<Vec<NodeType>> {
    let mut paths = Vec::new();
    let mut current = Some(*node);

    while let Some(n) = current {
        paths.push(root_to_node_type_path(&n));
        current = n.parent();
    }

    if !node.spec().inline && node.spec().content.matches(NodeType::Text) {
        let mut text_path = root_to_node_type_path(node);
        text_path.push(NodeType::Text);
        paths.push(text_path);
    }

    paths
}

fn modifier_allowed_on_any_path(ty: ModifierType, paths: &[Vec<NodeType>]) -> bool {
    let context = &Schema::modifier_spec(ty).context;
    let targets = context.rightmost_node_types();
    paths
        .iter()
        .filter(|path| {
            targets.is_empty()
                || path
                    .last()
                    .is_some_and(|node_type| targets.contains(node_type))
        })
        .any(|path| context.matches(path))
}

fn effective_modifier_on_node(node: &NodeRef<'_>, ty: ModifierType) -> Option<Modifier> {
    if let Some(m) = node.modifiers().iter().find(|m| m.as_type() == ty) {
        return Some(m.clone());
    }
    if !Schema::modifier_spec(ty).inheritable {
        return None;
    }
    for ancestor in node.ancestors().skip(1) {
        if let Some(m) = ancestor.modifiers().iter().find(|m| m.as_type() == ty) {
            return Some(m.clone());
        }
    }
    None
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
    walk_subtree_intersecting(&state.doc.root(), &rs, &mut out);
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
        let head = state.selection.head;
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
        let result = resolve_effective_modifiers_at(&state, &state.selection.head);
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
        let result = resolve_effective_modifiers_at(&state, &state.selection.head);
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
        let result = resolve_effective_modifiers_at(&state, &state.selection.head);
        assert!(!result.iter().any(|m| m.as_type() == ModifierType::FontSize));
    }

    #[test]
    fn middle_of_bold_text_inherits_bold() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 2)
        };
        let result = resolve_effective_modifiers_at(&state, &state.selection.head);
        assert!(result.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn end_of_bold_text_inherits_bold() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 5)
        };
        let result = resolve_effective_modifiers_at(&state, &state.selection.head);
        assert!(result.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn start_of_bold_text_does_not_inherit() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 0)
        };
        let result = resolve_effective_modifiers_at(&state, &state.selection.head);
        assert!(!result.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn end_of_link_does_not_inherit() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Click") [link(href: "https://example.com".to_string())] } } }
            selection: (t1, 5)
        };
        let result = resolve_effective_modifiers_at(&state, &state.selection.head);
        assert!(!result.iter().any(|m| matches!(m, Modifier::Link { .. })));
    }

    #[test]
    fn middle_of_link_inherits() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Click") [link(href: "https://example.com".to_string())] } } }
            selection: (t1, 2)
        };
        let result = resolve_effective_modifiers_at(&state, &state.selection.head);
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
        let result = resolve_effective_modifiers_at(&state, &state.selection.head);
        assert!(result.iter().any(|m| matches!(m, Modifier::Bold)));
    }

    #[test]
    fn collapsed_fold_title_filters_text_modifiers_by_context() {
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
        let result = resolve_effective_modifiers_at(&state, &state.selection.head);
        assert!(
            !result
                .iter()
                .any(|m| matches!(m, Modifier::FontSize { .. } | Modifier::Bold))
        );
    }

    #[test]
    fn collapsed_fold_title_modifier_state_has_no_inline_text_modifiers() {
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
        let s = resolve_modifier_state(&state);
        assert_eq!(s.bold, editor_common::Tri::Absent);
        assert_eq!(s.font_size, editor_common::Tri::Absent);
    }

    #[test]
    fn empty_text_node_inherits_all() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("") [bold] } } }
            selection: (t1, 0)
        };
        let result = resolve_effective_modifiers_at(&state, &state.selection.head);
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
        let result = resolve_effective_modifiers_at(&state, &state.selection.head);
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
        let from = state.selection.anchor;
        let to = state.selection.head;
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
        let s =
            resolve_modifier_state_in_range(&state, &state.selection.anchor, &state.selection.head);
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
        let s =
            resolve_modifier_state_in_range(&state, &state.selection.anchor, &state.selection.head);
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
        let s =
            resolve_modifier_state_in_range(&state, &state.selection.anchor, &state.selection.head);
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
        let s =
            resolve_modifier_state_in_range(&state, &state.selection.anchor, &state.selection.head);
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
        let s =
            resolve_modifier_state_in_range(&state, &state.selection.anchor, &state.selection.head);
        assert_eq!(s.alignment, editor_common::Tri::Mixed);
    }

    #[test]
    fn facade_collapsed_lifts_to_uniform() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hi") [bold] } } }
            selection: (t1, 1)
        };
        let s = resolve_modifier_state(&state);
        assert_eq!(s.bold, editor_common::Tri::Uniform { value: () });
    }

    #[test]
    fn facade_collapsed_absent_for_unset() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hi") } } }
            selection: (t1, 1)
        };
        let s = resolve_modifier_state(&state);
        assert_eq!(s.bold, editor_common::Tri::Absent);
    }

    #[test]
    fn range_block_gap_uniform_from_root() {
        let (state, ..) = state! {
            doc { root [block_gap(150)] { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let s =
            resolve_modifier_state_in_range(&state, &state.selection.anchor, &state.selection.head);
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
        let s =
            resolve_modifier_state_in_range(&state, &state.selection.anchor, &state.selection.head);
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
        let s =
            resolve_modifier_state_in_range(&state, &state.selection.anchor, &state.selection.head);
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
        let s = resolve_modifier_state(&state);
        assert_eq!(s.bold, editor_common::Tri::Mixed);
    }
}
