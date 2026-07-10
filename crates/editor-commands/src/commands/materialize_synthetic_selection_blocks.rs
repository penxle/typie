use editor_crdt::Dot;
use editor_model::{NodeType, NodeView};
use editor_state::Selection;
use editor_transaction::Transaction;

use crate::helpers::{materialize_position_block, materialize_synthetic_direct_children};
use crate::{CommandError, CommandResult};

pub fn materialize_synthetic_selection_blocks(tr: &mut Transaction) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    let is_synthetic = |node: Dot| node != Dot::ROOT && node.as_op_dot().is_none();
    if !is_synthetic(selection.anchor.node) && !is_synthetic(selection.head.node) {
        return Ok(false);
    }

    let (anchor, head) = if selection.anchor.node == selection.head.node {
        let materialized = materialize_position_block(tr, selection.anchor)?;
        (
            materialized,
            editor_state::Position {
                node: materialized.node,
                ..selection.head
            },
        )
    } else if is_synthetic(selection.anchor.node) && is_synthetic(selection.head.node) {
        let anchor_precedes_head = {
            let view = tr.view();
            let resolved = selection.resolve(&view).ok_or_else(|| {
                CommandError::Corrupted("cannot resolve synthetic selection".into())
            })?;
            resolved.anchor() < resolved.head()
        };
        if anchor_precedes_head {
            let anchor = materialize_position_block(tr, selection.anchor)?;
            let head = materialize_position_block(tr, selection.head)?;
            (anchor, head)
        } else {
            let head = materialize_position_block(tr, selection.head)?;
            let anchor = materialize_position_block(tr, selection.anchor)?;
            (anchor, head)
        }
    } else if is_synthetic(selection.anchor.node) {
        (
            materialize_position_block(tr, selection.anchor)?,
            selection.head,
        )
    } else {
        (
            selection.anchor,
            materialize_position_block(tr, selection.head)?,
        )
    };
    tr.set_selection(Some(Selection::new(anchor, head)))?;
    Ok(true)
}

pub fn materialize_synthetic_selected_block_children(
    tr: &mut Transaction,
    node_type: NodeType,
) -> CommandResult {
    let Some(selection) = tr.selection() else {
        return Ok(false);
    };
    if selection.is_collapsed() {
        return Ok(false);
    }

    let target = {
        let view = tr.view();
        let resolved = selection
            .resolve(&view)
            .ok_or_else(|| CommandError::Corrupted("cannot resolve selected block".into()))?;
        let Some(root) = view.root() else {
            return Ok(false);
        };
        exact_selected_block(&resolved, &root, node_type)
    };
    let Some(target) = target else {
        return Ok(false);
    };
    materialize_synthetic_direct_children(tr, target)
}

fn exact_selected_block(
    selection: &editor_state::ResolvedSelection<'_>,
    node: &NodeView<'_>,
    node_type: NodeType,
) -> Option<Dot> {
    let mut intersecting = node
        .child_blocks()
        .filter(|child| selection.intersects_subtree(child));
    let child = intersecting.next()?;
    if intersecting.next().is_some() {
        return None;
    }
    if child.node_type() == node_type && selection.contains_subtree(&child) {
        return Some(child.id());
    }
    if selection.contains_subtree(&child) || child.spec().is_textblock() {
        return None;
    }
    exact_selected_block(selection, &child, node_type)
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_model::NodeType;
    use editor_state::{Position, Selection};

    use super::*;
    use crate::test_utils::*;

    #[test]
    fn materializes_distinct_synthetic_endpoints_in_backward_document_order() {
        let (mut initial, ..) = state! {
            doc { root { fold paragraph {} } }
            selection: none
        };
        let (title, body) = {
            let view = initial.view();
            let fold = view
                .root()
                .unwrap()
                .child_blocks()
                .find(|block| block.node_type() == NodeType::Fold)
                .unwrap();
            let title = fold
                .child_blocks()
                .find(|block| block.node_type() == NodeType::FoldTitle)
                .unwrap();
            let content = fold
                .child_blocks()
                .find(|block| block.node_type() == NodeType::FoldContent)
                .unwrap();
            let body = content.child_blocks().next().unwrap();
            (title.id(), body.id())
        };
        initial.selection = Some(Selection::new(
            Position::new(body, 0),
            Position::new(title, 0),
        ));

        let (actual, ..) = transact!(initial, |tr| materialize_synthetic_selection_blocks(
            &mut tr,
        ));
        let (expected, ..) = state! {
            doc {
                root {
                    fold {
                        title: fold_title {}
                        fold_content { body: paragraph {} }
                    }
                    paragraph {}
                }
            }
            selection: (body, 0) -> (title, 0)
        };
        assert_state_eq!(&actual, &expected);
    }
}
