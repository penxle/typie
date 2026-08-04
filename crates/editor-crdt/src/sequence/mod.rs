use crate::oplog::{ListOp, NYI, OpLog, item_width, lv_cmp};
use crate::{Dot, FastMap, FastSet};
use editor_common::content_tree::{ContentTree, Cursor, Leaf, Sum};
use hashbrown::HashMap;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bias {
    Before,
    After,
}

/// The visible insertions immediately outside a delete's target span in the
/// delete's parent version. These dots are historical observations and may
/// themselves be tombstones in the current checkout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeletionGap {
    /// The insertion immediately before the deleted span, if any.
    pub left: Option<Dot>,
    /// The insertion immediately after the deleted span, if any.
    pub right: Option<Dot>,
}

#[derive(Clone, Copy, Debug)]
struct DeleteRecord {
    id: Dot,
    gap: DeletionGap,
}

type DeleteRefs = smallvec::SmallVec<[usize; 1]>;

#[derive(Clone, Debug, Default)]
struct DeletionIndex {
    records: FastMap<usize, DeleteRecord>,
    by_target: FastMap<usize, DeleteRefs>,
    inactive: FastSet<usize>,
}

impl DeletionIndex {
    fn record_delete(&mut self, delete_lv: usize, id: Dot, gap: DeletionGap, targets: &[usize]) {
        self.records.insert(delete_lv, DeleteRecord { id, gap });
        for &target in targets {
            let mut deletes = self.by_target.get(&target).cloned().unwrap_or_default();
            deletes.push(delete_lv);
            self.by_target.insert(target, deletes);
        }
    }

    fn deactivate_delete(&mut self, delete_lv: usize) {
        self.inactive.insert(delete_lv);
    }

    fn gap_for_target(
        &self,
        tree: &ContentTree<Run>,
        lv_of: &crate::DotMap<usize>,
        target: Dot,
    ) -> Option<DeletionGap> {
        let target_lv = *lv_of.get(&target)?;
        tree.doc_index_of_lv_checked(target_lv)?;
        self.by_target
            .get(&target_lv)?
            .iter()
            .filter(|&&delete_lv| !self.inactive.contains(&delete_lv))
            .filter_map(|&delete_lv| self.records.get(&delete_lv))
            .min_by(|a, b| {
                a.id.actor
                    .cmp(&b.id.actor)
                    .then(a.id.clock.cmp(&b.id.clock))
            })
            .map(|record| record.gap)
    }
}

#[derive(Clone, Debug)]
struct Run {
    start: Dot,
    start_lv: usize,
    len: usize,
    cur: i32,
    end: i32,
    ol: i32,
    rp: i32,
}

impl Run {
    fn single(lv: usize, dot: Dot, cur: i32, end: i32, ol: i32, rp: i32) -> Self {
        Run {
            start: dot,
            start_lv: lv,
            len: 1,
            cur,
            end,
            ol,
            rp,
        }
    }

    fn op_id_at(&self, offset: usize) -> usize {
        self.start_lv + offset
    }

    fn origin_left_at(&self, offset: usize) -> i32 {
        if offset == 0 {
            self.ol
        } else {
            (self.start_lv + offset - 1) as i32
        }
    }
}

impl Leaf for Run {
    fn sum(&self) -> Sum {
        Sum {
            count: self.len,
            cur: item_width(self.cur) * self.len,
            end: item_width(self.end) * self.len,
        }
    }

    fn run_len(&self) -> usize {
        self.len
    }

    fn try_append(&mut self, other: &Self) -> bool {
        let lv_contig = other.start_lv == self.start_lv + self.len;
        let dot_contig = other.start.actor == self.start.actor
            && other.start.clock == self.start.clock + self.len as u64;
        let state_eq = other.cur == self.cur && other.end == self.end;
        let rp_eq = other.rp == self.rp;
        let origin_chains = other.ol == (self.start_lv + self.len - 1) as i32;
        if lv_contig && dot_contig && state_eq && rp_eq && origin_chains {
            self.len += other.len;
            true
        } else {
            false
        }
    }

    fn split_at(&mut self, offset: usize) -> Self {
        debug_assert!(offset > 0 && offset < self.len);
        let right = Run {
            start: Dot::new(self.start.actor, self.start.clock + offset as u64),
            start_lv: self.start_lv + offset,
            len: self.len - offset,
            cur: self.cur,
            end: self.end,
            ol: (self.start_lv + offset - 1) as i32,
            rp: self.rp,
        };
        self.len = offset;
        right
    }

    fn lv_start(&self) -> usize {
        self.start_lv
    }

    fn contains_lv(&self, lv: usize) -> bool {
        lv >= self.start_lv && lv < self.start_lv + self.len
    }

    fn offset_of_lv(&self, lv: usize) -> usize {
        lv - self.start_lv
    }
}

#[derive(Clone, Debug)]
struct Ctx {
    tree: ContentTree<Run>,
    // `imbl::Vector` so cloning a `SeqCheckout` (copy-on-write on the typing
    // path) shares the per-op delete-target lists by pointer instead of
    // deep-copying ~one (usually empty) allocation per op.
    del_targets: imbl::Vector<Vec<usize>>,
    deletions: DeletionIndex,
    cur_version: Vec<usize>,
}

fn advance1<P: Clone>(ctx: &mut Ctx, log: &OpLog<P>, lv: usize) {
    match log.entries[lv].op {
        ListOp::Del { .. } => {
            for i in 0..ctx.del_targets[lv].len() {
                let target = ctx.del_targets[lv][i];
                ctx.tree.update_by_lv(target, |it| it.cur += 1);
            }
        }
        ListOp::Undel { .. } => {
            for i in 0..ctx.del_targets[lv].len() {
                let target = ctx.del_targets[lv][i];
                ctx.tree.update_by_lv(target, |it| {
                    assert!(it.cur >= 1, "undel advance underflow");
                    it.cur -= 1;
                });
            }
        }
        ListOp::Ins { .. } => ctx.tree.update_by_lv(lv, |it| it.cur = 0),
    }
}

fn retreat1<P: Clone>(ctx: &mut Ctx, log: &OpLog<P>, lv: usize) {
    match log.entries[lv].op {
        ListOp::Del { .. } => {
            for i in 0..ctx.del_targets[lv].len() {
                let target = ctx.del_targets[lv][i];
                ctx.tree.update_by_lv(target, |it| it.cur -= 1);
            }
        }
        ListOp::Undel { .. } => {
            for i in 0..ctx.del_targets[lv].len() {
                let target = ctx.del_targets[lv][i];
                ctx.tree.update_by_lv(target, |it| it.cur += 1);
            }
        }
        ListOp::Ins { .. } => ctx.tree.update_by_lv(lv, |it| it.cur -= 1),
    }
}

fn integrate<P: Clone>(
    tree: &ContentTree<Run>,
    log: &OpLog<P>,
    new_item: &Run,
    cursor: &mut Cursor,
) {
    let len = tree.len();
    match tree.cur_run(cursor) {
        None => return,
        Some(r) => {
            if r.cur != NYI {
                return;
            }
        }
    }
    let mut scanning = false;
    let mut scan = *cursor;
    let left_idx = cursor.doc_idx as i32 - 1;
    let right_idx = if new_item.rp == -1 {
        len as i32
    } else {
        tree.doc_index_of_lv(new_item.rp as usize) as i32
    };
    while scan.doc_idx < len {
        let run = match tree.cur_run(&scan) {
            Some(r) => r,
            None => break,
        };
        if run.cur != NYI {
            break;
        }
        let off = scan.off;
        let o_origin_left = run.origin_left_at(off);
        let o_right_parent = run.rp;
        let o_op_id = run.op_id_at(off);
        let o_left = if o_origin_left == -1 {
            -1
        } else {
            tree.doc_index_of_lv(o_origin_left as usize) as i32
        };
        if o_left < left_idx {
            break;
        } else if o_left == left_idx {
            let o_right = if o_right_parent == -1 {
                len as i32
            } else {
                tree.doc_index_of_lv(o_right_parent as usize) as i32
            };
            if o_right == right_idx
                && lv_cmp(log, new_item.op_id_at(0), o_op_id) == std::cmp::Ordering::Less
            {
                break;
            } else {
                scanning = o_right < right_idx;
            }
        }
        let add_end = item_width(run.end);
        let skipped = tree.step_run(&mut scan);
        scan.end_pos += add_end * skipped;
        if !scanning {
            *cursor = scan;
        }
    }
}

