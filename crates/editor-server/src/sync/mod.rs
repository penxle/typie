mod attribute;
mod conflict;
mod merge;
mod modifier;
mod reorder;
mod text;
mod tree;

pub use conflict::{
    AttributeScope, BranchSide, ConflictBranch, ConflictKind, ConflictRecord, ConflictTarget,
};
pub use merge::merge;

#[cfg(test)]
mod proptest_laws;

#[cfg(test)]
mod test_helpers;
