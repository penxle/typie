use hashbrown::HashSet;
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

/// Op-DAG storage. Immutable, structural-sharing append-only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpGraph<P> {
    actor: u64,
    next_clock: u64,
    ops: imbl::HashMap<Dot, Op<P>>,
    heads: imbl::HashSet<Dot>,
    /// Reverse parent index: each dot maps to the set of ops that reference
    /// it as a parent. Drives O(1) `has_child` checks during frontier
    /// maintenance and powers the cascade walks that keep `self_contained`
    /// accurate on data loss / restore.
    children: imbl::HashMap<Dot, imbl::HashSet<Dot>>,
    /// Subset of `ops` whose transitive ancestry is fully present locally.
    /// Maintained incrementally so `missing_for` can emit only replayable
    /// batches in O(walk) rather than scanning the full op set per round.
    /// Diverges from `ops` only after server-side data loss
    /// (`debug_remove`); `add` and `receive` keep parents-before-children
    /// invariants so they always preserve the set.
    self_contained: imbl::HashSet<Dot>,
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
            ops: imbl::HashMap::new(),
            heads: imbl::HashSet::new(),
            children: imbl::HashMap::new(),
            self_contained: imbl::HashSet::new(),
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

    /// Order is unstable across inserts; callers needing
    /// causality-respecting order should pass the result through
    /// [`OpGraph::topo_sort`].
    pub fn iter_all(&self) -> impl Iterator<Item = &Op<P>> + '_ {
        self.ops.values()
    }

    /// `true` when some op in `self.ops` has an ancestor missing locally —
    /// i.e. an ancestor was lost (storage failover, partial WAL replay, etc.)
    /// and its descendants are stranded. While this holds, [`missing_for`]
    /// silently drops the dangling descendants from emitted batches and the
    /// replica is "sync-degraded": new peers can fully sync but only see
    /// the self-contained prefix until a peer that still holds the missing
    /// ancestor pushes it back via [`CrdtError::UnknownHeads`] negative-ack.
    /// Production callers should surface this to operational alerting.
    ///
    /// [`missing_for`]: OpGraph::missing_for
    pub fn has_dangling(&self) -> bool {
        self.ops.len() != self.self_contained.len()
    }

    /// Test-only — drop a single op to model server-side data loss
    /// (replica failover with stale snapshot, point-in-time recovery, etc.).
    /// Leaves descendants intact with dangling parent references; the
    /// negative-ack recovery path is what restores consistency.
    #[cfg(test)]
    pub(crate) fn debug_remove(&self, dot: &Dot) -> Self
    where
        P: Clone,
    {
        let mut next = self.clone();
        if let Some(op) = next.ops.remove(dot) {
            for parent in &op.parents {
                if let Some(set) = next.children.get_mut(parent) {
                    set.remove(dot);
                    if set.is_empty() {
                        next.children.remove(parent);
                        // Parent has no children left in `next.ops`, so by
                        // the heads invariant it becomes a head — otherwise
                        // it stays unreachable from any frontier walk.
                        if next.ops.contains_key(parent) {
                            next.heads.insert(*parent);
                        }
                    }
                }
            }
        }
        next.heads.remove(dot);

        // Cascade through `children` to revoke `self_contained` for every
        // descendant — losing an ancestor breaks the transitive-ancestry
        // invariant for everything below it.
        let mut queue: Vec<Dot> = vec![*dot];
        while let Some(d) = queue.pop() {
            if next.self_contained.remove(&d).is_some()
                && let Some(children) = next.children.get(&d)
            {
                queue.extend(children.iter().copied());
            }
        }
        next
    }
}

