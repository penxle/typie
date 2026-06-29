use std::collections::HashSet;

use editor_crdt::Dot;
use editor_model::Modifier;
use editor_state::State;

use crate::steps::{set_style, support};
use crate::{HistoryMeta, StepError, Transaction};

/// Builds a transaction that transforms `state` back into `target`. Both states
/// must share dot lineage (i.e. `state` was produced by applying ops on top of
/// `target`); nodes are reconciled by their shared `Dot`.
pub fn build_revert_transaction(state: &State, target: &State) -> Result<Transaction, StepError> {
    let mut tr = Transaction::new(state);
    tr.update_meta(|m| m.history = HistoryMeta::Skip);
    tr.batch::<_, StepError>(|tr| {
        reconcile_styles(tr, target)?;
        let root = tr
            .view()
            .root()
            .map(|r| r.id())
            .ok_or(StepError::NodeNotFound(Dot::ROOT))?;
        reconcile_node(tr, target, root)?;
        Ok(())
    })?;
    Ok(tr)
}

fn reconcile_node(tr: &mut Transaction, target: &State, id: Dot) -> Result<(), StepError> {
    reconcile_attrs(tr, target, id)?;
    reconcile_modifiers(tr, target, id)?;
    reconcile_node_style(tr, target, id)?;
    reconcile_node_marker(tr, target, id)?;
    reconcile_text(tr, target, id)?;
    reconcile_children(tr, target, id)?;
    Ok(())
}

fn target_children(target: &State, id: Dot) -> Vec<Dot> {
    target
        .view()
        .node(id)
        .map(|n| n.child_blocks().map(|b| b.id()).collect())
        .unwrap_or_default()
}

fn current_children(tr: &Transaction, id: Dot) -> Vec<Dot> {
    tr.view()
        .node(id)
        .map(|n| n.child_blocks().map(|b| b.id()).collect())
        .unwrap_or_default()
}

fn reconcile_children(tr: &mut Transaction, target: &State, id: Dot) -> Result<(), StepError> {
    let target_ids = target_children(target, id);
    let target_set: HashSet<Dot> = target_ids.iter().copied().collect();

    for cid in current_children(tr, id) {
        if !target_set.contains(&cid) {
            tr.remove_subtree(cid)?;
        }
    }

    for (index, cid) in target_ids.iter().enumerate() {
        let cid = *cid;
        let live = tr.view().node(cid).is_some();
        if live {
            let cur = current_children(tr, id);
            if cur.iter().position(|x| *x == cid) != Some(index) {
                tr.move_node(cid, id, index)?;
            }
            reconcile_node(tr, target, cid)?;
        } else {
            revive_node(tr, target, cid, id, index)?;
        }
    }
    Ok(())
}

fn revive_node(
    tr: &mut Transaction,
    target: &State,
    cid: Dot,
    parent: Dot,
    index: usize,
) -> Result<(), StepError> {
    let subtree =
        support::capture_subtree(&target.projected, cid).ok_or(StepError::NodeNotFound(cid))?;
    tr.insert_subtree(parent, index, subtree)?;
    Ok(())
}

fn block_dot(id: Dot) -> Option<Dot> {
    id.as_op_dot().map(|d| d.dot())
}

fn reconcile_modifiers(tr: &mut Transaction, target: &State, id: Dot) -> Result<(), StepError> {
    let Some(dot) = block_dot(id) else {
        return Ok(());
    };
    if tr.view().node(id).is_none() {
        return Ok(());
    }
    let target_mods = target.projected.block_modifiers().modifiers_of(dot);
    let current_mods = tr.state().projected.block_modifiers().modifiers_of(dot);

    let add: Vec<Modifier> = target_mods
        .iter()
        .filter(|(ty, m)| current_mods.get(ty) != Some(*m))
        .map(|(_, m)| m.clone())
        .collect();
    let remove: Vec<Modifier> = current_mods
        .iter()
        .filter(|(ty, _)| !target_mods.contains_key(ty))
        .map(|(_, m)| m.clone())
        .collect();

    for m in add {
        tr.add_modifier(id, m)?;
    }
    for m in remove {
        tr.remove_modifier(id, m)?;
    }
    Ok(())
}

fn reconcile_text(tr: &mut Transaction, target: &State, id: Dot) -> Result<(), StepError> {
    let (Some(cur), tgt) = (
        tr.view().node(id).map(|n| n.inline_text()),
        target.view().node(id).map(|n| n.inline_text()),
    ) else {
        return Ok(());
    };
    let Some(tgt) = tgt else {
        return Ok(());
    };
    if cur == tgt {
        return Ok(());
    }
    let cur_chars: Vec<char> = cur.chars().collect();
    let tgt_chars: Vec<char> = tgt.chars().collect();

    let mut p = 0;
    while p < cur_chars.len() && p < tgt_chars.len() && cur_chars[p] == tgt_chars[p] {
        p += 1;
    }
    let mut s = 0;
    while s < (cur_chars.len() - p)
        && s < (tgt_chars.len() - p)
        && cur_chars[cur_chars.len() - 1 - s] == tgt_chars[tgt_chars.len() - 1 - s]
    {
        s += 1;
    }
    let remove_len = cur_chars.len() - s - p;
    if remove_len > 0 {
        tr.remove_text(id, p, remove_len)?;
    }
    let insert: String = tgt_chars[p..tgt_chars.len() - s].iter().collect();
    if !insert.is_empty() {
        tr.insert_text(id, p, &insert)?;
    }
    Ok(())
}

