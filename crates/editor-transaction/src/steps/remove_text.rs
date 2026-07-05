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
    let dots = support::char_leaf_dots_for_text(&batched.projected, block, offset, text)?;
    let ops = support::delete_dots_ops(&batched.projected, &dots);
    for op in ops {
        batched.apply(op)?;
    }
    Ok(())
}
