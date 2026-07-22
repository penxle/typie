use editor_crdt::Dot;
use editor_model::{ModifierType, NodeType};
use editor_state::StateError;

#[derive(Debug, thiserror::Error)]
pub enum StepError {
    #[error("node not found: {0:?}")]
    NodeNotFound(Dot),

    #[error("node attr kind does not match target node: {block:?}")]
    NodeAttrKindMismatch { block: Dot },

    #[error("old and new node attrs refer to different fields: {block:?}")]
    NodeAttrFieldMismatch { block: Dot },

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

    #[error("root subtree cannot be inserted")]
    RootSubtree,

    #[error("unknown-placeholder subtree cannot be reinserted")]
    UnknownSubtree,

    #[error("move target {block:?} carries unknown content and cannot move losslessly")]
    UnknownBearingMove { block: Dot },

    #[error(
        "repair scaffold {block:?} carries unknown content and cannot be materialized losslessly"
    )]
    UnknownBearingMaterialize { block: Dot },

    #[error("replace target {block:?} carries unknown content and cannot replace type losslessly")]
    UnknownBearingReplace { block: Dot },

    #[error("illegal insert slot {pos} for block {block:?}")]
    IllegalInsertSlot { block: Dot, pos: usize },

    #[error(
        "cannot replace {block:?} block type from {old_type:?} to {new_type:?} without changing its children"
    )]
    IncompatibleBlockTypeReplacement {
        block: Dot,
        old_type: NodeType,
        new_type: NodeType,
    },

    #[error("move item {item:?} is duplicated in the batch")]
    DuplicateMoveItem { item: Dot },

    #[error("move item {ancestor:?} is an ancestor of {descendant:?} in the same batch")]
    NonAntichainMoveItems { ancestor: Dot, descendant: Dot },

    #[error("move destination {dest:?} lies inside the forest being moved (item {item:?})")]
    MoveDestinationInsideForest { item: Dot, dest: Dot },

    #[error("a fresh move container root must be a block or atom, found: {node_type:?}")]
    InvalidMoveContainer { node_type: NodeType },

    #[error(transparent)]
    State(#[from] StateError),
}
