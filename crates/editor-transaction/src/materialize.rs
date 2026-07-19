use editor_crdt::Dot;
use editor_state::{ProjectedState, State};

use crate::steps::support;
use crate::{StepError, Transaction};

fn locate_scaffold(ps: &ProjectedState, target: Dot) -> Option<(Dot, Dot, usize)> {
    let mut node = target;
    let mut scaffold: Option<Dot> = None;
    loop {
        if node == Dot::ROOT {
            break;
        }
        if node.is_synthetic() {
            scaffold = Some(node);
        }
        node = ps.parent_of(node)?;
    }
    let s = scaffold?;
    let a = ps.parent_of(s)?;
    let idx = ps.child_elem_dots(a).iter().position(|d| *d == s)?;
    Some((a, s, idx))
}

fn child_index_path(ps: &ProjectedState, scaffold: Dot, target: Dot) -> Option<Vec<usize>> {
    let mut path = Vec::new();
    let mut node = target;
    while node != scaffold {
        let parent = ps.parent_of(node)?;
        let idx = ps.child_elem_dots(parent).iter().position(|d| *d == node)?;
        path.push(idx);
        node = parent;
    }
    path.reverse();
    Some(path)
}

pub fn can_materialize_repair_target(state: &State, target: Dot) -> bool {
    let ps = &state.projected;
    match locate_scaffold(ps, target) {
        None => true,
        Some((_, s, _)) => !support::subtree_has_unknown(ps, s),
    }
}

pub fn materialize_repair_target(tr: &mut Transaction, target: Dot) -> Result<Dot, StepError> {
    let (anchor, scaffold, index) = match locate_scaffold(&tr.state().projected, target) {
        None => return Ok(target),
        Some(loc) => loc,
    };
    if support::subtree_has_unknown(&tr.state().projected, scaffold) {
        return Err(StepError::UnknownBearingMaterialize { block: scaffold });
    }

    let captured = support::capture_subtree(&tr.state().projected, scaffold)
        .ok_or(StepError::NodeNotFound(scaffold))?;
    let synthetic_path = target
        .is_synthetic()
        .then(|| child_index_path(&tr.state().projected, scaffold, target))
        .flatten();

    let mut container: Option<Dot> = None;
    tr.batch::<_, StepError>(|tr| {
        // Delete the scaffold's owned real content (recorded against the real
        // anchor, so the inverse is a plain InsertSubtree), then re-issue it as a
        // real container aliased to those now-dead dots.
        tr.remove_subtree(scaffold)?;
        tr.reissue_subtree(anchor, index, captured)?;
        container = tr.state().projected.child_dot_at(anchor, index);
        Ok(())
    })?;
    let container = container.ok_or(StepError::NodeNotFound(anchor))?;

    if target.is_synthetic() {
        let ps = &tr.state().projected;
        let mut node = container;
        for idx in synthetic_path.unwrap_or_default() {
            match ps.child_dot_at(node, idx) {
                Some(next) => node = next,
                None => break,
            }
        }
        Ok(node)
    } else {
        let view = tr.view();
        Ok(view
            .alias_classes()
            .resolve_with(target, |d| view.node(d).is_some() || view.leaf(d).is_some()))
    }
}

#[cfg(test)]
mod tests {
    use editor_crdt::{ListOp, Op};
    use editor_macros::state;
    use editor_model::{EditOp, NodeType, SeqItem};
    use editor_state::State;

    use super::*;
    use crate::steps::support;

