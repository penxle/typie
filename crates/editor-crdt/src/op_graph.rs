use editor_macros::ffi;
use hashbrown::HashSet;
use serde::{Deserialize, Serialize};

use crate::{CrdtError, Dot};

/// One node in the op-DAG. `id` is the op's unique identifier (also reused as
/// the semantic identifier — RGA element id, OR-Set add token — by the
/// payload). `parents` are the op-DAG parents of this op (the heads of the
/// store at the moment this op was created). Stored normalized: sorted
/// ascending, no duplicates.
#[ffi]
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

    // `Arc`-wrapped so cloning the `OpGraph` (every transaction does, via
    // `State::clone`) shares each sealed changeset by pointer. A bare
    // `Vector<Changeset>` with few elements stores them inline and deep-copies
    // the contained ops `Vec` on every clone — `O(total ops)` per keystroke when
    // `from_plain` seals the whole document into one changeset. The `Arc` is an
    // internal storage detail; the wire/FFI form is still `Changeset`.
    changesets: imbl::Vector<std::sync::Arc<crate::Changeset<P>>>,

    /// Local-write accumulator. `add` pushes here; `commit` drains into
    /// `changesets`. Remote ingestion (`receive_changeset`) never touches
    /// this — it seals its own boundary directly into `changesets`, so
    /// `pending` only ever holds the in-flight transaction's ops.
    pending: Vec<Op<P>>,

    ops: imbl::HashMap<Dot, Op<P>>,
    heads: imbl::HashSet<Dot>,
    /// Reverse parent index: each dot maps to the set of ops that reference
    /// it as a parent. Drives O(1) `has_child` checks during frontier
    /// maintenance and powers the cascade walks that keep `self_contained`
    /// accurate on data loss / restore.
    children: imbl::HashMap<Dot, imbl::HashSet<Dot>>,
    /// Subset of `ops` whose transitive ancestry is fully present locally.
    /// Maintained incrementally so `missing_changesets_for` can emit only
    /// replayable batches in O(walk) rather than scanning the full op set
    /// per round. Diverges from `ops` only after server-side data loss
    /// (`debug_remove`); `add` and `receive_changeset` keep
    /// parents-before-children invariants so they always preserve the set.
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
            changesets: imbl::Vector::new(),
            pending: Vec::new(),
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

    pub fn changesets(&self) -> &imbl::Vector<std::sync::Arc<crate::Changeset<P>>> {
        &self.changesets
    }

    pub fn pending(&self) -> &[Op<P>] {
        &self.pending
    }

    /// Order is unstable across inserts; callers needing
    /// causality-respecting order should pass the result through
    /// [`OpGraph::topo_sort`].
    pub fn iter_all(&self) -> impl Iterator<Item = &Op<P>> + '_ {
        self.ops.values()
    }

    /// `true` when some op in `self.ops` has an ancestor missing locally —
    /// i.e. an ancestor was lost (storage failover, partial WAL replay, etc.)
    /// and its descendants are stranded. While this holds,
    /// [`missing_changesets_for`] silently drops the dangling changesets from
    /// emitted batches and the replica is "sync-degraded": new peers can fully
    /// sync but only see the self-contained prefix until a peer that still
    /// holds the missing ancestor pushes it back via
    /// [`CrdtError::UnknownHeads`] negative-ack. Production callers should
    /// surface this to operational alerting.
    ///
    /// [`missing_changesets_for`]: OpGraph::missing_changesets_for
    pub fn has_dangling(&self) -> bool {
        self.ops.len() != self.self_contained.len()
    }

    pub fn ancestry_of(&self, heads: &HashSet<Dot>) -> HashSet<Dot> {
        let mut seen: HashSet<Dot> = HashSet::new();
        let mut stack: Vec<Dot> = heads.iter().copied().collect();
        while let Some(dot) = stack.pop() {
            if !self.ops.contains_key(&dot) {
                continue;
            }
            if seen.insert(dot)
                && let Some(op) = self.ops.get(&dot)
            {
                stack.extend(op.parents.iter().copied());
            }
        }
        seen
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
    pub fn changesets_as_vec(&self) -> Vec<crate::Changeset<P>> {
        self.changesets
            .iter()
            .map(|cs| cs.as_ref().clone())
            .collect()
    }

    pub fn add(&self, payload: P) -> Result<(Self, Op<P>), CrdtError> {
        let mut next = self.clone();
        let op = next.add_mut(payload)?;
        Ok((next, op))
    }

    /// In-place variant of `add`. Skips the per-call `self.clone()` so callers
    /// that already own a mutable OpGraph (e.g. `BatchedState` for the duration
    /// of a batch) pay the persistent-collection clone cost once across many
    /// ops instead of per op.
    pub fn add_mut(&mut self, payload: P) -> Result<Op<P>, CrdtError> {
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

        self.next_clock = next_clock;
        self.ops.insert(id, op.clone());
        for p in &parents {
            self.heads.remove(p);
            self.children.entry(*p).or_default().insert(id);
        }
        self.heads.insert(id);
        // After server-side data loss, `debug_remove` re-promotes a non-SC
        // ancestor into `heads` so the frontier walk can still see it. A new
        // op built on such a head inherits its broken ancestry, so derive SC
        // status from the parents instead of assuming all heads qualify.
        // The op stays in `self.ops` either way; once the missing ancestor is
        // restored, `receive_changeset`'s `try_promote_self_contained` cascade
        // lifts it through `self.children` automatically.
        if parents.iter().all(|p| self.self_contained.contains(p)) {
            self.self_contained.insert(id);
        }
        self.pending.push(op.clone());

        Ok(op)
    }

    /// No-op when `pending` is empty so a transact that emits zero ops does
    /// not append a stray empty entry. Ops are sealed in push order, which
    /// matches `add`'s ancestry-first construction (each new op parents on
    /// current heads), so the resulting `Changeset.ops` satisfies the
    /// parents-before-children topological-order contract on `Changeset`.
    pub fn commit(&self) -> Self {
        let mut next = self.clone();
        next.commit_mut();
        next
    }

    /// In-place variant of `commit`. Skips the per-call `self.clone()` that
    /// would otherwise copy the entire `changesets` Vec on every transaction
    /// boundary — substantial under heavy typing where each commit pays
    /// O(history_size).
    pub fn commit_mut(&mut self) {
        if !self.pending.is_empty() {
            let ops = std::mem::take(&mut self.pending);
            self.changesets
                .push_back(std::sync::Arc::new(crate::Changeset { ops }));
        }
    }

    /// Atomicity invariant: each changeset in `self.changesets` is either
    /// fully in remote's ancestry or fully outside it after the
    /// self-contained filter — mixed → `PartialDuplicate`. The
    /// self-contained filter runs first so that legitimate degraded-
    /// storage cases (a cs holding a dangling-ancestor descendant) are
    /// silently dropped instead of being misclassified as corruption.
    pub fn missing_changesets_for(
        &self,
        remote_heads: &HashSet<Dot>,
    ) -> Result<Vec<crate::Changeset<P>>, CrdtError> {
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

        let mut out: Vec<crate::Changeset<P>> = Vec::new();
        for cs in &self.changesets {
            // Self-contained filter must precede the membership classification
            // below: a cs holding a dangling-ancestor descendant is a degraded-
            // storage case to drop silently, not an atomicity violation.
            if cs
                .ops
                .iter()
                .any(|op| !self.self_contained.contains(&op.id))
            {
                continue;
            }
            let mut all_known = true;
            let mut any_known = false;
            for op in &cs.ops {
                if known.contains(&op.id) {
                    any_known = true;
                } else {
                    all_known = false;
                }
            }
            if all_known {
                continue;
            }
            if any_known {
                let dots: Vec<Dot> = cs
                    .ops
                    .iter()
                    .filter(|op| known.contains(&op.id))
                    .map(|op| op.id)
                    .collect();
                return Err(CrdtError::PartialDuplicate { dots });
            }
            out.push(cs.as_ref().clone());
        }
        Ok(out)
    }

    /// Like [`missing_changesets_for`], but additionally filters out
    /// changesets whose ops were authored by a different actor — i.e.,
    /// changesets we ingested via [`receive_changeset`] rather than authored
    /// locally via [`add`] + [`commit`]. This is the right primitive for
    /// server-mediated sync: a peer must push only what it authored, since the
    /// server already holds anything it broadcast to us. Without this filter
    /// each broadcast op would be echoed back by every receiving peer.
    ///
    /// `commit` seals only the local-actor `pending` ops, and
    /// `receive_changeset` writes incoming changesets straight to
    /// `self.changesets` without touching `pending`, so a sealed changeset's
    /// ops are uniformly local- or remote-origin — the per-changeset
    /// `all(op.id.actor == self.actor)` check is a complete classifier.
    ///
    /// [`missing_changesets_for`]: OpGraph::missing_changesets_for
    /// [`receive_changeset`]: OpGraph::receive_changeset
    /// [`add`]: OpGraph::add
    /// [`commit`]: OpGraph::commit
    pub fn local_changesets_since(
        &self,
        remote_heads: &HashSet<Dot>,
    ) -> Result<Vec<crate::Changeset<P>>, CrdtError> {
        let mut out = self.missing_changesets_for(remote_heads)?;
        out.retain(|cs| cs.ops.iter().all(|op| op.id.actor == self.actor));
        Ok(out)
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

    /// Phase A validates fully against `&self` before any mutation; Phase B
    /// applies a clone-and-replace that is unreachable on the failure
    /// paths above. The whole call is therefore all-or-nothing.
    pub fn receive_changeset(&self, cs: crate::Changeset<P>) -> Result<Self, CrdtError> {
        if cs.ops.is_empty() {
            return Err(CrdtError::EmptyChangeset);
        }

        // Normalize parents up-front so the stored form and the
        // dot-conflict comparison below both use the canonical shape.
        let mut cs = cs;
        for op in &mut cs.ops {
            op.parents.sort();
            op.parents.dedup();
        }

        let mut local_known: hashbrown::HashSet<Dot> = hashbrown::HashSet::new();
        let mut already_dots: Vec<Dot> = Vec::new();

        for op in &cs.ops {
            // Intra-cs duplicate dot would silently overwrite in the Phase B
            // `ops.insert` and break atomicity, so reject before any mutation.
            if !local_known.insert(op.id) {
                return Err(CrdtError::DotConflict { dot: op.id });
            }

            if op.parents.contains(&op.id) {
                return Err(CrdtError::SelfReference { dot: op.id });
            }

            let missing: Vec<Dot> = op
                .parents
                .iter()
                .copied()
                .filter(|p| !self.ops.contains_key(p) && !local_known.contains(p))
                .collect();
            if !missing.is_empty() {
                return Err(CrdtError::MissingParents {
                    dot: op.id,
                    missing,
                });
            }

            if let Some(existing) = self.ops.get(&op.id)
                && (existing.parents != op.parents || existing.payload != op.payload)
            {
                return Err(CrdtError::DotConflict { dot: op.id });
            }
            if self.ops.contains_key(&op.id) {
                already_dots.push(op.id);
            }

            op.id
                .clock
                .checked_add(1)
                .ok_or(CrdtError::ClockOverflow { dot: op.id })?;
        }

        // A cs is genuinely idempotent only when every dot is known *and*
        // the cs matches a stored changeset verbatim — same dots under a
        // different boundary, or partial overlap, both signal corruption.
        if !already_dots.is_empty() {
            let all_known = already_dots.len() == cs.ops.len();
            if all_known && self.changesets.iter().any(|c| c.as_ref() == &cs) {
                return Ok(self.clone());
            }
            return Err(CrdtError::PartialDuplicate { dots: already_dots });
        }

        // Phase A above leaves no rejectable state; the cloned mutation below
        // is infallible.
        let mut next = self.clone();
        for op in &cs.ops {
            next.ops.insert(op.id, op.clone());
            for p in &op.parents {
                next.heads.remove(p);
                next.children.entry(*p).or_default().insert(op.id);
            }
            next.advance_clock_past(&op.id);
        }
        for op in &cs.ops {
            if next
                .children
                .get(&op.id)
                .is_none_or(imbl::HashSet::is_empty)
            {
                next.heads.insert(op.id);
            }
        }
        for op in &cs.ops {
            next = next.try_promote_self_contained(op.id);
        }
        next.changesets.push_back(std::sync::Arc::new(cs));
        Ok(next)
    }

    /// First failure short-circuits and the half-built `OpGraph` never
    /// escapes — callers see all-or-nothing replay.
    pub fn from_changesets(css: Vec<crate::Changeset<P>>) -> Result<Self, CrdtError> {
        let mut g = Self::new();
        for cs in css {
            g = g.receive_changeset(cs)?;
        }
        Ok(g)
    }
}

impl<P> OpGraph<P> {
    /// Phase B caller has already passed `op.id.clock.checked_add(1)` in
    /// Phase A, so the overflow path is unreachable here.
    fn advance_clock_past(&mut self, dot: &Dot) {
        let advanced = dot
            .clock
            .checked_add(1)
            .expect("Phase A must have rejected ClockOverflow");
        self.next_clock = self.next_clock.max(advanced);
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
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![a] })
            .unwrap();
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![b] })
            .unwrap();
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![c] })
            .unwrap();
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
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![root] })
            .unwrap();
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![a] })
            .unwrap();
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![b] })
            .unwrap();
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
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![root] })
            .unwrap();
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![a] })
            .unwrap();
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![b] })
            .unwrap();
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![m] })
            .unwrap();
        let heads: Vec<&Dot> = g.current_heads().collect();
        assert_eq!(
            heads,
            vec![&Dot::new(3, 0)],
            "merge collapses to single head"
        );
    }

    #[test]
    fn receive_with_partial_missing_parents_lists_only_missing() {
        let g: OpGraph<u32> = OpGraph::with_actor(0);
        let present = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 0,
        };
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![present.clone()],
            })
            .unwrap();
        let missing_a = Dot::new(99, 5);
        let missing_b = Dot::new(99, 6);
        let op = Op {
            id: Dot::new(99, 7),
            parents: vec![present.id, missing_a, missing_b],
            payload: 1,
        };
        let result = g.receive_changeset(crate::Changeset { ops: vec![op] });
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
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![root.clone()],
            })
            .unwrap();
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![child] })
            .unwrap();
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![root] })
            .unwrap();
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
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![observed],
            })
            .unwrap();
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
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![high] })
            .unwrap();
        let low = Op {
            id: Dot::new(98, 3),
            parents: vec![],
            payload: 2,
        };
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![low] })
            .unwrap();
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
        let cs = crate::Changeset { ops: vec![op] };
        let g = g.receive_changeset(cs.clone()).unwrap();
        let g = g.receive_changeset(cs).unwrap();
        let (_g, next) = g.add(2).unwrap();
        assert_eq!(next.id, Dot::new(7, 51));
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
        let g1 = g1
            .receive_changeset(crate::Changeset {
                ops: vec![a.clone()],
            })
            .unwrap();
        let g1 = g1
            .receive_changeset(crate::Changeset {
                ops: vec![b.clone()],
            })
            .unwrap();
        let g2 = g2
            .receive_changeset(crate::Changeset { ops: vec![b] })
            .unwrap();
        let g2 = g2
            .receive_changeset(crate::Changeset { ops: vec![a] })
            .unwrap();
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
        let g1 = g1
            .receive_changeset(crate::Changeset {
                ops: vec![a.clone()],
            })
            .unwrap();
        let g1 = g1
            .receive_changeset(crate::Changeset {
                ops: vec![b.clone()],
            })
            .unwrap();
        let g2 = g2
            .receive_changeset(crate::Changeset { ops: vec![b] })
            .unwrap();
        let g2 = g2
            .receive_changeset(crate::Changeset { ops: vec![a] })
            .unwrap();
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
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![theirs.clone()],
            })
            .unwrap();
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
    fn receive_does_not_resurrect_ancestor_as_head() {
        // a → b → c chain. Drop a to model server-side data loss, then
        // re-receive it via the recovery path. The descendants are still
        // present, so a must not become a head — heads must stay {c}.
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let (g, _b) = g.add(2).unwrap();
        let (g, c) = g.add(3).unwrap();
        let g = g.debug_remove(&a.id);
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![a.clone()],
            })
            .unwrap();
        let heads: HashSet<Dot> = g.current_heads().copied().collect();
        let expected: HashSet<Dot> = [c.id].into_iter().collect();
        assert_eq!(heads, expected);
    }

    #[test]
    fn missing_changesets_for_walk_reports_dropped_ancestor_as_unknown() {
        // a → b → c across three sealed changesets. Server simulates losing a.
        // missing_changesets_for({c}) walks c → b → a, finds a missing in
        // self.ops, and reports a as unknown so the peer can re-send it.
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let g = g.commit();
        let (g, _b) = g.add(2).unwrap();
        let g = g.commit();
        let (g, c) = g.add(3).unwrap();
        let g = g.commit();
        let g = g.debug_remove(&a.id);
        let remote_heads: HashSet<Dot> = [c.id].into_iter().collect();
        let err = g.missing_changesets_for(&remote_heads).unwrap_err();
        match err {
            CrdtError::UnknownHeads { unknown } => {
                assert_eq!(unknown, vec![a.id]);
            }
            _ => panic!("expected UnknownHeads"),
        }
    }

    #[test]
    fn add_after_lost_ancestor_does_not_mark_new_op_self_contained() {
        // a → b → c. Lose b, then locally add d on top of the now-broken
        // frontier (heads become {a, c} after `debug_remove` re-promotes a;
        // d's parents = [a, c]). c is no longer self-contained, so d
        // inherits its broken ancestry and must not be emitted by
        // `missing_changesets_for(empty)` — otherwise a fresh peer would
        // reject with `MissingParents`.
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let g = g.commit();
        let (g, b) = g.add(2).unwrap();
        let g = g.commit();
        let (g, _c) = g.add(3).unwrap();
        let g = g.commit();
        let g = g.debug_remove(&b.id);
        let (g, _d) = g.add(4).unwrap();
        let g = g.commit();
        let css = g.missing_changesets_for(&HashSet::new()).unwrap();
        // Only the cs holding `a` survives the self-contained filter.
        assert_eq!(css.len(), 1);
        assert_eq!(css[0].ops[0].id, a.id);
    }

    #[test]
    fn add_after_recovery_promotes_descendant_chain_to_self_contained() {
        // Same loss as above, but b is restored before sync. The cascade
        // through `self.children` must promote c and d (built post-loss)
        // back into `self_contained`, so `has_dangling` returns false and
        // the replica is no longer sync-degraded.
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, _a) = g.add(1).unwrap();
        let (g, b) = g.add(2).unwrap();
        let (g, _c) = g.add(3).unwrap();
        let g = g.debug_remove(&b.id);
        let (g, _d) = g.add(4).unwrap();
        assert!(g.has_dangling());
        // Restore b: the cascade must propagate self_contained through c and d.
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![b] })
            .unwrap();
        assert!(
            !g.has_dangling(),
            "all descendants must be self-contained after recovery"
        );
    }

    #[test]
    fn has_dangling_observable_after_non_head_loss() {
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
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![b.clone()],
            })
            .unwrap();
        assert!(!g.has_dangling());
    }

    #[test]
    fn ancestry_of_collects_causal_past_inclusive() {
        // a -> b -> c (linear), d (separate root)
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let (g, b) = g.add(2).unwrap();
        let (g, c) = g.add(3).unwrap();
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![Op {
                    id: Dot::new(9, 0),
                    parents: vec![],
                    payload: 99,
                }],
            })
            .unwrap();

        // heads = {b} ancestry = {a, b} (excludes c, d)
        let heads: HashSet<Dot> = [b.id].into_iter().collect();
        let anc = g.ancestry_of(&heads);
        let expected: HashSet<Dot> = [a.id, b.id].into_iter().collect();
        assert_eq!(anc, expected);
        assert!(!anc.contains(&c.id));
        assert!(!anc.contains(&Dot::new(9, 0)));
    }

    #[test]
    fn ancestry_of_skips_unknown_heads() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let unknown = Dot::new(42, 7);
        let heads: HashSet<Dot> = [a.id, unknown].into_iter().collect();
        let anc = g.ancestry_of(&heads);
        assert_eq!(anc, [a.id].into_iter().collect::<HashSet<Dot>>());
    }

    #[test]
    fn ancestry_of_dedups_diamond_merge() {
        let g: OpGraph<u32> = OpGraph::with_actor(0);
        let a = Op {
            id: Dot::new(9, 0),
            parents: vec![],
            payload: 0,
        };
        let b = Op {
            id: Dot::new(1, 0),
            parents: vec![a.id],
            payload: 1,
        };
        let c = Op {
            id: Dot::new(2, 0),
            parents: vec![a.id],
            payload: 2,
        };
        let m = Op {
            id: Dot::new(3, 0),
            parents: vec![b.id, c.id],
            payload: 3,
        };
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![a.clone()],
            })
            .unwrap();
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![b.clone()],
            })
            .unwrap();
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![c.clone()],
            })
            .unwrap();
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![m.clone()],
            })
            .unwrap();
        let heads: HashSet<Dot> = [m.id].into_iter().collect();
        let anc = g.ancestry_of(&heads);
        let expected: HashSet<Dot> = [a.id, b.id, c.id, m.id].into_iter().collect();
        assert_eq!(anc, expected);
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

    #[test]
    fn add_pushes_to_pending_only() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, _op) = g.add(42).unwrap();
        assert!(g.changesets().is_empty(), "no sealed cs yet");
        assert_eq!(g.pending().len(), 1, "op should be in pending");
        assert_eq!(g.len(), 1, "derived ops index sees pending op");
    }

    #[test]
    fn commit_seals_pending_into_one_changeset() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, _) = g.add(1).unwrap();
        let (g, _) = g.add(2).unwrap();
        let g = g.commit();
        assert_eq!(g.changesets().len(), 1);
        assert_eq!(g.changesets()[0].ops.len(), 2);
        assert!(g.pending().is_empty());
    }

    #[test]
    fn commit_on_empty_pending_is_noop() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let g_committed = g.clone().commit();
        assert_eq!(g_committed.changesets().len(), 0);
        assert!(g_committed.pending().is_empty());
    }

    #[test]
    fn add_after_commit_starts_new_changeset() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, _) = g.add(1).unwrap();
        let g = g.commit();
        let (g, _) = g.add(2).unwrap();
        let g = g.commit();
        assert_eq!(g.changesets().len(), 2);
        assert_eq!(g.changesets()[0].ops.len(), 1);
        assert_eq!(g.changesets()[1].ops.len(), 1);
    }

    #[test]
    fn receive_changeset_empty_rejects() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let cs = crate::Changeset::<u32> { ops: vec![] };
        let err = g.receive_changeset(cs).unwrap_err();
        assert!(matches!(err, CrdtError::EmptyChangeset));
    }

    #[test]
    fn receive_changeset_missing_parents_rejects_atomically() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let missing = Dot::new(99, 0);
        let op = Op {
            id: Dot::new(99, 1),
            parents: vec![missing],
            payload: 1,
        };
        let cs = crate::Changeset { ops: vec![op] };
        let result = g.receive_changeset(cs);
        assert!(matches!(result, Err(CrdtError::MissingParents { .. })));
    }

    #[test]
    fn receive_changeset_self_reference_rejects() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let id = Dot::new(99, 0);
        let op = Op {
            id,
            parents: vec![id],
            payload: 1,
        };
        let cs = crate::Changeset { ops: vec![op] };
        let result = g.receive_changeset(cs);
        assert!(matches!(result, Err(CrdtError::SelfReference { .. })));
    }

    #[test]
    fn receive_changeset_dot_conflict_rejects() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let op_a = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 1,
        };
        let g = g
            .receive_changeset(crate::Changeset { ops: vec![op_a] })
            .unwrap();
        let op_b = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 2,
        };
        let result = g.receive_changeset(crate::Changeset { ops: vec![op_b] });
        assert!(matches!(result, Err(CrdtError::DotConflict { .. })));
    }

    #[test]
    fn receive_changeset_full_duplicate_idempotent() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let op = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 1,
        };
        let cs = crate::Changeset { ops: vec![op] };
        let g1 = g.receive_changeset(cs.clone()).unwrap();
        let g2 = g1.clone().receive_changeset(cs).unwrap();
        assert_eq!(g1.changesets().len(), g2.changesets().len());
        assert_eq!(g1.len(), g2.len());
    }

    #[test]
    fn receive_changeset_partial_duplicate_rejects() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let a = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 1,
        };
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![a.clone()],
            })
            .unwrap();
        let b = Op {
            id: Dot::new(99, 1),
            parents: vec![a.id],
            payload: 2,
        };
        let cs = crate::Changeset { ops: vec![a, b] };
        let result = g.receive_changeset(cs);
        assert!(matches!(result, Err(CrdtError::PartialDuplicate { .. })));
    }

    #[test]
    fn receive_changeset_intra_cs_duplicate_dot_rejects() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let dot = Dot::new(99, 0);
        let a = Op {
            id: dot,
            parents: vec![],
            payload: 1,
        };
        let b = Op {
            id: dot,
            parents: vec![],
            payload: 2,
        };
        let cs = crate::Changeset { ops: vec![a, b] };
        let result = g.receive_changeset(cs);
        assert!(matches!(result, Err(CrdtError::DotConflict { .. })));
    }

    #[test]
    fn receive_changeset_normalizes_parents() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let p1 = Op {
            id: Dot::new(98, 0),
            parents: vec![],
            payload: 1,
        };
        let p2 = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 2,
        };
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![p1.clone(), p2.clone()],
            })
            .unwrap();
        let child = Op {
            id: Dot::new(99, 1),
            parents: vec![p2.id, p1.id, p2.id],
            payload: 3,
        };
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![child.clone()],
            })
            .unwrap();
        let stored = g.get(&child.id).unwrap();
        let mut expected = vec![p1.id, p2.id];
        expected.sort();
        assert_eq!(stored.parents, expected);
    }

    #[test]
    fn receive_changeset_clock_overflow_at_any_op_rejects() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let ok = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 1,
        };
        let bad = Op {
            id: Dot::new(99, u64::MAX),
            parents: vec![ok.id],
            payload: 2,
        };
        let cs = crate::Changeset { ops: vec![ok, bad] };
        let result = g.receive_changeset(cs);
        assert!(matches!(result, Err(CrdtError::ClockOverflow { .. })));
    }

    #[test]
    fn receive_changeset_single_op_dup_against_multi_op_cs_rejects() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
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
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![a.clone(), b],
            })
            .unwrap();
        let probe = crate::Changeset { ops: vec![a] };
        let result = g.receive_changeset(probe);
        assert!(matches!(result, Err(CrdtError::PartialDuplicate { .. })));
    }

    #[test]
    fn receive_changeset_same_dots_different_boundary_rejects() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
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
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![a.clone()],
            })
            .unwrap();
        let g = g
            .receive_changeset(crate::Changeset {
                ops: vec![b.clone()],
            })
            .unwrap();
        let result = g.receive_changeset(crate::Changeset { ops: vec![a, b] });
        assert!(matches!(result, Err(CrdtError::PartialDuplicate { .. })));
    }

    #[test]
    fn receive_changeset_success_appends_and_updates_caches() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
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
        let cs = crate::Changeset {
            ops: vec![a.clone(), b.clone()],
        };
        let g2 = g.receive_changeset(cs).unwrap();
        assert_eq!(g2.changesets().len(), 1);
        assert_eq!(g2.changesets()[0].ops.len(), 2);
        assert_eq!(g2.len(), 2);
        let heads: HashSet<Dot> = g2.current_heads().copied().collect();
        assert_eq!(heads, [b.id].into_iter().collect());
    }

    #[test]
    fn from_changesets_round_trip() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, _) = g.add(1).unwrap();
        let (g, _) = g.add(2).unwrap();
        let g = g.commit();
        let (g, _) = g.add(3).unwrap();
        let g = g.commit();

        let css = g.changesets_as_vec();
        let g2 = OpGraph::<u32>::from_changesets(css.clone()).unwrap();
        assert_eq!(g2.changesets_as_vec(), css);
        assert_eq!(g2.len(), 3);
        assert!(g2.graph_state_eq(&g));
    }

    #[test]
    fn from_changesets_rejects_bad_input() {
        let bad = crate::Changeset {
            ops: vec![Op {
                id: Dot::new(99, 1),
                parents: vec![Dot::new(99, 0)], // missing parent
                payload: 1u32,
            }],
        };
        let result = OpGraph::<u32>::from_changesets(vec![bad]);
        assert!(matches!(result, Err(CrdtError::MissingParents { .. })));
    }

    #[test]
    fn missing_changesets_for_empty_remote_returns_all() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, _) = g.add(1).unwrap();
        let (g, _) = g.add(2).unwrap();
        let g = g.commit();
        let (g, _) = g.add(3).unwrap();
        let g = g.commit();
        let css = g.missing_changesets_for(&HashSet::new()).unwrap();
        assert_eq!(css.len(), 2);
        assert_eq!(css[0].ops.len(), 2);
        assert_eq!(css[1].ops.len(), 1);
    }

    #[test]
    fn missing_changesets_for_full_remote_returns_empty() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, _) = g.add(1).unwrap();
        let g = g.commit();
        let head = *g.current_heads().next().unwrap();
        let css = g
            .missing_changesets_for(&[head].into_iter().collect())
            .unwrap();
        assert!(css.is_empty());
    }

    #[test]
    fn missing_changesets_for_partial_remote_returns_subset() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, op_a) = g.add(1).unwrap();
        let g = g.commit();
        let (g, _) = g.add(2).unwrap();
        let g = g.commit();
        let css = g
            .missing_changesets_for(&[op_a.id].into_iter().collect())
            .unwrap();
        assert_eq!(css.len(), 1);
        assert_eq!(css[0].ops[0].payload, 2);
    }

    #[test]
    fn missing_changesets_for_unknown_head_errors() {
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, _) = g.add(1).unwrap();
        let g = g.commit();
        let unknown = Dot::new(42, 0);
        let result = g.missing_changesets_for(&[unknown].into_iter().collect());
        assert!(matches!(result, Err(CrdtError::UnknownHeads { .. })));
    }

    #[test]
    fn missing_changesets_for_drops_dangling_cs_silently() {
        // Build a → b → c chain across three sealed cs, then simulate
        // server-side data loss of `b` via debug_remove. The cs containing
        // c is no longer self-contained; missing_changesets_for must drop
        // it silently (not surface as PartialDuplicate).
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let g = g.commit();
        let (g, b) = g.add(2).unwrap();
        let g = g.commit();
        let (g, _c) = g.add(3).unwrap();
        let g = g.commit();
        let g = g.debug_remove(&b.id);
        let css = g.missing_changesets_for(&HashSet::new()).unwrap();
        // Only the cs holding `a` survives the self-contained filter.
        assert_eq!(css.len(), 1);
        assert_eq!(css[0].ops[0].id, a.id);
    }

    #[test]
    fn missing_changesets_for_mixed_known_signals_partial_duplicate() {
        // Sealed cs containing two ops [a, b]. Tell missing_changesets_for
        // that remote already has a but not b. That mixed state is a
        // corruption signal — atomicity says boundaries are preserved
        // everywhere, so a known-prefix-of-cs case must surface as
        // PartialDuplicate, not silently emit half a cs.
        let g: OpGraph<u32> = OpGraph::with_actor(1);
        let (g, a) = g.add(1).unwrap();
        let (g, _b) = g.add(2).unwrap();
        let g = g.commit();
        let result = g.missing_changesets_for(&[a.id].into_iter().collect());
        assert!(matches!(result, Err(CrdtError::PartialDuplicate { .. })));
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
            .try_fold(OpGraph::with_actor(0), |g, op| {
                g.receive_changeset(crate::Changeset { ops: vec![op] })
            })
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
