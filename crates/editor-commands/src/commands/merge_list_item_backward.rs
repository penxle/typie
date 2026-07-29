use editor_model::{ChildView, NodeType};
use editor_state::{Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::helpers::find_enclosing_list_item_id;
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
    let first_paragraph = list_item
        .child_blocks()
        .next()
        .ok_or_else(|| CommandError::Corrupted("list_item missing first paragraph".into()))?;
    if first_paragraph.node_type() != NodeType::Paragraph
        || pos.node != first_paragraph.id()
        || pos.offset != 0
    {
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
    if prev.node_type() != NodeType::ListItem {
        return Err(CommandError::Corrupted(
            "list contains non-list_item child".into(),
        ));
    }
    let prev_id = prev.id();
    let prev_len = prev.child_blocks().count();
    if prev
        .children()
        .chain(list_item.children())
        .any(|child| matches!(child, ChildView::Leaf(_)))
    {
        return Err(CommandError::Corrupted(
            "list item contains an unsupported direct child".into(),
        ));
    }
    let children = list_item
        .child_blocks()
        .map(|child| child.id())
        .collect::<Vec<_>>();
    if children.is_empty() {
        return Err(CommandError::Corrupted(
            "list_item missing direct children".into(),
        ));
    }

    drop(view);

    tr.batch::<_, CommandError>(|tr| {
        let moved = tr.move_nodes_consecutive(&children, prev_id, prev_len)?;
        let first_moved = moved
            .first()
            .map(|moved| moved.root)
            .ok_or(CommandError::Corrupted(
                "list item merge moved no children".into(),
            ))?;
        tr.remove_subtree(list_item_id)?;

        let view = tr.view();
        if let Some(prev) = view.node(prev_id) {
            tr.apply_steps(fulfill(&prev))?;
        }
        tr.set_selection(Some(Selection::collapsed(Position::new(first_moved, 0))))?;
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
                        list_item {
                            paragraph { text("Hello") }
                            t2: paragraph { text("World") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
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
                            paragraph { text("A") }
                            bullet_list { list_item { paragraph { text("a1") } } }
                            t2: paragraph { text("B") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
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
                            paragraph { text("A") }
                            t2: paragraph { text("B") }
                            bullet_list { list_item { paragraph { text("b1") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
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
                            paragraph { text("A") }
                            bullet_list {
                                list_item { paragraph { text("a1") } }
                            }
                            t2: paragraph { text("B") }
                            bullet_list { list_item { paragraph { text("b1") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
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
                        list_item {
                            paragraph { text("A") }
                            t2: paragraph { text("B") [bold] }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
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
                        list_item {
                            paragraph { text("A") }
                            t2: paragraph { text("B") tab text("C") hard_break }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn merge_empty_current_into_prev() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
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
                        list_item {
                            paragraph { text("A") }
                            p2: paragraph {}
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
