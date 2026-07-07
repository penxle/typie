use editor_state::{BatchedState, PendingModifiers};

use crate::{Step, StepError};

pub(crate) fn inverse(old: PendingModifiers, new: PendingModifiers) -> Step {
    Step::SetPendingModifiers { old: new, new: old }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    new: &PendingModifiers,
) -> Result<(), StepError> {
    for entry in new {
        let ty = entry.as_type();
        if !ty.is_carry_kind() {
            return Err(StepError::InvalidPendingModifier { modifier_type: ty });
        }
    }
    batched.set_pending_modifiers(new.clone());
    Ok(())
}
