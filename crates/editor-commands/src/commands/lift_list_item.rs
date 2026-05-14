use editor_state::Position;
use editor_transaction::Transaction;

use crate::helpers::{
    collect_top_level_list_items_in_selection, find_enclosing_list_item_id, lift_list_item_inner,
};
use crate::{CommandError, CommandResult};

pub fn lift_list_item(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection();
    let doc = tr.doc();

    if selection.is_collapsed() {
        let pos = selection.head;
        let Some(list_item_id) = find_enclosing_list_item_id(&doc, pos.node_id) else {
            return Ok(false);
        };
        return lift_list_item_inner(tr, list_item_id);
    }

    let resolved = selection
        .resolve(&doc)
        .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
    let from = Position::from(resolved.from());
    let to = Position::from(resolved.to());

    let items = collect_top_level_list_items_in_selection(&doc, from, to);
    if items.is_empty() {
        return Ok(false);
    }

    // Lift later siblings first so the earlier items' parent-list indices are
    // not disturbed before they are processed.
    let mut any_lifted = false;
    for item_id in items.iter().rev() {
        let doc = tr.doc();
        if doc.node(*item_id).is_none() {
            continue;
        }
        if lift_list_item_inner(tr, *item_id)? {
            any_lifted = true;
        }
    }

    Ok(any_lifted)
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
                        list_item { paragraph { t1: text("A") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn outside_list_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("A") } } }
            selection: (t1, 0)
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
                        list_item { paragraph { t1: text("B") } }
                        list_item { paragraph { text("C") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { text("A") } } }
                    paragraph { t1: text("B") }
                    bullet_list { list_item { paragraph { text("C") } } }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_top_level_first_item() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("A") } }
                        list_item { paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                    bullet_list { list_item { paragraph { text("B") } } }
                    paragraph {}
                }
            }
            selection: (t1, 0)
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
                        list_item { paragraph { t1: text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { text("A") } } }
                    paragraph { t1: text("B") }
                    paragraph {}
                }
            }
            selection: (t1, 0)
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
                                list_item { paragraph { t1: text("B") } }
                                list_item { paragraph { text("C") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
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
                            paragraph { t1: text("B") }
                            bullet_list {
                                list_item { paragraph { text("C") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
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
                        list_item { paragraph { t1: text("B") } }
                        list_item { paragraph { text("C") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    ordered_list { list_item { paragraph { text("A") } } }
                    paragraph { t1: text("B") }
                    ordered_list { list_item { paragraph { text("C") } } }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_from_blockquote() {
        let (initial, ..) = state! {
            doc {
                root {
                    blockquote {
                        bullet_list { list_item { paragraph { t1: text("A") } } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote { paragraph { t1: text("A") } }
                    paragraph {}
                }
            }
            selection: (t1, 0)
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
                                paragraph { t1: text("A") }
                                bullet_list { list_item { paragraph { text("a1") } } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    blockquote {
                        paragraph { t1: text("A") }
                        bullet_list { list_item { paragraph { text("a1") } } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
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
                            paragraph { t1: text("A") }
                            bullet_list { list_item { paragraph { text("a1") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("A") }
                    bullet_list { list_item { paragraph { text("a1") } } }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn lift_range_across_separate_lists_returns_false() {
        // Selection that straddles two top-level lists has no common list ancestor;
        // collect_top_level_list_items_in_selection returns empty, so the range
        // branch is a no-op. The user can lift each list separately.
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { t1: text("A") } } }
                    bullet_list { list_item { paragraph { t2: text("B") } } }
                    paragraph {}
                }
            }
            selection: (t1, 0) -> (t2, 1)
        };
        transact_fail!(initial, |tr| lift_list_item(&mut tr));
    }

    #[test]
    fn lift_range_two_consecutive_items() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { paragraph { t1: text("B") } }
                        list_item { paragraph { t2: text("C") } }
                        list_item { paragraph { text("D") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0) -> (t2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| lift_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { text("A") } } }
                    paragraph { t1: text("B") }
                    paragraph { t2: text("C") }
                    bullet_list { list_item { paragraph { text("D") } } }
                    paragraph {}
                }
            }
            // Range lift processes items in reverse, so the final cursor lands on
            // the last-lifted (earliest) item's paragraph rather than spanning the range.
            selection: (t1, 0)
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
                                    paragraph { t_b: text("B") }
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
            selection: (t_b, 0)
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
                            paragraph { t_b: text("B") }
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
            selection: (t_b, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
