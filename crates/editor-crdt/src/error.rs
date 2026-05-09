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

    #[error("changeset has no ops")]
    EmptyChangeset,

    #[error("partial duplicate: some ops in changeset already known, others not. dots: {dots:?}")]
    PartialDuplicate { dots: Vec<Dot> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_changeset_renders_message() {
        let err = CrdtError::EmptyChangeset;
        assert_eq!(err.to_string(), "changeset has no ops");
    }

    #[test]
    fn partial_duplicate_lists_dots() {
        let dot = Dot::new(1, 2);
        let err = CrdtError::PartialDuplicate { dots: vec![dot] };
        assert!(err.to_string().contains("partial duplicate"));
        assert!(err.to_string().contains("actor: 1"));
        assert!(err.to_string().contains("clock: 2"));
    }
}
