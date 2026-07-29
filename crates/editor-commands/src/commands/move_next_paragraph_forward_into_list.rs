use editor_model::{ChildView, NodeType, Subtree};
use editor_state::Selection;
use editor_transaction::{Transaction, fulfill};

use crate::helpers::{ForwardListBoundary, find_forward_list_boundary};
use crate::{CommandError, CommandResult};

pub fn move_next_paragraph_forward_into_list(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let position = selection.head;
    let (list_id, list_len, source_id, source_parent_id, trailing_page_break_offset) = {
        let view = tr.view();
        let Some(ForwardListBoundary::NextBlock { list_id, next_id }) =
            find_forward_list_boundary(&view, position)?
        else {
            return Ok(false);
        };
        let list = view
            .node(list_id)
            .ok_or(CommandError::NodeNotFound(list_id))?;
        let source = view
            .node(next_id)
            .ok_or(CommandError::NodeNotFound(next_id))?;
        if source.node_type() != NodeType::Paragraph || source.dot().is_none() {
            return Ok(false);
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
            list_id,
            list.child_blocks()
                .filter(|child| child.dot().is_some())
                .count(),
            source.id(),
            source_parent_id,
            trailing_page_break_offset,
        )
    };

    tr.batch::<_, CommandError>(|tr| {
        if let Some(offset) = trailing_page_break_offset {
            tr.remove_child_slots(source_id, offset, offset + 1)?;
        }
        tr.insert_subtree_with_moved(
            list_id,
            list_len,
            Subtree::leaf(NodeType::ListItem.into_node().to_plain()),
            &[source_id],
        )?;
        let steps = {
            let view = tr.view();
            view.node(source_parent_id)
                .map(|parent| fulfill(&parent))
                .unwrap_or_default()
        };
        tr.apply_steps(steps)?;
        tr.set_selection(Some(Selection::collapsed(position)))?;
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
    fn next_paragraph_becomes_a_new_item_in_the_trailing_list() {
        let (initial, ..) = state! {
            doc { root {
                bullet_list { list_item { p1: paragraph { text("A") } } }
                paragraph { text("B") }
            } }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| move_next_paragraph_forward_into_list(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { p1: paragraph { text("A") } }
                    list_item { paragraph { text("B") } }
                }
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
        assert_projection_integrity(&actual);
    }

    #[test]
    fn deepest_trailing_caret_moves_paragraph_into_outermost_available_list() {
        let (initial, ..) = state! {
            doc { root {
                ordered_list {
                    list_item {
                        paragraph { text("A") }
                        bullet_list {
                            list_item { p1: paragraph { text("B") } }
                        }
                    }
                }
                paragraph { text("C") }
            } }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| move_next_paragraph_forward_into_list(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                ordered_list {
                    list_item {
                        paragraph { text("A") }
                        bullet_list {
                            list_item { p1: paragraph { text("B") } }
                        }
                    }
                    list_item { paragraph { text("C") } }
                }
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
        assert_projection_integrity(&actual);
    }

    #[test]
    fn moving_root_paragraph_into_list_strips_trailing_page_break() {
        let (initial, ..) = state! {
            doc { root {
                bullet_list { list_item { p1: paragraph { text("A") } } }
                paragraph {
                    text("B")
                    page_break
                }
            } }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| move_next_paragraph_forward_into_list(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { p1: paragraph { text("A") } }
                    list_item { paragraph { text("B") } }
                }
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
        assert_projection_integrity(&actual);
    }
}
