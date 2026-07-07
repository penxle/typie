use editor_crdt::Dot;
use editor_model::{ChildView, NodeType};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::helpers::{find_enclosing_list_item_id, merge_element_cross_parent};
use crate::{CommandError, CommandResult};

pub fn merge_list_item_backward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;
    if pos.offset != 0 {
        return Ok(false);
    }

    let view = tr.view();

    let list_item_id = match find_enclosing_list_item_id(&view, pos.node) {
        Some(id) => id,
        None => return Ok(false),
    };

    let list_item = view
        .node(list_item_id)
        .ok_or(CommandError::NodeNotFound(list_item_id))?;
    let paragraph = match list_item.first_child() {
        Some(ChildView::Block(p)) => p,
        _ => {
            return Err(CommandError::Corrupted(
                "list_item missing paragraph".into(),
            ));
        }
    };
    let paragraph_id = paragraph.id();

    if pos.node != paragraph_id || pos.offset != 0 {
        return Ok(false);
    }

    let list = list_item
        .parent()
        .ok_or(CommandError::NoParent(list_item_id))?;
    let li_idx = list_item
        .index()
        .ok_or_else(|| CommandError::orphan_child(list_item_id, list.id()))?;
    if li_idx == 0 {
        return Ok(false);
    }
    let prev = list
        .child_blocks()
        .nth(li_idx - 1)
        .ok_or(CommandError::Corrupted("prev list_item missing".into()))?;
    let prev_id = prev.id();
    let prev_paragraph = match prev.first_child() {
        Some(ChildView::Block(p)) => p,
        _ => {
            return Err(CommandError::Corrupted(
                "prev list_item missing paragraph".into(),
            ));
        }
    };
    let prev_paragraph_id = prev_paragraph.id();

    let target_sublist_id: Option<Dot> = prev
        .child_blocks()
        .find(|b| matches!(b.node_type(), NodeType::BulletList | NodeType::OrderedList))
        .map(|b| b.id());
    let moved_sublist_id: Option<Dot> = list_item
        .child_blocks()
        .find(|b| matches!(b.node_type(), NodeType::BulletList | NodeType::OrderedList))
        .map(|b| b.id());

    let join_cursor = Position {
        node: prev_paragraph_id,
        offset: prev_paragraph.children().count(),
        affinity: Affinity::Downstream,
    };

    drop(view);

    tr.batch::<_, CommandError>(|tr| {
        match (&target_sublist_id, &moved_sublist_id) {
            (Some(target), Some(moved)) => {
                let items: Vec<Dot> = {
                    let view = tr.view();
                    view.node(*moved)
                        .map(|m| m.child_blocks().map(|b| b.id()).collect())
                        .unwrap_or_default()
                };
                let base = {
                    let view = tr.view();
                    view.node(*target)
                        .ok_or(CommandError::NodeNotFound(*target))?
                        .child_blocks()
                        .count()
                };
                for (i, item) in items.into_iter().enumerate() {
                    tr.move_node(item, *target, base + i)?;
                }
            }
            (None, Some(moved)) => {
                let prev_len = {
                    let view = tr.view();
                    view.node(prev_id)
                        .ok_or(CommandError::NodeNotFound(prev_id))?
                        .child_blocks()
                        .count()
                };
                tr.move_node(*moved, prev_id, prev_len)?;
            }
            _ => {}
        }

        merge_element_cross_parent(tr, paragraph_id, prev_paragraph_id)?;
        tr.remove_subtree(list_item_id)?;

        let view = tr.view();
        if let Some(prev) = view.node(prev_id) {
            tr.apply_steps(fulfill(&prev))?;
        }
        Ok(())
    })?;

    tr.set_selection(Some(Selection::collapsed(join_cursor)))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn merge_two_text_items() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { t1: paragraph { text("Hello") } }
                        list_item { t2: paragraph { text("World") } }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| merge_list_item_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { t1: paragraph { text("HelloWorld") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 5)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn non_collapsed_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { t1: paragraph { text("A") } }
                        list_item { t2: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0) -> (t2, 1)
        };
        transact_fail!(initial, |tr| merge_list_item_backward(&mut tr));
    }

    #[test]
    fn no_prev_item_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { t1: paragraph { text("A") } } }
                    paragraph {}
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| merge_list_item_backward(&mut tr));
    }

    #[test]
    fn not_at_start_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                        list_item { t1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 1)
        };
        transact_fail!(initial, |tr| merge_list_item_backward(&mut tr));
    }

    #[test]
    fn outside_list_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("A") }
                    t1: paragraph { text("B") }
                }
            }
            selection: (t1, 0)
        };
        transact_fail!(initial, |tr| merge_list_item_backward(&mut tr));
    }

    #[test]
    fn merge_prev_has_sublist() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            t_a: paragraph { text("A") }
                            bullet_list { list_item { paragraph { text("a1") } } }
                        }
                        list_item { t2: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| merge_list_item_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            t_a: paragraph { text("AB") }
                            bullet_list { list_item { paragraph { text("a1") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t_a, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn merge_current_has_sublist() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { t_a: paragraph { text("A") } }
                        list_item {
                            t2: paragraph { text("B") }
                            bullet_list { list_item { paragraph { text("b1") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| merge_list_item_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            t_a: paragraph { text("AB") }
                            bullet_list { list_item { paragraph { text("b1") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t_a, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn merge_both_have_sublists() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            t_a: paragraph { text("A") }
                            bullet_list { list_item { paragraph { text("a1") } } }
                        }
                        list_item {
                            t2: paragraph { text("B") }
                            bullet_list { list_item { paragraph { text("b1") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| merge_list_item_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            t_a: paragraph { text("AB") }
                            bullet_list {
                                list_item { paragraph { text("a1") } }
                                list_item { paragraph { text("b1") } }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t_a, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn merge_preserves_merged_side_bold() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { t1: paragraph { text("A") } }
                        list_item { t2: paragraph { text("B") [bold] } }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| merge_list_item_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { t1: paragraph { text("A") text("B") [bold] } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn merge_preserves_merged_side_tab_and_hard_break() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { t1: paragraph { text("A") } }
                        list_item { t2: paragraph { text("B") tab text("C") hard_break } }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| merge_list_item_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { t1: paragraph { text("A") text("B") tab text("C") hard_break } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn merge_empty_current_into_prev() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { t_a: paragraph { text("A") } }
                        list_item { p2: paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| merge_list_item_backward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { t_a: paragraph { text("A") } }
                    }
                    paragraph {}
                }
            }
            selection: (t_a, 1)
        };
        assert_state_eq!(&actual, &expected);
    }
}
