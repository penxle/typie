use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::insert_text_at_caret;

pub fn insert_text(tr: &mut Transaction, text: &str) -> CommandResult {
    insert_text_at_caret(tr, text)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::CommandError;
    use crate::test_utils::*;

    #[test]
    fn empty_text_returns_error() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let err = transact_err!(initial, |tr| insert_text(&mut tr, ""));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn newline_returns_error() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let err = transact_err!(initial, |tr| insert_text(&mut tr, "a\nb"));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn carriage_return_returns_error() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let err = transact_err!(initial, |tr| insert_text(&mut tr, "a\rb"));
        assert!(matches!(err, CommandError::InvalidArgument(_)));
    }

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 3)
        };
        transact_fail!(initial, |tr| insert_text(&mut tr, "X"));
    }

    #[test]
    fn insert_into_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "XY"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("HeXYllo") } } }
            selection: (t1, 4)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_start_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "AB"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("ABHello") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "!"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello!") } } }
            selection: (t1, 6)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_unicode_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "한글"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello한글") } } }
            selection: (t1, 7)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_with_pending_bold_creates_new_node() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t2: text("X") [bold]
                    }
                }
            }
            selection: (t2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_start_with_different_mods_creates_node_before() {
        // Bold has Expand::After → not inherited at start → effective = []
        // Current mods = [Bold] → mismatch → new node before
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("X")
                        t2: text("Hello") [bold]
                    }
                }
            }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_in_middle_with_pending_splits_node() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        // "He" [] → "X" [Bold] → "llo" []
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("He")
                        t2: text("X") [bold]
                        t3: text("llo")
                    }
                }
            }
            selection: (t2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_of_link_creates_node_after() {
        // Link has Expand::None → not inherited → new node after
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Click") [link(href: "https://a.com".to_string())] } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, " here"));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Click") [link(href: "https://a.com".to_string())]
                        t2: text(" here")
                    }
                }
            }
            selection: (t2, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_of_bold_stays_inline() {
        // Bold has Expand::After → inherited at end → match → Case 1
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "!"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello!") [bold] } } }
            selection: (t1, 6)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_into_empty_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "Hello"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn pending_modifiers_cleared_after_insert() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        assert!(actual.pending_modifiers.is_empty());
    }

    #[test]
    fn insert_into_non_textblock_returns_error() {
        let (initial, ..) = state! {
            doc { root { hr: horizontal_rule {} } }
            selection: (hr, 0)
        };
        let err = transact_err!(initial, |tr| insert_text(&mut tr, "X"));
        assert!(matches!(err, CommandError::Step(_)));
    }

    #[test]
    fn pending_unset_on_bold_text_creates_new_node() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 5)
            pending_modifiers: [!bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello") [bold]
                        t2: text("X")
                    }
                }
            }
            selection: (t2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_text_into_empty_paragraph_with_marker_consumes_marker() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph [bold] {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "Y"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Y") [bold] } } }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_text_in_middle_split_state_coalesces_with_right_half() {
        let (initial, ..) = state! {
            doc { root { paragraph [bold] { t1: text("llo") [bold] } } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "Y"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("Yllo") [bold] } } }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_into_empty_paragraph_preserves_paragraph_only_modifier() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph [line_height(220)] {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        let (expected, ..) = state! {
            doc { root { paragraph [line_height(220)] { t1: text("X") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_into_empty_paragraph_with_mixed_markers_carries_only_text_applicable() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph [bold, line_height(220)] {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "Y"));
        let (expected, ..) = state! {
            doc { root { paragraph [line_height(220)] { t1: text("Y") [bold] } } }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_text_clears_paragraph_marker_even_if_text_already_styled() {
        let (initial, ..) = state! {
            doc { root { paragraph [bold] { t1: text("Hi") [bold] } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_text(&mut tr, "X"));
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("HiX") [bold] } } }
            selection: (t1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }
}
