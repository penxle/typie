use editor_state::StableSelection;
use editor_transaction::Transaction;

use crate::helpers::{
    ForwardListBoundary, find_forward_list_boundary, is_list_type, merge_adjacent_list_pair,
    restore_selection_after_adjacent_list_merge,
};
use crate::{CommandError, CommandResult};

pub fn merge_adjacent_list_forward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() {
        return Ok(false);
    }

    let (earlier_list_id, later_list_id) = {
        let view = tr.view();
        let Some(ForwardListBoundary::NextBlock { list_id, next_id }) =
            find_forward_list_boundary(&view, selection.head)?
        else {
            return Ok(false);
        };
        let earlier_list = view
            .node(list_id)
            .ok_or(CommandError::NodeNotFound(list_id))?;
        let later_list = view
            .node(next_id)
            .ok_or(CommandError::NodeNotFound(next_id))?;
        if !is_list_type(later_list.node_type()) {
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
    fn forward_keeps_earlier_list_kind_and_separate_items() {
        let (initial, ..) = state! {
            doc { root {
                ordered_list {
                    list_item { p1: paragraph { text("A") } }
                }
                bullet_list {
                    list_item { paragraph { text("B") } }
                }
            } }
            selection: (p1, 1)
        };
        let (actual, ..) = transact!(initial, |tr| merge_adjacent_list_forward(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                ordered_list {
                    list_item { p1: paragraph { text("A") } }
                    list_item { paragraph { text("B") } }
                }
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(&actual, &expected);
    }

    #[test]
    fn forward_from_trailing_nested_item_merges_outer_lists() {
        let (initial, ..) = state! {
            doc { root {
                ordered_list {
                    list_item {
                        paragraph { text("A") }
                        bullet_list { list_item { p1: paragraph { text("nested") } } }
                    }
                }
                bullet_list { list_item { paragraph { text("B") } } }
            } }
            selection: (p1, 6)
        };
        let (actual, ..) = transact!(initial, |tr| merge_adjacent_list_forward(&mut tr));
        let (expected, ..) = state! {
            doc { root {
                ordered_list {
                    list_item {
                        paragraph { text("A") }
                        bullet_list { list_item { p1: paragraph { text("nested") } } }
                    }
                    list_item { paragraph { text("B") } }
                }
            } }
            selection: (p1, 6)
        };
        assert_state_eq!(&actual, &expected);
        assert_projection_integrity(&actual);
    }
}
