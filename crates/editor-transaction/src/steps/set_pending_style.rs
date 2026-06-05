use editor_state::{BatchedState, PendingStyle};

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(old: Option<PendingStyle>, new: Option<PendingStyle>) -> Step {
    Step::SetPendingStyle { old: new, new: old }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    _validations: &mut Vec<Validation>,
    _old: &Option<PendingStyle>,
    new: &Option<PendingStyle>,
) -> Result<(), StepError> {
    batched.set_pending_style(new.clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::PendingStyle;

    use crate::Step;

    #[test]
    fn set_pending_style_apply() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let pending = Some(PendingStyle::Set {
            style_id: "s1".into(),
        });
        let step = Step::SetPendingStyle {
            old: None,
            new: pending.clone(),
        };
        let output = step.apply(&state).unwrap();

        assert_eq!(output.state.pending_style, pending);
    }

    #[test]
    fn set_pending_style_inverse_roundtrip() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let step = Step::SetPendingStyle {
            old: None,
            new: Some(PendingStyle::Set {
                style_id: "s1".into(),
            }),
        };
        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;

        assert_eq!(state3.pending_style, state.pending_style);
    }
}
