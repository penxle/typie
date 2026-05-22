use editor_model::{Node, NodeId};
use editor_transaction::Transaction;

use crate::CommandError;

/// Merge `source`'s children into `target` (appended at the end) and remove
/// `source`. Same net effect as `Transaction::merge_node`, but composed from
/// `MoveNode` + `RemoveSubtree` so the operation is properly invertible when
/// source and target live in different parents.
///
/// `Step::MergeNode`'s inverse `Step::SplitNode` reconstructs the source as a
/// sibling of `target`. When source originally lived in a different parent,
/// that reconstruction places it in the wrong container, leaving source's
/// original parent empty and tripping a content-validation panic at undo time.
/// `MoveNode` records `old_parent`/`old_index`, so each moved child finds its
/// way home; `RemoveSubtree` captures the source subtree for `InsertSubtree`
/// inverse.
///
/// For element nodes only — text-content merges are always same-parent and
/// stay on the `merge_node` path.
pub(crate) fn merge_element_cross_parent(
    tr: &mut Transaction,
    source_id: NodeId,
    target_id: NodeId,
) -> Result<(), CommandError> {
    let (child_ids, target_len) = {
        let doc = tr.doc();
        let source = doc
            .node(source_id)
            .ok_or(CommandError::NodeNotFound(source_id))?;
        let target = doc
            .node(target_id)
            .ok_or(CommandError::NodeNotFound(target_id))?;
        // Text nodes hold characters, not child slots; a `move_node` loop over
        // an empty `children` list would silently drop the text along with the
        // `remove_subtree` below. Same-parent text concatenation already runs
        // through `merge_node`'s text path, so this helper only needs to refuse
        // the wrong-input case rather than handle it.
        if matches!(source.node(), Node::Text(_)) {
            return Err(CommandError::ExpectedElementNode(source_id));
        }
        if matches!(target.node(), Node::Text(_)) {
            return Err(CommandError::ExpectedElementNode(target_id));
        }
        let child_ids: Vec<NodeId> = source.entry().children.iter().copied().collect();
        let target_len = target.entry().children.len();
        (child_ids, target_len)
    };

    for (i, child_id) in child_ids.into_iter().enumerate() {
        tr.move_node(child_id, target_id, target_len + i)?;
    }
    tr.remove_subtree(source_id)?;
    Ok(())
}
