use editor_state::{BatchedState, Selection};

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(old: Selection, new: Selection) -> Step {
    Step::SetSelection { old: new, new: old }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    _validations: &mut Vec<Validation>,
    _old: Selection,
    new: Selection,
) -> Result<(), StepError> {
    batched.set_selection(new);
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::{Position, Selection};

    use crate::Step;

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
