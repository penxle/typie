use editor_crdt::Dot;
use editor_model::Modifier;
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(block: Dot, modifier: Modifier) -> Step {
    Step::AddModifier { block, modifier }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    block: Dot,
    modifier: &Modifier,
) -> Result<(), StepError> {
    let key = modifier.as_type();
    let present = batched
        .projected
        .block_modifiers()
        .modifiers_of(block)
        .contains_key(&key);
    if !present {
        return Ok(());
    }
    batched.apply(support::block_modifier_clear(block, key))?;
    Ok(())
}
