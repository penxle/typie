use std::cmp::Ordering;

/// `cmp_keys` must be a total order that is immutable for keys already in a
/// tree, and `pos` must be monotone over it: `cmp_keys(a, b) != Greater`
/// implies `pos(a) <= pos(b)` whenever both resolve.
pub trait KeyResolve<K> {
    fn cmp_keys(&self, a: &K, b: &K) -> Ordering;
    fn pos(&self, k: &K) -> Option<usize>;
}

#[derive(Clone, Debug)]
struct Node<K, P> {
    start: K,
    end: K,
    payload: P,
    left: Option<usize>,
    right: Option<usize>,
    height: u8,
    max_end: K,
}

#[derive(Clone, Debug)]
pub struct OrderedIntervalTree<K, P> {
    nodes: imbl::Vector<Node<K, P>>,
    free: imbl::Vector<usize>,
    root: Option<usize>,
    len: usize,
}

impl<K: Clone, P: Clone + Ord> Default for OrderedIntervalTree<K, P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Clone, P: Clone + Ord> OrderedIntervalTree<K, P> {
    pub fn new() -> Self {
        OrderedIntervalTree {
            nodes: imbl::Vector::new(),
            free: imbl::Vector::new(),
            root: None,
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[cfg(test)]
    fn node_capacity(&self) -> usize {
        self.nodes.len()
    }

    fn node(&self, i: usize) -> &Node<K, P> {
        &self.nodes[i]
    }

    fn node_mut(&mut self, i: usize) -> &mut Node<K, P> {
        self.nodes.get_mut(i).expect("live arena slot")
    }

    /// Slots are mutated in place (imbl COW keeps clones intact) and freed
    /// slots are reused; allocation happens only for genuinely new intervals,
    /// so the arena stays bounded by the high-water live count.
    fn alloc(&mut self, node: Node<K, P>) -> usize {
        match self.free.last().copied() {
            Some(i) => {
                self.free.remove(self.free.len() - 1);
                self.nodes.set(i, node);
                i
            }
            None => {
                self.nodes.push_back(node);
                self.nodes.len() - 1
            }
        }
    }

    fn height(&self, n: Option<usize>) -> u8 {
        n.map(|i| self.node(i).height).unwrap_or(0)
    }

    fn max_end_of<R: KeyResolve<K>>(
        &self,
        ctx: &R,
        end: &K,
        left: Option<usize>,
        right: Option<usize>,
    ) -> K {
        let mut best = end.clone();
        for c in [left, right].into_iter().flatten() {
            let m = &self.node(c).max_end;
            if ctx.cmp_keys(m, &best) == Ordering::Greater {
                best = m.clone();
            }
        }
        best
    }

    fn refresh<R: KeyResolve<K>>(&mut self, ctx: &R, i: usize) {
        let (l, r) = (self.node(i).left, self.node(i).right);
        let height = 1 + self.height(l).max(self.height(r));
        let end = self.node(i).end.clone();
        let max_end = self.max_end_of(ctx, &end, l, r);
        let n = self.node_mut(i);
        n.height = height;
        n.max_end = max_end;
    }

    fn rotate_right<R: KeyResolve<K>>(&mut self, ctx: &R, i: usize) -> usize {
        let l = self.node(i).left.expect("rotate_right needs a left child");
        let lr = self.node(l).right;
        self.node_mut(i).left = lr;
        self.refresh(ctx, i);
        self.node_mut(l).right = Some(i);
        self.refresh(ctx, l);
        l
    }

    fn rotate_left<R: KeyResolve<K>>(&mut self, ctx: &R, i: usize) -> usize {
        let r = self.node(i).right.expect("rotate_left needs a right child");
        let rl = self.node(r).left;
        self.node_mut(i).right = rl;
        self.refresh(ctx, i);
        self.node_mut(r).left = Some(i);
        self.refresh(ctx, r);
        r
    }

    fn balance<R: KeyResolve<K>>(&mut self, ctx: &R, i: usize) -> usize {
        let (l, r) = (self.node(i).left, self.node(i).right);
        let bf = self.height(l) as i16 - self.height(r) as i16;
        if bf > 1 {
            let li = l.expect("left-heavy node has a left child");
            let ll = self.node(li).left;
            let lr = self.node(li).right;
            if self.height(ll) < self.height(lr) {
                let nl = self.rotate_left(ctx, li);
                self.node_mut(i).left = Some(nl);
                self.refresh(ctx, i);
            }
            self.rotate_right(ctx, i)
        } else if bf < -1 {
            let ri = r.expect("right-heavy node has a right child");
            let rl = self.node(ri).left;
            let rr = self.node(ri).right;
            if self.height(rr) < self.height(rl) {
                let nr = self.rotate_right(ctx, ri);
                self.node_mut(i).right = Some(nr);
                self.refresh(ctx, i);
            }
            self.rotate_left(ctx, i)
        } else {
            i
        }
    }

    fn cmp_entry<R: KeyResolve<K>>(&self, ctx: &R, start: &K, payload: &P, at: usize) -> Ordering {
        let n = self.node(at);
        match ctx.cmp_keys(start, &n.start) {
            Ordering::Equal => payload.cmp(&n.payload),
            o => o,
        }
    }

    fn ins<R: KeyResolve<K>>(
        &mut self,
        ctx: &R,
        at: Option<usize>,
        start: &K,
        end: &K,
        payload: &P,
    ) -> (usize, bool) {
        let Some(i) = at else {
            let max_end = end.clone();
            let idx = self.alloc(Node {
                start: start.clone(),
                end: end.clone(),
                payload: payload.clone(),
                left: None,
                right: None,
                height: 1,
                max_end,
            });
            return (idx, true);
        };
        match self.cmp_entry(ctx, start, payload, i) {
            Ordering::Equal => return (i, false),
            Ordering::Less => {
                let l = self.node(i).left;
                let (nl, inserted) = self.ins(ctx, l, start, end, payload);
                if !inserted {
                    return (i, false);
                }
                self.node_mut(i).left = Some(nl);
            }
            Ordering::Greater => {
                let r = self.node(i).right;
                let (nr, inserted) = self.ins(ctx, r, start, end, payload);
                if !inserted {
                    return (i, false);
                }
                self.node_mut(i).right = Some(nr);
            }
        }
        self.refresh(ctx, i);
        (self.balance(ctx, i), true)
    }

    /// An entry whose `(start, payload)` composite already exists is rejected
    /// and `false` is returned — duplicate/replayed insertion must not
    /// double-index.
    #[must_use]
    pub fn insert<R: KeyResolve<K>>(&mut self, ctx: &R, start: K, end: K, payload: P) -> bool {
        let (root, inserted) = self.ins(ctx, self.root, &start, &end, &payload);
        self.root = Some(root);
        if inserted {
            self.len += 1;
        }
        inserted
    }

    /// Bulk balanced construction from entries already strictly ascending in
    /// the `(start, payload)` composite order `insert` maintains, with no
    /// duplicate composites; O(n) comparisons total, all through `ctx`.
    pub fn build_from_sorted<R: KeyResolve<K>>(ctx: &R, entries: Vec<(K, K, P)>) -> Self {
        let mut tree = Self::new();
        let len = entries.len();
        if len == 0 {
            return tree;
        }
        let mut slots: Vec<Option<(K, K, P)>> = entries.into_iter().map(Some).collect();
        let root = tree.build_range(ctx, &mut slots, 0, len);
        tree.root = Some(root);
        tree.len = len;
        tree
    }

    fn build_range<R: KeyResolve<K>>(
        &mut self,
        ctx: &R,
        slots: &mut [Option<(K, K, P)>],
        lo: usize,
        hi: usize,
    ) -> usize {
        let mid = lo + (hi - lo) / 2;
        let left = (lo < mid).then(|| self.build_range(ctx, slots, lo, mid));
        let right = (mid + 1 < hi).then(|| self.build_range(ctx, slots, mid + 1, hi));
        let (start, end, payload) = slots[mid].take().expect("one take per slot");
        let height = 1 + self.height(left).max(self.height(right));
        let max_end = self.max_end_of(ctx, &end, left, right);
        self.alloc(Node {
            start,
            end,
            payload,
            left,
            right,
            height,
            max_end,
        })
    }

    pub fn stab<R: KeyResolve<K>>(&self, ctx: &R, p: usize, out: &mut Vec<P>) {
        self.stab_rec(ctx, self.root, p, out);
    }

    fn stab_rec<R: KeyResolve<K>>(&self, ctx: &R, at: Option<usize>, p: usize, out: &mut Vec<P>) {
        let Some(i) = at else { return };
        let n = self.node(i);
        if let Some(me) = ctx.pos(&n.max_end)
            && me <= p
        {
            return;
        }
        self.stab_rec(ctx, n.left, p, out);
        match ctx.pos(&n.start) {
            Some(sp) if sp > p => {}
            Some(_) => {
                if ctx.pos(&n.end).is_some_and(|e| e > p) {
                    out.push(n.payload.clone());
                }
                self.stab_rec(ctx, n.right, p, out);
            }
            None => {
                debug_assert!(false, "indexed key must resolve");
                self.stab_rec(ctx, n.right, p, out);
            }
        }
    }

    pub fn intersecting<R: KeyResolve<K>>(&self, ctx: &R, lo: usize, hi: usize, out: &mut Vec<P>) {
        self.inter_rec(ctx, self.root, lo, hi, out);
    }

    fn inter_rec<R: KeyResolve<K>>(
        &self,
        ctx: &R,
        at: Option<usize>,
        lo: usize,
        hi: usize,
        out: &mut Vec<P>,
    ) {
        let Some(i) = at else { return };
        let n = self.node(i);
        if let Some(me) = ctx.pos(&n.max_end)
            && me <= lo
        {
            return;
        }
        self.inter_rec(ctx, n.left, lo, hi, out);
        match ctx.pos(&n.start) {
            Some(sp) if sp >= hi => {}
            Some(sp) => {
                if let Some(ep) = ctx.pos(&n.end)
                    && sp < ep
                    && ep > lo
                {
                    out.push(n.payload.clone());
                }
                self.inter_rec(ctx, n.right, lo, hi, out);
            }
            None => {
                debug_assert!(false, "indexed key must resolve");
                self.inter_rec(ctx, n.right, lo, hi, out);
            }
        }
    }

    pub fn contains<R: KeyResolve<K>>(&self, ctx: &R, start: &K, payload: &P) -> bool {
        let mut at = self.root;
        while let Some(i) = at {
            match self.cmp_entry(ctx, start, payload, i) {
                Ordering::Equal => return true,
                Ordering::Less => at = self.node(i).left,
                Ordering::Greater => at = self.node(i).right,
            }
        }
        false
    }

    #[must_use]
    pub fn remove<R: KeyResolve<K>>(&mut self, ctx: &R, start: &K, payload: &P) -> bool {
        let (new_root, removed) = self.del(ctx, self.root, start, payload);
        if removed {
            self.root = new_root;
            self.len -= 1;
        }
        removed
    }

    fn del<R: KeyResolve<K>>(
        &mut self,
        ctx: &R,
        at: Option<usize>,
        start: &K,
        payload: &P,
    ) -> (Option<usize>, bool) {
        let Some(i) = at else { return (None, false) };
        match self.cmp_entry(ctx, start, payload, i) {
            Ordering::Less => {
                let l = self.node(i).left;
                let (nl, removed) = self.del(ctx, l, start, payload);
                if !removed {
                    return (Some(i), false);
                }
                self.node_mut(i).left = nl;
                self.refresh(ctx, i);
                (Some(self.balance(ctx, i)), true)
            }
            Ordering::Greater => {
                let r = self.node(i).right;
                let (nr, removed) = self.del(ctx, r, start, payload);
                if !removed {
                    return (Some(i), false);
                }
                self.node_mut(i).right = nr;
                self.refresh(ctx, i);
                (Some(self.balance(ctx, i)), true)
            }
            Ordering::Equal => {
                let (l, r) = (self.node(i).left, self.node(i).right);
                match (l, r) {
                    (None, sub) | (sub, None) => {
                        self.free.push_back(i);
                        (sub, true)
                    }
                    (Some(_), Some(ri)) => {
                        let (nr, min_idx) = self.take_min(ctx, ri);
                        let m = self.node(min_idx).clone();
                        self.free.push_back(min_idx);
                        let n = self.node_mut(i);
                        n.start = m.start;
                        n.end = m.end;
                        n.payload = m.payload;
                        n.right = nr;
                        self.refresh(ctx, i);
                        (Some(self.balance(ctx, i)), true)
                    }
                }
            }
        }
    }

    fn take_min<R: KeyResolve<K>>(&mut self, ctx: &R, at: usize) -> (Option<usize>, usize) {
        let l = self.node(at).left;
        let Some(li) = l else {
            return (self.node(at).right, at);
        };
        let (nl, min_idx) = self.take_min(ctx, li);
        self.node_mut(at).left = nl;
        self.refresh(ctx, at);
        (Some(self.balance(ctx, at)), min_idx)
    }

    pub fn in_order(&self) -> Vec<(K, P)> {
        let mut out = Vec::with_capacity(self.len);
        self.in_order_rec(self.root, &mut out);
        out
    }

    fn in_order_rec(&self, at: Option<usize>, out: &mut Vec<(K, P)>) {
        let Some(i) = at else { return };
        let n = self.node(i);
        self.in_order_rec(n.left, out);
        out.push((n.start.clone(), n.payload.clone()));
        self.in_order_rec(n.right, out);
    }

    #[cfg(test)]
    fn collect_reachable(
        &self,
        at: Option<usize>,
        out: &mut Vec<usize>,
        seen: &mut std::collections::HashSet<usize>,
    ) {
        let Some(i) = at else { return };
        assert!(
            seen.insert(i),
            "arena slot {i} reachable twice (cycle or sharing)"
        );
        out.push(i);
        self.collect_reachable(self.node(i).left, out, seen);
        self.collect_reachable(self.node(i).right, out, seen);
    }

    #[cfg(test)]
    fn assert_invariants<R: KeyResolve<K>>(&self, ctx: &R) {
        let mut reachable: Vec<usize> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        self.collect_reachable(self.root, &mut reachable, &mut seen);

        fn rec<K: Clone, P: Clone + Ord, R: KeyResolve<K>>(
            t: &OrderedIntervalTree<K, P>,
            ctx: &R,
            at: Option<usize>,
        ) -> (u8, Option<K>) {
            let Some(i) = at else { return (0, None) };
            let n = t.node(i);
            let (hl, ml) = rec(t, ctx, n.left);
            let (hr, mr) = rec(t, ctx, n.right);
            assert_eq!(n.height, 1 + hl.max(hr));
            assert!((hl as i16 - hr as i16).abs() <= 1, "AVL balance violated");
            let mut expect = n.end.clone();
            for m in [ml, mr].into_iter().flatten() {
                if ctx.cmp_keys(&m, &expect) == Ordering::Greater {
                    expect = m;
                }
            }
            assert_eq!(
                ctx.cmp_keys(&n.max_end, &expect),
                Ordering::Equal,
                "max_end augmentation stale"
            );
            (n.height, Some(n.max_end.clone()))
        }
        rec(self, ctx, self.root);

        assert_eq!(reachable.len(), self.len, "reachable node count equals len");
        let entries = self.in_order();
        for w in entries.windows(2) {
            let ord = ctx
                .cmp_keys(&w[0].0, &w[1].0)
                .then_with(|| w[0].1.cmp(&w[1].1));
            assert_eq!(
                ord,
                Ordering::Less,
                "in-order must be strictly increasing by (start, payload)"
            );
        }
        let free_vec: Vec<usize> = self.free.iter().copied().collect();
        let free: std::collections::HashSet<usize> = free_vec.iter().copied().collect();
        assert_eq!(
            free_vec.len(),
            free.len(),
            "free list must not hold duplicates"
        );
        assert!(
            free_vec.iter().all(|&i| i < self.nodes.len()),
            "free slots stay in arena bounds"
        );
        assert!(
            reachable.iter().all(|i| !free.contains(i)),
            "free and reachable slots must be disjoint"
        );
        assert_eq!(
            reachable.len() + free.len(),
            self.nodes.len(),
            "arena slots are exactly reachable + free"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    struct UsizeCtx;

    impl KeyResolve<usize> for UsizeCtx {
        fn cmp_keys(&self, a: &usize, b: &usize) -> Ordering {
            a.cmp(b)
        }
        fn pos(&self, k: &usize) -> Option<usize> {
            Some(*k)
        }
    }

    #[test]
    fn insert_keeps_in_order_sorted_by_start_then_payload() {
        let ctx = UsizeCtx;
        let mut t: OrderedIntervalTree<usize, u32> = OrderedIntervalTree::new();
        for (s, e, p) in [
            (5usize, 9usize, 1u32),
            (1, 3, 2),
            (5, 7, 0),
            (2, 8, 3),
            (1, 2, 1),
        ] {
            assert!(t.insert(&ctx, s, e, p));
        }
        assert_eq!(t.len(), 5);
        let starts: Vec<(usize, u32)> = t.in_order();
        assert_eq!(starts, vec![(1, 1), (1, 2), (2, 3), (5, 0), (5, 1)]);
    }

    #[test]
    fn clone_shares_then_diverges() {
        let ctx = UsizeCtx;
        let mut a: OrderedIntervalTree<usize, u32> = OrderedIntervalTree::new();
        for i in 0..50 {
            assert!(a.insert(&ctx, i, i + 2, i as u32));
        }
        let b = a.clone();
        assert!(a.insert(&ctx, 100, 101, 999));
        assert_eq!(a.len(), 51);
        assert_eq!(b.len(), 50);
        assert!(b.in_order().iter().all(|(k, _)| *k < 100));
    }

    #[test]
    fn duplicate_composite_insert_is_rejected() {
        let ctx = UsizeCtx;
        let mut t: OrderedIntervalTree<usize, u32> = OrderedIntervalTree::new();
        assert!(t.insert(&ctx, 1, 5, 7));
        assert!(
            !t.insert(&ctx, 1, 9, 7),
            "same (start, payload) is idempotent"
        );
        assert_eq!(t.len(), 1);
        assert_eq!(t.in_order(), vec![(1, 7)]);
        t.assert_invariants(&ctx);
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 256, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn in_order_matches_sorted_model(
            items in proptest::collection::vec((0usize..64, 0usize..64), 0..80),
        ) {
            let ctx = UsizeCtx;
            let mut t: OrderedIntervalTree<usize, u32> = OrderedIntervalTree::new();
            let mut model: Vec<(usize, u32)> = Vec::new();
            for (i, (s, e)) in items.into_iter().enumerate() {
                let p = i as u32;
                assert!(t.insert(&ctx, s, e, p));
                model.push((s, p));
            }
            model.sort();
            proptest::prop_assert_eq!(t.in_order(), model);
            t.assert_invariants(&ctx);
        }

        #[test]
        fn bulk_build_matches_incremental(
            items in proptest::collection::vec((0usize..64, 0usize..64), 0..80),
        ) {
            let ctx = UsizeCtx;
            let mut incr: OrderedIntervalTree<usize, u32> = OrderedIntervalTree::new();
            let mut entries: Vec<(usize, usize, u32)> = Vec::new();
            for (i, (s, e)) in items.into_iter().enumerate() {
                let p = i as u32;
                assert!(incr.insert(&ctx, s, e, p));
                entries.push((s, e, p));
            }
            entries.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.2.cmp(&b.2)));
            let bulk = OrderedIntervalTree::build_from_sorted(&ctx, entries);
            proptest::prop_assert_eq!(bulk.len(), incr.len());
            proptest::prop_assert_eq!(bulk.in_order(), incr.in_order());
            bulk.assert_invariants(&ctx);
            for p in 0..70 {
                let mut a = Vec::new();
                let mut b = Vec::new();
                bulk.stab(&ctx, p, &mut a);
                incr.stab(&ctx, p, &mut b);
                a.sort();
                b.sort();
                proptest::prop_assert_eq!(a, b, "stab p={}", p);
            }
        }
    }

    fn naive_stab(model: &[(usize, usize, u32)], p: usize) -> Vec<u32> {
        let mut out: Vec<u32> = model
            .iter()
            .filter(|(s, e, _)| *s <= p && p < *e)
            .map(|(_, _, v)| *v)
            .collect();
        out.sort();
        out
    }

    fn naive_intersecting(model: &[(usize, usize, u32)], lo: usize, hi: usize) -> Vec<u32> {
        let mut out: Vec<u32> = model
            .iter()
            .filter(|(s, e, _)| *s < *e && *s < hi && *e > lo)
            .map(|(_, _, v)| *v)
            .collect();
        out.sort();
        out
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 256, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn queries_and_removal_match_naive_model(
            items in proptest::collection::vec((0usize..48, 0usize..48), 0..64),
            removes in proptest::collection::vec(proptest::prelude::any::<proptest::sample::Index>(), 0..24),
        ) {
            let ctx = UsizeCtx;
            let mut t: OrderedIntervalTree<usize, u32> = OrderedIntervalTree::new();
            let mut model: Vec<(usize, usize, u32)> = Vec::new();
            for (i, (s, e)) in items.into_iter().enumerate() {
                let p = i as u32;
                assert!(t.insert(&ctx, s, e, p));
                model.push((s, e, p));
                t.assert_invariants(&ctx);
            }
            for r in removes {
                if model.is_empty() {
                    break;
                }
                let (s, _e, p) = model.remove(r.index(model.len()));
                proptest::prop_assert!(t.remove(&ctx, &s, &p));
                t.assert_invariants(&ctx);
            }
            proptest::prop_assert_eq!(t.len(), model.len());
            for p in 0..48 {
                let mut got = Vec::new();
                t.stab(&ctx, p, &mut got);
                got.sort();
                proptest::prop_assert_eq!(&got, &naive_stab(&model, p), "stab at {}", p);
            }
            for lo in (0..48).step_by(7) {
                for hi in (lo..48).step_by(5) {
                    let mut got = Vec::new();
                    t.intersecting(&ctx, lo, hi, &mut got);
                    got.sort();
                    proptest::prop_assert_eq!(&got, &naive_intersecting(&model, lo, hi), "intersecting [{}, {})", lo, hi);
                }
            }
        }
    }

    #[test]
    fn remove_of_absent_entry_is_false_and_lossless() {
        let ctx = UsizeCtx;
        let mut t: OrderedIntervalTree<usize, u32> = OrderedIntervalTree::new();
        assert!(t.insert(&ctx, 1, 5, 7));
        assert!(!t.insert(&ctx, 1, 9, 7), "duplicate composite rejected");
        let mut got = Vec::new();
        t.stab(&ctx, 6, &mut got);
        assert!(
            got.is_empty(),
            "rejected duplicate must not replace the end"
        );
        assert!(!t.remove(&ctx, &1, &8));
        assert!(!t.remove(&ctx, &2, &7));
        assert_eq!(t.len(), 1);
        assert!(t.remove(&ctx, &1, &7));
        assert!(t.is_empty());
    }

    #[test]
    fn clones_diverge_over_shared_free_slots() {
        let ctx = UsizeCtx;
        let mut base: OrderedIntervalTree<usize, u32> = OrderedIntervalTree::new();
        for i in 0..40usize {
            assert!(base.insert(&ctx, i, i + 3, i as u32));
        }
        for i in 0..20usize {
            assert!(base.remove(&ctx, &i, &(i as u32)));
        }
        let mut c1 = base.clone();
        let mut c2 = base.clone();
        for i in 100..115usize {
            assert!(c1.insert(&ctx, i, i + 1, i as u32));
        }
        for i in 200..210usize {
            assert!(c2.insert(&ctx, i, i + 2, i as u32));
        }
        for i in 20..25usize {
            assert!(base.remove(&ctx, &i, &(i as u32)));
        }
        base.assert_invariants(&ctx);
        c1.assert_invariants(&ctx);
        c2.assert_invariants(&ctx);
        let starts = |t: &OrderedIntervalTree<usize, u32>| {
            t.in_order().into_iter().map(|(k, _)| k).collect::<Vec<_>>()
        };
        assert_eq!(starts(&base), (25..40).collect::<Vec<_>>());
        assert_eq!(starts(&c1), (20..40).chain(100..115).collect::<Vec<_>>());
        assert_eq!(starts(&c2), (20..40).chain(200..210).collect::<Vec<_>>());
    }

    #[test]
    fn empty_and_reversed_intervals_never_stab() {
        let ctx = UsizeCtx;
        let mut t: OrderedIntervalTree<usize, u32> = OrderedIntervalTree::new();
        assert!(t.insert(&ctx, 3, 3, 0));
        assert!(t.insert(&ctx, 9, 2, 1));
        for p in 0..12 {
            let mut got = Vec::new();
            t.stab(&ctx, p, &mut got);
            assert!(got.is_empty(), "p={p}");
        }
        let mut got = Vec::new();
        t.intersecting(&ctx, 0, 12, &mut got);
        assert!(got.is_empty(), "empty/reversed intervals never intersect");
    }

    #[test]
    fn arena_slots_are_reused_after_removal() {
        let ctx = UsizeCtx;
        let mut t: OrderedIntervalTree<usize, u32> = OrderedIntervalTree::new();
        for round in 0..3u32 {
            for i in 0..100usize {
                assert!(t.insert(&ctx, i, i + 1, round * 100 + i as u32));
            }
            for i in 0..100usize {
                assert!(t.remove(&ctx, &i, &(round * 100 + i as u32)));
            }
        }
        assert!(t.is_empty());
        assert!(
            t.node_capacity() <= 100,
            "arena must stay at the live high-water mark, got {}",
            t.node_capacity()
        );
    }
}