impl<P> Default for OpGraph<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Clone> OpGraph<P> {
    pub fn add(&self, payload: P) -> Result<(Self, Op<P>), CrdtError> {
        let id = Dot::new(self.actor, self.next_clock);
        let next_clock = self
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

        let mut next = self.clone();
        next.next_clock = next_clock;
        next.ops.insert(id, op.clone());
        for p in &parents {
            next.heads.remove(p);
            next.children.entry(*p).or_default().insert(id);
        }
        next.heads.insert(id);
        // After server-side data loss, `debug_remove` re-promotes a non-SC
        // ancestor into `heads` so the frontier walk can still see it. A new
        // op built on such a head inherits its broken ancestry, so derive SC
        // status from the parents instead of assuming all heads qualify.
        // The op stays in `self.ops` either way; once the missing ancestor is
        // restored, `receive`'s `try_promote_self_contained` cascade lifts it
        // through `self.children` automatically.
        if parents.iter().all(|p| next.self_contained.contains(p)) {
            next.self_contained.insert(id);
        }

        Ok((next, op))
    }

    /// Compute the ops a peer is missing, given the peer's frontier
    /// `remote_heads`. The returned vector is `self.ops` minus the ancestry
    /// of `remote_heads`, emitted in ancestry-first (topological) order so
    /// that a peer applying the result via [`OpGraph::receive`] never hits
    /// a [`CrdtError::MissingParents`] rejection.
    ///
    /// Returns `Err(CrdtError::UnknownHeads)` if any dot encountered while
    /// walking `remote_heads` and their ancestry is not in `self.ops` —
    /// either a head itself or a missing parent reached through the walk.
    /// The unknown set drives the negative-ack path: the peer holds the op
    /// locally and must re-send it (with its own ancestry) so the local
    /// replica can recover from server-side data loss.
    ///
    /// When [`has_dangling`] is `true`, descendants of locally-lost ancestors
    /// are silently dropped from the emitted batch so the receiver never
    /// rejects with `MissingParents`. The `Ok` branch therefore signals only
    /// that the batch is replayable — not that it carries every reachable
    /// op. Callers should consult [`has_dangling`] separately to surface
    /// degraded sync for operational alerting.
    ///
    /// [`has_dangling`]: OpGraph::has_dangling
    pub fn missing_for(&self, remote_heads: &HashSet<Dot>) -> Result<Vec<Op<P>>, CrdtError> {
        let mut known: HashSet<Dot> = HashSet::new();
        let mut unknown: HashSet<Dot> = HashSet::new();
        let mut walk: Vec<Dot> = remote_heads.iter().copied().collect();
        while let Some(dot) = walk.pop() {
            if !self.ops.contains_key(&dot) {
                unknown.insert(dot);
                continue;
            }
            if known.insert(dot)
                && let Some(op) = self.ops.get(&dot)
            {
                walk.extend(op.parents.iter().copied());
            }
        }
        if !unknown.is_empty() {
            return Err(CrdtError::UnknownHeads {
                unknown: unknown.into_iter().collect(),
            });
        }

        // Restrict the emitted batch to ops whose full ancestry still lives
        // here. After a non-head ancestor is lost server-side its descendants
        // remain in `self.ops` but are no longer in `self_contained` — sending
        // them would force the peer to reject the batch with `MissingParents`.
        // We silently drop those descendants and let the negative-ack path
        // (`ResendRequest`) restore the lost ancestor from a peer that still
        // holds it.
        // Materialize a hashbrown snapshot to bridge to `dfs_post_order_filtered`'s
        // `&HashSet<Dot>` argument — `self.self_contained` is `imbl::HashSet`.
        let self_contained: HashSet<Dot> = self.self_contained.iter().copied().collect();
        Ok(self.dfs_post_order_filtered(
            self.heads.iter().copied().collect(),
            &known,
            Some(&self_contained),
        ))
    }

    /// Topologically sort the given dots in ancestry-first order. Parents
    /// outside `dots` are skipped — the caller controls the boundary, so the
    /// output is a self-contained ancestry-first batch over the chosen
    /// subset. Dots not present in `self.ops` are silently skipped.
    pub fn topo_sort(&self, dots: &HashSet<Dot>) -> Vec<Op<P>> {
        self.dfs_post_order_filtered(dots.iter().copied().collect(), &HashSet::new(), Some(dots))
    }