    // Injects a bare `ListItem[Paragraph]` directly under Root. Root does not accept
    // a ListItem, so the projection WRAPs it in a synthetic `BulletList` scaffold that
    // owns the real ListItem — a content-owning repair scaffold.
    fn root_wrapped_list_item() -> (State, Dot, Dot) {
        let (mut state, _p) = state! {
            doc { root { p: paragraph { text("x") } } }
            selection: (p, 0)
        };
        let end = support::seq_insert_pos(&state.projected, Dot::ROOT, 1).unwrap();
        let list_item = state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: end,
                item: SeqItem::Block {
                    node_type: NodeType::ListItem,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }))
            .unwrap()
            .id;
        let paragraph = state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos: end + 1,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT, list_item],
                    attrs: vec![],
                },
            }))
            .unwrap()
            .id;
        (state, list_item, paragraph)
    }

    fn scaffold_of(state: &State, real_child: Dot) -> Dot {
        state.projected.parent_of(real_child).unwrap()
    }

    fn alias_ops(ops: &[Op<EditOp>]) -> usize {
        ops.iter()
            .filter(|op| matches!(op.payload, EditOp::Alias(_)))
            .count()
    }

    #[test]
    fn real_target_without_scaffold_ancestor_is_a_noop() {
        let (state, p) = state! {
            doc { root { p: paragraph { text("hi") } } }
            selection: (p, 0)
        };
        let mut tr = Transaction::new(&state);
        let remapped = materialize_repair_target(&mut tr, p).unwrap();
        assert_eq!(
            remapped, p,
            "a real target under real ancestors is unchanged"
        );
        let (_, steps, ..) = tr.commit();
        assert!(steps.is_empty(), "no ops are produced for a real target");
    }

    #[test]
    fn wrap_scaffold_is_reissued_as_a_real_container_preserving_content() {
        let (state, list_item, paragraph) = root_wrapped_list_item();
        let scaffold = scaffold_of(&state, list_item);
        assert!(
            scaffold.is_synthetic(),
            "the wrapping BulletList is synthetic"
        );
        assert_eq!(
            state.projected.block_node_type(scaffold),
            Some(NodeType::BulletList)
        );

        let mut tr = Transaction::new(&state);
        let remapped = materialize_repair_target(&mut tr, paragraph).unwrap();
        assert!(!remapped.is_synthetic(), "remapped target is a real dot");

        let (after, ..) = tr.commit();
        let view = after.view();
        let root = view.root().unwrap();
        // Root now has the paragraph plus a real BulletList holding the list item.
        let bullet = root
            .child_blocks()
            .find(|b| b.node_type() == NodeType::BulletList)
            .expect("a real BulletList replaced the synthetic scaffold");
        assert!(!bullet.id().is_synthetic(), "the BulletList is now real");
        let li = bullet
            .child_blocks()
            .find(|b| b.node_type() == NodeType::ListItem)
            .expect("the list item survives inside the real BulletList");
        assert!(!li.id().is_synthetic());
        let para = li
            .child_blocks()
            .find(|b| b.node_type() == NodeType::Paragraph)
            .expect("the list item's paragraph survives");
        assert_eq!(
            remapped,
            para.id(),
            "remap points at the re-issued paragraph"
        );
    }

    #[test]
    fn synthetic_scaffold_target_remaps_to_the_new_container() {
        let (state, list_item, _paragraph) = root_wrapped_list_item();
        let scaffold = scaffold_of(&state, list_item);

        let mut tr = Transaction::new(&state);
        let remapped = materialize_repair_target(&mut tr, scaffold).unwrap();
        assert!(!remapped.is_synthetic());
        assert_eq!(
            tr.state().projected.block_node_type(remapped),
            Some(NodeType::BulletList),
            "a synthetic scaffold target remaps to the real container of the same type"
        );
    }

    #[test]
    fn reissue_emits_a_single_alias_op_pairing_real_content() {
        let (state, _list_item, paragraph) = root_wrapped_list_item();
        let mut tr = Transaction::new(&state);
        materialize_repair_target(&mut tr, paragraph).unwrap();
        let ops = tr.ops_for_test();
        assert_eq!(
            alias_ops(&ops),
            1,
            "re-issuing the owned real content emits exactly one alias op"
        );
    }

    #[test]
    fn unknown_bearing_scaffold_is_rejected_losslessly() {
        let (mut state, list_item, paragraph) = root_wrapped_list_item();
        // Inject an Unknown leaf inside the scaffold-owned paragraph.
        let pos =
            support::child_seq_insert_pos(&state.projected, paragraph, 0, NodeType::Text).unwrap();
        state
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos,
                item: SeqItem::Unknown {
                    tag: 7,
                    bytes: vec![0xAB],
                },
            }))
            .unwrap();
        let scaffold = scaffold_of(&state, list_item);
        assert!(support::subtree_has_unknown(&state.projected, scaffold));

        let mut tr = Transaction::new(&state);
        let before = tr.ops_for_test().len();
        let result = materialize_repair_target(&mut tr, paragraph);
        assert!(
            matches!(&result, Err(StepError::UnknownBearingMaterialize { block }) if *block == scaffold),
            "unknown-bearing scaffold is rejected, not silently dropped — {result:?}"
        );
        assert_eq!(
            tr.ops_for_test().len(),
            before,
            "rejection emits no ops (lossless)"
        );
    }

    #[test]
    fn block_op_on_synthetic_scaffold_is_lost_without_the_gate_but_lands_after_it() {
        use editor_model::{Alignment, ChildView, Modifier, ModifierType};

        fn any_real_block_center_aligned(state: &State) -> bool {
            let view = state.view();
            let Some(root) = view.root() else {
                return false;
            };
            root.descendants().any(|c| match c {
                ChildView::Block(b) => {
                    !b.id().is_synthetic()
                        && b.block_modifier(ModifierType::Alignment)
                            == Some(&Modifier::Alignment {
                                value: Alignment::Center,
                            })
                }
                ChildView::Leaf(_) => false,
            })
        }

        let (state, list_item, _p) = root_wrapped_list_item();
        let scaffold = scaffold_of(&state, list_item);
        let center = Modifier::Alignment {
            value: Alignment::Center,
        };

        // Targeting the synthetic scaffold directly: the op applies to a dot that no
        // real block owns, so no real block ends up carrying it.
        let mut tr = Transaction::new(&state);
        tr.add_modifier(scaffold, center.clone()).unwrap();
        let (lost, ..) = tr.commit();
        assert!(
            !any_real_block_center_aligned(&lost),
            "a block op against the synthetic scaffold reaches no real block (silent loss)"
        );

        // Through the gate: materialize first, then the same op lands on the real container.
        let mut tr = Transaction::new(&state);
        let real = materialize_repair_target(&mut tr, scaffold).unwrap();
        tr.add_modifier(real, center.clone()).unwrap();
        let (kept, ..) = tr.commit();
        assert!(
            any_real_block_center_aligned(&kept),
            "materializing the scaffold first lets the op land on a real block"
        );
    }

    #[test]
    fn probe_matches_materialize_outcome_across_the_divergence() {
        // (a) real target, no scaffold → feasible + Ok.
        let (state_a, p) = state! {
            doc { root { p: paragraph { text("hi") } } }
            selection: (p, 0)
        };
        assert!(can_materialize_repair_target(&state_a, p));
        assert!(materialize_repair_target(&mut Transaction::new(&state_a), p).is_ok());

        // (b) content scaffold, no Unknown → feasible + Ok.
        let (state_b, _li, paragraph_b) = root_wrapped_list_item();
        assert!(can_materialize_repair_target(&state_b, paragraph_b));
        assert!(materialize_repair_target(&mut Transaction::new(&state_b), paragraph_b).is_ok());

        // (c) scaffold with Unknown → infeasible + Err. Probe must not mutate.
        let (mut state_c, list_item_c, paragraph_c) = root_wrapped_list_item();
        let pos = support::child_seq_insert_pos(&state_c.projected, paragraph_c, 0, NodeType::Text)
            .unwrap();
        state_c
            .projected_mut()
            .apply(EditOp::Seq(ListOp::Ins {
                pos,
                item: SeqItem::Unknown {
                    tag: 1,
                    bytes: vec![0x01],
                },
            }))
            .unwrap();
        let _ = list_item_c;
        assert!(!can_materialize_repair_target(&state_c, paragraph_c));
        assert!(materialize_repair_target(&mut Transaction::new(&state_c), paragraph_c).is_err());
    }
}
