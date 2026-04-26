use editor_model::{Modifier, NodeId};
use editor_state::State;

use crate::transform::Conflict;
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

pub(crate) fn transform_against(
    local_node_id: NodeId,
    local_modifier: &Modifier,
    against: &Step,
) -> Result<Vec<Step>, Conflict> {
    crate::transform::transform_default(
        Step::RemoveModifier {
            node_id: local_node_id,
            modifier: local_modifier.clone(),
        },
        against,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_remove_modifier_against_remove_modifier_same_node_commutes() {
        let n = NodeId::new();
        let local = Step::RemoveModifier {
            node_id: n,
            modifier: editor_model::Modifier::Bold,
        };
        let against = Step::RemoveModifier {
            node_id: n,
            modifier: editor_model::Modifier::Italic,
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            vec![local.clone()],
        );
    }
}
