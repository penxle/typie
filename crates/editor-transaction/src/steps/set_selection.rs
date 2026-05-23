use editor_state::{BatchedState, StableSelection};

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(old: Option<StableSelection>, new: Option<StableSelection>) -> Step {
    Step::SetSelection { old: new, new: old }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    _validations: &mut Vec<Validation>,
    _old: Option<StableSelection>,
    new: Option<StableSelection>,
) -> Result<(), StepError> {
    let live = new.map(|s| s.thaw(&batched.doc));
    batched.set_selection(live);
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::{Position, Selection, StableSelection};

    use crate::Step;

    #[test]
    fn set_selection_apply() {
        let (s, t1) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let new_live = Selection::collapsed(Position::new(t1, 3));
        let step = Step::SetSelection {
            old: s
                .selection
                .as_ref()
                .map(|sel| StableSelection::freeze(sel, &s.doc)),
            new: Some(StableSelection::freeze(&new_live, &s.doc)),
        };
        let output = step.apply(&s).unwrap();

        assert_eq!(output.state.selection, Some(new_live));
    }

    #[test]
    fn set_selection_inverse_roundtrip() {
        let (s, t1) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };

        let original_live = s.selection.expect("fixture has selection");
        let new_live = Selection::collapsed(Position::new(t1, 3));
        let step = Step::SetSelection {
            old: Some(StableSelection::freeze(&original_live, &s.doc)),
            new: Some(StableSelection::freeze(&new_live, &s.doc)),
        };
        let s2 = step.apply(&s).unwrap().state;
        let s3 = step.inverse().apply(&s2).unwrap().state;

        assert_eq!(s3.selection, Some(original_live));
    }
}
