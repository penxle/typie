use std::cmp::Ordering;

use editor_common::order_interval_tree::{KeyResolve, OrderedIntervalTree};
use editor_crdt::Dot;
use editor_crdt::sequence::SeqCheckout;

use super::{Anchor, Bias};

/// Contract every implementation must uphold, and every caller must respect:
/// `order_index` is a total order over sequence-element dots that never
/// changes for a given pair of dots (the sequence never physically removes
/// or reorders elements), `anchor_pos` is monotone over it, and one index
/// must only ever be queried against checkouts of the same sequence it was
/// built with.
pub trait AnchorOrder {
    fn order_index(&self, dot: Dot) -> Option<usize>;
    fn anchor_pos(&self, a: &Anchor) -> Option<usize>;
}

impl AnchorOrder for SeqCheckout {
    fn order_index(&self, dot: Dot) -> Option<usize> {
        self.doc_index_of(dot)
    }

    fn anchor_pos(&self, a: &Anchor) -> Option<usize> {
        self.resolve_boundary(a.id, a.bias.into())
            .map(|b| b.position)
    }
}

fn bias_rank(b: Bias) -> u8 {
    match b {
        Bias::Before => 0,
        Bias::After => 1,
    }
}

struct Ctx<'a, R: AnchorOrder>(&'a R);

impl<'a, R: AnchorOrder> KeyResolve<Anchor> for Ctx<'a, R> {
    fn cmp_keys(&self, a: &Anchor, b: &Anchor) -> Ordering {
        let ka = self
            .0
            .order_index(a.id)
            .expect("indexed anchor must resolve against its own checkout");
        let kb = self
            .0
            .order_index(b.id)
            .expect("indexed anchor must resolve against its own checkout");
        ka.cmp(&kb).then(bias_rank(a.bias).cmp(&bias_rank(b.bias)))
    }

    fn pos(&self, k: &Anchor) -> Option<usize> {
        self.0.anchor_pos(k)
    }
}

#[derive(Clone, Debug)]
pub struct AnchorIntervalIndex<P: Clone + Ord = Dot> {
    tree: OrderedIntervalTree<Anchor, P>,
    pending: imbl::Vector<(Anchor, Anchor, P)>,
}

impl<P: Clone + Ord> Default for AnchorIntervalIndex<P> {
    fn default() -> Self {
        AnchorIntervalIndex {
            tree: OrderedIntervalTree::new(),
            pending: imbl::Vector::new(),
        }
    }
}

