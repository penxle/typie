use editor_model::{Modifier, NodeId};
use editor_state::State;

use crate::{MapAction, Mapping, Step, StepError, StepOutput};

pub(crate) fn build_mapping() -> Mapping {
    Mapping::identity()
}

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
        mapping: build_mapping(),
        validations: vec![],
    })
}

pub(crate) fn inverse(node_id: NodeId, modifier: Modifier) -> Step {
    Step::AddModifier { node_id, modifier }
}

pub(crate) fn rebase_against(node_id: NodeId, modifier: &Modifier, mapping: &Mapping) -> Vec<Step> {
    for action in mapping.actions() {
        if let MapAction::NodeDeleted { node } = *action {
            if node == node_id {
                return vec![];
            }
        }
    }
    vec![Step::RemoveModifier {
        node_id,
        modifier: modifier.clone(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_mapping_returns_identity() {
        assert_eq!(build_mapping(), Mapping::identity());
    }

    #[test]
    fn rebase_swallowed_by_node_deleted() {
        let n = NodeId::new();
        let mapping = Mapping::single(MapAction::NodeDeleted { node: n });
        let result = rebase_against(n, &Modifier::Bold, &mapping);
        assert!(result.is_empty());
    }

    #[test]
    fn rebase_unrelated_pass_through() {
        let n = NodeId::new();
        let other = NodeId::new();
        let mapping = Mapping::single(MapAction::TextInsert {
            node: other,
            offset: 0,
            len: 1,
            text: "x".into(),
        });
        let result = rebase_against(n, &Modifier::Bold, &mapping);
        assert_eq!(
            result,
            vec![Step::RemoveModifier {
                node_id: n,
                modifier: Modifier::Bold,
            }]
        );
    }
}
