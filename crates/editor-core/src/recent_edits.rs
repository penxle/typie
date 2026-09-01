use editor_crdt::{Dot, ListOp, Op};
use editor_macros::ffi;
use editor_model::{EditOp, SeqItem};
use editor_state::State;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use editor_common::time::Instant;

/// Paired with the server's fold bucket, so it is infrastructure rather than policy and
/// stays here. The *window* is policy and comes from the host — see
/// [`RecentEditTracker::enable`].
pub const RECENT_EDIT_BUCKET_MS: i64 = 10 * 60 * 1000;

#[ffi]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecentEditKind {
    Added,
    Modified,
    Deleted,
}

#[ffi]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RecentEditRegion {
    pub page_idx: u32,
    pub y: f32,
    pub height: f32,
    pub kind: RecentEditKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecentEditEffect {
    BlockCreated(Dot),
    BlockEdited(Dot),
    BlockDeleted {
        block: Dot,
        del_op: Dot,
        /// Where the marker should be drawn, resolved here rather than at query time.
        /// See [`DeletedRecord::site`].
        site: Option<Dot>,
    },
    BlockRestored(Dot),
    MoveArrival(Dot),
    MoveErase {
        old: Dot,
    },
}

#[derive(Clone, Copy, Debug)]
struct BlockRecord {
    created_at: Option<i64>,
    last_edit_at: i64,
    /// The newest edit dated by a contribution no undo can take back — a baseline
    /// replay's. Retraction floors `last_edit_at` here rather than dropping the
    /// record, so undoing this session's typing over an already-old block leaves
    /// the old block's own mark standing.
    base_edit_at: Option<i64>,
    /// Live edit-family contributions from tracked ops. The record survives a
    /// retraction while any remain, which is what keeps a mark and its causes in
    /// step under interleaved local and remote edits — a snapshot-and-restore
    /// scheme would let one op's retraction erase another op's mark.
    tracked_edits: u32,
}

/// One tracked op's imprint on the tracker, enough to take that op back exactly.
/// Edit-family effects are counted rather than snapshotted; the two that displace
/// a whole record keep what they displaced, and put it back only where the op's own
/// write is still the one standing — a concurrent deletion of the same block owns
/// its own marker, and taking this op back must not take that one with it.
#[derive(Clone, Copy, Debug)]
enum Contrib {
    Created(Dot),
    Edited(Dot),
    Deleted {
        block: Dot,
        /// The live record this deletion displaced, restorable only while the
        /// document still shows the block — a block a concurrent delete keeps gone
        /// must not come back into the live channel.
        prior_live: Option<BlockRecord>,
        /// The deletion marker this deletion displaced, if it wrote one at all.
        /// A block created inside the window leaves no marker, and then displaces
        /// nothing either.
        prior_deleted: Option<DeletedRecord>,
    },
    Undeleted {
        block: Dot,
        prior: Option<DeletedRecord>,
    },
}

#[derive(Debug)]
struct OpContribs {
    at: i64,
    contribs: Vec<Contrib>,
}

#[derive(Clone, Copy, Debug)]
pub struct DeletedRecord {
    pub at: i64,
    /// The deletion that wrote this record. Identity, not decoration: retracting a
    /// deletion may only drop the marker its own op put here, never one a
    /// concurrent deletion of the same block wrote over it.
    pub del_op: Dot,
    /// The still-shown block the marker hangs off, decided when the deletion was recorded.
    /// `None` means there was nothing to hang it off — the neighbour was gone too — and the
    /// record then costs a query nothing at all.
    ///
    /// Resolving this per query meant walking back over the deletion's own tombstones once
    /// per record on every call, which for a large deletion run is thousands of steps that
    /// almost always end in `None`. Recording it instead pays the walk once.
    ///
    /// Accepted drift: a block inserted between the site and the deletion afterwards is a
    /// nearer neighbour, but the marker stays on the site recorded here. The query only
    /// re-resolves when the site itself is gone, not when a better one appears — this is a
    /// decorative signal, and a stale-by-one-neighbour marker beats a per-frame walk. The
    /// drift runs one step further: once a re-resolve has written `None` here the query
    /// stops asking, so undoing the neighbour's deletion does not bring the marker back
    /// within this session — a reload replays the record and restores it.
    pub site: Option<Dot>,
}

#[derive(Debug, Default)]
pub struct RecentEditTracker {
    enabled: bool,
    base_ms: i64,
    window_ms: i64,
    base_instant: Option<Instant>,
    blocks: HashMap<Dot, BlockRecord>,
    deleted: HashMap<Dot, DeletedRecord>,
    op_contribs: HashMap<Dot, OpContribs>,
    last_prune_bucket: Option<i64>,
}

/// The block an op should be attributed to, or `None` when the op targets the document
/// root itself. The root's layout box spans whole pages, so attributing to it would paint
/// the entire track: a document-settings op (layout mode, base font) is deliberately left
/// unreported rather than reported over everything.
///
/// A leaf whose enclosing block *is* the root is a top-level atom — a rule, an image, a
/// file card — which the projection indexes under the root scope for lack of a block of
/// its own. It has its own layout box, so it stands as its own attribution unit instead of
/// falling through the root guard and vanishing.
fn block_of_dot(state: &State, dot: Dot) -> Option<Dot> {
    let view = state.view();
    let root = view.root().map(|r| r.id());
    if view.node(dot).is_some() {
        return (Some(dot) != root).then_some(dot);
    }
    let block = view.block_of(dot)?;
    Some(if Some(block) == root { dot } else { block })
}

fn is_block_ins(state: &State, dot: Dot) -> bool {
    matches!(
        state.projected.graph().get(&dot).map(|o| &o.payload),
        Some(EditOp::Seq(ListOp::Ins {
            item: SeqItem::Block { .. },
            ..
        }))
    )
}

/// The still-shown block a now-deleted leaf sequence-belongs to. Tombstone-inclusive,
/// so it resolves for leaves the document no longer shows; `None` when the enclosing
/// block itself is gone.
fn enclosing_live_block(state: &State, leaf: Dot) -> Option<Dot> {
    let checkout = state.projected.seq_checkout();
    let (marker, visible) =
        checkout.enclosing_marker(state.projected.seq(), leaf, &|item: &SeqItem| {
            matches!(item, SeqItem::Block { .. })
        })?;
    (visible && state.view().node(marker).is_some()).then_some(marker)
}

/// Where a gone block's marker should be drawn: the still-shown block it sequence-follows.
/// `None` once that neighbour is gone too, which drops the marker rather than floating it.
///
/// Callers record the answer in [`DeletedRecord::site`] instead of asking again per query;
/// the query path only comes back here when a recorded site has since been deleted.
pub(crate) fn deletion_site_block(state: &State, block: Dot) -> Option<Dot> {
    enclosing_live_block(state, block)
}

/// Whether a recorded [`DeletedRecord::site`] is still a block the document shows. One map
/// lookup — the query path's whole check, in place of the walk that produced the site.
pub(crate) fn site_is_live(state: &State, site: Dot) -> bool {
    state.view().node(site).is_some()
}

/// Every block marker's document index, ascending, for a state that will not change while
/// it is held. Baseline replay classifies its whole op set against one finished state, so
/// it pays a single sequence pass here and answers each `enclosing_live_block` by binary
/// search; walking back per query instead re-crosses the tombstones a long delete left,
/// which is quadratic across the ops of that delete run.
///
/// The live tick path does not build this: it classifies one op per keystroke, where a
/// full pass costs more than the walk it replaces.
pub(crate) struct BlockMarkerIndex {
    markers: Vec<(usize, Dot, bool)>,
}

impl BlockMarkerIndex {
    pub(crate) fn build(state: &State) -> Self {
        let checkout = state.projected.seq_checkout();
        let markers = checkout.marker_positions(state.projected.seq(), &|item: &SeqItem| {
            matches!(item, SeqItem::Block { .. })
        });
        Self { markers }
    }

    fn enclosing_live_block(&self, state: &State, leaf: Dot) -> Option<Dot> {
        let doc_idx = state.projected.seq_checkout().doc_index_of(leaf)?;
        let before = self.markers.partition_point(|(idx, _, _)| *idx < doc_idx);
        let (_, marker, visible) = *self.markers.get(before.checked_sub(1)?)?;
        (visible && state.view().node(marker).is_some()).then_some(marker)
    }
}

fn enclosing_live_block_via(
    state: &State,
    leaf: Dot,
    markers: Option<&BlockMarkerIndex>,
) -> Option<Dot> {
    match markers {
        Some(index) => index.enclosing_live_block(state, leaf),
        None => enclosing_live_block(state, leaf),
    }
}

fn effect_sort_key(effect: &RecentEditEffect) -> (u8, Dot, Dot) {
    match *effect {
        RecentEditEffect::BlockCreated(b) => (0, b, Dot::ROOT),
        RecentEditEffect::BlockEdited(b) => (1, b, Dot::ROOT),
        RecentEditEffect::BlockDeleted { block, del_op, .. } => (2, block, del_op),
        RecentEditEffect::BlockRestored(b) => (3, b, Dot::ROOT),
        RecentEditEffect::MoveArrival(b) => (4, b, Dot::ROOT),
        RecentEditEffect::MoveErase { old } => (5, old, Dot::ROOT),
    }
}

pub fn classify_op(state: &State, op: &Op<EditOp>) -> Vec<RecentEditEffect> {
    classify_op_with(state, op, None)
}

