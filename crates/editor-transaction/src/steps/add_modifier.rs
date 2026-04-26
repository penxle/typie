use editor_model::{Modifier, NodeId};
use editor_state::State;

use crate::{MapAction, Mapping, Step, StepError, StepOutput, Validation};

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
        mapping: build_mapping(),
        validations: vec![Validation::Modifier(node_id, modifier.as_type())],
    })
}

pub(crate) fn inverse(node_id: NodeId, modifier: Modifier) -> Step {
    Step::RemoveModifier { node_id, modifier }
}

pub(crate) fn rebase_against(node_id: NodeId, modifier: &Modifier, mapping: &Mapping) -> Vec<Step> {
    for action in mapping.actions() {
        if let MapAction::NodeDeleted { node } = *action {
            if node == node_id {
                return vec![];
            }
        }
    }
    vec![Step::AddModifier {
        node_id,
        modifier: modifier.clone(),
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
            vec![Step::AddModifier {
                node_id: n,
                modifier: Modifier::Bold,
            }]
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
