use editor_model::Modifier;
use editor_state::Selection;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::replace_range_with_text as replace_range_with_text_core;

pub fn replace_range_with_text(
    tr: &mut Transaction,
    selection: Selection,
    replacement: &str,
    paint_override: Option<Vec<Modifier>>,
) -> CommandResult {
    replace_range_with_text_core(tr, selection, replacement, paint_override)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::Position;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn empty_range_pure_insert_follows_continuation() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("ab") [bold] } } }
            selection: (p1, 2)
        };
        let sel = Selection::collapsed(Position::new(p1, 2));
        let (actual, ..) = transact!(initial, |tr| replace_range_with_text(
            &mut tr, sel, "X", None
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("abX") [bold] } } }
            selection: (p1, 3, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn range_overwrite_copies_first_charlike_paint() {
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("가나다") [bold] } } }
            selection: (p1, 0) -> (p1, 3)
        };
        let sel = Selection::new(Position::new(p1, 0), Position::new(p1, 3));
        let (actual, ..) = transact!(initial, |tr| replace_range_with_text(
            &mut tr, sel, "라", None
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("라") [bold] } } }
            selection: (p1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn replacement_paint_skips_leading_page_break() {
        use editor_state::replacement_paint;
        let (initial, p1, p2) = state! {
            doc { root {
                p1: paragraph { page_break }
                p2: paragraph { text("가") [bold] }
            } }
            selection: (p1, 0)
        };
        let paint = replacement_paint(
            &initial.projected,
            Position::new(p1, 0),
            Position::new(p2, 1),
        )
        .expect("first charlike found past the leading page break leaf");
        assert_eq!(paint, vec![editor_model::Modifier::Bold]);
    }

    #[test]
    fn replacement_paint_normalizes_reversed_range() {
        use editor_state::replacement_paint;
        let (initial, p1) = state! {
            doc { root { p1: paragraph { text("가") [bold] text("나") [italic] } } }
            selection: (p1, 0)
        };
        let paint = replacement_paint(
            &initial.projected,
            Position::new(p1, 2),
            Position::new(p1, 0),
        )
        .expect("charlike found");
        assert!(
            paint
                .iter()
                .any(|m| matches!(m, editor_model::Modifier::Bold))
        );
        assert!(
            !paint
                .iter()
                .any(|m| matches!(m, editor_model::Modifier::Italic))
        );
    }
}
