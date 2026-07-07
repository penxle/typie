use editor_crdt::Dot;
use editor_model::{Modifier, ModifierType};
use editor_state::BatchedState;

use crate::steps::support;
use crate::{Step, StepError};

pub(crate) fn inverse(
    block: Dot,
    ty: ModifierType,
    old: Option<Modifier>,
    new: Option<Modifier>,
) -> Step {
    Step::SetNodeCarry {
        block,
        ty,
        old: new,
        new: old,
    }
}

pub(crate) fn apply_to(
    batched: &mut BatchedState,
    block: Dot,
    ty: ModifierType,
    new: Option<Modifier>,
) -> Result<(), StepError> {
    let op = match new {
        Some(modifier) => support::node_carry_set(block, modifier),
        None => support::node_carry_clear(block, ty),
    };
    batched.apply(op)?;
    Ok(())
}
