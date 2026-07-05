use editor_crdt::Dot;
use editor_model::{ChildView, DocView, Modifier, ModifierType};
use editor_state::{PendingModifier, PendingModifiers, Position, leaf_span_in_range};
use editor_transaction::Transaction;

use crate::helpers::is_text_applicable;
use crate::{CommandError, CommandResult};

pub fn edit_modifier(
    tr: &mut Transaction,
    modifier_type: ModifierType,
    modifier: Option<Modifier>,
) -> CommandResult {
    if let Some(m) = &modifier
        && m.as_type() != modifier_type
    {
        return Err(CommandError::InvalidArgument(format!(
            "modifier type mismatch: op type {:?}, modifier {:?}",
            modifier_type,
            m.as_type()
        )));
    }
    if let Some(m) = &modifier
        && !m.is_valid()
    {
        return Ok(false);
    }
    if !is_text_applicable(modifier_type) {
        return Err(CommandError::InvalidArgument(format!(
            "edit_modifier is only valid for text-applicable modifiers; got {:?}",
            modifier_type
        )));
    }

    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor == selection.head {
        edit_modifier_collapsed(tr, modifier_type, modifier)
    } else {
        edit_modifier_range(tr, modifier_type, modifier)
    }
}

/// The contiguous char run around a collapsed caret that shares the same
/// effective value of `ty`. Returns the run's first/last char `Dot`s and the
/// shared value, or `None` when the caret is not inside such a span.
fn resolve_modifier_span(
    view: &DocView,
    pos: &Position,
    ty: ModifierType,
) -> Option<(Dot, Dot, Modifier)> {
    let node = view.node(pos.node)?;
    if !node.spec().is_textblock() {
        return None;
    }
    let children: Vec<ChildView> = node.children().collect();

    let has_value = |i: usize| {
        node.leaf_state_at(i)
            .is_some_and(|st| st.eff.get(&ty).is_some())
    };

    let seed = if has_value(pos.offset) {
        pos.offset
    } else {
        let left = pos.offset.checked_sub(1)?;
        if has_value(left) { left } else { return None }
    };

    let reference = node
        .leaf_state_at(seed)
        .and_then(|st| st.eff.get(&ty).cloned())?;

    let same = |i: usize| {
        node.leaf_state_at(i)
            .is_some_and(|st| st.eff.get(&ty) == Some(&reference))
    };

    let mut start = seed;
    while start > 0 && same(start - 1) {
        start -= 1;
    }
    let mut end = seed;
    while end + 1 < children.len() && same(end + 1) {
        end += 1;
    }

    let first = match &children[start] {
        ChildView::Leaf(l) => l.dot(),
        _ => return None,
    };
    let last = match &children[end] {
        ChildView::Leaf(l) => l.dot(),
        _ => return None,
    };
    Some((first, last, reference))
}

fn edit_modifier_collapsed(
    tr: &mut Transaction,
    modifier_type: ModifierType,
    modifier: Option<Modifier>,
) -> CommandResult {
    if !matches!(modifier_type, ModifierType::Link | ModifierType::Ruby) {
        let mut pending: PendingModifiers = tr
            .pending_modifiers()
            .iter()
            .filter(|pm| pm.as_type() != modifier_type)
            .cloned()
            .collect();
        pending.push(match modifier {
            Some(m) => PendingModifier::Set { modifier: m },
            None => PendingModifier::Unset { ty: modifier_type },
        });
        tr.set_pending_modifiers(pending)?;
        return Ok(true);
    }

    let pos: Position = tr
        .selection()
        .expect("entry caller guaranteed selection")
        .head;

    let span = {
        let view = tr.view();
        resolve_modifier_span(&view, &pos, modifier_type)
    };
    let Some((first, last, reference)) = span else {
        return Ok(false);
    };

    match modifier {
        Some(m) => tr.add_span_modifier(first, last, m)?,
        None => tr.remove_span_modifier(first, last, reference)?,
    }
    Ok(true)
}

