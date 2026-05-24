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
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0) -> (t1, 3)
        };
        transact_fail!(initial, |tr| insert_hard_break(&mut tr));
    }

    #[test]
    fn insert_at_start_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        hard_break
                        t1: text("Hello")
                    }
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_in_middle_of_text() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("He")
                        hard_break
                        t2: text("llo")
                    }
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_at_end_of_text() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        t1: text("Hello")
                        hard_break
                    }
                }
            }
            selection: (p1, 2)
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
    fn insert_at_end_with_next_text_sibling() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        t2: text("World")
                    }
                }
            }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello")
                        hard_break
                        t2: text("World")
                    }
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn pending_modifiers_preserved() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") } } }
            selection: (t1, 5)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        assert!(!actual.pending_modifiers.is_empty());
    }

    #[test]
    fn insert_hard_break_at_end_attaches_marker_to_paragraph() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph [bold] {
                        t1: text("Hello") [bold]
                        hard_break
                    }
                }
            }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_hard_break_in_middle_attaches_marker_to_paragraph() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph [bold] {
                        t1: text("He") [bold]
                        hard_break
                        t2: text("llo") [bold]
                    }
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_hard_break_at_start_attaches_no_marker() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("Hello") [bold] } } }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        hard_break
                        t1: text("Hello") [bold]
                    }
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
