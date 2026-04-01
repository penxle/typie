use editor_state::{Composition, State};

use crate::{Step, StepError, StepOutput};

pub(crate) fn apply(state: &State, new: &Option<Composition>) -> Result<StepOutput, StepError> {
    let mut new_state = state.clone();
    new_state.composition = new.clone();

    Ok(StepOutput {
        state: new_state,
        validations: vec![],
    })
}

pub(crate) fn inverse(old: Option<Composition>, new: Option<Composition>) -> Step {
    Step::SetComposition { old: new, new: old }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::*;

    use crate::*;

    #[test]
    fn set_composition_apply() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let comp = Some(Composition {
            node_id: t1,
            offset: 0,
            text: "ㅎ".into(),
        });

        let step = Step::SetComposition {
            old: None,
            new: comp.clone(),
        };

        let output = step.apply(&state).unwrap();

        assert_eq!(output.state.composition, comp);
    }

    #[test]
    fn set_composition_inverse_roundtrip() {
        let (state, t1) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let comp = Some(Composition {
            node_id: t1,
            offset: 0,
            text: "ㅎ".into(),
        });

        let step = Step::SetComposition {
            old: None,
            new: comp,
        };

        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;

        assert_eq!(state3.composition, state.composition);
    }
}
