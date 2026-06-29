use editor_state::BatchedState;
use editor_state::Selection;

use crate::{Step, StepError};

pub(crate) fn inverse(old: Option<Selection>, new: Option<Selection>) -> Step {
    Step::SetSelection { old: new, new: old }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    new: Option<Selection>,
) -> Result<(), StepError> {
    batched.set_selection(new);
    Ok(())
}
