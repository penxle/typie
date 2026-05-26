use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProseRun {
    pub(super) plain_range: Range<usize>,
    pub(super) flat_start: usize,
}
