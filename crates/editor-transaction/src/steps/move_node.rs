use editor_crdt::Dot;
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(
    block: Dot,
    old_parent: Dot,
    old_index: usize,
    new_parent: Dot,
    new_index: usize,
) -> Step {
    Step::MoveNode {
        block,
        old_parent: new_parent,
        old_index: new_index,
        new_parent: old_parent,
        new_index: old_index,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    block: Dot,
    _old_parent: Dot,
    _old_index: usize,
    new_parent: Dot,
    new_index: usize,
) -> Result<(), StepError> {
    let subtree = support::capture_subtree(&batched.projected, block)
        .ok_or(StepError::NodeNotFound(block))?;

    let del_ops = {
        let ps = &batched.projected;
        let dots = support::subtree_dots(ps, block).ok_or(StepError::NodeNotFound(block))?;
        support::delete_dots_ops(ps, &dots)
    };
    for op in del_ops {
        batched.apply(op)?;
    }

    let pos = support::child_seq_insert_pos(&batched.projected, new_parent, new_index)?;
    let parents = support::self_inclusive_parents(&batched.projected, new_parent)
        .ok_or(StepError::NodeNotFound(new_parent))?;
    let mut seq_pos = pos;
    support::emit_subtree(batched, &subtree, &parents, &mut seq_pos)?;
    Ok(())
}
