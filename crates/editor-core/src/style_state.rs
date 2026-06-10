use editor_common::Tri;
use editor_macros::ffi;
use editor_model::{Doc, Modifier, Node, NodeRef};
use editor_state::{PendingStyle, ResolvedSelection, State};
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

pub fn resolve_style_entries(doc: &Doc) -> Vec<StyleInfo> {
    let mut entries: Vec<StyleInfo> = doc
        .style_entries_iter()
        .filter(|(id, _)| doc.style_present(id))
        .map(|(id, entry)| StyleInfo {
            id: id.clone(),
            name: entry.name.get().clone(),
            modifiers: entry.modifiers.iter().cloned().collect(),
        })
        .collect();
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    entries
}

pub fn resolve_style_divergence(state: &State) -> bool {
    let applied = resolve_applied_style(state);
    let Tri::Uniform { value } = applied else {
        return false;
    };
    let style_id = value.value;

    let Some(sel) = state.selection.as_ref() else {
        return false;
    };

    if sel.is_collapsed() {
        let Some(node) = state.doc.node(sel.head.node_id) else {
            return false;
        };
        if is_run(&node) {
            return run_diverges_from_style(&node, &style_id);
        }
        return false;
    }

    let Some(rs) = sel.resolve(&state.doc) else {
        return false;
    };
    let Some(root) = state.doc.root() else {
        return false;
    };
    walk_for_divergence(&root, &rs, &style_id)
}

fn walk_for_divergence<'a>(node: &NodeRef<'a>, rs: &ResolvedSelection<'a>, style_id: &str) -> bool {
    if !rs.intersects_subtree(node) {
        return false;
    }
    if is_run(node) {
        return run_diverges_from_style(node, style_id);
    }
    for child in node.children() {
        if walk_for_divergence(&child, rs, style_id) {
            return true;
        }
    }
    false
}

fn run_diverges_from_style(run: &NodeRef<'_>, style_id: &str) -> bool {
    let doc = run.doc();
    let Some(style) = doc.style_entry(style_id) else {
        return false;
    };
    let style_types: Vec<_> = style.modifiers.iter().map(Modifier::as_type).collect();
    run.explicit_modifiers()
        .any(|m| style_types.contains(&m.as_type()))
}

