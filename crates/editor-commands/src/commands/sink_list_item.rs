use editor_crdt::Dot;
use editor_transaction::Transaction;

use crate::helpers::{
    capture_selection_anchors, collect_top_level_list_items_in_selection,
    restore_selection_anchors, sink_list_item_inner,
};
use crate::{CommandError, CommandResult};

pub fn sink_list_item(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };

    let items = {
        let view = tr.view();
        let resolved = selection
            .resolve(&view)
            .ok_or(CommandError::Corrupted("cannot resolve selection".into()))?;
        let from = resolved.from().position();
        let to = resolved.to().position();
        collect_top_level_list_items_in_selection(&view, from, to)
    };
    if items.is_empty() {
        return Ok(false);
    }

    // Google Docs behavior: if the first item has no prev sibling, the entire
    // range is a no-op rather than partially sinking later items.
    let first_has_prev = {
        let view = tr.view();
        let first = view
            .node(items[0])
            .ok_or_else(|| CommandError::NodeNotFound(items[0]))?;
        first.index().map(|i| i > 0).unwrap_or(false)
    };
    if !first_has_prev {
        return Ok(false);
    }

    let captured = {
        let view = tr.view();
        capture_selection_anchors(&view, &items, &selection)
    };

    let mut new_ids: Vec<Option<Dot>> = Vec::with_capacity(items.len());
    let mut any_sunk = false;
    for item_id in items.iter() {
        let exists = {
            let view = tr.view();
            view.node(*item_id).is_some()
        };
        if !exists {
            new_ids.push(None);
            continue;
        }
        let new_id = sink_list_item_inner(tr, *item_id)?;
        if new_id.is_some() {
            any_sunk = true;
        }
        new_ids.push(new_id);
    }
    if !any_sunk {
        return Ok(false);
    }

    if let Some((anchor, head)) = captured {
        let sel = {
            let view = tr.view();
            restore_selection_anchors(&view, &new_ids, &anchor, &head)
        };
        if let Some(sel) = sel {
            tr.set_selection(Some(sel))?;
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
    fn sink_range_across_separate_lists_returns_false() {
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
        transact_fail!(initial, |tr| sink_list_item(&mut tr));
    }

    #[test]
    fn sink_range_first_has_no_prev_returns_false() {
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
        transact_fail!(initial, |tr| sink_list_item(&mut tr));
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
