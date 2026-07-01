use editor_crdt::Dot;

use super::{SpanLog, SpanOp};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpanAnchorIndex {
    by_anchor: imbl::HashMap<Dot, Vec<Dot>>,
}

impl SpanAnchorIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(spans: &SpanLog) -> Self {
        let mut idx = Self::new();
        for (op_dot, op) in spans.iter() {
            idx.add(*op_dot, op);
        }
        idx
    }

    pub fn add(&mut self, op_dot: Dot, op: &SpanOp) {
        let (s, e) = op.anchors();
        self.link(s.id, op_dot);
        if e.id != s.id {
            self.link(e.id, op_dot);
        }
    }

    pub fn remove(&mut self, op_dot: Dot, op: &SpanOp) {
        let (s, e) = op.anchors();
        self.unlink(s.id, op_dot);
        if e.id != s.id {
            self.unlink(e.id, op_dot);
        }
    }

    pub fn spans_at(&self, anchor: Dot) -> &[Dot] {
        self.by_anchor
            .get(&anchor)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn spans_near(&self, anchors: impl IntoIterator<Item = Dot>) -> Vec<Dot> {
        let mut out: Vec<Dot> = Vec::new();
        for a in anchors {
            for &op in self.spans_at(a) {
                if let Err(i) = out.binary_search(&op) {
                    out.insert(i, op);
                }
            }
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        self.by_anchor.is_empty()
    }

    fn link(&mut self, anchor: Dot, op_dot: Dot) {
        let v = self.by_anchor.entry(anchor).or_default();
        if let Err(i) = v.binary_search(&op_dot) {
            v.insert(i, op_dot);
        }
    }

    fn unlink(&mut self, anchor: Dot, op_dot: Dot) {
        if let Some(v) = self.by_anchor.get_mut(&anchor) {
            if let Ok(i) = v.binary_search(&op_dot) {
                v.remove(i);
            }
            if v.is_empty() {
                self.by_anchor.remove(&anchor);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use proptest::prelude::*;

    use super::*;
    use crate::Modifier;
    use crate::span::{Anchor, Bias};

    fn anc(actor: u64, clock: u64, bias: Bias) -> Anchor {
        Anchor {
            id: Dot::new(actor, clock),
            bias,
        }
    }

    fn add_op(s: (u64, u64), e: (u64, u64), m: Modifier) -> SpanOp {
        SpanOp::AddSpan {
            start: anc(s.0, s.1, Bias::Before),
            end: anc(e.0, e.1, Bias::After),
            modifier: m,
        }
    }

    fn scan_at(spans: &[(Dot, SpanOp)], anchor: Dot) -> BTreeSet<Dot> {
        spans
            .iter()
            .filter(|(_, op)| {
                let (s, e) = op.anchors();
                s.id == anchor || e.id == anchor
            })
            .map(|(d, _)| *d)
            .collect()
    }

    #[test]
    fn build_indexes_both_anchors() {
        let op = add_op((1, 0), (1, 5), Modifier::Bold);
        let d = Dot::new(2, 0);
        let log = SpanLog::new().apply(d, op).unwrap();
        let idx = SpanAnchorIndex::build(&log);
        assert_eq!(idx.spans_at(Dot::new(1, 0)), &[d]);
        assert_eq!(idx.spans_at(Dot::new(1, 5)), &[d]);
        assert!(idx.spans_at(Dot::new(9, 9)).is_empty());
    }

    #[test]
    fn collapsed_anchor_indexed_once() {
        let op = SpanOp::AddSpan {
            start: anc(1, 0, Bias::Before),
            end: anc(1, 0, Bias::After),
            modifier: Modifier::Italic,
        };
        let d = Dot::new(2, 0);
        let log = SpanLog::new().apply(d, op).unwrap();
        let idx = SpanAnchorIndex::build(&log);
        assert_eq!(idx.spans_at(Dot::new(1, 0)), &[d]);
    }

    #[test]
    fn remove_unlinks_both_anchors() {
        let op = add_op((1, 0), (1, 5), Modifier::Bold);
        let d = Dot::new(2, 0);
        let mut idx = SpanAnchorIndex::new();
        idx.add(d, &op);
        idx.remove(d, &op);
        assert!(idx.is_empty());
    }

    #[test]
    fn spans_near_unions_and_dedups() {
        let a = add_op((1, 0), (1, 5), Modifier::Bold);
        let b = add_op((1, 5), (1, 9), Modifier::Italic);
        let da = Dot::new(2, 0);
        let db = Dot::new(2, 1);
        let log = SpanLog::new().apply(da, a).unwrap().apply(db, b).unwrap();
        let idx = SpanAnchorIndex::build(&log);
        let near = idx.spans_near([Dot::new(1, 5)]);
        assert_eq!(near, vec![da, db].into_iter().collect::<Vec<_>>());
    }

    fn arb_modifier() -> impl Strategy<Value = Modifier> {
        prop_oneof![Just(Modifier::Bold), Just(Modifier::Italic)]
    }

    fn arb_span_op() -> impl Strategy<Value = SpanOp> {
        let anchor = (
            0u64..4,
            0u64..6,
            prop_oneof![Just(Bias::Before), Just(Bias::After)],
        )
            .prop_map(|(a, c, b)| Anchor {
                id: Dot::new(a, c),
                bias: b,
            });
        prop_oneof![
            (anchor.clone(), anchor.clone(), arb_modifier()).prop_map(|(s, e, m)| {
                SpanOp::AddSpan {
                    start: s,
                    end: e,
                    modifier: m,
                }
            }),
            (anchor.clone(), anchor, arb_modifier()).prop_map(|(s, e, m)| SpanOp::RemoveSpan {
                start: s,
                end: e,
                modifier_type: m.as_type(),
            }),
        ]
    }

    fn paired(ops: &[SpanOp]) -> Vec<(Dot, SpanOp)> {
        ops.iter()
            .enumerate()
            .map(|(i, op)| (Dot::new(5, i as u64), op.clone()))
            .collect()
    }

    fn anchors_seen(pairs: &[(Dot, SpanOp)]) -> BTreeSet<Dot> {
        let mut s = BTreeSet::new();
        for (_, op) in pairs {
            let (a, b) = op.anchors();
            s.insert(a.id);
            s.insert(b.id);
        }
        s
    }

    proptest! {
        #[test]
        fn spans_at_matches_full_scan(ops in prop::collection::vec(arb_span_op(), 0..24)) {
            let pairs = paired(&ops);
            let mut log = SpanLog::new();
            for (d, op) in &pairs {
                log = log.apply(*d, op.clone()).unwrap();
            }
            let idx = SpanAnchorIndex::build(&log);
            for anchor in anchors_seen(&pairs) {
                let got: BTreeSet<Dot> = idx.spans_at(anchor).iter().copied().collect();
                prop_assert_eq!(got, scan_at(&pairs, anchor), "anchor {:?}", anchor);
            }
        }

        #[test]
        fn incremental_add_matches_build(ops in prop::collection::vec(arb_span_op(), 0..24)) {
            let pairs = paired(&ops);
            let mut log = SpanLog::new();
            let mut idx = SpanAnchorIndex::new();
            for (d, op) in &pairs {
                log = log.apply(*d, op.clone()).unwrap();
                idx.add(*d, op);
            }
            prop_assert_eq!(idx, SpanAnchorIndex::build(&log));
        }

        #[test]
        fn add_then_remove_restores(ops in prop::collection::vec(arb_span_op(), 0..24)) {
            let pairs = paired(&ops);
            let mut idx = SpanAnchorIndex::new();
            for (d, op) in &pairs {
                idx.add(*d, op);
            }
            let full = idx.clone();
            // re-adding is idempotent; removing then re-adding restores the index
            for (d, op) in &pairs {
                idx.remove(*d, op);
            }
            prop_assert!(idx.is_empty());
            for (d, op) in &pairs {
                idx.add(*d, op);
            }
            prop_assert_eq!(idx, full);
        }
    }
}
