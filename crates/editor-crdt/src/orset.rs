use serde::{Deserialize, Serialize};
use std::hash::Hash;

use crate::{CrdtError, Dot, FastMap, FastSet, ToPlain};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrSetOp<T> {
    Add { elem: T },
    Remove { observed: Dot },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrSet<T: Clone + Eq + Hash> {
    entries: FastMap<Dot, Entry<T>>,
    pending_tombstones: FastSet<Dot>,
    by_elem: FastMap<T, FastSet<Dot>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Entry<T> {
    elem: T,
    alive: bool,
}

impl<T: Clone + Eq + Hash> OrSet<T> {
    pub fn new() -> Self {
        Self {
            entries: FastMap::new(),
            pending_tombstones: FastSet::new(),
            by_elem: FastMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.by_elem.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_elem.is_empty()
    }

    pub fn contains(&self, elem: &T) -> bool {
        self.by_elem.contains_key(elem)
    }

    pub fn tags_for<'a>(&'a self, elem: &'a T) -> impl Iterator<Item = &'a Dot> + 'a {
        self.by_elem.get(elem).into_iter().flat_map(|s| s.iter())
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.by_elem.keys()
    }

    /// Returns `Err` if the same `Dot` arrives with a different `elem`.
    ///
    /// Dot uniqueness with deterministic payload is a precondition the op-generation
    /// layer must guarantee. Wire integrations should reject malformed ops before
    /// reaching this API; this `Err` is a defense-in-depth signal that wire validation
    /// failed.
    pub fn apply(&self, id: Dot, op: OrSetOp<T>) -> Result<Self, CrdtError> {
        match op {
            OrSetOp::Add { elem } => self.apply_add(id, elem),
            OrSetOp::Remove { observed } => Ok(self.apply_remove(observed)),
        }
    }

    fn apply_add(&self, id: Dot, elem: T) -> Result<Self, CrdtError> {
        if let Some(existing) = self.entries.get(&id) {
            // Two ops with the same Dot must have identical payloads — a violation
            // is a bug in the op generation layer (actor id collision, clock reuse,
            // etc.). Silent first-wins would diverge replicas.
            if existing.elem != elem {
                return Err(CrdtError::DotConflict { dot: id });
            }
            return Ok(self.clone());
        }
        let alive = !self.pending_tombstones.contains(&id);
        let new_by_elem = if alive {
            let updated = self
                .by_elem
                .get(&elem)
                .cloned()
                .unwrap_or_else(FastSet::new)
                .update(id);
            self.by_elem.update(elem.clone(), updated)
        } else {
            self.by_elem.clone()
        };
        let entry = Entry { elem, alive };
        Ok(Self {
            entries: self.entries.update(id, entry),
            pending_tombstones: self.pending_tombstones.without(&id),
            by_elem: new_by_elem,
        })
    }

    fn apply_remove(&self, observed: Dot) -> Self {
        if let Some(entry) = self.entries.get(&observed) {
            if !entry.alive {
                return self.clone();
            }
            let new_entry = Entry {
                elem: entry.elem.clone(),
                alive: false,
            };
            let new_by_elem = if let Some(set) = self.by_elem.get(&entry.elem).cloned() {
                let updated = set.without(&observed);
                if updated.is_empty() {
                    self.by_elem.without(&entry.elem)
                } else {
                    self.by_elem.update(entry.elem.clone(), updated)
                }
            } else {
                self.by_elem.clone()
            };
            return Self {
                entries: self.entries.update(observed, new_entry),
                pending_tombstones: self.pending_tombstones.clone(),
                by_elem: new_by_elem,
            };
        }
        Self {
            entries: self.entries.clone(),
            pending_tombstones: self.pending_tombstones.update(observed),
            by_elem: self.by_elem.clone(),
        }
    }
}

impl<T: Clone + Eq + Hash> Default for OrSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ToPlain for OrSet<T>
where
    T: Clone + Eq + std::hash::Hash + Ord,
{
    type Plain = std::collections::BTreeSet<T>;
    fn to_plain(&self) -> Self::Plain {
        self.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn empty_state() {
        let s: OrSet<u32> = OrSet::new();
        assert_eq!(s.len(), 0);
        assert!(s.is_empty());
        assert!(!s.contains(&42));
        assert_eq!(s.iter().count(), 0);
    }

    #[test]
    fn add_single_element() {
        let s = OrSet::new()
            .apply(Dot::new(1, 0), OrSetOp::Add { elem: 42u32 })
            .unwrap();
        assert_eq!(s.len(), 1);
        assert!(s.contains(&42));
        assert!(!s.is_empty());
    }

    #[test]
    fn add_same_dot_same_elem_idempotent() {
        let id = Dot::new(1, 0);
        let op = OrSetOp::Add { elem: 42u32 };
        let s = OrSet::new()
            .apply(id, op.clone())
            .unwrap()
            .apply(id, op.clone())
            .unwrap()
            .apply(id, op)
            .unwrap();
        assert_eq!(s.len(), 1);
        assert!(s.contains(&42));
    }

    #[test]
    fn iter_dedups_by_elem() {
        let s = OrSet::new()
            .apply(Dot::new(1, 0), OrSetOp::Add { elem: 42u32 })
            .unwrap()
            .apply(Dot::new(2, 0), OrSetOp::Add { elem: 42u32 })
            .unwrap();
        assert_eq!(s.len(), 1);
        assert_eq!(s.iter().count(), 1);
        assert!(s.contains(&42));
    }

    #[test]
    fn duplicate_dot_different_elem_returns_err() {
        let s = OrSet::new()
            .apply(Dot::new(1, 0), OrSetOp::Add { elem: 42u32 })
            .unwrap();
        let result = s.apply(Dot::new(1, 0), OrSetOp::Add { elem: 99u32 });
        assert_eq!(
            result,
            Err(CrdtError::DotConflict {
                dot: Dot::new(1, 0)
            })
        );
    }

    #[test]
    fn add_then_remove_yields_empty() {
        let s = OrSet::new()
            .apply(Dot::new(1, 0), OrSetOp::Add { elem: 42u32 })
            .unwrap()
            .apply(
                Dot::new(u64::MAX, 0),
                OrSetOp::Remove {
                    observed: Dot::new(1, 0),
                },
            )
            .unwrap();
        assert_eq!(s.len(), 0);
        assert!(s.is_empty());
        assert!(!s.contains(&42));
    }

    #[test]
    fn remove_existing_dead_idempotent() {
        let s = OrSet::new()
            .apply(Dot::new(1, 0), OrSetOp::Add { elem: 42u32 })
            .unwrap()
            .apply(
                Dot::new(u64::MAX, 0),
                OrSetOp::Remove {
                    observed: Dot::new(1, 0),
                },
            )
            .unwrap()
            .apply(
                Dot::new(u64::MAX, 1),
                OrSetOp::Remove {
                    observed: Dot::new(1, 0),
                },
            )
            .unwrap();
        assert_eq!(s.len(), 0);
        assert!(!s.contains(&42));
    }

    #[test]
    fn remove_unseen_then_add_arrives_dead() {
        let s = OrSet::new()
            .apply(
                Dot::new(u64::MAX, 0),
                OrSetOp::Remove {
                    observed: Dot::new(1, 0),
                },
            )
            .unwrap()
            .apply(Dot::new(1, 0), OrSetOp::Add { elem: 42u32 })
            .unwrap();
        assert_eq!(s.len(), 0);
        assert!(!s.contains(&42));
    }

    #[test]
    fn remove_twice_pending_idempotent() {
        // Applying Remove twice on an unseen Dot must register the tombstone exactly
        // once so that the late-arriving Add sees the same dead state either way.
        let s1 = OrSet::new()
            .apply(
                Dot::new(u64::MAX, 0),
                OrSetOp::Remove {
                    observed: Dot::new(1, 0),
                },
            )
            .unwrap();
        let s2 = s1
            .apply(
                Dot::new(u64::MAX, 1),
                OrSetOp::Remove {
                    observed: Dot::new(1, 0),
                },
            )
            .unwrap();
        let after_add_1 = s1
            .apply(Dot::new(1, 0), OrSetOp::Add { elem: 42u32 })
            .unwrap();
        let after_add_2 = s2
            .apply(Dot::new(1, 0), OrSetOp::Add { elem: 42u32 })
            .unwrap();
        assert_eq!(after_add_1, after_add_2);
        assert!(after_add_1.is_empty());
    }

    #[test]
    fn remove_unseen_other_ops_then_target_arrives() {
        let s = OrSet::new()
            .apply(
                Dot::new(u64::MAX, 0),
                OrSetOp::Remove {
                    observed: Dot::new(2, 0),
                },
            )
            .unwrap()
            .apply(Dot::new(1, 0), OrSetOp::Add { elem: 1u32 })
            .unwrap()
            .apply(Dot::new(2, 0), OrSetOp::Add { elem: 2u32 })
            .unwrap();
        assert!(s.contains(&1));
        assert!(!s.contains(&2));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn permutation_add_remove_vs_remove_add_converges() {
        let id_a = Dot::new(1, 0);
        let id_r = Dot::new(u64::MAX, 0);
        let op_a = OrSetOp::Add { elem: 42u32 };
        let op_r = OrSetOp::Remove {
            observed: Dot::new(1, 0),
        };
        let s1 = OrSet::new()
            .apply(id_a, op_a.clone())
            .unwrap()
            .apply(id_r, op_r.clone())
            .unwrap();
        let s2 = OrSet::new()
            .apply(id_r, op_r)
            .unwrap()
            .apply(id_a, op_a)
            .unwrap();
        assert_eq!(s1, s2);
        assert!(s1.is_empty());
    }

    #[test]
    fn remove_one_tag_keeps_element_add_wins() {
        let s = OrSet::new()
            .apply(Dot::new(1, 0), OrSetOp::Add { elem: 42u32 })
            .unwrap()
            .apply(Dot::new(2, 0), OrSetOp::Add { elem: 42u32 })
            .unwrap()
            .apply(
                Dot::new(u64::MAX, 0),
                OrSetOp::Remove {
                    observed: Dot::new(1, 0),
                },
            )
            .unwrap();
        assert_eq!(s.len(), 1);
        assert!(s.contains(&42));
    }

    #[test]
    fn tags_for_returns_alive_dots_only() {
        let s = OrSet::new()
            .apply(Dot::new(1, 0), OrSetOp::Add { elem: 42u32 })
            .unwrap()
            .apply(Dot::new(2, 0), OrSetOp::Add { elem: 42u32 })
            .unwrap()
            .apply(Dot::new(3, 0), OrSetOp::Add { elem: 99u32 })
            .unwrap()
            .apply(
                Dot::new(u64::MAX, 0),
                OrSetOp::Remove {
                    observed: Dot::new(1, 0),
                },
            )
            .unwrap();

        let tags_42: hashbrown::HashSet<Dot> = s.tags_for(&42).copied().collect();
        let tags_99: hashbrown::HashSet<Dot> = s.tags_for(&99).copied().collect();
        let tags_missing: Vec<&Dot> = s.tags_for(&999).collect();

        assert_eq!(tags_42.len(), 1);
        assert!(tags_42.contains(&Dot::new(2, 0)));
        assert!(!tags_42.contains(&Dot::new(1, 0)));
        assert_eq!(tags_99.len(), 1);
        assert!(tags_99.contains(&Dot::new(3, 0)));
        assert!(tags_missing.is_empty());
    }

    #[test]
    fn empty_orset_to_plain_is_empty_btreeset() {
        let s: OrSet<u32> = OrSet::new();
        assert_eq!(s.to_plain(), BTreeSet::new());
    }

    #[test]
    fn orset_to_plain_yields_alive_sorted() {
        let s = OrSet::<u32>::new()
            .apply(Dot::new(1, 0), OrSetOp::Add { elem: 2 })
            .unwrap()
            .apply(Dot::new(1, 1), OrSetOp::Add { elem: 1 })
            .unwrap();
        let mut expected = BTreeSet::new();
        expected.insert(1u32);
        expected.insert(2u32);
        assert_eq!(s.to_plain(), expected);
    }

    #[test]
    fn orset_to_plain_skips_tombstoned() {
        let d_a = Dot::new(1, 0);
        let d_remove = Dot::new(u64::MAX, 0);
        let s = OrSet::<u32>::new()
            .apply(d_a, OrSetOp::Add { elem: 7 })
            .unwrap()
            .apply(Dot::new(1, 1), OrSetOp::Add { elem: 9 })
            .unwrap()
            .apply(d_remove, OrSetOp::Remove { observed: d_a })
            .unwrap();
        let mut expected = BTreeSet::new();
        expected.insert(9u32);
        assert_eq!(s.to_plain(), expected);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::test_utils::permute;
    use hashbrown::HashMap;
    use proptest::prelude::*;

    /// Generate a sequence of *causally-valid* OR-Set ops:
    /// each Remove targets a Dot already produced by an earlier Add in this sequence.
    /// `apply` handles out-of-order delivery, but the generator models a realistic
    /// editor session; out-of-order arrival is exercised separately by permuting.
    pub(super) fn arb_op_sequence(
        max_ops: usize,
        num_actors: u64,
        elem_domain: u32,
    ) -> impl Strategy<Value = Vec<(Dot, OrSetOp<u32>)>> {
        let domain = elem_domain.max(1);
        proptest::collection::vec(
            (0u64..num_actors, any::<bool>(), any::<u8>(), 0u32..domain),
            0..=max_ops,
        )
        .prop_map(build_ops)
    }

    fn build_ops(raw: Vec<(u64, bool, u8, u32)>) -> Vec<(Dot, OrSetOp<u32>)> {
        let mut clocks: HashMap<u64, u64> = HashMap::new();
        let mut existing: Vec<Dot> = Vec::new();
        let mut ops: Vec<(Dot, OrSetOp<u32>)> = Vec::new();
        let mut remove_counter: u64 = 0;

        // No drop path: when want_remove=true but existing is empty the loop falls
        // through to the Add branch. Every raw entry produces exactly one op, keeping
        // the generator non-vacuous.
        for (actor, want_remove, target_byte, elem) in raw {
            let do_remove = want_remove && !existing.is_empty();
            if do_remove {
                let observed = existing[(target_byte as usize) % existing.len()];
                let id = Dot::new(u64::MAX, remove_counter);
                remove_counter += 1;
                ops.push((id, OrSetOp::Remove { observed }));
                continue;
            }
            let clock = clocks.entry(actor).or_insert(0);
            let id = Dot::new(actor, *clock);
            *clock += 1;
            ops.push((id, OrSetOp::Add { elem }));
            existing.push(id);
        }
        ops
    }

    pub(super) fn apply_all(ops: &[(Dot, OrSetOp<u32>)]) -> OrSet<u32> {
        ops.iter()
            .cloned()
            .fold(OrSet::new(), |s, (id, op)| s.apply(id, op).unwrap())
    }

    #[test]
    fn build_ops_smoke() {
        let ops = build_ops(vec![(0, false, 0, 1), (0, false, 0, 2), (1, false, 0, 1)]);
        assert_eq!(ops.len(), 3);
        let s = apply_all(&ops);
        assert_eq!(s.len(), 2);
    }

    proptest! {
        /// Applying a seed-derived permutation of an arbitrary op set yields the same
        /// final state as the original order. Not exhaustive over all permutations —
        /// proptest's randomized sampling provides reasonable statistical evidence.
        /// Compares full state equality (entries + pending_tombstones), not just the
        /// visible element set: hidden state divergence surfaces only after subsequent
        /// ops are applied.
        #[test]
        fn convergence_under_permutation(
            ops in arb_op_sequence(30, 3, 5),
            seed in any::<u64>(),
        ) {
            let permuted = permute(&ops, seed);
            let s1 = apply_all(&ops);
            let s2 = apply_all(&permuted);
            prop_assert_eq!(s1, s2);
        }
    }

    proptest! {
        /// Applying a seed-derived permutation of a multiset that contains each op
        /// twice yields the same result as applying each op once. Models at-least-once
        /// delivery where duplicates are interleaved with other ops rather than adjacent.
        #[test]
        fn idempotency_under_permutation(
            ops in arb_op_sequence(20, 2, 4),
            seed in any::<u64>(),
        ) {
            let single = apply_all(&ops);
            let doubled: Vec<(Dot, OrSetOp<u32>)> = ops
                .iter()
                .flat_map(|p| [p.clone(), p.clone()])
                .collect();
            let permuted_doubled = permute(&doubled, seed);
            let s2 = apply_all(&permuted_doubled);
            prop_assert_eq!(single, s2);
        }
    }

    proptest! {
        /// 2–5 actors emit Add and Remove ops over a small element domain. The
        /// structure is deterministic — each actor adds elem 0 at i=0, so all actors
        /// hold concurrent tokens on the same element, guaranteeing the multi-actor
        /// shared-elem scenario regardless of how proptest samples num_actors and
        /// ops_per_actor. Randomness comes from those parameters and the permutation seed.
        #[test]
        fn multi_actor_concurrent_over_shared_elems(
            num_actors in 2u64..=5,
            ops_per_actor in 2usize..=5,
            elem_domain in 2u32..=4,
            seed in any::<u64>(),
        ) {
            let mut ops: Vec<(Dot, OrSetOp<u32>)> = Vec::new();
            let mut existing: Vec<Dot> = Vec::new();
            let mut remove_counter: u64 = 0;

            for actor in 1..=num_actors {
                for i in 0..ops_per_actor {
                    let id = Dot::new(actor, i as u64);
                    if i % 2 == 0 || existing.is_empty() {
                        let elem = (i as u32) % elem_domain;
                        ops.push((id, OrSetOp::Add { elem }));
                        existing.push(id);
                    } else {
                        // Cross-actor target: (i + actor) % len selects a different
                        // index per actor, so Remove occasionally kills another actor's
                        // token and exercises add-wins under selective remove.
                        let observed = existing[(i + actor as usize) % existing.len()];
                        ops.push((Dot::new(u64::MAX, remove_counter), OrSetOp::Remove { observed }));
                        remove_counter += 1;
                    }
                }
            }

            let s1 = apply_all(&ops);
            let s2 = apply_all(&permute(&ops, seed));
            prop_assert_eq!(s1, s2);
        }
    }

    proptest! {
        /// Complex tombstone scenario:
        /// (1) Same elem gets multiple Adds with distinct Dots — guaranteed because
        ///     num_dots >= 2 * elem_domain and elems are assigned round-robin.
        /// (2) Multiple Removes with duplicates allowed — exercises idempotency alongside
        ///     the tombstone path.
        /// (3) Remove-before-Add via a pending tombstone — the last Dot is targeted by
        ///     a Remove placed at the front of the sequence, before the Add that creates
        ///     it, so the pending tombstone path is hit on natural apply order without
        ///     relying on permutation randomness.
        /// All permutations must reach the same final state.
        #[test]
        fn complex_tombstone(
            // num_dots = elem_domain*2 + extra_dots forces multiple Adds per elem.
            elem_domain in 2u32..=3,
            extra_dots in 0usize..=4,
            // Duplicate indices allowed — idempotency exercised alongside tombstones.
            remove_indices in proptest::collection::vec(0usize..8, 2..=5),
            seed in any::<u64>(),
        ) {
            let num_dots = (elem_domain as usize) * 2 + extra_dots;
            let mut ops: Vec<(Dot, OrSetOp<u32>)> = Vec::new();
            let mut existing: Vec<Dot> = Vec::new();
            let mut remove_counter: u64 = 0;

            // (3) Force pending tombstone: target the last Dot before it is added.
            let pending_observed = Dot::new(0, (num_dots as u64) - 1);
            ops.push((Dot::new(u64::MAX, remove_counter), OrSetOp::Remove { observed: pending_observed }));
            remove_counter += 1;

            // (1) Single actor, clock advancing, elem assigned round-robin.
            for i in 0..num_dots as u64 {
                let id = Dot::new(0, i);
                let elem = (i as u32) % elem_domain;
                ops.push((id, OrSetOp::Add { elem }));
                existing.push(id);
            }

            // (2) Remove sequence — duplicates allowed.
            for &idx in &remove_indices {
                let observed = existing[idx % existing.len()];
                ops.push((Dot::new(u64::MAX, remove_counter), OrSetOp::Remove { observed }));
                remove_counter += 1;
            }

            let s1 = apply_all(&ops);
            let s2 = apply_all(&permute(&ops, seed));
            prop_assert_eq!(s1, s2);
        }
    }

    proptest! {
        /// After every op apply, `len()` and `iter().count()` must agree with an
        /// independent oracle that counts distinct alive elements directly from entries.
        /// Permutation forces transient Remove-before-Add states through the pending
        /// tombstone path. The oracle catches dedup-arithmetic bugs but not alive-flag
        /// bugs — those are covered by other unit tests.
        #[test]
        fn len_iter_consistency(
            ops in arb_op_sequence(30, 3, 5),
            seed in any::<u64>(),
        ) {
            let permuted = permute(&ops, seed);
            let mut state: OrSet<u32> = OrSet::new();
            for (id, op) in &permuted {
                state = state.apply(*id, op.clone()).unwrap();
                let oracle: hashbrown::HashSet<u32> = state
                    .entries
                    .iter()
                    .filter(|(_, e)| e.alive)
                    .map(|(_, e)| e.elem)
                    .collect();
                prop_assert_eq!(
                    state.len(),
                    oracle.len(),
                    "len vs oracle mismatch after applying ({:?}, {:?})", id, op
                );
                prop_assert_eq!(
                    state.iter().count(),
                    oracle.len(),
                    "iter().count() vs oracle mismatch after applying ({:?}, {:?})", id, op
                );
            }
        }
    }
}
