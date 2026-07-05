use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::insert_tab_at_caret;

pub fn insert_tab(tr: &mut Transaction) -> CommandResult {
    insert_tab_at_caret(tr)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { t1: paragraph { text("Hello") } } }
            selection: (t1, 0) -> (t1, 3)
        };
        transact_fail!(initial, |tr| insert_tab(&mut tr));
    }

    #[test]
    fn insert_in_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { t1: paragraph { text("Hello") } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_tab(&mut tr));
        let (expected, ..) = state! {
            doc { root { t: paragraph { text("He") tab text("llo") } } }
            selection: (t, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_tab(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Hello") tab } } }
            selection: (p1, 6)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_in_empty_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_tab(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { tab } } }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_attaches_carryable_marker() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_tab(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph marker([bold]) { text("Hello") [bold] tab } } }
            selection: (p1, 6)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn tab_node_carries_font_size_metric() {
        let (initial, ..) = state! {
            doc { root { t1: paragraph { text("Hi") [font_size(2400)] } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_tab(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph marker([font_size(2400)]) { text("Hi") [font_size(2400)] tab [font_size(2400)] } } }
            selection: (p1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn tab_into_empty_paragraph_with_marker_carries_metric() {
        let (state, ..) = state! {
            doc { root { p1: paragraph marker([font_size(2400)]) {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(state, |tr| insert_tab(&mut tr));
        let (expected, ..) = state! {
            doc { root { p1: paragraph { tab [font_size(2400)] } } }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }
}
