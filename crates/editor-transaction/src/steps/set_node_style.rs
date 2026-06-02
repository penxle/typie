use editor_crdt::LwwRegOp;
use editor_model::{DocOp, NodeId};
use editor_state::BatchedState;

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(node_id: NodeId, old: Option<String>, new: Option<String>) -> Step {
    Step::SetNodeStyle {
        node_id,
        old: new,
        new: old,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    _validations: &mut Vec<Validation>,
    node_id: NodeId,
    new: Option<String>,
) -> Result<(), StepError> {
    let entry = batched
        .doc
        .get_entry(node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;
    let current = entry.style.get().clone();
    if current == new {
        return Ok(());
    }
    batched.apply(DocOp::NodeStyle {
        node_id,
        op: LwwRegOp::Set { value: new },
    })?;
    Ok(())
}
