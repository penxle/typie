use editor_model::{Node, NodeId};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, compact, fulfill};

use crate::helpers::{find_enclosing_list_item_id, merge_element_cross_parent};
use crate::{CommandError, CommandResult};

pub fn merge_list_item_forward(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection();
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    let doc = tr.doc();
    let node = doc
        .node(pos.node_id)
        .ok_or(CommandError::NodeNotFound(pos.node_id))?;

    let list_item_id = match find_enclosing_list_item_id(&doc, pos.node_id) {
        Some(id) => id,
        None => return Ok(false),
    };

    let list_item = doc
        .node(list_item_id)
        .ok_or(CommandError::NodeNotFound(list_item_id))?;
    let paragraph = list_item.first_child().ok_or(CommandError::Corrupted(
        "list_item missing paragraph".into(),
    ))?;
    let paragraph_id = paragraph.id();

    // Only fire when the cursor sits at the end of the list_item's paragraph —
    // either at the tail of its last text child or at the paragraph's end offset.
    let at_end = match node.node() {
        Node::Text(t) => {
            if node.next_sibling().is_some() {
                return Ok(false);
            }
            let parent = node.parent().ok_or(CommandError::NoParent(pos.node_id))?;
            if parent.id() != paragraph_id {
                return Ok(false);
            }
            pos.offset >= t.text.len()
        }
        Node::Paragraph(_) => {
            if node.id() != paragraph_id {
                return Ok(false);
            }
            pos.offset >= paragraph.entry().children.len()
        }
        _ => return Ok(false),
    };
    if !at_end {
        return Ok(false);
    }

    if list_item.last_child().map(|c| c.id()) != Some(paragraph_id) {
        return Ok(false);
    }

    // Cursor stays at the same offset of the same text/paragraph node — the
    // join appends content after this position, so the offset is preserved.
    let cursor_target = Position {
        node_id: pos.node_id,
        offset: pos.offset,
        affinity: Affinity::Upstream,
    };

    if let Some(next_li) = list_item.next_sibling()
        && matches!(next_li.node(), Node::ListItem(_))
    {
        let next_id = next_li.id();
        let next_paragraph = next_li.first_child().ok_or(CommandError::Corrupted(
            "next list_item missing paragraph".into(),
        ))?;
        let next_paragraph_id = next_paragraph.id();

        // A list_item's shape is `Paragraph, (BulletList|OrderedList)?`. Use a
        // type predicate rather than a fixed index so batch operations that
        // shift indices don't invalidate the lookup.
        let target_sublist_id: Option<NodeId> = list_item
            .children()
            .find(|c| matches!(c.node(), Node::BulletList(_) | Node::OrderedList(_)))
            .map(|c| c.id());
        let moved_sublist_id: Option<NodeId> = next_li
            .children()
            .find(|c| matches!(c.node(), Node::BulletList(_) | Node::OrderedList(_)))
            .map(|c| c.id());

        tr.batch::<_, CommandError>(|tr| {
            // Hoist next's sublist before removing next; otherwise the subtree
            // removal would take it down with the list_item.
            if let Some(moved_id) = moved_sublist_id {
                let doc = tr.doc();
                let current = doc
                    .node(list_item_id)
                    .ok_or(CommandError::NodeNotFound(list_item_id))?;
                let cur_len = current.entry().children.len();
                tr.move_node(moved_id, list_item_id, cur_len)?;
            }
            // When both sides carried a sublist, fold the moved one into the
            // existing one. merge_node preserves target's list type; ListItem
            // has no inner type variant so this is lossless even across
            // BulletList/OrderedList boundaries.
            if let (Some(target), Some(moved)) = (target_sublist_id, moved_sublist_id) {
                tr.merge_node(moved, target)?;
            }
            merge_element_cross_parent(tr, next_paragraph_id, paragraph_id)?;
            tr.remove_subtree(next_id)?;
            // Adjacent text runs with matching modifiers were brought together
            // by the merge; compact stitches them into a single node.
            let doc = tr.doc();
            if let Some(p) = doc.node(paragraph_id) {
                tr.apply_steps(compact(&p))?;
            }
            let doc = tr.doc();
            if let Some(current) = doc.node(list_item_id) {
                tr.apply_steps(fulfill(&current))?;
            }
            Ok(())
        })?;

        tr.set_selection(Selection::collapsed(cursor_target))?;
        return Ok(true);
    }

    let list = list_item
        .parent()
        .ok_or(CommandError::NoParent(list_item_id))?;
    let next_block = match list.next_sibling() {
        Some(b) => b,
        None => return Ok(false),
    };

    if !matches!(next_block.node(), Node::Paragraph(_)) {
        return Ok(false);
    }
    let next_block_id = next_block.id();
    let list_parent_id = list.parent().ok_or(CommandError::NoParent(list.id()))?.id();

    // Root-level paragraphs may end with a PageBreak (context
    // `Root > Paragraph > &`). The destination paragraph sits at
    // `Root > BulletList > ListItem > Paragraph` where PageBreak is not
    // permitted, so strip the marker before merging to avoid a context
    // validation failure.
    let trailing_page_break_id: Option<NodeId> = next_block
        .last_child()
        .filter(|c| matches!(c.node(), Node::PageBreak(_)))
        .map(|c| c.id());

    tr.batch::<_, CommandError>(|tr| {
        if let Some(pb_id) = trailing_page_break_id {
            tr.remove_subtree(pb_id)?;
        }
        merge_element_cross_parent(tr, next_block_id, paragraph_id)?;
        let doc = tr.doc();
        if let Some(p) = doc.node(paragraph_id) {
            tr.apply_steps(compact(&p))?;
        }
        let doc = tr.doc();
        if let Some(list_parent) = doc.node(list_parent_id) {
            tr.apply_steps(fulfill(&list_parent))?;
        }
        Ok(())
    })?;

    tr.set_selection(Selection::collapsed(cursor_target))?;
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
                        list_item { paragraph { t1: text("Hello") } }
                        list_item { paragraph { text("World") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 5)
        };
        let (actual, ..) = transact!(initial, |tr| merge_list_item_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("HelloWorld") } }
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
                        list_item { paragraph { t1: text("A") } }
                        list_item { paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 0) -> (t1, 1)
        };
        transact_fail!(initial, |tr| merge_list_item_forward(&mut tr));
    }

    #[test]
    fn not_at_end_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("Hello") } }
                        list_item { paragraph { text("World") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 3)
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
                            paragraph { t1: text("A") }
                            bullet_list { list_item { paragraph { text("a1") } } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 1)
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
                            paragraph { t1: text("A") }
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
            selection: (t1, 1)
        };
        transact_fail!(initial, |tr| merge_list_item_forward(&mut tr));
    }

    #[test]
    fn last_item_pulls_next_paragraph() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("A") } }
                    }
                    paragraph { text("B") }
                }
            }
            selection: (t1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| merge_list_item_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("AB") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 1)
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
                        list_item { paragraph { t1: text("A") } }
                    }
                    paragraph {
                        text("B")
                        page_break
                    }
                }
            }
            selection: (t1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| merge_list_item_forward(&mut tr));
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t1: text("AB") } }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn last_item_next_block_not_paragraph_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list { list_item { paragraph { t1: text("A") } } }
                    horizontal_rule
                    paragraph {}
                }
            }
            selection: (t1, 1)
        };
        transact_fail!(initial, |tr| merge_list_item_forward(&mut tr));
    }

    #[test]
    fn outside_list_returns_false() {
        let (initial, ..) = state! {
            doc { root { paragraph { t1: text("A") } } }
            selection: (t1, 1)
        };
        transact_fail!(initial, |tr| merge_list_item_forward(&mut tr));
    }
}