fn edit_modifier_range(
    tr: &mut Transaction,
    modifier_type: ModifierType,
    modifier: Option<Modifier>,
) -> CommandResult {
    let selection = tr.selection().expect("entry caller guaranteed selection");

    let (first, last, present) = {
        let view = tr.view();
        let rs = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let Some((first, last)) = leaf_span_in_range(&rs) else {
            return Ok(false);
        };
        let present = view
            .leaf_state_by_dot_slow(first)
            .and_then(|st| st.eff.get(&modifier_type).cloned());
        (first, last, present)
    };

    match modifier {
        Some(m) => {
            tr.add_span_modifier(first, last, m)?;
        }
        None => {
            if let Some(present) = present {
                tr.remove_span_modifier(first, last, present)?;
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::Modifier;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn collapsed_inside_link_set_updates_span_value() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Click") [link(href: "https://a.com".to_string())] } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Link {
                href: "https://b.com".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Click") [link(href: "https://b.com".to_string())] } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_inside_link_remove_clears_span() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Click") [link(href: "https://a.com".to_string())] } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            None,
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Click") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_link_inserts_on_plain_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Link {
                href: "https://a.com".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph {
                text("Hello") [link(href: "https://a.com".to_string())]
            } } }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_set_link_replaces_existing_value() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {
                text("Hello") [link(href: "https://a.com".to_string())]
            } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Link {
                href: "https://b.com".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph {
                text("Hello") [link(href: "https://b.com".to_string())]
            } } }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_remove_link_clears() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {
                text("Hello") [link(href: "https://a.com".to_string())]
            } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            None,
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_outside_link_set_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("plain") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact_fail!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Link {
                href: "https://a.com".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("plain") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn non_text_modifier_rejected() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let err = transact_err!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::LineHeight,
            Some(Modifier::LineHeight { value: 200 })
        ));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn value_modifier_type_mismatch_rejected() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        let err = transact_err!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Ruby {
                text: "x".to_string()
            })
        ));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn collapsed_background_color_none_updates_pending_not_document() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::BackgroundColor,
            None,
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
            pending_modifiers: [!background_color]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_background_color_set_updates_pending_not_document() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::BackgroundColor,
            Some(Modifier::BackgroundColor {
                value: "red".to_string()
            }),
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
            pending_modifiers: [background_color("red".to_string())]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_background_none_adjacent_to_painted_run_leaves_run_intact() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {
                text("Hello") [background_color("red".to_string())]
            } } }
            selection: (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::BackgroundColor,
            None,
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph {
                text("Hello") [background_color("red".to_string())]
            } } }
            selection: (p1, 3)
            pending_modifiers: [!background_color]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_pending_edit_preserves_other_pending() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
            pending_modifiers: [italic]
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::BackgroundColor,
            None,
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 3)
            pending_modifiers: [italic, !background_color]
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn empty_href_is_noop() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 5)
        };
        transact_fail!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Link {
                href: String::new()
            })
        ));
    }

    #[test]
    fn collapsed_adjacent_different_href_isolates() {
        let (initial, ..) = state! {
            doc { root { p: paragraph {
                text("Hello") [link(href: "https://a.com".to_string())]
                text("World") [link(href: "https://b.com".to_string())]
            } } }
            selection: (p, 7)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Link {
                href: "https://c.com".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root { p: paragraph {
                text("Hello") [link(href: "https://a.com".to_string())]
                text("World") [link(href: "https://c.com".to_string())]
            } } }
            selection: (p, 7)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_paragraph_boundary_not_crossed() {
        let (initial, ..) = state! {
            doc { root {
                p1: paragraph { text("a") [link(href: "https://a.com".to_string())] }
                p2: paragraph { text("b") [link(href: "https://a.com".to_string())] }
            } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Link {
                href: "https://c.com".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root {
                p1: paragraph { text("a") [link(href: "https://c.com".to_string())] }
                p2: paragraph { text("b") [link(href: "https://a.com".to_string())] }
            } }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_inside_ruby_set_updates_span_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Han") [ruby(text: "한".to_string())] } } }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Ruby,
            Some(Modifier::Ruby {
                text: "韓".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Han") [ruby(text: "韓".to_string())] } } }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn mixed_range_set_overwrites_all() {
        let (initial, ..) = state! {
            doc { root { p: paragraph {
                text("Hello") [link(href: "https://a.com".to_string())]
                text("World") [link(href: "https://b.com".to_string())]
            } } }
            selection: (p, 0) -> (p, 10)
        };
        let (actual, ..) = transact!(initial, |tr| edit_modifier(
            &mut tr,
            ModifierType::Link,
            Some(Modifier::Link {
                href: "https://c.com".to_string()
            })
        ));
        let (expected, ..) = state! {
            doc { root { p: paragraph {
                text("HelloWorld") [link(href: "https://c.com".to_string())]
            } } }
            selection: (p, 0) -> (p, 10)
        };
        assert_state_eq!(&actual, &expected);
    }
}
