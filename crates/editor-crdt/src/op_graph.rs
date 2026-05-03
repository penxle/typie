use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

use crate::{CrdtError, Dot};

/// One node in the op-DAG. `id` is the op's unique identifier (also reused as
/// the semantic identifier — RGA element id, OR-Set add token — by the
/// payload). `parents` are the op-DAG parents of this op (the heads of the
/// store at the moment this op was created). Stored normalized: sorted
/// ascending, no duplicates.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Op<P> {
    pub id: Dot,
    pub parents: Vec<Dot>,
    pub payload: P,
}

/// Op-DAG storage. Mutable, append-only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpGraph<P> {
    actor: u64,
    next_clock: u64,
    ops: HashMap<Dot, Op<P>>,
    heads: HashSet<Dot>,
}

impl<P: PartialEq> OpGraph<P> {
    pub fn graph_state_eq(&self, other: &Self) -> bool {
        self.ops == other.ops && self.heads == other.heads
    }
}

impl<P> OpGraph<P> {
    pub fn new() -> Self {
        let mut buf = [0u8; 8];
        getrandom::fill(&mut buf).expect("failed to generate random bytes");
        Self::with_actor(u64::from_le_bytes(buf))
    }

    pub fn with_actor(actor: u64) -> Self {
        Self {
            actor,
            next_clock: 0,
            ops: HashMap::new(),
            heads: HashSet::new(),
        }
    }

