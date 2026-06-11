use editor_crdt::{CrdtError, Dot};

use crate::{ModifierType, NodeId, NodeType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorKind {
    Children,
    Text,
}

impl std::fmt::Display for AnchorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnchorKind::Children => write!(f, "children"),
            AnchorKind::Text => write!(f, "text"),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum ModelError {
    #[error("node not found: {0:?}")]
    NodeNotFound(NodeId),

    #[error("attr applied to wrong node kind")]
    AttrNodeKindMismatch,

    #[error("presence kind conflict at {node_id:?}: existing {existing:?}, incoming {incoming:?}")]
    PresenceKindConflict {
        node_id: NodeId,
        existing: NodeType,
        incoming: NodeType,
    },

    #[error(
        "modifier OrMap key must equal value's discriminant: at {node_id:?}, key {key:?}, value type {value_type:?}"
    )]
    ModifierKeyMismatch {
        node_id: NodeId,
        key: ModifierType,
        value_type: ModifierType,
    },

    #[error("presence outer node_id {node_id:?} disagrees with inner OrMapOp::Set key {key:?}")]
    PresenceKeyMismatch { node_id: NodeId, key: NodeId },

    #[error(
        "style presence outer style_id {style_id:?} disagrees with inner OrMapOp::Set key {key:?}"
    )]
    StylePresenceKeyMismatch { style_id: String, key: String },

    #[error(transparent)]
    Crdt(#[from] CrdtError),

    #[error("content violation at {node_id:?}: {detail}")]
    ContentViolation { node_id: NodeId, detail: String },

    #[error("context violation at {node_id:?}: {detail}")]
    ContextViolation { node_id: NodeId, detail: String },

    #[error("modifier {modifier_type:?} not allowed in context of {node_id:?}: {detail}")]
    ModifierContextViolation {
        node_id: NodeId,
        modifier_type: ModifierType,
        detail: String,
    },

    #[error("parent/child desync: parent {parent:?}, child {child:?}")]
    ParentChildDesync { parent: NodeId, child: NodeId },

    #[error("text projection desync at node {node_id:?}")]
    TextProjectionDesync { node_id: NodeId },

    #[error("text current-location index desync")]
    TextIndexDesync,

    #[error("root uniqueness violation: count = {count}")]
    RootUniquenessViolation { count: usize },

    #[error("node {node_id:?} not reachable from root")]
    NodeUnreachable { node_id: NodeId },

    #[error("insert anchor {anchor:?} not present in {kind} on node {node_id:?}")]
    OrphanAnchor {
        node_id: NodeId,
        anchor: Dot,
        kind: AnchorKind,
    },

    #[error("head dot not present in graph: {dot:?}")]
    InvalidHead { dot: Dot },
}
