use editor_crdt::Dot;
use editor_model::{EditOp, NodeAttrOp, PlainNode};
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(block: Dot, old_node: PlainNode, new_node: PlainNode) -> Step {
    Step::SetNode {
        block,
        old_node: new_node,
        new_node: old_node,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    block: Dot,
    _old_node: &PlainNode,
    new_node: &PlainNode,
) -> Result<(), StepError> {
    if support::block_node_type(&batched.projected, block).is_none() {
        return Err(StepError::NodeNotFound(block));
    }
    let Some(dot) = block.as_op_dot() else {
        return Err(StepError::NodeNotFound(block));
    };
    let dot = dot.dot();
    for attr in new_node.to_attrs() {
        batched.apply(EditOp::NodeAttr(NodeAttrOp { target: dot, attr }))?;
    }
    Ok(())
}
