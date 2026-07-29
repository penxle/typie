use editor_model::ChildView;
use editor_state::Selection;
use editor_transaction::{Transaction, fulfill};

use crate::helpers::{ForwardListBoundary, find_forward_list_boundary};
use crate::{CommandError, CommandResult};

pub fn merge_list_item_forward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;
    let cursor_target = pos;

    let (list_item_id, next_id, children, current_len) = {
        let view = tr.view();
        let Some(ForwardListBoundary::NextListItem {
            current_item_id,
            next_item_id,
        }) = find_forward_list_boundary(&view, pos)?
        else {
            return Ok(false);
        };
        let current = view
            .node(current_item_id)
            .ok_or(CommandError::NodeNotFound(current_item_id))?;
        let next = view
            .node(next_item_id)
            .ok_or(CommandError::NodeNotFound(next_item_id))?;
        if current
            .children()
            .chain(next.children())
            .any(|child| matches!(child, ChildView::Leaf(_)))
        {
            return Err(CommandError::Corrupted(
                "list item contains an unsupported direct child".into(),
            ));
        }
        let children = next
            .child_blocks()
            .map(|child| child.id())
            .collect::<Vec<_>>();
        if children.is_empty() {
            return Err(CommandError::Corrupted(
                "next list_item missing direct children".into(),
            ));
        }
        (
            current_item_id,
            next_item_id,
            children,
            current.child_blocks().count(),
        )
    };

    tr.batch::<_, CommandError>(|tr| {
        tr.move_nodes_consecutive(&children, list_item_id, current_len)?;
        tr.remove_subtree(next_id)?;

        let view = tr.view();
        if let Some(current) = view.node(list_item_id) {
            tr.apply_steps(fulfill(&current))?;
        }
        tr.set_selection(Some(Selection::collapsed(cursor_target)))?;
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
    fn merge_two_text_items_forward() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                        list_item { paragraph { text("World") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| merge_list_item_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("Hello") }
                            paragraph { text("World") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn merge_next_item_moves_all_direct_children_in_order() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                        list_item {
                            paragraph { text("B") }
                            ordered_list { list_item { paragraph { text("nested") } } }
                            paragraph { text("C") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| merge_list_item_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("A") }
                            paragraph { text("B") }
                            ordered_list { list_item { paragraph { text("nested") } } }
                            paragraph { text("C") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
        assert_projection_integrity(&actual);
    }

    #[test]
    fn non_collapsed_returns_false() {
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
            selection: (p1, 0) -> (p1, 1)
        };
        transact_fail!(initial, |tr| merge_list_item_forward(&mut tr));
    }

    #[test]
    fn not_at_end_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("Hello") } }
                        list_item { paragraph { text("World") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 3)
        };
        transact_fail!(initial, |tr| merge_list_item_forward(&mut tr));
    }

    #[test]
    fn has_nested_sublist_returns_false() {
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
            selection: (p1, 1)
        };
        transact_fail!(initial, |tr| merge_list_item_forward(&mut tr));
    }

    #[test]
    fn merge_with_sublists_combined() {
        // Case A precondition: paragraph must be last child of list_item.
        // Here the current list_item has a sublist after the paragraph, so the
        // command bails out and lift_paragraph_forward handles the case.
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            p1: paragraph { text("A") }
                            bullet_list { list_item { paragraph { text("a1") } } }
                        }
                        list_item {
                            paragraph { text("B") }
                            bullet_list { list_item { paragraph { text("b1") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        transact_fail!(initial, |tr| merge_list_item_forward(&mut tr));
    }

    #[test]
    fn outside_list_returns_false() {
        let (initial, ..) = state! {
            doc { root { p1: paragraph { text("A") } } }
            selection: (p1, 1)
        };
        transact_fail!(initial, |tr| merge_list_item_forward(&mut tr));
    }
}
