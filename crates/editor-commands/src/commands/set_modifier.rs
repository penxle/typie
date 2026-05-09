use editor_model::Modifier;
use editor_state::{PendingModifier, PendingModifiers, Position};
use editor_transaction::Transaction;

use crate::helpers::{
    collect_text_nodes_in_range, compact_and_restore_selection, resolve_inherited_modifiers,
};
use crate::{CommandError, CommandResult};

fn is_unit_variant(modifier: &Modifier) -> bool {
    matches!(
        modifier,
        Modifier::Bold | Modifier::Italic | Modifier::Underline | Modifier::Strikethrough
    )
}

pub fn set_modifier(tr: &mut Transaction, modifier: Modifier) -> CommandResult {
    if is_unit_variant(&modifier) {
        return Err(CommandError::InvalidArgument(format!(
            "{:?} is a unit modifier, use toggle_modifier instead",
            modifier.as_type()
        )));
    }

    let modifier_type = modifier.as_type();
    let selection = tr.selection();

    if selection.is_collapsed() {
        return set_modifier_collapsed(tr, &modifier);
    }

    let doc = tr.doc();
    let resolved = selection
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
    let from = Position::from(resolved.from());
    let to = Position::from(resolved.to());

    let node_ids = collect_text_nodes_in_range(tr, &from, &to)?;

    for &node_id in &node_ids {
        let doc = tr.doc();
        let node = doc
            .node(node_id)
            .ok_or(CommandError::NodeNotFound(node_id))?;

        let inherited = resolve_inherited_modifiers(&node);
        let inherited_value = inherited.iter().find(|m| m.as_type() == modifier_type);

        // Remove existing modifier of same type
        if let Some(existing) = node.modifiers().find(|m| m.as_type() == modifier_type) {
            tr.remove_modifier(node_id, existing.clone())?;
        }

        // Add new modifier only if different from inherited
        if inherited_value != Some(&modifier) {
            tr.add_modifier(node_id, modifier.clone())?;
        }
    }

    compact_and_restore_selection(tr, &node_ids)?;

    Ok(true)
}

fn set_modifier_collapsed(tr: &mut Transaction, modifier: &Modifier) -> CommandResult {
    let modifier_type = modifier.as_type();
    let pos = tr.selection().head;
    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let inherited = resolve_inherited_modifiers(&node);
    let inherited_value = inherited.iter().find(|m| m.as_type() == modifier_type);

    let mut pending: PendingModifiers = tr
        .pending_modifiers()
        .iter()
        .filter(|pm| pm.as_type() != modifier_type)
        .cloned()
        .collect();

    if inherited_value == Some(modifier) {
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
    fn collapsed_set_font_size() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 3)
            pending_modifiers: [font_size(2400)]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_same_as_inherited_unsets() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") [font_size(2400)] }
                }
            }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 1600 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") [font_size(2400)] }
                }
            }
            selection: (t1, 3)
            pending_modifiers: [!font_size]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_text_color() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::TextColor {
                value: "#ff0000".to_string()
            }
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 3)
            pending_modifiers: [text_color("#ff0000".to_string())]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_unit_variant_rejected() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let err = transact_err!(initial, |tr| set_modifier(&mut tr, Modifier::Italic));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn range_set_font_size() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("Hello")
                        t2: text("World")
                    }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("HelloWorld") [font_size(2400)]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_same_as_inherited_removes() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("Hello") [font_size(2400)]
                        t2: text("World") [font_size(2400)]
                    }
                }
            }
            selection: (t1, 0) -> (t2, 5)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 1600 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("HelloWorld")
                    }
                }
            }
            selection: (t1, 0) -> (t1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_replaces_existing() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("Hello") [font_size(2400)]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 3200 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("Hello") [font_size(3200)]
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_partial_selection_splits() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        t1: text("HelloWorld")
                    }
                }
            }
            selection: (t1, 2) -> (t1, 7)
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph {
                        text("He")
                        t1: text("lloWo") [font_size(2400)]
                        text("rld")
                    }
                }
            }
            selection: (t1, 0) -> (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_set_preserves_other_pending() {
        let (initial, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 3)
            pending_modifiers: [italic]
        };
        let (actual, ..) = transact!(initial, |tr| set_modifier(
            &mut tr,
            Modifier::FontSize { value: 2400 }
        ));
        let (expected, ..) = state! {
            doc {
                root [font_size(1600)] {
                    paragraph { t1: text("Hello") }
                }
            }
            selection: (t1, 3)
            pending_modifiers: [italic, font_size(2400)]
        };
        assert_state_eq!(&actual, &expected);
    }
}
