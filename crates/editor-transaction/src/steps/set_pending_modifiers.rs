use editor_model::{NodeType, Schema};
use editor_state::{BatchedState, PendingModifiers};

use crate::{Step, StepError, Validation};

pub(crate) fn inverse(old: PendingModifiers, new: PendingModifiers) -> Step {
    Step::SetPendingModifiers { old: new, new: old }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    _validations: &mut Vec<Validation>,
    _old: &PendingModifiers,
    new: &PendingModifiers,
) -> Result<(), StepError> {
    for entry in new {
        let ty = entry.as_type();
        let targets = Schema::modifier_spec(ty).context.rightmost_node_types();
        if !targets.contains(&NodeType::Text) {
            return Err(StepError::InvalidPendingModifier { modifier_type: ty });
        }
    }
    batched.set_pending_modifiers(new.clone());
    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Modifier;
    use editor_state::PendingModifier;

    use crate::Step;

    #[test]
    fn set_pending_modifiers_apply() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let modifiers = vec![PendingModifier::Set {
            modifier: Modifier::Bold,
        }];
        let step = Step::SetPendingModifiers {
            old: vec![],
            new: modifiers.clone(),
        };
        let output = step.apply(&state).unwrap();

        assert_eq!(output.state.pending_modifiers, modifiers);
    }

    #[test]
    fn set_pending_modifiers_inverse_roundtrip() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let modifiers = vec![PendingModifier::Set {
            modifier: Modifier::Bold,
        }];
        let step = Step::SetPendingModifiers {
            old: vec![],
            new: modifiers,
        };
        let state2 = step.apply(&state).unwrap().state;
        let state3 = step.inverse().apply(&state2).unwrap().state;

        assert_eq!(state3.pending_modifiers, state.pending_modifiers);
    }

    #[test]
    fn rejects_non_text_applicable_pending_entry() {
        use editor_model::ModifierType;

        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let modifiers = vec![PendingModifier::Set {
            modifier: Modifier::LineHeight { value: 160 },
        }];
        let step = Step::SetPendingModifiers {
            old: vec![],
            new: modifiers,
        };
        let result = step.apply(&state);
        assert!(matches!(
            result,
            Err(crate::StepError::InvalidPendingModifier {
                modifier_type: ModifierType::LineHeight
            })
        ));
    }

    #[test]
    fn accepts_text_applicable_pending_entries() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };

        let modifiers = vec![
            PendingModifier::Set {
                modifier: Modifier::Bold,
            },
            PendingModifier::Set {
                modifier: Modifier::FontSize { value: 1600 },
            },
        ];
        let step = Step::SetPendingModifiers {
            old: vec![],
            new: modifiers.clone(),
        };
        assert!(step.apply(&state).is_ok());
    }
}
