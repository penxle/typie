use editor_state::Position;
use editor_transaction::Transaction;

use crate::helpers::{
    collect_top_level_list_items_in_selection, find_enclosing_list_item_id, sink_list_item_inner,
};
use crate::{CommandError, CommandResult};

pub fn sink_list_item(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let doc = tr.doc();

    if selection.is_collapsed() {
        let pos = selection.head;
        let Some(list_item_id) = find_enclosing_list_item_id(&doc, pos.node_id) else {
            return Ok(false);
        };
        return sink_list_item_inner(tr, list_item_id);
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

    // Google Docs behavior: if the first item has no prev sibling, the entire
    // range is a no-op rather than partially sinking later items.
    let first = doc
        .node(items[0])
        .ok_or(CommandError::NodeNotFound(items[0]))?;
    if first.prev_sibling().is_none() {
        return Ok(false);
    }

    let mut any_sunk = false;
    for item_id in items.iter() {
        let doc = tr.doc();
        if doc.node(*item_id).is_none() {
            continue;
        }
        if sink_list_item_inner(tr, *item_id)? {
            any_sunk = true;
        }
    }
    Ok(any_sunk)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn sink_simple_top_level() {
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
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list {
                                list_item { paragraph { t1: text("B") } }
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
    fn no_prev_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { t1: text("A") } } }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| sink_list_item(&mut tr));
    }

    #[test]
    fn outside_list_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("A") } } }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| sink_list_item(&mut tr));
    }

    #[test]
    fn sink_appends_to_existing_sublist() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list { list_item { paragraph { text("a1") } } }
                        }
                        list_item { paragraph { t1: text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list {
                                list_item { paragraph { text("a1") } }
                                list_item { paragraph { t1: text("B") } }
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
    fn sink_preserves_ordered_type() {
        let (initial, ..) = state! {
            doc {
                root {
                    ordered_list {
                        list_item { paragraph { text("A") } }
                        list_item { paragraph { t1: text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    ordered_list {
                        list_item {
                            paragraph { text("A") }
                            ordered_list {
                                list_item { paragraph { t1: text("B") } }
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
    fn sink_carries_existing_sublist() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item {
                            paragraph { t1: text("B") }
                            bullet_list { list_item { paragraph { text("b1") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list {
                                list_item {
                                    paragraph { t1: text("B") }
                                    bullet_list { list_item { paragraph { text("b1") } } }
                                }
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
    fn sink_range_two_items() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { paragraph { t1: text("B") } }
                        list_item { paragraph { t2: text("C") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0) -> (t2, 1)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            bullet_list {
                                list_item { paragraph { t1: text("B") } }
                                list_item { paragraph { t2: text("C") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0) -> (t2, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn sink_range_across_separate_lists_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { paragraph { t1: text("B") } }
                    }
                    bullet_list {
                        list_item { paragraph { text("C") } }
                        list_item { paragraph { t2: text("D") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0) -> (t2, 1)
        };
        transact_fail!(initial, |tr| sink_list_item(&mut tr));
    }

    #[test]
    fn sink_range_first_has_no_prev_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("A") } }
                        list_item { paragraph { t2: text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0) -> (t2, 1)
        };
        transact_fail!(initial, |tr| sink_list_item(&mut tr));
    }

    #[test]
    fn sink_into_prev_with_different_type_sublist_reuses_existing() {
        // prev_list_item already owns an ordered sublist (a state reachable only
        // via paste/import). Reusing it regardless of type preserves the single-
        // sublist invariant; the sunk item becomes a child of the ordered list.
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            ordered_list { list_item { paragraph { text("a1") } } }
                        }
                        list_item { paragraph { t1: text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| sink_list_item(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            ordered_list {
                                list_item { paragraph { text("a1") } }
                                list_item { paragraph { t1: text("B") } }
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
}
