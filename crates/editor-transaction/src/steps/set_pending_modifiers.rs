use editor_state::{PendingModifiers, State};

use crate::{Step, StepError, StepOutput};

pub(crate) fn apply(state: &State, new: &PendingModifiers) -> Result<StepOutput, StepError> {
    let mut new_state = state.clone();
    new_state.pending_modifiers = new.clone();

    Ok(StepOutput {
        state: new_state,
        validations: vec![],
    })
}

pub(crate) fn inverse(old: PendingModifiers, new: PendingModifiers) -> Step {
    Step::SetPendingModifiers { old: new, new: old }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::*;
    use editor_state::*;
    use smallvec::smallvec;

    use crate::*;

    #[test]
    fn set_pending_modifiers_apply() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let modifiers = smallvec![PendingModifier::Set(Modifier::Bold)];
        let step = Step::SetPendingModifiers {
            old: smallvec![],
            new: modifiers.clone(),
        };
        let output = step.apply(&state).unwrap();

        assert_eq!(output.state.pending_modifiers, modifiers);
    }

    #[test]
    fn set_pending_modifiers_inverse_roundtrip() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let modifiers = smallvec![PendingModifier::Set(Modifier::Bold)];
        let step = Step::SetPendingModifiers {
            old: smallvec![],
            new: modifiers,
        };

        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;

        assert_eq!(state3.pending_modifiers, state.pending_modifiers);
    }
}
