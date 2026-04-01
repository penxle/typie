use editor_model::{Modifier, NodeId};
use editor_state::State;

use crate::{Step, StepError, StepOutput};

pub(crate) fn apply(
    state: &State,
    node_id: NodeId,
    modifier: &Modifier,
) -> Result<StepOutput, StepError> {
    state
        .doc
        .get_entry(node_id)
        .ok_or(StepError::NodeNotFound(node_id))?;

    let doc = state.doc.with_node_updated(node_id, |mut entry| {
        entry.modifiers.retain(|m| m != modifier);
        entry
    });

    let mut new_state = state.clone();
    new_state.doc = doc;

    Ok(StepOutput {
        state: new_state,
        validations: vec![],
    })
}

pub(crate) fn inverse(node_id: NodeId, modifier: Modifier) -> Step {
    Step::AddModifier { node_id, modifier }
}
