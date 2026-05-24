use editor_crdt::OrMapOp;
use editor_model::{DocOp, Modifier, NodeId};
use editor_state::BatchedState;

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(node_id: NodeId, modifier: Modifier) -> Step {
    Step::RemoveModifier { node_id, modifier }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    validations: &mut Vec<Validation>,
    node_id: NodeId,
    modifier: &Modifier,
) -> Result<(), StepError> {
    batched.apply(DocOp::Modifier {
        node_id,
        op: OrMapOp::Set {
            key: modifier.as_type(),
            value: modifier.clone(),
        },
    })?;
    validations.push(Validation::Modifier(node_id, modifier.as_type()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::{Modifier, ModifierType};

    use crate::{Step, Transaction};

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
        let new_state = step.apply(&state).unwrap().state;
        let entry = new_state.doc.get_entry(t1).unwrap();

        assert!(entry.modifiers.contains_key(&ModifierType::Bold));
        assert_eq!(entry.modifiers.len(), 1);
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

        assert!(entry.modifiers.contains_key(&ModifierType::Bold));
        assert_eq!(entry.modifiers.len(), 1);
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
        assert!(tr.add_modifier(t1, Modifier::Bold).is_err());
    }

    #[test]
    fn add_bold_to_paragraph_is_valid() {
        let (state, p1) = state! {
            doc {
                root {
                    p1: paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        assert!(tr.add_modifier(p1, Modifier::Bold).is_ok());
    }

    #[test]
    fn add_font_family_to_paragraph_is_valid() {
        let (state, p1) = state! {
            doc {
                root {
                    p1: paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let mut tr = Transaction::new(&state);
        assert!(
            tr.add_modifier(
                p1,
                Modifier::FontFamily {
                    value: "Arial".to_string()
                }
            )
            .is_ok()
        );
    }
}
