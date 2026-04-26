use editor_model::{Modifier, NodeId};
use editor_state::State;

use crate::transform::Conflict;
use crate::{Step, StepError, StepOutput, Validation};

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

pub(crate) fn transform_against(
    local_node_id: NodeId,
    local_old: &[Modifier],
    local_new: &[Modifier],
    against: &Step,
) -> Result<Vec<Step>, Conflict> {
    crate::transform::transform_default(
        Step::SetModifiers {
            node_id: local_node_id,
            old_modifiers: local_old.to_vec(),
            new_modifiers: local_new.to_vec(),
        },
        against,
    )
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::*;

    use crate::Transaction;
    use crate::*;

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
    fn transform_set_modifiers_against_set_modifiers_same_node_commutes() {
        let n = NodeId::new();
        let local = Step::SetModifiers {
            node_id: n,
            old_modifiers: vec![],
            new_modifiers: vec![editor_model::Modifier::Bold],
        };
        let against = Step::SetModifiers {
            node_id: n,
            old_modifiers: vec![],
            new_modifiers: vec![editor_model::Modifier::Italic],
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            vec![local.clone()],
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
