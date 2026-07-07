use editor_macros::ffi;
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

use crate::dot_map::DotMap;
use crate::{CrdtError, Dot};

/// Sorted-ascending child list. Sorted so set semantics (dedup, equality)
/// hold regardless of insertion order; inline capacity 2 keeps the common
/// one-child case heap-free.
type ChildSet = smallvec::SmallVec<[Dot; 2]>;

fn child_insert(set: &mut ChildSet, dot: Dot) {
    if let Err(i) = set.binary_search(&dot) {
        set.insert(i, dot);
    }
}

#[cfg(test)]
fn child_remove(set: &mut ChildSet, dot: &Dot) {
    if let Ok(i) = set.binary_search(dot) {
        set.remove(i);
    }
}

/// Sealed-changeset descriptor: the member dots, in op order, compressed as
/// consecutive-clock runs. Storing the full `Changeset` duplicated every
/// op's payload and parents in memory; the wire/FFI `Changeset` is
/// materialized from `ops` on demand instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangesetRef {
    runs: smallvec::SmallVec<[(Dot, u32); 2]>,
    len: u32,
}

impl ChangesetRef {
    fn from_ops<P>(ops: &[Op<P>]) -> Self {
        let mut runs: smallvec::SmallVec<[(Dot, u32); 2]> = smallvec::SmallVec::new();
        for op in ops {
            match runs.last_mut() {
                Some((start, len))
                    if op.id.actor == start.actor
                        && op.id.clock == start.clock + u64::from(*len) =>
                {
                    *len += 1
                }
                _ => runs.push((op.id, 1)),
            }
        }
        ChangesetRef {
            runs,
            len: ops.len() as u32,
        }
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn first(&self) -> Option<Dot> {
        self.runs.first().map(|(d, _)| *d)
    }

    pub fn dots(&self) -> impl Iterator<Item = Dot> + '_ {
        self.runs.iter().flat_map(|(start, len)| {
            (0..u64::from(*len)).map(move |i| Dot::new(start.actor, start.clock + i))
        })
    }

    fn matches_ops<P>(&self, ops: &[Op<P>]) -> bool {
        self.len() == ops.len() && self.dots().zip(ops).all(|(d, op)| d == op.id)
    }
}

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
#[derive(Clone, Debug)]
pub struct OpGraph<P> {
    actor: u64,
    next_clock: u64,

    changesets: imbl::Vector<ChangesetRef>,

    /// Local-write accumulator. `add` pushes here; `commit` drains into
    /// `changesets`. Remote ingestion (`receive_changeset`) never touches
    /// this — it seals its own boundary directly into `changesets`, so
    /// `pending` only ever holds the in-flight transaction's ops.
    pending: Vec<Op<P>>,

    ops: DotMap<Op<P>>,
    heads: imbl::HashSet<Dot>,
    /// Reverse parent index: each dot maps to the set of ops that reference
    /// it as a parent. Drives O(1) `has_child` checks during frontier
    /// maintenance and powers the cascade walks that keep `self_contained`
    /// accurate on data loss / restore. Values are sorted-ascending
    /// `SmallVec`s: almost every op has exactly one child (linear typing
    /// history), and a per-op `imbl::HashSet` costs hundreds of heap bytes
    /// each — on an 800k-op document this map alone dominated graph memory.
    children: DotMap<ChildSet>,
    /// Subset of `ops` whose transitive ancestry is fully present locally.
    /// Maintained incrementally so `missing_changesets_for` can emit only
    /// replayable batches in O(walk) rather than scanning the full op set
    /// per round. Diverges from `ops` only after server-side data loss
    /// (`debug_remove`); `add` and `receive_changeset` keep
    /// parents-before-children invariants so they always preserve the set.
    self_contained: DotMap<()>,
}

impl<P: Clone + PartialEq> OpGraph<P> {
    pub fn graph_state_eq(&self, other: &Self) -> bool {
        self.ops == other.ops && self.heads == other.heads
    }
}

// Manual impls: `derive` can't infer the `P: Clone` bound the `DotMap`
// fields need for comparison.
impl<P: Clone + PartialEq> PartialEq for OpGraph<P> {
    fn eq(&self, other: &Self) -> bool {
        self.actor == other.actor
            && self.next_clock == other.next_clock
            && self.changesets == other.changesets
            && self.pending == other.pending
            && self.ops == other.ops
            && self.heads == other.heads
            && self.children == other.children
            && self.self_contained == other.self_contained
    }
}

impl<P: Clone + Eq> Eq for OpGraph<P> {}

