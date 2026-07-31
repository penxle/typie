use editor_model::NodeType;
use editor_state::StableSelection;
use editor_transaction::Transaction;

use crate::helpers::{
    find_enclosing_list_item_id, is_list_type, merge_adjacent_list_pair, prev_sibling,
    restore_selection_after_adjacent_list_merge,
};
use crate::{CommandError, CommandResult};

pub fn merge_adjacent_list_backward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() || selection.head.offset != 0 {
        return Ok(false);
    }

    let (earlier_list_id, later_list_id) = {
        let view = tr.view();
        let paragraph = view
            .node(selection.head.node)
            .ok_or(CommandError::NodeNotFound(selection.head.node))?;
        if paragraph.node_type() != NodeType::Paragraph {
            return Ok(false);
        }
        let Some(item_id) = find_enclosing_list_item_id(&view, paragraph.id()) else {
            return Ok(false);
        };
        let item = view
            .node(item_id)
            .ok_or(CommandError::NodeNotFound(item_id))?;
        if paragraph.parent().map(|parent| parent.id()) != Some(item_id)
            || item.child_blocks().next().map(|child| child.id()) != Some(paragraph.id())
            || item.index() != Some(0)
        {
            return Ok(false);
        }

        let later_list = item.parent().ok_or(CommandError::NoParent(item_id))?;
        let Some(editor_model::ChildView::Block(earlier_list)) = prev_sibling(&later_list) else {
            return Ok(false);
        };
        if !is_list_type(earlier_list.node_type()) {
            return Ok(false);
        }
        (earlier_list.id(), later_list.id())
    };

    let stable = StableSelection::capture(&selection, &tr.view());
    let merged = merge_adjacent_list_pair(tr, earlier_list_id, later_list_id)?;
    restore_selection_after_adjacent_list_merge(tr, selection, stable, &merged)?;

    Ok(true)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn backward_keeps_earlier_list_kind_and_separate_items() {
        let (initial, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { paragraph { text("A") } }
                }
                ordered_list {
                    list_item {
                        p2: paragraph { text("B") }
                        bullet_list {
                            list_item { paragraph { text("nested") } }
                        }
                        paragraph { text("C") }
                    }
                    list_item { paragraph { text("D") } }
                }
            } }
            selection: (p2, 0)
        };
        let (actual, ..) = transact!(initial, |tr| merge_adjacent_list_backward(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                bullet_list {
                    list_item { paragraph { text("A") } }
                    list_item {
                        p2: paragraph { text("B") }
                        bullet_list {
                            list_item { paragraph { text("nested") } }
                        }
                        paragraph { text("C") }
                    }
                    list_item { paragraph { text("D") } }
                }
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(&actual, &expected);
        assert_projection_integrity(&actual);
    }

    #[test]
    fn backward_inside_nonfirst_item_returns_false() {
        let (initial, ..) = state! {
            doc { root {
                bullet_list { list_item { paragraph { text("A") } } }
                ordered_list {
                    list_item { paragraph { text("B") } }
                    list_item { p2: paragraph { text("C") } }
                }
            } }
            selection: (p2, 0)
        };
        transact_fail!(initial, |tr| merge_adjacent_list_backward(&mut tr));
    }
}
