use editor_state::{Selection, State};

use crate::{Mapping, Step, StepError, StepOutput};

pub(crate) fn apply(state: &State, new: &Selection) -> Result<StepOutput, StepError> {
    let mut new_state = state.clone();
    new_state.selection = *new;

    Ok(StepOutput {
        state: new_state,
        mapping: Mapping::identity(),
        validations: vec![],
    })
}

pub(crate) fn inverse(old: Selection, new: Selection) -> Step {
    Step::SetSelection { old: new, new: old }
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::*;

    use crate::*;

    #[test]
    fn set_selection_apply() {
        let (state, t1) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let new_sel = Selection::collapsed(Position::new(t1, 3));
        let step = Step::SetSelection {
            old: state.selection,
            new: new_sel,
        };

        let output = step.apply(&state).unwrap();

        assert_eq!(output.state.selection, new_sel);
    }

    #[test]
    fn set_selection_inverse_roundtrip() {
        let (state, t1) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let new_sel = Selection::collapsed(Position::new(t1, 3));
        let step = Step::SetSelection {
            old: state.selection,
            new: new_sel,
        };

        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;

        assert_eq!(state3.selection, state.selection);
    }
}
