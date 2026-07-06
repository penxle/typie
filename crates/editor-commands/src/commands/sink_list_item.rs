use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::sink_selected_list_items;

pub fn sink_list_item(tr: &mut Transaction) -> CommandResult {
    sink_selected_list_items(tr)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn sink_simple_top_level() {
        let (initial, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list {
                                list_item { p1: paragraph { text("B") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn no_prev_returns_false() {
        let (initial, _) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| sink_list_item(&mut tr));
    }

    #[test]
    fn outside_list_returns_false() {
        let (initial, _) = state! {
            doc { root { p1: paragraph { text("A") } } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| sink_list_item(&mut tr));
    }

    #[test]
    fn sink_appends_to_existing_sublist() {
        let (initial, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list { list_item { paragraph { text("a1") } } }
                        }
                        list_item { p1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list {
                                list_item { paragraph { text("a1") } }
                                list_item { p1: paragraph { text("B") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sink_preserves_ordered_type() {
        let (initial, _) = state! {
            doc {
                root {
                    ordered_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, _) = state! {
            doc {
                root {
                    ordered_list {
                        list_item {
                            paragraph { text("A") }
                            ordered_list {
                                list_item { p1: paragraph { text("B") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sink_carries_existing_sublist() {
        let (initial, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item {
                            p1: paragraph { text("B") }
                            bullet_list { list_item { paragraph { text("b1") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list {
                                list_item {
                                    p1: paragraph { text("B") }
                                    bullet_list { list_item { paragraph { text("b1") } } }
                                }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sink_range_two_items() {
        let (initial, _, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                        list_item { p2: paragraph { text("C") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, _, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list {
                                list_item { p1: paragraph { text("B") } }
                                list_item { p2: paragraph { text("C") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sink_range_across_separate_lists_indents_groups_that_can_move() {
        let (initial, _, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                    }
                    bullet_list {
                        list_item { paragraph { text("C") } }
                        list_item { p2: paragraph { text("D") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, _, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list {
                                list_item { p1: paragraph { text("B") } }
                            }
                        }
                    }
                    bullet_list {
                        list_item { paragraph { text("C") } }
                        list_item { p2: paragraph { text("D") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sink_range_first_has_no_prev_consumes_without_change() {
        let (initial, _, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                        list_item { p2: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, _, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                        list_item { p2: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sink_range_with_plain_paragraph_consumes_when_first_list_item_cannot_indent() {
        let (initial, _, _) = state! {
            doc {
                root {
                    p0: paragraph { text("plain") }
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p0, 0) -> (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, _, _) = state! {
            doc {
                root {
                    p0: paragraph { text("plain") }
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p0, 0) -> (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sink_range_with_trailing_plain_paragraph_indents_only_list_items() {
        let (initial, _, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                    }
                    p0: paragraph { text("plain") }
                }
            }
            selection: (p1, 0) -> (p0, 1)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, _, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list {
                                list_item { p1: paragraph { text("B") } }
                            }
                        }
                    }
                    p0: paragraph { text("plain") }
                }
            }
            selection: (p1, 0) -> (p0, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sink_range_parent_and_nested_child_moves_parent_once_and_preserves_range() {
        let (initial, _, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("P") } }
                        list_item {
                            p1: paragraph { text("A") }
                            bullet_list {
                                list_item { p2: paragraph { text("B") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, _, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("P") }
                            bullet_list {
                                list_item {
                                    p1: paragraph { text("A") }
                                    bullet_list {
                                        list_item { p2: paragraph { text("B") } }
                                    }
                                }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sink_into_prev_with_different_type_sublist_reuses_existing() {
        // prev_list_item already owns an ordered sublist (a state reachable only
        // via paste/import). Reusing it regardless of type preserves the single-
        // sublist invariant; the sunk item becomes a child of the ordered list.
        let (initial, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            ordered_list { list_item { paragraph { text("a1") } } }
                        }
                        list_item { p1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, _) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            ordered_list {
                                list_item { paragraph { text("a1") } }
                                list_item { p1: paragraph { text("B") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
