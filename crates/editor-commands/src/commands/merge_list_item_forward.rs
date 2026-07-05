use editor_crdt::Dot;
use editor_model::{ChildView, NodeType};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::helpers::{find_enclosing_list_item_id, merge_element_cross_parent};
use crate::{CommandError, CommandResult};

pub fn merge_list_item_forward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;
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

    let at_end = pos.node == paragraph_id && pos.offset >= paragraph.children().count();
    if !at_end {
        return Ok(false);
    }

    if list_item.child_blocks().last().map(|b| b.id()) != Some(paragraph_id) {
        return Ok(false);
    }

    let cursor_target = Position {
        node: pos.node,
        offset: pos.offset,
        affinity: Affinity::Downstream,
    };

    let list = list_item
        .parent()
        .ok_or(CommandError::NoParent(list_item_id))?;
    let li_idx = list_item
        .index()
        .ok_or_else(|| CommandError::orphan_child(list_item_id, list.id()))?;
    let next = list.child_blocks().nth(li_idx + 1);

    if let Some(next_li) = next
        && next_li.node_type() == NodeType::ListItem
    {
        let next_id = next_li.id();
        let next_paragraph = match next_li.first_child() {
            Some(ChildView::Block(p)) => p,
            _ => {
                return Err(CommandError::Corrupted(
                    "next list_item missing paragraph".into(),
                ));
            }
        };
        let next_paragraph_id = next_paragraph.id();

        let target_sublist_id: Option<Dot> = list_item
            .child_blocks()
            .find(|b| matches!(b.node_type(), NodeType::BulletList | NodeType::OrderedList))
            .map(|b| b.id());
        let moved_sublist_id: Option<Dot> = next_li
            .child_blocks()
            .find(|b| matches!(b.node_type(), NodeType::BulletList | NodeType::OrderedList))
            .map(|b| b.id());

        drop(view);

        tr.batch::<_, CommandError>(|tr| {
            if let Some(moved_id) = &moved_sublist_id {
                let cur_len = {
                    let view = tr.view();
                    view.node(list_item_id)
                        .ok_or(CommandError::NodeNotFound(list_item_id))?
                        .child_blocks()
                        .count()
                };
                tr.move_node(*moved_id, list_item_id, cur_len)?;
            }
            if let (Some(target), Some(_)) = (&target_sublist_id, &moved_sublist_id) {
                tr.merge_node(*target)?;
            }
            merge_element_cross_parent(tr, next_paragraph_id, paragraph_id)?;
            tr.remove_subtree(next_id)?;

            let view = tr.view();
            if let Some(current) = view.node(list_item_id) {
                tr.apply_steps(fulfill(&current))?;
            }
            Ok(())
        })?;

        tr.set_selection(Some(Selection::collapsed(cursor_target)))?;
        return Ok(true);
    }

    let list_parent = list.parent().ok_or(CommandError::NoParent(list.id()))?;
    let list_idx = list
        .index()
        .ok_or_else(|| CommandError::orphan_child(list.id(), list_parent.id()))?;
    let next_block = match list_parent.child_at(list_idx + 1) {
        Some(ChildView::Block(b)) if b.node_type() == NodeType::Paragraph => b,
        _ => return Ok(false),
    };
    let next_block_id = next_block.id();
    let list_parent_id = list_parent.id();

    let trailing_page_break_offset: Option<usize> = match next_block.last_child() {
        Some(ChildView::Leaf(l)) if l.node_type() == NodeType::PageBreak => {
            Some(next_block.children().count() - 1)
        }
        _ => None,
    };

    drop(view);

    tr.batch::<_, CommandError>(|tr| {
        if let Some(pb_offset) = trailing_page_break_offset {
            tr.remove_child_slots(next_block_id, pb_offset, pb_offset + 1)?;
        }
        merge_element_cross_parent(tr, next_block_id, paragraph_id)?;
        let view = tr.view();
        if let Some(list_parent) = view.node(list_parent_id) {
            tr.apply_steps(fulfill(&list_parent))?;
        }
        Ok(())
    })?;

    tr.set_selection(Some(Selection::collapsed(cursor_target)))?;
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
                        list_item { p1: paragraph { text("HelloWorld") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 5)
        };
        assert_state_eq!(&actual, &expected);
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
    fn last_item_pulls_next_paragraph() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                    }
                    paragraph { text("B") }
                }
            }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| merge_list_item_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("AB") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn last_item_pulls_next_paragraph_strips_trailing_page_break() {
        // Root-level paragraph may carry a trailing PageBreak, but pulling it
        // into a list_item paragraph would violate PageBreak's `Root > Paragraph`
        // context, so the marker must be stripped before merge.
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                    }
                    paragraph {
                        text("B")
                        page_break
                    }
                }
            }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| merge_list_item_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("AB") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn last_item_next_block_not_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { p1: paragraph { text("A") } } }
                    horizontal_rule
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
