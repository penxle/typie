use editor_crdt::Dot;
use editor_model::{AliasOp, EditOp, NodeType, PlainNode, Subtree};
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(parent: Dot, index: usize, subtree: Subtree) -> Step {
    Step::RemoveSubtree {
        parent,
        index,
        subtree,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    parent: Dot,
    index: usize,
    subtree: &Subtree,
) -> Result<(), StepError> {
    if subtree.node == PlainNode::Unknown {
        return Err(StepError::UnknownSubtree);
    }
    if subtree.node.as_type() == NodeType::Root {
        return Err(StepError::RootSubtree);
    }
    let (del_ops, pos, parents, host) = {
        let ps = &batched.projected;
        let raw_pos = support::child_seq_insert_pos(ps, parent, index, subtree.node.as_type())?;
        let parents =
            support::self_inclusive_parents(ps, parent).ok_or(StepError::NodeNotFound(parent))?;
        let host = support::parent_host_type(ps, &parents);
        let live: Vec<Dot> = subtree
            .collect_source_dots()
            .into_iter()
            .filter(|&d| ps.seq_visible_pos(d).is_some())
            .collect();
        if live.is_empty() {
            (Vec::new(), raw_pos, parents, host)
        } else {
            let dots = support::with_hidden_copies(ps, live);
            let before = dots
                .iter()
                .filter_map(|&d| ps.seq_flat_pos(d))
                .filter(|&p| p < raw_pos)
                .count();
            (
                support::delete_dots_ops(ps, &dots),
                raw_pos - before,
                parents,
                host,
            )
        }
    };

    for op in del_ops {
        batched.apply(op)?;
    }

    let mut seq_pos = pos;
    let mut pairs: Vec<(Dot, Dot)> = Vec::new();
    support::emit_subtree(batched, subtree, &parents, host, &mut seq_pos, &mut pairs)?;
    if !pairs.is_empty() {
        batched.apply(EditOp::Alias(AliasOp {
            pairs: support::compress_alias_pairs(&pairs),
        }))?;
    }
    Ok(())
}
