use editor_model::{ChildView, NodeType};
use editor_state::{Affinity, Position, Selection};
use editor_transaction::{Transaction, fulfill};

use crate::helpers::{
    find_enclosing_list_item_id, materialize_position_block, merge_element_cross_parent,
};
use crate::{CommandError, CommandResult};

pub fn join_next_paragraph_forward_into_list_item(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.anchor != selection.head {
        return Ok(false);
    }

    let pos = selection.head;
    let (source_id, target_id, target_offset, source_parent_id, trailing_page_break_offset) = {
        let view = tr.state().view();
        let list_item_id = match find_enclosing_list_item_id(&view, pos.node) {
            Some(id) => id,
            None => return Ok(false),
        };
        let list_item = view
            .node(list_item_id)
            .ok_or(CommandError::NodeNotFound(list_item_id))?;
        let target = match list_item.first_child() {
            Some(ChildView::Block(p)) if p.node_type() == NodeType::Paragraph => p,
            _ => {
                return Err(CommandError::Corrupted(
                    "list_item missing paragraph".into(),
                ));
            }
        };
        if pos.node != target.id() || pos.offset < target.children().count() {
            return Ok(false);
        }
        if list_item.child_blocks().last().map(|b| b.id()) != Some(target.id()) {
            return Ok(false);
        }

        let list = list_item
            .parent()
            .ok_or(CommandError::NoParent(list_item_id))?;
        if list.child_blocks().last().map(|b| b.id()) != Some(list_item_id) {
            return Ok(false);
        }

        let list_parent = list.parent().ok_or(CommandError::NoParent(list.id()))?;
        let list_idx = list
            .index()
            .ok_or_else(|| CommandError::orphan_child(list.id(), list_parent.id()))?;
        let source = match list_parent.child_at(list_idx + 1) {
            Some(ChildView::Block(b))
                if b.node_type() == NodeType::Paragraph && b.dot().is_some() =>
            {
                b
            }
            _ => return Ok(false),
        };

        let source_child_count = source.children().count();
        let trailing_page_break_offset = match source.last_child() {
            Some(ChildView::Leaf(l)) if l.node_type() == NodeType::PageBreak => {
                Some(source_child_count - 1)
            }
            _ => None,
        };

        (
            source.id(),
            target.id(),
            pos.offset,
            list_parent.id(),
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

    tr.batch::<_, CommandError>(|tr| {
        if let Some(pb_offset) = trailing_page_break_offset {
            tr.remove_child_slots(source_id, pb_offset, pb_offset + 1)?;
        }
        merge_element_cross_parent(tr, source_id, target.node)?;
        let steps = {
            let view = tr.state().view();
            view.node(source_parent_id)
                .map(|parent| fulfill(&parent))
                .unwrap_or_default()
        };
        tr.apply_steps(steps)?;
        Ok(())
    })?;

    tr.set_selection(Some(Selection::collapsed(target)))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

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
        let (actual, ..) = transact!(initial, |tr| {
            join_next_paragraph_forward_into_list_item(&mut tr)
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
    fn synthetic_last_item_pulls_next_paragraph() {
        let (initial, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    bullet_list {
                        synthetic list_item {
                            p1: synthetic paragraph {}
                        }
                    }
                    paragraph { text("B") }
                }
            }
            selection: (p1, 0)
        };
        let (actual, ..) = transact!(initial, |tr| {
            join_next_paragraph_forward_into_list_item(&mut tr)
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
    fn last_item_pulls_next_paragraph_preserves_bold() {
        let (initial, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") } }
                    }
                    paragraph { text("B") [bold] }
                }
            }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| {
            join_next_paragraph_forward_into_list_item(&mut tr)
        });
        let (expected, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item { p1: paragraph { text("A") text("B") [bold] } }
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
        let (actual, ..) = transact!(initial, |tr| {
            join_next_paragraph_forward_into_list_item(&mut tr)
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
    fn next_list_item_returns_false() {
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
            selection: (p1, 1)
        };
        transact_fail!(initial, |tr| {
            join_next_paragraph_forward_into_list_item(&mut tr)
        });
    }

    #[test]
    fn next_block_not_paragraph_returns_false() {
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
        transact_fail!(initial, |tr| {
            join_next_paragraph_forward_into_list_item(&mut tr)
        });
    }
}
