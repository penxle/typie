use editor_state::Selection;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{delete_selection_range, insert_text_at_caret};

pub fn replace_tracked_range(
    tr: &mut Transaction,
    selection: Selection,
    replacement: &str,
) -> CommandResult {
    if selection.is_collapsed() {
        return Ok(false);
    }
    if replacement.contains(['\n', '\r']) {
        return Ok(false);
    }

    tr.set_selection(Some(selection))?;
    delete_selection_range(tr, selection)?;

    if !replacement.is_empty() {
        insert_text_at_caret(tr, replacement)?;
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::assert_state_eq;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn replace_within_text() {
        let (initial, t1) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 0)
        };
        let sel = Selection::new(
            editor_state::Position::new(t1, 6),
            editor_state::Position::new(t1, 11),
        );
        let (actual, ..) = transact!(initial, |tr| replace_tracked_range(&mut tr, sel, "earth"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello earth") } } }
            selection: (t1, 11)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn replace_with_empty_deletes_range() {
        let (initial, t1) = state! {
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 0)
        };
        let sel = Selection::new(
            editor_state::Position::new(t1, 5),
            editor_state::Position::new(t1, 11),
        );
        let (actual, ..) = transact!(initial, |tr| replace_tracked_range(&mut tr, sel, ""));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_selection_returns_false() {
        let (initial, t1) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let sel = Selection::collapsed(editor_state::Position::new(t1, 2));
        transact_fail!(initial, |tr| replace_tracked_range(&mut tr, sel, "x"));
    }

    #[test]
    fn replacement_with_newline_returns_false() {
        let (initial, t1) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        let sel = Selection::new(
            editor_state::Position::new(t1, 0),
            editor_state::Position::new(t1, 5),
        );
        transact_fail!(initial, |tr| replace_tracked_range(&mut tr, sel, "a\nb"));
    }
}
