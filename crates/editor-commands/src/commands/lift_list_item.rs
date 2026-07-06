use editor_transaction::Transaction;

use crate::CommandResult;
use crate::helpers::lift_selected_list_items;

pub fn lift_list_item(tr: &mut Transaction) -> CommandResult {
    lift_selected_list_items(tr)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn lift_top_level_single_item() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn outside_list_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("A") } } }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| lift_list_item(&mut tr));
    }

    #[test]
    fn lift_top_level_middle_splits_list() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                        list_item { paragraph { text("C") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { text("A") } } }
                    p1: paragraph { text("B") }
                    bullet_list { list_item { paragraph { text("C") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_top_level_first_item() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                        list_item { paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    bullet_list { list_item { paragraph { text("B") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_top_level_last_item() {
        let (initial, ..) = state! {
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
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { text("A") } } }
                    p1: paragraph { text("B") }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_nested_middle_moves_after_items_into_lifted() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("outer") }
                            bullet_list {
                                list_item { paragraph { text("A") } }
                                list_item { p1: paragraph { text("B") } }
                                list_item { paragraph { text("C") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("outer") }
                            bullet_list {
                                list_item { paragraph { text("A") } }
                            }
                        }
                        list_item {
                            p1: paragraph { text("B") }
                            bullet_list {
                                list_item { paragraph { text("C") } }
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
    fn lift_preserves_list_type_ordered() {
        let (initial, ..) = state! {
            doc {
                root {
                    ordered_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                        list_item { paragraph { text("C") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    ordered_list { list_item { paragraph { text("A") } } }
                    p1: paragraph { text("B") }
                    ordered_list { list_item { paragraph { text("C") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_from_blockquote() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        bullet_list { list_item { p1: paragraph { text("A") } } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote { p1: paragraph { text("A") } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_from_blockquote_carries_sublist() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        bullet_list {
                            list_item {
                                p1: paragraph { text("A") }
                                bullet_list { list_item { paragraph { text("a1") } } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote {
                        p1: paragraph { text("A") }
                        bullet_list { list_item { paragraph { text("a1") } } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_list_item_with_sublist() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("A") }
                            bullet_list { list_item { paragraph { text("a1") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    bullet_list { list_item { paragraph { text("a1") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_range_across_separate_lists_lifts_each_list() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    bullet_list { list_item { p2: paragraph { text("B") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    p2: paragraph { text("B") }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_range_two_consecutive_items() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { p1: paragraph { text("B") } }
                        list_item { p2: paragraph { text("C") } }
                        list_item { paragraph { text("D") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { text("A") } } }
                    p1: paragraph { text("B") }
                    p2: paragraph { text("C") }
                    bullet_list { list_item { paragraph { text("D") } } }
                    paragraph {}
                }
            }
            selection: (p1, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_range_with_plain_paragraph_lifts_only_list_items() {
        let (initial, ..) = state! {
            doc {
                root {
                    p0: paragraph { text("plain") }
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (p0, 0) -> (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p0: paragraph { text("plain") }
                    p1: paragraph { text("A") }
                    paragraph {}
                }
            }
            selection: (p0, 0) -> (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_range_parent_and_nested_child_lifts_each_one_level() {
        let (initial, ..) = state! {
            doc {
                root {
                    p0: paragraph { text("plain") }
                    bullet_list {
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
            selection: (p0, 0) -> (p2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p0: paragraph { text("plain") }
                    p1: paragraph { text("A") }
                    bullet_list {
                        list_item { p2: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p0, 0) -> (p2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_range_top_level_item_with_nested_endpoint_preserves_range() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
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
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("A") }
                    bullet_list {
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
    fn lift_nested_item_with_existing_sublist_appends_after_items() {
        // Nested list_item B already owns a sublist and is followed by trailing
        // siblings C, D. Lifting B must move it out as a sibling on the outer list
        // and append C, D into B's existing sublist — list_item allows at most one
        // trailing sublist, so a second one cannot be created.
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("outer") }
                            bullet_list {
                                list_item {
                                    p_b: paragraph { text("B") }
                                    bullet_list { list_item { paragraph { text("b_sub") } } }
                                }
                                list_item { paragraph { text("C") } }
                                list_item { paragraph { text("D") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p_b, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        // Original nested sublist becomes empty after B's lift and is pruned, so the
        // first list_item retains only its `outer` paragraph. B becomes a new sibling
        // on the outer list with C, D appended into its pre-existing sublist.
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("outer") }
                        }
                        list_item {
                            p_b: paragraph { text("B") }
                            bullet_list {
                                list_item { paragraph { text("b_sub") } }
                                list_item { paragraph { text("C") } }
                                list_item { paragraph { text("D") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p_b, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
