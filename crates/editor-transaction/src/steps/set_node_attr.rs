use editor_crdt::Dot;
use editor_model::{EditOp, NodeAttr, NodeAttrOp};
use editor_state::BatchedState;

use crate::{Step, StepError};

pub(crate) fn inverse(block: Dot, old: NodeAttr, new: NodeAttr) -> Step {
    Step::SetNodeAttr {
        block,
        old: new,
        new: old,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    block: Dot,
    old: &NodeAttr,
    new: &NodeAttr,
) -> Result<(), StepError> {
    let node = batched
        .projected
        .block_node(block)
        .or_else(|| batched.projected.atom_leaf_node(block))
        .ok_or(StepError::NodeNotFound(block))?;
    if !old.same_field(new) {
        return Err(StepError::NodeAttrFieldMismatch { block });
    }
    if !node
        .to_plain()
        .to_attrs()
        .iter()
        .any(|attr| attr.same_field(new))
    {
        return Err(StepError::NodeAttrKindMismatch { block });
    }
    let Some(target) = editor_model::anchor_dot(block) else {
        return Err(StepError::NodeNotFound(block));
    };
    batched.apply(EditOp::NodeAttr(NodeAttrOp {
        target,
        attr: new.clone(),
    }))?;
    Ok(())
}
