use editor_common::Tri;
use editor_macros::ffi;
use editor_model::{Doc, Modifier, Node, NodeRef};
use editor_state::{ResolvedSelection, State};
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
    if !matches!(applied, Tri::Uniform { .. }) {
        return false;
    }

    let Some(sel) = state.selection.as_ref() else {
        return false;
    };

    if sel.is_collapsed() {
        let Some(node) = state.doc.node(sel.head.node_id) else {
            return false;
        };
        if matches!(node.node(), Node::Text(_)) {
            return node.explicit_modifiers().next().is_some();
        }
        let Some(textblock) = node.ancestors().find(|n| n.spec().is_textblock()) else {
            return false;
        };
        return subtree_has_inline_modifier(&textblock);
    }

    let Some(rs) = sel.resolve(&state.doc) else {
        return false;
    };
    let Some(root) = state.doc.root() else {
        return false;
    };
    walk_for_divergence(&root, &rs)
}

fn subtree_has_inline_modifier(node: &NodeRef<'_>) -> bool {
    if matches!(node.node(), Node::Text(_)) {
        return node.explicit_modifiers().next().is_some();
    }
    for child in node.children() {
        if subtree_has_inline_modifier(&child) {
            return true;
        }
    }
    false
}

fn walk_for_divergence<'a>(node: &NodeRef<'a>, rs: &ResolvedSelection<'a>) -> bool {
    if !rs.intersects_subtree(node) {
        return false;
    }
    if matches!(node.node(), Node::Text(_)) {
        return node.explicit_modifiers().next().is_some();
    }
    for child in node.children() {
        if walk_for_divergence(&child, rs) {
            return true;
        }
    }
    false
}

pub fn resolve_applied_style(state: &State) -> Tri<StyleRefValue> {
    let Some(sel) = state.selection.as_ref() else {
        return Tri::Absent;
    };

    if sel.is_collapsed() {
        let Some(node) = state.doc.node(sel.head.node_id) else {
            return Tri::Absent;
        };
        let textblock = node.ancestors().find(|n| n.spec().is_textblock());
        return textblock
            .map(|n| pick_block_style(&n))
            .unwrap_or(Tri::Absent);
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
    fold_textblocks(&root, &rs, &mut |block| {
        if mixed {
            return;
        }
        let style = block_style(&block);
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

fn pick_block_style(block: &NodeRef<'_>) -> Tri<StyleRefValue> {
    match block_style(block) {
        Some(value) => Tri::Uniform {
            value: StyleRefValue { value },
        },
        None => Tri::Absent,
    }
}

fn block_style(block: &NodeRef<'_>) -> Option<String> {
    block.entry().style.get().clone()
}

fn fold_textblocks<'a>(
    node: &NodeRef<'a>,
    rs: &ResolvedSelection<'a>,
    f: &mut dyn FnMut(NodeRef<'a>),
) {
    if !rs.intersects_subtree(node) {
        return;
    }
    if node.spec().is_textblock() {
        f(*node);
        return;
    }
    for child in node.children() {
        fold_textblocks(&child, rs, f);
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
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
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("Hi") } } }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        tr.set_node_style(p1, Some("heading-1".into())).unwrap();
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
}
