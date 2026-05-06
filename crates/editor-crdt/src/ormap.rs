use serde::{Deserialize, Serialize};
use std::hash::Hash;

use crate::{CrdtError, Dot, ToPlain};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrMapOp<K, V> {
    Set {
        key: K,
        value: V,
    },
    /// `observed` — the add-token dots this unset has observed at generation time.
    /// Concurrent adds whose dots are not in `observed` survive (add-wins): the
    /// unset cannot kill what it did not see.
    /// `observed` must be ascending-sorted and deduplicated by the op generator
    /// (canonical wire form for hash/equality stability).
    Unset {
        observed: Vec<Dot>,
    },
}

/// **Standalone-POC representation — do not embed in an editor as-is.**
/// `get()` / `contains_key()` / `iter()` / `len()` are O(n) full-scan over entries.
/// Editor integration must replace this with an inverted index `K → HashSet<Dot>`
/// (or a per-K winner cache) before exposing `OrMap<K, V>` to user-facing operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrMap<K, V> {
    entries: imbl::HashMap<Dot, OrMapEntry<K, V>>,
    pending_tombstones: imbl::HashSet<Dot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OrMapEntry<K, V> {
    key: K,
    value: V,
    alive: bool,
}

impl<K, V> OrMap<K, V> {
    pub fn new() -> Self {
        Self {
            entries: imbl::HashMap::new(),
            pending_tombstones: imbl::HashSet::new(),
        }
    }
}

impl<K: Clone + Eq + Hash, V: Clone + Eq> OrMap<K, V> {
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.entries
            .iter()
            .filter(|(_, e)| e.alive && &e.key == key)
            .max_by_key(|(d, _)| *d)
            .map(|(_, e)| &e.value)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> + '_ {
        let mut winners: std::collections::HashMap<&K, (Dot, &V)> =
            std::collections::HashMap::new();
        for (dot, entry) in self.entries.iter() {
            if !entry.alive {
                continue;
            }
            winners
                .entry(&entry.key)
                .and_modify(|(d, v)| {
                    if *dot > *d {
                        *d = *dot;
                        *v = &entry.value;
                    }
                })
                .or_insert((*dot, &entry.value));
        }
        winners.into_iter().map(|(k, (_, v))| (k, v))
    }

    pub fn tags_for<'a>(&'a self, key: &'a K) -> impl Iterator<Item = &'a Dot> + 'a {
        self.entries.iter().filter_map(move |(dot, e)| {
            if e.alive && &e.key == key {
                Some(dot)
            } else {
                None
            }
        })
    }

    pub fn apply(&self, id: Dot, op: OrMapOp<K, V>) -> Result<Self, CrdtError> {
        match op {
            OrMapOp::Set { key, value } => self.apply_set(id, key, value),
            OrMapOp::Unset { observed } => Ok(self.apply_unset(observed)),
        }
    }

    fn apply_set(&self, id: Dot, key: K, value: V) -> Result<Self, CrdtError> {
        if let Some(existing) = self.entries.get(&id) {
            if existing.key != key || existing.value != value {
                return Err(CrdtError::DotConflict { dot: id });
            }
            return Ok(self.clone());
        }
        let alive = !self.pending_tombstones.contains(&id);
        let entry = OrMapEntry { key, value, alive };
        Ok(Self {
            entries: self.entries.update(id, entry),
            pending_tombstones: self.pending_tombstones.without(&id),
        })
    }

    fn apply_unset(&self, observed: Vec<Dot>) -> Self {
        let mut entries = self.entries.clone();
        let mut pending = self.pending_tombstones.clone();
        for dot in observed {
            match entries.get(&dot) {
                Some(entry) if entry.alive => {
                    let new_entry = OrMapEntry {
                        key: entry.key.clone(),
                        value: entry.value.clone(),
                        alive: false,
                    };
                    entries = entries.update(dot, new_entry);
                }
                Some(_) => {}
                None => {
                    pending = pending.update(dot);
                }
            }
        }
        Self {
            entries,
            pending_tombstones: pending,
        }
    }
}

