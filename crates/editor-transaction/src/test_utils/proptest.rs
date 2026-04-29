use proptest::prelude::*;

use editor_macros::state;
use editor_model::{Modifier, NodeId};
use editor_state::State;

use crate::Step;

const FIXED_TEXT: &str = "abcdefghij";
const TEXT_LEN: usize = 10;

#[derive(Debug, Clone)]
pub struct TransformScenario {
    pub state: State,
    pub a: Step,
    pub b: Step,
}

fn fixed_state() -> (State, NodeId, NodeId) {
    let (state, p1, t1) = state! {
        doc { root { p1: paragraph { t1: text("abcdefghij") } } }
        selection: (t1, 0)
    };
    (state, p1, t1)
}

pub fn arb_syncable_step(text_id: NodeId, paragraph_id: NodeId) -> impl Strategy<Value = Step> {
    prop_oneof![
        (0usize..=TEXT_LEN, "[a-z]{0,4}").prop_map(move |(offset, text)| Step::InsertText {
            node_id: text_id,
            offset,
            text
        }),
        (0usize..TEXT_LEN, 1usize..=4).prop_filter_map(
            "offset + len must be <= TEXT_LEN",
            move |(offset, len)| {
                if offset + len > TEXT_LEN {
                    return None;
                }
                let removed: String = FIXED_TEXT.chars().skip(offset).take(len).collect();
                Some(Step::RemoveText {
                    node_id: text_id,
                    offset,
                    text: removed,
                })
            }
        ),
        Just(Step::AddModifier {
            node_id: paragraph_id,
            modifier: Modifier::Bold
        }),
        Just(Step::AddModifier {
            node_id: paragraph_id,
            modifier: Modifier::Italic
        }),
        Just(Step::RemoveModifier {
            node_id: paragraph_id,
            modifier: Modifier::Underline
        }),
    ]
}

pub fn transform_scenario() -> impl Strategy<Value = TransformScenario> {
    let (state, paragraph_id, text_id) = fixed_state();
    let state_for_steps = state.clone();
    (
        Just(state),
        arb_syncable_step(text_id, paragraph_id),
        arb_syncable_step(text_id, paragraph_id),
    )
        .prop_filter("a and b must apply to state", move |(_, a, b)| {
            a.apply(&state_for_steps).is_ok() && b.apply(&state_for_steps).is_ok()
        })
        .prop_map(|(state, a, b)| TransformScenario { state, a, b })
}

#[cfg(test)]
mod sanity {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn generator_produces_applicable_scenarios(scenario in transform_scenario()) {
            let TransformScenario { state, a, b } = scenario;
            prop_assert!(a.apply(&state).is_ok(), "step a must apply");
            prop_assert!(b.apply(&state).is_ok(), "step b must apply");
        }
    }
}
