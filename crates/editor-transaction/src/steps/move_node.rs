use editor_crdt::Dot;
use editor_model::{AliasOp, EditOp};
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

/// The identity a `move_node`/composite-move emits: the re-published root dot
/// of the moved subtree, and every old→new dot pairing `emit_subtree` walked
/// (the moved subtree's own root plus every descendant), exposed so a caller
/// can resolve a moved child's fresh dot without re-reading the tree.
#[derive(Clone, Debug, PartialEq)]
pub struct MovedNode {
    pub root: Dot,
    pub pairs: Vec<(Dot, Dot)>,
}

pub(crate) fn inverse(
    block: Dot,
    old_parent: Dot,
    old_index: usize,
    new_parent: Dot,
    new_index: usize,
) -> Step {
    Step::MoveNode {
        block,
        old_parent: new_parent,
        old_index: new_index,
        new_parent: old_parent,
        new_index: old_index,
    }
}

/// Moves `block`'s subtree to `(new_parent, new_index)`.
///
/// Fast path (`new_parent` outside the moved subtree AND `new_parent !=
/// old_parent`): every projected read — the subtree capture, the deleted
/// dots' pre-delete flat positions, the destination's insert slot and tree
/// parents — happens before the first emitted op. The destination position is
/// computed against the pre-delete tree, then corrected by the count of
/// deleted (visible) positions strictly before it — pure arithmetic over the
/// pre-captured positions, no post-delete projected read. A same-parent
/// reposition can't be precomputed this way (the pre-delete index and
/// post-delete index of a slot in the SAME parent mean different things), so
/// it falls back to the original order: delete, then re-read the (now
/// flushed) projection for the insert slot.
pub(crate) fn apply_to(
    batched: &mut BatchedState,
    block: Dot,
    old_parent: Dot,
    _old_index: usize,
    new_parent: Dot,
    new_index: usize,
) -> Result<MovedNode, StepError> {
    if support::subtree_has_unknown(&batched.projected, block) {
        return Err(StepError::UnknownBearingMove { block });
    }

    let subtree = support::capture_subtree(&batched.projected, block)
        .ok_or(StepError::NodeNotFound(block))?;
    let dots =
        support::subtree_dots(&batched.projected, block).ok_or(StepError::NodeNotFound(block))?;

    let fast = new_parent != old_parent && !dots.contains(&new_parent);

    let (seq_pos_start, parents, host) = if fast {
        let ps = &batched.projected;
        let raw_pos =
            support::child_seq_insert_pos(ps, new_parent, new_index, subtree.node.as_type())?;
        let parents = support::self_inclusive_parents(ps, new_parent)
            .ok_or(StepError::NodeNotFound(new_parent))?;
        let host = support::parent_host_type(ps, &parents);
        // Visible-space correction: how many of the subtree's own (pre-delete)
        // flat positions land before the raw insert position — computed via
        // `seq_flat_pos` (warm-latest, not the stale projected tree), so this
        // needs no read after the delete below.
        let positions: Vec<usize> = dots.iter().filter_map(|&d| ps.seq_flat_pos(d)).collect();
        let before = positions.iter().filter(|&&p| p < raw_pos).count();
        let del_ops = support::delete_dots_ops(ps, &dots);
        for op in del_ops {
            batched.apply(op)?;
        }
        (raw_pos - before, parents, host)
    } else {
        let del_ops = support::delete_dots_ops(&batched.projected, &dots);
        for op in del_ops {
            batched.apply(op)?;
        }
        let ps = batched.projected_clean()?;
        let pos = support::child_seq_insert_pos(ps, new_parent, new_index, subtree.node.as_type())?;
        let parents = support::self_inclusive_parents(ps, new_parent)
            .ok_or(StepError::NodeNotFound(new_parent))?;
        let host = support::parent_host_type(ps, &parents);
        (pos, parents, host)
    };

    let mut seq_pos = seq_pos_start;
    let mut pairs: Vec<(Dot, Dot)> = Vec::new();
    let root = support::emit_subtree(batched, &subtree, &parents, host, &mut seq_pos, &mut pairs)?
        .ok_or(StepError::NodeNotFound(block))?;
    if !pairs.is_empty() {
        batched.apply(EditOp::Alias(AliasOp {
            pairs: support::compress_alias_pairs(&pairs),
        }))?;
    }
    Ok(MovedNode { root, pairs })
}
