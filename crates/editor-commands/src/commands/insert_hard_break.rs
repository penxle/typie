use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::insert_hard_break_at_caret;

pub fn insert_hard_break(tr: &mut Transaction) -> CommandResult {
    insert_hard_break_at_caret(tr)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn non_collapsed_selection_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0) -> (p1, 3)
        };
        transact_fail!(initial, |tr| insert_hard_break(&mut tr));
    }

    #[test]
    fn insert_at_start_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        hard_break
                        text("Hello")
                    }
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_in_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("He")
                        hard_break
                        text("llo")
                    }
                }
            }
            selection: (p1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello")
                        hard_break
                    }
                }
            }
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
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        hard_break
                    }
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn pending_modifiers_preserved() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        assert!(!actual.pending_modifiers.is_empty());
    }

    #[test]
    fn insert_hard_break_at_end_attaches_marker_to_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph marker([bold]) {
                        text("Hello") [bold]
                        hard_break
                    }
                }
            }
            selection: (p1, 6)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_hard_break_in_middle_attaches_marker_to_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph marker([bold]) {
                        text("He") [bold]
                        hard_break
                        text("llo") [bold]
                    }
                }
            }
            selection: (p1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_hard_break_at_start_attaches_no_marker() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        hard_break
                        text("Hello") [bold]
                    }
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }
}
