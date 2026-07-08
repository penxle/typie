use editor_model::{ChildView, NodeType};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::helpers::{materialize_position_block, merge_element_cross_parent, prev_sibling};
use crate::{CommandError, CommandResult};

pub fn join_paragraph_backward_into_prev_list_item(tr: &mut Transaction) -> CommandResult {
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

    let (source_id, target_id, target_offset, source_parent_id, trailing_page_break_offset) = {
        let view = tr.state().view();
        let source = view
            .node(pos.node)
            .ok_or(CommandError::NodeNotFound(pos.node))?;
        if source.node_type() != NodeType::Paragraph || source.dot().is_none() {
            return Ok(false);
        }

        let prev = match prev_sibling(&source) {
            Some(ChildView::Block(prev))
                if matches!(
                    prev.node_type(),
                    NodeType::BulletList | NodeType::OrderedList
                ) =>
            {
                prev
            }
            _ => return Ok(false),
        };
        let list_item = match prev.child_blocks().last() {
            Some(item) if item.node_type() == NodeType::ListItem => item,
            _ => return Ok(false),
        };
        let target = match list_item.first_child() {
            Some(ChildView::Block(p)) if p.node_type() == NodeType::Paragraph => p,
            _ => {
                return Err(CommandError::Corrupted(
                    "list_item missing paragraph".into(),
                ));
            }
        };
        if list_item.child_blocks().last().map(|b| b.id()) != Some(target.id()) {
            return Ok(false);
        }

        let source_parent_id = source
            .parent()
            .ok_or(CommandError::NoParent(pos.node))?
            .id();
        let source_child_count = source.children().count();
        let trailing_page_break_offset = match source.last_child() {
            Some(ChildView::Leaf(l)) if l.node_type() == NodeType::PageBreak => {
                Some(source_child_count - 1)
            }
            _ => None,
        };

        (
            pos.node,
            target.id(),
            target.children().count(),
            source_parent_id,
            trailing_page_break_offset,
        )
    };

    let target = materialize_position_block(
        tr,
        Position {
            node: target_id,
            offset: target_offset,
            affinity: Affinity::Downstream,
        },
    )?;
    let target_id = target.node;
    let target_offset = target.offset;

    tr.batch::<_, CommandError>(|tr| {
        if let Some(pb_offset) = trailing_page_break_offset {
            tr.remove_child_slots(source_id, pb_offset, pb_offset + 1)?;
        }
        merge_element_cross_parent(tr, source_id, target_id)?;
        let steps = {
            let view = tr.state().view();
            view.node(source_parent_id)
                .map(|parent| fulfill(&parent))
                .unwrap_or_default()
        };
        tr.apply_steps(steps)?;
        Ok(())
    })?;

    tr.set_selection(Some(Selection::collapsed(Position {
        node: target_id,
        offset: target_offset,
        affinity: Affinity::Downstream,
    })))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn paragraph_after_list_joins_into_last_list_item() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                    }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| {
            join_paragraph_backward_into_prev_list_item(&mut tr)
        });
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
    fn paragraph_after_synthetic_empty_list_joins_into_item() {
        let (initial, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    bullet_list {
                        synthetic list_item {
                            p1: synthetic paragraph {}
                        }
                    }
                    p2: paragraph { text("B") }
                }
            }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| {
            join_paragraph_backward_into_prev_list_item(&mut tr)
        });
        let (expected, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    bullet_list {
                        list_item { p1: paragraph { text("B") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn previous_non_list_returns_false() {
        let (initial, ..) = state! {
            doc {
                root {
                    paragraph { text("A") }
                    p1: paragraph { text("B") }
                }
            }
            selection: (p1, 0)
        };
        transact_fail!(initial, |tr| {
            join_paragraph_backward_into_prev_list_item(&mut tr)
        });
    }
}
