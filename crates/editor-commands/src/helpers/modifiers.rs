use editor_model::{
    ContextExpr, Doc, Expand, Modifier, ModifierType, Node, NodeId, NodeRef, NodeType, Schema,
};
use editor_state::{PendingModifier, PendingModifiers, ResolvedSelection};

pub(crate) fn resolve_effective_modifiers(
    node: &NodeRef,
    offset: usize,
    pending_modifiers: &PendingModifiers,
) -> Vec<Modifier> {
    let base_modifiers = resolve_base_modifiers(node, offset);
    apply_pending_delta(base_modifiers, pending_modifiers)
}

fn resolve_base_modifiers(node: &NodeRef, offset: usize) -> Vec<Modifier> {
    let Node::Text(text_node) = node.node() else {
        return vec![];
    };

    let node_len = text_node.text.len();
    let at_start = offset == 0 && node_len > 0;
    let at_end = offset == node_len && node_len > 0;

    if !at_start && !at_end {
        return node.modifiers().cloned().collect();
    }

    node.modifiers()
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

/// Collects inherited modifiers from the ancestor chain (excluding the node itself).
/// For each modifier type, returns the nearest ancestor's value.
/// Root has all modifiers (invariant).
// TODO: dedup with editor_state::modifier_resolution::resolve_inherited_modifiers — follow-up plan
pub(crate) fn resolve_inherited_modifiers(node: &NodeRef) -> Vec<Modifier> {
    let mut found = Vec::new();
    for ancestor in node.ancestors().skip(1) {
        for modifier in ancestor.modifiers() {
            let t = modifier.as_type();
            if !Schema::modifier_spec(t).inheritable {
                continue;
            }
            if !found.iter().any(|m: &Modifier| m.as_type() == t) {
                found.push(modifier.clone());
            }
        }
    }
    found
}

pub(crate) fn is_text_applicable(modifier_type: ModifierType) -> bool {
    Schema::modifier_spec(modifier_type)
        .context
        .rightmost_node_types()
        .contains(&NodeType::Text)
}

pub(crate) fn is_modifier_applicable_to_node(node: &NodeRef, modifier_type: ModifierType) -> bool {
    let context = &Schema::modifier_spec(modifier_type).context;
    let targets = context.rightmost_node_types();
    if !targets.contains(&node.as_type()) {
        return false;
    }

    let mut path: Vec<NodeType> = node.ancestors().map(|a| a.as_type()).collect();
    path.reverse();
    context.matches(&path)
}

pub(crate) fn filter_applicable_node_ids(
    doc: &Doc,
    node_ids: &[NodeId],
    modifier_type: ModifierType,
) -> Vec<NodeId> {
    node_ids
        .iter()
        .copied()
        .filter(|node_id| {
            doc.node(*node_id)
                .is_some_and(|node| is_modifier_applicable_to_node(&node, modifier_type))
        })
        .collect()
}

pub(crate) fn resolve_applicable_target_collapsed<'a>(
    doc: &'a Doc,
    cursor_node_id: NodeId,
    modifier_type: ModifierType,
) -> Option<NodeRef<'a>> {
    let context = &Schema::modifier_spec(modifier_type).context;
    let targets = context.rightmost_node_types();
    debug_assert!(
        !targets.is_empty(),
        "modifier {modifier_type:?} has no resolvable target types from {context:?}"
    );

    let cursor = doc.node(cursor_node_id)?;
    for n in cursor.ancestors() {
        if !targets.contains(&n.as_type()) {
            continue;
        }
        let mut path: Vec<NodeType> = n.ancestors().map(|a| a.as_type()).collect();
        path.reverse();
        if context.matches(&path) {
            return Some(n);
        }
    }
    None
}

pub(crate) fn collect_applicable_targets_in_range<'a>(
    doc: &'a Doc,
    resolved: &ResolvedSelection<'a>,
    modifier_type: ModifierType,
) -> Vec<NodeRef<'a>> {
    let context = &Schema::modifier_spec(modifier_type).context;
    let targets = context.rightmost_node_types();
    debug_assert!(
        !targets.is_empty(),
        "modifier {modifier_type:?} has no resolvable target types from {context:?}"
    );
    let mut out = Vec::new();
    let Some(root) = doc.root() else {
        return out;
    };
    walk_collect_targets(&root, resolved, &targets, context, &mut out);
    out
}