    /// Iterative DFS post-order from `roots`, emitting each visited dot in
    /// ancestry-first (parents-before-children) order. Iterative — not
    /// recursive — so deep chains do not blow the stack.
    ///
    /// `skip`: dots whose ancestry must not be entered. Treated as an
    /// already-known boundary — neither the dot nor its parents are emitted.
    /// `include`: when `Some`, only dots in the set are emitted; their
    /// parents outside the set are skipped (used to restrict to a subset).
    fn dfs_post_order_filtered(
        &self,
        roots: Vec<Dot>,
        skip: &HashSet<Dot>,
        include: Option<&HashSet<Dot>>,
    ) -> Vec<Op<P>> {
        enum Frame {
            Enter(Dot),
            Emit(Dot),
        }
        let mut visited: HashSet<Dot> = HashSet::new();
        let mut stack: Vec<Frame> = roots.into_iter().map(Frame::Enter).collect();
        let mut result: Vec<Op<P>> = Vec::new();
        while let Some(frame) = stack.pop() {
            match frame {
                Frame::Enter(dot) => {
                    if skip.contains(&dot) {
                        continue;
                    }
                    if !visited.insert(dot) {
                        continue;
                    }
                    if let Some(op) = self.ops.get(&dot) {
                        if include.is_none_or(|set| set.contains(&dot)) {
                            stack.push(Frame::Emit(dot));
                        }
                        for &parent in &op.parents {
                            stack.push(Frame::Enter(parent));
                        }
                    }
                }
                Frame::Emit(dot) => {
                    if let Some(op) = self.ops.get(&dot) {
                        result.push(op.clone());
                    }
                }
            }
        }
        result
    }
}

