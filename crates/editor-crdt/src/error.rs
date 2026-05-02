use crate::Dot;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CrdtError {
    #[error("Dot {dot:?} already exists with a different payload")]
    DotPayloadConflict { dot: Dot },
}
