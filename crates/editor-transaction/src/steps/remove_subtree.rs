use editor_crdt::Dot;
use editor_model::Subtree;
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(parent: Dot, index: usize, subtree: Subtree) -> Step {
    Step::InsertSubtree {
        parent,
        index,
        subtree,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    parent: Dot,
    index: usize,
    _subtree: &Subtree,
) -> Result<(), StepError> {
    let ops = {
        let ps = &batched.projected;
        let children = support::child_elem_ids(ps, parent);
        let elem = *children.get(index).ok_or(StepError::IndexOutOfBounds {
            parent,
            index,
            len: children.len(),
        })?;
        let dots = match ps.view().node(elem) {
            Some(_) => support::subtree_dots(ps, elem).ok_or(StepError::NodeNotFound(elem))?,
            None => match elem.as_op_dot() {
                Some(d) => vec![d.dot()],
                None => return Ok(()),
            },
        };
        support::delete_dots_ops(ps, &dots)
    };
    for op in ops {
        batched.apply(op)?;
    }
    Ok(())
}
