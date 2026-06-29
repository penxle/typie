use editor_state::{BatchedState, Composition};

use crate::{Step, StepError};

pub(crate) fn inverse(old: Option<Composition>, new: Option<Composition>) -> Step {
    Step::SetComposition { old: new, new: old }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    new: Option<Composition>,
) -> Result<(), StepError> {
    batched.set_composition(new);
    Ok(())
}