impl<P: Clone> OpGraph<P> {
    pub fn new() -> Self {
        let mut buf = [0u8; 8];
        let actor = loop {
            getrandom::fill(&mut buf).expect("failed to generate random bytes");
            let candidate = u64::from_le_bytes(buf);
            if candidate != 0 {
                break candidate;
            }
        };
        Self::with_actor(actor)
    }

    pub fn with_actor(actor: u64) -> Self {
        Self {
            actor,
            next_clock: 0,
            changesets: imbl::Vector::new(),
            pending: Vec::new(),
            ops: DotMap::new(),
            heads: imbl::HashSet::new(),
            children: DotMap::new(),
            self_contained: DotMap::new(),
        }
    }

    pub fn current_heads(&self) -> impl Iterator<Item = &Dot> + '_ {
        self.heads.iter()
    }

    /// Frontier of a changeset set *without building the graph*: an op is a
    /// head iff no op references it as a parent, i.e. `all op ids − all
    /// referenced parent ids`. `O(total ops)` with two plain `HashSet`s and no
    /// persistent-collection construction, so it is the right primitive when
    /// only the heads are needed (heads / durableHeads reporting) rather than
    /// the whole DAG — a full `from_changesets` build is orders of magnitude
    /// more expensive. For any graph `from_changesets` accepts (no dangling
    /// parents) this equals [`current_heads`], since `heads` there is exactly
    /// the set of childless ops.
    ///
    /// [`current_heads`]: OpGraph::current_heads
    pub fn heads_of(css: &[crate::Changeset<P>]) -> Vec<Dot> {
        let mut all: HashSet<Dot> = HashSet::new();
        let mut parents: HashSet<Dot> = HashSet::new();
        for cs in css {
            for op in &cs.ops {
                all.insert(op.id);
                for p in &op.parents {
                    parents.insert(*p);
                }
            }
        }
        let mut heads: Vec<Dot> = all.into_iter().filter(|d| !parents.contains(d)).collect();
        // Deterministic wire output; consumers dedupe into a set anyway.
        heads.sort();
        heads
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

    pub fn changesets(&self) -> &imbl::Vector<ChangesetRef> {
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

    /// All ops in ancestry-first order *without cloning them*: sealed
    /// changesets in storage order, then `pending` in push order. Both
    /// append paths (`receive_changeset_mut`, `add_mut`) only accept ops
    /// whose parents are already present, so this concatenation is a valid
    /// topological order for any graph they built. Returns `None` when the
    /// invariant does not hold (e.g. after test-only `debug_remove`, or a
    /// duplicate/missing op) — callers must fall back to [`topo_sort`].
    ///
    /// [`topo_sort`]: OpGraph::topo_sort
    pub fn ordered_ops(&self) -> Option<Vec<&Op<P>>> {
        let mut out: Vec<&Op<P>> = Vec::with_capacity(self.ops.len());
        let mut seen: HashSet<Dot> = HashSet::with_capacity(self.ops.len());
        let all = self
            .changesets
            .iter()
            .flat_map(|r| r.dots())
            .map(|d| self.ops.get(&d))
            .chain(self.pending.iter().map(Some));
        for op in all {
            let op = op?;
            if !op.parents.iter().all(|p| seen.contains(p)) || !seen.insert(op.id) {
                return None;
            }
            out.push(op);
        }
        if out.len() != self.ops.len() {
            return None;
        }
        Some(out)
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
                    child_remove(set, dot);
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

impl<P: Clone> Default for OpGraph<P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Clone> OpGraph<P> {
    /// Rebuild the wire-form `Changeset` for a sealed descriptor from `ops`.
    /// Every dot of a sealed changeset is present in `ops` (the test-only
    /// `debug_remove` is the sole violation, and its callers filter through
    /// `self_contained` first).
    pub fn materialize_changeset(&self, r: &ChangesetRef) -> crate::Changeset<P> {
        crate::Changeset {
            ops: r
                .dots()
                .map(|d| self.ops.get(&d).expect("sealed op present").clone())
                .collect(),
        }
    }

    pub fn changesets_as_vec(&self) -> Vec<crate::Changeset<P>> {
        self.changesets
            .iter()
            .map(|r| self.materialize_changeset(r))
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
            child_insert(self.children.entry_or_default(*p), id);
        }
        self.heads.insert(id);
        // After server-side data loss, `debug_remove` re-promotes a non-SC
        // ancestor into `heads` so the frontier walk can still see it. A new
        // op built on such a head inherits its broken ancestry, so derive SC
        // status from the parents instead of assuming all heads qualify.
        // The op stays in `self.ops` either way; once the missing ancestor is
        // restored, `receive_changeset`'s `try_promote_self_contained` cascade
        // lifts it through `self.children` automatically.
        if parents.iter().all(|p| self.self_contained.contains_key(p)) {
            self.self_contained.insert(id, ());
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
            self.changesets.push_back(ChangesetRef::from_ops(&ops));
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
        for r in &self.changesets {
            // Self-contained filter must precede the membership classification
            // below: a cs holding a dangling-ancestor descendant is a degraded-
            // storage case to drop silently, not an atomicity violation.
            if r.dots().any(|d| !self.self_contained.contains_key(&d)) {
                continue;
            }
            let mut all_known = true;
            let mut any_known = false;
            for d in r.dots() {
                if known.contains(&d) {
                    any_known = true;
                } else {
                    all_known = false;
                }
            }
            if all_known {
                continue;
            }
            if any_known {
                let dots: Vec<Dot> = r.dots().filter(|d| known.contains(d)).collect();
                return Err(CrdtError::PartialDuplicate { dots });
            }
            out.push(self.materialize_changeset(r));
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

/// Reusable per-changeset scratch buffers. A bulk replay
/// (`from_changesets`, `receive_changesets_ordered`) processes hundreds of
/// thousands of changesets; allocating these fresh per changeset dominated
/// the replay under wasm's allocator.
#[derive(Default)]
struct ReceiveScratch {
    local_known: HashSet<Dot>,
    already_dots: Vec<Dot>,
    promote_queue: Vec<Dot>,
}

impl<P: Clone + Eq> OpGraph<P> {
    fn try_promote_self_contained_mut(&mut self, root: Dot, queue: &mut Vec<Dot>) {
        queue.clear();
        queue.push(root);
        while let Some(dot) = queue.pop() {
            if self.self_contained.contains_key(&dot) {
                continue;
            }
            let promotable = match self.ops.get(&dot) {
                Some(op) => op
                    .parents
                    .iter()
                    .all(|p| self.self_contained.contains_key(p)),
                None => continue,
            };
            if !promotable {
                continue;
            }
            self.self_contained.insert(dot, ());
            if let Some(children) = self.children.get(&dot) {
                queue.extend(children.iter().copied());
            }
        }
    }

    /// Phase A validates fully against `&self` before any mutation; Phase B
    /// applies a clone-and-replace that is unreachable on the failure
    /// paths above. The whole call is therefore all-or-nothing.
    pub fn receive_changeset(&self, cs: crate::Changeset<P>) -> Result<Self, CrdtError> {
        let mut next = self.clone();
        next.receive_changeset_mut(cs)?;
        Ok(next)
    }

    /// In-place variant of [`receive_changeset`]. Skips the per-call
    /// `self.clone()` so a bulk builder (`from_changesets`) that owns the graph
    /// mutates the persistent collections while they are uniquely referenced —
    /// each `imbl` insert then updates in place instead of copy-on-write
    /// path-copying, turning an `O(N)`-clone-per-changeset replay into a single
    /// linear pass. Validation is Phase-A-before-Phase-B all-or-nothing: on any
    /// `Err` the graph is left untouched (Phase A only reads `self`).
    pub fn receive_changeset_mut(&mut self, cs: crate::Changeset<P>) -> Result<(), CrdtError> {
        self.receive_changeset_mut_scratch(cs, &mut ReceiveScratch::default())
    }

    fn receive_changeset_mut_scratch(
        &mut self,
        cs: crate::Changeset<P>,
        scratch: &mut ReceiveScratch,
    ) -> Result<(), CrdtError> {
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

        let local_known = &mut scratch.local_known;
        let already_dots = &mut scratch.already_dots;
        local_known.clear();
        already_dots.clear();

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
            // Phase A already proved every known dot's stored op matches this
            // cs verbatim (parents + payload), so idempotency only needs a
            // stored changeset with the exact same dot sequence.
            let all_known = already_dots.len() == cs.ops.len();
            if all_known && self.changesets.iter().any(|r| r.matches_ops(&cs.ops)) {
                return Ok(());
            }
            return Err(CrdtError::PartialDuplicate {
                dots: std::mem::take(already_dots),
            });
        }

        // Phase A above leaves no rejectable state; the mutation below is
        // infallible.
        for op in &cs.ops {
            for p in &op.parents {
                self.heads.remove(p);
                child_insert(self.children.entry_or_default(*p), op.id);
            }
            self.advance_clock_past(&op.id);
        }
        for op in &cs.ops {
            if self.children.get(&op.id).is_none_or(|c| c.is_empty()) {
                self.heads.insert(op.id);
            }
        }
        let cref = ChangesetRef::from_ops(&cs.ops);
        for op in cs.ops {
            let id = op.id;
            self.ops.insert(id, op);
        }
        for d in cref.dots() {
            self.try_promote_self_contained_mut(d, &mut scratch.promote_queue);
        }
        self.changesets.push_back(cref);
        Ok(())
    }

    /// First failure short-circuits and the half-built `OpGraph` never
    /// escapes — callers see all-or-nothing replay. Builds in place
    /// (`receive_changeset_mut`) so the whole replay is a single linear pass,
    /// not an `O(N)` persistent-collection clone per changeset.
    pub fn from_changesets(css: Vec<crate::Changeset<P>>) -> Result<Self, CrdtError> {
        let mut g = Self::new();
        let mut scratch = ReceiveScratch::default();
        for cs in css {
            g.receive_changeset_mut_scratch(cs, &mut scratch)?;
        }
        Ok(g)
    }

    pub fn receive_changesets_ordered(
        &self,
        css: Vec<crate::Changeset<P>>,
    ) -> (Self, Vec<crate::Changeset<P>>) {
        use hashbrown::HashMap;
        let n = css.len();
        // 각 도입 dot -> 그 dot을 만드는 css 인덱스.
        let mut producer: HashMap<Dot, usize> = HashMap::new();
        for (i, cs) in css.iter().enumerate() {
            for op in &cs.ops {
                producer.insert(op.id, i);
            }
        }
        // unmet[i] = i가 아직 못 받은 부모 의존 수(그래프에도 없고 같은 css 내 선행도 아닌 부모).
        // dependents[j] = j가 적용되면 unmet 감소할 css들.
        let mut unmet: Vec<usize> = vec![0; n];
        let mut dependents: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, cs) in css.iter().enumerate() {
            let mut intra: HashSet<Dot> = HashSet::new();
            for op in &cs.ops {
                for p in &op.parents {
                    if intra.contains(p) || self.ops.contains_key(p) {
                        continue;
                    }
                    match producer.get(p) {
                        Some(&j) if j != i => {
                            unmet[i] += 1;
                            dependents.entry(j).or_default().push(i);
                        }
                        Some(_) => {}
                        None => unmet[i] += 1, // 그래프에도 pending에도 없는 부모 → 영영 미충족(고아)
                    }
                }
                intra.insert(op.id);
            }
        }
        let mut graph = self.clone();
        let mut queue: std::collections::VecDeque<usize> =
            (0..n).filter(|&i| unmet[i] == 0).collect();
        let mut applied: Vec<bool> = vec![false; n];
        let mut scratch = ReceiveScratch::default();
        while let Some(i) = queue.pop_front() {
            if applied[i] {
                continue;
            }
            // receive_changeset_mut은 verbatim 중복에 Ok(())를 돌려주므로 진전으로 처리됨.
            // Err 시 Phase A 전량거부라 graph는 무변경 → 안전하게 in-place 재시도/드롭.
            match graph.receive_changeset_mut_scratch(css[i].clone(), &mut scratch) {
                Ok(()) => {
                    applied[i] = true;
                    if let Some(deps) = dependents.get(&i) {
                        for &d in deps {
                            unmet[d] = unmet[d].saturating_sub(1);
                            if unmet[d] == 0 {
                                queue.push_back(d);
                            }
                        }
                    }
                }
                Err(_) => { /* PartialDuplicate 등: 적용 안 함 → 아래에서 dropped 처리 */
                }
            }
        }
        let dropped: Vec<crate::Changeset<P>> = css
            .into_iter()
            .enumerate()
            .filter(|(i, _)| !applied[*i])
            .map(|(_, cs)| cs)
            .collect();
        (graph, dropped)
    }

    /// Same readiness partition as [`Self::partition_ready`], but over indices into
    /// `css` instead of the changesets themselves — callers that hold an out-of-band
    /// byte representation aligned 1:1 with `css` (e.g. a bundle split into
    /// per-changeset byte chunks) can select from that representation instead of
    /// re-encoding the values. `ready` is in dependency order, `blocked` in original
    /// order — `partition_ready` is a value-picking wrapper over this.
    pub fn partition_ready_indices(&self, css: &[crate::Changeset<P>]) -> (Vec<usize>, Vec<usize>) {
        // Kahn O(n+refs) (round-2 MED: naive 반복 스캔 금지). ready는 의존성 순서, blocked는 원래 순서.
        use hashbrown::HashMap;
        let n = css.len();
        let mut producer: HashMap<Dot, usize> = HashMap::new();
        for (i, cs) in css.iter().enumerate() {
            for op in &cs.ops {
                producer.insert(op.id, i);
            }
        }
        let mut unmet: Vec<usize> = vec![0; n];
        let mut dependents: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, cs) in css.iter().enumerate() {
            let mut intra: HashSet<Dot> = HashSet::new();
            for op in &cs.ops {
                for p in &op.parents {
                    if intra.contains(p) || self.ops.contains_key(p) {
                        continue;
                    }
                    match producer.get(p) {
                        Some(&j) if j != i => {
                            unmet[i] += 1;
                            dependents.entry(j).or_default().push(i);
                        }
                        Some(_) => {}
                        None => unmet[i] += 1, // 부모가 그래프에도 pending에도 없음 → blocked
                    }
                }
                intra.insert(op.id);
            }
        }
        let mut queue: std::collections::VecDeque<usize> =
            (0..n).filter(|&i| unmet[i] == 0).collect();
        let mut ready_flag: Vec<bool> = vec![false; n];
        let mut ready_order: Vec<usize> = Vec::new();
        while let Some(i) = queue.pop_front() {
            if ready_flag[i] {
                continue;
            }
            ready_flag[i] = true;
            ready_order.push(i);
            if let Some(deps) = dependents.get(&i) {
                for &d in deps {
                    unmet[d] = unmet[d].saturating_sub(1);
                    if unmet[d] == 0 {
                        queue.push_back(d);
                    }
                }
            }
        }
        let blocked_order: Vec<usize> = (0..n).filter(|&i| !ready_flag[i]).collect();
        (ready_order, blocked_order)
    }

    pub fn partition_ready(
        &self,
        css: Vec<crate::Changeset<P>>,
    ) -> (Vec<crate::Changeset<P>>, Vec<crate::Changeset<P>>) {
        let (ready_order, blocked_order) = self.partition_ready_indices(&css);
        let mut slots: Vec<Option<crate::Changeset<P>>> = css.into_iter().map(Some).collect();
        let ready: Vec<crate::Changeset<P>> = ready_order
            .into_iter()
            .map(|i| slots[i].take().expect("ready slot present"))
            .collect();
        let blocked: Vec<crate::Changeset<P>> = blocked_order
            .into_iter()
            .map(|i| slots[i].take().expect("blocked slot present"))
            .collect();
        (ready, blocked)
    }

    pub fn missing_changesets_tolerant(
        &self,
        remote_heads: &HashSet<Dot>,
    ) -> Vec<crate::Changeset<P>> {
        // Fast path: if the remote already knows every one of our heads, its causal
        // history is a superset of ours — there is nothing to send. Skips any walk on
        // the common case of a fully-synced peer (the per-tick sync queries that
        // otherwise re-walk the whole graph each time).
        if self.heads.iter().all(|h| remote_heads.contains(h)) {
            return Vec::new();
        }
        // Two ways to find what the remote lacks: forward-walk its frontier to build its
        // causal history (`O(remote ancestry)`, cheap when it is *far behind*), or
        // backward-walk our unknown region from our heads (`O(unknown)`, cheap when it is
        // *nearly synced* — the dominant per-tick case). We don't know a priori which is
        // smaller, so run both interleaved and take whichever finishes first: total work
        // is `O(min(remote ancestry, unknown))`, optimal for both extremes with no wasted
        // half-walk from a mis-guessed budget.
        match self.remote_frontier_delta(remote_heads) {
            FrontierDelta::RemoteKnown(known) => self.collect_sendable(|dot| known.contains(dot)),
            FrontierDelta::LocalUnknown(unknown) => {
                self.collect_sendable(|dot| !unknown.contains(dot))
            }
        }
    }

    /// Emit every changeset the remote lacks, given `is_known(dot)` telling whether the
    /// remote already has an op. A changeset every op of which is known is skipped;
    /// otherwise it is sent, subject to the `self_contained` filter (a changeset with a
    /// non-self-contained op — only possible after server-side data loss — cannot be
    /// replayed and is withheld).
    fn collect_sendable(&self, is_known: impl Fn(&Dot) -> bool) -> Vec<crate::Changeset<P>> {
        // When every op is self-contained (the healthy graph — no degraded-storage
        // descendants), the per-op self-contained filter can never reject a changeset,
        // so skip it entirely (an O(1) length check vs an O(N log N) sweep of `imbl`
        // probes over the whole history when sending everything to a fresh peer).
        let all_self_contained = self.self_contained.len() == self.ops.len();
        let mut out: Vec<crate::Changeset<P>> = Vec::new();
        for r in &self.changesets {
            // Cheap `is_known` membership (a plain `HashSet`, O(1)) first: a fully-known
            // changeset is rejected without the per-op `self_contained` probe,
            // reserving that for the ones we may send.
            if r.dots().all(|d| is_known(&d)) {
                continue; // fully known → peer already has it
            }
            if !all_self_contained && r.dots().any(|d| !self.self_contained.contains_key(&d)) {
                continue;
            }
            out.push(self.materialize_changeset(r));
        }
        out
    }
}

/// Result of the interleaved frontier race in [`OpGraph::remote_frontier_delta`]:
/// whichever walk finished first. `RemoteKnown` holds the remote's full causal history
/// (forward walk won); `LocalUnknown` holds the ops the remote lacks (backward walk won).
enum FrontierDelta {
    RemoteKnown(HashSet<Dot>),
    LocalUnknown(HashSet<Dot>),
}

impl<P: Clone> OpGraph<P> {
    /// Interleaved forward (remote-ancestry) and backward (our-unknown-region) walks,
    /// returning whichever completes first. The forward walk builds the remote's causal
    /// history; the backward walk builds the set of our ops the remote lacks, pruning at
    /// the frontier. Both are correct on their own — the race just picks the cheaper one,
    /// giving `O(min(remote ancestry, unknown region))` total.
    fn remote_frontier_delta(&self, remote_heads: &HashSet<Dot>) -> FrontierDelta {
        let mut fwd_known: HashSet<Dot> = HashSet::new();
        let mut fwd_stack: Vec<Dot> = remote_heads.iter().copied().collect();

        let mut bwd_unknown: HashSet<Dot> = HashSet::new();
        let mut bwd_visited: HashSet<Dot> = HashSet::new();
        let mut bwd_stack: Vec<Dot> = self.heads.iter().copied().collect();
        let mut memo: HashMap<Dot, bool> = HashMap::new();

        loop {
            // One forward step: pop a remote ancestor, enqueue its parents.
            match fwd_stack.pop() {
                None => return FrontierDelta::RemoteKnown(fwd_known),
                Some(dot) => {
                    if self.ops.contains_key(&dot)
                        && fwd_known.insert(dot)
                        && let Some(op) = self.ops.get(&dot)
                    {
                        fwd_stack.extend(op.parents.iter().copied());
                    }
                }
            }

            // One productive backward step: pop until we classify a fresh unknown op
            // (skipping already-visited and remote-known dots), then descend it.
            loop {
                match bwd_stack.pop() {
                    None => return FrontierDelta::LocalUnknown(bwd_unknown),
                    Some(d) => {
                        if !bwd_visited.insert(d) {
                            continue;
                        }
                        if self.is_ancestor_of_frontier(d, remote_heads, &mut memo) {
                            continue; // remote has this and everything below it
                        }
                        bwd_unknown.insert(d);
                        if let Some(op) = self.ops.get(&d) {
                            for p in &op.parents {
                                bwd_stack.push(*p);
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    /// Whether `start` is an ancestor-or-self of some remote head — i.e. the remote
    /// already has it. Memoized on `memo`. Propagates known-ness *downward* through the
    /// reverse-parent (`children`) index: `start` is known iff it is a remote head or
    /// any of its children is known. Iterative post-order to bound the WASM stack.
    fn is_ancestor_of_frontier(
        &self,
        start: Dot,
        remote_heads: &HashSet<Dot>,
        memo: &mut HashMap<Dot, bool>,
    ) -> bool {
        if let Some(&v) = memo.get(&start) {
            return v;
        }
        let mut stack: Vec<(Dot, bool)> = vec![(start, false)];
        while let Some((d, expanded)) = stack.pop() {
            if memo.contains_key(&d) {
                continue;
            }
            if remote_heads.contains(&d) {
                memo.insert(d, true);
                continue;
            }
            let children = self.children.get(&d);
            if expanded {
                let known = children
                    .map(|cs| cs.iter().any(|c| memo.get(c).copied().unwrap_or(false)))
                    .unwrap_or(false);
                memo.insert(d, known);
                continue;
            }
            // First visit: resolve via already-known children, else defer behind the
            // unresolved ones.
            let mut pending: Vec<Dot> = Vec::new();
            let mut known = false;
            if let Some(cs) = children {
                for c in cs {
                    match memo.get(c) {
                        Some(true) => {
                            known = true;
                            break;
                        }
                        Some(false) => {}
                        None => pending.push(*c),
                    }
                }
            }
            if known {
                memo.insert(d, true);
            } else if pending.is_empty() {
                memo.insert(d, false);
            } else {
                stack.push((d, true));
                for c in pending {
                    stack.push((c, false));
                }
            }
        }
        memo[&start]
    }

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
        assert_eq!(g.changesets()[0].len(), 2);
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
        assert_eq!(g.changesets()[0].len(), 1);
        assert_eq!(g.changesets()[1].len(), 1);
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
        assert_eq!(g2.changesets()[0].len(), 2);
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
    fn heads_of_matches_current_heads() {
        let root = Op {
            id: Dot::new(1, 0),
            parents: vec![],
            payload: 0u32,
        };
        let a = Op {
            id: Dot::new(1, 1),
            parents: vec![root.id],
            payload: 1,
        };
        let b = Op {
            id: Dot::new(2, 0), // concurrent branch, different actor
            parents: vec![root.id],
            payload: 2,
        };
        let m = Op {
            id: Dot::new(1, 2),
            parents: vec![a.id, b.id], // merge
            payload: 3,
        };

        let assert_frontier_matches = |css: &[crate::Changeset<u32>]| {
            let g = OpGraph::<u32>::from_changesets(css.to_vec()).unwrap();
            let mut expected: Vec<Dot> = g.current_heads().copied().collect();
            expected.sort();
            assert_eq!(OpGraph::<u32>::heads_of(css), expected);
        };

        // Two concurrent heads (a, b).
        assert_frontier_matches(&[
            crate::Changeset {
                ops: vec![root.clone()],
            },
            crate::Changeset {
                ops: vec![a.clone()],
            },
            crate::Changeset {
                ops: vec![b.clone()],
            },
        ]);
        // Merged into a single head (m).
        assert_frontier_matches(&[
            crate::Changeset { ops: vec![root] },
            crate::Changeset { ops: vec![a] },
            crate::Changeset { ops: vec![b] },
            crate::Changeset { ops: vec![m] },
        ]);
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

    #[test]
    fn receive_changesets_ordered_applies_shuffled_and_drops_orphans() {
        let g: OpGraph<u32> = OpGraph::with_actor(0);
        let root = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 0,
        };
        let a = Op {
            id: Dot::new(99, 1),
            parents: vec![root.id],
            payload: 1,
        };
        let b = Op {
            id: Dot::new(99, 2),
            parents: vec![a.id],
            payload: 2,
        };
        let orphan = Op {
            id: Dot::new(99, 3),
            parents: vec![Dot::new(77, 7)],
            payload: 3,
        };

        let css = vec![
            crate::Changeset {
                ops: vec![b.clone()],
            },
            crate::Changeset {
                ops: vec![root.clone()],
            },
            crate::Changeset {
                ops: vec![a.clone()],
            },
            crate::Changeset {
                ops: vec![orphan.clone()],
            },
        ];

        let (next, dropped) = g.receive_changesets_ordered(css);
        assert!(next.contains(&root.id));
        assert!(next.contains(&a.id));
        assert!(next.contains(&b.id));
        assert!(!next.contains(&orphan.id), "orphan must not be applied");
        assert_eq!(dropped.len(), 1, "exactly the orphan changeset is dropped");
        assert_eq!(dropped[0].ops[0].id, orphan.id);
    }

    #[test]
    fn partition_ready_splits_by_parent_presence() {
        let g: OpGraph<u32> = OpGraph::with_actor(0);
        let root = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 0,
        };
        let a = Op {
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
            .receive_changeset(crate::Changeset {
                ops: vec![a.clone()],
            })
            .unwrap();

        let b = Op {
            id: Dot::new(99, 2),
            parents: vec![a.id],
            payload: 2,
        };
        let c = Op {
            id: Dot::new(99, 3),
            parents: vec![b.id],
            payload: 3,
        };
        let blocked = Op {
            id: Dot::new(99, 9),
            parents: vec![Dot::new(55, 5)],
            payload: 9,
        };

        let css = vec![
            crate::Changeset {
                ops: vec![c.clone()],
            },
            crate::Changeset {
                ops: vec![b.clone()],
            },
            crate::Changeset {
                ops: vec![blocked.clone()],
            },
        ];

        let (ready, still_blocked) = g.partition_ready(css);
        let ready_ids: Vec<Dot> = ready.iter().map(|cs| cs.ops[0].id).collect();
        assert_eq!(
            ready_ids,
            vec![b.id, c.id],
            "ready in dependency order: b then c"
        );
        assert_eq!(still_blocked.len(), 1);
        assert_eq!(still_blocked[0].ops[0].id, blocked.id);
        assert!(!g.contains(&b.id));
        assert!(!g.contains(&c.id));
    }

    #[test]
    fn partition_ready_indices_matches_partition_ready_order() {
        let g: OpGraph<u32> = OpGraph::with_actor(0);
        let root = Op {
            id: Dot::new(99, 0),
            parents: vec![],
            payload: 0,
        };
        let a = Op {
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
            .receive_changeset(crate::Changeset {
                ops: vec![a.clone()],
            })
            .unwrap();

        let b = Op {
            id: Dot::new(99, 2),
            parents: vec![a.id],
            payload: 2,
        };
        let c = Op {
            id: Dot::new(99, 3),
            parents: vec![b.id],
            payload: 3,
        };
        let blocked = Op {
            id: Dot::new(99, 9),
            parents: vec![Dot::new(55, 5)],
            payload: 9,
        };

        // Same fixture as `partition_ready_splits_by_parent_presence` (css = [c, b,
        // blocked]) — indices must line up with that test's value-based ready order
        // (b then c), i.e. index 1 (b) then index 0 (c).
        let css = vec![
            crate::Changeset {
                ops: vec![c.clone()],
            },
            crate::Changeset {
                ops: vec![b.clone()],
            },
            crate::Changeset {
                ops: vec![blocked.clone()],
            },
        ];

        let (ready_idx, blocked_idx) = g.partition_ready_indices(&css);
        assert_eq!(
            ready_idx,
            vec![1, 0],
            "ready indices in dependency order: b(1) then c(0)"
        );
        assert_eq!(blocked_idx, vec![2]);
    }

    #[test]
    fn partition_ready_classifies_verbatim_duplicate_as_ready() {
        let g: OpGraph<u32> = OpGraph::with_actor(0);
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
        let (ready, blocked) = g.partition_ready(vec![crate::Changeset {
            ops: vec![a.clone()],
        }]);
        assert_eq!(
            ready.len(),
            1,
            "verbatim duplicate is ready (applies as no-op)"
        );
        assert!(blocked.is_empty());
    }

    #[test]
    fn missing_changesets_tolerant_skips_unknown_heads_without_error() {
        let g: OpGraph<u32> = OpGraph::with_actor(0);
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
        let heads: HashSet<Dot> = [Dot::new(99, 0), Dot::new(123, 456)].into_iter().collect();
        let out = g.missing_changesets_tolerant(&heads);
        assert!(
            out.is_empty(),
            "known head excludes a; unknown head skipped, no UnknownHeads"
        );

        let all = g.missing_changesets_tolerant(&HashSet::new());
        assert_eq!(all.len(), 1);
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
                .iter()
                .map(|(d, _)| d)
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
            for (start, _) in g.ops.iter() {
                let mut on_path: HashSet<Dot> = HashSet::new();
                prop_assert!(
                    dfs_acyclic(&g, start, &mut on_path, &mut seen),
                    "cycle detected starting from {:?}", start
                );
            }
        }
    }

    /// Forward-walk-only reference: the remote's known set is the ancestry of its
    /// frontier; a changeset is sent iff not fully known (and self-contained). This is
    /// the original, obviously-correct algorithm — the interleaved `missing_changesets_
    /// tolerant` must produce the identical changeset set for every graph and frontier.
    fn forward_reference(g: &OpGraph<u32>, remote_heads: &HashSet<Dot>) -> HashSet<Vec<Dot>> {
        let mut known: HashSet<Dot> = HashSet::new();
        let mut walk: Vec<Dot> = remote_heads.iter().copied().collect();
        while let Some(dot) = walk.pop() {
            if !g.ops.contains_key(&dot) {
                continue;
            }
            if known.insert(dot)
                && let Some(op) = g.ops.get(&dot)
            {
                walk.extend(op.parents.iter().copied());
            }
        }
        let all_self_contained = g.self_contained.len() == g.ops.len();
        let mut out: HashSet<Vec<Dot>> = HashSet::new();
        for r in &g.changesets {
            if r.dots().all(|d| known.contains(&d)) {
                continue;
            }
            if !all_self_contained && r.dots().any(|d| !g.self_contained.contains_key(&d)) {
                continue;
            }
            out.insert(r.dots().collect());
        }
        out
    }

    fn changeset_set(css: &[crate::Changeset<u32>]) -> HashSet<Vec<Dot>> {
        css.iter()
            .map(|cs| cs.ops.iter().map(|o| o.id).collect())
            .collect()
    }

    proptest! {
        /// The interleaved forward/backward frontier race must return exactly the
        /// changesets the forward-only reference does — for arbitrary DAGs and arbitrary
        /// remote frontiers (empty, full, and every partial cut, including non-head ops).
        #[test]
        fn missing_changesets_matches_forward_reference(
            ops in arb_op_sequence(30, 3),
            perm_seed in any::<u64>(),
            heads_mask in any::<u64>(),
        ) {
            let g = apply_all(&causal_permute(&ops, perm_seed));
            let mut dots: Vec<Dot> = g.ops.iter().map(|(d, _)| d).collect();
            dots.sort();
            let remote_heads: HashSet<Dot> = dots
                .iter()
                .enumerate()
                .filter(|&(i, _)| (heads_mask >> (i % 64)) & 1 == 1)
                .map(|(_, d)| *d)
                .collect();

            let got = changeset_set(&g.missing_changesets_tolerant(&remote_heads));
            let expected = forward_reference(&g, &remote_heads);
            prop_assert_eq!(got, expected, "remote_heads={:?}", remote_heads);
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