fn reconcile_attrs(tr: &mut Transaction, target: &State, id: Dot) -> Result<(), StepError> {
    let Some(target_plain) = target.view().node(id).map(|n| n.node().to_plain()) else {
        return Ok(());
    };
    let Some(current_plain) = tr.view().node(id).map(|n| n.node().to_plain()) else {
        return Ok(());
    };
    if current_plain != target_plain {
        tr.set_node(id, target_plain)?;
    }
    Ok(())
}

fn reconcile_styles(tr: &mut Transaction, target: &State) -> Result<(), StepError> {
    let target_ids: Vec<String> = target
        .projected
        .styles()
        .registered_entries()
        .keys()
        .cloned()
        .collect();
    for id in &target_ids {
        let Some(target_plain) = set_style::capture_style_entry(&target.projected, id) else {
            continue;
        };
        let current_plain = set_style::capture_style_entry(&tr.state().projected, id);
        if current_plain.as_ref() != Some(&target_plain) {
            tr.set_style(id.clone(), Some(target_plain))?;
        }
    }
    let current_ids: Vec<String> = tr
        .state()
        .projected
        .styles()
        .registered_entries()
        .keys()
        .cloned()
        .collect();
    for id in current_ids {
        if !target.projected.styles().registered(&id) {
            tr.set_style(id, None)?;
        }
    }
    Ok(())
}

fn reconcile_node_style(tr: &mut Transaction, target: &State, id: Dot) -> Result<(), StepError> {
    let Some(dot) = block_dot(id) else {
        return Ok(());
    };
    if tr.view().node(id).is_none() {
        return Ok(());
    }
    let target_style = target.projected.node_styles().value_of(dot);
    let current_style = tr.state().projected.node_styles().value_of(dot);
    if current_style != target_style {
        tr.set_node_style(id, target_style)?;
    }
    Ok(())
}

