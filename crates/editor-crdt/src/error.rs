use crate::Dot;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CrdtError {
    #[error("Dot {dot:?} already exists with different op contents")]
    DotConflict { dot: Dot },

    #[error("Op {dot:?} cannot reference itself in parents")]
    SelfReference { dot: Dot },

    #[error("Op {dot:?} references missing parents: {missing:?}")]
    MissingParents { dot: Dot, missing: Vec<Dot> },

    #[error("Op {dot:?} clock cannot advance — overflow")]
    ClockOverflow { dot: Dot },

    #[error("Remote heads include dots not present in self.ops: {unknown:?}")]
    UnknownHeads { unknown: Vec<Dot> },

    #[error("offset {offset} out of bounds (len {len})")]
    OffsetOutOfBounds { offset: usize, len: usize },
}
