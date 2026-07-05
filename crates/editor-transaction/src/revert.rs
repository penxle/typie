use std::collections::HashSet;

use editor_crdt::Dot;
use editor_model::{ChildView, Modifier, NodeView, PlainNode, Subtree};
use editor_state::State;

use crate::steps::support;
use crate::{HistoryMeta, StepError, Transaction};

/// Builds a transaction that transforms `state` back into `target`. Both states
/// must share dot lineage (i.e. `state` was produced by applying ops on top of
/// `target`); nodes are reconciled by their shared `Dot`.
pub fn build_revert_transaction(state: &State, target: &State) -> Result<Transaction, StepError> {
    let mut tr = Transaction::new(state);
    tr.update_meta(|m| m.history = HistoryMeta::Skip);
    tr.batch::<_, StepError>(|tr| {
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
    reconcile_node_marker(tr, target, id)?;
    reconcile_inline_children(tr, target, id)?;
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

#[derive(Clone, Debug, PartialEq)]
enum InlineLeafToken {
    Char(char),
    Atom {
        node: PlainNode,
        modifiers: Vec<Modifier>,
    },
}

#[derive(Clone, Debug)]
struct InlineLeafUnit {
    slot: usize,
    token: InlineLeafToken,
}

fn inline_leaf_units(node: &NodeView<'_>) -> Vec<InlineLeafUnit> {
    let mut units = Vec::new();
    for (slot, child) in node.children().enumerate() {
        let ChildView::Leaf(leaf) = child else {
            continue;
        };
        if let Some(ch) = leaf.as_char() {
            units.push(InlineLeafUnit {
                slot,
                token: InlineLeafToken::Char(ch),
            });
        } else if let Some(atom) = leaf.as_atom() {
            let atom_node = atom.clone().into_node().to_plain();
            let modifiers = node.leaf_own_modifiers_at(slot);
            units.push(InlineLeafUnit {
                slot,
                token: InlineLeafToken::Atom {
                    node: atom_node,
                    modifiers,
                },
            });
        }
    }
    units
}

fn remove_inline_leaf_units(
    tr: &mut Transaction,
    id: Dot,
    units: &[InlineLeafUnit],
) -> Result<(), StepError> {
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for unit in units {
        match ranges.last_mut() {
            Some((_, end)) if *end == unit.slot => *end = unit.slot + 1,
            _ => ranges.push((unit.slot, unit.slot + 1)),
        }
    }

    for (from, to) in ranges.into_iter().rev() {
        tr.remove_child_slots(id, from, to)?;
    }
    Ok(())
}

fn flush_insert_text(
    tr: &mut Transaction,
    id: Dot,
    start: &mut Option<usize>,
    text: &mut String,
) -> Result<(), StepError> {
    if let Some(offset) = start.take()
        && !text.is_empty()
    {
        tr.insert_text(id, offset, text)?;
        text.clear();
    }
    Ok(())
}

fn insert_inline_leaf_units(
    tr: &mut Transaction,
    id: Dot,
    units: &[InlineLeafUnit],
) -> Result<(), StepError> {
    let mut text_start = None;
    let mut next_text_slot = 0;
    let mut text = String::new();

    for unit in units {
        match &unit.token {
            InlineLeafToken::Char(ch) => {
                if text_start.is_none() || next_text_slot != unit.slot {
                    flush_insert_text(tr, id, &mut text_start, &mut text)?;
                    text_start = Some(unit.slot);
                }
                text.push(*ch);
                next_text_slot = unit.slot + 1;
            }
            InlineLeafToken::Atom { node, modifiers } => {
                flush_insert_text(tr, id, &mut text_start, &mut text)?;
                tr.insert_subtree(
                    id,
                    unit.slot,
                    Subtree {
                        node: node.clone(),
                        modifiers: modifiers.clone(),
                        marker: None,
                        children: Vec::new(),
                    },
                )?;
                next_text_slot = unit.slot + 1;
            }
        }
    }
    flush_insert_text(tr, id, &mut text_start, &mut text)?;
    Ok(())
}

fn reconcile_inline_children(
    tr: &mut Transaction,
    target: &State,
    id: Dot,
) -> Result<(), StepError> {
    let (cur_units, tgt_units) = {
        let cur_view = tr.view();
        let Some(cur_node) = cur_view.node(id) else {
            return Ok(());
        };
        let target_view = target.view();
        let Some(tgt_node) = target_view.node(id) else {
            return Ok(());
        };
        (inline_leaf_units(&cur_node), inline_leaf_units(&tgt_node))
    };

    let mut prefix = 0;
    while prefix < cur_units.len()
        && prefix < tgt_units.len()
        && cur_units[prefix].token == tgt_units[prefix].token
    {
        prefix += 1;
    }

    let mut suffix = 0;
    while suffix < (cur_units.len() - prefix)
        && suffix < (tgt_units.len() - prefix)
        && cur_units[cur_units.len() - 1 - suffix].token
            == tgt_units[tgt_units.len() - 1 - suffix].token
    {
        suffix += 1;
    }

    let cur_end = cur_units.len() - suffix;
    let tgt_end = tgt_units.len() - suffix;
    if prefix == cur_end && prefix == tgt_end {
        return Ok(());
    }

    remove_inline_leaf_units(tr, id, &cur_units[prefix..cur_end])?;
    insert_inline_leaf_units(tr, id, &tgt_units[prefix..tgt_end])?;
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
        PlainParagraphNode, Subtree,
    };
    use editor_state::assert_state_eq;

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
    fn reverts_text_change_after_tab() {
        let (target, p1) = state! {
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 0)
        };
        let mut pre = Transaction::new(&target);
        pre.remove_text(p1, 2, 1).unwrap();
        pre.insert_text(p1, 2, "c").unwrap();
        let (changed, ..) = pre.commit();

        let tr = build_revert_transaction(&changed, &target).unwrap();
        let (reverted, ..) = tr.commit();

        assert_state_eq!(&reverted, &target);
    }

    #[test]
    fn reverts_text_change_after_hard_break() {
        let (target, p1) = state! {
            doc { root { p1: paragraph { text("a") hard_break text("b") } } }
            selection: (p1, 0)
        };
        let mut pre = Transaction::new(&target);
        pre.remove_text(p1, 2, 1).unwrap();
        pre.insert_text(p1, 2, "c").unwrap();
        let (changed, ..) = pre.commit();

        let tr = build_revert_transaction(&changed, &target).unwrap();
        let (reverted, ..) = tr.commit();

        assert_state_eq!(&reverted, &target);
    }

    #[test]
    fn reverts_inserted_tab() {
        let (target, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 0)
        };
        let mut pre = Transaction::new(&target);
        pre.insert_subtree(p1, 1, Subtree::leaf(PlainNode::Tab(Default::default())))
            .unwrap();
        let (changed, ..) = pre.commit();

        let tr = build_revert_transaction(&changed, &target).unwrap();
        let (reverted, ..) = tr.commit();

        assert_state_eq!(&reverted, &target);
    }

    #[test]
    fn reverts_deleted_tab() {
        let (target, p1) = state! {
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 0)
        };
        let mut pre = Transaction::new(&target);
        pre.remove_child_slots(p1, 1, 2).unwrap();
        let (changed, ..) = pre.commit();

        let tr = build_revert_transaction(&changed, &target).unwrap();
        let (reverted, ..) = tr.commit();

        assert_state_eq!(&reverted, &target);
    }

    #[test]
    fn reverts_inserted_hard_break() {
        let (target, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 0)
        };
        let mut pre = Transaction::new(&target);
        pre.insert_subtree(
            p1,
            1,
            Subtree::leaf(PlainNode::HardBreak(Default::default())),
        )
        .unwrap();
        let (changed, ..) = pre.commit();

        let tr = build_revert_transaction(&changed, &target).unwrap();
        let (reverted, ..) = tr.commit();

        assert_state_eq!(&reverted, &target);
    }

    #[test]
    fn reverts_deleted_hard_break() {
        let (target, p1) = state! {
            doc { root { p1: paragraph { text("a") hard_break text("b") } } }
            selection: (p1, 0)
        };
        let mut pre = Transaction::new(&target);
        pre.remove_child_slots(p1, 1, 2).unwrap();
        let (changed, ..) = pre.commit();

        let tr = build_revert_transaction(&changed, &target).unwrap();
        let (reverted, ..) = tr.commit();

        assert_state_eq!(&reverted, &target);
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
}
