use editor_crdt::Dot;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListVerdict<P> {
    Change(P),
    AbsorbOnly,
    NotApplicable,
}

impl<P> ListVerdict<P> {
    pub fn changes(&self) -> bool {
        matches!(self, ListVerdict::Change(_))
    }
}

pub struct OutdentPlan {
    pub items: Vec<Dot>,
}

pub struct IndentPlan {
    pub items: Vec<Dot>,
}

pub(crate) struct LiftOfKindPlan {
    pub(crate) items: Vec<Dot>,
}
