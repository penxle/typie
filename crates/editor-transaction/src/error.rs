use editor_crdt::Dot;
use editor_model::ModifierType;
use editor_state::StateError;

#[derive(Debug, thiserror::Error)]
pub enum StepError {
    #[error("node not found: {0:?}")]
    NodeNotFound(Dot),

    #[error("offset {offset} out of bounds for node {block:?} (len: {len})")]
    OffsetOutOfBounds {
        block: Dot,
        offset: usize,
        len: usize,
    },

    #[error("index {index} out of bounds for children of {parent:?} (len: {len})")]
    IndexOutOfBounds {
        parent: Dot,
        index: usize,
        len: usize,
    },

    #[error("pending modifier {modifier_type:?} is not applicable to Text and cannot be staged")]
    InvalidPendingModifier { modifier_type: ModifierType },

    #[error("merge target {block:?} has no following sibling block to merge")]
    MergeNoSibling { block: Dot },

    #[error(transparent)]
    State(#[from] StateError),
}
