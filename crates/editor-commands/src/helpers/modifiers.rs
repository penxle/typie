use editor_common::StrExt;
use editor_model::{Modifier, Node, NodeRef};
use editor_schema::{Expand, ModifierSpecExt};
use editor_state::{PendingModifier, PendingModifiers};

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

fn apply_pending_delta(mut modifiers: Vec<Modifier>, pending: &PendingModifiers) -> Vec<Modifier> {
    for pm in pending {
        match pm {
            PendingModifier::Set(m) => {
                modifiers.retain(|existing| existing.as_type() != m.as_type());
                modifiers.push(m.clone());
            }
            PendingModifier::Unset(t) => {
                modifiers.retain(|existing| existing.as_type() != *t);
            }
        }
    }

    modifiers
}

/// Collects inherited modifiers from the ancestor chain (excluding the node itself).
/// For each modifier type, returns the nearest ancestor's value.
/// Root has all modifiers (invariant).
pub(crate) fn resolve_inherited_modifiers(node: &NodeRef) -> Vec<Modifier> {
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

#[cfg(test)]
mod tests {
    use editor_macros::state;

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
}
