use editor_crdt::{Dot, ListOp};
use editor_model::EditOp;
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(emitted: Vec<Dot>) -> Step {
    Step::UndeleteOpaque { dels: emitted }
}

pub(crate) fn inverse_of_undelete() -> Step {
    Step::DeleteOpaque {
        dots: Vec::new(),
        emitted: Vec::new(),
    }
}

pub(crate) fn apply_to(batched: &mut BatchedState, dots: &[Dot]) -> Result<(), StepError> {
    let ops = support::delete_dots_ops(&batched.projected, dots);
    for op in ops {
        batched.apply(op)?;
    }
    Ok(())
}

pub(crate) fn apply_to_undelete(batched: &mut BatchedState, dels: &[Dot]) -> Result<(), StepError> {
    for &del in dels {
        batched.apply(EditOp::Seq(ListOp::Undel { del }))?;
    }
    Ok(())
}
