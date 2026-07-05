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

    #[error("invalid child range [{from}, {to}) for node {parent:?}")]
    InvalidChildRange { parent: Dot, from: usize, to: usize },

    #[error("invalid child slot {index} for node {parent:?}")]
    InvalidChildSlot { parent: Dot, index: usize },

    #[error("expected text char at offset {offset} for node {block:?}")]
    ExpectedText { block: Dot, offset: usize },

    #[error(
        "text mismatch at offset {offset} for node {block:?}: expected {expected:?}, found {actual:?}"
    )]
    TextMismatch {
        block: Dot,
        offset: usize,
        expected: char,
        actual: char,
    },

    #[error("pending modifier {modifier_type:?} is not applicable to Text and cannot be staged")]
    InvalidPendingModifier { modifier_type: ModifierType },

    #[error("merge target {block:?} has no following sibling block to merge")]
    MergeNoSibling { block: Dot },

    #[error(transparent)]
    State(#[from] StateError),
}
