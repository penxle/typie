use std::collections::HashMap;

use editor_crdt::Dot;

use super::covering::SegCovering;
use crate::NodeType;
use crate::projection::{LeafEff, LeafOwn};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seg {
    pub count: usize,
    pub leaf_type: NodeType,
    pub style: Option<String>,
    pub covering: Option<SegCovering>,
    /// Per-leaf-input singleton (`node_attrs` carrier or non-inline leaf):
    /// derivation reads per-leaf inputs the key can't capture, so it never merges.
    pub attrs_singleton: bool,
    pub eff: LeafEff,
    pub own: LeafOwn,
}

impl Seg {
    pub fn key_eq(&self, o: &Seg) -> bool {
        !self.attrs_singleton
            && !o.attrs_singleton
            && self.leaf_type == o.leaf_type
            && self.style == o.style
            && covering_eq(&self.covering, &o.covering)
    }
}

fn covering_eq(a: &Option<SegCovering>, b: &Option<SegCovering>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b) || a == b,
        _ => false,
    }
}

/// Run segments of a block: leaves with equal `(leaf_type, style, covering)`
/// coalesced into one entry, backed by a `SumTree` summed on `count` (the
/// segment's leaf span) so a leaf ordinal locates its owning segment in
/// `O(log segs)` via `find_by_offset` instead of an `O(segs)` walk.
#[derive(Clone, Debug, Default)]
pub struct BlockSegs {
    blocks: HashMap<Dot, editor_common::SumTree<Seg, usize>>,
}

