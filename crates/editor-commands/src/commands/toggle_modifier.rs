use editor_model::{Modifier, ModifierType};
use editor_state::resolve_modifier_state;
use editor_state::{PendingModifier, PendingModifiers};
use editor_transaction::Transaction;

use crate::helpers::{modifier_from_unit_type, range_has_modifier, toggle_modifier_range};
use crate::{CommandError, CommandResult};

pub fn toggle_modifier(tr: &mut Transaction, modifier_type: ModifierType) -> CommandResult {
    let modifier = modifier_from_unit_type(modifier_type)?;
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    if selection.anchor == selection.head {
        return toggle_modifier_collapsed(tr, modifier_type, &modifier);
    }

    toggle_modifier_range(tr, selection, modifier_type)
}

fn toggle_modifier_collapsed(
    tr: &mut Transaction,
    modifier_type: ModifierType,
    modifier: &Modifier,
) -> CommandResult {
    let selection = tr.selection().expect("entry caller guaranteed selection");
    let pos = selection.head;
    {
        let view = tr.view();
        view.node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;
    }

    let has_modifier = {
        let ms = resolve_modifier_state(&tr.state().projected, &selection, tr.pending_modifiers())
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        range_has_modifier(&ms, modifier_type)
    };

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
    fn toggle_modifier_returns_false_when_no_selection() {
        let (initial, ..) = state! {
            doc { root { paragraph { text("Hello") } } }
            selection: none
        };
        let mut tr = editor_transaction::Transaction::new(&initial);
        let result = toggle_modifier(&mut tr, ModifierType::Italic);
        assert!(matches!(result, Ok(false)));
    }

    #[test]
    fn collapsed_toggle_italic_on() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
            pending_modifiers: [italic]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_toggle_italic_off() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [italic] } } }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [italic] } } }
            selection: (p1, 3)
            pending_modifiers: [!italic]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_toggle_underline_on() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(
            &mut tr,
            ModifierType::Underline
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
            pending_modifiers: [underline]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_toggle_strikethrough_on() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(
            &mut tr,
            ModifierType::Strikethrough
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
            pending_modifiers: [strikethrough]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_toggle_bold_rejected() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let err = transact_err!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Bold));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn collapsed_toggle_preserves_other_pending() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
            pending_modifiers: [italic]
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(
            &mut tr,
            ModifierType::Underline
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
            pending_modifiers: [italic, underline]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_italic_on() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("HelloWorld") } } }
            selection: (p1, 0) -> (p1, 10)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root { p1: paragraph carry([italic]) { text("HelloWorld") [italic] } } }
            selection: (p1, 0) -> (p1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_italic_off() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph carry([italic]) { text("HelloWorld") [italic] } } }
            selection: (p1, 0) -> (p1, 10)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("HelloWorld") } } }
            selection: (p1, 0) -> (p1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_italic_mixed_turns_on() {
        let (initial, ..) = state! {
            doc { root { p: paragraph {
                text("Hello") [italic]
                text("World")
            } } }
            selection: (p, 0) -> (p, 10)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root { p: paragraph carry([italic]) { text("HelloWorld") [italic] } } }
            selection: (p, 0) -> (p, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn block_unit_selection_toggle_italic_on() {
        let (initial, ..) = state! {
            doc { r1: root { p1: paragraph { text("Hello") } } }
            selection: (r1, 0, >) -> (r1, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { r1: root { p1: paragraph carry([italic]) { text("Hello") [italic] } } }
            selection: (r1, 0, >) -> (r1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn block_unit_selection_toggle_italic_off() {
        let (initial, ..) = state! {
            doc { r1: root { p1: paragraph carry([italic]) { text("Hello") [italic] } } }
            selection: (r1, 0, >) -> (r1, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { r1: root { p1: paragraph { text("Hello") } } }
            selection: (r1, 0, >) -> (r1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_partial_selection_applies_span_to_substring() {
        let (initial, ..) = state! {
            doc { root { p: paragraph { text("HelloWorld") } } }
            selection: (p, 2) -> (p, 7)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root { p: paragraph {
                text("He")
                text("lloWo") [italic]
                text("rld")
            } } }
            selection: (p, 2) -> (p, 7)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn cell_rect_toggle_italic_applies_to_rect_cells_only() {
        let (initial, ..) = state! {
            doc { root {
                table {
                    tr1: table_row {
                        table_cell { paragraph { text("1") } }
                        table_cell { paragraph { text("2") } }
                        table_cell { paragraph { text("3") } }
                    }
                    table_row {
                        table_cell { paragraph { text("4") } }
                        table_cell { paragraph { text("5") } }
                        table_cell { paragraph { text("6") } }
                    }
                    table_row {
                        table_cell { paragraph { text("7") } }
                        table_cell { paragraph { text("8") } }
                        table_cell { paragraph { text("9") } }
                    }
                    tr2: table_row {
                        table_cell { paragraph { text("10") } }
                        table_cell { paragraph { text("11") } }
                        table_cell { paragraph { text("12") } }
                    }
                }
                paragraph {}
            } }
            selection: (tr1, 0, >) -> (tr2, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root {
                table {
                    tr1: table_row {
                        table_cell { paragraph carry([italic]) { text("1") [italic] } }
                        table_cell { paragraph { text("2") } }
                        table_cell { paragraph { text("3") } }
                    }
                    table_row {
                        table_cell { paragraph carry([italic]) { text("4") [italic] } }
                        table_cell { paragraph { text("5") } }
                        table_cell { paragraph { text("6") } }
                    }
                    table_row {
                        table_cell { paragraph carry([italic]) { text("7") [italic] } }
                        table_cell { paragraph { text("8") } }
                        table_cell { paragraph { text("9") } }
                    }
                    tr2: table_row {
                        table_cell { paragraph carry([italic]) { text("10") [italic] } }
                        table_cell { paragraph { text("11") } }
                        table_cell { paragraph { text("12") } }
                    }
                }
                paragraph {}
            } }
            selection: (tr1, 0, >) -> (tr2, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn cell_rect_toggle_italic_twice_roundtrips() {
        let (initial, ..) = state! {
            doc { root {
                table {
                    tr1: table_row {
                        table_cell { paragraph { text("1") } }
                        table_cell { paragraph { text("2") } }
                    }
                    tr2: table_row {
                        table_cell { paragraph { text("3") } }
                        table_cell { paragraph { text("4") } }
                    }
                }
            } }
            selection: (tr1, 0, >) -> (tr2, 1, <)
        };
        let (once, ..) = transact!(initial.clone(), |tr| toggle_modifier(
            &mut tr,
            ModifierType::Italic
        ));
        let (twice, ..) = transact!(once, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        // The second toggle must read the rect (uniform italic) and remove it,
        // not see the unselected cells as mixed and add italic again.
        assert_state_eq!(&twice, &initial);
    }

    #[test]
    fn cell_rect_covers_rect_columns_outside_flat_path_range() {
        // Column 1 via offsets (2, >) -> (1, <): tr1's column-1 cell precedes
        // the flat from-path (tr1, 2), so only the rect interpretation covers it.
        let (initial, ..) = state! {
            doc { root {
                table {
                    tr1: table_row {
                        table_cell { paragraph { text("1") } }
                        table_cell { paragraph { text("2") } }
                        table_cell { paragraph { text("3") } }
                    }
                    tr2: table_row {
                        table_cell { paragraph { text("4") } }
                        table_cell { paragraph { text("5") } }
                        table_cell { paragraph { text("6") } }
                    }
                }
            } }
            selection: (tr1, 2, >) -> (tr2, 1, <)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let (expected, ..) = state! {
            doc { root {
                table {
                    tr1: table_row {
                        table_cell { paragraph { text("1") } }
                        table_cell { paragraph carry([italic]) { text("2") [italic] } }
                        table_cell { paragraph { text("3") } }
                    }
                    tr2: table_row {
                        table_cell { paragraph { text("4") } }
                        table_cell { paragraph carry([italic]) { text("5") [italic] } }
                        table_cell { paragraph { text("6") } }
                    }
                }
            } }
            selection: (tr1, 2, >) -> (tr2, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_italic_apply_then_aggregate_uniform() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("HelloWorld") } } }
            selection: (p1, 0) -> (p1, 10)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));
        let ms = resolve_modifier_state(
            &actual.projected,
            actual.selection.as_ref().unwrap(),
            &actual.pending_modifiers,
        )
        .unwrap();
        assert_eq!(ms.italic, editor_common::Tri::Uniform { value: () });
    }
}
