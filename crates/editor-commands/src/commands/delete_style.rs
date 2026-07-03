use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{
    ChildView, ContextExpr, Modifier, ModifierType, NodeType, NodeView, OwnModifier,
};
use editor_transaction::Transaction;

use crate::helpers::{capture_style_entry, is_text_applicable};
use crate::{CommandError, CommandResult};

/// Whether the leaf carries `ty` as an explicit (non-style) own modifier — the
/// segment-map equivalent of `LeafView::own_no_style(ty).is_some()`.
fn has_explicit_own(own: &BTreeMap<ModifierType, OwnModifier>, ty: ModifierType) -> bool {
    own.get(&ty).is_some_and(|o| !o.from_style)
}

enum Bake {
    BlockModifier { block: Dot, modifier: Modifier },
    LeafSpan { leaf: Dot, modifier: Modifier },
    ClearStyle { target: Dot },
}

pub fn delete_style(tr: &mut Transaction, style_id: String) -> CommandResult {
    let root_referenced = {
        let view = tr.view();
        let root_style = view
            .root()
            .and_then(|r| r.dot())
            .and_then(|d| tr.state().projected.node_styles().value_of(d));
        root_style.as_deref() == Some(style_id.as_str())
    };
    if root_referenced {
        return Err(CommandError::InvalidArgument(format!(
            "cannot delete style {style_id:?}: referenced by the root node (document default)"
        )));
    }

    let style_modifiers: Vec<Modifier> = capture_style_entry(tr.state(), &style_id)
        .map(|e| e.modifiers.into_iter().collect())
        .unwrap_or_default();

    let plans: Vec<Bake> = {
        let view = tr.view();
        let referencing: Vec<Dot> = tr
            .state()
            .projected
            .node_styles()
            .project()
            .iter()
            .filter_map(|(dot, val)| {
                if val.as_deref() == Some(style_id.as_str()) {
                    Some(*dot)
                } else {
                    None
                }
            })
            .collect();

        let mut out: Vec<Bake> = Vec::new();
        for elem in &referencing {
            let Some(op) = elem.as_op_dot() else {
                continue;
            };
            let dot = op.dot();
            if let Some(block) = view.node(*elem) {
                // Every leaf in the subtree is a direct inline child of the block or
                // one of its descendant blocks; each block serves own maps from segments.
                let leaves: Vec<(Dot, &BTreeMap<ModifierType, OwnModifier>)> =
                    std::iter::once(block)
                        .chain(block.descendants().filter_map(|c| match c {
                            ChildView::Block(b) => Some(b),
                            ChildView::Leaf(_) => None,
                        }))
                        .flat_map(|b| b.inline().into_iter().map(|it| (it.dot, it.own_modifiers)))
                        .collect();
                for modifier in &style_modifiers {
                    let ty = modifier.as_type();
                    if is_text_applicable(ty) && !leaves.is_empty() {
                        for (dot, own) in &leaves {
                            if has_explicit_own(own, ty) {
                                continue;
                            }
                            out.push(Bake::LeafSpan {
                                leaf: *dot,
                                modifier: modifier.clone(),
                            });
                        }
                    } else {
                        if block.block_modifier(ty).is_some() {
                            continue;
                        }
                        if !valid_on(&block, ty) {
                            continue;
                        }
                        out.push(Bake::BlockModifier {
                            block: *elem,
                            modifier: modifier.clone(),
                        });
                    }
                }
                out.push(Bake::ClearStyle { target: *elem });
            } else if let Some(st) = view.leaf_state_by_dot_slow(dot) {
                for modifier in &style_modifiers {
                    let ty = modifier.as_type();
                    if !is_text_applicable(ty) {
                        continue;
                    }
                    if has_explicit_own(st.own, ty) {
                        continue;
                    }
                    out.push(Bake::LeafSpan {
                        leaf: *elem,
                        modifier: modifier.clone(),
                    });
                }
                out.push(Bake::ClearStyle { target: *elem });
            }
        }
        out
    };

    for bake in plans {
        match bake {
            Bake::BlockModifier { block, modifier } => {
                tr.add_modifier(block, modifier)?;
            }
            Bake::LeafSpan { leaf, modifier } => {
                if let Some(op) = leaf.as_op_dot() {
                    let d = op.dot();
                    tr.add_span_modifier(d, d, modifier)?;
                }
            }
            Bake::ClearStyle { target } => {
                tr.set_node_style(target, None)?;
            }
        }
    }

    tr.set_style(style_id, None)?;
    Ok(true)
}

fn valid_on(node: &NodeView<'_>, ty: ModifierType) -> bool {
    let ctx = &ty.spec().context;
    if *ctx == ContextExpr::Any {
        return true;
    }
    let path: Vec<NodeType> = node
        .ancestors()
        .map(|n| n.node_type())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    ctx.matches(&path)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::commands::define_style;
    use crate::test_utils::*;

    #[test]
    fn removes_presence() {
        let (initial, _p1) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "heading-1".into(),
            "제목 1".into(),
            vec![],
        ));
        let (deleted, ..) = transact!(defined, |tr| delete_style(&mut tr, "heading-1".into()));

        assert!(!deleted.projected.styles().registered("heading-1"));
    }

    #[test]
    fn refuses_delete_of_root_referenced_style() {
        let (state, _p1) = state! {
            doc {
                styles { base: "기본" [font_size(1600)] }
                root @base [] { p1: paragraph { text("hi") } }
            }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        assert!(delete_style(&mut tr, "base".into()).is_err());

        let view = tr.view();
        let root_style = view
            .root()
            .and_then(|r| r.dot())
            .and_then(|d| tr.state().projected.node_styles().value_of(d));
        assert_eq!(
            root_style.as_deref(),
            Some("base"),
            "root.style must be preserved"
        );
    }

    #[test]
    fn allows_delete_of_unreferenced_style() {
        let (state, _p1) = state! {
            doc {
                styles { s: "s" [bold] }
                root { p1: paragraph { text("hi") } }
            }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        assert!(delete_style(&mut tr, "s".into()).is_ok());
    }
}
