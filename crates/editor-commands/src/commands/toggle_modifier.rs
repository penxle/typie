use editor_model::{Modifier, ModifierType};
use editor_state::{PendingModifier, PendingModifiers, Position, resolve_effective_modifiers_at};
use editor_transaction::Transaction;

use crate::helpers::{
    check_range_all_has_modifier, collect_text_nodes_in_range, compact_and_restore_selection,
    filter_applicable_node_ids,
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
    let applicable_node_ids = filter_applicable_node_ids(&tr.doc(), &node_ids, modifier_type);

    if applicable_node_ids.is_empty() {
        return Ok(false);
    }

    let doc = tr.doc();
    let nodes: Vec<_> = applicable_node_ids
        .iter()
        .filter_map(|id| doc.node(*id))
        .collect();
    let all_have = check_range_all_has_modifier(&nodes, modifier_type);

    if all_have {
        for &node_id in &applicable_node_ids {
            tr.remove_modifier(node_id, modifier.clone())?;
        }
    } else {
        for &node_id in &applicable_node_ids {
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
    fn range_toggle_italic_skips_fold_title_applies_to_paragraph() {
        let (initial, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("Title") }
                    fold_content { paragraph { t2: text("Body") } }
                }
            } }
            selection: (t1, 0) -> (t2, 4)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("Title") }
                    fold_content { paragraph { t2: text("Body") [italic] } }
                }
            } }
            selection: (t1, 0) -> (t2, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_underline_skips_fold_title_applies_to_paragraph() {
        let (initial, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("Title") }
                    fold_content { paragraph { t2: text("Body") } }
                }
            } }
            selection: (t1, 0) -> (t2, 4)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(
            &mut tr,
            ModifierType::Underline
        ));
        let (expected, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("Title") }
                    fold_content { paragraph { t2: text("Body") [underline] } }
                }
            } }
            selection: (t1, 0) -> (t2, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_strikethrough_skips_fold_title_applies_to_paragraph() {
        let (initial, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("Title") }
                    fold_content { paragraph { t2: text("Body") } }
                }
            } }
            selection: (t1, 0) -> (t2, 4)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(
            &mut tr,
            ModifierType::Strikethrough
        ));
        let (expected, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("Title") }
                    fold_content { paragraph { t2: text("Body") [strikethrough] } }
                }
            } }
            selection: (t1, 0) -> (t2, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_italic_on_fold_title_only_is_noop() {
        let (initial, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("Title") }
                    fold_content { paragraph { text("Body") } }
                }
            } }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) =
            transact_fail!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root {
                fold {
                    fold_title { t1: text("Title") }
                    fold_content { paragraph { text("Body") } }
                }
            } }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_italic_cell_bracket_across_rows_applies_to_all_cells() {
        let (initial, tr1, tr2, ..) = state! {
            doc {
                root {
                    table {
                        tr1: table_row {
                            table_cell { paragraph { text("1") } }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                        }
                        table_row {
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                        }
                        tr2: table_row {
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph { text("1234") } }
                            table_cell { paragraph {} }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (tr1, 0, >) -> (tr2, 4, <)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));

        assert_eq!(actual.selection.anchor.node_id, tr1);
        assert_eq!(actual.selection.anchor.offset, 0);
        assert_eq!(actual.selection.head.node_id, tr2);
        assert_eq!(actual.selection.head.offset, 4);

        let mut italic_texts = Vec::new();
        for desc in actual.doc.root().unwrap().descendants() {
            if let editor_model::Node::Text(t) = desc.node()
                && desc
                    .modifiers()
                    .any(|m| m.as_type() == ModifierType::Italic)
            {
                italic_texts.push(t.text.to_string());
            }
        }
        italic_texts.sort();
        assert_eq!(italic_texts, vec!["1", "1234"]);
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
