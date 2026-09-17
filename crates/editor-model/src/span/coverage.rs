use hashbrown::HashMap;
use std::collections::BTreeMap;

use editor_crdt::Dot;
use editor_crdt::sequence::SeqResolve;

use super::SpanLog;
use crate::{Modifier, ModifierType, NodeType, Schema};

struct ResolvedSpan {
    op_dot: Dot,
    start: usize,
    end: usize,
}

fn resolve_spans<R: SeqResolve>(spans: &SpanLog, resolver: &R) -> Vec<ResolvedSpan> {
    spans
        .iter()
        .filter_map(|(op_dot, op)| {
            let (sa, ea) = op.anchors();
            let s = resolver.resolve_boundary(sa.id, sa.bias.into())?.position;
            let e = resolver.resolve_boundary(ea.id, ea.bias.into())?.position;
            if s >= e {
                return None;
            }
            Some(ResolvedSpan {
                op_dot: *op_dot,
                start: s,
                end: e,
            })
        })
        .collect()
}

/// The set of span ids whose resolved `[start,end]` cover visible position `pos`.
///
/// Naive reference resolve — the oracle baseline the interval index is verified
/// against. Not called on any warm production path; keep it independent of the
/// index so differential tests stay meaningful.
pub fn spans_covering<R: SeqResolve>(pos: usize, spans: &SpanLog, resolver: &R) -> Vec<Dot> {
    let mut out: Vec<Dot> = resolve_spans(spans, resolver)
        .into_iter()
        .filter(|r| r.start <= pos && pos < r.end)
        .map(|r| r.op_dot)
        .collect();
    out.sort();
    out
}

/// All spans resolved to `[start, end)` once, to query many positions cheaply.
/// `spans_covering` re-resolves every span (an `O(log)` boundary lookup each) on
/// every call. Build this once and call [`ResolvedSpans::covering`] per leaf: the
/// spans are kept sorted by `start` with the max `end` of every midpoint subtree,
/// so a query skips whole subtrees that end before the position — `O(log + hits)`
/// instead of a scan of every span per leaf.
pub struct ResolvedSpans {
    spans: Vec<ResolvedSpan>,
    max_end: Vec<usize>,
}

impl ResolvedSpans {
    pub fn build<R: SeqResolve>(spans: &SpanLog, resolver: &R) -> Self {
        Self::from_resolved(resolve_spans(spans, resolver))
    }

    fn from_resolved(mut spans: Vec<ResolvedSpan>) -> Self {
        spans.sort_by_key(|r| r.start);
        let mut max_end = vec![0; spans.len()];
        fn fill(spans: &[ResolvedSpan], max_end: &mut [usize], lo: usize, hi: usize) -> usize {
            if lo >= hi {
                return 0;
            }
            let mid = lo + (hi - lo) / 2;
            let own = spans[mid].end;
            let left = fill(spans, max_end, lo, mid);
            let right = fill(spans, max_end, mid + 1, hi);
            max_end[mid] = own.max(left).max(right);
            max_end[mid]
        }
        let len = spans.len();
        fill(&spans, &mut max_end, 0, len);
        Self { spans, max_end }
    }

    fn collect(&self, lo: usize, hi: usize, pos: usize, out: &mut Vec<Dot>) {
        if lo >= hi {
            return;
        }
        let mid = lo + (hi - lo) / 2;
        if self.max_end[mid] <= pos {
            return;
        }
        self.collect(lo, mid, pos, out);
        let span = &self.spans[mid];
        if span.start > pos {
            return;
        }
        if pos < span.end {
            out.push(span.op_dot);
        }
        self.collect(mid + 1, hi, pos, out);
    }

    pub fn covering(&self, pos: usize) -> Vec<Dot> {
        let mut out: Vec<Dot> = Vec::new();
        self.collect(0, self.spans.len(), pos, &mut out);
        out.sort();
        out
    }
}

/// Resolve a leaf's explicit effect from the span ids that cover it, applying the
/// per-modifier target filter and last-writer-wins (max op-dot) the same way
/// `derive_explicit_effect` does for the whole document.
pub fn explicit_from_covering(
    covering: &[Dot],
    spans: &SpanLog,
    leaf_path: &[NodeType],
) -> BTreeMap<ModifierType, Modifier> {
    let mut by_type: HashMap<ModifierType, (Dot, Option<Modifier>)> = HashMap::new();
    for &op_dot in covering {
        let Some(op) = spans.get(op_dot) else {
            continue;
        };
        let (ty, effect) = super::derive::span_op_effect(op);
        if !Schema::modifier_spec(ty).target.matches(leaf_path) {
            continue;
        }
        let win = match by_type.get(&ty) {
            Some((cur, _)) => op_dot > *cur,
            None => true,
        };
        if win {
            by_type.insert(ty, (op_dot, effect));
        }
    }
    by_type
        .into_iter()
        .filter_map(|(t, (_, e))| e.map(|e| (t, e)))
        .collect()
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::Modifier;
    use crate::span::{Anchor, Bias, SpanOp};

    proptest! {
        #[test]
        fn covering_matches_a_scan_of_every_span(
            ranges in proptest::collection::vec((0usize..40, 1usize..40), 0..80),
        ) {
            let resolved = |ranges: &[(usize, usize)]| -> Vec<ResolvedSpan> {
                ranges
                    .iter()
                    .enumerate()
                    .map(|(i, (start, len))| ResolvedSpan {
                        op_dot: Dot::new(1 + (i as u64 % 3), i as u64 / 2),
                        start: *start,
                        end: start + len,
                    })
                    .collect()
            };
            let index = ResolvedSpans::from_resolved(resolved(&ranges));
            for pos in 0..82 {
                let mut expected: Vec<Dot> = resolved(&ranges)
                    .iter()
                    .filter(|r| r.start <= pos && pos < r.end)
                    .map(|r| r.op_dot)
                    .collect();
                expected.sort();
                prop_assert_eq!(index.covering(pos), expected);
            }
        }
    }

    #[test]
    fn explicit_lww_picks_max_op_dot() {
        let path = [NodeType::Root, NodeType::Paragraph, NodeType::Text];
        // both Bold over the same leaf; op clock 5 should win over 0
        let mut log = SpanLog::new();
        let lo = Dot::new(2, 0);
        let hi = Dot::new(2, 5);
        log = log
            .apply(
                lo,
                SpanOp::AddSpan {
                    start: Anchor {
                        id: Dot::new(1, 0),
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: Dot::new(1, 9),
                        bias: Bias::After,
                    },
                    modifier: Modifier::FontSize { value: 1000 },
                },
            )
            .unwrap();
        log = log
            .apply(
                hi,
                SpanOp::AddSpan {
                    start: Anchor {
                        id: Dot::new(1, 0),
                        bias: Bias::Before,
                    },
                    end: Anchor {
                        id: Dot::new(1, 9),
                        bias: Bias::After,
                    },
                    modifier: Modifier::FontSize { value: 2000 },
                },
            )
            .unwrap();
        let ex = explicit_from_covering(&[lo, hi], &log, &path);
        assert_eq!(
            ex.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 2000 })
        );
    }
}
