use editor_crdt::Dot;
use editor_model::Marker;
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(block: Dot, old: Option<Marker>, new: Option<Marker>) -> Step {
    Step::SetNodeMarker {
        block,
        old: new,
        new: old,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    block: Dot,
    new: Option<Marker>,
) -> Result<(), StepError> {
    let current = batched.projected.node_markers().value_of(block);
    if current == new {
        return Ok(());
    }
    batched.apply(support::node_marker_set(block, new))?;
    Ok(())
}
