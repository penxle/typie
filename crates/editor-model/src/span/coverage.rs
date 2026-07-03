use std::collections::{BTreeMap, HashMap};

use editor_crdt::Dot;
use editor_crdt::sequence::SeqResolve;

use super::{ExplicitEffect, SpanLog};
use crate::{ModifierType, NodeType, Schema};

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
    use super::*;
    use crate::Modifier;
    use crate::span::{Anchor, Bias, SpanOp};

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
}
