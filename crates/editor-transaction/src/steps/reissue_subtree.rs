use editor_crdt::Dot;
use editor_model::{AliasOp, EditOp, NodeType, PlainNode, Subtree};
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
    let mut pairs: Vec<(Dot, Dot)> = Vec::new();
    support::emit_subtree(batched, subtree, &parents, &mut seq_pos, &mut pairs)?;
    if !pairs.is_empty() {
        batched.apply(EditOp::Alias(AliasOp {
            pairs: support::compress_alias_pairs(&pairs),
        }))?;
    }
    Ok(())
}
