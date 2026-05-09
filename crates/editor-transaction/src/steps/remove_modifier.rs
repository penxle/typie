use editor_crdt::{Dot, OrMapOp};
use editor_model::{DocOp, Modifier, NodeId};
use editor_state::BatchedState;

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(node_id: NodeId, modifier: Modifier) -> Step {
    Step::AddModifier { node_id, modifier }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    _validations: &mut Vec<Validation>,
    node_id: NodeId,
    modifier: &Modifier,
) -> Result<(), StepError> {
    let key = modifier.as_type();
    let mut observed: Vec<Dot> = {
        let entry = batched
            .doc
            .get_entry(node_id)
            .ok_or(StepError::NodeNotFound(node_id))?;
        entry.modifiers.tags_for(&key).copied().collect()
    };
    if observed.is_empty() {
        return Ok(());
    }
    observed.sort_unstable();
    observed.dedup();
    batched.apply(DocOp::Modifier {
        node_id,
        op: OrMapOp::Unset { observed },
    })?;
    Ok(())
}
