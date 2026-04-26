mod doc;
mod step;
mod structural;
mod swallow;
mod text;

pub use doc::arb_doc;
pub use step::arb_syncable_step;
pub use swallow::{SwallowScenario, arb_swallow_scenario};

use proptest::prelude::*;

use editor_model::NodeId;
use editor_state::State;

use crate::Step;

#[derive(Debug, Clone)]
pub struct TransformScenario {
    pub state: State,
    pub a: Step,
    pub b: Step,
}

#[derive(Debug, Clone)]
pub struct MultiStepScenario {
    pub state: State,
    pub locals: Vec<Step>,
    pub againsts: Vec<Step>,
}

// LWW pairs violate strict OT commutativity by design — see docs/editor-architecture/lww-and-ot-invariant.md.
pub(crate) fn is_lww_pair(a: &Step, b: &Step) -> bool {
    use Step::*;

    fn anchor_node(s: &Step) -> Option<NodeId> {
        match s {
            AddModifier { node_id, .. }
            | RemoveModifier { node_id, .. }
            | SetModifiers { node_id, .. }
            | SetNode { node_id, .. } => Some(*node_id),
            _ => None,
        }
    }

    if matches!(a, SetDocumentAttrs { .. }) && matches!(b, SetDocumentAttrs { .. }) {
        return true;
    }

    match (anchor_node(a), anchor_node(b)) {
        (Some(na), Some(nb)) if na == nb => match (a, b) {
            (SetModifiers { .. }, _) | (_, SetModifiers { .. }) => true,
            (SetNode { .. }, _) | (_, SetNode { .. }) => true,
            (AddModifier { modifier: ma, .. }, RemoveModifier { modifier: mb, .. })
            | (RemoveModifier { modifier: ma, .. }, AddModifier { modifier: mb, .. })
                if ma == mb =>
            {
                true
            }
            _ => false,
        },
        _ => false,
    }
}

// SplitNode/MergeNode lack parent_id, so scopes_conflict cannot detect InsertSubtree × same-parent
// structural conflicts — these are UnsupportedStructural by design (documented for follow-up).
pub(crate) fn is_unsupported_structural_parent_pair(state: &State, a: &Step, b: &Step) -> bool {
    fn check(state: &State, insert: &Step, structural: &Step) -> bool {
        let insert_parent = match insert {
            Step::InsertSubtree { parent_id, .. } => *parent_id,
            _ => return false,
        };
        let structural_children: Vec<NodeId> = match structural {
            Step::SplitNode { node_id, .. } => vec![*node_id],
            Step::MergeNode {
                node_id, target_id, ..
            } => vec![*node_id, *target_id],
            _ => return false,
        };
        structural_children.iter().any(|child_id| {
            state
                .doc
                .get_entry(*child_id)
                .and_then(|e| e.parent)
                .map(|p| p == insert_parent)
                .unwrap_or(false)
        })
    }

    check(state, a, b) || check(state, b, a)
}

pub fn transform_scenario() -> impl Strategy<Value = TransformScenario> {
    arb_doc().prop_flat_map(|(state, index)| {
        let s_a = state.clone();
        let s_b = state.clone();
        let i_a = index.clone();
        let i_b = index.clone();
        let state_for_struct = state.clone();
        let state_for_filter = state.clone();
        (
            Just(state),
            arb_syncable_step(s_a, i_a),
            arb_syncable_step(s_b, i_b),
        )
            .prop_filter("LWW pair excluded from OT invariant", |(_, a, b)| {
                !is_lww_pair(a, b)
            })
            .prop_filter(
                "Structural × InsertSubtree same-parent (parent metadata missing in SplitNode/MergeNode)",
                move |(_, a, b)| {
                    !is_unsupported_structural_parent_pair(&state_for_struct, a, b)
                },
            )
            .prop_filter("a and b must apply to state", move |(_, a, b)| {
                a.apply(&state_for_filter).is_ok() && b.apply(&state_for_filter).is_ok()
            })
            .prop_map(|(state, a, b)| TransformScenario { state, a, b })
    })
}