impl<P: Clone + Eq> OpGraph<P> {
    pub fn receive(&self, mut op: Op<P>) -> Result<Self, CrdtError> {
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
                return Ok(self.clone());
            }
            return Err(CrdtError::DotConflict { dot: op.id });
        }

        let mut next = self.clone();
        next.sync_clock_for(&op.id)?;

        let id = op.id;
        for p in &op.parents {
            next.heads.remove(p);
            next.children.entry(*p).or_default().insert(id);
        }

        // Skip the head insert when an existing op already lists this dot
        // as a parent. The recovery path can deliver an ancestor after its
        // descendants are already present (server lost a non-head op and
        // the client resends it); blindly inserting would resurrect the
        // ancestor as a head and leave `current_heads` reporting an op
        // that has children. The `children` index makes this an O(1)
        // check so the hot path stays linear during storage replay and
        // initial sync.
        if next.children.get(&id).is_none_or(imbl::HashSet::is_empty) {
            next.heads.insert(id);
        }
        next.ops.insert(id, op);

        // Promote into `self_contained` if every parent is already there,
        // then cascade to the new op's children — a previously dangling
        // descendant whose only missing ancestor was just restored becomes
        // self-contained too.
        Ok(next.try_promote_self_contained(id))
    }

    fn try_promote_self_contained(mut self, root: Dot) -> Self {
        let mut queue: Vec<Dot> = vec![root];
        while let Some(dot) = queue.pop() {
            if self.self_contained.contains(&dot) {
                continue;
            }
            let Some(op) = self.ops.get(&dot) else {
                continue;
            };
            if !op.parents.iter().all(|p| self.self_contained.contains(p)) {
                continue;
            }
            self.self_contained.insert(dot);
            if let Some(children) = self.children.get(&dot) {
                queue.extend(children.iter().copied());
            }
        }
        self
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
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, op) = g.add(42).unwrap();
        assert_eq!(op.id, Dot::new(1, 0));
        assert!(op.parents.is_empty(), "first op is genesis (no parents)");
        assert_eq!(op.payload, 42);
        assert_eq!(g.len(), 1);
        let heads: Vec<&Dot> = g.current_heads().collect();
        assert_eq!(heads, vec![&Dot::new(1, 0)]);
    }

    #[test]
    fn add_second_op_parents_first() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, _) = g.add(10).unwrap();
        let (g, op2) = g.add(20).unwrap();
        assert_eq!(op2.id, Dot::new(1, 1));
        assert_eq!(op2.parents, vec![Dot::new(1, 0)]);
        assert_eq!(g.len(), 2);
        let heads: Vec<&Dot> = g.current_heads().collect();
        assert_eq!(heads, vec![&Dot::new(1, 1)], "only the latest op is head");
    }

    #[test]
    fn add_advances_next_clock() {
        let g: OpGraph<u32> = OpGraph::with_actor(5);
        let (g, op1) = g.add(1).unwrap();
        let (g, op2) = g.add(2).unwrap();
        let (_g, op3) = g.add(3).unwrap();
        assert_eq!(op1.id, Dot::new(5, 0));
        assert_eq!(op2.id, Dot::new(5, 1));
        assert_eq!(op3.id, Dot::new(5, 2));
    }

    #[test]
    fn receive_genesis_op() {
        let g: OpGraph<u32> = OpGraph::with_actor(0);
        let op = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 1,
        };
        let g = g.receive(op.clone()).unwrap();
        assert_eq!(g.len(), 1);
        assert!(g.contains(&Dot::new(99, 0)));
        let heads: Vec<&Dot> = g.current_heads().collect();
        assert_eq!(heads, vec![&Dot::new(99, 0)]);
    }

    #[test]
    fn receive_linear_chain() {
        let g: OpGraph<u32> = OpGraph::with_actor(0);
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
        let g = g.receive(a).unwrap();
        let g = g.receive(b).unwrap();
        let g = g.receive(c).unwrap();
        assert_eq!(g.len(), 3);
        let heads: Vec<&Dot> = g.current_heads().collect();
        assert_eq!(heads, vec![&Dot::new(99, 2)], "only the leaf is head");
    }

    #[test]
    fn receive_branching_two_heads() {
        let g: OpGraph<u32> = OpGraph::with_actor(0);
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
        let g = g.receive(root).unwrap();
        let g = g.receive(a).unwrap();
        let g = g.receive(b).unwrap();
        let heads: HashSet<Dot> = g.current_heads().copied().collect();
        let expected: HashSet<Dot> = [Dot::new(1, 0), Dot::new(2, 0)].into_iter().collect();
        assert_eq!(heads, expected);
    }

    #[test]
    fn receive_merging_back_to_one_head() {
        let g: OpGraph<u32> = OpGraph::with_actor(0);
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
        let g = g.receive(root).unwrap();
        let g = g.receive(a).unwrap();
        let g = g.receive(b).unwrap();
        let g = g.receive(m).unwrap();
        let heads: Vec<&Dot> = g.current_heads().collect();
        assert_eq!(
            heads,
            vec![&Dot::new(3, 0)],
            "merge collapses to single head"
        );
    }

    #[test]
    fn receive_self_reference_rejected() {
        let g: OpGraph<u32> = OpGraph::with_actor(0);
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
        let g: OpGraph<u32> = OpGraph::with_actor(0);
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
        let g: OpGraph<u32> = OpGraph::with_actor(0);
        let present = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 0,
        };
        let g = g.receive(present.clone()).unwrap();
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
        let g: OpGraph<u32> = OpGraph::with_actor(0);
        let p = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 0,
        };
        let g = g.receive(p.clone()).unwrap();
        let op_dup = Op {
            id: Dot::new(99, 1),
            parents: vec![p.id, p.id, p.id],
            payload: 1,
        };
        let g = g.receive(op_dup).unwrap();
        let stored = g.get(&Dot::new(99, 1)).unwrap();
        assert_eq!(stored.parents, vec![p.id]);
    }

    #[test]
    fn receive_normalizes_unsorted_multi_parents() {
        let g: OpGraph<u32> = OpGraph::with_actor(0);
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
        let g = g.receive(a.clone()).unwrap();
        let g = g.receive(b.clone()).unwrap();

        let op_unsorted = Op {
            id: Dot::new(99, 1),
            parents: vec![b.id, a.id],
            payload: 1,
        };
        let g = g.receive(op_unsorted).unwrap();
        let stored = g.get(&Dot::new(99, 1)).unwrap();
        assert_eq!(stored.parents, vec![a.id, b.id]);
    }

    #[test]
    fn receive_same_op_twice_is_idempotent() {
        let g: OpGraph<u32> = OpGraph::with_actor(0);
        let op = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 7,
        };
        let g = g.receive(op.clone()).unwrap();
        let g = g.receive(op.clone()).unwrap();
        assert_eq!(g.len(), 1);
        assert_eq!(g.get(&Dot::new(99, 0)), Some(&op));
    }

    #[test]
    fn receive_same_dot_different_payload_rejected() {
        let g: OpGraph<u32> = OpGraph::with_actor(0);
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
        let g = g.receive(op_a.clone()).unwrap();
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
        let g: OpGraph<u32> = OpGraph::with_actor(0);
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
        let g = g.receive(root_a.clone()).unwrap();
        let g = g.receive(root_b.clone()).unwrap();
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
        let g = g.receive(op_x).unwrap();
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
        let g: OpGraph<u32> = OpGraph::with_actor(0);
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
        let g = g.receive(root.clone()).unwrap();
        let g = g.receive(child.clone()).unwrap();
        let g = g.receive(root).unwrap();
        let heads: Vec<&Dot> = g.current_heads().collect();
        assert_eq!(heads, vec![&Dot::new(99, 1)]);
    }

    #[test]
    fn receive_advances_next_clock_past_observed_op() {
        let g: OpGraph<u32> = OpGraph::with_actor(7);
        let observed = Op {
            id: Dot::new(99, 50),
            parents: vec![],
            payload: 1,
        };
        let g = g.receive(observed).unwrap();
        let (_g, next) = g.add(2).unwrap();
        assert_eq!(next.id, Dot::new(7, 51));
    }

    #[test]
    fn receive_lower_clock_does_not_lower_next_clock() {
        let g: OpGraph<u32> = OpGraph::with_actor(7);
        let high = Op {
            id: Dot::new(99, 10),
            parents: vec![],
            payload: 1,
        };
        let g = g.receive(high).unwrap();
        let low = Op {
            id: Dot::new(98, 3),
            parents: vec![],
            payload: 2,
        };
        let g = g.receive(low).unwrap();
        let (_g, next) = g.add(3).unwrap();
        assert_eq!(next.id, Dot::new(7, 11));
    }

    #[test]
    fn receive_idempotent_op_keeps_next_clock_advanced() {
        let g: OpGraph<u32> = OpGraph::with_actor(7);
        let op = Op {
            id: Dot::new(99, 50),
            parents: vec![],
            payload: 1,
        };
        let g = g.receive(op.clone()).unwrap();
        let g = g.receive(op).unwrap();
        let (_g, next) = g.add(2).unwrap();
        assert_eq!(next.id, Dot::new(7, 51));
    }

    #[test]
    fn receive_max_clock_returns_clock_overflow() {
        let g: OpGraph<u32> = OpGraph::with_actor(7);
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
        let g1: OpGraph<u32> = OpGraph::with_actor(0);
        let g2: OpGraph<u32> = OpGraph::with_actor(42);
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
        let g1 = g1.receive(a.clone()).unwrap();
        let g1 = g1.receive(b.clone()).unwrap();
        let g2 = g2.receive(b).unwrap();
        let g2 = g2.receive(a).unwrap();
        assert_ne!(g1, g2);
    }

    #[test]
    fn graph_state_eq_ignores_actor_and_clock() {
        let g1: OpGraph<u32> = OpGraph::with_actor(0);
        let g2: OpGraph<u32> = OpGraph::with_actor(42);
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
        let g1 = g1.receive(a.clone()).unwrap();
        let g1 = g1.receive(b.clone()).unwrap();
        let g2 = g2.receive(b).unwrap();
        let g2 = g2.receive(a).unwrap();
        assert!(g1.graph_state_eq(&g2));
    }

    #[test]
    fn mixed_local_and_remote_then_merge() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, mine_a) = g.add(10).unwrap();
        let theirs = Op {
            id: Dot::new(2, 0),
            parents: vec![],
            payload: 20,
        };
        let g = g.receive(theirs.clone()).unwrap();
        let heads: HashSet<Dot> = g.current_heads().copied().collect();
        let expected: HashSet<Dot> = [mine_a.id, theirs.id].into_iter().collect();
        assert_eq!(heads, expected);
        let (g, merge) = g.add(30).unwrap();
        let merge_parents: HashSet<Dot> = merge.parents.iter().copied().collect();
        assert_eq!(merge_parents, expected);
        let heads_after: Vec<&Dot> = g.current_heads().collect();
        assert_eq!(heads_after, vec![&merge.id]);
    }

    #[test]
    fn missing_for_empty_remote_returns_self_ops_topo_sorted() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let (g, b) = g.add(2).unwrap();
        let (g, c) = g.add(3).unwrap();
        let result = g.missing_for(&HashSet::new()).unwrap();
        let ids: Vec<Dot> = result.iter().map(|op| op.id).collect();
        assert_eq!(ids, vec![a.id, b.id, c.id]);
    }

    #[test]
    fn missing_for_full_remote_returns_empty() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, _a) = g.add(1).unwrap();
        let (g, _b) = g.add(2).unwrap();
        let (g, c) = g.add(3).unwrap();
        let remote_heads: HashSet<Dot> = [c.id].into_iter().collect();
        let result = g.missing_for(&remote_heads).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn missing_for_partial_remote_returns_only_descendants() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let (g, b) = g.add(2).unwrap();
        let (g, c) = g.add(3).unwrap();
        let remote_heads: HashSet<Dot> = [a.id].into_iter().collect();
        let result = g.missing_for(&remote_heads).unwrap();
        let ids: Vec<Dot> = result.iter().map(|op| op.id).collect();
        assert_eq!(ids, vec![b.id, c.id]);
    }

    #[test]
    fn missing_for_unknown_remote_head_returns_err() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, _a) = g.add(1).unwrap();
        let (g, _b) = g.add(2).unwrap();
        let unknown = Dot::new(99, 0);
        let remote_heads: HashSet<Dot> = [unknown].into_iter().collect();
        let err = g.missing_for(&remote_heads).unwrap_err();
        assert_eq!(
            err,
            CrdtError::UnknownHeads {
                unknown: vec![unknown]
            }
        );
    }

    #[test]
    fn receive_does_not_resurrect_ancestor_as_head() {
        // a → b → c chain. Drop a to model server-side data loss, then
        // re-receive it via the recovery path. The descendants are still
        // present, so a must not become a head — heads must stay {c}.
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let (g, _b) = g.add(2).unwrap();
        let (g, c) = g.add(3).unwrap();
        let g = g.debug_remove(&a.id);
        let g = g.receive(a.clone()).unwrap();
        let heads: HashSet<Dot> = g.current_heads().copied().collect();
        let expected: HashSet<Dot> = [c.id].into_iter().collect();
        assert_eq!(heads, expected);
    }

    #[test]
    fn missing_for_walk_reports_dropped_ancestor_as_unknown() {
        // a → b → c. Server simulates losing a (server-side rollback). Then
        // missing_for({c}) walks c → b → a, finds a missing in self.ops,
        // and reports a as unknown so the peer can re-send it.
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let (g, _b) = g.add(2).unwrap();
        let (g, c) = g.add(3).unwrap();
        let g = g.debug_remove(&a.id);
        let remote_heads: HashSet<Dot> = [c.id].into_iter().collect();
        let err = g.missing_for(&remote_heads).unwrap_err();
        match err {
            CrdtError::UnknownHeads { unknown } => {
                assert_eq!(unknown, vec![a.id]);
            }
            _ => panic!("expected UnknownHeads"),
        }
    }

    #[test]
    fn missing_for_empty_remote_skips_descendants_of_lost_ancestor() {
        // a → b → c. Server loses non-head op b (replica failover, stale
        // snapshot, etc.). A fresh peer with no frontier asks for everything.
        // b and c reference a missing parent, so emitting them would force
        // the peer to reject the batch with `MissingParents`. We instead
        // emit only the self-contained subset {a}; the negative-ack path
        // restores b later from a peer that still holds it.
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let (g, b) = g.add(2).unwrap();
        let (g, _c) = g.add(3).unwrap();
        let g = g.debug_remove(&b.id);
        let result = g.missing_for(&HashSet::new()).unwrap();
        let ids: Vec<Dot> = result.iter().map(|op| op.id).collect();
        assert_eq!(ids, vec![a.id]);
    }

    #[test]
    fn add_after_lost_ancestor_does_not_mark_new_op_self_contained() {
        // a → b → c. Lose b, then locally add d on top of the now-broken
        // frontier (heads become {a, c} after `debug_remove` re-promotes a;
        // d's parents = [a, c]). c is no longer self-contained, so d
        // inherits its broken ancestry and must not be emitted by
        // `missing_for(empty)` — otherwise a fresh peer would receive d
        // without c/b and reject with `MissingParents`.
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let (g, b) = g.add(2).unwrap();
        let (g, _c) = g.add(3).unwrap();
        let g = g.debug_remove(&b.id);
        let (g, _d) = g.add(4).unwrap();
        let result = g.missing_for(&HashSet::new()).unwrap();
        let ids: Vec<Dot> = result.iter().map(|op| op.id).collect();
        assert_eq!(ids, vec![a.id]);
    }

    #[test]
    fn add_after_recovery_promotes_descendant_chain_to_self_contained() {
        // Same loss as above, but b is restored before sync. The cascade
        // through `self.children` must promote c and d (built post-loss)
        // back into `self_contained`, so `missing_for(empty)` emits the
        // full chain in ancestry-first order.
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let (g, b) = g.add(2).unwrap();
        let (g, c) = g.add(3).unwrap();
        let g = g.debug_remove(&b.id);
        let (g, d) = g.add(4).unwrap();
        let g = g.receive(b.clone()).unwrap();
        let result = g.missing_for(&HashSet::new()).unwrap();
        let ids: Vec<Dot> = result.iter().map(|op| op.id).collect();
        assert_eq!(ids.len(), 4);
        let pos = |dot: Dot| ids.iter().position(|&x| x == dot).unwrap();
        assert!(pos(a.id) < pos(b.id));
        assert!(pos(b.id) < pos(c.id));
        assert!(pos(c.id) < pos(d.id));
    }

    #[test]
    fn has_dangling_observable_after_non_head_loss() {
        // After losing a non-head ancestor, `missing_for` filters its
        // descendants out and returns `Ok` with no caller-visible signal.
        // `has_dangling` is the operational signal: it stays `true` until a
        // peer pushes the missing ancestor back, at which point the cascade
        // promotes the descendants and the replica is no longer degraded.
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, _a) = g.add(1).unwrap();
        let (g, b) = g.add(2).unwrap();
        let (g, _c) = g.add(3).unwrap();
        assert!(!g.has_dangling());
        let g = g.debug_remove(&b.id);
        assert!(g.has_dangling());
        let g = g.receive(b.clone()).unwrap();
        assert!(!g.has_dangling());
    }

    #[test]
    fn missing_for_older_frontier_skips_descendants_of_lost_ancestor() {
        // Same loss scenario as above, but the peer already has `a`. The
        // emitted batch must remain self-contained — empty here, since the
        // only remaining self-contained op is what the peer already holds.
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let (g, b) = g.add(2).unwrap();
        let (g, _c) = g.add(3).unwrap();
        let g = g.debug_remove(&b.id);
        let remote_heads: HashSet<Dot> = [a.id].into_iter().collect();
        let result = g.missing_for(&remote_heads).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn missing_for_branch_topology_topo_sorted() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let b = Op {
            id: Dot::new(2, 0),
            parents: vec![a.id],
            payload: 2,
        };
        let c = Op {
            id: Dot::new(3, 0),
            parents: vec![a.id],
            payload: 3,
        };
        let g = g.receive(b.clone()).unwrap();
        let g = g.receive(c.clone()).unwrap();
        let (g, d) = g.add(4).unwrap();

        let remote_heads: HashSet<Dot> = [a.id].into_iter().collect();
        let result = g.missing_for(&remote_heads).unwrap();
        let ids: Vec<Dot> = result.iter().map(|op| op.id).collect();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&b.id));
        assert!(ids.contains(&c.id));
        assert!(ids.contains(&d.id));
        let pos_b = ids.iter().position(|&x| x == b.id).unwrap();
        let pos_c = ids.iter().position(|&x| x == c.id).unwrap();
        let pos_d = ids.iter().position(|&x| x == d.id).unwrap();
        assert!(pos_b < pos_d);
        assert!(pos_c < pos_d);
    }

    #[test]
    fn topo_sort_linear_chain() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let (g, b) = g.add(2).unwrap();
        let (g, c) = g.add(3).unwrap();
        let dots: HashSet<Dot> = [a.id, b.id, c.id].into_iter().collect();
        let result = g.topo_sort(&dots);
        let ids: Vec<Dot> = result.iter().map(|op| op.id).collect();
        assert_eq!(ids, vec![a.id, b.id, c.id]);
    }

    #[test]
    fn topo_sort_subset_skips_parents_outside_dots() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, _a) = g.add(1).unwrap();
        let (g, b) = g.add(2).unwrap();
        let (g, c) = g.add(3).unwrap();
        let dots: HashSet<Dot> = [b.id, c.id].into_iter().collect();
        let result = g.topo_sort(&dots);
        let ids: Vec<Dot> = result.iter().map(|op| op.id).collect();
        assert_eq!(ids, vec![b.id, c.id]);
    }

    #[test]
    fn topo_sort_empty_dots() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let result = g.topo_sort(&HashSet::new());
        assert!(result.is_empty());
    }

    #[test]
    fn topo_sort_skips_unknown_dots() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let unknown = Dot::new(99, 0);
        let dots: HashSet<Dot> = [a.id, unknown].into_iter().collect();
        let result = g.topo_sort(&dots);
        let ids: Vec<Dot> = result.iter().map(|op| op.id).collect();
        assert_eq!(ids, vec![a.id]);
    }

    #[test]
    fn topo_sort_with_internal_gap_preserves_ancestry() {
        // a → b → c, dots = {a, c} (b excluded). Traversal must still walk
        // through b to reach a; otherwise c emits before a and breaks the
        // ancestry-first contract.
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let (g, _b) = g.add(2).unwrap();
        let (g, c) = g.add(3).unwrap();
        let dots: HashSet<Dot> = [a.id, c.id].into_iter().collect();
        let result = g.topo_sort(&dots);
        let ids: Vec<Dot> = result.iter().map(|op| op.id).collect();
        assert_eq!(ids, vec![a.id, c.id]);
    }

    #[test]
    fn iter_all_yields_every_op() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let (g, b) = g.add(2).unwrap();
        let (g, c) = g.add(3).unwrap();
        let collected: HashSet<Dot> = g.iter_all().map(|op| op.id).collect();
        let expected: HashSet<Dot> = [a.id, b.id, c.id].into_iter().collect();
        assert_eq!(collected, expected);
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
        ops.iter()
            .cloned()
            .try_fold(OpGraph::with_actor(0), |g, op| g.receive(op))
            .unwrap()
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
