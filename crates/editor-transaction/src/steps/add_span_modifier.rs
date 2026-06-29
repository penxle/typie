use editor_crdt::Dot;
use editor_model::Modifier;
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(first: Dot, last: Dot, modifier: Modifier) -> Step {
    Step::RemoveSpanModifier {
        first,
        last,
        modifier,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    first: Dot,
    last: Dot,
    modifier: &Modifier,
) -> Result<(), StepError> {
    batched.apply(support::span_add(first, last, modifier.clone()))?;
    Ok(())
}
