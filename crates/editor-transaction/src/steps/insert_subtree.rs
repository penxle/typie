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

/// Inserts `subtree` and returns the dot of its root — `None` only for a
/// top-level `Text` subtree (character-per-dot; a block/atom root always
/// yields `Some`), so a caller that needs a fresh parent to insert further
/// content beneath can use the returned dot without re-reading the tree.
pub(crate) fn apply_to(
    batched: &mut BatchedState,
    parent: Dot,
    index: usize,
    subtree: &Subtree,
) -> Result<Option<Dot>, StepError> {
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
    let host = support::parent_host_type(&batched.projected, &parents);
    let mut seq_pos = pos;
    support::emit_subtree(
        batched,
        subtree,
        &parents,
        host,
        &mut seq_pos,
        &mut Vec::new(),
    )
}
