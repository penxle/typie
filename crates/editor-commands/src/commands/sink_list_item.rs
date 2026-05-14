use editor_model::{Node, NodeId, NodeType, Subtree};
use editor_state::Position;
use editor_transaction::{Transaction, fulfill};

use crate::helpers::{collect_top_level_list_items_in_selection, find_enclosing_list_item_id};
use crate::{CommandError, CommandResult};

pub fn sink_list_item(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection();
    let doc = tr.doc();

    if selection.is_collapsed() {
        let pos = selection.head;
        let Some(list_item_id) = find_enclosing_list_item_id(&doc, pos.node_id) else {
            return Ok(false);
        };
        return sink_single(tr, list_item_id);
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
        if sink_single(tr, *item_id)? {
            any_sunk = true;
        }
    }
    Ok(any_sunk)
}

fn sink_single(tr: &mut Transaction, list_item_id: NodeId) -> CommandResult {
    let doc = tr.doc();
    let list_item = doc
        .node(list_item_id)
        .ok_or(CommandError::NodeNotFound(list_item_id))?;
    if !matches!(list_item.node(), Node::ListItem(_)) {
        return Ok(false);
    }

    let prev = match list_item.prev_sibling() {
        Some(p) => p,
        None => return Ok(false),
    };
    let prev_id = prev.id();

    let list = list_item
        .parent()
        .ok_or(CommandError::NoParent(list_item_id))?;
    let list_type = list.as_type();
    if !matches!(list_type, NodeType::BulletList | NodeType::OrderedList) {
        return Ok(false);
    }

    // A list_item allows at most one trailing sublist. Reuse any existing one
    // regardless of its type — creating a second sublist would violate the
    // schema, so type-matching can't be enforced here.
    let target_sublist_id = prev
        .children()
        .find(|c| matches!(c.node(), Node::BulletList(_) | Node::OrderedList(_)))
        .map(|c| c.id());

    tr.batch::<_, CommandError>(|tr| {
        let target_id = match target_sublist_id {
            Some(id) => id,
            None => {
                let new_sublist_id = NodeId::new();
                let new_node = list_type.into_node().to_plain();
                let doc = tr.doc();
                let prev = doc
                    .node(prev_id)
                    .ok_or(CommandError::NodeNotFound(prev_id))?;
                let insert_at = prev.entry().children.len();
                tr.insert_subtree(prev_id, insert_at, Subtree::leaf(new_sublist_id, new_node))?;
                new_sublist_id
            }
        };

        let doc = tr.doc();
        let target = doc
            .node(target_id)
            .ok_or(CommandError::NodeNotFound(target_id))?;
        let target_len = target.entry().children.len();
        tr.move_node(list_item_id, target_id, target_len)?;

        let doc = tr.doc();
        if let Some(prev) = doc.node(prev_id) {
            tr.apply_steps(fulfill(&prev))?;
        }
        Ok(())
    })?;

    Ok(true)
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
