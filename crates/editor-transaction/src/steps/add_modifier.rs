use editor_crdt::Dot;
use editor_model::Modifier;
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(block: Dot, modifier: Modifier) -> Step {
    Step::RemoveModifier { block, modifier }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    block: Dot,
    modifier: &Modifier,
) -> Result<(), StepError> {
    // Block modifiers target any node dot, including the implicit root
    // (Dot::ROOT, a permanent synthetic anchor) — not just real op dots.
    batched.apply(support::block_modifier_set(block, modifier.clone()))?;
    Ok(())
}
