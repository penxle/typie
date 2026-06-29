use proptest::prelude::*;

use editor_crdt::Dot;
use editor_macros::state;
use editor_model::Modifier;
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

fn fixed_state() -> (State, Dot) {
    let (state, p1) = state! {
        doc { root { p1: paragraph { text("abcdefghij") } } }
        selection: (p1, 0)
    };
    (state, p1)
}

pub fn arb_syncable_step(block: Dot) -> impl Strategy<Value = Step> {
    let b1 = block;
    let b2 = block;
    let b3 = block;
    let b4 = block;
    let b5 = block;
    prop_oneof![
        (0usize..=TEXT_LEN, "[a-z]{0,4}").prop_map(move |(offset, text)| Step::InsertText {
            block: b1,
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
                    block: b2,
                    offset,
                    text: removed,
                })
            }
        ),
        Just(Step::AddModifier {
            block: b3,
            modifier: Modifier::Bold
        }),
        Just(Step::AddModifier {
            block: b4,
            modifier: Modifier::Italic
        }),
        Just(Step::RemoveModifier {
            block: b5,
            modifier: Modifier::Underline
        }),
    ]
}

pub fn transform_scenario() -> impl Strategy<Value = TransformScenario> {
    let (state, block) = fixed_state();
    let state_for_steps = state.clone();
    (
        Just(state),
        arb_syncable_step(block),
        arb_syncable_step(block),
    )
        .prop_filter("a and b must apply to state", move |(_, a, b)| {
            a.apply(&state_for_steps).is_ok() && b.apply(&state_for_steps).is_ok()
        })
        .prop_map(|(state, a, b)| TransformScenario { state, a, b })
}

#[cfg(test)]
mod proptests {
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
