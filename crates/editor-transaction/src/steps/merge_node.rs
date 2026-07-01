use editor_crdt::{Dot, ListOp};
use editor_model::EditOp;
use editor_state::BatchedState;

use crate::{Step, StepError};

pub(crate) fn inverse(block: Dot, offset: usize) -> Step {
    Step::SplitNode { block, offset }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    block: Dot,
    _offset: usize,
) -> Result<(), StepError> {
    let sib_dot = {
        let ps = &batched.projected;
        if !ps.is_block(block) {
            return Ok(());
        }
        let parent = ps
            .parent_of(block)
            .ok_or(StepError::MergeNoSibling { block })?;
        let siblings = ps.child_block_dots(parent);
        let pos = siblings
            .iter()
            .position(|id| *id == block)
            .ok_or(StepError::MergeNoSibling { block })?;
        let sib = siblings
            .get(pos + 1)
            .copied()
            .ok_or(StepError::MergeNoSibling { block })?;
        match sib.as_op_dot() {
            Some(d) => d.dot(),
            None => {
                return Err(StepError::MergeNoSibling { block });
            }
        }
    };
    let del_pos = batched
        .projected
        .seq_flat_pos(sib_dot)
        .ok_or(StepError::MergeNoSibling { block })?;
    batched.apply(EditOp::Seq(ListOp::Del {
        pos: del_pos,
        len: 1,
    }))?;
    Ok(())
}
