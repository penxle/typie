use editor_model::{ChildView, NodeType, Subtree};
use editor_state::{Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::helpers::{is_list_type, prev_sibling};
use crate::{CommandError, CommandResult};

pub fn move_paragraph_backward_into_prev_list(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() || selection.head.offset != 0 {
        return Ok(false);
    }

    let position = selection.head;
    let (source_id, source_parent_id, list_id, list_len, trailing_page_break_offset) = {
        let view = tr.view();
        let source = view
            .node(position.node)
            .ok_or(CommandError::NodeNotFound(position.node))?;
        if source.node_type() != NodeType::Paragraph || source.dot().is_none() {
            return Ok(false);
        }
        let Some(ChildView::Block(list)) = prev_sibling(&source) else {
            return Ok(false);
        };
        if !is_list_type(list.node_type()) {
            return Ok(false);
        }
        if list
            .children()
            .any(|child| matches!(child, ChildView::Leaf(_)))
        {
            return Err(CommandError::Corrupted(
                "list contains an unsupported direct child".into(),
            ));
        }

        let source_parent_id = source
            .parent()
            .ok_or(CommandError::NoParent(source.id()))?
            .id();
        let trailing_page_break_offset = match source.last_child() {
            Some(ChildView::Leaf(leaf)) if leaf.node_type() == NodeType::PageBreak => {
                Some(source.children().count() - 1)
            }
            _ => None,
        };
        (
            source.id(),
            source_parent_id,
            list.id(),
            list.child_blocks()
                .filter(|child| child.dot().is_some())
                .count(),
            trailing_page_break_offset,
        )
    };

    tr.batch::<_, CommandError>(|tr| {
        if let Some(offset) = trailing_page_break_offset {
            tr.remove_child_slots(source_id, offset, offset + 1)?;
        }
        let (_, moved) = tr.insert_subtree_with_moved(
            list_id,
            list_len,
            Subtree::leaf(NodeType::ListItem.into_node().to_plain()),
            &[source_id],
        )?;
        let moved_source = moved
            .first()
            .map(|moved| moved.root)
            .ok_or(CommandError::Corrupted(
                "moving paragraph into list moved no paragraph".into(),
            ))?;
        let steps = {
            let view = tr.view();
            view.node(source_parent_id)
                .map(|parent| fulfill(&parent))
                .unwrap_or_default()
        };
        tr.apply_steps(steps)?;
        tr.set_selection(Some(Selection::collapsed(Position {
            node: moved_source,
            offset: position.offset,
            affinity: position.affinity,
        })))?;
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
    fn paragraph_after_list_becomes_a_new_list_item() {
        let (initial, ..) = state! {
            doc { root {
                bullet_list { list_item { paragraph { text("A") } } }
                p2: paragraph { text("B") }
            } }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| move_paragraph_backward_into_prev_list(
            &mut tr
        ));
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { paragraph { text("A") } }
                    list_item { p2: paragraph { text("B") } }
                }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_projection_integrity(&actual);
    }

    #[test]
    fn paragraph_after_nested_list_enters_that_list_before_its_previous_item() {
        let (initial, ..) = state! {
            doc { root {
                bullet_list {
                    list_item {
                        paragraph { text("A") }
                        ordered_list {
                            list_item { paragraph { text("B") } }
                        }
                        p2: paragraph { text("C") }
                    }
                }
            } }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| move_paragraph_backward_into_prev_list(
            &mut tr
        ));
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item {
                        paragraph { text("A") }
                        ordered_list {
                            list_item { paragraph { text("B") } }
                            list_item { p2: paragraph { text("C") } }
                        }
                    }
                }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_projection_integrity(&actual);
    }

    #[test]
    fn moving_root_paragraph_into_list_strips_trailing_page_break() {
        let (initial, ..) = state! {
            doc { root {
                bullet_list { list_item { paragraph { text("A") } } }
                p2: paragraph {
                    text("B")
                    page_break
                }
            } }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| move_paragraph_backward_into_prev_list(
            &mut tr
        ));
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { paragraph { text("A") } }
                    list_item { p2: paragraph { text("B") } }
                }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_projection_integrity(&actual);
    }
}