pub fn resolve_applied_style(state: &State) -> Tri<StyleRefValue> {
    let Some(sel) = state.selection.as_ref() else {
        return Tri::Absent;
    };

    if sel.is_collapsed() {
        let Some(node) = state.doc.node(sel.head.node_id) else {
            return Tri::Absent;
        };
        let base = collapsed_base_style(&node);
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

    let Some(rs) = sel.resolve(&state.doc) else {
        return Tri::Absent;
    };
    let Some(root) = state.doc.root() else {
        return Tri::Absent;
    };

    let mut canonical: Option<String> = None;
    let mut absent_seen = false;
    let mut mixed = false;
    fold_runs(&root, &rs, &mut |run| {
        if mixed {
            return;
        }
        let style = run.entry().style.get().clone();
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
    });

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

fn collapsed_base_style(node: &NodeRef<'_>) -> Option<String> {
    if is_run(node) {
        return node.entry().style.get().clone();
    }
    if node.spec().is_textblock() && !node.children().any(|c| is_run(&c)) {
        return node.marker().and_then(|m| m.style.clone());
    }
    None
}

fn is_run(node: &NodeRef<'_>) -> bool {
    matches!(node.node(), Node::Text(_) | Node::Tab(_))
}

fn fold_runs<'a>(node: &NodeRef<'a>, rs: &ResolvedSelection<'a>, f: &mut dyn FnMut(NodeRef<'a>)) {
    if !rs.intersects_subtree(node) {
        return;
    }
    if is_run(node) {
        f(*node);
        return;
    }
    for child in node.children() {
        fold_runs(&child, rs, f);
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
        assert!(resolve_style_entries(&state.doc).is_empty());
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
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hi") } } }
            selection: (t1, 1)
        };
        let mut tr = Transaction::new(&state);
        tr.set_node_style(t1, Some("heading-1".into())).unwrap();
        let (next, _, _, _, _) = tr.commit();

        assert_eq!(
            resolve_applied_style(&next),
            Tri::Uniform {
                value: StyleRefValue {
                    value: "heading-1".into()
                }
            }
        );
    }

    #[test]
    fn applied_style_uniform_over_styled_runs() {
        let (initial, t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let mut tr = Transaction::new(&initial);
        tr.set_style(
            "h1".into(),
            Some(editor_model::PlainStyleEntry {
                name: "x".into(),
                modifiers: Default::default(),
            }),
        )
        .unwrap();
        tr.set_node_style(t1, Some("h1".into())).unwrap();
        let (with_style, _, _, _, _) = tr.commit();

        assert_eq!(
            resolve_applied_style(&with_style),
            Tri::Uniform {
                value: StyleRefValue { value: "h1".into() }
            }
        );
    }

    #[test]
    fn applied_style_mixed_over_runs_with_different_styles() {
        let (initial, t1, t2) = state! {
            doc { root { paragraph { t1: text("Hello") t2: text("World") } } }
            selection: (t1, 0) -> (t2, 5)
        };
        let mut tr = Transaction::new(&initial);
        tr.set_node_style(t1, Some("h1".into())).unwrap();
        tr.set_node_style(t2, Some("h2".into())).unwrap();
        let (with_style, _, _, _, _) = tr.commit();

        assert_eq!(resolve_applied_style(&with_style), Tri::Mixed);
    }

    #[test]
    fn applied_style_mixed_when_some_runs_unstyled() {
        let (initial, t1, _t2) = state! {
            doc { root { paragraph { t1: text("Hello") t2: text("World") } } }
            selection: (t1, 0) -> (t2, 5)
        };
        let mut tr = Transaction::new(&initial);
        tr.set_node_style(t1, Some("h1".into())).unwrap();
        let (with_style, _, _, _, _) = tr.commit();

        assert_eq!(resolve_applied_style(&with_style), Tri::Mixed);
    }

    #[test]
    fn applied_style_absent_over_unstyled_runs() {
        let (state, _t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_eq!(resolve_applied_style(&state), Tri::Absent);
    }

    #[test]
    fn applied_style_reflects_pending_set_at_collapsed_caret() {
        let (state, _t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
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
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let mut tr = Transaction::new(&state);
        tr.set_node_style(t1, Some("h1".into())).unwrap();
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
        let (initial, t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
        };
        let mut tr = Transaction::new(&initial);
        tr.set_style(
            "h1".into(),
            Some(editor_model::PlainStyleEntry {
                name: "x".into(),
                modifiers: [Modifier::Bold].into_iter().collect(),
            }),
        )
        .unwrap();
        tr.set_node_style(t1, Some("h1".into())).unwrap();
        let (with_style, _, _, _, _) = tr.commit();

        assert!(!resolve_style_divergence(&with_style));
    }

    #[test]
    fn divergence_true_when_styled_run_overrides_style_modifier() {
        let (initial, t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 5)
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
        tr.add_modifier(t1, Modifier::FontSize { value: 1200 })
            .unwrap();
        tr.set_node_style(t1, Some("h1".into())).unwrap();
        let (with_style, _, _, _, _) = tr.commit();

        assert!(resolve_style_divergence(&with_style));
    }

    #[test]
    fn divergence_false_when_run_modifier_not_in_style() {
        let (initial, t1) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
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
        tr.set_node_style(t1, Some("h1".into())).unwrap();
        let (with_style, _, _, _, _) = tr.commit();

        assert!(!resolve_style_divergence(&with_style));
    }

    #[test]
    fn divergence_false_when_no_uniform_style() {
        let (state, _t1) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 0) -> (t1, 5)
        };
        assert!(!resolve_style_divergence(&state));
    }
}
