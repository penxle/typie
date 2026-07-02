use std::collections::{BTreeMap, HashMap};

use editor_crdt::Dot;
use editor_crdt::sequence::SeqResolve;

use super::{ExplicitEffect, SpanLog};
use crate::seq::SeqItem;
use crate::{ModifierType, NodeType, Schema};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LeafSpanCoverage {
    by_leaf: imbl::HashMap<Dot, Vec<Dot>>,
}

struct ResolvedSpan {
    op_dot: Dot,
    start: usize,
    end: usize,
}

impl LeafSpanCoverage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build<R: SeqResolve>(
        elements: &[(Dot, SeqItem)],
        spans: &SpanLog,
        resolver: &R,
    ) -> Self {
        // No visible leaves ⇒ no coverage to compute. Skip resolving the whole span log
        // (`O(all spans)`), which a reproject after a select-all-delete would otherwise
        // pay for a now-empty document.
        if elements.is_empty() {
            return Self::default();
        }
        let resolved = resolve_spans(spans, resolver);
        let mut by_leaf = imbl::HashMap::new();
        for (pos, (dot, _)) in elements.iter().enumerate() {
            let mut cov: Vec<Dot> = resolved
                .iter()
                .filter(|r| r.start <= pos && pos < r.end)
                .map(|r| r.op_dot)
                .collect();
            if !cov.is_empty() {
                cov.sort();
                by_leaf.insert(*dot, cov);
            }
        }
        Self { by_leaf }
    }

    pub fn covering(&self, leaf: Dot) -> &[Dot] {
        self.by_leaf.get(&leaf).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn is_empty(&self) -> bool {
        self.by_leaf.is_empty()
    }

    pub fn set(&mut self, leaf: Dot, mut spans: Vec<Dot>) {
        if spans.is_empty() {
            self.by_leaf.remove(&leaf);
        } else {
            spans.sort();
            spans.dedup();
            self.by_leaf.insert(leaf, spans);
        }
    }

    pub fn remove_leaf(&mut self, leaf: Dot) {
        self.by_leaf.remove(&leaf);
    }

    pub fn add_span_to(&mut self, leaf: Dot, span: Dot) {
        let v = self.by_leaf.entry(leaf).or_default();
        if let Err(i) = v.binary_search(&span) {
            v.insert(i, span);
        }
    }

    pub fn remove_span_from(&mut self, leaf: Dot, span: Dot) {
        if let Some(v) = self.by_leaf.get_mut(&leaf) {
            if let Ok(i) = v.binary_search(&span) {
                v.remove(i);
            }
            if v.is_empty() {
                self.by_leaf.remove(&leaf);
            }
        }
    }
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
/// every call; looping it over a block's leaves is `O(leaves · spans · log)`. Build
/// this once and call [`ResolvedSpans::covering`] per leaf for `O(leaves · spans)`
/// integer comparisons with no repeated boundary resolution.
pub struct ResolvedSpans {
    spans: Vec<ResolvedSpan>,
}

impl ResolvedSpans {
    pub fn build<R: SeqResolve>(spans: &SpanLog, resolver: &R) -> Self {
        Self {
            spans: resolve_spans(spans, resolver),
        }
    }

    pub fn covering(&self, pos: usize) -> Vec<Dot> {
        let mut out: Vec<Dot> = self
            .spans
            .iter()
            .filter(|r| r.start <= pos && pos < r.end)
            .map(|r| r.op_dot)
            .collect();
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
) -> BTreeMap<ModifierType, ExplicitEffect> {
    let mut by_type: HashMap<ModifierType, (Dot, Option<ExplicitEffect>)> = HashMap::new();
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
    use std::collections::BTreeSet;

    use editor_crdt::sequence::{Bias as CrdtBias, Boundary};

    use super::*;
    use crate::Modifier;
    use crate::span::{Anchor, Bias, SpanOp};

    struct Mock {
        // (anchor dot, crdt bias) -> visible position
        at: HashMap<(Dot, CrdtBias), usize>,
    }

    impl SeqResolve for Mock {
        fn resolve_boundary(&self, id: Dot, bias: CrdtBias) -> Option<Boundary> {
            self.at.get(&(id, bias)).map(|&p| Boundary {
                position: p,
                visible: true,
            })
        }
    }

    fn mk(entries: &[(u64, (u64, Bias, usize), (u64, Bias, usize), Modifier)]) -> (SpanLog, Mock) {
        let mut log = SpanLog::new();
        let mut at = HashMap::new();
        for (op_clock, (sa, sb, sp), (ea, eb, ep), m) in entries {
            let op = SpanOp::AddSpan {
                start: Anchor {
                    id: Dot::new(1, *sa),
                    bias: *sb,
                },
                end: Anchor {
                    id: Dot::new(1, *ea),
                    bias: *eb,
                },
                modifier: m.clone(),
            };
            log = log.apply(Dot::new(2, *op_clock), op).unwrap();
            at.insert((Dot::new(1, *sa), CrdtBias::from(*sb)), *sp);
            at.insert((Dot::new(1, *ea), CrdtBias::from(*eb)), *ep);
        }
        (log, Mock { at })
    }

    fn elements(n: usize) -> Vec<(Dot, SeqItem)> {
        (0..n)
            .map(|i| (Dot::new(3, i as u64), SeqItem::Char('a')))
            .collect()
    }

    fn brute_covering(pos: usize, entries: &[(u64, usize, usize)]) -> BTreeSet<Dot> {
        entries
            .iter()
            .filter(|(_, s, e)| *s <= pos && pos < *e)
            .map(|(c, _, _)| Dot::new(2, *c))
            .collect()
    }

    #[test]
    fn build_matches_brute_force_positional() {
        // span A: [1,4) bold, span B: [2,3) italic
        let (log, mock) = mk(&[
            (0, (0, Bias::Before, 1), (3, Bias::After, 4), Modifier::Bold),
            (
                1,
                (1, Bias::Before, 2),
                (2, Bias::After, 3),
                Modifier::Italic,
            ),
        ]);
        let cov = LeafSpanCoverage::build(&elements(6), &log, &mock);
        let ranges = [(0u64, 1usize, 4usize), (1, 2, 3)];
        for pos in 0..6 {
            let leaf = Dot::new(3, pos as u64);
            let got: BTreeSet<Dot> = cov.covering(leaf).iter().copied().collect();
            assert_eq!(
                got,
                brute_covering(pos, &ranges),
                "coverage mismatch at pos {pos}"
            );
        }
    }

    #[test]
    fn degenerate_span_covers_nothing() {
        // start resolves at 3, end at 3 -> dropped
        let (log, mock) = mk(&[(0, (0, Bias::After, 3), (1, Bias::Before, 3), Modifier::Bold)]);
        let cov = LeafSpanCoverage::build(&elements(5), &log, &mock);
        for pos in 0..5 {
            assert!(cov.covering(Dot::new(3, pos as u64)).is_empty());
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
            Some(&ExplicitEffect::Set(Modifier::FontSize { value: 2000 }))
        );
    }

    #[test]
    fn incremental_mutation_matches_set() {
        let mut cov = LeafSpanCoverage::new();
        let leaf = Dot::new(3, 0);
        cov.add_span_to(leaf, Dot::new(2, 5));
        cov.add_span_to(leaf, Dot::new(2, 1));
        cov.add_span_to(leaf, Dot::new(2, 5));
        assert_eq!(cov.covering(leaf), &[Dot::new(2, 1), Dot::new(2, 5)]);
        cov.remove_span_from(leaf, Dot::new(2, 1));
        assert_eq!(cov.covering(leaf), &[Dot::new(2, 5)]);
        cov.remove_span_from(leaf, Dot::new(2, 5));
        assert!(cov.covering(leaf).is_empty());
        assert!(cov.is_empty());
    }
}
