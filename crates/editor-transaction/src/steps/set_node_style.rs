use editor_crdt::Dot;
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(block: Dot, old: Option<String>, new: Option<String>) -> Step {
    Step::SetNodeStyle {
        block,
        old: new,
        new: old,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    block: Dot,
    new: Option<String>,
) -> Result<(), StepError> {
    let current = batched.projected.node_styles().value_of(block);
    if current == new {
        return Ok(());
    }
    batched.apply(support::node_style_set(block, new))?;
    Ok(())
}
