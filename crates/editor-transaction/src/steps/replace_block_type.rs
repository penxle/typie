use editor_crdt::Dot;
use editor_model::{AliasOp, Child, EditOp, NodeType};
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(block: Dot, old_type: NodeType, new_type: NodeType) -> Step {
    Step::ReplaceBlockType {
        block,
        old_type: new_type,
        new_type: old_type,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    block: Dot,
    _old_type: NodeType,
    new_type: NodeType,
) -> Result<(), StepError> {
    let block = {
        let view = batched.projected.view();
        view.alias_classes().resolve_with(block, |dot| {
            batched.projected.block_node_type(dot).is_some()
        })
    };

    if support::subtree_has_unknown(&batched.projected, block) {
        return Err(StepError::UnknownBearingReplace { block });
    }

    let (parent, index, subtree, dots) = {
        let ps = &batched.projected;
        let old_type = ps
            .block_node_type(block)
            .ok_or(StepError::NodeNotFound(block))?;
        let child_types: Vec<NodeType> = ps
            .block_children(block)
            .ok_or(StepError::NodeNotFound(block))?
            .into_iter()
            .filter_map(|child| match child {
                Child::Block(id) => ps.block_node_type(id),
                Child::Leaf { item, .. } => item.as_child_type(),
            })
            .collect();
        if !new_type.spec().content.matches_sequence(&child_types) {
            return Err(StepError::IncompatibleBlockTypeReplacement {
                block,
                old_type,
                new_type,
            });
        }

        let parent = ps.parent_of(block).ok_or(StepError::NodeNotFound(block))?;
        let children = support::child_elem_ids(ps, parent);
        let index = children
            .iter()
            .position(|d| *d == block)
            .ok_or(StepError::NodeNotFound(block))?;
        let mut subtree =
            support::capture_subtree(ps, block).ok_or(StepError::NodeNotFound(block))?;
        subtree.node = new_type.into_node().to_plain();
        let dots = support::subtree_dots(ps, block).ok_or(StepError::NodeNotFound(block))?;
        (parent, index, subtree, dots)
    };

    for op in support::delete_dots_ops(&batched.projected, &dots) {
        batched.apply(op)?;
    }

    let pos = support::child_seq_insert_pos(&batched.projected, parent, index)?;
    let parents = support::self_inclusive_parents(&batched.projected, parent)
        .ok_or(StepError::NodeNotFound(parent))?;
    let mut seq_pos = pos;
    let mut pairs = Vec::new();
    support::emit_subtree(batched, &subtree, &parents, &mut seq_pos, &mut pairs)?;
    if !pairs.is_empty() {
        batched.apply(EditOp::Alias(AliasOp {
            pairs: support::compress_alias_pairs(&pairs),
        }))?;
    }

    Ok(())
}
