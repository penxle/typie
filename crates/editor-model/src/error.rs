use editor_crdt::{CrdtError, Dot};

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum ModelError {
    #[error("attr applied to wrong node kind")]
    AttrNodeKindMismatch,

    #[error(transparent)]
    Crdt(#[from] CrdtError),

    #[error("text current-location index desync")]
    TextIndexDesync,

    #[error("root uniqueness violation: count = {count}")]
    RootUniquenessViolation { count: usize },

    #[error("head dot not present in graph: {dot:?}")]
    InvalidHead { dot: Dot },
}
