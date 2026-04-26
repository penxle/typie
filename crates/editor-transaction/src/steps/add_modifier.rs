use editor_model::{Modifier, NodeId};
use editor_state::State;

use crate::transform::Conflict;
use crate::{Step, StepError, StepOutput, Validation};

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
        if !entry.modifiers.contains(modifier) {
            entry.modifiers.push(modifier.clone());
            entry.modifiers.sort_by_key(|m| m.as_type());
        }
        entry
    });

    let mut new_state = state.clone();
    new_state.doc = doc;

    Ok(StepOutput {
        state: new_state,
        validations: vec![Validation::Modifier(node_id, modifier.as_type())],
    })
}

pub(crate) fn inverse(node_id: NodeId, modifier: Modifier) -> Step {
    Step::RemoveModifier { node_id, modifier }
}

pub(crate) fn transform_against(
    local_node_id: NodeId,
    local_modifier: &Modifier,
    against: &Step,
) -> Result<Vec<Step>, Conflict> {
    crate::transform::transform_default(
        Step::AddModifier {
            node_id: local_node_id,
            modifier: local_modifier.clone(),
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
    fn add_modifier_apply() {
        let (state, t1) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello World")
                    }
                }
            }
            selection: (t1, 0)
        };

        let step = Step::AddModifier {
            node_id: t1,
            modifier: Modifier::Bold,
        };

        let output = step.apply(&state).unwrap();
        let new_state = output.state;
        let entry = new_state.doc.get_entry(t1).unwrap();

        assert_eq!(entry.modifiers, vec![Modifier::Bold]);
    }

    #[test]
    fn add_modifier_idempotent() {
        let (state, t1) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello World")
                    }
                }
            }
            selection: (t1, 0)
        };

        let step = Step::AddModifier {
            node_id: t1,
            modifier: Modifier::Bold,
        };

        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.apply(&state2).unwrap().state;
        let entry = state3.doc.get_entry(t1).unwrap();

        assert_eq!(entry.modifiers, vec![Modifier::Bold]);
    }

    #[test]
    fn add_bold_in_fold_title_context_violation() {
        let (state, t1) = state! {
            doc {
                root {
                    fold {
                        fold_title {
                            t1: text("Title")
                        }
                        fold_content {
                            paragraph
                        }
                    }
                }
            }
            selection: (t1, 0)
        };

        let mut tr = Transaction::new(&state);
        let result = tr.add_modifier(t1, Modifier::Bold);

        assert!(result.is_err());
    }

    #[test]
    fn transform_add_modifier_against_add_modifier_same_node_commutes() {
        let n = NodeId::new();
        let local = Step::AddModifier {
            node_id: n,
            modifier: editor_model::Modifier::Bold,
        };
        let against = Step::AddModifier {
            node_id: n,
            modifier: editor_model::Modifier::Italic,
        };
        assert_eq!(
            crate::transform::transform(&local, &against).unwrap(),
            vec![local.clone()],
        );
    }

    #[test]
    fn add_then_remove_modifier_roundtrip() {
        let (state, t1) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello World")
                    }
                }
            }
            selection: (t1, 0)
        };

        let step = Step::AddModifier {
            node_id: t1,
            modifier: Modifier::Bold,
        };

        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;
        let entry = state3.doc.get_entry(t1).unwrap();

        assert!(entry.modifiers.is_empty());
    }
}
