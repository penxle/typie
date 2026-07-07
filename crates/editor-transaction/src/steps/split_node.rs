use editor_crdt::{Dot, ListOp};
use editor_model::{EditOp, SeqItem};
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(block: Dot, offset: usize) -> Step {
    Step::MergeNode { block, offset }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    block: Dot,
    offset: usize,
) -> Result<(), StepError> {
    let ps = &batched.projected;
    let node_type = support::block_node_type(ps, block).ok_or(StepError::NodeNotFound(block))?;
    let parents = support::parents_chain(ps, block).ok_or(StepError::NodeNotFound(block))?;
    let len = support::children_count(ps, block).unwrap_or(0);
    let pos = support::seq_insert_pos(ps, block, offset).ok_or(StepError::OffsetOutOfBounds {
        block,
        offset,
        len,
    })?;
    let new_dot = batched
        .apply(EditOp::Seq(ListOp::Ins {
            pos,
            item: SeqItem::Block {
                node_type,
                parents,
                attrs: vec![],
            },
        }))?
        .id;
    if let Some(src) = block.as_op_dot() {
        let overlays = support::block_overlay_ops(&batched.projected, src.dot(), new_dot);
        for op in overlays {
            batched.apply(op)?;
        }
    }
    Ok(())
}
