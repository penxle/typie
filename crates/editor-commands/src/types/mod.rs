#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceProvenance {
    Formatted,
    Plain,
}

impl SliceProvenance {
    pub(crate) fn is_plain(self) -> bool {
        matches!(self, SliceProvenance::Plain)
    }
}
