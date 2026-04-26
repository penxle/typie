use editor_model::{Doc, NodeId, Schema, Subtree};
use editor_state::State;
use proptest::prelude::*;

use crate::Step;
use crate::test_utils::proptest::doc::arb_doc;
use crate::test_utils::proptest::step::{arb_step_for_anchor, walk_node_ids};

#[derive(Debug, Clone)]
pub struct SwallowScenario {
    pub state: State,
    pub local: Step,
    pub against: Step,
}

pub fn arb_swallow_scenario() -> impl Strategy<Value = SwallowScenario> {
    arb_doc()
        .prop_filter_map(
            "doc must have a non-structural ancestor with descendants for swallow",
            |(state, _index)| {
                pick_swallow_anchor(&state).map(|(parent_id, idx, ancestor_id, descendants)| {
                    (state, parent_id, idx, ancestor_id, descendants)
                })
            },
        )
        .prop_flat_map(|(state, parent_id, idx, ancestor_id, descendants)| {
            let subtree = Subtree::capture(&state.doc, ancestor_id).expect("captured");
            let against = Step::RemoveSubtree {
                parent_id,
                index: idx,
                subtree,
            };
            let state_for_step = state.clone();
            (
                Just(state),
                Just(against),
                proptest::sample::select(descendants)
                    .prop_flat_map(move |d| arb_step_for_anchor(state_for_step.clone(), d)),
            )
                .prop_map(|(state, against, local)| SwallowScenario {
                    state,
                    local,
                    against,
                })
        })
}

fn pick_swallow_anchor(state: &State) -> Option<(NodeId, usize, NodeId, Vec<NodeId>)> {
    for parent_id in walk_node_ids(&state.doc) {
        let entry = state.doc.get_entry(parent_id)?;
        for (i, cid) in entry.children.iter().enumerate() {
            let cty = state.doc.get_entry(*cid)?.node.as_type();
            if Schema::node_spec(cty).structural {
                continue;
            }
            let descendants = collect_descendants(&state.doc, *cid);
            if descendants.is_empty() {
                continue;
            }
            // Ensure the parent permits removing this child without violating its content schema.
            // RemoveSubtree apply rejects when the resulting children sequence fails to match.
            let mut probe = entry.children.clone();
            probe.remove(i);
            let probe_types: Vec<_> = probe
                .iter()
                .filter_map(|id| state.doc.get_entry(*id).map(|e| e.node.as_type()))
                .collect();
            if !Schema::node_spec(entry.node.as_type())
                .content
                .matches_sequence(&probe_types)
            {
                continue;
            }
            return Some((parent_id, i, *cid, descendants));
        }
    }
    None
}

fn collect_descendants(doc: &Doc, root: NodeId) -> Vec<NodeId> {
    let mut all = Vec::new();
    walk_collect(doc, root, &mut all);
    all.into_iter().filter(|id| *id != root).collect()
}

fn walk_collect(doc: &Doc, id: NodeId, out: &mut Vec<NodeId>) {
    out.push(id);
    if let Some(e) = doc.get_entry(id) {
        for c in &e.children {
            walk_collect(doc, *c, out);
        }
    }
}

#[cfg(test)]
mod sanity {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn against_apply_succeeds_and_swallows_local_anchor(scenario in arb_swallow_scenario()) {
            let SwallowScenario { state, local, against } = scenario;
            prop_assert!(against.apply(&state).is_ok(), "against must apply: {against:?}");
            let local_anchor = anchor_of(&local).expect("local must have anchor");
            if let Step::RemoveSubtree { subtree, .. } = &against {
                prop_assert!(
                    subtree.contains_node(local_anchor),
                    "against subtree must contain local anchor; subtree={subtree:?} anchor={local_anchor:?}"
                );
            } else {
                prop_assert!(false, "against must be RemoveSubtree");
            }
        }
    }

    fn anchor_of(step: &Step) -> Option<NodeId> {
        match step {
            Step::InsertText { node_id, .. }
            | Step::RemoveText { node_id, .. }
            | Step::AddModifier { node_id, .. }
            | Step::RemoveModifier { node_id, .. }
            | Step::SetModifiers { node_id, .. }
            | Step::SetNode { node_id, .. } => Some(*node_id),
            _ => None,
        }
    }
}
