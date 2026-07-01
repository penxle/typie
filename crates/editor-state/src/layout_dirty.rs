use editor_crdt::Dot;
use hashbrown::HashSet;

#[derive(Clone, Debug)]
pub enum LayoutDirty {
    Full,
    Incremental {
        content: HashSet<Dot>,
        structural: HashSet<Dot>,
    },
}

impl LayoutDirty {
    pub fn empty() -> Self {
        LayoutDirty::Incremental {
            content: HashSet::new(),
            structural: HashSet::new(),
        }
    }

    pub fn is_full(&self) -> bool {
        matches!(self, LayoutDirty::Full)
    }

    pub fn mark_full(&mut self) {
        *self = LayoutDirty::Full;
    }

    pub fn mark_content(&mut self, dot: Dot) {
        if let LayoutDirty::Incremental { content, .. } = self {
            content.insert(dot);
        }
    }

    pub fn mark_structural(&mut self, dot: Dot) {
        if let LayoutDirty::Incremental { structural, .. } = self {
            structural.insert(dot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: u64, c: u64) -> Dot {
        Dot::new(a, c)
    }

    #[test]
    fn empty_is_incremental_and_records_content() {
        let mut ld = LayoutDirty::empty();
        assert!(!ld.is_full());
        ld.mark_content(d(1, 1));
        match &ld {
            LayoutDirty::Incremental {
                content,
                structural,
            } => {
                assert!(content.contains(&d(1, 1)));
                assert!(structural.is_empty());
            }
            LayoutDirty::Full => panic!("should be incremental"),
        }
    }

    #[test]
    fn full_dominates_and_swallows_further_marks() {
        let mut ld = LayoutDirty::empty();
        ld.mark_full();
        ld.mark_content(d(1, 1));
        ld.mark_structural(d(1, 2));
        assert!(ld.is_full());
    }
}
