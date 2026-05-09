use editor_model::{ModifierType, NodeId};
use editor_state::StateError;

#[derive(Debug, thiserror::Error)]
pub enum StepError {
    #[error("node not found: {0}")]
    NodeNotFound(NodeId),

    #[error("offset {offset} out of bounds for node {node_id} (len: {len})")]
    OffsetOutOfBounds {
        node_id: NodeId,
        offset: usize,
        len: usize,
    },

    #[error("index {index} out of bounds for children of {parent_id} (len: {len})")]
    IndexOutOfBounds {
        parent_id: NodeId,
        index: usize,
        len: usize,
    },

    #[error("expected text node: {0}")]
    ExpectedTextNode(NodeId),

    #[error("content violation at {node_id}: {detail}")]
    ContentViolation { node_id: NodeId, detail: String },

    #[error("context violation at {node_id}: {detail}")]
    ContextViolation { node_id: NodeId, detail: String },

    #[error("modifier context violation at {node_id}: {modifier_type:?} — {detail}")]
    ModifierContextViolation {
        node_id: NodeId,
        modifier_type: ModifierType,
        detail: String,
    },

    #[error(transparent)]
    State(#[from] StateError),
}