fn reconcile_node_marker(tr: &mut Transaction, target: &State, id: Dot) -> Result<(), StepError> {
    let Some(dot) = block_dot(id) else {
        return Ok(());
    };
    if tr.view().node(id).is_none() {
        return Ok(());
    }
    let target_marker = target.projected.node_markers().value_of(dot);
    let current_marker = tr.state().projected.node_markers().value_of(dot);
    if current_marker != target_marker {
        tr.set_marker(id, target_marker)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Transaction;
    use editor_macros::state;
    use editor_model::{
        CalloutVariant, ModifierType, Node, NodeType, NodeView, PlainCalloutNode, PlainNode,
        PlainParagraphNode, PlainStyleEntry, Subtree,
    };

    fn snapshot(state: &State) -> Vec<(usize, NodeType, String)> {
        fn walk(nv: &NodeView, depth: usize, out: &mut Vec<(usize, NodeType, String)>) {
            out.push((depth, nv.node_type(), nv.inline_text()));
            for b in nv.child_blocks() {
                walk(&b, depth + 1, out);
            }
        }
        let view = state.view();
        let mut out = Vec::new();
        if let Some(root) = view.root() {
            walk(&root, 0, &mut out);
        }
        out
    }

    fn block_mod(state: &State, id: &Dot, ty: ModifierType) -> Option<Modifier> {
        state
            .view()
            .node(*id)
            .and_then(|n| n.block_modifier(ty).cloned())
    }

    #[test]
    fn reverts_modifier_change() {
        let (target, p1) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let mut pre = Transaction::new(&target);
        pre.add_modifier(p1, Modifier::Bold).unwrap();
        let (changed, ..) = pre.commit();
        assert!(block_mod(&changed, &p1, ModifierType::Bold).is_some());

        let tr = build_revert_transaction(&changed, &target).unwrap();
        let (reverted, ..) = tr.commit();
        assert!(block_mod(&reverted, &p1, ModifierType::Bold).is_none());
    }

    #[test]
    fn reverts_text_change_preserving_common_affixes() {
        let (target, p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 0)
        };
        let mut pre = Transaction::new(&target);
        pre.insert_text(p1, 6, "BRAVE ").unwrap();
        let (changed, ..) = pre.commit();
        assert_eq!(
            changed.view().node(p1).unwrap().inline_text(),
            "hello BRAVE world"
        );

        let tr = build_revert_transaction(&changed, &target).unwrap();
        let (reverted, ..) = tr.commit();
        assert_eq!(
            reverted.view().node(p1).unwrap().inline_text(),
            "hello world"
        );
    }

    #[test]
    fn reverts_block_deletion_via_revival() {
        let (target, _p1, p2) = state! {
            doc { root { p1: paragraph { text("first") } p2: paragraph { text("second") } } }
            selection: (p1, 0)
        };
        let before = snapshot(&target);
        let mut pre = Transaction::new(&target);
        pre.remove_subtree(p2).unwrap();
        let (changed, ..) = pre.commit();
        assert_eq!(changed.view().root().unwrap().child_blocks().count(), 1);

        let tr = build_revert_transaction(&changed, &target).unwrap();
        let (reverted, ..) = tr.commit();
        assert_eq!(snapshot(&reverted), before);
    }

    #[test]
    fn reverts_block_insertion() {
        let (target, ..) = state! {
            doc { root { p1: paragraph { text("only") } } }
            selection: (p1, 0)
        };
        let before = snapshot(&target);
        let root = target.view().root().unwrap().id();
        let mut pre = Transaction::new(&target);
        pre.insert_subtree(
            root,
            1,
            Subtree::leaf(PlainNode::Paragraph(PlainParagraphNode::default())),
        )
        .unwrap();
        let (changed, ..) = pre.commit();

        let tr = build_revert_transaction(&changed, &target).unwrap();
        let (reverted, ..) = tr.commit();
        assert_eq!(snapshot(&reverted), before);
    }

    #[test]
    fn reverts_sibling_reorder() {
        let (target, _p1, p2) = state! {
            doc { root { p1: paragraph { text("one") } p2: paragraph { text("two") } } }
            selection: (p1, 0)
        };
        let before = snapshot(&target);
        let root = target.view().root().unwrap().id();
        let mut pre = Transaction::new(&target);
        pre.move_node(p2, root, 0).unwrap();
        let (changed, ..) = pre.commit();

        let tr = build_revert_transaction(&changed, &target).unwrap();
        let (reverted, ..) = tr.commit();
        assert_eq!(snapshot(&reverted), before);
    }

    #[test]
    fn reverts_node_attr_change() {
        let (target, c1) = state! {
            doc { root { c1: callout { paragraph { text("x") } } } }
            selection: (c1, 0)
        };
        let mut pre = Transaction::new(&target);
        pre.set_node(
            c1,
            PlainNode::Callout(PlainCalloutNode {
                variant: CalloutVariant::Warning,
            }),
        )
        .unwrap();
        let (changed, ..) = pre.commit();

        let tr = build_revert_transaction(&changed, &target).unwrap();
        let (reverted, _, _, _, meta) = tr.commit();
        assert!(matches!(meta.history, HistoryMeta::Skip));
        if let Node::Callout(n) = reverted.view().node(c1).unwrap().node() {
            assert_eq!(*n.variant.get(), CalloutVariant::Info);
        } else {
            panic!("expected callout");
        }
    }

    #[test]
    fn reverts_style_creation() {
        let (target, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        let mut pre = Transaction::new(&target);
        pre.set_style(
            "s2".into(),
            Some(PlainStyleEntry {
                name: "s2".into(),
                modifiers: std::iter::once(Modifier::Bold).collect(),
            }),
        )
        .unwrap();
        let (changed, ..) = pre.commit();
        assert!(changed.projected.styles().registered("s2"));

        let tr = build_revert_transaction(&changed, &target).unwrap();
        let (reverted, ..) = tr.commit();
        assert!(!reverted.projected.styles().registered("s2"));
    }

    #[test]
    fn reverts_style_deletion() {
        let (target, ..) = state! {
            doc {
                styles { s: "s" [bold] }
                root { p1: paragraph { text("hi") } }
            }
            selection: (p1, 0)
        };
        let mut pre = Transaction::new(&target);
        pre.set_style("s".into(), None).unwrap();
        let (changed, ..) = pre.commit();
        assert!(!changed.projected.styles().registered("s"));

        let tr = build_revert_transaction(&changed, &target).unwrap();
        let (reverted, ..) = tr.commit();
        assert!(reverted.projected.styles().registered("s"));
        let mods: Vec<Modifier> = reverted
            .projected
            .styles()
            .style_entry("s")
            .unwrap()
            .modifiers
            .iter()
            .cloned()
            .collect();
        assert_eq!(mods, vec![Modifier::Bold]);
    }

    #[test]
    fn reverts_node_style_ref_change() {
        let (target, p1) = state! {
            doc {
                styles { s: "s" [bold] }
                root { p1: paragraph { text("hi") } }
            }
            selection: (p1, 0)
        };
        let mut pre = Transaction::new(&target);
        pre.set_node_style(p1, Some("s".into())).unwrap();
        let (changed, ..) = pre.commit();

        let tr = build_revert_transaction(&changed, &target).unwrap();
        let (reverted, ..) = tr.commit();
        assert_eq!(reverted.projected.node_styles().value_of(p1), None);
    }
}
