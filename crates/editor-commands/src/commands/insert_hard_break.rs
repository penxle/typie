use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::{consume_pending_modifiers, insert_hard_break_at_caret};

pub fn insert_hard_break(tr: &mut Transaction) -> CommandResult {
    let changed = insert_hard_break_at_caret(tr, None)?;
    if changed {
        consume_pending_modifiers(tr)?;
    }
    Ok(changed)
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
    fn pending_modifiers_consumed() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") } } }
            selection: (p1, 5)
            pending_modifiers: [bold]
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello")
                        hard_break [bold]
                    }
                }
            }
            selection: (p1, 6)
        };
        assert_state_eq!(&actual, &expected);
        assert!(actual.pending_modifiers.is_empty());
    }

    #[test]
    fn insert_hard_break_at_end_copies_left_paint_writes_no_block_carry() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello") [bold]
                        hard_break [bold]
                    }
                }
            }
            selection: (p1, 6)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_hard_break_in_middle_copies_left_paint_writes_no_block_carry() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 2)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("He") [bold]
                        hard_break [bold]
                        text("llo") [bold]
                    }
                }
            }
            selection: (p1, 3)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn hard_break_in_empty_paragraph_copies_carry_paint_preserves_block_carry() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph carry([bold]) {} } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph carry([bold]) {
                        hard_break [bold]
                    }
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_hard_break_at_start_copies_right_neighbor_paint() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("Hello") [bold] } } }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        hard_break [bold]
                        text("Hello") [bold]
                    }
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn insert_hard_break_copies_left_font_family_and_weight() {
        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello") [font_family("KoPubBatang".to_string()), font_weight(700)]
                    }
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello") [font_family("KoPubBatang".to_string()), font_weight(700)]
                        hard_break [font_family("KoPubBatang".to_string()), font_weight(700)]
                    }
                }
            }
            selection: (p1, 6)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn typing_after_hard_break_inherits_its_paint() {
        use crate::commands::insert_text;

        let (initial, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello") [font_family("KoPubBatang".to_string()), font_weight(700)]
                    }
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| {
            insert_hard_break(&mut tr).unwrap();
            insert_text(&mut tr, "World")
        });
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("Hello") [font_family("KoPubBatang".to_string()), font_weight(700)]
                        hard_break [font_family("KoPubBatang".to_string()), font_weight(700)]
                        text("World") [font_family("KoPubBatang".to_string()), font_weight(700)]
                    }
                }
            }
            selection: (p1, 11, <)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn hard_break_in_link_run_omits_link() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("ab") [link(href: "https://a.com".to_string())] } } }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| insert_hard_break(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("a") [link(href: "https://a.com".to_string())]
                        hard_break
                        text("b") [link(href: "https://a.com".to_string())]
                    }
                }
            }
            selection: (p1, 2)
        };
        assert_state_eq!(&actual, &expected);
    }
}
