#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Affinity {
    Upstream,
    Downstream,
}

impl Default for Affinity {
    fn default() -> Self {
        Affinity::Downstream
    }
}
