use editor_model::{Node, NodeId};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, compact, fulfill};

use crate::helpers::{find_enclosing_list_item_id, merge_element_cross_parent};
use crate::{CommandError, CommandResult};

pub fn merge_list_item_backward(tr: &mut Transaction) -> CommandResult {
    let selection = tr.selection();
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let pos = selection.head;
    if pos.offset != 0 {
        return Ok(false);
    }

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

    // Only fire when the cursor is anchored at the very start of the list_item's
    // paragraph — either at offset 0 of the paragraph itself, or at offset 0 of
    // its first inline child.
    match node.node() {
        Node::Text(_) => {
            if node.prev_sibling().is_some() {
                return Ok(false);
            }
            let parent = node.parent().ok_or(CommandError::NoParent(pos.node_id))?;
            if parent.id() != paragraph_id {
                return Ok(false);
            }
        }
        Node::Paragraph(_) => {
            if node.id() != paragraph_id {
                return Ok(false);
            }
        }
        _ => return Ok(false),
    }

    let prev_list_item = match list_item.prev_sibling() {
        Some(p) => p,
        None => return Ok(false),
    };
    let prev_id = prev_list_item.id();
    let prev_paragraph = prev_list_item.first_child().ok_or(CommandError::Corrupted(
        "prev list_item missing paragraph".into(),
    ))?;
    let prev_paragraph_id = prev_paragraph.id();

    // A list_item's content shape is `Paragraph, (BulletList|OrderedList)?`, so
    // locate any trailing sublist by node type rather than fixed index — batch
    // operations may shift indices but the type predicate stays valid.
    let target_sublist_id: Option<NodeId> = prev_list_item
        .children()
        .find(|c| matches!(c.node(), Node::BulletList(_) | Node::OrderedList(_)))
        .map(|c| c.id());
    let moved_sublist_id: Option<NodeId> = list_item
        .children()
        .find(|c| matches!(c.node(), Node::BulletList(_) | Node::OrderedList(_)))
        .map(|c| c.id());

    // Capture the join point (end of prev_paragraph's content) before mutation
    // so the post-merge cursor lands where prev's text ended.
    let join_cursor = match prev_paragraph.last_child() {
        Some(child) => match child.node() {
            Node::Text(t) => Position {
                node_id: child.id(),
                offset: t.text.len(),
                affinity: Affinity::Upstream,
            },
            _ => Position {
                node_id: prev_paragraph_id,
                offset: prev_paragraph.entry().children.len(),
                affinity: Affinity::Upstream,
            },
        },
        None => Position {
            node_id: prev_paragraph_id,
            offset: 0,
            affinity: Affinity::Upstream,
        },
    };

    tr.batch::<_, CommandError>(|tr| {
        // Move current's sublist to the end of prev before merging paragraphs,
        // so the sublist survives the list_item removal.
        if let Some(moved_id) = moved_sublist_id {
            let doc = tr.doc();
            let prev = doc
                .node(prev_id)
                .ok_or(CommandError::NodeNotFound(prev_id))?;
            let prev_len = prev.entry().children.len();
            tr.move_node(moved_id, prev_id, prev_len)?;
        }

        // When both sides carried a sublist, fold the moved one into prev's
        // existing sublist. merge_node preserves the target's list type and
        // moves the source's list_items in; ListItem has no inner type variant
        // so this is lossless even across BulletList/OrderedList boundaries.
        if let (Some(target), Some(moved)) = (target_sublist_id, moved_sublist_id) {
            tr.merge_node(moved, target)?;
        }

        merge_element_cross_parent(tr, paragraph_id, prev_paragraph_id)?;
        tr.remove_subtree(list_item_id)?;

        // Adjacent text runs with matching modifiers were brought together by
        // the merge; compact stitches them into a single node.
        let doc = tr.doc();
        if let Some(p) = doc.node(prev_paragraph_id) {
            tr.apply_steps(compact(&p))?;
        }

        let doc = tr.doc();
        if let Some(prev) = doc.node(prev_id) {
            tr.apply_steps(fulfill(&prev))?;
        }
        Ok(())
    })?;

    tr.set_selection(Selection::collapsed(join_cursor))?;
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
                        list_item { paragraph { t1: text("Hello") } }
                        list_item { paragraph { t2: text("World") } }
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
                        list_item { paragraph { t2: text("B") } }
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
                    bullet_list { list_item { paragraph { t1: text("A") } } }
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
                        list_item { paragraph { t1: text("B") } }
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
                    paragraph { t1: text("B") }
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
                            paragraph { t_a: text("A") }
                            bullet_list { list_item { paragraph { text("a1") } } }
                        }
                        list_item { paragraph { t2: text("B") } }
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
                            paragraph { t_a: text("AB") }
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
                        list_item { paragraph { t_a: text("A") } }
                        list_item {
                            paragraph { t2: text("B") }
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
                            paragraph { t_a: text("AB") }
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
                            paragraph { t_a: text("A") }
                            bullet_list { list_item { paragraph { text("a1") } } }
                        }
                        list_item {
                            paragraph { t2: text("B") }
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
                            paragraph { t_a: text("AB") }
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
    fn merge_empty_current_into_prev() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { t_a: text("A") } }
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
                        list_item { paragraph { t_a: text("A") } }
                    }
                    paragraph {}
                }
            }
            selection: (t_a, 1)
        };
        assert_state_eq!(&actual, &expected);
    }
}
