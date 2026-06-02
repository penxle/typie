use editor_model::{ContextExpr, Modifier, Node, NodeId, NodeRef, NodeType};
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{capture_style_entry, is_text_applicable};

pub fn delete_style(tr: &mut Transaction, style_id: String) -> CommandResult {
    let style_modifiers: Vec<Modifier> = capture_style_entry(&tr.state().doc, &style_id)
        .map(|e| e.modifiers.into_iter().collect())
        .unwrap_or_default();

    // (target_node_id, additions on target itself, additions per text descendant)
    type Plan = (NodeId, Vec<Modifier>, Vec<(NodeId, Vec<Modifier>)>);
    let plans: Vec<Plan> = {
        let doc = &tr.state().doc;
        doc.nodes_iter()
            .filter_map(|(node_id, _)| {
                let node = doc.node(*node_id)?;
                let entry = node.entry();
                if entry.style.get().as_deref() != Some(style_id.as_str()) {
                    return None;
                }

                let text_descendants: Vec<NodeRef> = node
                    .descendants()
                    .filter(|n| matches!(n.node(), Node::Text(_)))
                    .collect();

                let mut on_target: Vec<Modifier> = Vec::new();
                let mut on_texts: Vec<(NodeId, Vec<Modifier>)> = text_descendants
                    .iter()
                    .map(|n| (n.id(), Vec::new()))
                    .collect();

                for modifier in &style_modifiers {
                    let ty = modifier.as_type();
                    let inline = is_text_applicable(ty);

                    if inline && !text_descendants.is_empty() {
                        for (i, text_node) in text_descendants.iter().enumerate() {
                            let already_explicit =
                                text_node.explicit_modifiers().any(|m| m.as_type() == ty);
                            if already_explicit {
                                continue;
                            }
                            if !valid_on(text_node, ty) {
                                continue;
                            }
                            on_texts[i].1.push(modifier.clone());
                        }
                    } else {
                        let already_explicit =
                            entry.modifiers.iter().any(|(_, m)| m.as_type() == ty);
                        if already_explicit {
                            continue;
                        }
                        if !valid_on(&node, ty) {
                            continue;
                        }
                        on_target.push(modifier.clone());
                    }
                }

                Some((*node_id, on_target, on_texts))
            })
            .collect()
    };

    for (target_id, on_target, on_texts) in plans {
        for modifier in on_target {
            tr.add_modifier(target_id, modifier)?;
        }
        for (text_id, modifiers) in on_texts {
            for modifier in modifiers {
                tr.add_modifier(text_id, modifier)?;
            }
        }
        tr.set_node_style(target_id, None)?;
    }

    tr.set_style(style_id, None)?;
    Ok(true)
}

fn valid_on(node: &NodeRef<'_>, ty: editor_model::ModifierType) -> bool {
    let ctx = &ty.spec().context;
    if *ctx == ContextExpr::Any {
        return true;
    }
    let path: Vec<NodeType> = node
        .ancestors()
        .map(|n| n.as_type())
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
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let (defined, ..) = transact!(initial, |tr| define_style(
            &mut tr,
            "heading-1".into(),
            "제목 1".into(),
            vec![],
        ));
        let (deleted, ..) = transact!(defined, |tr| delete_style(&mut tr, "heading-1".into()));

        assert!(!deleted.doc.style_present("heading-1"));
    }
}