/// Apply `f` to every target element, batching maximal consecutive-`lv` spans into one
/// `update_run_by_lv` call each. A contiguous run of freshly (un)deleted elements is
/// tombstoned/restored in `O(runs)` instead of `O(len)`. `targets` are collected in
/// document order; where that matches insertion order (the usual sequential document)
/// the whole span collapses to a handful of batches. Concurrent-edit reordering just
/// yields shorter spans — never incorrect ones, since a span is only ever consecutive
/// `lv`s, and elements sharing a run always share their per-run state.
fn batch_update_targets(tree: &mut ContentTree<Run>, targets: &[usize], f: impl Fn(&mut Run)) {
    let mut i = 0;
    while i < targets.len() {
        let seg_start = targets[i];
        let mut j = i;
        while j + 1 < targets.len() && targets[j + 1] == targets[j] + 1 {
            j += 1;
        }
        let count = targets[j] - seg_start + 1;
        tree.update_run_by_lv(seg_start, count, &f);
        i = j + 1;
    }
}

fn dot_at_cur_position(tree: &ContentTree<Run>, pos: usize) -> Option<Dot> {
    let cursor = tree.cursor_at_cur_pos(pos + 1);
    let (run, offset) = tree.prev_slot(&cursor)?;
    debug_assert_eq!(run.cur, 0, "previous cur-position slot must be visible");
    Some(Dot::new(run.start.actor, run.start.clock + offset as u64))
}