pub fn multi_step_scenario() -> impl Strategy<Value = MultiStepScenario> {
    arb_doc().prop_flat_map(|(state, index)| {
        let s_l = state.clone();
        let s_a = state.clone();
        let i_l = index.clone();
        let i_a = index.clone();
        let state_pair = state.clone();
        let state_apply = state.clone();
        (
            Just(state),
            proptest::collection::vec(arb_syncable_step(s_l, i_l), 1..=5),
            proptest::collection::vec(arb_syncable_step(s_a, i_a), 1..=5),
        )
            .prop_filter(
                "all cross-pairs must be transform-safe (no LWW, no unsupported-structural-parent)",
                move |(_, locals, againsts)| {
                    for l in locals {
                        for a in againsts {
                            if is_lww_pair(l, a) {
                                return false;
                            }
                            if is_unsupported_structural_parent_pair(&state_pair, l, a) {
                                return false;
                            }
                        }
                    }
                    true
                },
            )
            .prop_filter(
                "locals and againsts must apply sequentially to state",
                move |(_, locals, againsts)| {
                    apply_seq_ok(&state_apply, locals) && apply_seq_ok(&state_apply, againsts)
                },
            )
            .prop_map(|(state, locals, againsts)| MultiStepScenario {
                state,
                locals,
                againsts,
            })
    })
}

fn apply_seq_ok(state: &State, steps: &[Step]) -> bool {
    let mut current = state.clone();
    for s in steps {
        match s.apply(&current) {
            Ok(eff) => current = eff.state,
            Err(_) => return false,
        }
    }
    true
}

#[cfg(test)]
mod sanity {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn generator_produces_applicable_scenarios(scenario in transform_scenario()) {
            let TransformScenario { state, a, b, .. } = scenario;
            prop_assert!(a.apply(&state).is_ok(), "step a must apply");
            prop_assert!(b.apply(&state).is_ok(), "step b must apply");
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use editor_macros::state;
    use editor_model::{Modifier, Node, NodeId, ParagraphNode, Subtree};

    use super::*;
    use crate::Step;

    #[test]
    fn structural_parent_pair_detects_insert_vs_split_same_parent() {
        let (state, p1, _t1) = state! {
            doc { root { p1: paragraph { t1: text("x") } } }
            selection: (t1, 0)
        };

        let insert = Step::InsertSubtree {
            parent_id: NodeId::ROOT,
            index: 1,
            subtree: Subtree::leaf(NodeId::new(), Node::Paragraph(ParagraphNode::default())),
        };
        let split = Step::SplitNode {
            node_id: p1,
            offset: 0,
            new_node_id: NodeId::new(),
        };

        assert!(is_unsupported_structural_parent_pair(
            &state, &insert, &split
        ));
        assert!(is_unsupported_structural_parent_pair(
            &state, &split, &insert
        ));
    }

    #[test]
    fn structural_parent_pair_rejects_when_parents_differ() {
        let (state, p1, _t1) = state! {
            doc { root { p1: paragraph { t1: text("x") } } }
            selection: (t1, 0)
        };

        let unrelated_parent = NodeId::new();
        let insert = Step::InsertSubtree {
            parent_id: unrelated_parent,
            index: 0,
            subtree: Subtree::leaf(NodeId::new(), Node::Paragraph(ParagraphNode::default())),
        };
        let split = Step::SplitNode {
            node_id: p1,
            offset: 0,
            new_node_id: NodeId::new(),
        };

        assert!(!is_unsupported_structural_parent_pair(
            &state, &insert, &split
        ));
        assert!(!is_unsupported_structural_parent_pair(
            &state, &split, &insert
        ));
    }

    #[test]
    fn structural_parent_pair_handles_merge_against_insert() {
        let (state, p1, _t1, p2, _t2) = state! {
            doc { root { p1: paragraph { t1: text("x") } p2: paragraph { t2: text("y") } } }
            selection: (t1, 0)
        };

        let insert = Step::InsertSubtree {
            parent_id: NodeId::ROOT,
            index: 1,
            subtree: Subtree::leaf(NodeId::new(), Node::Paragraph(ParagraphNode::default())),
        };
        let merge = Step::MergeNode {
            node_id: p2,
            target_id: p1,
            offset: 0,
        };

        assert!(is_unsupported_structural_parent_pair(
            &state, &insert, &merge
        ));
        assert!(is_unsupported_structural_parent_pair(
            &state, &merge, &insert
        ));
    }

    #[test]
    fn structural_parent_pair_rejects_non_structural_pairs() {
        let (state, _p1, t1) = state! {
            doc { root { p1: paragraph { t1: text("x") } } }
            selection: (t1, 0)
        };

        let insert = Step::InsertSubtree {
            parent_id: NodeId::ROOT,
            index: 0,
            subtree: Subtree::leaf(NodeId::new(), Node::Paragraph(ParagraphNode::default())),
        };
        let modifier = Step::AddModifier {
            node_id: t1,
            modifier: Modifier::Bold,
        };

        assert!(!is_unsupported_structural_parent_pair(
            &state, &insert, &modifier
        ));
        assert!(!is_unsupported_structural_parent_pair(
            &state, &modifier, &insert
        ));
    }
}
