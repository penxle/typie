use hashbrown::HashMap;
use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_model::{
    EffectiveSources, Modifier, ModifierType, NodeType, NodeView, Schema, resolve_effective,
};

use crate::Position;
use crate::continuation::{apply_pending, continuation_at};
use crate::pending_modifier::PendingModifier;
use crate::projected_state::ProjectedState;
use crate::state::State;

fn parents_path(host: &NodeView) -> Vec<(NodeType, Option<Dot>)> {
    let mut v: Vec<(NodeType, Option<Dot>)> = host
        .ancestors()
        .skip(1)
        .map(|n| (n.node_type(), n.dot()))
        .collect();
    v.reverse();
    v
}

fn self_path(host: &NodeView) -> Vec<(NodeType, Option<Dot>)> {
    let mut v: Vec<(NodeType, Option<Dot>)> =
        host.ancestors().map(|n| (n.node_type(), n.dot())).collect();
    v.reverse();
    v
}

fn inherited_over(
    ancestors: &[(NodeType, Option<Dot>)],
    state: &ProjectedState,
) -> BTreeMap<ModifierType, Modifier> {
    let empty: HashMap<Dot, BTreeMap<ModifierType, Modifier>> = HashMap::new();
    let src = EffectiveSources {
        block_modifiers: state.block_modifiers(),
        explicit_spans: &empty,
        node_attrs: &state.projected().node_attrs,
    };
    resolve_effective(ancestors, None, NodeType::Text, true, &src)
}

/// Effective inline modifiers a caret at `pos` would carry (no pending overrides).
pub fn resolve_effective_modifiers_at(state: &State, pos: &Position) -> Vec<Modifier> {
    caret_modifiers(&state.projected, pos, &[])
        .into_values()
        .collect()
}

pub(crate) fn caret_modifiers(
    state: &ProjectedState,
    pos: &Position,
    pending: &[PendingModifier],
) -> BTreeMap<ModifierType, Modifier> {
    let view = state.view();
    let Some(host) = view.node(pos.node) else {
        return BTreeMap::new();
    };

    if !Schema::node_spec(host.node_type()).is_textblock() {
        let mut out = inherited_over(&parents_path(&host), state);
        apply_pending(&mut out, pending);
        return out;
    }

    let mut out = continuation_at(state, pos.node, pos.offset);
    apply_pending(&mut out, pending);
    for (ty, m) in inherited_over(&self_path(&host), state) {
        out.entry(ty).or_insert(m);
    }
    out
}

#[cfg(test)]
mod tests {
    use editor_crdt::{Dot, ListOp, OpGraph};
    use editor_model::{
        Anchor, AtomLeaf, Bias, EditOp, Modifier, ModifierAttrOp, ModifierType, NodeType, SeqItem,
    };

    use super::*;
    use crate::affinity::Affinity;
    use crate::pending_modifier::PendingModifier;