impl BlockSegs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_block(&mut self, block: Dot, segs: Vec<Seg>) {
        let tree: editor_common::SumTree<Seg, usize> = segs
            .into_iter()
            .map(|s| {
                let n = s.count;
                (s, n)
            })
            .collect();
        self.blocks.insert(block, tree);
    }

    pub fn remove_block(&mut self, block: Dot) {
        self.blocks.remove(&block);
    }

    /// The segment covering leaf ordinal `ordinal` and the leaf's offset
    /// within it, or `None` past the block's last leaf.
    pub fn seg_at(&self, block: Dot, ordinal: usize) -> Option<(&Seg, usize)> {
        let tree = self.blocks.get(&block)?;
        let (idx, intra) = tree.find_by_offset(ordinal)?;
        tree.get(idx).map(|s| (s, intra))
    }

    pub fn group_iter(&self, block: Dot) -> impl Iterator<Item = &Seg> + '_ {
        self.blocks.get(&block).into_iter().flat_map(|t| t.iter())
    }

    pub fn leaf_count(&self, block: Dot) -> usize {
        self.blocks.get(&block).map(|t| t.total_size()).unwrap_or(0)
    }

    /// Total run-segment count across all blocks — the O(segments) memory
    /// metric (vs. O(leaves) under the old per-leaf maps).
    pub fn total_segs(&self) -> usize {
        self.blocks.values().map(|t| t.len()).sum()
    }

    /// Number of blocks with segment data.
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Insert a single leaf (`seg.count == 1`) at leaf ordinal `ordinal`,
    /// joining an adjacent same-key segment instead of always allocating a
    /// fresh one-leaf segment. Creates the block's entry if missing.
    pub fn insert_leaf(&mut self, block: Dot, ordinal: usize, seg: Seg) {
        let tree = self.blocks.entry(block).or_default();
        match tree.find_by_offset(ordinal) {
            None => {
                // Past the last leaf (or the block is empty): only a previous
                // segment can absorb it.
                if let Some(last_idx) = tree.len().checked_sub(1) {
                    let last = tree.get(last_idx).expect("last exists").clone();
                    if last.key_eq(&seg) {
                        let n = last.count + 1;
                        let mut merged = last;
                        merged.count = n;
                        tree.set(last_idx, merged, n);
                        return;
                    }
                }
                tree.push(seg, 1);
            }
            Some((idx, intra)) => {
                let at = tree.get(idx).expect("seg exists").clone();
                if intra == 0 {
                    // Ordinal sits on a boundary: either neighbor may absorb it.
                    if at.key_eq(&seg) {
                        let n = at.count + 1;
                        let mut merged = at;
                        merged.count = n;
                        tree.set(idx, merged, n);
                        return;
                    }
                    if idx > 0 {
                        let prev = tree.get(idx - 1).expect("prev exists").clone();
                        if prev.key_eq(&seg) {
                            let n = prev.count + 1;
                            let mut merged = prev;
                            merged.count = n;
                            tree.set(idx - 1, merged, n);
                            return;
                        }
                    }
                    tree.insert(idx, seg, 1);
                } else if at.key_eq(&seg) {
                    let n = at.count + 1;
                    let mut merged = at;
                    merged.count = n;
                    tree.set(idx, merged, n);
                } else {
                    let mut left = at.clone();
                    left.count = intra;
                    let mut right = at;
                    right.count -= intra;
                    let rn = right.count;
                    tree.set(idx, left, intra);
                    tree.insert(idx + 1, seg, 1);
                    tree.insert(idx + 2, right, rn);
                }
            }
        }
    }

    /// Remove the leaf at ordinal `ordinal`: shrink its segment, or — if it
    /// was the segment's only leaf — drop the segment and merge its
    /// now-adjacent neighbors when they share a key.
    pub fn remove_leaf(&mut self, block: Dot, ordinal: usize) {
        let Some(tree) = self.blocks.get_mut(&block) else {
            return;
        };
        let Some((idx, _)) = tree.find_by_offset(ordinal) else {
            return;
        };
        let seg = tree.get(idx).expect("seg exists").clone();
        if seg.count == 1 {
            tree.remove(idx);
            if idx > 0 && idx < tree.len() {
                let left = tree.get(idx - 1).expect("left exists").clone();
                let right = tree.get(idx).expect("right exists").clone();
                if left.key_eq(&right) {
                    let n = left.count + right.count;
                    let mut merged = left;
                    merged.count = n;
                    tree.set(idx - 1, merged, n);
                    tree.remove(idx);
                }
            }
        } else {
            let n = seg.count - 1;
            let mut updated = seg;
            updated.count = n;
            tree.set(idx, updated, n);
        }
    }

    /// Split at `lo`/`hi` so both land on segment boundaries, replace each
    /// fully-covered segment with `mutate(seg, start_ordinal)` (`None` =
    /// unchanged) where `start_ordinal` is the leaf ordinal of the segment's
    /// first leaf within the block (so callers can resolve a singleton seg's
    /// real leaf dot), then merge adjacent key-equal segments across the touched
    /// range — inclusive of the segments just outside `[lo, hi)`, since `mutate`
    /// can make a covered segment key-equal to an untouched neighbor. Returns
    /// whether `mutate` returned `Some` for any segment (a covering-only change
    /// counts, even when the derived state is unchanged).
    pub fn apply_range(
        &mut self,
        block: Dot,
        lo: usize,
        hi: usize,
        mutate: &mut dyn FnMut(&Seg, usize) -> Option<Seg>,
    ) -> bool {
        if lo >= hi {
            return false;
        }
        let Some(tree) = self.blocks.get_mut(&block) else {
            return false;
        };
        Self::split_at(tree, lo);
        Self::split_at(tree, hi);
        let start_idx = tree
            .find_by_offset(lo)
            .map(|(i, _)| i)
            .unwrap_or(tree.len());
        let end_idx = tree
            .find_by_offset(hi)
            .map(|(i, _)| i)
            .unwrap_or(tree.len());

        // The loop only replaces segments in place (count preserved), so seg
        // indices and ordinals stay put; track the running start ordinal from `lo`.
        let mut changed = false;
        let mut ord = lo;
        for idx in start_idx..end_idx {
            let seg = tree.get(idx).expect("seg exists").clone();
            if let Some(new) = mutate(&seg, ord) {
                let n = new.count;
                tree.set(idx, new, n);
                changed = true;
            }
            ord += seg.count;
        }

        // Merge pass over [start_idx - 1, end_idx] inclusive: the outer
        // neighbors weren't touched but may now key-match a mutated segment.
        let mut i = start_idx.saturating_sub(1);
        let mut right = end_idx;
        while i < right && i + 1 < tree.len() {
            let a = tree.get(i).expect("seg exists").clone();
            let b = tree.get(i + 1).expect("seg exists").clone();
            if a.key_eq(&b) {
                let n = a.count + b.count;
                let mut merged = a;
                merged.count = n;
                tree.set(i, merged, n);
                tree.remove(i + 1);
                right -= 1;
            } else {
                i += 1;
            }
        }
        changed
    }

    /// Split the segment spanning `offset` into two at that boundary, if it
    /// doesn't already fall on one. `O(log segs)`.
    fn split_at(tree: &mut editor_common::SumTree<Seg, usize>, offset: usize) {
        let Some((idx, intra)) = tree.find_by_offset(offset) else {
            return;
        };
        if intra == 0 {
            return;
        }
        let seg = tree.get(idx).expect("seg exists").clone();
        let mut left = seg.clone();
        left.count = intra;
        let mut right = seg;
        right.count -= intra;
        let rn = right.count;
        tree.set(idx, left, intra);
        tree.insert(idx + 1, right, rn);
    }
}

