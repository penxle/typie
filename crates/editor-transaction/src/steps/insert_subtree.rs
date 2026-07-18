use editor_crdt::Dot;
use editor_model::{NodeType, PlainNode, Subtree};
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(parent: Dot, index: usize, subtree: Subtree) -> Step {
    Step::RemoveSubtree {
        parent,
        index,
        subtree,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    parent: Dot,
    index: usize,
    subtree: &Subtree,
) -> Result<(), StepError> {
    // A fundamentally un-insertable subtree root keeps its specific error, ahead
    // of the slot guard's generic `IllegalInsertSlot`.
    if subtree.node == PlainNode::Unknown {
        return Err(StepError::UnknownSubtree);
    }
    if subtree.node.as_type() == NodeType::Root {
        return Err(StepError::RootSubtree);
    }
    let pos =
        support::child_seq_insert_pos(&batched.projected, parent, index, subtree.node.as_type())?;
    let parents = support::self_inclusive_parents(&batched.projected, parent)
        .ok_or(StepError::NodeNotFound(parent))?;
    let mut seq_pos = pos;
    support::emit_subtree(batched, subtree, &parents, &mut seq_pos, &mut Vec::new())?;
    Ok(())
}
