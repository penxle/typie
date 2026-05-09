use editor_state::{BatchedState, Composition};

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(old: Option<Composition>, new: Option<Composition>) -> Step {
    Step::SetComposition { old: new, new: old }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    _validations: &mut Vec<Validation>,
    _old: Option<Composition>,
    new: Option<Composition>,
) -> Result<(), StepError> {
    batched.set_composition(new);
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::Composition;

    use crate::Step;

    #[test]
    fn set_composition_apply() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let comp = Some(Composition { start: 0, end: 1 });
        let step = Step::SetComposition {
            old: None,
            new: comp,
        };
        let output = step.apply(&state).unwrap();

        assert_eq!(output.state.composition, comp);
    }

    #[test]
    fn set_composition_inverse_roundtrip() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let comp = Some(Composition { start: 0, end: 1 });
        let step = Step::SetComposition {
            old: None,
            new: comp,
        };
        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;

        assert_eq!(state3.composition, state.composition);
    }
}
