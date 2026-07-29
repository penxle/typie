use editor_model::{NodeType, Subtree};
use editor_state::{Position, Selection};
use editor_transaction::Transaction;

use crate::helpers::lift_list_item_inner;
use crate::{CommandError, CommandResult};

pub fn lift_trailing_empty_list_item_paragraph(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() || selection.head.offset != 0 {
        return Ok(false);
    }

    let position = selection.head;
    let (paragraph_id, list_item_id, list_id, item_index) = {
        let view = tr.view();
        let paragraph = view
            .node(position.node)
            .ok_or(CommandError::NodeNotFound(position.node))?;
        if paragraph.node_type() != NodeType::Paragraph
            || paragraph.first_child().is_some()
            || paragraph.dot().is_none()
        {
            return Ok(false);
        }

        let list_item = paragraph
            .parent()
            .ok_or(CommandError::NoParent(paragraph.id()))?;
        if list_item.node_type() != NodeType::ListItem
            || list_item.child_blocks().count() <= 1
            || list_item.child_blocks().last().map(|child| child.id()) != Some(paragraph.id())
        {
            return Ok(false);
        }

        let list = list_item
            .parent()
            .ok_or(CommandError::NoParent(list_item.id()))?;
        let item_index = list
            .child_blocks()
            .position(|child| child.id() == list_item.id())
            .ok_or_else(|| CommandError::orphan_child(list_item.id(), list.id()))?;
        (paragraph.id(), list_item.id(), list.id(), item_index)
    };

    tr.batch::<_, CommandError>(|tr| {
        let (new_item_id, moved) = tr.insert_subtree_with_moved(
            list_id,
            item_index + 1,
            Subtree::leaf(NodeType::ListItem.into_node().to_plain()),
            &[paragraph_id],
        )?;
        let moved_paragraph_id =
            moved
                .first()
                .map(|moved| moved.root)
                .ok_or(CommandError::Corrupted(
                    "moving trailing paragraph into list item moved no paragraph".into(),
                ))?;
        tr.set_selection(Some(Selection::collapsed(Position {
            node: moved_paragraph_id,
            offset: position.offset,
            affinity: position.affinity,
        })))?;

        if !lift_list_item_inner(tr, new_item_id)? {
            return Err(CommandError::Corrupted(format!(
                "cannot lift temporary list item {new_item_id:?} created from {list_item_id:?}"
            )));
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
    fn top_level_trailing_empty_paragraph_lifts_between_split_lists() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            p1: paragraph {}
                        }
                        list_item { paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| {
            lift_trailing_empty_list_item_paragraph(&mut tr)
        });
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { paragraph { text("A") } }
                    }
                    p1: paragraph {}
                    bullet_list {
                        list_item { paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_projection_integrity(&actual);
    }

    #[test]
    fn middle_empty_paragraph_is_not_lifted() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph { text("A") }
                            p1: paragraph {}
                            paragraph { text("B") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| {
            lift_trailing_empty_list_item_paragraph(&mut tr)
        });
    }
}