impl PartialEq for BlockSegs {
    /// Observable per-leaf equality over the union of block keys: each block's
    /// segments are expanded to their per-leaf `(leaf_type, style, eff, own)` tuples
    /// and the sequences compared. `covering` keys and the coalescing structure are
    /// excluded — a covering key is an internal derivation input, not observable, so a
    /// target-filtered span (which rewrites coverings but changes no `eff`) leaves the
    /// document observably unchanged. This backs both the cold-vs-warm equivalence
    /// tests and `state_observably_changed`; covering-key correctness is proven
    /// independently (and more strongly) by the log-derived oracle. An absent entry and
    /// an empty tree both expand to nothing, so they compare equal.
    fn eq(&self, o: &Self) -> bool {
        fn per_leaf(
            segs: &BlockSegs,
            block: Dot,
        ) -> impl Iterator<Item = (NodeType, &Option<String>, &LeafEff, &LeafOwn)> + '_ {
            segs.group_iter(block).flat_map(|s| {
                std::iter::repeat((s.leaf_type, &s.style, &s.eff, &s.own)).take(s.count)
            })
        }
        self.blocks
            .keys()
            .chain(o.blocks.keys())
            .all(|&b| per_leaf(self, b).eq(per_leaf(o, b)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Modifier, ModifierType, NodeType};
    use editor_crdt::Dot;
    use std::sync::Arc;

    fn eff(mods: &[Modifier]) -> crate::LeafEff {
        Arc::new(mods.iter().map(|m| (m.as_type(), m.clone())).collect())
    }
    fn plain_seg(count: usize) -> Seg {
        Seg {
            count,
            leaf_type: NodeType::Text,
            style: None,
            covering: None,
            attrs_singleton: false,
            eff: Default::default(),
            own: Default::default(),
        }
    }
    fn bold_seg(count: usize, dot: Dot) -> Seg {
        Seg {
            covering: Some(Arc::new([(ModifierType::Bold, dot)].into_iter().collect())),
            eff: eff(&[Modifier::Bold]),
            ..plain_seg(count)
        }
    }

    #[test]
    fn apply_range_splits_at_boundaries_and_merges_inside() {
        let block = Dot::new(7, 0);
        let mut segs = BlockSegs::new();
        segs.set_block(block, vec![plain_seg(10)]);
        let d = Dot::new(2, 1);
        let changed = segs.apply_range(block, 3, 7, &mut |s, _ord| {
            Some(Seg {
                covering: Some(Arc::new([(ModifierType::Bold, d)].into_iter().collect())),
                eff: eff(&[Modifier::Bold]),
                ..s.clone()
            })
        });
        assert!(changed);
        let got: Vec<(usize, bool)> = segs
            .group_iter(block)
            .map(|s| (s.count, s.covering.is_some()))
            .collect();
        assert_eq!(got, vec![(3, false), (4, true), (3, false)]);
        assert_eq!(segs.leaf_count(block), 10);
    }

    #[test]
    fn apply_range_merges_adjacent_equal_keys() {
        let block = Dot::new(7, 0);
        let d = Dot::new(2, 1);
        let mut segs = BlockSegs::new();
        segs.set_block(block, vec![bold_seg(4, d), plain_seg(2), bold_seg(4, d)]);
        // cover the plain middle with the same bold winner -> all three merge
        let changed = segs.apply_range(block, 4, 6, &mut |s, _ord| {
            Some(Seg {
                covering: Some(Arc::new([(ModifierType::Bold, d)].into_iter().collect())),
                eff: eff(&[Modifier::Bold]),
                ..s.clone()
            })
        });
        assert!(changed);
        let got: Vec<usize> = segs.group_iter(block).map(|s| s.count).collect();
        assert_eq!(got, vec![10]);
    }

    #[test]
    fn insert_leaf_joins_matching_seg_and_splits_otherwise() {
        let block = Dot::new(7, 0);
        let mut segs = BlockSegs::new();
        segs.set_block(block, vec![plain_seg(4)]);
        segs.insert_leaf(block, 2, plain_seg(1));
        assert_eq!(
            segs.group_iter(block).map(|s| s.count).collect::<Vec<_>>(),
            vec![5]
        );
        let d = Dot::new(2, 9);
        segs.insert_leaf(block, 2, bold_seg(1, d));
        assert_eq!(
            segs.group_iter(block).map(|s| s.count).collect::<Vec<_>>(),
            vec![2, 1, 3]
        );
    }

    #[test]
    fn remove_leaf_drops_empty_and_merges() {
        let block = Dot::new(7, 0);
        let d = Dot::new(2, 9);
        let mut segs = BlockSegs::new();
        segs.set_block(block, vec![plain_seg(2), bold_seg(1, d), plain_seg(3)]);
        segs.remove_leaf(block, 2);
        assert_eq!(
            segs.group_iter(block).map(|s| s.count).collect::<Vec<_>>(),
            vec![5]
        );
    }

    #[test]
    fn attrs_singleton_never_merges() {
        let block = Dot::new(7, 0);
        let mut segs = BlockSegs::new();
        let single = Seg {
            attrs_singleton: true,
            ..plain_seg(1)
        };
        segs.set_block(block, vec![plain_seg(2)]);
        segs.insert_leaf(block, 1, single);
        assert_eq!(
            segs.group_iter(block).map(|s| s.count).collect::<Vec<_>>(),
            vec![1, 1, 1]
        );
    }

    #[test]
    fn seg_at_returns_seg_and_intra_offset() {
        let block = Dot::new(7, 0);
        let d = Dot::new(2, 1);
        let mut segs = BlockSegs::new();
        segs.set_block(block, vec![plain_seg(3), bold_seg(2, d)]);
        let (s, intra) = segs.seg_at(block, 4).unwrap();
        assert!(s.covering.is_some());
        assert_eq!(intra, 1);
        assert!(segs.seg_at(block, 5).is_none());
    }

    fn stamp_eff(covering: &Option<SegCovering>) -> crate::LeafEff {
        if covering
            .as_ref()
            .is_some_and(|c| c.contains_key(&ModifierType::Bold))
        {
            eff(&[Modifier::Bold])
        } else {
            Default::default()
        }
    }

    // key: 0 plain, 1/2 bold under one of two competing actors, 3 attrs_singleton.
    fn model_seg(key: u8, clock: u8) -> Seg {
        match key % 4 {
            0 => plain_seg(1),
            1 => bold_seg(1, Dot::new(9, (clock % 6) as u64)),
            2 => bold_seg(1, Dot::new(10, (clock % 6) as u64)),
            _ => Seg {
                attrs_singleton: true,
                ..plain_seg(1)
            },
        }
    }

    /// Reference re-segmentation: merge adjacent per-leaf entries with equal
    /// `key_eq`, summing counts and keeping the earlier entry's payload — the
    /// same survivor rule `BlockSegs`'s merges use.
    fn group_model(model: &[Seg]) -> Vec<Seg> {
        let mut out: Vec<Seg> = Vec::new();
        for s in model {
            match out.last_mut() {
                Some(last) if last.key_eq(s) => last.count += 1,
                _ => out.push(s.clone()),
            }
        }
        out
    }

    #[derive(Clone, Debug)]
    enum ModelOp {
        Insert { pos_pct: u8, key: u8, clock: u8 },
        Remove { pos_pct: u8 },
        Range { lo_pct: u8, hi_pct: u8, clock: u8 },
    }

    fn arb_op() -> impl proptest::prelude::Strategy<Value = ModelOp> {
        use proptest::prelude::*;
        prop_oneof![
            (any::<u8>(), 0u8..4, 0u8..6).prop_map(|(pos_pct, key, clock)| ModelOp::Insert {
                pos_pct,
                key,
                clock
            }),
            any::<u8>().prop_map(|pos_pct| ModelOp::Remove { pos_pct }),
            (any::<u8>(), any::<u8>(), 0u8..6).prop_map(|(lo_pct, hi_pct, clock)| ModelOp::Range {
                lo_pct,
                hi_pct,
                clock
            }),
        ]
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig { cases: 128, ..proptest::prelude::ProptestConfig::default() })]
        #[test]
        fn block_segs_matches_naive_model(ops in proptest::collection::vec(arb_op(), 0..60)) {
            let block = Dot::new(7, 0);
            let mut segs = BlockSegs::new();
            let mut model: Vec<Seg> = Vec::new();

            for op in ops {
                match op {
                    ModelOp::Insert { pos_pct, key, clock } => {
                        let pos = (pos_pct as usize) % (model.len() + 1);
                        let seg = model_seg(key, clock);
                        model.insert(pos, seg.clone());
                        segs.insert_leaf(block, pos, seg);
                    }
                    ModelOp::Remove { pos_pct } => {
                        if model.is_empty() {
                            continue;
                        }
                        let pos = (pos_pct as usize) % model.len();
                        model.remove(pos);
                        segs.remove_leaf(block, pos);
                    }
                    ModelOp::Range { lo_pct, hi_pct, clock } => {
                        let len = model.len();
                        if len == 0 {
                            continue;
                        }
                        let a = (lo_pct as usize) % (len + 1);
                        let b = (hi_pct as usize) % (len + 1);
                        let (lo, hi) = if a < b {
                            (a, b)
                        } else if b < a {
                            (b, a)
                        } else {
                            continue;
                        };
                        let actor = if clock % 2 == 0 { 9 } else { 10 };
                        let dot = Dot::new(actor, (clock % 6) as u64);
                        for s in &mut model[lo..hi] {
                            if let Some(nc) = crate::covering_absorb(s.covering.as_ref(), ModifierType::Bold, dot) {
                                s.eff = stamp_eff(&Some(nc.clone()));
                                s.covering = Some(nc);
                            }
                        }
                        segs.apply_range(block, lo, hi, &mut |s, _ord| {
                            crate::covering_absorb(s.covering.as_ref(), ModifierType::Bold, dot).map(|nc| {
                                let mut ns = s.clone();
                                ns.eff = stamp_eff(&Some(nc.clone()));
                                ns.covering = Some(nc);
                                ns
                            })
                        });
                    }
                }

                proptest::prop_assert_eq!(segs.leaf_count(block), model.len());
                let actual: Vec<Seg> = segs.group_iter(block).cloned().collect();
                proptest::prop_assert_eq!(actual, group_model(&model));
            }
        }
    }
}
