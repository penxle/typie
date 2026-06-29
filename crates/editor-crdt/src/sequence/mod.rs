use crate::Dot;
use crate::oplog::{ListOp, NYI, OpLog, item_width, lv_cmp};
use editor_common::content_tree::{ContentTree, Cursor, Leaf, Sum};
use std::collections::BinaryHeap;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bias {
    Before,
    After,
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

struct Ctx {
    tree: ContentTree<Run>,
    del_targets: Vec<Vec<usize>>,
    cur_version: Vec<usize>,
}

fn advance1<P>(ctx: &mut Ctx, log: &OpLog<P>, lv: usize) {
    match log.ops[lv] {
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

fn retreat1<P>(ctx: &mut Ctx, log: &OpLog<P>, lv: usize) {
    match log.ops[lv] {
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

fn integrate<P>(tree: &ContentTree<Run>, log: &OpLog<P>, new_item: &Run, cursor: &mut Cursor) {
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

fn apply1<P: Clone>(ctx: &mut Ctx, snap: &mut Vec<(Dot, P)>, log: &OpLog<P>, lv: usize) {
    match log.ops[lv].clone() {
        ListOp::Del { pos, len } => {
            let visible = ctx.tree.cur_len();
            debug_assert!(
                pos <= visible && len <= visible - pos,
                "range delete out of bounds: pos={pos} len={len} visible={visible}"
            );
            let mut targets = Vec::with_capacity(len);
            let mut removals = Vec::with_capacity(len);
            if len > 0 {
                let mut c = ctx.tree.cursor_at_cur_pos(pos);
                for _ in 0..len {
                    loop {
                        let (cur_state, end_run) = {
                            let r = ctx.tree.cur_run(&c).expect("del target exists");
                            (r.cur, item_width(r.end))
                        };
                        if cur_state == 0 {
                            break;
                        }
                        let skipped = ctx.tree.step_run(&mut c);
                        c.end_pos += end_run * skipped;
                    }
                    let (target_lv, target_end_state) = {
                        let r = ctx.tree.cur_run(&c).expect("del target");
                        (r.op_id_at(c.off), r.end)
                    };
                    targets.push(target_lv);
                    if target_end_state == 0 {
                        removals.push(c.end_pos);
                    }
                    ctx.tree.step(&mut c);
                    c.end_pos += item_width(target_end_state);
                }
                for &target_lv in &targets {
                    ctx.tree.update_by_lv(target_lv, |it| {
                        it.cur += 1;
                        it.end += 1;
                    });
                }
                for &end_pos in removals.iter().rev() {
                    snap.remove(end_pos);
                }
            }
            ctx.del_targets[lv] = targets;
        }
        ListOp::Undel { del } => {
            let del_lv = *log.lv_of.get(&del).expect("undel references unknown del");
            let targets = ctx.del_targets[del_lv].clone();
            let mut reappear: Vec<usize> = Vec::new();
            for &target in &targets {
                let old_end = {
                    let (run, _) = ctx.tree.get(ctx.tree.doc_index_of_lv(target));
                    run.end
                };
                if old_end == 1 {
                    reappear.push(target);
                }
                ctx.tree.update_by_lv(target, |it| {
                    assert!(it.cur >= 1 && it.end >= 1, "undel underflow");
                    it.cur -= 1;
                    it.end -= 1;
                });
            }
            let mut inserts: Vec<(usize, Dot, P)> = reappear
                .iter()
                .map(|&t| {
                    let rank = ctx.tree.end_rank_at_doc_index(ctx.tree.doc_index_of_lv(t));
                    let item = match log.ops[t].clone() {
                        ListOp::Ins { item, .. } => item,
                        _ => unreachable!("undel target must be an Ins element"),
                    };
                    (rank, log.dots[t], item)
                })
                .collect();
            inserts.sort_by_key(|&(r, _, _)| r);
            for (rank, dot, item) in inserts {
                snap.insert(rank, (dot, item));
            }
            ctx.del_targets[lv] = targets;
        }
        ListOp::Ins { pos, item } => {
            let c = ctx.tree.cursor_at_cur_pos(pos);
            let origin_left = if c.doc_idx == 0 {
                -1
            } else {
                let (run, off) = ctx.tree.get(c.doc_idx - 1);
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
            let new_item = Run::single(lv, log.dots[lv], 0, 0, origin_left, right_parent);
            let mut c = c;
            integrate(&ctx.tree, log, &new_item, &mut c);
            ctx.tree.insert(c.doc_idx, new_item);
            snap.insert(c.end_pos, (log.dots[lv], item));
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Flag {
    A,
    B,
    Shared,
}

fn diff<P>(log: &OpLog<P>, a: &[usize], b: &[usize]) -> (Vec<usize>, Vec<usize>) {
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

        for &p in &log.parents[v] {
            enq(&mut queue, &mut flags, &mut num_shared, p, flag);
        }
    }

    (a_only, b_only)
}

fn apply_one<P: Clone>(ctx: &mut Ctx, snap: &mut Vec<(Dot, P)>, log: &OpLog<P>, lv: usize) {
    let (a_only, b_only) = diff(log, &ctx.cur_version, &log.parents[lv]);
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
    apply1(ctx, snap, log, lv);
    ctx.cur_version = vec![lv];
}

fn replay<P: Clone>(log: &OpLog<P>) -> (Ctx, Vec<(Dot, P)>) {
    let n = log.ops.len();
    let mut ctx = Ctx {
        tree: ContentTree::new(),
        del_targets: vec![Vec::new(); n],
        cur_version: Vec::new(),
    };
    let mut snap: Vec<(Dot, P)> = Vec::new();
    for lv in 0..n {
        apply_one(&mut ctx, &mut snap, log, lv);
    }
    (ctx, snap)
}

pub fn checkout<P: Clone>(log: &OpLog<P>) -> Vec<(Dot, P)> {
    let (_ctx, snap) = replay(log);
    snap
}

#[cfg(test)]
pub(crate) fn checkout_text(log: &OpLog<char>) -> String {
    checkout(log).into_iter().map(|(_id, c)| c).collect()
}

pub(crate) fn checkout_with_index<P: Clone>(log: &OpLog<P>) -> (Vec<(Dot, P)>, ResolveIndex) {
    let (ctx, snap) = replay(log);
    let lv_of: HashMap<Dot, usize> = log.dots.iter().enumerate().map(|(i, d)| (*d, i)).collect();
    let index = ResolveIndex {
        tree: ctx.tree,
        lv_of,
        del_targets: ctx.del_targets,
    };
    (snap, index)
}

pub(crate) struct ResolveIndex {
    tree: ContentTree<Run>,
    lv_of: HashMap<Dot, usize>,
    del_targets: Vec<Vec<usize>>,
}

impl ResolveIndex {
    fn lv_of(&self, id: Dot) -> Option<usize> {
        self.lv_of.get(&id).copied()
    }

    fn locate(&self, lv: usize) -> (usize, i32) {
        let doc_idx = self.tree.doc_index_of_lv(lv);
        let (run, _off) = self.tree.get(doc_idx);
        (doc_idx, run.end)
    }

    fn end_rank_at(&self, doc_idx: usize) -> usize {
        self.tree.end_rank_at_doc_index(doc_idx)
    }

    /// For a deletion op `del`, the visible position (`Before` bias) of the first
    /// element it deleted and the number of elements it deleted. Used to invert
    /// an `Undel` (i.e. redo a deletion): re-delete the now-restored elements.
    fn del_target_span(&self, del: Dot) -> Option<(usize, usize)> {
        let del_lv = self.lv_of(del)?;
        let targets = self.del_targets.get(del_lv)?;
        let first = *targets.first()?;
        let (doc_idx, _end) = self.locate(first);
        Some((self.end_rank_at(doc_idx), targets.len()))
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
        let lv = self.index.lv_of(id)?;
        let (doc_idx, end_state) = self.index.locate(lv);
        let visible = end_state == 0;
        let r = self.index.end_rank_at(doc_idx);
        let position = match bias {
            Bias::Before => r,
            Bias::After => r + usize::from(visible),
        };
        Some(Boundary { position, visible })
    }

    /// `(position, len)` of the elements deleted by deletion op `del`, resolved
    /// against the current (post-undel) tree. See [`ResolveIndex::del_target_span`].
    pub fn del_target_span(&self, del: Dot) -> Option<(usize, usize)> {
        self.index.del_target_span(del)
    }
}

pub fn checkout_with_resolver<P: Clone>(log: &OpLog<P>) -> (Vec<(Dot, P)>, BoundaryResolver) {
    let (elems, index) = checkout_with_index(log);
    (elems, BoundaryResolver { index })
}

#[cfg(test)]
pub(crate) fn checkout_runs(log: &OpLog) -> (String, usize, usize) {
    let (ctx, snap) = replay(log);
    let elems = ctx.tree.len();
    let runs = ctx.tree.run_count();
    (snap.into_iter().map(|(_id, c)| c).collect(), elems, runs)
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
}