impl<K, V> Default for OrMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> ToPlain for OrMap<K, V>
where
    K: Clone + Eq + std::hash::Hash + Ord,
    V: Clone + Eq,
{
    type Plain = std::collections::BTreeMap<K, V>;
    fn to_plain(&self) -> Self::Plain {
        self.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_state() {
        let m: OrMap<u32, u32> = OrMap::new();
        assert_eq!(m.len(), 0);
        assert!(m.is_empty());
        assert_eq!(m.get(&42), None);
        assert!(!m.contains_key(&42));
        assert_eq!(m.iter().count(), 0);
        assert_eq!(m.tags_for(&42).count(), 0);
    }

    #[test]
    fn default_equals_new() {
        let a: OrMap<u32, u32> = OrMap::new();
        let b: OrMap<u32, u32> = OrMap::default();
        assert_eq!(a, b);
    }

    #[test]
    fn set_single_key() {
        let m = OrMap::<u32, u32>::new()
            .apply(Dot::new(1, 0), OrMapOp::Set { key: 7, value: 42 })
            .unwrap();
        assert_eq!(m.len(), 1);
        assert!(m.contains_key(&7));
        assert_eq!(m.get(&7), Some(&42));
        assert_eq!(m.iter().count(), 1);
    }

    #[test]
    fn set_two_distinct_keys() {
        let m = OrMap::<u32, u32>::new()
            .apply(Dot::new(1, 0), OrMapOp::Set { key: 7, value: 42 })
            .unwrap()
            .apply(Dot::new(1, 1), OrMapOp::Set { key: 9, value: 99 })
            .unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(&7), Some(&42));
        assert_eq!(m.get(&9), Some(&99));
    }

    #[test]
    fn set_then_unset_yields_empty() {
        let m = OrMap::<u32, u32>::new()
            .apply(Dot::new(1, 0), OrMapOp::Set { key: 7, value: 42 })
            .unwrap()
            .apply(
                Dot::new(u64::MAX, 0),
                OrMapOp::Unset {
                    observed: vec![Dot::new(1, 0)],
                },
            )
            .unwrap();
        assert_eq!(m.len(), 0);
        assert!(m.is_empty());
        assert_eq!(m.get(&7), None);
        assert!(!m.contains_key(&7));
        assert_eq!(m.tags_for(&7).count(), 0);
    }

    #[test]
    fn partial_unset_preserves_remaining_alive() {
        let m = OrMap::<u32, u32>::new()
            .apply(Dot::new(1, 0), OrMapOp::Set { key: 7, value: 42 })
            .unwrap()
            .apply(Dot::new(2, 0), OrMapOp::Set { key: 7, value: 99 })
            .unwrap()
            .apply(
                Dot::new(u64::MAX, 0),
                OrMapOp::Unset {
                    observed: vec![Dot::new(1, 0)],
                },
            )
            .unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m.get(&7), Some(&99));
        let tags: Vec<Dot> = m.tags_for(&7).copied().collect();
        assert_eq!(tags, vec![Dot::new(2, 0)]);
    }

    #[test]
    fn unset_dead_entry_idempotent() {
        let unset_op = OrMapOp::Unset {
            observed: vec![Dot::new(1, 0)],
        };
        let m = OrMap::<u32, u32>::new()
            .apply(Dot::new(1, 0), OrMapOp::Set { key: 7, value: 42 })
            .unwrap()
            .apply(Dot::new(u64::MAX, 0), unset_op.clone())
            .unwrap()
            .apply(Dot::new(u64::MAX, 1), unset_op)
            .unwrap();
        assert_eq!(m.len(), 0);
        assert_eq!(m.get(&7), None);
    }

    #[test]
    fn set_same_dot_same_payload_idempotent() {
        let id = Dot::new(1, 0);
        let op = OrMapOp::Set {
            key: 7u32,
            value: 42u32,
        };
        let m = OrMap::<u32, u32>::new()
            .apply(id, op.clone())
            .unwrap()
            .apply(id, op.clone())
            .unwrap()
            .apply(id, op)
            .unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m.get(&7), Some(&42));
    }

    #[test]
    fn set_same_dot_different_value_returns_dot_conflict() {
        let id = Dot::new(1, 0);
        let m = OrMap::<u32, u32>::new()
            .apply(id, OrMapOp::Set { key: 7, value: 42 })
            .unwrap();
        let result = m.apply(id, OrMapOp::Set { key: 7, value: 99 });
        assert_eq!(result, Err(CrdtError::DotConflict { dot: id }));
    }

    #[test]
    fn set_same_dot_different_key_returns_dot_conflict() {
        let id = Dot::new(1, 0);
        let m = OrMap::<u32, u32>::new()
            .apply(id, OrMapOp::Set { key: 7, value: 42 })
            .unwrap();
        let result = m.apply(id, OrMapOp::Set { key: 9, value: 42 });
        assert_eq!(result, Err(CrdtError::DotConflict { dot: id }));
    }

    #[test]
    fn same_key_two_dots_max_wins() {
        // Dot::cmp clock-primary 의 직접 verify: (1,5) clock=5 > (2,3) clock=3.
        // Higher clock 의 lower actor 가 winner — actor-primary 로 swap 시 fail.
        let d_winner = Dot::new(1, 5);
        let d_loser = Dot::new(2, 3);
        let m = OrMap::<u32, u32>::new()
            .apply(d_winner, OrMapOp::Set { key: 7, value: 42 })
            .unwrap()
            .apply(d_loser, OrMapOp::Set { key: 7, value: 99 })
            .unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m.get(&7), Some(&42));
        let tags: std::collections::HashSet<Dot> = m.tags_for(&7).copied().collect();
        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&d_winner));
        assert!(tags.contains(&d_loser));
    }

    #[test]
    fn unset_unseen_then_set_arrives_dead() {
        let m = OrMap::<u32, u32>::new()
            .apply(
                Dot::new(u64::MAX, 0),
                OrMapOp::Unset {
                    observed: vec![Dot::new(1, 0)],
                },
            )
            .unwrap()
            .apply(Dot::new(1, 0), OrMapOp::Set { key: 7, value: 42 })
            .unwrap();
        assert_eq!(m.len(), 0);
        assert_eq!(m.get(&7), None);
        assert_eq!(m.tags_for(&7).count(), 0);
    }

    #[test]
    fn unset_unseen_idempotent_pending() {
        let unset = OrMapOp::Unset {
            observed: vec![Dot::new(1, 0)],
        };
        let m1 = OrMap::<u32, u32>::new()
            .apply(Dot::new(u64::MAX, 0), unset.clone())
            .unwrap();
        let m2 = m1.apply(Dot::new(u64::MAX, 1), unset).unwrap();
        let after_set_1 = m1
            .apply(Dot::new(1, 0), OrMapOp::Set { key: 7, value: 42 })
            .unwrap();
        let after_set_2 = m2
            .apply(Dot::new(1, 0), OrMapOp::Set { key: 7, value: 42 })
            .unwrap();
        assert_eq!(after_set_1, after_set_2);
        assert!(after_set_1.is_empty());
    }

    #[test]
    fn unset_unseen_other_ops_then_target_arrives() {
        let m = OrMap::<u32, u32>::new()
            .apply(
                Dot::new(u64::MAX, 0),
                OrMapOp::Unset {
                    observed: vec![Dot::new(2, 0)],
                },
            )
            .unwrap()
            .apply(Dot::new(1, 0), OrMapOp::Set { key: 7, value: 1 })
            .unwrap()
            .apply(Dot::new(2, 0), OrMapOp::Set { key: 9, value: 2 })
            .unwrap();
        assert_eq!(m.get(&7), Some(&1));
        assert_eq!(m.get(&9), None);
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn unset_only_affects_observed_dots_across_keys() {
        // Unset 의 observed = K1 의 dot. K2 의 alive token 영향 없음.
        let d_k1 = Dot::new(1, 0);
        let d_k2 = Dot::new(1, 1);
        let m = OrMap::<u32, u32>::new()
            .apply(d_k1, OrMapOp::Set { key: 7, value: 42 })
            .unwrap()
            .apply(d_k2, OrMapOp::Set { key: 9, value: 99 })
            .unwrap()
            .apply(
                Dot::new(u64::MAX, 0),
                OrMapOp::Unset {
                    observed: vec![d_k1],
                },
            )
            .unwrap();
        assert_eq!(m.get(&7), None, "K1 (with d_k1) tombstoned");
        assert_eq!(
            m.get(&9),
            Some(&99),
            "K2 (with d_k2 NOT in observed) untouched"
        );
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn add_wins_concurrent_set_vs_unset_concrete() {
        // Two concurrent Sets on K=7: d3=(1,3) value 11, d2=(1,2) value 22.
        // d3 > d2 by Dot::cmp (clock primary). Then Unset(observed={d3}).
        // After apply: d3 alive=false, d2 alive=true.
        // get(K=7) returns the alive d2's value (22), NOT the dead d3's value (11) —
        // a global-monotonic-ts winner registry would surface 11 here, since d3
        // would remain the highest-ever-applied ts. OrMap's alive-only LWW excludes d3.
        let d2 = Dot::new(1, 2);
        let d3 = Dot::new(1, 3);
        let d4 = Dot::new(u64::MAX, 0);
        let m = OrMap::<u32, u32>::new()
            .apply(d3, OrMapOp::Set { key: 7, value: 11 })
            .unwrap()
            .apply(d2, OrMapOp::Set { key: 7, value: 22 })
            .unwrap()
            .apply(d4, OrMapOp::Unset { observed: vec![d3] })
            .unwrap();
        assert_eq!(
            m.get(&7),
            Some(&22),
            "alive (d2) wins, dead (d3) does not surface"
        );
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn iter_dedups_by_key() {
        // 두 개의 alive set tokens for K=7 (different dots, different values).
        // iter() 는 K 별 dedup → 1 회 emit, value = max(Dot) winner.
        let m = OrMap::<u32, u32>::new()
            .apply(Dot::new(1, 0), OrMapOp::Set { key: 7, value: 42 })
            .unwrap()
            .apply(Dot::new(2, 0), OrMapOp::Set { key: 7, value: 99 })
            .unwrap()
            .apply(Dot::new(3, 0), OrMapOp::Set { key: 9, value: 1 })
            .unwrap();
        let mut entries: Vec<(u32, u32)> = m.iter().map(|(k, v)| (*k, *v)).collect();
        entries.sort();
        assert_eq!(entries, vec![(7, 99), (9, 1)]);
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn empty_ormap_to_plain_is_empty_btreemap() {
        use std::collections::BTreeMap;
        let m: OrMap<u32, u32> = OrMap::new();
        assert_eq!(m.to_plain(), BTreeMap::new());
    }

    #[test]
    fn ormap_to_plain_returns_canonical_winners() {
        use std::collections::BTreeMap;
        let m = OrMap::<u32, u32>::new()
            .apply(Dot::new(1, 0), OrMapOp::Set { key: 7, value: 42 })
            .unwrap()
            .apply(Dot::new(1, 1), OrMapOp::Set { key: 9, value: 99 })
            .unwrap();
        let mut expected = BTreeMap::new();
        expected.insert(7u32, 42u32);
        expected.insert(9u32, 99u32);
        assert_eq!(m.to_plain(), expected);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::test_utils::permute;
    use proptest::prelude::*;
    use std::collections::HashMap;

    /// Generate a sequence of *causally-valid* OrMap ops. Each Set produces a fresh
    /// dot per actor; each Unset's observed holds the K-atomic alive set at generation
    /// time. `apply` accepts out-of-order delivery, but the generator models a realistic
    /// editor session; out-of-order arrival is exercised separately by permuting.
    pub(super) fn arb_op_sequence(
        max_ops: usize,
        num_actors: u64,
        key_domain: u32,
        value_domain: u32,
    ) -> impl Strategy<Value = Vec<(Dot, OrMapOp<u32, u32>)>> {
        let key_dom = key_domain.max(1);
        let value_dom = value_domain.max(1);
        proptest::collection::vec(
            (
                0u64..num_actors,
                any::<bool>(),
                0u32..key_dom,
                0u32..value_dom,
            ),
            0..=max_ops,
        )
        .prop_map(build_ops)
    }

    fn build_ops(raw: Vec<(u64, bool, u32, u32)>) -> Vec<(Dot, OrMapOp<u32, u32>)> {
        let mut clocks: HashMap<u64, u64> = HashMap::new();
        // K → alive dots (K-atomic invariant — generator collects K's full alive set on unset).
        let mut alive_per_key: HashMap<u32, Vec<Dot>> = HashMap::new();
        let mut ops: Vec<(Dot, OrMapOp<u32, u32>)> = Vec::new();
        let mut unset_counter: u64 = 0;

        for (actor, want_unset, key, value) in raw {
            let do_unset = want_unset
                && alive_per_key
                    .get(&key)
                    .map(|v| !v.is_empty())
                    .unwrap_or(false);
            if do_unset {
                let mut observed: Vec<Dot> = alive_per_key.remove(&key).unwrap_or_default();
                // Canonicalize: sort + dedup for hash/equality stability.
                observed.sort();
                observed.dedup();
                let id = Dot::new(u64::MAX, unset_counter);
                unset_counter += 1;
                ops.push((id, OrMapOp::Unset { observed }));
                continue;
            }
            let clock = clocks.entry(actor).or_insert(0);
            let id = Dot::new(actor, *clock);
            *clock += 1;
            ops.push((id, OrMapOp::Set { key, value }));
            alive_per_key.entry(key).or_default().push(id);
        }
        ops
    }

    pub(super) fn apply_all(ops: &[(Dot, OrMapOp<u32, u32>)]) -> OrMap<u32, u32> {
        ops.iter()
            .cloned()
            .fold(OrMap::new(), |m, (id, op)| m.apply(id, op).unwrap())
    }

    #[test]
    fn build_ops_smoke() {
        let ops = build_ops(vec![
            (0, false, 7, 42),
            (0, false, 9, 99),
            (1, false, 7, 11),
        ]);
        assert_eq!(ops.len(), 3);
        let m = apply_all(&ops);
        // K=7 has two alive tokens (dots (0,0) and (1,0)), K=9 has one. iter dedups by K.
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn sample_state_shape_smoke() {
        // Mixed sequence — Set/Unset interleaved across two keys, two actors.
        // Verifies generator-augmented state shape after full apply: alive K count,
        // pending dot count after Unset-before-Set, distinct dots in entries.
        let d_a0 = Dot::new(1, 0);
        let d_a1 = Dot::new(1, 1);
        let d_b0 = Dot::new(2, 0);
        let d_unset = Dot::new(u64::MAX, 0);
        let d_pending = Dot::new(u64::MAX, 1);
        let ops = vec![
            (
                d_a0,
                OrMapOp::Set {
                    key: 7u32,
                    value: 1u32,
                },
            ),
            (d_a1, OrMapOp::Set { key: 9, value: 2 }),
            (d_b0, OrMapOp::Set { key: 7, value: 3 }),
            (
                d_unset,
                OrMapOp::Unset {
                    observed: vec![d_a0],
                },
            ),
            // Pending: observed dot not yet emitted in this sequence.
            (
                d_pending,
                OrMapOp::Unset {
                    observed: vec![Dot::new(99, 99)],
                },
            ),
        ];
        let m = apply_all(&ops);
        // K=7: alive d_b0 → value 3. K=9: alive d_a1 → value 2.
        assert_eq!(m.len(), 2);
        assert_eq!(m.get(&7), Some(&3));
        assert_eq!(m.get(&9), Some(&2));
        assert_eq!(m.entries.len(), 3);
        assert_eq!(m.pending_tombstones.len(), 1);
        assert!(m.pending_tombstones.contains(&Dot::new(99, 99)));
    }

    proptest! {
        /// Applying a seed-derived permutation of an arbitrary op set yields the same
        /// final state as the original order. proptest's randomized sampling provides
        /// statistical evidence (not exhaustive). Compares full state equality
        /// (entries + pending_tombstones), not just visible map: hidden state
        /// divergence surfaces only after subsequent ops applied.
        #[test]
        fn convergence_under_permutation(
            ops in arb_op_sequence(30, 3, 5, 4),
            seed in any::<u64>(),
        ) {
            let permuted = permute(&ops, seed);
            let m1 = apply_all(&ops);
            let m2 = apply_all(&permuted);
            prop_assert_eq!(m1, m2);
        }

        /// Multiset of each op duplicated, permuted — same final state as single.
        /// Models at-least-once delivery with duplicates interleaved.
        #[test]
        fn idempotency_under_permutation(
            ops in arb_op_sequence(20, 2, 4, 3),
            seed in any::<u64>(),
        ) {
            let single = apply_all(&ops);
            let doubled: Vec<_> = ops.iter().flat_map(|p| [p.clone(), p.clone()]).collect();
            let permuted_doubled = permute(&doubled, seed);
            let twice = apply_all(&permuted_doubled);
            prop_assert_eq!(single, twice);
        }

        /// Add-wins property — concurrent Set d_concurrent (not in any later Unset's
        /// observed) must survive in the final state. Witness: 3-op shape with
        /// different actors for d_first and d_concurrent (no per-actor clock chain
        /// implying observation), exhaustively over all 6 permutations.
        #[test]
        fn add_wins_concurrent_set_vs_unset_all_permutations(
            actor_first in 1u64..=5,
            actor_concurrent in 6u64..=10,
            key in 0u32..4,
            v_first in any::<u32>(),
            v_concurrent in any::<u32>(),
        ) {
            let d_first = Dot::new(actor_first, 0);
            let d_concurrent = Dot::new(actor_concurrent, 0);
            let d_unset = Dot::new(u64::MAX, 0);
            let op_first = (d_first, OrMapOp::Set { key, value: v_first });
            let op_unset = (d_unset, OrMapOp::Unset { observed: vec![d_first] });
            let op_concurrent = (d_concurrent, OrMapOp::Set { key, value: v_concurrent });

            let permutations: [Vec<(Dot, OrMapOp<u32, u32>)>; 6] = [
                vec![op_first.clone(), op_unset.clone(), op_concurrent.clone()],
                vec![op_first.clone(), op_concurrent.clone(), op_unset.clone()],
                vec![op_unset.clone(), op_first.clone(), op_concurrent.clone()],
                vec![op_unset.clone(), op_concurrent.clone(), op_first.clone()],
                vec![op_concurrent.clone(), op_first.clone(), op_unset.clone()],
                vec![op_concurrent, op_unset, op_first],
            ];
            for perm in &permutations {
                let m = apply_all(perm);
                // get(K) returns the surviving Set's value. Because clock=0 and
                // actor_concurrent > actor_first, Dot::cmp makes d_concurrent the
                // winner regardless of tombstoning correctness — so this assertion
                // alone passes even with broken tombstoning. tags_for assertion
                // below ensures the dead dot is actually removed from the live set.
                prop_assert_eq!(m.get(&key), Some(&v_concurrent), "d_concurrent must surface in perm {:?}", perm);
                prop_assert_eq!(m.len(), 1);
                let tags: std::collections::HashSet<Dot> = m.tags_for(&key).copied().collect();
                prop_assert!(tags.contains(&d_concurrent), "d_concurrent must be alive in perm {:?}", perm);
                prop_assert!(!tags.contains(&d_first), "d_first (in observed) must be dead in perm {:?}", perm);
                prop_assert_eq!(tags.len(), 1, "exactly one alive tag for the survivor in perm {:?}", perm);
            }
        }

        /// Winner = max(Dot) among K's alive tokens, verified by an independent
        /// op-stream replay oracle. The oracle is built from the applied op history
        /// directly, covering get/iter/tags_for on independent code paths.
        #[test]
        fn winner_is_max_alive_dot(
            ops in arb_op_sequence(25, 3, 4, 4),
            seed in any::<u64>(),
        ) {
            let permuted = permute(&ops, seed);
            let mut state: OrMap<u32, u32> = OrMap::new();
            let mut applied: Vec<(Dot, OrMapOp<u32, u32>)> = Vec::new();
            for (id, op) in &permuted {
                state = state.apply(*id, op.clone()).unwrap();
                applied.push((*id, op.clone()));

                // Op stream replay oracle — independent of OrMap internals.
                // alive_dots: dot → (key, value) for every Set ever observed
                //   (idempotent on duplicate dot — first-insert wins, matching apply's
                //    same-dot replay path which preserves the existing payload).
                // dead_dots: union of every Unset's observed.
                // Winner per K = max(Dot) where dot ∈ alive_dots, key matches K, dot ∉ dead_dots.
                let mut alive_dots: HashMap<Dot, (u32, u32)> = HashMap::new();
                let mut dead_dots: std::collections::HashSet<Dot> =
                    std::collections::HashSet::new();
                for (op_id, op_payload) in &applied {
                    match op_payload {
                        OrMapOp::Set { key: k, value: v } => {
                            alive_dots.entry(*op_id).or_insert((*k, *v));
                        }
                        OrMapOp::Unset { observed } => {
                            for d in observed {
                                dead_dots.insert(*d);
                            }
                        }
                    }
                }
                let mut expected: HashMap<u32, (Dot, u32)> = HashMap::new();
                for (dot, (k, v)) in &alive_dots {
                    if dead_dots.contains(dot) {
                        continue;
                    }
                    expected
                        .entry(*k)
                        .and_modify(|(d, val)| {
                            if *dot > *d {
                                *d = *dot;
                                *val = *v;
                            }
                        })
                        .or_insert((*dot, *v));
                }
                // Expected tags_for set per K.
                let mut expected_tags: HashMap<u32, std::collections::HashSet<Dot>> = HashMap::new();
                for (dot, (k, _v)) in &alive_dots {
                    if dead_dots.contains(dot) {
                        continue;
                    }
                    expected_tags.entry(*k).or_default().insert(*dot);
                }
                // Expected iter pairs (K → V at winner).
                let expected_pairs: HashMap<u32, u32> =
                    expected.iter().map(|(k, (_, v))| (*k, *v)).collect();

                for key in 0u32..4 {
                    let expected_value = expected.get(&key).map(|(_, v)| *v);
                    let actual = state.get(&key).copied();
                    prop_assert_eq!(
                        actual, expected_value,
                        "get mismatch for key {} after op ({:?}, {:?})", key, id, op
                    );
                    let actual_tags: std::collections::HashSet<Dot> =
                        state.tags_for(&key).copied().collect();
                    let expected_tags_for_k = expected_tags.get(&key).cloned().unwrap_or_default();
                    prop_assert_eq!(
                        actual_tags, expected_tags_for_k,
                        "tags_for mismatch for key {} after op ({:?}, {:?})", key, id, op
                    );
                }

                let actual_pairs: HashMap<u32, u32> =
                    state.iter().map(|(k, v)| (*k, *v)).collect();
                prop_assert_eq!(
                    actual_pairs, expected_pairs,
                    "iter pair mismatch after op ({:?}, {:?})", id, op
                );
            }
        }

        /// 2-5 actors emit Set/Unset over a small key/value domain. Structure's
        /// deterministic part — each actor's i=0 emits K=0 set — guarantees
        /// concurrent shared-K alive tokens. Randomness from num_actors,
        /// ops_per_actor, and permutation seed.
        #[test]
        fn multi_actor_convergence(
            num_actors in 2u64..=5,
            ops_per_actor in 2usize..=5,
            key_domain in 2u32..=4,
            value_domain in 2u32..=4,
            seed in any::<u64>(),
        ) {
            let mut ops: Vec<(Dot, OrMapOp<u32, u32>)> = Vec::new();
            // K → alive dots, for K-atomic unset construction.
            let mut alive_per_key: std::collections::HashMap<u32, Vec<Dot>> = std::collections::HashMap::new();
            let mut unset_counter: u64 = 0;

            for actor in 1..=num_actors {
                for i in 0..ops_per_actor {
                    let id = Dot::new(actor, i as u64);
                    let do_unset = i % 3 == 2 && {
                        let key = (i as u32) % key_domain;
                        alive_per_key.get(&key).map(|v| !v.is_empty()).unwrap_or(false)
                    };
                    if do_unset {
                        let key = (i as u32) % key_domain;
                        let mut observed: Vec<Dot> = alive_per_key.remove(&key).unwrap_or_default();
                        observed.sort();
                        observed.dedup();
                        ops.push((Dot::new(u64::MAX, unset_counter), OrMapOp::Unset { observed }));
                        unset_counter += 1;
                    } else {
                        let key = (i as u32) % key_domain;
                        let value = (actor as u32 * 17 + i as u32) % value_domain;
                        ops.push((id, OrMapOp::Set { key, value }));
                        alive_per_key.entry(key).or_default().push(id);
                    }
                }
            }

            let m1 = apply_all(&ops);
            let m2 = apply_all(&permute(&ops, seed));
            prop_assert_eq!(m1, m2);
        }

        /// After every op apply, `len()` and `iter().count()` agree with an
        /// independent oracle that counts distinct alive K's directly from entries.
        /// Permutation forces transient Unset-before-Set states through pending path.
        #[test]
        fn len_iter_consistency(
            ops in arb_op_sequence(30, 3, 5, 5),
            seed in any::<u64>(),
        ) {
            let permuted = permute(&ops, seed);
            let mut state: OrMap<u32, u32> = OrMap::new();
            for (id, op) in &permuted {
                state = state.apply(*id, op.clone()).unwrap();
                let oracle: std::collections::HashSet<u32> = state
                    .entries
                    .iter()
                    .filter(|(_, e)| e.alive)
                    .map(|(_, e)| e.key)
                    .collect();
                prop_assert_eq!(
                    state.len(),
                    oracle.len(),
                    "len mismatch after ({:?}, {:?})", id, op
                );
                prop_assert_eq!(
                    state.iter().count(),
                    oracle.len(),
                    "iter().count() mismatch after ({:?}, {:?})", id, op
                );
            }
        }
    }
}
