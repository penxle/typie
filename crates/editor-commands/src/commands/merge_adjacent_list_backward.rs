use editor_model::{ChildView, NodeType};
use editor_state::{StableResolveCtx, StableSelection};
use editor_transaction::Transaction;

use crate::helpers::{find_enclosing_list_item_id, is_list_type, prev_sibling};
use crate::{CommandError, CommandResult};

pub fn merge_adjacent_list_backward(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if !selection.is_collapsed() || selection.head.offset != 0 {
        return Ok(false);
    }

    let (earlier_list_id, later_list_id, earlier_len, items) = {
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
        if earlier_list
            .children()
            .chain(later_list.children())
            .any(|child| matches!(child, ChildView::Leaf(_)))
        {
            return Err(CommandError::Corrupted(
                "list contains an unsupported direct child".into(),
            ));
        }
        let items = later_list
            .child_blocks()
            .map(|item| item.id())
            .collect::<Vec<_>>();
        if items.is_empty() {
            return Err(CommandError::Corrupted(
                "later list contains no list items".into(),
            ));
        }
        (
            earlier_list.id(),
            later_list.id(),
            earlier_list.child_blocks().count(),
            items,
        )
    };

    let stable_selection = StableSelection::capture(&selection, &tr.view());
    tr.batch::<_, CommandError>(|tr| {
        tr.move_nodes_consecutive(&items, earlier_list_id, earlier_len)?;
        tr.remove_subtree(later_list_id)?;
        let resolved = {
            let view = tr.view();
            let ctx = StableResolveCtx::from_live(&view, tr.state().projected.seq_checkout());
            stable_selection.resolve(&ctx)
        }
        .ok_or_else(|| {
            CommandError::Corrupted("cannot restore selection after list merge".into())
        })?;
        tr.set_selection(Some(resolved))?;
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
