use editor_common::Tri;
use editor_crdt::Dot;
use editor_macros::ffi;
use editor_model::{ChildView, DocView, LeafView, Modifier, ModifierType};
use editor_state::{PendingStyle, Position, State, leaves_in_range};
use serde::{Deserialize, Serialize};

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StyleInfo {
    pub id: String,
    pub name: String,
    pub modifiers: Vec<Modifier>,
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StyleRefValue {
    pub value: String,
}

pub fn resolve_style_entries(state: &State) -> Vec<StyleInfo> {
    let styles = state.projected.styles();
    let mut entries: Vec<StyleInfo> = styles
        .registered_entries()
        .iter()
        .map(|(id, entry)| StyleInfo {
            id: id.clone(),
            name: entry.name.get().clone(),
            modifiers: entry.modifiers.iter().cloned().collect(),
        })
        .collect();
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    entries
}

fn leaf_style(state: &State, dot: Dot) -> Option<String> {
    state
        .projected
        .projected()
        .node_styles
        .get(&dot)
        .cloned()
        .flatten()
}

fn inherited_block_style(state: &State, block: editor_model::NodeView<'_>) -> Option<String> {
    block
        .ancestors()
        .find_map(|node| node.dot().and_then(|dot| leaf_style(state, dot)))
}

fn effective_leaf_style(state: &State, leaf: LeafView<'_>) -> Option<String> {
    leaf_style(state, leaf.dot()).or_else(|| {
        leaf.parent()
            .and_then(|block| inherited_block_style(state, block))
    })
}

fn block_marker_style(state: &State, block: Dot) -> Option<String> {
    state
        .projected
        .projected()
        .node_markers
        .get(&block)
        .cloned()
        .flatten()
        .and_then(|m| m.style)
}

fn caret_leaf<'a>(view: &'a DocView<'a>, pos: Position) -> Option<LeafView<'a>> {
    let block = view.node(pos.node)?;
    let idx = if pos.offset > 0 { pos.offset - 1 } else { 0 };
    match block.child_at(idx) {
        Some(ChildView::Leaf(l)) => Some(l),
        _ => None,
    }
}

fn collapsed_base_style(state: &State, view: &DocView, pos: Position) -> Option<String> {
    if let Some(leaf) = caret_leaf(view, pos) {
        return effective_leaf_style(state, leaf);
    }
    let block = view.node(pos.node)?;
    if block.spec().is_textblock() {
        return block_marker_style(state, pos.node).or_else(|| inherited_block_style(state, block));
    }
    None
}

fn style_modifier_types(state: &State, style_id: &str) -> Vec<ModifierType> {
    state
        .projected
        .styles()
        .style_entry(style_id)
        .map(|entry| entry.modifiers.iter().map(Modifier::as_type).collect())
        .unwrap_or_default()
}

fn leaf_diverges(leaf: LeafView<'_>, style_types: &[ModifierType]) -> bool {
    leaf.own_modifiers()
        .iter()
        .filter(|(_, own)| !own.from_style)
        .any(|(ty, _)| style_types.contains(ty))
}

pub fn resolve_style_divergence(state: &State) -> bool {
    let Tri::Uniform { value } = resolve_applied_style(state) else {
        return false;
    };
    let style_id = value.value;
    let style_types = style_modifier_types(state, &style_id);
    if style_types.is_empty() {
        return false;
    }

    let Some(sel) = state.selection.as_ref() else {
        return false;
    };
    let view = state.view();

    if sel.is_collapsed() {
        return caret_leaf(&view, sel.head).is_some_and(|leaf| leaf_diverges(leaf, &style_types));
    }

    let Some(rs) = sel.resolve(&view) else {
        return false;
    };
    let leaves = leaves_in_range(&rs);
    leaves
        .into_iter()
        .any(|leaf| leaf_diverges(leaf, &style_types))
}

pub fn resolve_applied_style(state: &State) -> Tri<StyleRefValue> {
    let Some(sel) = state.selection.as_ref() else {
        return Tri::Absent;
    };
    let view = state.view();

    if sel.is_collapsed() {
        let base = collapsed_base_style(state, &view, sel.head);
        let applied = match &state.pending_style {
            Some(PendingStyle::Set { style_id }) => Some(style_id.clone()),
            Some(PendingStyle::Unset) => None,
            None => base,
        };
        return match applied {
            Some(value) => Tri::Uniform {
                value: StyleRefValue { value },
            },
            None => Tri::Absent,
        };
    }

    let Some(rs) = sel.resolve(&view) else {
        return Tri::Absent;
    };
    let leaves = leaves_in_range(&rs);

    let mut canonical: Option<String> = None;
    let mut absent_seen = false;
    let mut mixed = false;
    for leaf in leaves {
        let style = effective_leaf_style(state, leaf);
        match (style, &canonical) {
            (Some(s), Some(c)) if &s == c => {}
            (Some(_), Some(_)) => mixed = true,
            (Some(s), None) => {
                if absent_seen {
                    mixed = true;
                } else {
                    canonical = Some(s);
                }
            }
            (None, Some(_)) => mixed = true,
            (None, None) => absent_seen = true,
        }
        if mixed {
            break;
        }
    }

    if mixed {
        Tri::Mixed
    } else if let Some(value) = canonical {
        Tri::Uniform {
            value: StyleRefValue { value },
        }
    } else {
        Tri::Absent
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::PendingStyle;
    use editor_transaction::Transaction;

    use super::*;

    #[test]
    fn entries_empty_when_no_styles() {
        let (state, ..) = state! {
            doc { root { paragraph { text("Hi") } } }
            selection: none
        };
        assert!(resolve_style_entries(&state).is_empty());
    }

    #[test]
    fn applied_style_absent_when_no_selection() {
        let (state, ..) = state! {
            doc { root { paragraph { text("Hi") } } }
            selection: none
        };
        assert_eq!(resolve_applied_style(&state), Tri::Absent);
    }

    #[test]
    fn applied_style_uniform_on_collapsed_caret() {
        let (state, ..) = state! {
            doc { styles { heading1: "x" } root { p1: paragraph { text("Hi") @heading1 } } }
            selection: (p1, 1)
        };
        assert_eq!(
            resolve_applied_style(&state),
            Tri::Uniform {
                value: StyleRefValue {
                    value: "heading1".into()
                }
            }
        );
    }

    #[test]
    fn applied_style_uniform_over_styled_runs() {
        let (with_style, ..) = state! {
            doc { styles { h1: "x" } root { p1: paragraph { text("Hello") @h1 } } }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_eq!(
            resolve_applied_style(&with_style),
            Tri::Uniform {
                value: StyleRefValue { value: "h1".into() }
            }
        );
    }

    #[test]
    fn applied_style_mixed_over_runs_with_different_styles() {
        let (with_style, ..) = state! {
            doc {
                styles { h1: "x" h2: "y" }
                root { p1: paragraph { text("Hello") @h1 text("World") @h2 } }
            }
            selection: (p1, 0) -> (p1, 10)
        };
        assert_eq!(resolve_applied_style(&with_style), Tri::Mixed);
    }

    #[test]
    fn applied_style_mixed_when_some_runs_unstyled() {
        let (with_style, ..) = state! {
            doc {
                styles { h1: "x" }
                root { p1: paragraph { text("Hello") @h1 text("World") } }
            }
            selection: (p1, 0) -> (p1, 10)
        };
        assert_eq!(resolve_applied_style(&with_style), Tri::Mixed);
    }

    #[test]
    fn applied_style_absent_over_unstyled_runs() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_eq!(resolve_applied_style(&state), Tri::Absent);
    }

    #[test]
    fn applied_style_uses_inherited_root_style_over_unstyled_runs() {
        let (state, ..) = state! {
            doc {
                styles { base: "기본" [font_size(1600)] }
                root @base [] { p1: paragraph { text("Hello") } }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_eq!(
            resolve_applied_style(&state),
            Tri::Uniform {
                value: StyleRefValue {
                    value: "base".into()
                }
            }
        );
    }

    #[test]
    fn applied_style_reflects_pending_set_at_collapsed_caret() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let mut tr = Transaction::new(&state);
        tr.set_pending_style(Some(PendingStyle::Set {
            style_id: "h1".into(),
        }))
        .unwrap();
        let (next, _, _, _, _) = tr.commit();

        assert_eq!(
            resolve_applied_style(&next),
            Tri::Uniform {
                value: StyleRefValue { value: "h1".into() }
            }
        );
    }

    #[test]
    fn applied_style_reflects_pending_unset_over_styled_run_at_collapsed_caret() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let mut tr = Transaction::new(&state);
        tr.set_node_style(p1, Some("h1".into())).unwrap();
        tr.set_pending_style(Some(PendingStyle::Unset)).unwrap();
        let (next, _, _, _, _) = tr.commit();

        assert_eq!(resolve_applied_style(&next), Tri::Absent);
    }

    #[test]
    fn applied_style_uniform_on_empty_paragraph_marker() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.set_marker(
            p1,
            Some(editor_model::Marker {
                modifiers: vec![],
                style: Some("h1".into()),
            }),
        )
        .unwrap();
        let (next, _, _, _, _) = tr.commit();

        assert_eq!(
            resolve_applied_style(&next),
            Tri::Uniform {
                value: StyleRefValue { value: "h1".into() }
            }
        );
    }

    #[test]
    fn divergence_false_when_styled_run_has_no_override() {
        let (with_style, ..) = state! {
            doc { styles { h1: "x" [bold] } root { p1: paragraph { text("Hello") @h1 } } }
            selection: (p1, 0) -> (p1, 5)
        };
        assert!(!resolve_style_divergence(&with_style));
    }

    #[test]
    fn divergence_true_when_styled_run_overrides_style_modifier() {
        let (with_style, ..) = state! {
            doc {
                styles { h1: "x" [font_size(2800)] }
                root { p1: paragraph { text("Hello") @h1 [font_size(1200)] } }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert!(resolve_style_divergence(&with_style));
    }

    #[test]
    fn divergence_true_when_run_overrides_inherited_root_style_modifier() {
        let (state, ..) = state! {
            doc {
                styles { base: "기본" [font_size(1600)] }
                root @base [] { p1: paragraph { text("Hello") [font_size(1200)] } }
            }
            selection: (p1, 0) -> (p1, 5)
        };
        assert!(resolve_style_divergence(&state));
    }

    #[test]
    fn divergence_false_when_run_modifier_not_in_style() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let mut tr = Transaction::new(&initial);
        tr.set_style(
            "h1".into(),
            Some(editor_model::PlainStyleEntry {
                name: "x".into(),
                modifiers: [Modifier::FontSize { value: 2800 }].into_iter().collect(),
            }),
        )
        .unwrap();
        tr.set_node_style(p1, Some("h1".into())).unwrap();
        let (with_style, _, _, _, _) = tr.commit();

        assert!(!resolve_style_divergence(&with_style));
    }

    #[test]
    fn divergence_false_when_no_uniform_style() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 0) -> (p1, 5)
        };
        assert!(!resolve_style_divergence(&state));
    }
}
