use editor_common::Tri;
use editor_crdt::Dot;
use editor_model::{ChildView, DocView, Modifier, ModifierState, ModifierType};
use editor_state::ResolvedSelection;
use editor_state::{PendingModifier, PendingModifiers};
use editor_state::{resolve_modifier_state, resolve_modifier_state_in_range};
use editor_transaction::Transaction;

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

fn range_has_modifier(ms: &ModifierState, ty: ModifierType) -> bool {
    matches!(
        match ty {
            ModifierType::Italic => &ms.italic,
            ModifierType::Underline => &ms.underline,
            ModifierType::Strikethrough => &ms.strikethrough,
            _ => return false,
        },
        Tri::Uniform { .. }
    )
}

fn span_dots(view: &DocView, rs: &ResolvedSelection) -> Option<(Dot, Dot)> {
    let from = rs.from();
    let to = rs.to();

    let from_child = view.node(from.node())?.child_at(from.offset())?;
    let first = match from_child {
        ChildView::Leaf(l) => l.dot(),
        ChildView::Block(b) => b.dot()?,
    };

    let to_off = to.offset().checked_sub(1)?;
    let to_child = view.node(to.node())?.child_at(to_off)?;
    let last = match to_child {
        ChildView::Leaf(l) => l.dot(),
        ChildView::Block(b) => b.dot()?,
    };

    Some((first, last))
}

pub fn toggle_modifier(tr: &mut Transaction, modifier_type: ModifierType) -> CommandResult {
    let modifier = modifier_from_unit_type(modifier_type)?;
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    if selection.anchor == selection.head {
        return toggle_modifier_collapsed(tr, modifier_type, &modifier);
    }

    let (first, last, all_have) = {
        let view = tr.view();
        let rs = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let Some((first, last)) = span_dots(&view, &rs) else {
            return Ok(false);
        };
        let ms = resolve_modifier_state_in_range(&rs);
        (first, last, range_has_modifier(&ms, modifier_type))
    };

    if all_have {
        tr.clear_span_modifier(first, last, modifier)?;
    } else {
        tr.add_span_modifier(first, last, modifier)?;
    }

    Ok(true)
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
            doc { root { p1: paragraph { text("HelloWorld") [italic] } } }
            selection: (p1, 0) -> (p1, 10)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_toggle_italic_off() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("HelloWorld") [italic] } } }
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
    fn range_toggle_italic_off_from_style_reports_off() {
        let (initial, ..) = state! {
            doc {
                styles { em: "강조" [italic] }
                root { p1: paragraph @em { text("HelloWorld") } }
            }
            selection: (p1, 0) -> (p1, 10)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));

        let view = actual.view();
        let rs = actual
            .selection
            .as_ref()
            .and_then(|selection| selection.resolve(&view))
            .expect("selection still resolves");
        let ms = resolve_modifier_state_in_range(&rs);
        assert_eq!(ms.italic, Tri::Absent);
    }

    #[test]
    fn range_toggle_italic_off_from_run_style_reports_off() {
        let (initial, ..) = state! {
            doc {
                styles { em: "강조" [italic] }
                root { p1: paragraph { text("HelloWorld") @em } }
            }
            selection: (p1, 0) -> (p1, 10)
        };
        let (actual, ..) = transact!(initial, |tr| toggle_modifier(&mut tr, ModifierType::Italic));

        let view = actual.view();
        let rs = actual
            .selection
            .as_ref()
            .and_then(|selection| selection.resolve(&view))
            .expect("selection still resolves");
        let ms = resolve_modifier_state_in_range(&rs);
        assert_eq!(
            ms.italic,
            Tri::Absent,
            "clear must beat the run's own style"
        );
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
            doc { root { p: paragraph { text("HelloWorld") [italic] } } }
            selection: (p, 0) -> (p, 10)
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
}
