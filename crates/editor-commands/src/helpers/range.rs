use editor_crdt::Dot;
use editor_model::{ChildView, DocView};
use editor_state::{Position, Selection};
use editor_transaction::Transaction;

use crate::CommandError;

pub(crate) fn inline_leaf_dots_in_range(
    view: &DocView,
    from: &Position,
    to: &Position,
) -> Vec<Dot> {
    let Some(rs) = Selection::new(*from, *to).resolve(view) else {
        return Vec::new();
    };
    let lo = rs.from().position();
    let hi = rs.to().position();
    let (Some(lo_r), Some(hi_r)) = (lo.resolve(view), hi.resolve(view)) else {
        return Vec::new();
    };

    let mut blocks = Vec::new();
    if let Some(root) = view.root() {
        blocks.push(root);
        for d in root.descendants() {
            if let ChildView::Block(b) = d {
                blocks.push(b);
            }
        }
    }

    let mut out = Vec::new();
    for block in blocks {
        let block_id = block.id();
        for (i, child) in block.children().enumerate() {
            let ChildView::Leaf(l) = child else { continue };
            let (Some(start), Some(end)) = (
                Position::new(block_id, i).resolve(view),
                Position::new(block_id, i + 1).resolve(view),
            ) else {
                continue;
            };
            if lo_r <= start && end <= hi_r {
                out.push(l.dot());
            }
        }
    }
    out
}

pub(crate) fn compact_textblocks_for_nodes(
    _tr: &mut Transaction,
    _ids: &[Dot],
) -> Result<(), CommandError> {
    Ok(())
}
