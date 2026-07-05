use std::collections::BTreeMap;
use std::sync::Arc;

use editor_crdt::Dot;

use super::{SpanLog, SpanOp};
use crate::{Modifier, ModifierType, NodeType, Schema};

/// Per-type LWW winner of the span ops covering a stretch. Per-type max is a
/// semilattice, so keeping only the winner is lossless for every future op
/// (local or remote): `winner' = max(winner, new)`. Bounded by #ModifierType —
/// history-independent, unlike the full covering set it replaces.
pub type Covering = BTreeMap<ModifierType, Dot>;
pub type SegCovering = Arc<Covering>;

pub fn covering_of_op(op: &SpanOp) -> ModifierType {
    match op {
        SpanOp::AddSpan { modifier, .. } => modifier.as_type(),
        SpanOp::RemoveSpan { modifier_type, .. } => *modifier_type,
    }
}

/// Absorb one covering op into the winner map. `None` = unchanged (the op
/// lost LWW for its type), so callers can share the existing Arc.
pub fn covering_absorb(
    cov: Option<&SegCovering>,
    ty: ModifierType,
    op_dot: Dot,
) -> Option<SegCovering> {
    if let Some(cur) = cov.and_then(|c| c.get(&ty))
        && *cur >= op_dot
    {
        return None;
    }
    let mut next: Covering = cov.map(|c| (**c).clone()).unwrap_or_default();
    next.insert(ty, op_dot);
    Some(Arc::new(next))
}

/// Target-filtered explicit effects of the winner ops — the winners-only
/// equivalent of `explicit_from_covering`.
pub fn explicit_from_winners(
    cov: &Covering,
    spans: &SpanLog,
    leaf_path: &[NodeType],
) -> BTreeMap<ModifierType, Modifier> {
    cov.iter()
        .filter_map(|(ty, dot)| {
            if !Schema::modifier_spec(*ty).target.matches(leaf_path) {
                return None;
            }
            let op = spans.get(*dot)?;
            let (_, effect) = super::derive::span_op_effect(op);
            effect.map(|e| (*ty, e))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use editor_crdt::Dot;

    use super::*;
    use crate::span::{Anchor, Bias, SpanLog, SpanOp};
    use crate::{Modifier, ModifierType, NodeType};

    fn add_op(id: u64, m: Modifier) -> (Dot, SpanOp) {
        (
            Dot::new(2, id),
            SpanOp::AddSpan {
                start: Anchor {
                    id: Dot::new(1, 0),
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: Dot::new(1, 9),
                    bias: Bias::After,
                },
                modifier: m,
            },
        )
    }

    #[test]
    fn absorb_keeps_per_type_max() {
        let c1 = covering_absorb(None, ModifierType::Bold, Dot::new(2, 5)).expect("new winner");
        // older remote op for the same type loses — no new map
        assert!(covering_absorb(Some(&c1), ModifierType::Bold, Dot::new(2, 3)).is_none());
        // newer op wins
        let c2 =
            covering_absorb(Some(&c1), ModifierType::Bold, Dot::new(2, 7)).expect("newer wins");
        assert_eq!(c2.get(&ModifierType::Bold), Some(&Dot::new(2, 7)));
        // different type is independent
        let c3 =
            covering_absorb(Some(&c2), ModifierType::Italic, Dot::new(2, 1)).expect("new type");
        assert_eq!(c3.len(), 2);
    }

    #[test]
    fn explicit_from_winners_matches_explicit_from_covering() {
        // Same scenario as coverage.rs::explicit_lww_picks_max_op_dot, via winners.
        let path = [NodeType::Root, NodeType::Paragraph, NodeType::Text];
        let mut log = SpanLog::new();
        let (lo_dot, lo_op) = add_op(0, Modifier::FontSize { value: 1000 });
        let (hi_dot, hi_op) = add_op(5, Modifier::FontSize { value: 2000 });
        log = log.apply(lo_dot, lo_op).unwrap();
        log = log.apply(hi_dot, hi_op).unwrap();
        let winners: Covering = [(ModifierType::FontSize, hi_dot)].into_iter().collect();
        let ex = explicit_from_winners(&winners, &log, &path);
        assert_eq!(
            ex.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 2000 })
        );
        // full covering list must agree with the winners-only resolution
        let full = crate::span::explicit_from_covering(&[lo_dot, hi_dot], &log, &path);
        assert_eq!(ex, full);
    }

    proptest::proptest! {
        #[test]
        fn winners_resolution_matches_full_set(ops in proptest::collection::vec((0u64..64, 0u8..6), 1..24)) {
            let path = [NodeType::Root, NodeType::Paragraph, NodeType::Text];
            let mut log = SpanLog::new();
            let mut dots: Vec<Dot> = Vec::new();
            let mut winners: Option<SegCovering> = None;
            for (clock, kind) in ops {
                let dot = Dot::new(2, clock);
                if dots.contains(&dot) { continue; }
                let op = match kind {
                    0 => SpanOp::AddSpan { start: Anchor { id: Dot::new(1,0), bias: Bias::Before }, end: Anchor { id: Dot::new(1,9), bias: Bias::After }, modifier: Modifier::Bold },
                    1 => SpanOp::AddSpan { start: Anchor { id: Dot::new(1,0), bias: Bias::Before }, end: Anchor { id: Dot::new(1,9), bias: Bias::After }, modifier: Modifier::Italic },
                    2 => SpanOp::AddSpan { start: Anchor { id: Dot::new(1,0), bias: Bias::Before }, end: Anchor { id: Dot::new(1,9), bias: Bias::After }, modifier: Modifier::FontWeight { value: 700 } },
                    3 => SpanOp::RemoveSpan { start: Anchor { id: Dot::new(1,0), bias: Bias::Before }, end: Anchor { id: Dot::new(1,9), bias: Bias::After }, modifier_type: ModifierType::Bold },
                    4 => SpanOp::RemoveSpan { start: Anchor { id: Dot::new(1,0), bias: Bias::Before }, end: Anchor { id: Dot::new(1,9), bias: Bias::After }, modifier_type: ModifierType::Italic },
                    _ => SpanOp::RemoveSpan { start: Anchor { id: Dot::new(1,0), bias: Bias::Before }, end: Anchor { id: Dot::new(1,9), bias: Bias::After }, modifier_type: ModifierType::FontWeight },
                };
                log = log.apply(dot, op.clone()).unwrap();
                dots.push(dot);
                if let Some(n) = covering_absorb(winners.as_ref(), covering_of_op(&op), dot) {
                    winners = Some(n);
                }
            }
            let full = crate::span::explicit_from_covering(&dots, &log, &path);
            let win = explicit_from_winners(winners.as_deref().unwrap(), &log, &path);
            proptest::prop_assert_eq!(win, full);
        }
    }
}