fn walk_collect_targets<'a>(
    node: &NodeRef<'a>,
    rs: &ResolvedSelection<'a>,
    targets: &[NodeType],
    context: &ContextExpr,
    out: &mut Vec<NodeRef<'a>>,
) {
    if !rs.intersects_subtree(node) {
        return;
    }
    if targets.contains(&node.as_type()) {
        let mut path: Vec<NodeType> = node.ancestors().map(|a| a.as_type()).collect();
        path.reverse();
        if context.matches(&path) {
            out.push(*node);
        }
    }
    for child in node.children() {
        walk_collect_targets(&child, rs, targets, context, out);
    }
}

pub(crate) fn check_range_all_has_modifier(nodes: &[NodeRef], modifier_type: ModifierType) -> bool {
    nodes
        .iter()
        .all(|node| node.modifiers().any(|m| m.as_type() == modifier_type))
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::ModifierType;

    use super::*;

    fn node_at(state: &editor_state::State) -> NodeRef<'_> {
        state.doc.node(state.selection.head.node_id).unwrap()
    }

    #[test]
    fn middle_of_bold_text_inherits_bold() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 2)
        };
        let result = resolve_effective_modifiers(&node_at(&state), 2, &state.pending_modifiers);
        assert_eq!(result, vec![Modifier::Bold]);
    }

    #[test]
    fn end_of_bold_text_inherits_bold() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 5)
        };
        let result = resolve_effective_modifiers(&node_at(&state), 5, &state.pending_modifiers);
        assert_eq!(result, vec![Modifier::Bold]);
    }

    #[test]
    fn start_of_bold_text_does_not_inherit() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 0)
        };
        let result = resolve_effective_modifiers(&node_at(&state), 0, &state.pending_modifiers);
        assert!(result.is_empty());
    }

    #[test]
    fn end_of_link_does_not_inherit() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Click") [link(href: "https://example.com".to_string())] } } }
            selection: (t1, 5)
        };
        let result = resolve_effective_modifiers(&node_at(&state), 5, &state.pending_modifiers);
        assert!(result.is_empty());
    }

    #[test]
    fn middle_of_link_inherits() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Click") [link(href: "https://example.com".to_string())] } } }
            selection: (t1, 2)
        };
        let result = resolve_effective_modifiers(&node_at(&state), 2, &state.pending_modifiers);
        assert_eq!(
            result,
            vec![Modifier::Link {
                href: "https://example.com".into()
            }]
        );
    }

    #[test]
    fn pending_set_adds_modifier() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
            pending_modifiers: [bold]
        };
        let result = resolve_effective_modifiers(&node_at(&state), 2, &state.pending_modifiers);
        assert_eq!(result, vec![Modifier::Bold]);
    }

    #[test]
    fn pending_unset_removes_modifier() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 2)
            pending_modifiers: [!bold]
        };
        let result = resolve_effective_modifiers(&node_at(&state), 2, &state.pending_modifiers);
        assert!(result.is_empty());
    }

    #[test]
    fn non_text_node_returns_only_pending() {
        let (state, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
            pending_modifiers: [bold]
        };
        let result = resolve_effective_modifiers(&node_at(&state), 0, &state.pending_modifiers);
        assert_eq!(result, vec![Modifier::Bold]);
    }

    #[test]
    fn empty_text_node_inherits_all() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("") [bold] } } }
            selection: (t1, 0)
        };
        let result = resolve_effective_modifiers(&node_at(&state), 0, &state.pending_modifiers);
        assert_eq!(result, vec![Modifier::Bold]);
    }

    #[test]
    fn inherited_weight_from_root_modifiers() {
        let (state, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };
        let inherited = resolve_inherited_modifiers(&node_at(&state));
        assert!(
            inherited
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 400 }))
        );
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
        let inherited = resolve_inherited_modifiers(&node_at(&state));
        assert!(
            inherited
                .iter()
                .any(|m| matches!(m, Modifier::FontWeight { value: 700 }))
        );
    }

    #[test]
    fn check_range_all_has_italic() {
        let (state, t1, t2) = state! {
            doc { root { paragraph {
                t1: text("Hello") [italic]
                t2: text("World") [italic]
            } } }
            selection: (t1, 0)
        };
        let nodes: Vec<_> = [t1, t2]
            .iter()
            .filter_map(|id| state.doc.node(*id))
            .collect();
        assert!(check_range_all_has_modifier(&nodes, ModifierType::Italic));
    }

    #[test]
    fn check_range_not_all_has_italic() {
        let (state, t1, t2) = state! {
            doc { root { paragraph {
                t1: text("Hello") [italic]
                t2: text("World")
            } } }
            selection: (t1, 0)
        };
        let nodes: Vec<_> = [t1, t2]
            .iter()
            .filter_map(|id| state.doc.node(*id))
            .collect();
        assert!(!check_range_all_has_modifier(&nodes, ModifierType::Italic));
    }

    #[test]
    fn check_range_empty_is_true() {
        let nodes: Vec<NodeRef> = vec![];
        assert!(check_range_all_has_modifier(&nodes, ModifierType::Italic));
    }

    #[test]
    fn applicable_target_for_line_height_in_text_returns_paragraph() {
        let (state, p1, t1) = state! {
            doc { root { p1: paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let target = resolve_applicable_target_collapsed(&state.doc, t1, ModifierType::LineHeight);
        assert_eq!(target.map(|n| n.id()), Some(p1));
    }

    #[test]
    fn applicable_target_for_block_gap_returns_root() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let target = resolve_applicable_target_collapsed(&state.doc, t1, ModifierType::BlockGap);
        assert_eq!(target.map(|n| n.id()), Some(editor_model::NodeId::ROOT));
    }

    #[test]
    fn applicable_target_for_bold_in_text_returns_text_self() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let target = resolve_applicable_target_collapsed(&state.doc, t1, ModifierType::Bold);
        assert_eq!(target.map(|n| n.id()), Some(t1));
    }

    #[test]
    fn applicable_target_for_line_height_on_hr_returns_none() {
        let (state, hr) = state! {
            doc { root { hr: horizontal_rule {} paragraph { text("Hello") } } }
            selection: (hr, 0)
        };
        let target = resolve_applicable_target_collapsed(&state.doc, hr, ModifierType::LineHeight);
        assert!(target.is_none());
    }

    #[test]
    fn applicable_target_when_cursor_is_paragraph_itself_returns_paragraph() {
        let (state, p1) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let target = resolve_applicable_target_collapsed(&state.doc, p1, ModifierType::LineHeight);
        assert_eq!(target.map(|n| n.id()), Some(p1));
    }

    #[test]
    fn collect_targets_line_height_two_paragraphs() {
        let (state, p1, _, p2, ..) = state! {
            doc { root {
                p1: paragraph { t1: text("Hello") }
                p2: paragraph { t2: text("World") }
            } }
            selection: (t1, 2) -> (t2, 3)
        };
        let resolved = state.selection.resolve(&state.doc).unwrap();
        let ids: Vec<_> =
            collect_applicable_targets_in_range(&state.doc, &resolved, ModifierType::LineHeight)
                .into_iter()
                .map(|n| n.id())
                .collect();
        assert_eq!(ids, vec![p1, p2]);
    }

    #[test]
    fn collect_targets_block_gap_returns_root_only() {
        let (state, ..) = state! {
            doc { root {
                paragraph { t1: text("Hello") }
                paragraph { t2: text("World") }
            } }
            selection: (t1, 0) -> (t2, 5)
        };
        let resolved = state.selection.resolve(&state.doc).unwrap();
        let ids: Vec<_> =
            collect_applicable_targets_in_range(&state.doc, &resolved, ModifierType::BlockGap)
                .into_iter()
                .map(|n| n.id())
                .collect();
        assert_eq!(ids, vec![editor_model::NodeId::ROOT]);
    }

    #[test]
    fn collect_targets_alignment_paragraph_image_paragraph() {
        let (state, p1, _, img, p2, ..) = state! {
            doc { root {
                p1: paragraph { t1: text("A") }
                img: image
                p2: paragraph { t2: text("B") }
            } }
            selection: (t1, 0) -> (t2, 1)
        };
        let resolved = state.selection.resolve(&state.doc).unwrap();
        let ids: Vec<_> =
            collect_applicable_targets_in_range(&state.doc, &resolved, ModifierType::Alignment)
                .into_iter()
                .map(|n| n.id())
                .collect();
        assert_eq!(ids, vec![p1, img, p2]);
    }

    #[test]
    fn collect_targets_line_height_partial_overlap_includes_paragraph() {
        let (state, p1, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hello") } } }
            selection: (t1, 1) -> (t1, 4)
        };
        let resolved = state.selection.resolve(&state.doc).unwrap();
        let ids: Vec<_> =
            collect_applicable_targets_in_range(&state.doc, &resolved, ModifierType::LineHeight)
                .into_iter()
                .map(|n| n.id())
                .collect();
        assert_eq!(ids, vec![p1]);
    }

    #[test]
    fn inherited_modifiers_skip_non_inheritable_ancestor() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph [alignment(Alignment::Right)] {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };
        let inherited = resolve_inherited_modifiers(&node_at(&state));
        assert!(
            !inherited
                .iter()
                .any(|m| matches!(m, Modifier::Alignment { .. })),
            "Alignment is inheritable: false; ancestor value must not appear as inherited"
        );
    }
}
