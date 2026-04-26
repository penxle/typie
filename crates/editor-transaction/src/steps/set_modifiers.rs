use editor_model::{Modifier, NodeId};
use editor_state::State;

use crate::{MapAction, Mapping, Step, StepError, StepOutput, Validation};

pub(crate) fn build_mapping() -> Mapping {
    Mapping::identity()
}

pub(crate) fn apply(
    state: &State,
    node_id: NodeId,
    new_modifiers: &[Modifier],
) -> Result<StepOutput, StepError> {
    let doc = state.doc.with_node_updated(node_id, |mut entry| {
        entry.modifiers = new_modifiers.to_vec();
        entry
    });

    let mut new_state = state.clone();
    new_state.doc = doc;

    let validations = new_modifiers
        .iter()
        .map(|m| Validation::Modifier(node_id, m.as_type()))
        .collect();

    Ok(StepOutput {
        state: new_state,
        mapping: build_mapping(),
        validations,
    })
}

pub(crate) fn inverse(
    node_id: NodeId,
    old_modifiers: Vec<Modifier>,
    new_modifiers: Vec<Modifier>,
) -> Step {
    Step::SetModifiers {
        node_id,
        old_modifiers: new_modifiers,
        new_modifiers: old_modifiers,
    }
}

pub(crate) fn rebase_against(
    node_id: NodeId,
    old_modifiers: &[Modifier],
    new_modifiers: &[Modifier],
    mapping: &Mapping,
) -> Vec<Step> {
    for action in mapping.actions() {
        if let MapAction::NodeDeleted { node } = *action {
            if node == node_id {
                return vec![];
            }
        }
    }
    vec![Step::SetModifiers {
        node_id,
        old_modifiers: old_modifiers.to_vec(),
        new_modifiers: new_modifiers.to_vec(),
    }]
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::Transaction;

    #[test]
    fn build_mapping_returns_identity() {
        assert_eq!(build_mapping(), Mapping::identity());
    }

    #[test]
    fn set_modifiers_apply() {
        let (state, _, t1) = state! {
            doc { root { p1: paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let modifiers = vec![Modifier::Bold];
        let step = Step::SetModifiers {
            node_id: t1,
            old_modifiers: vec![],
            new_modifiers: modifiers.clone(),
        };
        let output = step.apply(&state).unwrap();
        let new_state = output.state;

        assert_eq!(new_state.doc.get_entry(t1).unwrap().modifiers, modifiers);
    }

    #[test]
    fn set_modifiers_context_violation() {
        let (state, _, t1) = state! {
            doc { root { p1: paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        let result = tr.set_modifiers(t1, vec![Modifier::LineHeight { value: 160 }]);

        assert!(result.is_err());
    }

    #[test]
    fn rebase_swallowed_by_node_deleted() {
        let n = NodeId::new();
        let mapping = Mapping::single(MapAction::NodeDeleted { node: n });
        let result = rebase_against(n, &[], &[Modifier::Bold], &mapping);
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
        let result = rebase_against(n, &[], &[Modifier::Bold], &mapping);
        assert_eq!(
            result,
            vec![Step::SetModifiers {
                node_id: n,
                old_modifiers: vec![],
                new_modifiers: vec![Modifier::Bold],
            }]
        );
    }

    #[test]
    fn set_modifiers_inverse_roundtrip() {
        let (state, _, t1) = state! {
            doc { root { p1: paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let modifiers = vec![Modifier::Bold];
        let step = Step::SetModifiers {
            node_id: t1,
            old_modifiers: vec![],
            new_modifiers: modifiers,
        };

        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;

        assert_eq!(
            state3.doc.get_entry(t1).unwrap().modifiers,
            state.doc.get_entry(t1).unwrap().modifiers
        );
    }
}
