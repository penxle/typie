use editor_crdt::Dot;
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(block: Dot, offset: usize, text: String) -> Step {
    Step::InsertText {
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
    let len = text.chars().count();
    let dots = support::leaf_dots_in_range(&batched.projected, block, offset, len)?;
    let ops = support::delete_dots_ops(&batched.projected, &dots);
    for op in ops {
        batched.apply(op)?;
    }
    Ok(())
}