fn apply1<P: Clone>(ctx: &mut Ctx, log: &OpLog<P>, lv: usize, op: &ListOp<P>, dot: Dot) {
    match op {
        ListOp::Del { pos, len } => {
            let (pos, len) = (*pos, *len);
            let visible = ctx.tree.cur_len();
            debug_assert!(
                pos <= visible && len <= visible - pos,
                "range delete out of bounds: pos={pos} len={len} visible={visible}"
            );
            let mut targets = Vec::with_capacity(len);
            if len > 0 {
                let gap = DeletionGap {
                    left: pos
                        .checked_sub(1)
                        .and_then(|position| dot_at_cur_position(&ctx.tree, position)),
                    right: (pos + len < visible)
                        .then(|| dot_at_cur_position(&ctx.tree, pos + len))
                        .flatten(),
                };
                let mut c = ctx.tree.cursor_at_cur_pos(pos);
                for _ in 0..len {
                    loop {
                        let cur_state = {
                            let r = ctx.tree.cur_run(&c).expect("del target exists");
                            r.cur
                        };
                        if cur_state == 0 {
                            break;
                        }
                        ctx.tree.step_run(&mut c);
                    }
                    let target_lv = {
                        let r = ctx.tree.cur_run(&c).expect("del target");
                        r.op_id_at(c.off)
                    };
                    targets.push(target_lv);
                    ctx.tree.step(&mut c);
                }
                ctx.deletions.record_delete(lv, dot, gap, &targets);
                batch_update_targets(&mut ctx.tree, &targets, |it| {
                    it.cur += 1;
                    it.end += 1;
                });
            }
            ctx.del_targets.set(lv, targets);
        }
        ListOp::Undel { del } => {
            let del = *del;
            let del_lv = *log.lv_of.get(&del).expect("undel references unknown del");
            ctx.deletions.deactivate_delete(del_lv);
            let targets = ctx.del_targets[del_lv].clone();
            batch_update_targets(&mut ctx.tree, &targets, |it| {
                assert!(it.cur >= 1 && it.end >= 1, "undel underflow");
                it.cur -= 1;
                it.end -= 1;
            });
            ctx.del_targets.set(lv, targets);
        }
        ListOp::Ins { pos, .. } => {
            let c = ctx.tree.cursor_at_cur_pos(*pos);
            let origin_left = if c.doc_idx == 0 {
                -1
            } else {
                // Local backward step from the cursor instead of a second
                // root descend (`get(doc_idx - 1)`).
                let (run, off) = ctx.tree.prev_slot(&c).expect("doc_idx > 0 has predecessor");
                run.op_id_at(off) as i32
            };
            let mut right_parent = -1;
            let mut scan = c;
            while scan.doc_idx < ctx.tree.len() {
                let (next_cur_state, next_origin_left, next_op_id) = match ctx.tree.cur_run(&scan) {
                    Some(r) => (r.cur, r.origin_left_at(scan.off), r.op_id_at(scan.off)),
                    None => break,
                };
                if next_cur_state != NYI {
                    right_parent = if next_origin_left == origin_left {
                        next_op_id as i32
                    } else {
                        -1
                    };
                    break;
                }
                ctx.tree.step_run(&mut scan);
            }
            let new_item = Run::single(lv, dot, 0, 0, origin_left, right_parent);
            let mut c = c;
            integrate(&ctx.tree, log, &new_item, &mut c);
            // The cursor already addresses the insertion slot — skip the
            // third root descend the position-based `insert` would pay.
            ctx.tree.insert_at_cursor(&c, new_item);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Flag {
    A,
    B,
    Shared,
}

fn diff<P: Clone>(log: &OpLog<P>, a: &[usize], b: &[usize]) -> (Vec<usize>, Vec<usize>) {
    let mut flags: HashMap<usize, Flag> = HashMap::new();
    let mut queue: BinaryHeap<usize> = BinaryHeap::new();
    let mut num_shared = 0usize;

    let enq = |queue: &mut BinaryHeap<usize>,
               flags: &mut HashMap<usize, Flag>,
               num_shared: &mut usize,
               v: usize,
               flag: Flag| {
        match flags.get(&v).copied() {
            None => {
                queue.push(v);
                flags.insert(v, flag);
                if flag == Flag::Shared {
                    *num_shared += 1;
                }
            }
            Some(current) => {
                if flag != current && current != Flag::Shared {
                    flags.insert(v, Flag::Shared);
                    *num_shared += 1;
                }
            }
        }
    };

    for &v in a {
        enq(&mut queue, &mut flags, &mut num_shared, v, Flag::A);
    }
    for &v in b {
        enq(&mut queue, &mut flags, &mut num_shared, v, Flag::B);
    }

    let mut a_only: Vec<usize> = Vec::new();
    let mut b_only: Vec<usize> = Vec::new();

    while queue.len() > num_shared {
        let v = queue.pop().expect("queue non-empty while len > num_shared");
        let flag = *flags.get(&v).expect("queued LV has a flag");

        if flag == Flag::Shared {
            num_shared -= 1;
        } else if flag == Flag::A {
            a_only.push(v);
        } else {
            b_only.push(v);
        }

        for &p in &log.entries[v].parents {
            enq(&mut queue, &mut flags, &mut num_shared, p, flag);
        }
    }

    (a_only, b_only)
}

fn apply_one<P: Clone>(
    ctx: &mut Ctx,
    log: &OpLog<P>,
    lv: usize,
    op: &ListOp<P>,
    dot: Dot,
    parents: &crate::oplog::LvParents,
) {
    // Sequential hot path: the op parents on exactly the current version
    // (every op of a linear history), so the version diff is empty — skip
    // the per-op `diff` HashMap/heap entirely.
    if ctx.cur_version.as_slice() != parents.as_slice() {
        let (a_only, b_only) = diff(log, &ctx.cur_version, parents);
        let mut retreat = a_only;
        retreat.sort_unstable_by(|x, y| y.cmp(x));
        for r in retreat {
            retreat1(ctx, log, r);
        }
        let mut advance = b_only;
        advance.sort_unstable();
        for a in advance {
            advance1(ctx, log, a);
        }
    }
    apply1(ctx, log, lv, op, dot);
    ctx.cur_version.clear();
    ctx.cur_version.push(lv);
}

#[derive(Clone, Debug)]
pub struct SeqCheckout {
    ctx: Ctx,
    // Persistent so a `ProjectedState` clone shares it in `O(1)`; a plain HashMap here
    // rehashes `O(total ops)` per copy-on-write mutation on the typing path, which on a
    // large op history dominates the per-keystroke clone.
    lv_of: crate::DotMap<usize>,
    applied: usize,
}

impl Default for SeqCheckout {
    fn default() -> Self {
        SeqCheckout {
            ctx: Ctx {
                tree: ContentTree::new(),
                del_targets: imbl::Vector::new(),
                deletions: DeletionIndex::default(),
                cur_version: Vec::new(),
            },
            lv_of: crate::DotMap::new(),
            applied: 0,
        }
    }
}

impl SeqCheckout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn applied_len(&self) -> usize {
        self.applied
    }

    pub fn apply_range<P: Clone>(&mut self, log: &OpLog<P>, range: std::ops::Range<usize>) {
        debug_assert_eq!(range.start, self.applied, "apply_range must be contiguous");
        while self.ctx.del_targets.len() < log.entries.len() {
            self.ctx.del_targets.push_back(Vec::new());
        }
        // Focus cursor: O(1) chunk-cached sequential access instead of an
        // O(log n) RRB tree walk per `log.entries[lv]` index.
        let mut entries = log.entries.focus();
        for lv in range {
            let e = entries.index(lv);
            apply_one(&mut self.ctx, log, lv, &e.op, e.dot, &e.parents);
            self.lv_of.insert(e.dot, lv);
            self.applied = lv + 1;
        }
    }

    pub fn apply_tail<P: Clone>(&mut self, log: &OpLog<P>) {
        let from = self.applied;
        self.apply_range(log, from..log.entries.len());
    }

    pub fn visible_len(&self) -> usize {
        self.ctx.tree.end_len()
    }

    pub fn snapshot<P: Clone>(&self, log: &OpLog<P>) -> Vec<(Dot, P)> {
        let visible = self.ctx.tree.end_len();
        // A heavily-edited document accumulates tombstone runs; once they dwarf the
        // visible elements, walking every run is `O(#runs)` and dominates each
        // reprojection. Above ~8× fragmentation, reading by visible position via the
        // end-dimension order statistics — `O(visible · log)` — is the cheaper path and
        // skips the tombstones entirely.
        if self.ctx.tree.run_count() > visible.saturating_mul(8).max(64) {
            return self.snapshot_range(log, 0..visible);
        }
        let mut out = Vec::with_capacity(visible);
        let mut entries = log.entries.focus();
        for run in self.ctx.tree.iter_runs() {
            if run.end != 0 {
                continue;
            }
            for off in 0..run.len {
                let lv = run.start_lv + off;
                let e = entries.index(lv);
                if let ListOp::Ins { item, .. } = &e.op {
                    out.push((e.dot, item.clone()));
                }
            }
        }
        out
    }

    /// Snapshot only the visible elements in document-position range `[start,end)`.
    /// O((end-start) · log) via end-dimension order-statistics — does not scan the
    /// whole sequence.
    pub fn snapshot_range<P: Clone>(
        &self,
        log: &OpLog<P>,
        range: std::ops::Range<usize>,
    ) -> Vec<(Dot, P)> {
        range
            .filter_map(|pos| {
                let lv = self.ctx.tree.end_pos_to_lv(pos)?;
                let e = &log.entries[lv];
                match &e.op {
                    ListOp::Ins { item, .. } => Some((e.dot, item.clone())),
                    _ => None,
                }
            })
            .collect()
    }

    pub fn iter_visible<'a, P: Clone>(
        &'a self,
        log: &'a OpLog<P>,
    ) -> impl Iterator<Item = (Dot, &'a P)> + 'a {
        self.ctx
            .tree
            .iter_runs()
            .filter(|r| r.end == 0)
            .flat_map(move |r| {
                (0..r.len).filter_map(move |off| {
                    let lv = r.start_lv + off;
                    let e = &log.entries[lv];
                    match &e.op {
                        ListOp::Ins { item, .. } => Some((e.dot, item)),
                        _ => None,
                    }
                })
            })
    }

    pub fn resolve_boundary(&self, id: Dot, bias: Bias) -> Option<Boundary> {
        resolve_boundary_in(&self.ctx.tree, &self.lv_of, id, bias)
    }

    pub fn resolve_boundary_checked(&self, id: Dot, bias: Bias) -> Option<Boundary> {
        resolve_boundary_checked_in(&self.ctx.tree, &self.lv_of, id, bias)
    }

    /// Returns the canonical active delete's parent-version gap for `target`.
    ///
    /// Returns `None` when `target` is not a sequence insertion or has no
    /// active delete.
    pub fn deletion_gap(&self, target: Dot) -> Option<DeletionGap> {
        self.ctx
            .deletions
            .gap_for_target(&self.ctx.tree, &self.lv_of, target)
    }

    pub fn doc_index_of(&self, dot: Dot) -> Option<usize> {
        let lv = *self.lv_of.get(&dot)?;
        self.ctx.tree.doc_index_of_lv_checked(lv)
    }

    pub fn del_target_positions(&self, del: Dot) -> Vec<usize> {
        del_target_positions_in(&self.ctx.tree, &self.lv_of, &self.ctx.del_targets, del)
    }

    pub fn dot_at_visible<P: Clone>(&self, log: &OpLog<P>, pos: usize) -> Option<Dot> {
        let lv = self.ctx.tree.end_pos_to_lv(pos)?;
        Some(log.entries[lv].dot)
    }

    /// Dots of the invisible (tombstone) items sitting strictly between visible
    /// position `pos` and the next visible element. Anchors on these ghosts
    /// resolve to boundaries inside that gap, so a caller reasoning about span
    /// boundaries around `pos` must consider them. O(ghosts · log n).
    pub fn invisible_dots_after_visible(&self, pos: usize) -> Vec<Dot> {
        let Some(lv) = self.ctx.tree.end_pos_to_lv(pos) else {
            return Vec::new();
        };
        let mut i = self.ctx.tree.doc_index_of_lv(lv) + 1;
        let total = self.ctx.tree.len();
        let mut out = Vec::new();
        while i < total {
            let (run, off) = self.ctx.tree.get(i);
            if run.end == 0 {
                break;
            }
            out.push(Dot::new(run.start.actor, run.start.clock + off as u64));
            i += 1;
        }
        out
    }

    pub fn del_target_dots<P: Clone>(&self, log: &OpLog<P>, del: Dot) -> Vec<Dot> {
        let Some(&del_lv) = self.lv_of.get(&del) else {
            return Vec::new();
        };
        let Some(targets) = self.ctx.del_targets.get(del_lv) else {
            return Vec::new();
        };
        targets.iter().map(|&lv| log.entries[lv].dot).collect()
    }

    /// Number of targets a delete op removed, without materializing their dots. Lets a
    /// bulk-delete fast path decide on size in `O(1)` instead of building an `O(len)`
    /// dot vector just to read its length.
    pub fn del_target_count(&self, del: Dot) -> usize {
        self.lv_of
            .get(&del)
            .and_then(|&del_lv| self.ctx.del_targets.get(del_lv))
            .map_or(0, |targets| targets.len())
    }

    pub fn into_resolver(self) -> BoundaryResolver {
        BoundaryResolver {
            index: ResolveIndex {
                tree: self.ctx.tree,
                lv_of: self.lv_of,
                del_targets: self.ctx.del_targets,
                deletions: self.ctx.deletions,
            },
        }
    }
}

pub fn checkout<P: Clone>(log: &OpLog<P>) -> Vec<(Dot, P)> {
    let mut c = SeqCheckout::new();
    c.apply_tail(log);
    c.snapshot(log)
}

#[cfg(test)]
pub(crate) fn checkout_text(log: &OpLog<char>) -> String {
    checkout(log).into_iter().map(|(_id, c)| c).collect()
}

pub(crate) fn checkout_with_index<P: Clone>(log: &OpLog<P>) -> (Vec<(Dot, P)>, ResolveIndex) {
    let mut c = SeqCheckout::new();
    c.apply_tail(log);
    let snap = c.snapshot(log);
    let index = ResolveIndex {
        tree: c.ctx.tree,
        lv_of: c.lv_of,
        del_targets: c.ctx.del_targets,
        deletions: c.ctx.deletions,
    };
    (snap, index)
}

pub(crate) struct ResolveIndex {
    tree: ContentTree<Run>,
    lv_of: crate::DotMap<usize>,
    del_targets: imbl::Vector<Vec<usize>>,
    deletions: DeletionIndex,
}

fn resolve_boundary_in(
    tree: &ContentTree<Run>,
    lv_of: &crate::DotMap<usize>,
    id: Dot,
    bias: Bias,
) -> Option<Boundary> {
    let lv = *lv_of.get(&id)?;
    let doc_idx = tree.doc_index_of_lv(lv);
    let (run, _off) = tree.get(doc_idx);
    let visible = run.end == 0;
    let r = tree.end_rank_at_doc_index(doc_idx);
    let position = match bias {
        Bias::Before => r,
        Bias::After => r + usize::from(visible),
    };
    Some(Boundary { position, visible })
}

/// `resolve_boundary`, but total: `None` (never a panic or a garbage index) when
/// `id` is not a sequence *element* — a `Del`/`Undel` op's own dot is in `lv_of`
/// but occupies no content run, and an unknown dot is in neither.
fn resolve_boundary_checked_in(
    tree: &ContentTree<Run>,
    lv_of: &crate::DotMap<usize>,
    id: Dot,
    bias: Bias,
) -> Option<Boundary> {
    let lv = *lv_of.get(&id)?;
    let doc_idx = tree.doc_index_of_lv_checked(lv)?;
    let (run, _off) = tree.get(doc_idx);
    let visible = run.end == 0;
    let r = tree.end_rank_at_doc_index(doc_idx);
    let position = match bias {
        Bias::Before => r,
        Bias::After => r + usize::from(visible),
    };
    Some(Boundary { position, visible })
}

fn del_target_positions_in(
    tree: &ContentTree<Run>,
    lv_of: &crate::DotMap<usize>,
    del_targets: &imbl::Vector<Vec<usize>>,
    del: Dot,
) -> Vec<usize> {
    let Some(del_lv) = lv_of.get(&del).copied() else {
        return Vec::new();
    };
    let Some(targets) = del_targets.get(del_lv) else {
        return Vec::new();
    };
    let mut positions: Vec<usize> = Vec::new();
    for &t in targets {
        let doc_idx = tree.doc_index_of_lv(t);
        let (run, _off) = tree.get(doc_idx);
        if run.end == 0 {
            positions.push(tree.end_rank_at_doc_index(doc_idx));
        }
    }
    positions.sort_unstable_by(|a, b| b.cmp(a));
    positions
}

impl ResolveIndex {
    fn del_target_positions(&self, del: Dot) -> Vec<usize> {
        del_target_positions_in(&self.tree, &self.lv_of, &self.del_targets, del)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Boundary {
    pub position: usize,
    pub visible: bool,
}

pub struct BoundaryResolver {
    index: ResolveIndex,
}

impl BoundaryResolver {
    pub fn resolve_boundary(&self, id: Dot, bias: Bias) -> Option<Boundary> {
        resolve_boundary_in(&self.index.tree, &self.index.lv_of, id, bias)
    }

    pub fn resolve_boundary_checked(&self, id: Dot, bias: Bias) -> Option<Boundary> {
        resolve_boundary_checked_in(&self.index.tree, &self.index.lv_of, id, bias)
    }

    /// Returns the canonical active delete's parent-version gap for `target`.
    ///
    /// Returns `None` when `target` is not a sequence insertion or has no
    /// active delete.
    pub fn deletion_gap(&self, target: Dot) -> Option<DeletionGap> {
        self.index
            .deletions
            .gap_for_target(&self.index.tree, &self.index.lv_of, target)
    }

    /// Descending current visible positions of `del`'s still-visible targets,
    /// for redoing a deletion. See [`ResolveIndex::del_target_positions`].
    pub fn del_target_positions(&self, del: Dot) -> Vec<usize> {
        self.index.del_target_positions(del)
    }
}

pub trait SeqResolve {
    fn resolve_boundary(&self, id: Dot, bias: Bias) -> Option<Boundary>;
}

impl SeqResolve for BoundaryResolver {
    fn resolve_boundary(&self, id: Dot, bias: Bias) -> Option<Boundary> {
        BoundaryResolver::resolve_boundary(self, id, bias)
    }
}

impl SeqResolve for SeqCheckout {
    fn resolve_boundary(&self, id: Dot, bias: Bias) -> Option<Boundary> {
        SeqCheckout::resolve_boundary(self, id, bias)
    }
}

impl<T: SeqResolve + ?Sized> SeqResolve for &T {
    fn resolve_boundary(&self, id: Dot, bias: Bias) -> Option<Boundary> {
        (**self).resolve_boundary(id, bias)
    }
}

pub fn checkout_with_resolver<P: Clone>(log: &OpLog<P>) -> (Vec<(Dot, P)>, BoundaryResolver) {
    let (elems, index) = checkout_with_index(log);
    (elems, BoundaryResolver { index })
}

#[cfg(test)]
pub(crate) fn checkout_runs(log: &OpLog) -> (String, usize, usize) {
    let mut c = SeqCheckout::new();
    c.apply_tail(log);
    let elems = c.ctx.tree.len();
    let runs = c.ctx.tree.run_count();
    (
        c.snapshot(log).into_iter().map(|(_id, ch)| ch).collect(),
        elems,
        runs,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dot;
    use crate::oplog::{InputEvent, build_oplog};

    fn ins(a: u64, c: u64, p: &[Dot], pos: usize, ch: char) -> InputEvent {
        InputEvent {
            id: Dot::new(a, c),
            parents: p.to_vec(),
            op: ListOp::Ins { pos, item: ch },
        }
    }
    fn del_at(a: u64, c: u64, p: &[Dot], pos: usize) -> InputEvent {
        InputEvent {
            id: Dot::new(a, c),
            parents: p.to_vec(),
            op: ListOp::Del { pos, len: 1 },
        }
    }

    fn del_range(a: u64, c: u64, p: &[Dot], pos: usize, len: usize) -> InputEvent {
        InputEvent {
            id: Dot::new(a, c),
            parents: p.to_vec(),
            op: ListOp::Del { pos, len },
        }
    }

    fn undel(a: u64, c: u64, p: &[Dot], del: Dot) -> InputEvent {
        InputEvent {
            id: Dot::new(a, c),
            parents: p.to_vec(),
            op: ListOp::Undel { del },
        }
    }

    fn doc(ev: &[InputEvent]) -> String {
        checkout_text(&build_oplog(ev))
    }

    #[test]
    fn range_delete_three_middle_seq() {
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let c = Dot::new(1, 2);
        let d = Dot::new(1, 3);
        let e = Dot::new(1, 4);
        let ev = vec![
            ins(1, 0, &[], 0, 'a'),
            ins(1, 1, &[a], 1, 'b'),
            ins(1, 2, &[b], 2, 'c'),
            ins(1, 3, &[c], 3, 'd'),
            ins(1, 4, &[d], 4, 'e'),
            del_range(1, 5, &[e], 1, 3),
        ];
        assert_eq!(doc(&ev), "ae");
    }

    fn abc_del_b_resolver() -> (BoundaryResolver, Dot, Dot, Dot) {
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let c = Dot::new(1, 2);
        let ev = vec![
            ins(1, 0, &[], 0, 'a'),
            ins(1, 1, &[a], 1, 'b'),
            ins(1, 2, &[b], 2, 'c'),
            del_at(1, 3, &[c], 1),
        ];
        let (elems, resolver) = checkout_with_resolver(&build_oplog(&ev));
        let doc: String = elems.into_iter().map(|(_id, ch)| ch).collect();
        assert_eq!(doc, "ac");
        (resolver, a, b, c)
    }

    #[test]
    fn boundary_visible_before_is_rank_after_is_rank_plus_one() {
        let (r, a, _b, c) = abc_del_b_resolver();
        assert_eq!(r.resolve_boundary(a, Bias::Before).unwrap().position, 0);
        assert_eq!(r.resolve_boundary(a, Bias::After).unwrap().position, 1);
        assert_eq!(r.resolve_boundary(c, Bias::Before).unwrap().position, 1);
        assert_eq!(r.resolve_boundary(c, Bias::After).unwrap().position, 2);
        assert!(r.resolve_boundary(a, Bias::Before).unwrap().visible);
    }

    #[test]
    fn boundary_tombstone_collapses_both_biases_and_marks_invisible() {
        let (r, _a, b, _c) = abc_del_b_resolver();
        let before = r.resolve_boundary(b, Bias::Before).unwrap();
        let after = r.resolve_boundary(b, Bias::After).unwrap();
        assert_eq!(before.position, 1);
        assert_eq!(after.position, 1);
        assert!(!before.visible && !after.visible);
    }

    #[test]
    fn boundary_missing_dot_is_none() {
        let (r, _a, _b, _c) = abc_del_b_resolver();
        assert!(r.resolve_boundary(Dot::new(9, 9), Bias::After).is_none());
    }

    #[test]
    fn deletion_gap_records_the_delete_parent_version_for_every_target() {
        let a = Dot::new(1, 0);
        let z = Dot::new(1, 1);
        let b = Dot::new(1, 2);
        let c = Dot::new(1, 3);
        let d = Dot::new(1, 4);
        let del_bc = Dot::new(1, 5);
        let later = Dot::new(1, 6);
        let ev = vec![
            ins(1, 0, &[], 0, 'a'),
            ins(1, 1, &[a], 1, 'Z'),
            ins(1, 2, &[z], 2, 'b'),
            ins(1, 3, &[b], 3, 'c'),
            ins(1, 4, &[c], 4, 'd'),
            del_range(1, 5, &[d], 2, 2),
            ins(1, 6, &[del_bc], 2, 'X'),
        ];
        let log = build_oplog(&ev);
        let expected = Some(DeletionGap {
            left: Some(z),
            right: Some(d),
        });
        let (_elems, cold) = checkout_with_resolver(&log);
        let mut warm = SeqCheckout::new();
        warm.apply_tail(&log);

        assert_eq!(warm.deletion_gap(b), expected);
        assert_eq!(warm.deletion_gap(c), expected);
        assert_eq!(cold.deletion_gap(b), expected);
        assert_eq!(cold.deletion_gap(c), expected);
        assert_ne!(warm.deletion_gap(b).unwrap().left, Some(later));
        assert_eq!(warm.deletion_gap(a), None);
        assert_eq!(cold.deletion_gap(del_bc), None);
    }

    #[derive(Clone, Copy)]
    struct OverlapDots {
        target: Dot,
        base_left: Dot,
        base_right: Dot,
        a_insert: Dot,
        a_delete: Dot,
        b_delete: Dot,
        a_undel: Dot,
        a_insert_2: Dot,
        a_redo: Dot,
    }

    fn overlapping_delete_log(a_branch_first: bool) -> (OpLog<char>, OverlapDots) {
        let mut log = OpLog::new();
        let chars = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i'];
        for (clock, ch) in chars.into_iter().enumerate() {
            let parents = if clock == 0 {
                Vec::new()
            } else {
                vec![Dot::new(0, clock as u64 - 1)]
            };
            log.push(ins(0, clock as u64, &parents, clock, ch));
        }

        let base_head = Dot::new(0, 8);
        let dots = OverlapDots {
            target: Dot::new(0, 4),
            base_left: Dot::new(0, 3),
            base_right: Dot::new(0, 5),
            a_insert: Dot::new(1, 9),
            a_delete: Dot::new(1, 10),
            b_delete: Dot::new(2, 9),
            a_undel: Dot::new(1, 11),
            a_insert_2: Dot::new(1, 12),
            a_redo: Dot::new(1, 13),
        };
        let a_insert = ins(1, 9, &[base_head], 4, 'Z');
        let a_delete = del_at(1, 10, &[dots.a_insert], 5);
        let b_delete = del_at(2, 9, &[base_head], 4);
        if a_branch_first {
            log.push(a_insert);
            log.push(a_delete);
            log.push(b_delete);
        } else {
            log.push(b_delete);
            log.push(a_insert);
            log.push(a_delete);
        }
        (log, dots)
    }

    fn append_a_undel(log: &mut OpLog<char>, dots: OverlapDots) {
        log.push(undel(
            dots.a_undel.actor,
            dots.a_undel.clock,
            &[dots.a_delete],
            dots.a_delete,
        ));
    }

    fn append_a_redo(log: &mut OpLog<char>, dots: OverlapDots) {
        log.push(ins(
            dots.a_insert_2.actor,
            dots.a_insert_2.clock,
            &[dots.a_undel],
            5,
            'W',
        ));
        log.push(del_at(
            dots.a_redo.actor,
            dots.a_redo.clock,
            &[dots.a_insert_2],
            6,
        ));
    }

    #[test]
    fn deletion_gap_uses_actor_first_active_delete_across_local_lv_orders() {
        let (mut log_a_first, dots) = overlapping_delete_log(true);
        let (mut log_b_first, other_dots) = overlapping_delete_log(false);
        assert_eq!(dots.target, other_dots.target);
        assert!(
            dots.a_delete > dots.b_delete,
            "Dot::Ord must prefer B so this fixture catches clock-first comparison"
        );

        let a_gap = Some(DeletionGap {
            left: Some(dots.a_insert),
            right: Some(dots.base_right),
        });
        let b_gap = Some(DeletionGap {
            left: Some(dots.base_left),
            right: Some(dots.base_right),
        });
        let redo_gap = Some(DeletionGap {
            left: Some(dots.a_insert_2),
            right: Some(dots.base_right),
        });

        let mut warm_a_first = SeqCheckout::new();
        warm_a_first.apply_tail(&log_a_first);
        let mut warm_b_first = SeqCheckout::new();
        warm_b_first.apply_tail(&log_b_first);
        assert_eq!(warm_a_first.deletion_gap(dots.target), a_gap);
        assert_eq!(warm_b_first.deletion_gap(dots.target), a_gap);
        assert_eq!(
            checkout_with_resolver(&log_a_first)
                .1
                .deletion_gap(dots.target),
            a_gap
        );
        assert_eq!(
            checkout_with_resolver(&log_b_first)
                .1
                .deletion_gap(dots.target),
            a_gap
        );

        append_a_undel(&mut log_a_first, dots);
        append_a_undel(&mut log_b_first, dots);
        warm_a_first.apply_tail(&log_a_first);
        warm_b_first.apply_tail(&log_b_first);
        assert_eq!(warm_a_first.deletion_gap(dots.target), b_gap);
        assert_eq!(warm_b_first.deletion_gap(dots.target), b_gap);
        assert_eq!(
            checkout_with_resolver(&log_a_first)
                .1
                .deletion_gap(dots.target),
            b_gap
        );
        assert_eq!(
            checkout_with_resolver(&log_b_first)
                .1
                .deletion_gap(dots.target),
            b_gap
        );

        append_a_redo(&mut log_a_first, dots);
        append_a_redo(&mut log_b_first, dots);
        warm_a_first.apply_tail(&log_a_first);
        warm_b_first.apply_tail(&log_b_first);
        assert_eq!(warm_a_first.deletion_gap(dots.target), redo_gap);
        assert_eq!(warm_b_first.deletion_gap(dots.target), redo_gap);
        assert_eq!(
            checkout_with_resolver(&log_a_first)
                .1
                .deletion_gap(dots.target),
            redo_gap
        );
        assert_eq!(
            checkout_with_resolver(&log_b_first)
                .1
                .deletion_gap(dots.target),
            redo_gap
        );
        assert_eq!(warm_a_first.deletion_gap(dots.a_delete), None);
        assert_eq!(warm_a_first.deletion_gap(dots.a_undel), None);
    }

    #[test]
    fn deletion_gap_handles_document_edges_zero_length_and_deleted_survivors() {
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let c = Dot::new(1, 2);
        let d = Dot::new(1, 3);
        let del_d = Dot::new(1, 4);
        let del_c = Dot::new(1, 5);
        let del_a = Dot::new(1, 6);
        let zero = Dot::new(1, 7);
        let ev = vec![
            ins(1, 0, &[], 0, 'a'),
            ins(1, 1, &[a], 1, 'b'),
            ins(1, 2, &[b], 2, 'c'),
            ins(1, 3, &[c], 3, 'd'),
            del_at(1, 4, &[d], 3),
            del_at(1, 5, &[del_d], 2),
            del_at(1, 6, &[del_c], 0),
            del_range(1, 7, &[del_a], 0, 0),
        ];
        let log = build_oplog(&ev);
        let mut warm = SeqCheckout::new();
        warm.apply_tail(&log);
        let cold = checkout_with_resolver(&log).1;

        assert_eq!(
            warm.deletion_gap(d),
            Some(DeletionGap {
                left: Some(c),
                right: None,
            })
        );
        assert_eq!(
            warm.deletion_gap(c),
            Some(DeletionGap {
                left: Some(b),
                right: None,
            })
        );
        assert_eq!(
            warm.deletion_gap(a),
            Some(DeletionGap {
                left: None,
                right: Some(b),
            })
        );
        assert_eq!(cold.deletion_gap(d), warm.deletion_gap(d));
        assert_eq!(cold.deletion_gap(c), warm.deletion_gap(c));
        assert_eq!(cold.deletion_gap(a), warm.deletion_gap(a));
        assert_eq!(warm.deletion_gap(b), None);
        assert_eq!(warm.deletion_gap(zero), None);
        assert_eq!(warm.deletion_gap(Dot::new(9, 9)), None);
        assert_eq!(warm.deletion_gap(Dot::synthetic(1)), None);
    }

    #[test]
    fn linear_insert_delete() {
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let c = Dot::new(1, 2);
        let ev = vec![
            ins(1, 0, &[], 0, 'h'),
            ins(1, 1, &[a], 1, 'i'),
            ins(1, 2, &[b], 2, '!'),
            InputEvent {
                id: Dot::new(1, 3),
                parents: vec![c],
                op: ListOp::Del { pos: 1, len: 1 },
            },
        ];
        assert_eq!(doc(&ev), "h!");
    }

    #[test]
    fn concurrent_forward_words() {
        let a1 = Dot::new(1, 0);
        let a2 = Dot::new(2, 0);
        let ev = vec![
            ins(1, 0, &[], 0, 'a'),
            InputEvent {
                id: Dot::new(1, 1),
                parents: vec![a1],
                op: ListOp::Ins { pos: 1, item: 'b' },
            },
            ins(2, 0, &[], 0, 'x'),
            InputEvent {
                id: Dot::new(2, 1),
                parents: vec![a2],
                op: ListOp::Ins { pos: 1, item: 'y' },
            },
        ];
        let s = doc(&ev);
        assert!(s == "abxy" || s == "xyab", "interleaved: {s}");
    }

    #[test]
    fn concurrent_backward_words() {
        let z = Dot::new(9, 0);
        let ev = vec![
            ins(9, 0, &[], 0, 'Z'),
            ins(1, 0, &[z], 0, 'A'),
            InputEvent {
                id: Dot::new(1, 1),
                parents: vec![Dot::new(1, 0)],
                op: ListOp::Ins { pos: 0, item: 'B' },
            },
            ins(2, 0, &[z], 0, 'X'),
            InputEvent {
                id: Dot::new(2, 1),
                parents: vec![Dot::new(2, 0)],
                op: ListOp::Ins { pos: 0, item: 'Y' },
            },
        ];
        let s = doc(&ev);
        assert!(s == "BAYXZ" || s == "YXBAZ", "interleaved: {s}");
    }

    #[test]
    fn single_actor_forward_is_one_run() {
        let mut ev = Vec::new();
        for c in 0..500u64 {
            let parents = if c == 0 {
                vec![]
            } else {
                vec![Dot::new(7, c - 1)]
            };
            ev.push(InputEvent {
                id: Dot::new(7, c),
                parents,
                op: ListOp::Ins {
                    pos: c as usize,
                    item: 'a',
                },
            });
        }
        let (doc, elems, runs) = checkout_runs(&build_oplog(&ev));
        assert_eq!(doc.chars().count(), 500);
        assert_eq!(elems, 500);
        assert_eq!(runs, 1, "forward typing must collapse to a single run");
    }

    #[test]
    fn forward_with_few_deletes_stays_low_run_count() {
        let mut ev = Vec::new();
        for c in 0..500u64 {
            let parents = if c == 0 {
                vec![]
            } else {
                vec![Dot::new(7, c - 1)]
            };
            ev.push(InputEvent {
                id: Dot::new(7, c),
                parents,
                op: ListOp::Ins {
                    pos: c as usize,
                    item: 'a',
                },
            });
        }
        for (clock, pos) in (500u64..).zip([400usize, 300, 200, 100]) {
            ev.push(InputEvent {
                id: Dot::new(7, clock),
                parents: vec![Dot::new(7, clock - 1)],
                op: ListOp::Del { pos, len: 1 },
            });
        }
        let (_doc, elems, runs) = checkout_runs(&build_oplog(&ev));
        assert_eq!(elems, 500);
        assert!(
            runs <= 16,
            "expected few runs after sparse deletes, got {runs}"
        );
    }

    #[test]
    fn undel_restores_range_delete_seq() {
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let c = Dot::new(1, 2);
        let d = Dot::new(1, 3);
        let e = Dot::new(1, 4);
        let del = Dot::new(1, 5);
        let ev = vec![
            ins(1, 0, &[], 0, 'a'),
            ins(1, 1, &[a], 1, 'b'),
            ins(1, 2, &[b], 2, 'c'),
            ins(1, 3, &[c], 3, 'd'),
            ins(1, 4, &[d], 4, 'e'),
            del_range(1, 5, &[e], 1, 3),
            undel(1, 6, &[del], del),
        ];
        let log = build_oplog(&ev);
        assert_eq!(checkout_text(&log), "abcde");
    }

    #[test]
    fn undel_one_of_two_concurrent_deletes_seq() {
        let a = Dot::new(0, 0);
        let b = Dot::new(0, 1);
        let c = Dot::new(0, 2);
        let del_a = Dot::new(1, 0);
        let ev = vec![
            ins(0, 0, &[], 0, 'a'),
            ins(0, 1, &[a], 1, 'b'),
            ins(0, 2, &[b], 2, 'c'),
            del_range(1, 0, &[c], 1, 1),
            del_range(2, 0, &[c], 1, 1),
            undel(1, 1, &[del_a], del_a),
        ];
        let log = build_oplog(&ev);
        assert_eq!(checkout_text(&log), "ac");
    }

    #[test]
    fn concurrent_edit_with_undel_seq() {
        let a = Dot::new(0, 0);
        let b = Dot::new(0, 1);
        let c = Dot::new(0, 2);
        let del = Dot::new(1, 0);
        let ev = vec![
            ins(0, 0, &[], 0, 'a'),
            ins(0, 1, &[a], 1, 'b'),
            ins(0, 2, &[b], 2, 'c'),
            del_range(1, 0, &[c], 1, 1),
            undel(1, 1, &[del], del),
            InputEvent {
                id: Dot::new(2, 0),
                parents: vec![c],
                op: ListOp::Ins { pos: 1, item: 'X' },
            },
        ];
        let log = build_oplog(&ev);
        assert_eq!(checkout_text(&log), "aXbc");
    }

    #[test]
    fn range_delete_crosses_existing_tombstone_seq() {
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let c = Dot::new(1, 2);
        let d1 = Dot::new(1, 3);
        let ev = vec![
            ins(1, 0, &[], 0, 'a'),
            ins(1, 1, &[a], 1, 'b'),
            ins(1, 2, &[b], 2, 'c'),
            del_range(1, 3, &[c], 1, 1),
            del_range(1, 4, &[d1], 0, 2),
        ];
        assert_eq!(doc(&ev), "");
    }

    #[test]
    fn concurrent_range_delete_two_overlapping() {
        let a = Dot::new(0, 0);
        let b = Dot::new(0, 1);
        let c = Dot::new(0, 2);
        let d = Dot::new(0, 3);
        let e = Dot::new(0, 4);
        let ev = vec![
            ins(0, 0, &[], 0, 'a'),
            ins(0, 1, &[a], 1, 'b'),
            ins(0, 2, &[b], 2, 'c'),
            ins(0, 3, &[c], 3, 'd'),
            ins(0, 4, &[d], 4, 'e'),
            del_range(1, 0, &[e], 1, 3),
            del_range(2, 0, &[e], 2, 2),
        ];
        let log = build_oplog(&ev);
        assert_eq!(checkout_text(&log), "ae");
    }

    #[test]
    fn range_delete_large_prefix_clears_document() {
        let n = 4000u64;
        let mut ev = Vec::with_capacity(n as usize + 1);
        for c in 0..n {
            let parents = if c == 0 {
                vec![]
            } else {
                vec![Dot::new(7, c - 1)]
            };
            ev.push(InputEvent {
                id: Dot::new(7, c),
                parents,
                op: ListOp::Ins {
                    pos: c as usize,
                    item: 'a',
                },
            });
        }
        ev.push(del_range(7, n, &[Dot::new(7, n - 1)], 0, n as usize));
        let log = build_oplog(&ev);
        assert_eq!(checkout_text(&log), "");
    }

    #[test]
    fn concurrent_insert_inside_deleted_range() {
        let a = Dot::new(0, 0);
        let b = Dot::new(0, 1);
        let c = Dot::new(0, 2);
        let d = Dot::new(0, 3);
        let e = Dot::new(0, 4);
        let ev = vec![
            ins(0, 0, &[], 0, 'a'),
            ins(0, 1, &[a], 1, 'b'),
            ins(0, 2, &[b], 2, 'c'),
            ins(0, 3, &[c], 3, 'd'),
            ins(0, 4, &[d], 4, 'e'),
            del_range(1, 0, &[e], 1, 3),
            InputEvent {
                id: Dot::new(2, 0),
                parents: vec![e],
                op: ListOp::Ins { pos: 2, item: 'X' },
            },
        ];
        let log = build_oplog(&ev);
        assert_eq!(checkout_text(&log), "aXe");
    }

    #[test]
    fn undel_restores_correct_identity_with_duplicate_chars() {
        let a0 = Dot::new(1, 0);
        let x = Dot::new(1, 1);
        let a1 = Dot::new(1, 2);
        let del = Dot::new(1, 3);
        let ev = vec![
            ins(1, 0, &[], 0, 'a'),
            ins(1, 1, &[a0], 1, 'X'),
            ins(1, 2, &[x], 2, 'a'),
            del_range(1, 3, &[a1], 2, 1),
            undel(1, 4, &[del], del),
        ];
        let log = build_oplog(&ev);
        let got: Vec<(Dot, char)> = checkout(&log);
        assert_eq!(got, vec![(a0, 'a'), (x, 'X'), (a1, 'a')]);
    }

    #[test]
    fn undel_restores_two_noncontiguous_with_duplicate_chars() {
        let a0 = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let a1 = Dot::new(1, 2);
        let del_b = Dot::new(1, 3);
        let del_aa = Dot::new(1, 4);
        let ev = vec![
            ins(1, 0, &[], 0, 'a'),
            ins(1, 1, &[a0], 1, 'b'),
            ins(1, 2, &[b], 2, 'a'),
            del_range(1, 3, &[a1], 1, 1),
            del_range(1, 4, &[del_b], 0, 2),
            undel(1, 5, &[del_aa], del_aa),
        ];
        let log = build_oplog(&ev);
        let got: Vec<(Dot, char)> = checkout(&log);
        assert_eq!(got, vec![(a0, 'a'), (a1, 'a')]);
    }

    #[test]
    #[should_panic(expected = "undel underflow")]
    fn double_undel_panics() {
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let del = Dot::new(1, 2);
        let u1 = Dot::new(1, 3);
        let ev = vec![
            ins(1, 0, &[], 0, 'a'),
            ins(1, 1, &[a], 1, 'b'),
            del_range(1, 2, &[b], 1, 1),
            undel(1, 3, &[del], del),
            undel(1, 4, &[u1], del),
        ];
        let _ = checkout_text(&build_oplog(&ev));
    }

    #[test]
    fn seqcheckout_resolver_matches_cold() {
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let c = Dot::new(1, 2);
        let ev = vec![
            ins(1, 0, &[], 0, 'a'),
            ins(1, 1, &[a], 1, 'b'),
            ins(1, 2, &[b], 2, 'c'),
            del_at(1, 3, &[c], 1),
        ];
        let log = build_oplog(&ev);
        let (_cold_elems, cold) = checkout_with_resolver(&log);
        let mut warm = SeqCheckout::new();
        warm.apply_tail(&log);
        for (d, bias) in [
            (a, Bias::Before),
            (a, Bias::After),
            (b, Bias::Before),
            (c, Bias::After),
        ] {
            assert_eq!(
                warm.resolve_boundary(d, bias),
                cold.resolve_boundary(d, bias)
            );
        }
        let warm_doc: String = warm.iter_visible(&log).map(|(_d, ch)| *ch).collect();
        assert_eq!(warm_doc, "ac");
    }

    #[test]
    fn dot_at_visible_matches_snapshot_order() {
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let ev = vec![
            ins(1, 0, &[], 0, 'a'),
            ins(1, 1, &[a], 1, 'b'),
            ins(2, 0, &[a], 1, 'c'),
            ins(1, 2, &[b], 2, 'd'),
        ];
        let log = build_oplog(&ev);
        let mut sc = SeqCheckout::new();
        sc.apply_tail(&log);
        let snap = sc.snapshot(&log);
        for (i, (d, _)) in snap.iter().enumerate() {
            assert_eq!(sc.dot_at_visible(&log, i), Some(*d), "pos {i}");
        }
        assert_eq!(sc.dot_at_visible(&log, snap.len()), None);
    }

    #[test]
    fn doc_index_order_is_stable_under_insert_and_delete() {
        let a = Dot::new(1, 1);
        let b = Dot::new(1, 2);
        let c = Dot::new(1, 3);
        let mut ev = vec![
            InputEvent {
                id: a,
                parents: vec![],
                op: ListOp::Ins { pos: 0, item: 'a' },
            },
            InputEvent {
                id: b,
                parents: vec![a],
                op: ListOp::Ins { pos: 1, item: 'b' },
            },
            InputEvent {
                id: c,
                parents: vec![b],
                op: ListOp::Ins { pos: 2, item: 'c' },
            },
        ];
        let log = build_oplog(&ev);
        let mut co = SeqCheckout::new();
        co.apply_tail(&log);

        let ia = co.doc_index_of(a).unwrap();
        let ib = co.doc_index_of(b).unwrap();
        let ic = co.doc_index_of(c).unwrap();
        assert!(ia < ib && ib < ic);
        assert_eq!(co.doc_index_of(Dot::new(9, 9)), None);

        let mid = Dot::new(1, 4);
        ev.push(InputEvent {
            id: mid,
            parents: vec![c],
            op: ListOp::Ins { pos: 1, item: 'x' },
        });
        let del = Dot::new(1, 5);
        ev.push(InputEvent {
            id: del,
            parents: vec![mid],
            op: ListOp::Del { pos: 2, len: 1 },
        });
        let undel = Dot::new(1, 6);
        ev.push(InputEvent {
            id: undel,
            parents: vec![del],
            op: ListOp::Undel { del },
        });
        let log = build_oplog(&ev);
        let mut co = SeqCheckout::new();
        co.apply_tail(&log);

        let (ia2, im, ib2, ic2) = (
            co.doc_index_of(a).unwrap(),
            co.doc_index_of(mid).unwrap(),
            co.doc_index_of(b).unwrap(),
            co.doc_index_of(c).unwrap(),
        );
        assert!(
            ia2 < im && im < ib2 && ib2 < ic2,
            "insert shifts values, never order"
        );
        assert!(
            co.resolve_boundary(b, Bias::Before)
                .is_some_and(|r| r.visible),
            "b is visible again after the undelete"
        );
        assert_eq!(
            co.doc_index_of(del),
            None,
            "a Del op dot is in lv_of but is not a sequence element"
        );
        assert_eq!(
            co.doc_index_of(undel),
            None,
            "an Undel op dot is in lv_of but is not a sequence element"
        );
    }

    #[test]
    fn resolve_boundary_checked_is_total_over_all_dot_classes() {
        let a = Dot::new(1, 0);
        let b = Dot::new(1, 1);
        let c = Dot::new(1, 2);
        let del_b = Dot::new(1, 3);
        let del_c = Dot::new(1, 4);
        let undel_b = Dot::new(1, 5);
        let unknown = Dot::new(999, 999);
        let ev = vec![
            ins(1, 0, &[], 0, 'a'),
            ins(1, 1, &[a], 1, 'b'),
            ins(1, 2, &[b], 2, 'c'),
            del_at(1, 3, &[c], 1),
            del_at(1, 4, &[del_b], 1),
            undel(1, 5, &[del_c], del_b),
        ];
        let log = build_oplog(&ev);
        assert_eq!(doc(&ev), "ab");

        let mut co = SeqCheckout::new();
        co.apply_tail(&log);
        assert!(
            co.resolve_boundary_checked(a, Bias::Before)
                .unwrap()
                .visible
        );
        assert!(
            co.resolve_boundary_checked(b, Bias::Before)
                .unwrap()
                .visible
        );
        assert!(
            !co.resolve_boundary_checked(c, Bias::Before)
                .unwrap()
                .visible
        );
        assert_eq!(co.resolve_boundary_checked(del_b, Bias::Before), None);
        assert_eq!(co.resolve_boundary_checked(del_c, Bias::Before), None);
        assert_eq!(co.resolve_boundary_checked(undel_b, Bias::Before), None);
        assert_eq!(co.resolve_boundary_checked(unknown, Bias::Before), None);

        let (_elems, resolver) = checkout_with_resolver(&log);
        assert!(
            resolver
                .resolve_boundary_checked(a, Bias::Before)
                .unwrap()
                .visible
        );
        assert!(
            resolver
                .resolve_boundary_checked(b, Bias::Before)
                .unwrap()
                .visible
        );
        assert!(
            !resolver
                .resolve_boundary_checked(c, Bias::Before)
                .unwrap()
                .visible
        );
        assert_eq!(resolver.resolve_boundary_checked(del_b, Bias::Before), None);
        assert_eq!(resolver.resolve_boundary_checked(del_c, Bias::Before), None);
        assert_eq!(
            resolver.resolve_boundary_checked(undel_b, Bias::Before),
            None
        );
        assert_eq!(
            resolver.resolve_boundary_checked(unknown, Bias::Before),
            None
        );
    }

    use hashbrown::HashSet;
    use proptest::prelude::*;
    use std::collections::BTreeMap;

    fn sub(events: &[InputEvent], parents: &[Dot]) -> Vec<InputEvent> {
        let map: HashMap<Dot, &InputEvent> = events.iter().map(|e| (e.id, e)).collect();
        let mut anc: HashSet<Dot> = HashSet::new();
        let mut st = parents.to_vec();
        while let Some(d) = st.pop() {
            if anc.insert(d)
                && let Some(e) = map.get(&d)
            {
                st.extend(e.parents.iter().copied());
            }
        }
        events
            .iter()
            .filter(|e| anc.contains(&e.id))
            .cloned()
            .collect()
    }

    fn build_undel(raw: Vec<(u64, u8, u8, u8, char, bool)>) -> Vec<InputEvent> {
        let mut clock: HashMap<u64, u64> = HashMap::new();
        let mut front: BTreeMap<u64, Dot> = BTreeMap::new();
        let mut del_authored: HashMap<u64, Vec<Dot>> = HashMap::new();
        let mut undeled: HashSet<Dot> = HashSet::new();
        let mut out: Vec<InputEvent> = Vec::new();
        for (actor, action, target, del_len, ch, sync_other) in raw {
            let cl = *clock.get(&actor).unwrap_or(&0);
            clock.insert(actor, cl + 1);
            let id = Dot::new(actor, cl);
            let mut parents: Vec<Dot> = Vec::new();
            if let Some(f) = front.get(&actor) {
                parents.push(*f);
            }
            if sync_other && let Some((_, f)) = front.iter().find(|(act, _)| **act != actor) {
                parents.push(*f);
            }
            let vis = checkout_text(&build_oplog(&sub(&out, &parents)))
                .chars()
                .count();
            let candidates: Vec<Dot> = del_authored
                .get(&actor)
                .map(|v| v.iter().copied().filter(|d| !undeled.contains(d)).collect())
                .unwrap_or_default();
            let op = if action % 3 == 2 && !candidates.is_empty() {
                let del = candidates[(target as usize) % candidates.len()];
                undeled.insert(del);
                ListOp::Undel { del }
            } else if action % 3 == 1 && vis > 0 {
                let pos = (target as usize) % vis;
                let len = 1 + (del_len as usize) % (vis - pos);
                del_authored.entry(actor).or_default().push(id);
                ListOp::Del { pos, len }
            } else {
                ListOp::Ins {
                    pos: (target as usize) % (vis + 1),
                    item: ch,
                }
            };
            out.push(InputEvent { id, parents, op });
            front.insert(actor, id);
        }
        out
    }

    fn arb_events_undel(max: usize, actors: u64) -> impl Strategy<Value = Vec<InputEvent>> {
        proptest::collection::vec(
            (
                0u64..actors,
                any::<u8>(),
                any::<u8>(),
                any::<u8>(),
                any::<char>(),
                any::<bool>(),
            ),
            0..=max,
        )
        .prop_map(build_undel)
    }

    fn chunks(len: usize, sizes: &[usize]) -> Vec<std::ops::Range<usize>> {
        let mut out = Vec::new();
        let mut start = 0usize;
        let mut i = 0usize;
        while start < len {
            let step = (sizes.get(i).copied().unwrap_or(1) % len.max(1)).max(1);
            let end = (start + step).min(len);
            out.push(start..end);
            start = end;
            i += 1;
        }
        out
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 384, ..ProptestConfig::default() })]
        #[test]
        fn warm_matches_cold_for_any_chunking(
            events in arb_events_undel(40, 3),
            sizes in proptest::collection::vec(1usize..7, 0..40),
        ) {
            let log = build_oplog(&events);
            let cold = checkout(&log);
            let (_cold_elems, cold_res) = checkout_with_resolver(&log);

            let mut warm = SeqCheckout::new();
            for r in chunks(log.entries.len(), &sizes) {
                warm.apply_range(&log, r);
            }
            prop_assert_eq!(warm.snapshot(&log), cold.clone());
            prop_assert_eq!(warm.visible_len(), cold.len());
            for &(dot, _) in &cold {
                for bias in [Bias::Before, Bias::After] {
                    prop_assert_eq!(
                        warm.resolve_boundary(dot, bias),
                        cold_res.resolve_boundary(dot, bias)
                    );
                }
            }
        }

        #[test]
        fn chunking_is_independent(events in arb_events_undel(40, 3)) {
            let log = build_oplog(&events);
            let mut one = SeqCheckout::new();
            one.apply_tail(&log);
            let mut many = SeqCheckout::new();
            for lv in 0..log.entries.len() {
                many.apply_range(&log, lv..lv + 1);
            }
            prop_assert_eq!(one.snapshot(&log), many.snapshot(&log));
        }
    }

    #[test]
    fn run_compaction_characterization() {
        let mut ev = Vec::new();
        for c in 0..200u64 {
            let parents = if c == 0 {
                vec![]
            } else {
                vec![Dot::new(7, c - 1)]
            };
            ev.push(ins(7, c, &parents, c as usize, 'a'));
        }
        let log = build_oplog(&ev);
        let mut warm = SeqCheckout::new();
        warm.apply_tail(&log);
        assert_eq!(warm.snapshot(&log).len(), 200);
        assert_eq!(
            warm.ctx.tree.run_count(),
            1,
            "forward typing must stay one run"
        );
    }
}
