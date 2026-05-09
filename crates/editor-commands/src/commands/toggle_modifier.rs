use editor_model::{Modifier, ModifierType};
use editor_state::{PendingModifier, PendingModifiers, Position, resolve_effective_modifiers_at};
use editor_transaction::Transaction;

use crate::helpers::{
    check_range_all_has_modifier, collect_text_nodes_in_range, compact_and_restore_selection,
};
use crate::{CommandError, CommandResult};

fn modifier_from_unit_type(modifier_type: ModifierType) -> Result<Modifier, CommandError> {
    match modifier_type {
        ModifierType::Italic => Ok(Modifier::Italic),
        ModifierType::Underline => Ok(Modifier::Underline),
        ModifierType::Strikethrough => Ok(Modifier::Strikethrough),
        other => Err(CommandError::InvalidArgument(format!(
            "{other:?} is not a unit modifier type"
        ))),
    }
}

pub fn toggle_modifier(tr: &mut Transaction, modifier_type: ModifierType) -> CommandResult {
    let modifier = modifier_from_unit_type(modifier_type)?;
    let selection = tr.selection();

    if selection.is_collapsed() {
        return toggle_modifier_collapsed(tr, modifier_type, &modifier);
    }

    let doc = tr.doc();
    let resolved = selection
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
    let from = Position::from(resolved.from());
    let to = Position::from(resolved.to());

    let node_ids = collect_text_nodes_in_range(tr, &from, &to)?;

    let doc = tr.doc();
    let nodes: Vec<_> = node_ids.iter().filter_map(|id| doc.node(*id)).collect();
    let all_have = check_range_all_has_modifier(&nodes, modifier_type);

    if all_have {
        for &node_id in &node_ids {
            tr.remove_modifier(node_id, modifier.clone())?;
        }
    } else {
        for &node_id in &node_ids {
            let doc = tr.doc();
            let node = doc
                .node(node_id)
                .ok_or(CommandError::NodeNotFound(node_id))?;
            if !node.modifiers().any(|m| m.as_type() == modifier_type) {
                tr.add_modifier(node_id, modifier.clone())?;
            }
        }
    }

    compact_and_restore_selection(tr, &node_ids)?;

    Ok(true)
}

fn toggle_modifier_collapsed(
    tr: &mut Transaction,
    modifier_type: ModifierType,
    modifier: &Modifier,
) -> CommandResult {
    let pos = tr.selection().head;
    let doc = tr.doc();
    doc.node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;
    let effective = resolve_effective_modifiers_at(tr.state(), &pos);
    let has_modifier = effective.iter().any(|m| m.as_type() == modifier_type);

    let mut pending: PendingModifiers = tr
        .pending_modifiers()
        .iter()
        .filter(|pm| pm.as_type() != modifier_type)
        .cloned()
        .collect();

    if has_modifier {
        pending.push(PendingModifier::Unset { ty: modifier_type });
    } else {
        pending.push(PendingModifier::Set {
            modifier: modifier.clone(),
        });
    }

    tr.set_pending_modifiers(pending)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn collapsed_toggle_italic_on() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 3)
            pending_modifiers: [italic]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_toggle_italic_off() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [italic] } } }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [italic] } } }
            selection: (t1, 3)
            pending_modifiers: [!italic]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_toggle_underline_on() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(
            &mut tr,
            ModifierType::Underline
        ));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
            pending_modifiers: [underline]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_toggle_strikethrough_on() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(
            &mut tr,
            ModifierType::Strikethrough
        ));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
            pending_modifiers: [strikethrough]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_toggle_bold_rejected() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let err = transact_err!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Bold));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn collapsed_toggle_preserves_other_pending() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 3)
            pending_modifiers: [italic]
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(
            &mut tr,
            ModifierType::Underline
        ));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 3)
            pending_modifiers: [italic, underline]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_italic_on() {
        let (initial, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello")
                t2: text("World")
            } } }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root { paragraph {
                t1: text("HelloWorld") [italic]
            } } }
            selection: (t1, 0) -> (t1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_italic_off() {
        let (initial, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello") [italic]
                t2: text("World") [italic]
            } } }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root { paragraph {
                t1: text("HelloWorld")
            } } }
            selection: (t1, 0) -> (t1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_italic_mixed_turns_on() {
        let (initial, ..) = state! {
            doc { root { paragraph {
                t1: text("Hello") [italic]
                t2: text("World")
            } } }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root { paragraph {
                t1: text("HelloWorld") [italic]
            } } }
            selection: (t1, 0) -> (t1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_partial_selection_splits() {
        let (initial, ..) = state! {
            doc { root { paragraph {
                t1: text("HelloWorld")
            } } }
            selection: (t1, 2) -> (t1, 7)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root { paragraph {
                text("He")
                t1: text("lloWo") [italic]
                text("rld")
            } } }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }
}
