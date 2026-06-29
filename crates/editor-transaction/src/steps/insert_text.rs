use editor_crdt::Dot;
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(block: Dot, offset: usize, text: String) -> Step {
    Step::RemoveText {
        block,
        offset,
        text,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    block: Dot,
    offset: usize,
    text: &str,
) -> Result<(), StepError> {
    let ops = support::insert_text_ops(&batched.projected, block, offset, text)?;
    for op in ops {
        batched.apply(op)?;
    }
    Ok(())
}