    pub fn current_heads(&self) -> impl Iterator<Item = &Dot> + '_ {
        self.heads.iter()
    }

    pub fn contains(&self, dot: &Dot) -> bool {
        self.ops.contains_key(dot)
    }

    pub fn get(&self, dot: &Dot) -> Option<&Op<P>> {
        self.ops.get(dot)
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

impl<P> Default for OpGraph<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Clone> OpGraph<P> {
    pub fn add(&mut self, payload: P) -> Result<Op<P>, CrdtError> {
        let id = Dot::new(self.actor, self.next_clock);

        self.next_clock = self
            .next_clock
            .checked_add(1)
            .ok_or(CrdtError::ClockOverflow { dot: id })?;

        let mut parents: Vec<Dot> = self.heads.iter().copied().collect();
        parents.sort();

        let op = Op {
            id,
            parents: parents.clone(),
            payload,
        };
        self.ops.insert(id, op.clone());

        for p in &parents {
            self.heads.remove(p);
        }

        self.heads.insert(id);

        Ok(op)
    }
}

impl<P: Clone + Eq> OpGraph<P> {
    pub fn receive(&mut self, mut op: Op<P>) -> Result<(), CrdtError> {
        if op.parents.contains(&op.id) {
            return Err(CrdtError::SelfReference { dot: op.id });
        }

        op.parents.sort();
        op.parents.dedup();

        let missing: Vec<Dot> = op
            .parents
            .iter()
            .copied()
            .filter(|p| !self.ops.contains_key(p))
            .collect();
        if !missing.is_empty() {
            return Err(CrdtError::MissingParents {
                dot: op.id,
                missing,
            });
        }

        if let Some(existing) = self.ops.get(&op.id) {
            if existing.parents == op.parents && existing.payload == op.payload {
                return Ok(());
            }
            return Err(CrdtError::DotConflict { dot: op.id });
        }

        self.sync_clock_for(&op.id)?;

        for p in &op.parents {
            self.heads.remove(p);
        }

        self.heads.insert(op.id);
        self.ops.insert(op.id, op);

        Ok(())
    }

    fn sync_clock_for(&mut self, dot: &Dot) -> Result<(), CrdtError> {
        let advanced = dot
            .clock
            .checked_add(1)
            .ok_or(CrdtError::ClockOverflow { dot: *dot })?;
        self.next_clock = self.next_clock.max(advanced);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_first_op_genesis() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(1);
        let op = g.add(42).unwrap();
        assert_eq!(op.id, Dot::new(1, 0));
        assert!(op.parents.is_empty(), "first op is genesis (no parents)");
        assert_eq!(op.payload, 42);
        assert_eq!(g.len(), 1);
        let heads: Vec<&Dot> = g.current_heads().collect();
        assert_eq!(heads, vec![&Dot::new(1, 0)]);
    }

    #[test]
    fn add_second_op_parents_first() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(1);
        g.add(10).unwrap();
        let op2 = g.add(20).unwrap();
        assert_eq!(op2.id, Dot::new(1, 1));
        assert_eq!(op2.parents, vec![Dot::new(1, 0)]);
        assert_eq!(g.len(), 2);
        let heads: Vec<&Dot> = g.current_heads().collect();
        assert_eq!(heads, vec![&Dot::new(1, 1)], "only the latest op is head");
    }

    #[test]
    fn add_advances_next_clock() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(5);
        let op1 = g.add(1).unwrap();
        let op2 = g.add(2).unwrap();
        let op3 = g.add(3).unwrap();
        assert_eq!(op1.id, Dot::new(5, 0));
        assert_eq!(op2.id, Dot::new(5, 1));
        assert_eq!(op3.id, Dot::new(5, 2));
    }

    #[test]
    fn receive_genesis_op() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(0);
        let op = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 1,
        };
        g.receive(op.clone()).unwrap();
        assert_eq!(g.len(), 1);
        assert!(g.contains(&Dot::new(99, 0)));
        let heads: Vec<&Dot> = g.current_heads().collect();
        assert_eq!(heads, vec![&Dot::new(99, 0)]);
    }

    #[test]
    fn receive_linear_chain() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(0);
        let a = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 1,
        };
        let b = Op {
            id: Dot::new(99, 1),
            parents: vec![a.id],
            payload: 2,
        };
        let c = Op {
            id: Dot::new(99, 2),
            parents: vec![b.id],
            payload: 3,
        };
        g.receive(a).unwrap();
        g.receive(b).unwrap();
        g.receive(c).unwrap();
        assert_eq!(g.len(), 3);
        let heads: Vec<&Dot> = g.current_heads().collect();
        assert_eq!(heads, vec![&Dot::new(99, 2)], "only the leaf is head");
    }

    #[test]
    fn receive_branching_two_heads() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(0);
        let root = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 0,
        };
        let a = Op {
            id: Dot::new(1, 0),
            parents: vec![root.id],
            payload: 1,
        };
        let b = Op {
            id: Dot::new(2, 0),
            parents: vec![root.id],
            payload: 2,
        };
        g.receive(root).unwrap();
        g.receive(a).unwrap();
        g.receive(b).unwrap();
        let heads: HashSet<Dot> = g.current_heads().copied().collect();
        let expected: HashSet<Dot> = [Dot::new(1, 0), Dot::new(2, 0)].into_iter().collect();
        assert_eq!(heads, expected);
    }

    #[test]
    fn receive_merging_back_to_one_head() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(0);
        let root = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 0,
        };
        let a = Op {
            id: Dot::new(1, 0),
            parents: vec![root.id],
            payload: 1,
        };
        let b = Op {
            id: Dot::new(2, 0),
            parents: vec![root.id],
            payload: 2,
        };
        let m = Op {
            id: Dot::new(3, 0),
            parents: vec![a.id, b.id],
            payload: 3,
        };
        g.receive(root).unwrap();
        g.receive(a).unwrap();
        g.receive(b).unwrap();
        g.receive(m).unwrap();
        let heads: Vec<&Dot> = g.current_heads().collect();
        assert_eq!(
            heads,
            vec![&Dot::new(3, 0)],
            "merge collapses to single head"
        );
    }

    #[test]
    fn receive_self_reference_rejected() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(0);
        let id = Dot::new(99, 0);
        let op = Op {
            id,
            parents: vec![id],
            payload: 1,
        };
        let result = g.receive(op);
        assert_eq!(result, Err(CrdtError::SelfReference { dot: id }));
        assert_eq!(g.len(), 0, "rejected op must not enter the store");
    }

    #[test]
    fn receive_with_missing_parent_rejected() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(0);
        let missing = Dot::new(99, 0);
        let op = Op {
            id: Dot::new(99, 1),
            parents: vec![missing],
            payload: 1,
        };
        let result = g.receive(op);
        assert_eq!(
            result,
            Err(CrdtError::MissingParents {
                dot: Dot::new(99, 1),
                missing: vec![missing],
            })
        );
        assert_eq!(g.len(), 0);
    }

    #[test]
    fn receive_with_partial_missing_parents_lists_only_missing() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(0);
        let present = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 0,
        };
        g.receive(present.clone()).unwrap();
        let missing_a = Dot::new(99, 5);
        let missing_b = Dot::new(99, 6);
        let op = Op {
            id: Dot::new(99, 7),
            parents: vec![present.id, missing_a, missing_b],
            payload: 1,
        };
        let result = g.receive(op);
        // After internal sort, missing entries appear in sorted order.
        let mut expected_missing = vec![missing_a, missing_b];
        expected_missing.sort();
        assert_eq!(
            result,
            Err(CrdtError::MissingParents {
                dot: Dot::new(99, 7),
                missing: expected_missing,
            })
        );
    }

    #[test]
    fn receive_normalizes_duplicate_parents() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(0);
        let p = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 0,
        };
        g.receive(p.clone()).unwrap();
        let op_dup = Op {
            id: Dot::new(99, 1),
            parents: vec![p.id, p.id, p.id],
            payload: 1,
        };
        g.receive(op_dup).unwrap();
        let stored = g.get(&Dot::new(99, 1)).unwrap();
        assert_eq!(stored.parents, vec![p.id]);
    }

    #[test]
    fn receive_normalizes_unsorted_multi_parents() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(0);
        // Dot::Ord ranks (clock=0, actor=98) < (clock=0, actor=99).
        let a = Op {
            id: Dot::new(98, 0),
            parents: vec![],
            payload: 0,
        };
        let b = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 0,
        };
        g.receive(a.clone()).unwrap();
        g.receive(b.clone()).unwrap();

        let op_unsorted = Op {
            id: Dot::new(99, 1),
            parents: vec![b.id, a.id],
            payload: 1,
        };
        g.receive(op_unsorted).unwrap();
        let stored = g.get(&Dot::new(99, 1)).unwrap();
        assert_eq!(stored.parents, vec![a.id, b.id]);
    }

    #[test]
    fn receive_same_op_twice_is_idempotent() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(0);
        let op = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 7,
        };
        g.receive(op.clone()).unwrap();
        g.receive(op.clone()).unwrap();
        assert_eq!(g.len(), 1);
        assert_eq!(g.get(&Dot::new(99, 0)), Some(&op));
    }

    #[test]
    fn receive_same_dot_different_payload_rejected() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(0);
        let op_a = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 1,
        };
        let op_b = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 2,
        };
        g.receive(op_a.clone()).unwrap();
        let result = g.receive(op_b);
        assert_eq!(
            result,
            Err(CrdtError::DotConflict {
                dot: Dot::new(99, 0)
            })
        );
        assert_eq!(g.get(&Dot::new(99, 0)), Some(&op_a), "first wins");
    }

    #[test]
    fn receive_same_dot_different_parents_rejected() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(0);
        let root_a = Op {
            id: Dot::new(98, 0),
            parents: vec![],
            payload: 0,
        };
        let root_b = Op {
            id: Dot::new(97, 0),
            parents: vec![],
            payload: 0,
        };
        g.receive(root_a.clone()).unwrap();
        g.receive(root_b.clone()).unwrap();
        let op_x = Op {
            id: Dot::new(99, 0),
            parents: vec![root_a.id],
            payload: 1,
        };
        let op_y = Op {
            id: Dot::new(99, 0),
            parents: vec![root_b.id],
            payload: 1,
        };
        g.receive(op_x).unwrap();
        let result = g.receive(op_y);
        assert_eq!(
            result,
            Err(CrdtError::DotConflict {
                dot: Dot::new(99, 0)
            })
        );
    }

    #[test]
    fn receive_duplicate_does_not_resurrect_a_head() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(0);
        let root = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 0,
        };
        let child = Op {
            id: Dot::new(99, 1),
            parents: vec![root.id],
            payload: 1,
        };
        g.receive(root.clone()).unwrap();
        g.receive(child.clone()).unwrap();
        g.receive(root).unwrap();
        let heads: Vec<&Dot> = g.current_heads().collect();
        assert_eq!(heads, vec![&Dot::new(99, 1)]);
    }

    #[test]
    fn receive_advances_next_clock_past_observed_op() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(7);
        let observed = Op {
            id: Dot::new(99, 50),
            parents: vec![],
            payload: 1,
        };
        g.receive(observed).unwrap();
        let next = g.add(2).unwrap();
        assert_eq!(next.id, Dot::new(7, 51));
    }

    #[test]
    fn receive_lower_clock_does_not_lower_next_clock() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(7);
        let high = Op {
            id: Dot::new(99, 10),
            parents: vec![],
            payload: 1,
        };
        g.receive(high).unwrap();
        let low = Op {
            id: Dot::new(98, 3),
            parents: vec![],
            payload: 2,
        };
        g.receive(low).unwrap();
        let next = g.add(3).unwrap();
        assert_eq!(next.id, Dot::new(7, 11));
    }

    #[test]
    fn receive_idempotent_op_keeps_next_clock_advanced() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(7);
        let op = Op {
            id: Dot::new(99, 50),
            parents: vec![],
            payload: 1,
        };
        g.receive(op.clone()).unwrap();
        g.receive(op).unwrap();
        let next = g.add(2).unwrap();
        assert_eq!(next.id, Dot::new(7, 51));
    }

    #[test]
    fn receive_max_clock_returns_clock_overflow() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(7);
        let op = Op {
            id: Dot::new(99, u64::MAX),
            parents: vec![],
            payload: 1,
        };
        let result = g.receive(op);
        assert_eq!(
            result,
            Err(CrdtError::ClockOverflow {
                dot: Dot::new(99, u64::MAX),
            })
        );
        assert_eq!(g.len(), 0);
    }

    #[test]
    fn derive_partial_eq_compares_full_state() {
        let mut g1: OpGraph<u32> = OpGraph::with_actor(0);
        let mut g2: OpGraph<u32> = OpGraph::with_actor(42);
        let a = Op {
            id: Dot::new(1, 0),
            parents: vec![],
            payload: 1,
        };
        let b = Op {
            id: Dot::new(2, 0),
            parents: vec![],
            payload: 2,
        };
        g1.receive(a.clone()).unwrap();
        g1.receive(b.clone()).unwrap();
        g2.receive(b).unwrap();
        g2.receive(a).unwrap();
        assert_ne!(g1, g2);
    }

    #[test]
    fn graph_state_eq_ignores_actor_and_clock() {
        let mut g1: OpGraph<u32> = OpGraph::with_actor(0);
        let mut g2: OpGraph<u32> = OpGraph::with_actor(42);
        let a = Op {
            id: Dot::new(1, 0),
            parents: vec![],
            payload: 1,
        };
        let b = Op {
            id: Dot::new(2, 0),
            parents: vec![],
            payload: 2,
        };
        g1.receive(a.clone()).unwrap();
        g1.receive(b.clone()).unwrap();
        g2.receive(b).unwrap();
        g2.receive(a).unwrap();
        assert!(g1.graph_state_eq(&g2));
    }

    #[test]
    fn mixed_local_and_remote_then_merge() {
        let mut g: OpGraph<u32> = OpGraph::with_actor(1);
        let mine_a = g.add(10).unwrap();
        let theirs = Op {
            id: Dot::new(2, 0),
            parents: vec![],
            payload: 20,
        };
        g.receive(theirs.clone()).unwrap();
        let heads: HashSet<Dot> = g.current_heads().copied().collect();
        let expected: HashSet<Dot> = [mine_a.id, theirs.id].into_iter().collect();
        assert_eq!(heads, expected);
        let merge = g.add(30).unwrap();
        let merge_parents: HashSet<Dot> = merge.parents.iter().copied().collect();
        assert_eq!(merge_parents, expected);
        let heads_after: Vec<&Dot> = g.current_heads().collect();
        assert_eq!(heads_after, vec![&merge.id]);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use crate::test_utils::causal_permute;
    use proptest::prelude::*;
    use std::collections::HashMap as StdHashMap;

    /// Build a causally-valid op sequence. Each new op picks a deterministic
    /// subset of all previously emitted ops as its parents, using
    /// `parent_byte`'s bits as an inclusion mask. This produces both linear
    /// chains and branches, exercising the full op-DAG topology.
    pub(super) fn arb_op_sequence(
        max_ops: usize,
        num_actors: u64,
    ) -> impl Strategy<Value = Vec<Op<u32>>> {
        proptest::collection::vec((0u64..num_actors, any::<u8>(), any::<u32>()), 0..=max_ops)
            .prop_map(build_ops)
    }

    fn build_ops(raw: Vec<(u64, u8, u32)>) -> Vec<Op<u32>> {
        let mut clocks: StdHashMap<u64, u64> = StdHashMap::new();
        let mut emitted: Vec<Dot> = Vec::new();
        let mut ops: Vec<Op<u32>> = Vec::new();

        for (actor, parent_byte, payload) in raw {
            let clock = clocks.entry(actor).or_insert(0);
            let id = Dot::new(actor, *clock);
            *clock += 1;

            // Each previously emitted op is independently included as a parent
            // based on a bit of `parent_byte`. Bits cycle modulo 8 for emitted
            // sequences longer than 8 — proptest sequences are bounded
            // (max ~30) so wrap is acceptable. If the resulting set is empty
            // and there is at least one emitted op, fall back to the most
            // recent emitted op so the new op chains in.
            let parents: Vec<Dot> = if emitted.is_empty() {
                vec![]
            } else {
                let mut ps: Vec<Dot> = emitted
                    .iter()
                    .enumerate()
                    .filter(|&(i, _)| (parent_byte >> (i % 8)) & 1 == 1)
                    .map(|(_, d)| *d)
                    .collect();
                if ps.is_empty() {
                    ps.push(*emitted.last().unwrap());
                }
                ps
            };

            emitted.push(id);
            ops.push(Op {
                id,
                parents,
                payload,
            });
        }
        ops
    }

    fn apply_all(ops: &[Op<u32>]) -> OpGraph<u32> {
        let mut g: OpGraph<u32> = OpGraph::with_actor(0);
        for op in ops {
            g.receive(op.clone()).unwrap();
        }
        g
    }

    #[test]
    fn build_ops_smoke_linear() {
        let ops = build_ops(vec![(1, 0, 1), (1, 0, 2), (2, 0, 3)]);
        assert_eq!(ops.len(), 3);
        let g = apply_all(&ops);
        assert_eq!(g.len(), 3);
        assert_eq!(g.current_heads().count(), 1, "linear chain has one head");
    }

    #[test]
    fn build_ops_smoke_branching() {
        let ops = build_ops(vec![(1, 0, 1), (1, 0, 2), (2, 0b01, 3)]);
        assert_eq!(ops.len(), 3);
        assert_eq!(ops[2].parents, vec![Dot::new(1, 0)]);
        let g = apply_all(&ops);
        let heads: HashSet<Dot> = g.current_heads().copied().collect();
        let expected: HashSet<Dot> = [Dot::new(1, 1), Dot::new(2, 0)].into_iter().collect();
        assert_eq!(heads, expected, "branching produces multiple heads");
    }

    proptest! {
        #[test]
        fn convergence_under_causal_permutation(
            ops in arb_op_sequence(30, 3),
            seed1 in any::<u64>(),
            seed2 in any::<u64>(),
        ) {
            let perm1 = causal_permute(&ops, seed1);
            let perm2 = causal_permute(&ops, seed2);
            let s1 = apply_all(&perm1);
            let s2 = apply_all(&perm2);
            prop_assert!(s1.graph_state_eq(&s2), "convergence: {:?} vs {:?}", s1, s2);
        }
    }

    proptest! {
        /// Applying each op twice (in some causally-valid permutation of the
        /// duplicated multiset) yields the same final graph state as
        /// applying each op once.
        #[test]
        fn idempotency_under_permutation(
            ops in arb_op_sequence(20, 2),
            seed in any::<u64>(),
        ) {
            let single = apply_all(&ops);
            let doubled: Vec<Op<u32>> = ops.iter().flat_map(|op| [op.clone(), op.clone()]).collect();
            let perm = causal_permute(&doubled, seed);
            let twice = apply_all(&perm);
            prop_assert!(single.graph_state_eq(&twice), "idempotency: {:?} vs {:?}", single, twice);
        }
    }

    proptest! {
        /// Heads correctness (bidirectional): the `heads` set must equal
        /// exactly the set of ops *not referenced* as a parent by any other
        /// op. Both directions: every actual head is unreferenced, and every
        /// unreferenced op is a head — catches both spurious heads and
        /// orphaned heads.
        #[test]
        fn heads_correctness(
            ops in arb_op_sequence(30, 3),
            seed in any::<u64>(),
        ) {
            let g = apply_all(&causal_permute(&ops, seed));
            let referenced: HashSet<Dot> = g
                .ops
                .values()
                .flat_map(|op| op.parents.iter().copied())
                .collect();
            let expected_heads: HashSet<Dot> = g
                .ops
                .keys()
                .copied()
                .filter(|id| !referenced.contains(id))
                .collect();
            let actual_heads: HashSet<Dot> = g.heads.iter().copied().collect();
            prop_assert_eq!(
                actual_heads,
                expected_heads,
                "heads must equal the set of unreferenced ops"
            );
        }
    }

    proptest! {
        /// Acyclic invariant (regression): from any op, traversing parents
        /// must not re-enter a node already on the current DFS path. Uses
        /// white/gray/black DFS coloring — `on_path` is the set of currently
        /// gray (in-progress) nodes, `seen` is the union of gray and black
        /// (in-progress + finished). A parent edge into a node already in
        /// `on_path` means a back-edge, i.e., cycle.
        #[test]
        fn acyclic_invariant(
            ops in arb_op_sequence(30, 3),
            seed in any::<u64>(),
        ) {
            let g = apply_all(&causal_permute(&ops, seed));
            let mut seen: HashSet<Dot> = HashSet::new();
            for (start, _) in &g.ops {
                let mut on_path: HashSet<Dot> = HashSet::new();
                prop_assert!(
                    dfs_acyclic(&g, *start, &mut on_path, &mut seen),
                    "cycle detected starting from {:?}", start
                );
            }
        }
    }

    /// Recursive DFS that returns `false` if it re-enters a node already on
    /// the current path (back-edge = cycle). `seen` short-circuits subgraphs
    /// already proven acyclic from prior starts.
    fn dfs_acyclic(
        g: &OpGraph<u32>,
        node: Dot,
        on_path: &mut HashSet<Dot>,
        seen: &mut HashSet<Dot>,
    ) -> bool {
        if on_path.contains(&node) {
            return false;
        }
        if seen.contains(&node) {
            return true;
        }
        on_path.insert(node);
        seen.insert(node);
        if let Some(op) = g.ops.get(&node) {
            for p in &op.parents {
                if !dfs_acyclic(g, *p, on_path, seen) {
                    return false;
                }
            }
        }
        on_path.remove(&node);
        true
    }
}
