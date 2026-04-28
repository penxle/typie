use editor_common::StrExt;
use editor_model::{
    ContextExpr, Expand, Modifier, ModifierState, ModifierType, Node, NodeRef, NodeType, Schema,
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
        // Two-step filter: type prefilter via `modifier_target_types` + `ContextExpr::matches`.
        // The prefilter is necessary because `Not(...)` contexts (e.g. Bold's `!FoldTitle > Text`)
        // are permissive — `context.matches` succeeds for any path that doesn't fit the negated
        // pattern, including ancestor paths. Restricting evaluation to the modifier's actual
        // target types prevents absent values from non-targets corrupting the aggregate.
        let targets = modifier_target_types(context);
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

/// Walk a `ContextExpr` to its rightmost-leaf node positions to discover the
/// concrete `NodeType`s the modifier targets (e.g. `Bold` → `[Text]`,
/// `Alignment` → `[Paragraph, Image, Table]`). Used as a type-level prefilter so
/// the aggregator doesn't conflate ancestors with actual targets when the
/// context uses `Not(...)` (which is permissive over ancestor paths).
///
/// Contract: assumes `Not(...)` wraps a `Child(...)` whose deepest leaf is the
/// actual target (e.g. `!FoldTitle > Text` means "Text not under FoldTitle";
/// target is `Text`). A bare `Not(Node(X))` would incorrectly return `[X]`, but
/// no current modifier uses that shape. The `debug_assert!` at the call site
/// catches degenerate cases (e.g. `Any` / `GlobStar` / `SelfRef`) where the
/// recursion bottoms out without yielding any concrete type.
fn modifier_target_types(expr: &ContextExpr) -> Vec<NodeType> {
    match expr {
        ContextExpr::Node(t) => vec![*t],
        ContextExpr::AnyOf(exprs) => exprs.iter().flat_map(modifier_target_types).collect(),
        ContextExpr::Child { child, .. } => modifier_target_types(child),
        ContextExpr::Not(inner) => modifier_target_types(inner),
        ContextExpr::SelfRef | ContextExpr::Any | ContextExpr::GlobStar => Vec::new(),
    }
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