pub(crate) fn classify_op_with(
    state: &State,
    op: &Op<EditOp>,
    markers: Option<&BlockMarkerIndex>,
) -> Vec<RecentEditEffect> {
    let mut out = Vec::new();
    match &op.payload {
        EditOp::Seq(ListOp::Ins {
            item: SeqItem::Block { .. },
            ..
        }) => {
            out.push(RecentEditEffect::BlockCreated(op.id));
        }
        EditOp::Seq(ListOp::Ins { .. }) => {
            if let Some(b) = block_of_dot(state, op.id) {
                // A top-level atom attributes to itself, so inserting one is the arrival of
                // a whole unit rather than an edit inside one that was already there.
                if b == op.id {
                    out.push(RecentEditEffect::BlockCreated(b));
                } else {
                    out.push(RecentEditEffect::BlockEdited(b));
                }
            }
        }
        EditOp::Seq(ListOp::Del { .. }) => {
            let targets = {
                let checkout = state.projected.seq_checkout();
                checkout.del_target_dots(state.projected.seq(), op.id)
            };
            // One delete's targets are the visible run it swallowed, in document order.
            // Any block marker inside that run is now a tombstone, so every leaf target
            // past the first such marker walks back into it and resolves to nothing —
            // only the run's outermost leaves can still reach a live enclosing block.
            // Walking back from all of them instead is quadratic: a several-thousand
            // character delete re-walks its own tombstones on every target.
            let mut first_leaf = None;
            let mut last_leaf = None;
            for target in targets {
                if is_block_ins(state, target) {
                    // Resolved once, here, and carried into the record: see
                    // `DeletedRecord::site`. Each block target walks back only as far as the
                    // previous block marker, so the whole loop stays linear in the targets
                    // it already visits.
                    out.push(RecentEditEffect::BlockDeleted {
                        block: target,
                        del_op: op.id,
                        site: enclosing_live_block_via(state, target, markers),
                    });
                } else {
                    first_leaf.get_or_insert(target);
                    last_leaf = Some(target);
                }
            }
            let ends = [first_leaf, last_leaf.filter(|l| Some(*l) != first_leaf)];
            for leaf in ends.into_iter().flatten() {
                if let Some(b) = enclosing_live_block_via(state, leaf, markers) {
                    out.push(RecentEditEffect::BlockEdited(b));
                }
            }
        }
        EditOp::Seq(ListOp::Undel { del }) => {
            let targets = {
                let checkout = state.projected.seq_checkout();
                checkout.del_target_dots(state.projected.seq(), *del)
            };
            for target in targets {
                if let Some(b) = block_of_dot(state, target) {
                    if b == target {
                        out.push(RecentEditEffect::BlockRestored(target));
                    } else {
                        out.push(RecentEditEffect::BlockEdited(b));
                    }
                }
            }
        }
        EditOp::Span(sop) => {
            let (start, end) = sop.anchors();
            let bounds = {
                let checkout = state.projected.seq_checkout();
                let pos_of = |dot: Dot| {
                    checkout
                        .resolve_boundary_checked(dot, editor_crdt::sequence::Bias::Before)
                        .map(|b| b.position)
                };
                pos_of(start.id).zip(pos_of(end.id))
            };
            match bounds {
                Some((s, e)) => {
                    let (lo, hi) = if s <= e { (s, e) } else { (e, s) };
                    let mut last = None;
                    for pos in lo..=hi {
                        let dot = {
                            let checkout = state.projected.seq_checkout();
                            checkout.dot_at_visible(state.projected.seq(), pos)
                        };
                        let Some(d) = dot else { continue };
                        let Some(b) = block_of_dot(state, d) else {
                            continue;
                        };
                        if last != Some(b) {
                            out.push(RecentEditEffect::BlockEdited(b));
                            last = Some(b);
                        }
                    }
                }
                None => {
                    for anchor in [start, end] {
                        if let Some(b) = block_of_dot(state, anchor.id) {
                            out.push(RecentEditEffect::BlockEdited(b));
                        }
                    }
                }
            }
        }
        EditOp::NodeAttr(a) => {
            if let Some(b) = block_of_dot(state, a.target) {
                out.push(RecentEditEffect::BlockEdited(b));
            }
        }
        EditOp::BlockModifier(m) | EditOp::NodeCarry(m) => {
            if let Some(b) = block_of_dot(state, m.target_key().0) {
                out.push(RecentEditEffect::BlockEdited(b));
            }
        }
        EditOp::Alias(a) => {
            for run in &a.pairs {
                for i in 0..run.len as u64 {
                    let old = Dot::new(run.old_start.actor, run.old_start.clock + i);
                    let new = Dot::new(run.new_start.actor, run.new_start.clock + i);
                    if is_block_ins(state, old) {
                        out.push(RecentEditEffect::MoveErase { old });
                    }
                    if let Some(b) = block_of_dot(state, new) {
                        out.push(RecentEditEffect::MoveArrival(b));
                    }
                }
            }
        }
        EditOp::Unknown { .. } => {}
    }
    out.sort_by_key(effect_sort_key);
    out.dedup();
    out
}

fn bucket_of(ms: i64) -> i64 {
    ms.div_euclid(RECENT_EDIT_BUCKET_MS) * RECENT_EDIT_BUCKET_MS
}

/// Put back the record a deletion displaced, folding it into whatever has been
/// recorded for the block since instead of overwriting it. An edit landing between
/// the deletion and its undo — or in the very same tick — is a contribution of its
/// own, and a plain `insert` of the snapshot would take that contribution's mark
/// away with the deletion's.
fn merge_restored(blocks: &mut HashMap<Dot, BlockRecord>, block: Dot, prior: BlockRecord) {
    let Some(current) = blocks.get_mut(&block) else {
        blocks.insert(block, prior);
        return;
    };
    current.created_at = current.created_at.or(prior.created_at);
    current.last_edit_at = current.last_edit_at.max(prior.last_edit_at);
    current.base_edit_at = current.base_edit_at.max(prior.base_edit_at);
    current.tracked_edits += prior.tracked_edits;
}

impl RecentEditTracker {
    /// Starts tracking, taking the recency window from the host so the mark colours and
    /// the query that seeds the baseline can never disagree about how far back "recent"
    /// reaches. A window shorter than one bucket cannot be represented by the bucketed
    /// timestamps, so it is raised to a bucket.
    pub fn enable(&mut self, now_ms: i64, window_ms: i64) {
        self.enabled = true;
        self.base_ms = now_ms;
        self.window_ms = window_ms.max(RECENT_EDIT_BUCKET_MS);
        self.base_instant = Some(Instant::now());
        // A re-injected base can move the clock backwards, and a bucket remembered
        // against the old one would then park the self-prune forever.
        self.last_prune_bucket = None;
    }

