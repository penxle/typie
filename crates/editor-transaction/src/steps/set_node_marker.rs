use editor_crdt::LwwRegOp;
use editor_model::{DocOp, Marker, NodeId};
use editor_state::BatchedState;

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(node_id: NodeId, old: Option<Marker>, new: Option<Marker>) -> Step {
    Step::SetNodeMarker {
        node_id,
        old: new,
        new: old,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    _validations: &mut Vec<Validation>,
    node_id: NodeId,
    new: Option<Marker>,
) -> Result<(), StepError> {
    let entry = batched
        .doc
        .get_entry(node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;
    let current = entry.marker.get().clone();
    if current == new {
        return Ok(());
    }
    batched.apply(DocOp::NodeMarker {
        node_id,
        op: LwwRegOp::Set { value: new },
    })?;
    Ok(())
}