    fn seq_block(pos: usize, node_type: NodeType, parents: Vec<Dot>) -> EditOp {
        EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Block {
                node_type,
                parents,
                attrs: vec![],
            },
        })
    }

    fn new_para(leaves: &[SeqItem]) -> (ProjectedState, Dot, Dot, Vec<Dot>) {
        let mut g = OpGraph::<EditOp>::with_actor(1);
        let root = Dot::ROOT;
        let p = g
            .add_mut(seq_block(0, NodeType::Paragraph, vec![root]))
            .unwrap()
            .id;
        let mut leaf_dots = Vec::new();
        for (i, it) in leaves.iter().enumerate() {
            let d = g
                .add_mut(EditOp::Seq(ListOp::Ins {
                    pos: 1 + i,
                    item: it.clone(),
                }))
                .unwrap()
                .id;
            leaf_dots.push(d);
        }
        (ProjectedState::from_graph(g).unwrap(), root, p, leaf_dots)
    }

    fn set_span(state: &mut ProjectedState, leaf: Dot, m: Modifier) {
        state
            .apply(EditOp::Span(editor_model::SpanOp::AddSpan {
                start: Anchor {
                    id: leaf,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: leaf,
                    bias: Bias::After,
                },
                modifier: m,
            }))
            .unwrap();
    }

    fn set_block_mod(state: &mut ProjectedState, target: Dot, m: Modifier) {
        state
            .apply(EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target,
                modifier: m,
            }))
            .unwrap();
    }

    fn set_carry(state: &mut ProjectedState, target: Dot, m: Modifier) {
        state
            .apply(EditOp::NodeCarry(ModifierAttrOp::SetModifier {
                target,
                modifier: m,
            }))
            .unwrap();
    }

    fn caret(state: &ProjectedState, node: Dot, offset: usize) -> BTreeMap<ModifierType, Modifier> {
        caret_modifiers(state, &Position::new(node, offset), &[])
    }

    #[test]
    fn empty_paragraph_carry_surfaces() {
        let (mut state, _root, p, _) = new_para(&[]);
        set_carry(&mut state, p, Modifier::Bold);
        let out = caret(&state, p, 0);
        assert_eq!(out.get(&ModifierType::Bold), Some(&Modifier::Bold));
    }

    #[test]
    fn interior_run_has_all_modifiers() {
        let (mut state, _root, p, leaves) = new_para(&[SeqItem::Char('a'), SeqItem::Char('b')]);
        set_span(&mut state, leaves[0], Modifier::Bold);
        set_span(&mut state, leaves[1], Modifier::Bold);
        let out = caret(&state, p, 1);
        assert_eq!(out.get(&ModifierType::Bold), Some(&Modifier::Bold));
    }

    #[test]
    fn structural_container_caret_inherited_only() {
        let mut g = OpGraph::<EditOp>::with_actor(1);
        let root = Dot::ROOT;
        let bq = g
            .add_mut(seq_block(0, NodeType::Blockquote, vec![root]))
            .unwrap()
            .id;
        g.add_mut(seq_block(1, NodeType::Paragraph, vec![root, bq]))
            .unwrap();
        g.add_mut(EditOp::Seq(ListOp::Ins {
            pos: 2,
            item: SeqItem::Char('x'),
        }))
        .unwrap();
        let mut state = ProjectedState::from_graph(g).unwrap();
        set_block_mod(&mut state, root, Modifier::FontSize { value: 1600 });
        let out = caret(&state, bq, 0);
        assert_eq!(
            out.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
    }

    #[test]
    fn pending_overlay_applies() {
        let (state, _root, p, _) = new_para(&[SeqItem::Char('a')]);
        let out = caret_modifiers(
            &state,
            &Position::new(p, 1),
            &[PendingModifier::Set {
                modifier: Modifier::Italic,
            }],
        );
        assert_eq!(out.get(&ModifierType::Italic), Some(&Modifier::Italic));
    }

    #[test]
    fn empty_paragraph_pending_overlay_applies() {
        let (state, _root, p, _) = new_para(&[]);
        let out = caret_modifiers(
            &state,
            &Position::new(p, 0),
            &[PendingModifier::Set {
                modifier: Modifier::Italic,
            }],
        );
        assert_eq!(out.get(&ModifierType::Italic), Some(&Modifier::Italic));
    }

    #[test]
    fn pending_unset_keeps_inherited_value_alive() {
        let (mut state, root, p, _) = new_para(&[]);
        set_block_mod(&mut state, root, Modifier::FontWeight { value: 700 });
        let out = caret_modifiers(
            &state,
            &Position::new(p, 0),
            &[PendingModifier::Unset {
                ty: ModifierType::FontWeight,
            }],
        );
        assert_eq!(
            out.get(&ModifierType::FontWeight),
            Some(&Modifier::FontWeight { value: 700 }),
            "pending unset empties the own layer but the inherited value still surfaces"
        );
    }

    #[test]
    fn link_boundary_excluded() {
        let (mut state, _root, p, leaves) = new_para(&[SeqItem::Char('a'), SeqItem::Char('b')]);
        set_span(&mut state, leaves[0], Modifier::Link { href: "x".into() });
        let out = caret(&state, p, 1);
        assert!(!out.contains_key(&ModifierType::Link));
    }

    #[test]
    fn link_present_inside_run() {
        let (mut state, _root, p, leaves) = new_para(&[SeqItem::Char('a'), SeqItem::Char('b')]);
        set_span(&mut state, leaves[0], Modifier::Link { href: "x".into() });
        set_span(&mut state, leaves[1], Modifier::Link { href: "x".into() });
        let out = caret(&state, p, 1);
        assert_eq!(
            out.get(&ModifierType::Link),
            Some(&Modifier::Link { href: "x".into() })
        );
    }

    #[test]
    fn offset_zero_copies_right_neighbor_paint() {
        let (mut state, _root, p, leaves) = new_para(&[SeqItem::Char('a')]);
        set_span(&mut state, leaves[0], Modifier::Bold);
        let out = caret(&state, p, 0);
        assert_eq!(
            out.get(&ModifierType::Bold),
            Some(&Modifier::Bold),
            "typing before a run copies the right neighbor's paint"
        );
    }

    #[test]
    fn offset_zero_right_own_beats_inherited() {
        let (mut state, root, p, leaves) = new_para(&[SeqItem::Char('a')]);
        set_span(&mut state, leaves[0], Modifier::FontSize { value: 1200 });
        set_block_mod(&mut state, root, Modifier::FontSize { value: 1600 });
        let out = caret(&state, p, 0);
        assert_eq!(
            out.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1200 })
        );
    }

    #[test]
    fn caret_after_charlike_atom_reads_its_own_modifier() {
        let (mut state, _root, p, leaves) = new_para(&[SeqItem::Atom(AtomLeaf::Tab)]);
        set_span(&mut state, leaves[0], Modifier::Bold);
        let out = caret(&state, p, 1);
        assert_eq!(out.get(&ModifierType::Bold), Some(&Modifier::Bold));
    }

    #[test]
    fn plain_left_atom_blocks_earlier_paint() {
        let (mut state, _root, p, leaves) =
            new_para(&[SeqItem::Char('a'), SeqItem::Atom(AtomLeaf::HardBreak)]);
        set_span(&mut state, leaves[0], Modifier::Bold);
        let out = caret(&state, p, 2);
        assert!(
            !out.contains_key(&ModifierType::Bold),
            "the nearest left charlike is a plain hard break, so earlier bold does not carry"
        );
    }

    #[test]
    fn page_break_only_paragraph_reads_carry() {
        let (mut state, _root, p, _) = new_para(&[SeqItem::Atom(AtomLeaf::PageBreak)]);
        set_carry(&mut state, p, Modifier::Bold);
        let out = caret(&state, p, 0);
        assert_eq!(out.get(&ModifierType::Bold), Some(&Modifier::Bold));
    }

    #[test]
    fn caret_after_trailing_page_break_skips_to_bold_char() {
        let (mut state, _root, p, leaves) =
            new_para(&[SeqItem::Char('a'), SeqItem::Atom(AtomLeaf::PageBreak)]);
        set_span(&mut state, leaves[0], Modifier::Bold);
        let out = caret(&state, p, 2);
        assert_eq!(
            out.get(&ModifierType::Bold),
            Some(&Modifier::Bold),
            "caret after a trailing page break skips the non-charlike atom to find the bold char on its left"
        );
    }

    #[test]
    fn hard_break_only_paragraph_does_not_read_carry() {
        let (mut state, _root, p, _) = new_para(&[SeqItem::Atom(AtomLeaf::HardBreak)]);
        set_carry(&mut state, p, Modifier::Bold);
        let out = caret(&state, p, 0);
        assert!(
            !out.contains_key(&ModifierType::Bold),
            "a plain hard break is a charlike neighbor, so its empty paint is not replaced by carry"
        );
    }

    #[test]
    fn empty_carry_beats_inherited() {
        let (mut state, root, p, _) = new_para(&[]);
        set_block_mod(&mut state, root, Modifier::FontSize { value: 1600 });
        set_carry(&mut state, p, Modifier::FontSize { value: 1200 });
        let out = caret(&state, p, 0);
        assert_eq!(
            out.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1200 })
        );
    }

    #[test]
    fn out_of_range_carry_value_not_surfaced() {
        let (mut state, _root, p, _) = new_para(&[]);
        set_carry(&mut state, p, Modifier::FontSize { value: 399 });
        let out = caret(&state, p, 0);
        assert!(
            !out.contains_key(&ModifierType::FontSize),
            "a carry whose value is outside the valid space must not reach the caret"
        );
    }

    #[test]
    fn derived_empty_paragraph_inherited_only() {
        let mut g = OpGraph::<EditOp>::with_actor(1);
        let root = Dot::ROOT;
        let bq = g
            .add_mut(seq_block(0, NodeType::Blockquote, vec![root]))
            .unwrap()
            .id;
        g.add_mut(seq_block(1, NodeType::Paragraph, vec![root, bq]))
            .unwrap();
        g.add_mut(EditOp::Seq(ListOp::Ins {
            pos: 2,
            item: SeqItem::Char('x'),
        }))
        .unwrap();
        let mut state = ProjectedState::from_graph(g).unwrap();
        set_block_mod(&mut state, root, Modifier::FontSize { value: 1600 });
        let derived = {
            let view = state.view();
            view.root()
                .unwrap()
                .child_blocks()
                .find(|b| b.id().is_synthetic())
                .map(|b| b.id())
                .expect("derived trailing paragraph")
        };
        let out = caret(&state, derived, 0);
        assert_eq!(
            out.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
        assert!(!out.contains_key(&ModifierType::Bold));
    }

    fn fold_title_state() -> (ProjectedState, Dot) {
        let mut g = OpGraph::<EditOp>::with_actor(1);
        let root = Dot::ROOT;
        let fold = g
            .add_mut(seq_block(0, NodeType::Fold, vec![root]))
            .unwrap()
            .id;
        let title = g
            .add_mut(seq_block(1, NodeType::FoldTitle, vec![root, fold]))
            .unwrap()
            .id;
        let content = g
            .add_mut(seq_block(2, NodeType::FoldContent, vec![root, fold]))
            .unwrap()
            .id;
        g.add_mut(seq_block(3, NodeType::Paragraph, vec![root, fold, content]))
            .unwrap();
        (ProjectedState::from_graph(g).unwrap(), title)
    }

    #[test]
    fn empty_foldtitle_carry_surfaces() {
        let (mut state, title) = fold_title_state();
        set_carry(&mut state, title, Modifier::Bold);
        let out = caret(&state, title, 0);
        assert_eq!(out.get(&ModifierType::Bold), Some(&Modifier::Bold));
    }

    #[test]
    fn foldtitle_implicit_beats_ancestor_record() {
        let (mut state, title) = fold_title_state();
        set_block_mod(&mut state, Dot::ROOT, Modifier::FontWeight { value: 700 });
        let out = caret(&state, title, 0);
        assert_eq!(
            out.get(&ModifierType::FontWeight),
            Some(&Modifier::FontWeight { value: 500 }),
            "the FoldTitle implicit weight wins over the root record"
        );
    }

    #[test]
    fn caret_ignores_affinity() {
        let (mut state, _root, p, leaves) = new_para(&[SeqItem::Char('a')]);
        set_span(&mut state, leaves[0], Modifier::Bold);
        for offset in 0..=1 {
            let up = caret_modifiers(
                &state,
                &Position {
                    node: p,
                    offset,
                    affinity: Affinity::Upstream,
                },
                &[],
            );
            let down = caret_modifiers(
                &state,
                &Position {
                    node: p,
                    offset,
                    affinity: Affinity::Downstream,
                },
                &[],
            );
            assert_eq!(
                up, down,
                "affinity must not change the caret paint at offset {offset}"
            );
        }
    }

    fn arb_para_chars() -> impl proptest::strategy::Strategy<Value = Vec<char>> {
        use proptest::prelude::*;
        proptest::collection::vec(prop::sample::select(vec!['a', 'b', 'c']), 0..6)
    }

    proptest::proptest! {
        #[test]
        fn resolve_never_panics(chars in arb_para_chars()) {
            let leaves: Vec<SeqItem> = chars.iter().map(|c| SeqItem::Char(*c)).collect();
            let (mut state, _root, p, leaf_dots) = new_para(&leaves);
            if let Some(&d) = leaf_dots.first() {
                set_span(&mut state, d, Modifier::Bold);
            }
            for offset in 0..=chars.len() {
                for affinity in [Affinity::Downstream, Affinity::Upstream] {
                    let pos = Position { node: p, offset, affinity };
                    let _ = caret_modifiers(&state, &pos, &[]);
                }
            }
        }
    }
}