    pub fn window_ms(&self) -> i64 {
        self.window_ms
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn current_ms(&self) -> i64 {
        match self.base_instant {
            Some(base) => self.base_ms + base.elapsed().as_millis() as i64,
            None => self.base_ms,
        }
    }

    /// Record effects that no undo can ever take back — a baseline replay's. Their
    /// dating survives every retraction, as [`BlockRecord::base_edit_at`]. Named for
    /// what it means rather than left as the general `record`, so a live-edit path
    /// cannot opt out of retraction by reaching for the shorter name.
    pub(crate) fn record_baseline(&mut self, effects: &[RecentEditEffect], at_ms: i64) {
        self.record_with(None, effects, at_ms);
    }

    /// Record effects attributed to `op`, so that undoing `op` can take exactly
    /// these effects back through [`Self::retract`].
    pub(crate) fn record_tracked(&mut self, op: Dot, effects: &[RecentEditEffect], at_ms: i64) {
        self.record_with(Some(op), effects, at_ms);
    }

    fn record_with(&mut self, op: Option<Dot>, effects: &[RecentEditEffect], at_ms: i64) {
        let at = bucket_of(at_ms);
        // `Vec::new` does not allocate until it is pushed into, so the untracked
        // baseline replay — one call per op of a cold load — pays nothing for a list
        // only the tracked path reads.
        let tracked = op.is_some();
        let mut contribs = Vec::new();
        for effect in effects {
            match *effect {
                RecentEditEffect::BlockCreated(b) => {
                    // The displaced deletion record is not kept: `b` is the insertion
                    // op's own id, so a block cannot be created into an id the
                    // deleted channel already holds.
                    self.deleted.remove(&b);
                    self.blocks.insert(
                        b,
                        BlockRecord {
                            created_at: Some(at),
                            last_edit_at: at,
                            base_edit_at: op.is_none().then_some(at),
                            tracked_edits: 0,
                        },
                    );
                    if tracked {
                        contribs.push(Contrib::Created(b));
                    }
                }
                RecentEditEffect::BlockEdited(b) => {
                    self.record_edit(op, b, at);
                    if tracked {
                        contribs.push(Contrib::Edited(b));
                    }
                }
                RecentEditEffect::BlockDeleted {
                    block,
                    del_op,
                    site,
                } => {
                    let prior_live = self.blocks.remove(&block);
                    let created_in_window = prior_live
                        .is_some_and(|r| r.created_at.is_some_and(|c| at - c < self.window_ms));
                    let prior_deleted = (!created_in_window)
                        .then(|| {
                            self.deleted
                                .insert(block, DeletedRecord { at, del_op, site })
                        })
                        .flatten();
                    if tracked {
                        contribs.push(Contrib::Deleted {
                            block,
                            prior_live,
                            prior_deleted,
                        });
                    }
                }
                RecentEditEffect::BlockRestored(b) => {
                    let prior = self.deleted.remove(&b);
                    self.record_edit(op, b, at);
                    if tracked {
                        contribs.push(Contrib::Undeleted { block: b, prior });
                        contribs.push(Contrib::Edited(b));
                    }
                }
                RecentEditEffect::MoveArrival(b) => {
                    self.record_edit(op, b, at).created_at = None;
                    if tracked {
                        contribs.push(Contrib::Edited(b));
                    }
                }
                RecentEditEffect::MoveErase { old } => {
                    let prior = self.deleted.remove(&old);
                    if tracked {
                        contribs.push(Contrib::Undeleted { block: old, prior });
                    }
                }
            }
        }
        if let Some(op) = op
            && !contribs.is_empty()
        {
            self.op_contribs.insert(op, OpContribs { at, contribs });
        }
    }

    fn record_edit(&mut self, op: Option<Dot>, block: Dot, at: i64) -> &mut BlockRecord {
        let entry = self.blocks.entry(block).or_insert(BlockRecord {
            created_at: None,
            last_edit_at: at,
            base_edit_at: None,
            tracked_edits: 0,
        });
        entry.last_edit_at = entry.last_edit_at.max(at);
        match op {
            Some(_) => entry.tracked_edits += 1,
            None => entry.base_edit_at = Some(entry.base_edit_at.unwrap_or(at).max(at)),
        }
        entry
    }

    /// Take back what the given ops contributed, so that what is left is what the
    /// contributions still standing justify. Not a full rewind: a surviving
    /// contribution's `last_edit_at` is left where it is (see [`Self::retract_edit`]),
    /// and a move arrival's downgrade of `created_at` is not undone.
    ///
    /// `ops` are unwound in the order given, which is the order they must come apart
    /// in — newest first. A deletion has to be taken back before the edits it
    /// swallowed, or the record it restores carries counts for contributions that
    /// are already gone.
    pub(crate) fn retract(&mut self, state: &State, ops: &[Dot]) {
        for op in ops {
            let Some(entry) = self.op_contribs.remove(op) else {
                continue;
            };
            for contrib in entry.contribs.iter().rev() {
                match *contrib {
                    Contrib::Created(b) => {
                        self.blocks.remove(&b);
                    }
                    Contrib::Edited(b) => self.retract_edit(b),
                    Contrib::Deleted {
                        block,
                        prior_live,
                        prior_deleted,
                    } => {
                        if self.deleted.get(&block).is_some_and(|r| r.del_op == *op) {
                            match prior_deleted {
                                Some(displaced) => self.deleted.insert(block, displaced),
                                None => self.deleted.remove(&block),
                            };
                        }
                        if let Some(prior) = prior_live
                            && state.view().node(block).is_some()
                        {
                            merge_restored(&mut self.blocks, block, prior);
                        }
                    }
                    Contrib::Undeleted { block, prior } => {
                        if let Some(record) = prior
                            && !self.deleted.contains_key(&block)
                        {
                            self.deleted.insert(block, record);
                        }
                    }
                }
            }
        }
    }

    /// Drop one edit-family contribution. `last_edit_at` only ever falls back to a
    /// floor the surviving contributions justify, never below one — a block whose
    /// other edits are still standing keeps their (newer) dating, which delays its
    /// expiry rather than risking an early one.
    fn retract_edit(&mut self, block: Dot) {
        let Some(record) = self.blocks.get_mut(&block) else {
            return;
        };
        record.tracked_edits = record.tracked_edits.saturating_sub(1);
        if record.tracked_edits > 0 {
            return;
        }
        match record
            .base_edit_at
            .into_iter()
            .chain(record.created_at)
            .max()
        {
            Some(floor) => record.last_edit_at = floor,
            None => {
                self.blocks.remove(&block);
            }
        }
    }

    /// Prune on the tracker's own clock rather than the query path's, once per
    /// bucket. [`Self::prune`] otherwise runs only when the host asks for regions,
    /// which it stops doing when the marks are switched off — and `op_contribs`,
    /// keyed by op rather than by block, then grows for the life of the session.
    pub(crate) fn prune_if_bucket_advanced(&mut self, now_ms: i64) {
        let bucket = bucket_of(now_ms);
        if self.last_prune_bucket.is_some_and(|last| last >= bucket) {
            return;
        }
        self.last_prune_bucket = Some(bucket);
        self.prune(now_ms);
    }

    pub fn prune(&mut self, now_ms: i64) {
        let cutoff = bucket_of(now_ms) - self.window_ms;
        self.blocks.retain(|_, r| r.last_edit_at >= cutoff);
        self.deleted.retain(|_, r| r.at >= cutoff);

        let mut expired = Vec::new();
        self.op_contribs.retain(|_, e| {
            if e.at >= cutoff {
                return true;
            }
            expired.extend(e.contribs.iter().copied());
            false
        });
        // An expired op can no longer be retracted, so its claim on the blocks it
        // dated has to go with it. A count left standing for it would outlive the
        // retraction of every contribution that still can be taken back, and the
        // record would then survive with nothing justifying it — the presence
        // judgement itself, which the spec does not let drift, unlike the dating.
        for contrib in expired {
            if let Contrib::Edited(block) = contrib
                && let Some(record) = self.blocks.get_mut(&block)
            {
                record.tracked_edits = record.tracked_edits.saturating_sub(1);
            }
        }
    }

    /// Blocks the document still holds. Never yields [`RecentEditKind::Deleted`] —
    /// deletions leave through [`Self::deleted_blocks`], so a caller that wants every
    /// recent edit has to read both channels.
    pub fn live_blocks(&self, now_ms: i64) -> Vec<(Dot, RecentEditKind)> {
        let cutoff = bucket_of(now_ms) - self.window_ms;
        let mut out: Vec<(Dot, RecentEditKind)> = self
            .blocks
            .iter()
            .filter(|(_, r)| r.last_edit_at >= cutoff)
            .map(|(d, r)| {
                let kind = match r.created_at {
                    Some(c) if c > cutoff => RecentEditKind::Added,
                    _ => RecentEditKind::Modified,
                };
                (*d, kind)
            })
            .collect();
        out.sort_by_key(|(d, _)| *d);
        out
    }

    pub fn deleted_blocks(&self, now_ms: i64) -> Vec<(Dot, DeletedRecord)> {
        let cutoff = bucket_of(now_ms) - self.window_ms;
        let mut out: Vec<(Dot, DeletedRecord)> = self
            .deleted
            .iter()
            .filter(|(_, r)| r.at >= cutoff)
            .map(|(d, r)| (*d, *r))
            .collect();
        out.sort_by_key(|(d, _)| *d);
        out
    }

    /// Replace a deletion's recorded site after the query path found the old one gone.
    pub(crate) fn repair_deleted_site(&mut self, block: Dot, site: Option<Dot>) {
        if let Some(record) = self.deleted.get_mut(&block) {
            record.site = site;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::Editor;
    use crate::message::{
        Break, HistoryOp, InputModifiers, InsertionOp, Key, KeyEvent, Message, ModifierOp,
    };
    use editor_macros::state;
    use editor_model::{
        AliasOp, AliasRun, Anchor, AtomLeaf, Bias, CalloutNodeAttr, CalloutVariant, ChildView,
        HorizontalRuleVariant, LayoutMode, Modifier, ModifierAttrOp, ModifierType, NodeAttr,
        NodeAttrOp, NodeType, NodeView, RootNodeAttr, SpanOp,
    };
    use editor_transaction::Transaction;
    use hashbrown::HashSet;
    use proptest::prelude::*;

    fn apply(state: &State, payload: EditOp) -> (State, Op<EditOp>) {
        state.apply(payload).unwrap()
    }

    /// The window the website injects. The core takes it from the host now, so tests pin
    /// it explicitly instead of reading a core constant.
    const WINDOW_MS: i64 = 24 * 60 * 60 * 1000;

    /// Ready to receive a baseline: the tracker refuses to date anything before the host
    /// has told it how far back "recent" reaches.
    fn editor_for_state(state: State) -> Editor {
        let mut editor = Editor::new_test(state);
        editor.enable_recent_edits(0, WINDOW_MS);
        editor
    }

    fn heads_of(state: &State) -> Vec<Dot> {
        state.projected.graph().current_heads().copied().collect()
    }

    fn live_block_set(state: &State) -> HashSet<Dot> {
        fn walk(node: &NodeView<'_>, out: &mut HashSet<Dot>) {
            for child in node.child_blocks() {
                out.insert(child.id());
                walk(&child, out);
            }
        }

        let view = state.view();
        let mut out = HashSet::new();
        if let Some(root) = view.root() {
            out.insert(root.id());
            walk(&root, &mut out);
        }
        out
    }

    fn live_blocks_of_type(state: &State, node_type: NodeType) -> Vec<Dot> {
        fn walk(node: &NodeView<'_>, node_type: NodeType, out: &mut Vec<Dot>) {
            for child in node.child_blocks() {
                if child.node_type() == node_type {
                    out.push(child.id());
                }
                walk(&child, node_type, out);
            }
        }

        let view = state.view();
        let mut out = Vec::new();
        if let Some(root) = view.root() {
            walk(&root, node_type, &mut out);
        }
        out
    }

    fn visible_len(state: &State) -> usize {
        state.projected.seq_checkout().visible_len()
    }

    #[test]
    fn char_ins_into_existing_block_is_block_edited() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let (next, op) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 3,
                item: SeqItem::Char('c'),
            }),
        );
        let effects = classify_op(&next, &op);
        assert_eq!(effects, vec![RecentEditEffect::BlockEdited(p1)]);
    }

    #[test]
    fn block_ins_is_block_created() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let (next, op) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 3,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }),
        );
        let effects = classify_op(&next, &op);
        assert_eq!(effects, vec![RecentEditEffect::BlockCreated(op.id)]);
    }

    #[test]
    fn char_del_marks_its_enclosing_block_edited() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let (next, op) = apply(&state, EditOp::Seq(ListOp::Del { pos: 2, len: 1 }));
        let effects = classify_op(&next, &op);
        assert_eq!(effects, vec![RecentEditEffect::BlockEdited(p1)]);
    }

    #[test]
    fn block_del_is_block_deleted() {
        let (state, p1, _p2) = state! {
            doc { root { p1: paragraph { text("a") } p2: paragraph { text("b") } } }
            selection: none
        };
        let (next, op) = apply(&state, EditOp::Seq(ListOp::Del { pos: 0, len: 2 }));
        let effects = classify_op(&next, &op);
        assert_eq!(
            effects,
            vec![RecentEditEffect::BlockDeleted {
                block: p1,
                del_op: op.id,
                // nothing precedes `p1`, so the marker has nowhere to hang
                site: None,
            }]
        );
    }

    #[test]
    fn multi_block_del_edits_the_survivor_and_deletes_every_engulfed_block() {
        let (state, a, b, c) = state! {
            doc {
                root {
                    a: paragraph { text("aaa") }
                    b: paragraph { text("bbb") }
                    c: paragraph { text("ccc") }
                }
            }
            selection: none
        };
        // Visible run [a3, b, b1, b2, b3, c, c1]: the tail of `a`, all of `b`, and the
        // head of `c` — which engulfs `c`'s own marker, so `c` is deleted, not edited.
        let (next, op) = apply(&state, EditOp::Seq(ListOp::Del { pos: 3, len: 7 }));
        let effects = classify_op(&next, &op);

        assert_eq!(effects.len(), 3, "got {effects:?}");
        assert!(effects.contains(&RecentEditEffect::BlockEdited(a)));
        assert!(effects.contains(&RecentEditEffect::BlockDeleted {
            block: b,
            del_op: op.id,
            // first block of the run: its neighbour `a` survived
            site: Some(a),
        }));
        assert!(effects.contains(&RecentEditEffect::BlockDeleted {
            block: c,
            del_op: op.id,
            // deeper in the run: the only thing before it is `b`'s tombstone
            site: None,
        }));
    }

    #[test]
    fn long_intra_block_del_edits_that_block_once() {
        let (state, p) = state! {
            doc { root { p: paragraph { text("abcdefghij") } } }
            selection: none
        };
        let (next, op) = apply(&state, EditOp::Seq(ListOp::Del { pos: 2, len: 6 }));
        let effects = classify_op(&next, &op);
        assert_eq!(effects, vec![RecentEditEffect::BlockEdited(p)]);
    }

    #[test]
    fn span_marks_covered_block_edited_across_a_tombstone() {
        let (state, p1, _p2) = state! {
            doc { root { p1: paragraph { text("ab") } p2: paragraph { text("cd") } } }
            selection: none
        };
        let (state, _) = apply(&state, EditOp::Seq(ListOp::Del { pos: 1, len: 1 }));

        let leaf = {
            let view = state.view();
            match view.node(p1).unwrap().child_at(0).unwrap() {
                ChildView::Leaf(leaf) => leaf.dot(),
                ChildView::Block(_) => panic!("paragraph child is a leaf"),
            }
        };

        let (next, op) = apply(
            &state,
            EditOp::Span(SpanOp::AddSpan {
                start: Anchor {
                    id: leaf,
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: leaf,
                    bias: Bias::After,
                },
                modifier: Modifier::Bold,
            }),
        );
        let effects = classify_op(&next, &op);
        assert_eq!(effects, vec![RecentEditEffect::BlockEdited(p1)]);
    }

    #[test]
    fn undel_of_block_delete_restores_block() {
        let (state, p1, _p2) = state! {
            doc { root { p1: paragraph {} p2: paragraph { text("b") } } }
            selection: none
        };
        let (state, del) = apply(&state, EditOp::Seq(ListOp::Del { pos: 0, len: 1 }));
        let (next, op) = apply(&state, EditOp::Seq(ListOp::Undel { del: del.id }));
        let effects = classify_op(&next, &op);
        assert_eq!(effects, vec![RecentEditEffect::BlockRestored(p1)]);
    }

    #[test]
    fn undel_of_leaf_delete_marks_block_edited() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let (state, del) = apply(&state, EditOp::Seq(ListOp::Del { pos: 2, len: 1 }));
        let (next, op) = apply(&state, EditOp::Seq(ListOp::Undel { del: del.id }));
        let effects = classify_op(&next, &op);
        assert_eq!(effects, vec![RecentEditEffect::BlockEdited(p1)]);
    }

    #[test]
    fn node_attr_marks_target_block_edited() {
        let (state, c, _p1) = state! {
            doc { root { c: callout { p1: paragraph { text("a") } } } }
            selection: none
        };
        let (next, op) = apply(
            &state,
            EditOp::NodeAttr(NodeAttrOp {
                target: c,
                attr: NodeAttr::Callout {
                    attr: CalloutNodeAttr::Variant(CalloutVariant::Warning),
                },
            }),
        );
        let effects = classify_op(&next, &op);
        assert_eq!(effects, vec![RecentEditEffect::BlockEdited(c)]);
    }

    #[test]
    fn block_modifier_on_block_target_marks_it_edited() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("a") } } }
            selection: none
        };
        let (next, op) = apply(
            &state,
            EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                target: p1,
                modifier: Modifier::FontSize { value: 1600 },
            }),
        );
        let effects = classify_op(&next, &op);
        assert_eq!(effects, vec![RecentEditEffect::BlockEdited(p1)]);
    }

    #[test]
    fn node_carry_on_leaf_target_attributes_to_its_block() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let leaf = {
            let view = state.view();
            match view.node(p1).unwrap().child_at(0).unwrap() {
                ChildView::Leaf(leaf) => leaf.dot(),
                ChildView::Block(_) => panic!("paragraph child is a leaf"),
            }
        };
        assert_ne!(leaf, p1);

        let (next, op) = apply(
            &state,
            EditOp::NodeCarry(ModifierAttrOp::ClearModifier {
                target: leaf,
                key: ModifierType::Bold,
            }),
        );
        let effects = classify_op(&next, &op);
        assert_eq!(effects, vec![RecentEditEffect::BlockEdited(p1)]);
    }

    #[test]
    fn alias_marks_arrival_moved_and_erases_departure() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let (state, ins) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 3,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }),
        );
        let (next, op) = apply(
            &state,
            EditOp::Alias(AliasOp {
                pairs: vec![AliasRun {
                    old_start: p1,
                    len: 1,
                    new_start: ins.id,
                }],
            }),
        );
        let effects = classify_op(&next, &op);
        assert_eq!(
            effects,
            vec![
                RecentEditEffect::MoveArrival(ins.id),
                RecentEditEffect::MoveErase { old: p1 },
            ]
        );
    }

    #[test]
    fn alias_run_expansion_covers_inner_blocks_of_a_moved_subtree() {
        let (state, c, p1, p2) = state! {
            doc {
                root {
                    c: callout {
                        p1: paragraph { text("a") }
                        p2: paragraph { text("b") }
                    }
                    paragraph { text("z") }
                }
            }
            selection: none
        };
        let view_before = state.view();
        let root = view_before.root().unwrap().id();
        drop(view_before);

        let mut tr = Transaction::new(&state);
        tr.move_node(c, root, 1).unwrap();
        let (after, _steps, recorded, _effects, _meta) = tr.commit();

        let alias = recorded
            .iter()
            .find(|r| matches!(r.op.payload, EditOp::Alias(_)))
            .expect("move_node emits an alias op");

        let EditOp::Alias(alias_op) = &alias.op.payload else {
            unreachable!()
        };
        assert_eq!(
            alias_op.pairs.len(),
            1,
            "전제: 압축이 서브트리 전체를 한 런으로 묶는다"
        );
        assert_eq!(
            alias_op.pairs[0].len, 5,
            "전제: 그 런이 블록 3개와 문자 2개를 함께 덮는다 (블록 경계를 넘는다)"
        );

        let effects = classify_op(&after, &alias.op);

        let mut erased: Vec<Dot> = effects
            .iter()
            .filter_map(|e| match e {
                RecentEditEffect::MoveErase { old } => Some(*old),
                _ => None,
            })
            .collect();
        erased.sort();
        let mut expected_erased = vec![c, p1, p2];
        expected_erased.sort();
        assert_eq!(
            erased, expected_erased,
            "이동한 서브트리의 블록 전부가 출발지 삭제 계상에서 빠져야 한다"
        );

        let moved_blocks = {
            let view = after.view();
            let callout = view
                .root()
                .unwrap()
                .child_blocks()
                .find(|n| n.node_type() == NodeType::Callout)
                .expect("이동한 callout");
            let mut out = vec![callout.id()];
            out.extend(callout.child_blocks().map(|n| n.id()));
            out.sort();
            out
        };
        let mut arrived: Vec<Dot> = effects
            .iter()
            .filter_map(|e| match e {
                RecentEditEffect::MoveArrival(b) => Some(*b),
                _ => None,
            })
            .collect();
        arrived.sort();
        assert_eq!(
            arrived, moved_blocks,
            "도착지의 블록 전부가 Added가 아니라 이동 도착으로 계상돼야 한다"
        );
    }

    #[test]
    fn created_then_deleted_in_window_leaves_no_record() {
        let mut t = RecentEditTracker::default();
        t.enable(1_000_000, WINDOW_MS);
        let b = Dot::new(1, 10);
        let del = Dot::new(1, 20);
        t.record_baseline(&[RecentEditEffect::BlockCreated(b)], 1_000_000);
        t.record_baseline(
            &[RecentEditEffect::BlockDeleted {
                block: b,
                del_op: del,
                site: None,
            }],
            1_000_500,
        );
        assert!(t.live_blocks(1_001_000).is_empty());
        assert!(t.deleted_blocks(1_001_000).is_empty());
    }

    #[test]
    fn old_block_deletion_is_recorded() {
        let mut t = RecentEditTracker::default();
        t.enable(1_000_000, WINDOW_MS);
        let b = Dot::new(1, 10);
        let del = Dot::new(1, 20);
        t.record_baseline(
            &[RecentEditEffect::BlockDeleted {
                block: b,
                del_op: del,
                site: None,
            }],
            1_000_000,
        );
        assert_eq!(t.deleted_blocks(1_001_000).len(), 1);
    }

    #[test]
    fn creation_in_window_reports_added() {
        let mut t = RecentEditTracker::default();
        t.enable(1_000_000, WINDOW_MS);
        let b = Dot::new(1, 10);
        t.record_baseline(&[RecentEditEffect::BlockCreated(b)], 1_000_000);
        assert_eq!(t.live_blocks(1_001_000), vec![(b, RecentEditKind::Added)]);
    }

    #[test]
    fn edits_within_the_window_keep_a_new_block_added() {
        let mut t = RecentEditTracker::default();
        t.enable(0, WINDOW_MS);
        let b = Dot::new(1, 10);
        t.record_baseline(&[RecentEditEffect::BlockCreated(b)], 0);
        t.record_baseline(&[RecentEditEffect::BlockEdited(b)], RECENT_EDIT_BUCKET_MS);
        let now = WINDOW_MS - RECENT_EDIT_BUCKET_MS;
        t.record_baseline(&[RecentEditEffect::BlockEdited(b)], now);
        assert_eq!(t.live_blocks(now), vec![(b, RecentEditKind::Added)]);
    }

    #[test]
    fn creation_ages_into_modified_as_window_slides() {
        let mut t = RecentEditTracker::default();
        t.enable(0, WINDOW_MS);
        let b = Dot::new(1, 10);
        t.record_baseline(&[RecentEditEffect::BlockCreated(b)], 0);
        t.record_baseline(&[RecentEditEffect::BlockEdited(b)], WINDOW_MS - 1000);
        let now = WINDOW_MS + 1000;
        let live = t.live_blocks(now);
        assert_eq!(live, vec![(b, RecentEditKind::Modified)]);
    }

    #[test]
    fn expiry_drops_records_outside_window() {
        let mut t = RecentEditTracker::default();
        t.enable(0, WINDOW_MS);
        let b = Dot::new(1, 10);
        t.record_baseline(&[RecentEditEffect::BlockEdited(b)], 0);
        assert!(t.live_blocks(WINDOW_MS + RECENT_EDIT_BUCKET_MS).is_empty());
    }

    #[test]
    fn prune_drops_out_of_window_records_from_both_channels() {
        let window = 3 * RECENT_EDIT_BUCKET_MS;
        let mut t = RecentEditTracker::default();
        t.enable(0, window);

        let stale_live = Dot::new(1, 1);
        let stale_gone = Dot::new(1, 2);
        let fresh_live = Dot::new(1, 3);
        let fresh_gone = Dot::new(1, 4);
        t.record_baseline(&[RecentEditEffect::BlockEdited(stale_live)], 0);
        t.record_baseline(
            &[RecentEditEffect::BlockDeleted {
                block: stale_gone,
                del_op: Dot::new(9, 1),
                site: None,
            }],
            0,
        );
        let now = 4 * RECENT_EDIT_BUCKET_MS;
        t.record_baseline(&[RecentEditEffect::BlockEdited(fresh_live)], now);
        t.record_baseline(
            &[RecentEditEffect::BlockDeleted {
                block: fresh_gone,
                del_op: Dot::new(9, 2),
                site: None,
            }],
            now,
        );

        t.prune(now);

        assert_eq!(
            t.live_blocks(0),
            vec![(fresh_live, RecentEditKind::Modified)],
            "queried at a `now` whose own cutoff would still admit the stale record, so \
             what is left is what `prune` kept, not what the query filtered"
        );
        assert_eq!(
            t.deleted_blocks(0)
                .into_iter()
                .map(|(d, _)| d)
                .collect::<Vec<_>>(),
            vec![fresh_gone],
            "the deleted channel is pruned on the same cutoff, not only the live one"
        );
    }

    #[test]
    fn move_arrival_downgrades_created_to_modified_and_erases_deletion() {
        let mut t = RecentEditTracker::default();
        t.enable(1_000_000, WINDOW_MS);
        let old = Dot::new(1, 10);
        let new = Dot::new(1, 30);
        let del = Dot::new(1, 20);
        t.record_baseline(
            &[
                RecentEditEffect::BlockDeleted {
                    block: old,
                    del_op: del,
                    site: None,
                },
                RecentEditEffect::BlockCreated(new),
                RecentEditEffect::MoveErase { old },
                RecentEditEffect::MoveArrival(new),
            ],
            1_000_000,
        );
        assert!(t.deleted_blocks(1_001_000).is_empty());
        assert_eq!(
            t.live_blocks(1_001_000),
            vec![(new, RecentEditKind::Modified)]
        );
    }

    #[test]
    fn move_erase_clears_the_deletion_record_but_leaves_a_live_mark_standing() {
        let mut t = RecentEditTracker::default();
        t.enable(1_000_000, WINDOW_MS);
        let old = Dot::new(1, 10);
        t.record_baseline(&[RecentEditEffect::BlockEdited(old)], 1_000_000);
        t.record_baseline(
            &[RecentEditEffect::BlockDeleted {
                block: old,
                del_op: Dot::new(1, 20),
                site: None,
            }],
            1_000_500,
        );
        t.record_baseline(&[RecentEditEffect::MoveErase { old }], 1_000_500);
        assert!(
            t.deleted_blocks(1_001_000).is_empty(),
            "the departure is not a deletion, so the deleted channel drops it"
        );

        t.record_baseline(&[RecentEditEffect::BlockEdited(old)], 1_000_500);
        t.record_baseline(&[RecentEditEffect::MoveErase { old }], 1_000_500);
        assert_eq!(
            t.live_blocks(1_001_000),
            vec![(old, RecentEditKind::Modified)],
            "`MoveErase` only drops the deletion record; a live mark on the departed block \
             is left for the document to contradict, so the live channel is not a \
             `is still in the document` guarantee"
        );
    }

    #[test]
    fn baseline_restores_past_edits_with_bucket_dating() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let frontier = heads_of(&state);

        let (state, _block) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 3,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }),
        );
        let (state, _ch) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 4,
                item: SeqItem::Char('x'),
            }),
        );

        let mut editor = editor_for_state(state);
        let now = WINDOW_MS;
        let accepted =
            editor.set_recent_edit_baseline(now, vec![(now - 3 * RECENT_EDIT_BUCKET_MS, frontier)]);
        assert_eq!(accepted, 1);

        let regions = editor.recent_edit_regions(now);
        assert!(
            regions.iter().any(|r| r.kind == RecentEditKind::Added),
            "post-baseline block insert must replay as Added, got {regions:?}"
        );
    }

    #[test]
    fn baseline_dates_ops_between_buckets_and_drops_pre_window_ones() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let frontier_before_block = heads_of(&state);

        let (state, block) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 3,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }),
        );
        let frontier_after_block = heads_of(&state);
        let (state, _ch) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 4,
                item: SeqItem::Char('x'),
            }),
        );

        let mut editor = editor_for_state(state);
        let now = 3 * WINDOW_MS;
        let accepted = editor.set_recent_edit_baseline(
            now,
            vec![
                (0, frontier_before_block),
                (WINDOW_MS, frontier_after_block),
            ],
        );
        assert_eq!(accepted, 2);

        let live = editor.recent_edits.live_blocks(now);
        assert_eq!(
            live,
            vec![(block.id, RecentEditKind::Modified)],
            "the creation predates the window, so only the later char insert survives — as Modified"
        );
    }

    #[test]
    fn root_targeted_document_settings_mark_nothing() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let frontier = heads_of(&state);
        let (state, op) = apply(
            &state,
            EditOp::NodeAttr(NodeAttrOp {
                target: Dot::ROOT,
                attr: NodeAttr::Root {
                    attr: RootNodeAttr::LayoutMode(LayoutMode::Continuous { max_width: 600 }),
                },
            }),
        );
        assert_eq!(
            classify_op(&state, &op),
            vec![],
            "the root's layout box spans whole pages, so it must never be attributed to"
        );

        let mut editor = editor_for_state(state);
        let now = WINDOW_MS;
        assert_eq!(editor.set_recent_edit_baseline(now, vec![(0, frontier)]), 1);
        assert!(
            editor.recent_edit_regions(now).is_empty(),
            "a document setting change must not paint the whole track"
        );
    }

    fn horizontal_rule() -> SeqItem {
        SeqItem::BlockAtom {
            leaf: AtomLeaf::HorizontalRule {
                variant: HorizontalRuleVariant::Line,
            },
            parents: vec![Dot::ROOT],
        }
    }

    #[test]
    fn top_level_atom_insert_is_added_and_paints_its_own_box() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let frontier = heads_of(&state);
        let (state, op) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 3,
                item: horizontal_rule(),
            }),
        );
        assert_eq!(
            state.view().block_of(op.id),
            state.view().root().map(|r| r.id()),
            "fixture must produce a root-direct atom"
        );
        assert_eq!(
            classify_op(&state, &op),
            vec![RecentEditEffect::BlockCreated(op.id)]
        );

        let mut editor = editor_for_state(state);
        let now = WINDOW_MS;
        assert_eq!(editor.set_recent_edit_baseline(now, vec![(0, frontier)]), 1);
        let regions = editor.recent_edit_regions(now);
        assert_eq!(
            regions
                .iter()
                .filter(|r| r.kind == RecentEditKind::Added)
                .count(),
            1,
            "the rule is its own attribution unit and lays out as an atom — got {regions:?}"
        );
    }

    #[test]
    fn top_level_atom_attr_change_is_modified() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let (state, ins) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 3,
                item: horizontal_rule(),
            }),
        );
        let frontier = heads_of(&state);
        let (state, op) = apply(
            &state,
            EditOp::NodeAttr(NodeAttrOp {
                target: ins.id,
                attr: NodeAttr::HorizontalRule {
                    attr: editor_model::HorizontalRuleNodeAttr::Variant(
                        HorizontalRuleVariant::Zigzag,
                    ),
                },
            }),
        );
        assert_eq!(
            classify_op(&state, &op),
            vec![RecentEditEffect::BlockEdited(ins.id)]
        );

        let mut editor = editor_for_state(state);
        let now = WINDOW_MS;
        assert_eq!(editor.set_recent_edit_baseline(now, vec![(0, frontier)]), 1);
        let regions = editor.recent_edit_regions(now);
        assert_eq!(
            regions
                .iter()
                .filter(|r| r.kind == RecentEditKind::Modified)
                .count(),
            1,
            "changing the rule's variant marks the rule, not the whole page — got {regions:?}"
        );
    }

    #[test]
    fn top_level_atom_delete_marks_the_block_it_followed() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let (state, ins) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 3,
                item: horizontal_rule(),
            }),
        );
        let frontier = heads_of(&state);
        let (state, op) = apply(&state, EditOp::Seq(ListOp::Del { pos: 3, len: 1 }));
        assert!(
            state.view().block_of(ins.id).is_none(),
            "fixture must actually remove the rule"
        );
        // The rule carries no block marker of its own, so its deletion reads as an edit to
        // the block it sequence-followed rather than as a deleted block of its own.
        assert_eq!(
            classify_op(&state, &op),
            vec![RecentEditEffect::BlockEdited(p1)]
        );

        let mut editor = editor_for_state(state);
        let now = WINDOW_MS;
        assert_eq!(editor.set_recent_edit_baseline(now, vec![(0, frontier)]), 1);
        let regions = editor.recent_edit_regions(now);
        assert!(
            !regions.iter().any(|r| r.kind == RecentEditKind::Deleted),
            "no block marker was removed, so no deletion marker is drawn — got {regions:?}"
        );
        assert_eq!(
            regions
                .iter()
                .filter(|r| r.kind == RecentEditKind::Modified)
                .count(),
            1
        );
    }

    #[test]
    fn deleting_the_first_block_reports_no_deletion_marker() {
        let (state, _p1, _p2) = state! {
            doc { root { p1: paragraph { text("a") } p2: paragraph { text("b") } } }
            selection: none
        };
        let frontier = heads_of(&state);
        let (state, _del) = apply(&state, EditOp::Seq(ListOp::Del { pos: 0, len: 2 }));

        let mut editor = editor_for_state(state);
        let now = WINDOW_MS;
        assert_eq!(editor.set_recent_edit_baseline(now, vec![(0, frontier)]), 1);

        let regions = editor.recent_edit_regions(now);
        assert!(
            !regions.iter().any(|r| r.kind == RecentEditKind::Deleted),
            "nothing precedes the first block, so the marker has no site — got {regions:?}"
        );
    }

    #[test]
    fn consecutive_block_deletions_report_a_single_marker() {
        let (state, _p1, _p2, _p3) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    p2: paragraph { text("b") }
                    p3: paragraph { text("c") }
                }
            }
            selection: none
        };
        let frontier = heads_of(&state);
        let (state, _del) = apply(&state, EditOp::Seq(ListOp::Del { pos: 2, len: 4 }));

        let mut editor = editor_for_state(state);
        let now = WINDOW_MS;
        assert_eq!(editor.set_recent_edit_baseline(now, vec![(0, frontier)]), 1);

        let regions = editor.recent_edit_regions(now);
        let deleted: Vec<_> = regions
            .iter()
            .filter(|r| r.kind == RecentEditKind::Deleted)
            .collect();
        assert_eq!(
            deleted.len(),
            1,
            "two blocks vanished at one site, so the track gets one marker — got {regions:?}"
        );
    }

    #[test]
    fn blocks_inside_a_deletion_run_record_no_site_at_all() {
        let (state, p1, _p2, _p3, _p4) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    p2: paragraph { text("b") }
                    p3: paragraph { text("c") }
                    p4: paragraph { text("d") }
                }
            }
            selection: none
        };
        let frontier = heads_of(&state);
        // From `p2`'s marker through the end: three blocks vanish in one run.
        let (state, _del) = apply(&state, EditOp::Seq(ListOp::Del { pos: 2, len: 6 }));

        let mut editor = editor_for_state(state);
        let now = WINDOW_MS;
        assert_eq!(editor.set_recent_edit_baseline(now, vec![(0, frontier)]), 1);

        let records = editor.recent_edits.deleted_blocks(now);
        assert_eq!(
            records.len(),
            3,
            "three blocks were deleted — got {records:?}"
        );
        let sited: Vec<Dot> = records.iter().filter_map(|(_, r)| r.site).collect();
        assert_eq!(
            sited,
            vec![p1],
            "only the run's first block reaches a surviving neighbour; the rest must record \
             no site at all, so a query never walks for them — got {records:?}"
        );

        let regions = editor.recent_edit_regions(now);
        assert_eq!(
            regions
                .iter()
                .filter(|r| r.kind == RecentEditKind::Deleted)
                .count(),
            1
        );
    }

    #[test]
    fn the_recorded_site_is_what_places_the_marker() {
        let (state, p1, _p2) = state! {
            doc { root { p1: paragraph { text("a") } p2: paragraph { text("b") } } }
            selection: none
        };
        let frontier = heads_of(&state);
        let (state, _del) = apply(&state, EditOp::Seq(ListOp::Del { pos: 2, len: 2 }));

        let mut editor = editor_for_state(state);
        let now = WINDOW_MS;
        assert_eq!(editor.set_recent_edit_baseline(now, vec![(0, frontier)]), 1);

        let records = editor.recent_edits.deleted_blocks(now);
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].1.site,
            Some(p1),
            "the surviving neighbour is decided when the deletion is recorded"
        );

        let first = editor.recent_edit_regions(now);
        let second = editor.recent_edit_regions(now);
        assert_eq!(first, second, "a repeat query must not move the marker");
        assert_eq!(
            editor.recent_edits.deleted_blocks(now)[0].1.site,
            Some(p1),
            "a live site is consumed as-is, never re-resolved"
        );

        let neighbour = editor.view.node_box_rects(&[p1]);
        let deleted: Vec<_> = first
            .iter()
            .filter(|r| r.kind == RecentEditKind::Deleted)
            .collect();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].y, neighbour[0].rect.y + neighbour[0].rect.height);
    }

    #[test]
    fn a_site_deleted_afterwards_is_repaired_on_the_next_query() {
        let (state, p1, _p2, _p3) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    p2: paragraph { text("b") }
                    p3: paragraph { text("c") }
                }
            }
            selection: none
        };
        // Live recording, one op at a time: `p3` goes while `p2` is still shown, so its
        // site is `p2`; the later deletion of `p2` leaves that recorded site stale.
        let (after3, del3) = apply(&state, EditOp::Seq(ListOp::Del { pos: 4, len: 2 }));
        let effects3 = classify_op(&after3, &del3);
        let (after2, del2) = apply(&after3, EditOp::Seq(ListOp::Del { pos: 2, len: 2 }));
        let effects2 = classify_op(&after2, &del2);

        let p3 = match effects3.as_slice() {
            [RecentEditEffect::BlockDeleted { block, site, .. }] => {
                assert!(
                    site.is_some(),
                    "fixture must record a live site to invalidate"
                );
                *block
            }
            other => panic!("expected one block deletion, got {other:?}"),
        };

        let mut editor = editor_for_state(after2);
        let now = WINDOW_MS;
        editor.recent_edits.enable(now, WINDOW_MS);
        editor.recent_edits.record_baseline(&effects3, now);
        editor.recent_edits.record_baseline(&effects2, now);

        let stale = editor
            .recent_edits
            .deleted_blocks(now)
            .into_iter()
            .find(|(b, _)| *b == p3)
            .expect("p3 must be recorded");
        assert!(
            stale
                .1
                .site
                .is_some_and(|s| !site_is_live(&editor.state, s)),
            "the recorded site must be the block that has since been deleted"
        );

        let regions = editor.recent_edit_regions(now);
        assert_eq!(
            regions
                .iter()
                .filter(|r| r.kind == RecentEditKind::Deleted)
                .count(),
            1,
            "`p3`'s recorded site is gone and re-resolving it lands on nothing, so only \
             `p2`'s marker is left — got {regions:?}"
        );

        let repaired = editor
            .recent_edits
            .deleted_blocks(now)
            .into_iter()
            .find(|(b, _)| *b == p3)
            .expect("p3 must still be recorded");
        assert_eq!(
            repaired.1.site, None,
            "the query re-resolved the dead site and wrote the answer back, so the next \
             query walks nothing"
        );
        // The surviving marker is `p2`'s, sitting on `p1`.
        let neighbour = editor.view.node_box_rects(&[p1]);
        let deleted: Vec<_> = regions
            .iter()
            .filter(|r| r.kind == RecentEditKind::Deleted)
            .collect();
        assert_eq!(deleted[0].y, neighbour[0].rect.y + neighbour[0].rect.height);
    }

    #[test]
    fn deleted_block_reports_a_zero_height_region_at_its_surviving_neighbour() {
        let (state, p1, _p2) = state! {
            doc { root { p1: paragraph { text("a") } p2: paragraph { text("b") } } }
            selection: none
        };
        let frontier = heads_of(&state);
        let (state, _del) = apply(&state, EditOp::Seq(ListOp::Del { pos: 2, len: 2 }));

        let mut editor = editor_for_state(state);
        let now = WINDOW_MS;
        assert_eq!(editor.set_recent_edit_baseline(now, vec![(0, frontier)]), 1);

        let regions = editor.recent_edit_regions(now);
        let deleted: Vec<_> = regions
            .iter()
            .filter(|r| r.kind == RecentEditKind::Deleted)
            .collect();
        assert_eq!(deleted.len(), 1, "got {regions:?}");
        assert_eq!(deleted[0].height, 0.0);

        let neighbour = editor.view.node_box_rects(&[p1]);
        assert!(
            neighbour[0].rect.height > 0.0,
            "the neighbour must have height, or bottom and top would be indistinguishable"
        );
        assert_eq!(
            deleted[0].y,
            neighbour[0].rect.y + neighbour[0].rect.height,
            "the marker belongs at the preceding block's bottom edge"
        );
        assert_eq!(deleted[0].page_idx as usize, neighbour[0].page_idx);
    }

    #[test]
    fn baseline_with_an_empty_frontier_is_discarded() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let mut editor = editor_for_state(state);
        assert_eq!(
            editor.set_recent_edit_baseline(WINDOW_MS, vec![(1000, vec![])]),
            0,
            "an empty frontier means `ops_after_frontier` would answer with the whole history"
        );
        assert!(editor.recent_edit_regions(WINDOW_MS).is_empty());
    }

    #[test]
    fn baseline_with_unknown_dot_is_discarded() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let mut editor = editor_for_state(state);
        let bogus = vec![(1000, vec![Dot::new(99, 99)])];
        assert_eq!(editor.set_recent_edit_baseline(WINDOW_MS, bogus), 0);
        assert!(editor.recent_edit_regions(WINDOW_MS).is_empty());
    }

    #[test]
    fn a_baseline_batch_keeps_its_valid_buckets_and_drops_the_rest() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let frontier = heads_of(&state);
        let (state, block) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 3,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }),
        );
        let (state, _ch) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 4,
                item: SeqItem::Char('x'),
            }),
        );

        let mut editor = editor_for_state(state);
        let now = WINDOW_MS;
        let accepted = editor.set_recent_edit_baseline(
            now,
            vec![
                (0, frontier),
                (now / 2, vec![]),
                (now / 2, vec![Dot::new(99, 99)]),
                (now / 2, vec![p1, Dot::new(99, 99)]),
            ],
        );
        assert_eq!(
            accepted, 1,
            "the batch's one usable bucket is kept while the empty, the unknown-dot and the \
             part-unknown ones go"
        );
        assert_eq!(
            editor.recent_edits.live_blocks(now),
            vec![(block.id, RecentEditKind::Added)],
            "only the kept bucket dates the replay: a frontier with a dot this graph cannot \
             resolve subtracts nothing through it, so accepting one would date ops older than \
             its bucket — here the seed edits to {p1:?} — as recent"
        );
    }

    #[test]
    fn one_query_reports_added_modified_and_deleted_at_once() {
        let (state, p1, p2, _p3) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    p2: paragraph { text("b") }
                    p3: paragraph { text("c") }
                }
            }
            selection: none
        };
        let frontier = heads_of(&state);

        let (state, _edit) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 2,
                item: SeqItem::Char('x'),
            }),
        );
        let (state, added) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 7,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }),
        );
        let (state, _fill) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 8,
                item: SeqItem::Char('y'),
            }),
        );
        let (state, _del) = apply(&state, EditOp::Seq(ListOp::Del { pos: 5, len: 2 }));

        let mut editor = editor_for_state(state);
        let now = WINDOW_MS;
        assert_eq!(editor.set_recent_edit_baseline(now, vec![(0, frontier)]), 1);

        assert_eq!(
            editor.recent_edits.live_blocks(now),
            vec![
                (p1, RecentEditKind::Modified),
                (added.id, RecentEditKind::Added)
            ]
        );

        let regions = editor.recent_edit_regions(now);
        let count = |kind| regions.iter().filter(|r| r.kind == kind).count();
        assert_eq!(
            (
                count(RecentEditKind::Added),
                count(RecentEditKind::Modified),
                count(RecentEditKind::Deleted)
            ),
            (1, 1, 1),
            "one query carries all three kinds — got {regions:?}"
        );
        assert_eq!(regions.len(), 3, "and nothing else — got {regions:?}");

        let neighbour = editor.view.node_content_rects(&[p2]);
        let deleted = regions
            .iter()
            .find(|r| r.kind == RecentEditKind::Deleted)
            .unwrap();
        assert_eq!(
            deleted.y,
            neighbour[0].rect.y + neighbour[0].rect.height,
            "the deletion hangs off the untouched block it followed, which is itself unmarked"
        );
    }

    #[test]
    fn the_injected_window_is_what_decides_expiry_and_added() {
        let short = 30 * RECENT_EDIT_BUCKET_MS;
        let b = Dot::new(1, 10);

        let mut narrow = RecentEditTracker::default();
        narrow.enable(0, short);
        narrow.record_baseline(&[RecentEditEffect::BlockCreated(b)], 0);

        let mut wide = RecentEditTracker::default();
        wide.enable(0, WINDOW_MS);
        wide.record_baseline(&[RecentEditEffect::BlockCreated(b)], 0);

        // Inside both windows: a fresh creation, either way.
        assert_eq!(
            narrow.live_blocks(RECENT_EDIT_BUCKET_MS),
            vec![(b, RecentEditKind::Added)]
        );
        assert_eq!(
            wide.live_blocks(RECENT_EDIT_BUCKET_MS),
            vec![(b, RecentEditKind::Added)]
        );

        // Exactly at the narrow window's trailing edge: the creation has aged out of
        // "new" there while the wide window still calls it new.
        assert_eq!(
            narrow.live_blocks(short),
            vec![(b, RecentEditKind::Modified)],
            "the host's window is what ages a creation out of Added"
        );
        assert_eq!(wide.live_blocks(short), vec![(b, RecentEditKind::Added)]);

        // Past the narrow window entirely.
        let later = short + RECENT_EDIT_BUCKET_MS;
        assert!(
            narrow.live_blocks(later).is_empty(),
            "the host's window is what expires records, not a constant in here"
        );
        assert_eq!(
            wide.live_blocks(later),
            vec![(b, RecentEditKind::Added)],
            "the same record is untouched under a wider window"
        );

        narrow.prune(later);
        assert!(narrow.live_blocks(0).is_empty(), "prune uses it too");
    }

    #[test]
    fn a_window_shorter_than_a_bucket_is_raised_to_one() {
        let mut t = RecentEditTracker::default();
        t.enable(0, 1);
        assert_eq!(t.window_ms(), RECENT_EDIT_BUCKET_MS);
    }

    #[test]
    fn re_enabling_re_clamps_the_window_every_time() {
        let mut t = RecentEditTracker::default();
        assert!(!t.enabled());

        t.enable(0, 5 * RECENT_EDIT_BUCKET_MS);
        assert!(t.enabled());
        assert_eq!(t.window_ms(), 5 * RECENT_EDIT_BUCKET_MS);

        for injected in [
            RECENT_EDIT_BUCKET_MS - 1,
            0,
            -RECENT_EDIT_BUCKET_MS,
            i64::MIN,
        ] {
            t.enable(0, injected);
            assert_eq!(
                t.window_ms(),
                RECENT_EDIT_BUCKET_MS,
                "a re-injected window of {injected} must clamp to a bucket, or the bucketed \
                 timestamps could never fall inside it"
            );
        }

        t.enable(0, 2 * RECENT_EDIT_BUCKET_MS);
        assert_eq!(
            t.window_ms(),
            2 * RECENT_EDIT_BUCKET_MS,
            "the clamp is a floor, so a later valid window still takes effect"
        );
    }

    #[test]
    fn the_clock_starts_at_the_injected_base_and_only_moves_forward() {
        let mut t = RecentEditTracker::default();
        assert_eq!(
            t.current_ms(),
            0,
            "before `enable` there is no base to count from"
        );

        let base = 1_700_000_000_000;
        t.enable(base, WINDOW_MS);
        let first = t.current_ms();
        let second = t.current_ms();
        assert!(first >= base, "the host's base is the floor — got {first}");
        assert!(second >= first, "got {first} then {second}");
        assert!(
            first - base < RECENT_EDIT_BUCKET_MS,
            "the offset is elapsed real time since `enable`, not a jump — got {first}"
        );

        let rebased = base + 7 * WINDOW_MS;
        t.enable(rebased, WINDOW_MS);
        assert!(
            t.current_ms() >= rebased,
            "a re-injected base replaces the old one instead of accumulating"
        );
        assert!(t.current_ms() < rebased + RECENT_EDIT_BUCKET_MS);
    }

    #[test]
    fn a_baseline_before_the_window_arrives_is_refused() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: none
        };
        let frontier = heads_of(&state);
        let (state, _op) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 3,
                item: SeqItem::Char('c'),
            }),
        );

        let mut editor = Editor::new_test(state);
        assert_eq!(
            editor.set_recent_edit_baseline(WINDOW_MS, vec![(0, frontier)]),
            0,
            "without the host's window there is nothing to date against"
        );
        assert!(editor.recent_edits.live_blocks(WINDOW_MS).is_empty());
    }

    #[test]
    fn tick_records_edits_only_once_enabled() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };

        // Deliberately not `editor_for_state`: this half must stay un-enabled.
        let mut idle = Editor::new_test(state.clone());
        idle.apply(Message::Insertion {
            op: InsertionOp::Text { text: "x".into() },
        });
        assert!(!idle.recent_edits.enabled());
        assert!(idle.recent_edits.live_blocks(0).is_empty());

        let mut editor = editor_for_state(state);
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: "x".into() },
        });

        let live = editor.recent_edits.live_blocks(0);
        assert_eq!(live, vec![(p1, RecentEditKind::Modified)]);
        assert!(
            editor
                .recent_edit_regions(0)
                .iter()
                .any(|r| r.kind == RecentEditKind::Modified)
        );
    }

    fn undo(editor: &mut Editor) {
        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });
    }

    fn redo(editor: &mut Editor) {
        editor.apply(Message::History {
            op: HistoryOp::Redo,
        });
    }

    fn type_text(editor: &mut Editor, text: &str) {
        editor.apply(Message::Insertion {
            op: InsertionOp::Text { text: text.into() },
        });
    }

    #[test]
    fn undoing_typing_clears_the_mark_and_redoing_earns_it_back() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = editor_for_state(state);

        type_text(&mut editor, "x");
        assert_eq!(
            editor.recent_edits.live_blocks(0),
            vec![(p1, RecentEditKind::Modified)]
        );

        undo(&mut editor);
        assert!(
            editor.recent_edits.live_blocks(0).is_empty(),
            "the only edit dating the block was taken back, so its mark goes with it"
        );
        assert!(editor.recent_edits.deleted_blocks(0).is_empty());

        redo(&mut editor);
        assert_eq!(
            editor.recent_edits.live_blocks(0),
            vec![(p1, RecentEditKind::Modified)],
            "the re-applied op is an ordinary edit, which re-earns the mark"
        );
    }

    #[test]
    fn undoing_a_formatting_change_clears_the_mark() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 3)
        };
        let mut editor = editor_for_state(state);

        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Bold,
            },
        });
        assert_eq!(
            editor.recent_edits.live_blocks(0),
            vec![(p1, RecentEditKind::Modified)]
        );

        undo(&mut editor);
        assert!(
            editor.recent_edits.live_blocks(0).is_empty(),
            "a span op's mark is retracted like any other edit's"
        );
    }

    #[test]
    fn undoing_the_deletion_of_a_block_created_in_the_window_brings_added_back() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 2)
        };
        let mut editor = editor_for_state(state);

        editor.apply(Message::Insertion {
            op: InsertionOp::Break {
                kind: Break::Paragraph,
            },
        });
        type_text(&mut editor, "z");
        let created: Vec<Dot> = editor
            .recent_edits
            .live_blocks(0)
            .into_iter()
            .filter(|(_, k)| *k == RecentEditKind::Added)
            .map(|(d, _)| d)
            .collect();
        assert_eq!(created.len(), 1, "the break must add exactly one block");

        editor.apply(Message::Key {
            event: KeyEvent {
                key: Key::Backspace,
                modifiers: InputModifiers::default(),
            },
        });
        editor.apply(Message::Key {
            event: KeyEvent {
                key: Key::Backspace,
                modifiers: InputModifiers::default(),
            },
        });
        assert!(
            !editor
                .recent_edits
                .live_blocks(0)
                .iter()
                .any(|(d, _)| *d == created[0]),
            "the new block is gone, so its Added mark is too"
        );

        undo(&mut editor);
        assert_eq!(
            editor
                .recent_edits
                .live_blocks(0)
                .into_iter()
                .filter(|(d, _)| *d == created[0])
                .collect::<Vec<_>>(),
            vec![(created[0], RecentEditKind::Added)],
            "undoing the deletion restores the record the deletion displaced, so a block \
             created inside the window comes back green rather than merely edited"
        );
        assert!(
            editor.recent_edits.deleted_blocks(0).is_empty(),
            "and nothing is left in the deleted channel"
        );
    }

    #[test]
    fn undoing_the_deletion_of_a_pre_existing_block_clears_the_deletion_marker() {
        let (state, _p1, p2) = state! {
            doc { root { p1: paragraph { text("a") } p2: paragraph { text("b") } } }
            selection: (p2, 0)
        };
        let mut editor = editor_for_state(state);

        editor.apply(Message::Key {
            event: KeyEvent {
                key: Key::Backspace,
                modifiers: InputModifiers::default(),
            },
        });
        assert_eq!(
            editor
                .recent_edits
                .deleted_blocks(0)
                .into_iter()
                .map(|(d, _)| d)
                .collect::<Vec<_>>(),
            vec![p2],
            "merging p2 away deletes a block that predates the window, so it is recorded red"
        );

        undo(&mut editor);
        assert!(
            editor.recent_edits.deleted_blocks(0).is_empty(),
            "undo takes the deletion back, so the red marker goes"
        );
        assert!(
            !editor
                .recent_edits
                .live_blocks(0)
                .iter()
                .any(|(d, _)| *d == p2),
            "and the restored block is not marked as edited either — it was never edited"
        );
    }

    #[test]
    fn a_baseline_contribution_survives_the_undo_of_live_typing() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 2)
        };
        let before_seed = heads_of(&state);
        let (state, _seed) = apply(
            &state,
            EditOp::Seq(ListOp::Ins {
                pos: 3,
                item: SeqItem::Char('c'),
            }),
        );
        let after_seed = heads_of(&state);

        let mut editor = editor_for_state(state);
        let base_at = -3 * RECENT_EDIT_BUCKET_MS;
        // Two buckets, so the seeded op is dated by the later one instead of falling
        // through to `now_ms` as everything past the last frontier does.
        assert_eq!(
            editor.set_recent_edit_baseline(
                0,
                vec![
                    (base_at - RECENT_EDIT_BUCKET_MS, before_seed),
                    (base_at, after_seed),
                ]
            ),
            2
        );
        assert_eq!(
            editor.recent_edits.live_blocks(0),
            vec![(p1, RecentEditKind::Modified)]
        );

        type_text(&mut editor, "x");
        undo(&mut editor);

        assert_eq!(
            editor.recent_edits.live_blocks(0),
            vec![(p1, RecentEditKind::Modified)],
            "the replayed baseline edit is not this session's to take back, so the mark stays"
        );
        // A `now` whose cutoff sits between the baseline's date and the retracted
        // typing's: the record only drops out here if its dating rolled back.
        let past_the_baseline = WINDOW_MS - 2 * RECENT_EDIT_BUCKET_MS;
        assert!(
            editor
                .recent_edits
                .live_blocks(past_the_baseline)
                .is_empty(),
            "and it is dated by the baseline again, not by the typing that was undone"
        );
    }

    #[test]
    fn undoing_one_of_two_unmerged_edits_leaves_the_other_standing() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 3)
        };
        let mut editor = editor_for_state(state);

        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Bold,
            },
        });
        let undos_after_first = editor.history_undos_len();
        type_text(&mut editor, "z");
        assert_eq!(
            editor.history_undos_len(),
            undos_after_first + 1,
            "the two edits must land in separate undo units, or the test proves nothing"
        );

        undo(&mut editor);
        assert_eq!(
            editor.recent_edits.live_blocks(0),
            vec![(p1, RecentEditKind::Modified)],
            "the formatting edit still dates the block, so the mark survives"
        );
    }

    #[test]
    fn prune_drops_op_entries_whose_contributions_expired() {
        let window = 3 * RECENT_EDIT_BUCKET_MS;
        let mut t = RecentEditTracker::default();
        t.enable(0, window);

        let stale_op = Dot::new(1, 1);
        let fresh_op = Dot::new(1, 2);
        t.record_tracked(
            stale_op,
            &[RecentEditEffect::BlockEdited(Dot::new(2, 1))],
            0,
        );
        let now = 4 * RECENT_EDIT_BUCKET_MS;
        t.record_tracked(
            fresh_op,
            &[RecentEditEffect::BlockEdited(Dot::new(2, 2))],
            now,
        );

        t.prune(now);

        assert_eq!(
            t.op_contribs.keys().copied().collect::<Vec<_>>(),
            vec![fresh_op],
            "the retraction index is pruned on the same cutoff as the record it serves, or it \
             grows for the life of the session"
        );
    }

    #[test]
    fn undoing_a_block_creation_leaves_no_record_in_either_channel() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 2)
        };
        let mut editor = editor_for_state(state);

        editor.apply(Message::Insertion {
            op: InsertionOp::Break {
                kind: Break::Paragraph,
            },
        });
        assert_eq!(
            editor
                .recent_edits
                .live_blocks(0)
                .into_iter()
                .filter(|(_, k)| *k == RecentEditKind::Added)
                .count(),
            1
        );

        undo(&mut editor);
        assert!(
            editor.recent_edits.live_blocks(0).is_empty(),
            "the created block is gone from the document, so it is gone from the tracker"
        );
        assert!(
            editor.recent_edits.deleted_blocks(0).is_empty(),
            "and undoing a creation is not a deletion — the inverse op must not be classified"
        );
    }

    /// A tick carries every message of one frame, so an edit and the undo of that
    /// same edit reach the tracker together. `Editor::apply` cannot express this —
    /// it enqueues one message and ticks.
    fn apply_together(editor: &mut Editor, messages: Vec<Message>) {
        editor.enqueue_request(messages).unwrap();
        editor.tick().unwrap();
    }

    fn bold_at(editor: &Editor, block: Dot, slot: usize) -> bool {
        let view = editor.state().view();
        let dot = match view.node(block).unwrap().child_at(slot) {
            Some(ChildView::Leaf(leaf)) => leaf.dot(),
            _ => panic!("no leaf at slot {slot}"),
        };
        view.leaf_state_by_dot_slow(dot)
            .unwrap()
            .eff
            .contains_key(&ModifierType::Bold)
    }

    fn toggle_bold() -> Message {
        Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Bold,
            },
        }
    }

    // A formatting op, not typing: an insertion's classification reads the
    // post-undo document and finds the character already gone, so it hides this
    // ordering bug behind an empty effect list. A span op's anchors still resolve
    // after its undo, so a mis-ordered tick records the mark for real.
    #[test]
    fn an_edit_and_its_undo_in_the_same_tick_leave_no_mark() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 3)
        };
        let mut editor = editor_for_state(state);

        apply_together(
            &mut editor,
            vec![
                toggle_bold(),
                Message::History {
                    op: HistoryOp::Undo,
                },
            ],
        );

        assert!(
            !bold_at(&editor, p1, 0),
            "전제: 한 tick 안에서 서식과 그 undo가 모두 적용된다"
        );
        assert!(
            editor.recent_edits.live_blocks(0).is_empty(),
            "the edit was taken back inside the tick that recorded it, so the mark must go \
             with it — retracting before recording would look for a contribution this tick \
             had not written yet, miss it, and leave a mark no later undo could match"
        );
        assert!(editor.recent_edits.deleted_blocks(0).is_empty());
    }

    #[test]
    fn a_redo_and_the_undo_that_follows_it_in_the_same_tick_leave_no_mark() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0) -> (p1, 3)
        };
        let mut editor = editor_for_state(state);

        editor.apply(toggle_bold());
        undo(&mut editor);
        assert!(editor.recent_edits.live_blocks(0).is_empty());

        apply_together(
            &mut editor,
            vec![
                Message::History {
                    op: HistoryOp::Redo,
                },
                Message::History {
                    op: HistoryOp::Undo,
                },
            ],
        );

        assert!(
            !bold_at(&editor, p1, 0),
            "전제: 한 tick 안의 redo와 undo가 문서를 원위치시킨다"
        );
        assert!(
            editor.recent_edits.live_blocks(0).is_empty(),
            "the redo's re-applied op is recorded and retracted inside one tick"
        );
    }

    /// The two-peer fixture a real concurrent deletion needs is heavier than the
    /// guard it would exercise, so the ownership check is pinned directly: two
    /// deletions of one block, only one of them retracted.
    #[test]
    fn retracting_a_deletion_leaves_a_concurrent_deletions_marker_standing() {
        let (state, p1, p2) = state! {
            doc { root { p1: paragraph { text("a") } p2: paragraph { text("b") } } }
            selection: none
        };
        let mut t = RecentEditTracker::default();
        t.enable(0, WINDOW_MS);

        let mine = Dot::new(1, 1);
        let theirs = Dot::new(2, 1);
        for del_op in [mine, theirs] {
            t.record_tracked(
                del_op,
                &[RecentEditEffect::BlockDeleted {
                    block: p2,
                    del_op,
                    site: Some(p1),
                }],
                0,
            );
        }

        t.retract(&state, &[mine]);

        assert_eq!(
            t.deleted_blocks(0)
                .into_iter()
                .map(|(b, r)| (b, r.del_op))
                .collect::<Vec<_>>(),
            vec![(p2, theirs)],
            "the marker standing was written by the other deletion, which this undo did not \
             take back — an unconditional remove would erase a mark this op never made"
        );
    }

    #[test]
    fn retracting_a_deletion_does_not_revive_a_block_the_document_still_hides() {
        let (state, p1, p2) = state! {
            doc { root { p1: paragraph { text("a") } p2: paragraph { text("b") } } }
            selection: none
        };
        // `p2` really is gone here, so a retraction that restores its live record
        // would mark a block the document does not show.
        let (without_p2, _del) = apply(&state, EditOp::Seq(ListOp::Del { pos: 2, len: 2 }));
        assert!(
            without_p2.view().node(p2).is_none(),
            "전제: p2가 실제로 사라진다"
        );

        let mut t = RecentEditTracker::default();
        t.enable(0, WINDOW_MS);
        let edit = Dot::new(1, 1);
        let del_op = Dot::new(1, 2);
        t.record_tracked(edit, &[RecentEditEffect::BlockEdited(p2)], 0);
        t.record_tracked(
            del_op,
            &[RecentEditEffect::BlockDeleted {
                block: p2,
                del_op,
                site: Some(p1),
            }],
            0,
        );

        t.retract(&without_p2, &[del_op]);

        assert!(
            t.live_blocks(0).is_empty(),
            "the displaced record is only put back while the document still shows the block"
        );
        assert!(t.deleted_blocks(0).is_empty(), "this op's own marker goes");
    }

    #[test]
    fn a_deletion_is_retracted_before_the_edits_it_swallowed() {
        let (state, p1, p2) = state! {
            doc { root { p1: paragraph { text("a") } p2: paragraph { text("b") } } }
            selection: none
        };
        let mut t = RecentEditTracker::default();
        t.enable(0, WINDOW_MS);

        let edit = Dot::new(1, 1);
        let del_op = Dot::new(1, 2);
        t.record_tracked(edit, &[RecentEditEffect::BlockEdited(p2)], 0);
        t.record_tracked(
            del_op,
            &[RecentEditEffect::BlockDeleted {
                block: p2,
                del_op,
                site: Some(p1),
            }],
            0,
        );

        // Newest first, as the ops come apart.
        t.retract(&state, &[del_op, edit]);

        assert!(
            t.live_blocks(0).is_empty(),
            "unwinding the edit first would find its record already swallowed by the \
             deletion, and the deletion would then restore a snapshot still counting it"
        );
        assert!(t.deleted_blocks(0).is_empty());
    }

    #[test]
    fn retracting_a_move_puts_back_the_deletion_record_the_departure_dropped() {
        let (state, p1, p2) = state! {
            doc { root { p1: paragraph { text("a") } p2: paragraph { text("b") } } }
            selection: none
        };
        let mut t = RecentEditTracker::default();
        t.enable(0, WINDOW_MS);

        let del_op = Dot::new(1, 1);
        let alias = Dot::new(1, 2);
        t.record_tracked(
            del_op,
            &[RecentEditEffect::BlockDeleted {
                block: p2,
                del_op,
                site: Some(p1),
            }],
            0,
        );
        t.record_tracked(alias, &[RecentEditEffect::MoveErase { old: p2 }], 0);
        assert!(
            t.deleted_blocks(0).is_empty(),
            "전제: 이동 출발은 삭제로 계상되지 않는다"
        );

        t.retract(&state, &[alias]);

        assert_eq!(
            t.deleted_blocks(0)
                .into_iter()
                .map(|(b, r)| (b, r.del_op))
                .collect::<Vec<_>>(),
            vec![(p2, del_op)],
            "the departure dropped a deletion record, so taking the departure back has to \
             hand it to the deletion's own retraction to remove"
        );
    }

    #[test]
    fn the_recording_path_prunes_on_its_own_clock_without_a_query() {
        let mut t = RecentEditTracker::default();
        t.enable(0, 3 * RECENT_EDIT_BUCKET_MS);

        let stale = Dot::new(1, 1);
        t.record_tracked(stale, &[RecentEditEffect::BlockEdited(Dot::new(2, 1))], 0);
        t.prune_if_bucket_advanced(0);
        assert_eq!(t.op_contribs.len(), 1, "nothing has expired yet");

        let later = 5 * RECENT_EDIT_BUCKET_MS;
        let fresh = Dot::new(1, 2);
        t.record_tracked(
            fresh,
            &[RecentEditEffect::BlockEdited(Dot::new(2, 2))],
            later,
        );
        t.prune_if_bucket_advanced(later);
        assert_eq!(
            t.op_contribs.keys().copied().collect::<Vec<_>>(),
            vec![fresh],
            "the op index is bounded by the engine's own clock — a host that never asks for \
             regions (the marks are switched off) must not leave it growing for the session"
        );

        let same_bucket = Dot::new(1, 3);
        t.record_tracked(
            same_bucket,
            &[RecentEditEffect::BlockEdited(Dot::new(2, 3))],
            0,
        );
        t.prune_if_bucket_advanced(later + 1);
        assert_eq!(
            t.op_contribs.len(),
            2,
            "and it runs once per bucket, not once per recorded op"
        );
    }

    #[test]
    fn pruning_an_expired_op_drops_its_claim_on_the_blocks_it_dated() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("a") } } }
            selection: none
        };
        let window = 3 * RECENT_EDIT_BUCKET_MS;
        let mut t = RecentEditTracker::default();
        t.enable(0, window);

        let block = Dot::new(2, 1);
        let expired = Dot::new(1, 1);
        let recent = Dot::new(1, 2);
        t.record_tracked(expired, &[RecentEditEffect::BlockEdited(block)], 0);
        let now = 4 * RECENT_EDIT_BUCKET_MS;
        t.record_tracked(recent, &[RecentEditEffect::BlockEdited(block)], now);

        t.prune(now);
        assert_eq!(
            t.live_blocks(now),
            vec![(block, RecentEditKind::Modified)],
            "전제: 최신 기여가 레코드를 살려둔다"
        );

        t.retract(&state, &[recent]);

        assert!(
            t.live_blocks(now).is_empty(),
            "the expired op can never be retracted, so pruning it had to drop its count too \
             — a count outliving every contribution that can still be taken back would keep \
             the record standing with nothing justifying it, and presence is not a judgement \
             the spec lets drift"
        );
    }

    proptest! {
        #[test]
        fn tracked_blocks_stay_live_and_untouched_ones_stay_unmarked(
            script in prop::collection::vec((0u8..7, any::<u8>()), 1..40)
        ) {
            let (mut state, p0, _p1) = state! {
                doc {
                    root {
                        p0: paragraph { text("keep") }
                        p1: paragraph { text("seed") }
                    }
                }
                selection: none
            };
            let baseline_blocks = live_block_set(&state);
            let frontier = heads_of(&state);

            let mut editor = editor_for_state(state.clone());
            let accepted =
                editor.set_recent_edit_baseline(WINDOW_MS, vec![(0, frontier)]);
            prop_assert_eq!(accepted, 1);
            prop_assert!(
                editor.recent_edits.live_blocks(WINDOW_MS).is_empty(),
                "a baseline taken at the current heads has nothing to replay"
            );

            let mut dels: Vec<Dot> = Vec::new();
            for (step, pick) in script {
                let len = visible_len(&state);
                let pick = pick as usize;
                let payload = match step {
                    0 => EditOp::Seq(ListOp::Ins {
                        pos: len,
                        item: SeqItem::Char('x'),
                    }),
                    // `len > 7` keeps every deletion strictly after `p1`'s marker, so the
                    // seed blocks stay whole and `p0` stays provably untouched.
                    1 if len > 7 => EditOp::Seq(ListOp::Del { pos: len - 1, len: 1 }),
                    1 => continue,
                    2 => EditOp::Seq(ListOp::Ins {
                        pos: len,
                        item: SeqItem::Block {
                            node_type: NodeType::Paragraph,
                            parents: vec![Dot::ROOT],
                            attrs: vec![],
                        },
                    }),
                    3 => EditOp::Seq(ListOp::Ins {
                        pos: len,
                        item: SeqItem::Block {
                            node_type: NodeType::Callout,
                            parents: vec![Dot::ROOT],
                            attrs: vec![],
                        },
                    }),
                    4 => {
                        let callouts = live_blocks_of_type(&state, NodeType::Callout);
                        if callouts.is_empty() {
                            continue;
                        }
                        EditOp::NodeAttr(NodeAttrOp {
                            target: callouts[pick % callouts.len()],
                            attr: NodeAttr::Callout {
                                attr: CalloutNodeAttr::Variant(if pick.is_multiple_of(2) {
                                    CalloutVariant::Warning
                                } else {
                                    CalloutVariant::Success
                                }),
                            },
                        })
                    }
                    5 => {
                        let targets: Vec<Dot> = live_blocks_of_type(&state, NodeType::Paragraph)
                            .into_iter()
                            .filter(|d| *d != p0)
                            .collect();
                        if targets.is_empty() {
                            continue;
                        }
                        EditOp::BlockModifier(ModifierAttrOp::SetModifier {
                            target: targets[pick % targets.len()],
                            modifier: Modifier::FontSize {
                                value: 1200 + (pick as u32 % 8) * 100,
                            },
                        })
                    }
                    // Taken out of the pool as it is used: a second undel of the same
                    // delete underflows the sequence's per-target counters.
                    _ => {
                        if dels.is_empty() {
                            continue;
                        }
                        let del = dels.swap_remove(pick % dels.len());
                        EditOp::Seq(ListOp::Undel { del })
                    }
                };
                let deletes = matches!(payload, EditOp::Seq(ListOp::Del { .. }));
                let Ok((next, op)) = state.apply(payload) else {
                    continue;
                };
                state = next;
                if deletes {
                    dels.push(op.id);
                }
                let effects = classify_op(&state, &op);
                editor
                    .recent_edits
                    .record_baseline(&effects, WINDOW_MS);
            }

            let current_blocks = live_block_set(&state);
            let live = editor
                .recent_edits
                .live_blocks(WINDOW_MS + 1000);

            for (dot, kind) in &live {
                let synthesized_by_projection = state.projected.graph().get(dot).is_none();
                prop_assert!(
                    current_blocks.contains(dot) || synthesized_by_projection,
                    "live mark on {:?}, an op-identified block the document no longer shows. \
                     Every effect this script can emit either keeps its block or takes the \
                     mark away with it; `MoveErase` is the one that does not, and no generated \
                     op produces it. A mark on a block the projection synthesized (an empty \
                     container's implicit paragraph) is exempt: that identity is derived rather \
                     than an op id, so it is replaced as the document changes and the mark goes \
                     stale, resolving to no rect and painting nothing",
                    dot
                );
                if *kind == RecentEditKind::Added {
                    prop_assert!(
                        !baseline_blocks.contains(dot),
                        "block {:?} existed at baseline yet reports Added",
                        dot
                    );
                }
            }

            prop_assert!(
                !live.iter().any(|(d, _)| *d == p0),
                "untouched pre-existing block was marked"
            );
        }

        #[test]
        fn marker_index_classifies_a_replay_exactly_as_the_backward_walk(
            script in prop::collection::vec(0u8..4, 1..40)
        ) {
            let (mut state, _p0, _p1) = state! {
                doc {
                    root {
                        p0: paragraph { text("keep") }
                        p1: paragraph { text("seed") }
                    }
                }
                selection: none
            };

            let mut ops = Vec::new();
            for step in script {
                let len = visible_len(&state);
                let payload = match step {
                    0 => EditOp::Seq(ListOp::Ins {
                        pos: len,
                        item: SeqItem::Char('x'),
                    }),
                    1 => EditOp::Seq(ListOp::Ins {
                        pos: len,
                        item: SeqItem::Block {
                            node_type: NodeType::Paragraph,
                            parents: vec![Dot::ROOT],
                            attrs: vec![],
                        },
                    }),
                    // From the front, so each later delete's targets sit behind a growing
                    // stretch of tombstones — the shape the index exists for.
                    2 if len > 1 => EditOp::Seq(ListOp::Del { pos: 1, len: 1 }),
                    _ if len > 3 => EditOp::Seq(ListOp::Del { pos: 1, len: 3 }),
                    _ => continue,
                };
                let Ok((next, op)) = state.apply(payload) else {
                    continue;
                };
                state = next;
                ops.push(op);
            }

            let index = BlockMarkerIndex::build(&state);
            for op in &ops {
                prop_assert_eq!(
                    classify_op_with(&state, op, Some(&index)),
                    classify_op(&state, op),
                    "indexed classification diverged for {:?}",
                    op.id
                );
            }
        }
    }
}
