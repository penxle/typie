use editor_crdt::Dot;
use editor_state::{Position, Selection};
use editor_transaction::Transaction;

use super::materialize_position_block;
use crate::CommandError;

/// `true` for a scaffold dot that `materialize_position_block` can turn into a
/// real op dot. Excludes [`Dot::ROOT`]: it is synthetic too, but it is a
/// permanent implicit anchor, never a materializable scaffold.
pub(crate) fn is_materializable_synthetic(node: Dot) -> bool {
    node != Dot::ROOT && node.as_op_dot().is_none()
}

/// Materialize any synthetic scaffold block holding a selection endpoint so
/// every downstream step targets real dots. Returns the remapped selection, or
/// `None` when both endpoints are already real. Endpoints are materialized in
/// document order: filler scaffold ids are keyed by parent slot, so the earlier
/// insertion must not invalidate the later endpoint's identity.
pub(crate) fn materialize_selection_endpoints(
    tr: &mut Transaction,
    selection: Selection,
) -> Result<Option<Selection>, CommandError> {
    let is_synthetic = is_materializable_synthetic;
    if !is_synthetic(selection.anchor.node) && !is_synthetic(selection.head.node) {
        return Ok(None);
    }

    let (anchor, head) = if selection.anchor.node == selection.head.node {
        let materialized = materialize_position_block(tr, selection.anchor)?;
        (
            materialized,
            Position {
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
    Ok(Some(Selection::new(anchor, head)))
}

pub(crate) fn materialize_target(tr: &mut Transaction, target: Dot) -> Result<Dot, CommandError> {
    let before = tr.selection();
    let remapped = editor_transaction::materialize_repair_target(tr, target)?;
    if remapped == target {
        return Ok(remapped);
    }
    if let Some(before) = before {
        let selection = {
            let view = tr.view();
            let remap = |pos: Position| {
                let node = if pos.node == target {
                    remapped
                } else {
                    view.alias_classes().resolve_with(pos.node, |d| {
                        view.node(d).is_some() || view.leaf(d).is_some()
                    })
                };
                Position { node, ..pos }
            };
            Selection::new(remap(before.anchor), remap(before.head))
        };
        tr.set_selection(Some(selection))?;
    }
    Ok(remapped)
}
