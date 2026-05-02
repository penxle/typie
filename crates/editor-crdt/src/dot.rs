use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// CRDT-standard `(actor, clock)` identity. Used as char-id here; meant to be reused
/// across future primitives (sequence-CRDT, OR-Set add tokens, LWW register timestamps,
/// op identity).
///
/// **`clock` is per-actor monotonic — *not a strict Lamport timestamp*.**
/// We don't bump our clock when observing remote ops, so cross-actor comparison
/// does not reflect causal precedence. Sufficient for RGA tie-break (deterministic
/// ordering) but reusing this for LWW winner determination requires the op-generation
/// layer to provide Lamport semantics (`L = max(L_self, observed_max) + 1`) separately.
///
/// `Ord` / `PartialOrd` are implemented manually — `derive` depends on field
/// declaration order, which is fragile. Clock-primary is just a setup that *can*
/// evolve into a Lamport-compatible form later; it is not itself a Lamport guarantee
/// (see caveat above).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Dot {
    pub actor: u64,
    pub clock: u64,
}

impl Ord for Dot {
    fn cmp(&self, other: &Self) -> Ordering {
        self.clock
            .cmp(&other.clock)
            .then_with(|| self.actor.cmp(&other.actor))
    }
}

impl PartialOrd for Dot {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ord_clock_primary() {
        let lo_actor_high_clock = Dot {
            actor: 1,
            clock: 10,
        };
        let hi_actor_low_clock = Dot { actor: 5, clock: 2 };
        assert!(hi_actor_low_clock < lo_actor_high_clock);
    }

    #[test]
    fn ord_actor_tie_break_on_equal_clock() {
        let x = Dot { actor: 1, clock: 3 };
        let y = Dot { actor: 2, clock: 3 };
        assert!(x < y);
    }

    #[test]
    fn ord_same_actor_by_clock() {
        let a = Dot { actor: 5, clock: 1 };
        let b = Dot { actor: 5, clock: 7 };
        assert!(a < b);
    }
}