impl<P: Clone + Ord> AnchorIntervalIndex<P> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.tree.len() + self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tree.is_empty() && self.pending.is_empty()
    }

    pub fn resolved_len(&self) -> usize {
        self.tree.len()
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    /// Full rebuild from an authoritative interval source (e.g. the span log
    /// after a load-grade event); unresolved anchors park in pending.
    pub fn build<R: AnchorOrder>(
        ctx: &R,
        intervals: impl IntoIterator<Item = (Anchor, Anchor, P)>,
    ) -> Self {
        let mut idx = Self::new();
        for (s, e, p) in intervals {
            let _ = idx.insert(ctx, s, e, p);
        }
        idx
    }

    #[must_use]
    pub fn insert<R: AnchorOrder>(
        &mut self,
        ctx: &R,
        start: Anchor,
        end: Anchor,
        payload: P,
    ) -> bool {
        if self
            .pending
            .iter()
            .any(|(s, _, d)| *s == start && *d == payload)
        {
            return false;
        }
        let start_resolved = ctx.order_index(start.id).is_some();
        if start_resolved && self.tree.contains(&Ctx(ctx), &start, &payload) {
            return false;
        }
        if !start_resolved || ctx.order_index(end.id).is_none() {
            self.pending.push_back((start, end, payload));
            return true;
        }
        self.tree.insert(&Ctx(ctx), start, end, payload)
    }

    #[must_use]
    pub fn remove<R: AnchorOrder>(&mut self, ctx: &R, start: &Anchor, payload: &P) -> bool {
        if let Some(i) = self
            .pending
            .iter()
            .position(|(s, _, d)| s == start && d == payload)
        {
            self.pending.remove(i);
            return true;
        }
        if ctx.order_index(start.id).is_none() {
            return false;
        }
        self.tree.remove(&Ctx(ctx), start, payload)
    }

    pub fn flush_pending<R: AnchorOrder>(&mut self, ctx: &R) {
        let parked = std::mem::take(&mut self.pending);
        for (start, end, payload) in parked {
            let _ = self.insert(ctx, start, end, payload);
        }
    }

    /// Re-attempts only the parked intervals that name `dot` as an anchor — the
    /// only entries a fresh insertion of `dot` can newly resolve — and returns
    /// the payloads actually promoted into the tree, so the caller can re-apply
    /// their effects to state derived while they were unresolvable. Per-call
    /// cost: O(pending) dot comparisons plus O(log) insertion for the matches,
    /// instead of re-attempting the whole quarantine.
    pub fn flush_pending_for<R: AnchorOrder>(&mut self, ctx: &R, dot: Dot) -> Vec<P> {
        let parked = std::mem::take(&mut self.pending);
        let mut promoted = Vec::new();
        for (start, end, payload) in parked {
            let names_dot = start.id == dot || end.id == dot;
            if names_dot && ctx.order_index(start.id).is_some() && ctx.order_index(end.id).is_some()
            {
                if self.insert(ctx, start, end, payload.clone()) {
                    promoted.push(payload);
                }
            } else {
                self.pending.push_back((start, end, payload));
            }
        }
        promoted
    }

    pub fn stab<R: AnchorOrder>(&self, ctx: &R, p: usize) -> Vec<P> {
        let mut out = Vec::new();
        self.tree.stab(&Ctx(ctx), p, &mut out);
        out.sort();
        out
    }

    pub fn intersecting<R: AnchorOrder>(&self, ctx: &R, lo: usize, hi: usize) -> Vec<P> {
        let mut out = Vec::new();
        self.tree.intersecting(&Ctx(ctx), lo, hi, &mut out);
        out.sort();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::sequence::SeqCheckout;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};

    fn checkout_of(items: &[(Dot, char)]) -> SeqCheckout {
        let mut ev = Vec::new();
        let mut prev: Option<Dot> = None;
        for (i, (id, ch)) in items.iter().enumerate() {
            ev.push(InputEvent {
                id: *id,
                parents: prev.into_iter().collect(),
                op: ListOp::Ins { pos: i, item: *ch },
            });
            prev = Some(*id);
        }
        let log = build_oplog(&ev);
        let mut co = SeqCheckout::new();
        co.apply_tail(&log);
        co
    }

    fn anchor(d: Dot, bias: Bias) -> Anchor {
        Anchor { id: d, bias }
    }

    #[test]
    fn stab_matches_positional_predicate() {
        let dots: Vec<Dot> = (1..=6).map(|c| Dot::new(1, c)).collect();
        let co = checkout_of(&dots.iter().map(|d| (*d, 'x')).collect::<Vec<_>>());
        let mut idx = AnchorIntervalIndex::new();
        let s1 = Dot::new(7, 1);
        let s2 = Dot::new(7, 2);
        assert!(idx.insert(
            &co,
            anchor(dots[1], Bias::Before),
            anchor(dots[3], Bias::After),
            s1
        ));
        assert!(idx.insert(
            &co,
            anchor(dots[0], Bias::After),
            anchor(dots[2], Bias::Before),
            s2
        ));
        assert_eq!(idx.len(), 2);

        let expect = |p: usize| {
            let mut v = Vec::new();
            for (s, e, d) in [
                (
                    co.resolve_boundary(dots[1], editor_crdt::sequence::Bias::Before)
                        .unwrap()
                        .position,
                    co.resolve_boundary(dots[3], editor_crdt::sequence::Bias::After)
                        .unwrap()
                        .position,
                    s1,
                ),
                (
                    co.resolve_boundary(dots[0], editor_crdt::sequence::Bias::After)
                        .unwrap()
                        .position,
                    co.resolve_boundary(dots[2], editor_crdt::sequence::Bias::Before)
                        .unwrap()
                        .position,
                    s2,
                ),
            ] {
                if s <= p && p < e {
                    v.push(d);
                }
            }
            v.sort();
            v
        };
        for p in 0..8 {
            assert_eq!(idx.stab(&co, p), expect(p), "p={p}");
        }
    }

    #[test]
    fn unresolvable_anchor_parks_in_pending_until_flush() {
        let dots: Vec<Dot> = (1..=3).map(|c| Dot::new(1, c)).collect();
        let co = checkout_of(&dots.iter().map(|d| (*d, 'x')).collect::<Vec<_>>());
        let mut idx = AnchorIntervalIndex::new();
        let ghost = Dot::new(9, 99);
        assert!(idx.insert(
            &co,
            anchor(ghost, Bias::Before),
            anchor(dots[1], Bias::After),
            Dot::new(7, 1)
        ));
        assert_eq!(idx.resolved_len(), 0);
        assert_eq!(idx.len(), 1, "pending counts toward the total");
        assert_eq!(idx.pending_len(), 1);
        assert!(idx.stab(&co, 1).is_empty());
        idx.flush_pending(&co);
        assert_eq!(idx.pending_len(), 1, "still unresolvable");

        let mut ev: Vec<InputEvent<char>> = Vec::new();
        let mut prev: Option<Dot> = None;
        for (i, d) in dots.iter().enumerate() {
            ev.push(InputEvent {
                id: *d,
                parents: prev.into_iter().collect(),
                op: ListOp::Ins { pos: i, item: 'x' },
            });
            prev = Some(*d);
        }
        ev.push(InputEvent {
            id: ghost,
            parents: prev.into_iter().collect(),
            op: ListOp::Ins { pos: 0, item: 'g' },
        });
        let log = build_oplog(&ev);
        let mut co2 = SeqCheckout::new();
        co2.apply_tail(&log);
        idx.flush_pending(&co2);
        assert_eq!(idx.pending_len(), 0);
        assert_eq!(idx.len(), 1);
        assert_eq!(idx.stab(&co2, 1), vec![Dot::new(7, 1)]);
    }

    #[test]
    fn remove_covers_tree_and_pending() {
        let dots: Vec<Dot> = (1..=3).map(|c| Dot::new(1, c)).collect();
        let co = checkout_of(&dots.iter().map(|d| (*d, 'x')).collect::<Vec<_>>());
        let mut idx = AnchorIntervalIndex::new();
        let op = Dot::new(7, 1);
        let st = anchor(dots[0], Bias::Before);
        assert!(idx.insert(&co, st, anchor(dots[2], Bias::After), op));
        assert!(idx.remove(&co, &st, &op));
        assert!(idx.is_empty());

        let ghost = Dot::new(9, 99);
        let gst = anchor(ghost, Bias::Before);
        assert!(idx.insert(&co, gst, anchor(dots[1], Bias::After), op));
        assert_eq!(idx.pending_len(), 1);
        assert!(idx.remove(&co, &gst, &op));
        assert_eq!(idx.pending_len(), 0);
        assert!(idx.is_empty());
    }

    #[test]
    fn cross_store_duplicates_and_unresolved_removal() {
        let dots: Vec<Dot> = (1..=3).map(|c| Dot::new(1, c)).collect();
        let co = checkout_of(&dots.iter().map(|d| (*d, 'x')).collect::<Vec<_>>());
        let mut idx = AnchorIntervalIndex::new();
        let ghost = Dot::new(9, 99);
        let op = Dot::new(7, 1);
        let st = anchor(dots[0], Bias::Before);
        assert!(idx.insert(&co, st, anchor(ghost, Bias::After), op));
        assert!(
            !idx.insert(&co, st, anchor(dots[2], Bias::After), op),
            "a pending composite blocks its resolved duplicate"
        );
        let op2 = Dot::new(7, 2);
        assert!(idx.insert(
            &co,
            anchor(dots[1], Bias::Before),
            anchor(dots[2], Bias::After),
            op2
        ));
        assert!(
            !idx.insert(
                &co,
                anchor(dots[1], Bias::Before),
                anchor(ghost, Bias::After),
                op2
            ),
            "a resolved composite blocks its pending duplicate"
        );
        assert_eq!(idx.len(), 2);
        assert!(
            !idx.remove(&co, &anchor(ghost, Bias::Before), &Dot::new(7, 9)),
            "removing an absent entry with an unresolved start is a clean false"
        );
    }

    #[test]
    fn targeted_flush_promotes_only_entries_naming_the_dot() {
        let dots: Vec<Dot> = (1..=4).map(|c| Dot::new(1, c)).collect();
        let co = checkout_of(&dots.iter().map(|d| (*d, 'x')).collect::<Vec<_>>());
        let mut idx = AnchorIntervalIndex::new();
        let late_a = Dot::new(1, 9);
        let late_b = Dot::new(1, 10);
        assert!(idx.insert(
            &co,
            anchor(dots[0], Bias::Before),
            anchor(late_a, Bias::After),
            Dot::new(7, 1)
        ));
        assert!(idx.insert(
            &co,
            anchor(dots[1], Bias::Before),
            anchor(late_b, Bias::After),
            Dot::new(7, 2)
        ));
        assert!(idx.insert(
            &co,
            anchor(dots[2], Bias::Before),
            anchor(late_a, Bias::After),
            Dot::new(7, 3)
        ));
        assert_eq!(idx.pending_len(), 3);
        assert!(
            idx.flush_pending_for(&co, dots[3]).is_empty(),
            "no entry names dots[3]"
        );
        assert_eq!(idx.pending_len(), 3);
        let mut extended: Vec<(Dot, char)> = dots.iter().map(|d| (*d, 'x')).collect();
        extended.push((late_a, 'y'));
        extended.push((late_b, 'z'));
        let co2 = checkout_of(&extended);
        let mut promoted = idx.flush_pending_for(&co2, late_a);
        promoted.sort();
        assert_eq!(
            promoted,
            vec![Dot::new(7, 1), Dot::new(7, 3)],
            "one Ins must promote EVERY entry naming late_a — not just the first match"
        );
        assert_eq!(idx.pending_len(), 1);
        assert_eq!(idx.flush_pending_for(&co2, late_b), vec![Dot::new(7, 2)]);
        assert_eq!(idx.pending_len(), 0);
        assert_eq!(idx.resolved_len(), 3);
    }
}
