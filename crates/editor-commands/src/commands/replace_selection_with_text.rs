use editor_model::Modifier;
use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::replace_range_with_text;

pub fn replace_selection_with_text(
    tr: &mut Transaction,
    replacement: &str,
    paint_override: Option<Vec<Modifier>>,
) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    replace_range_with_text(tr, selection, replacement, paint_override)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn bold_full_selection_overwrite_stays_bold() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("가나다") [bold] } } }
            selection: (p1, 0) -> (p1, 3)
        };
        let (actual, ..) = transact!(initial, |tr| replace_selection_with_text(
            &mut tr, "라", None
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("라") [bold] } } }
            selection: (p1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn reversed_selection_overwrite_copies_document_order_first_charlike() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("가") [bold] text("나") [italic] } } }
            selection: (p1, 2) -> (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| replace_selection_with_text(
            &mut tr, "라", None
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("라") [bold] } } }
            selection: (p1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn partial_link_selection_overwrite_keeps_link() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("가나") [link(href: "https://a.com".to_string())]
                    }
                }
            }
            selection: (p1, 1) -> (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| replace_selection_with_text(
            &mut tr, "다", None
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("가다") [link(href: "https://a.com".to_string())]
                    }
                }
            }
            selection: (p1, 2, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn collapsed_selection_pure_insert() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| replace_selection_with_text(
            &mut tr, "!", None
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hi!") } } }
            selection: (p1, 3, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn pending_modifiers_overlay_captured_paint_then_consume() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("가나") [bold] } } }
            selection: (p1, 0) -> (p1, 2)
            pending_modifiers: [italic]
        };
        let (actual, ..) = transact!(initial, |tr| replace_selection_with_text(
            &mut tr, "라", None
        ));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("라") [bold, italic] } } }
            selection: (p1, 1, <)
        };
        assert_state_eq!(&actual, &expected);
        assert!(actual.pending_modifiers.is_empty());
    }

    #[test]
    fn no_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { text("hi") } } }
            selection: none
        };
        let mut tr = Transaction::new(&initial);
        assert!(matches!(
            replace_selection_with_text(&mut tr, "x", None),
            Ok(false)
        ));
    }
}
