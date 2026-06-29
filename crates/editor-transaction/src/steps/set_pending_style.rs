use editor_state::{BatchedState, PendingStyle};

use crate::{Step, StepError};

pub(crate) fn inverse(old: Option<PendingStyle>, new: Option<PendingStyle>) -> Step {
    Step::SetPendingStyle { old: new, new: old }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    new: &Option<PendingStyle>,
) -> Result<(), StepError> {
    batched.set_pending_style(new.clone());
    Ok(())
}
