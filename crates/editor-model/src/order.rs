use editor_crdt::Dot;
use editor_crdt::sequence::{Bias, BoundaryResolver, SeqCheckout};

/// Sequence-order oracle for dot-keyed child lookup: the rank a binary search
/// over projected children can key by. `Some(rank)` iff `d` is a sequence
/// insertion element (visible or tombstone) whose position exists; `None` for
/// every other dot (a `Del`/`Undel` op's own dot, an unknown dot) — never a
/// panic. A tombstone shares its rank with the next visible element, so a rank
/// match alone never proves identity; callers must re-check the id.
pub trait SeqOrder {
    fn visible_rank(&self, d: Dot) -> Option<usize>;
}

impl SeqOrder for SeqCheckout {
    fn visible_rank(&self, d: Dot) -> Option<usize> {
        self.resolve_boundary_checked(d, Bias::Before)
            .map(|b| b.position)
    }
}

impl SeqOrder for BoundaryResolver {
    fn visible_rank(&self, d: Dot) -> Option<usize> {
        self.resolve_boundary_checked(d, Bias::Before)
            .map(|b| b.position)
    }
}
