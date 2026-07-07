mod derive;
pub use derive::*;

mod effective;
pub use effective::{
    EffectiveSources, OwnModifier, derive_block_effective, own_modifiers_for_leaf,
    resolve_effective,
};

mod anchor;
pub use anchor::*;

mod coverage;
pub use coverage::*;

mod covering;
pub use covering::*;

mod segs;
pub use segs::*;

use editor_crdt::{CrdtError, Dot};
use serde::{Deserialize, Serialize};

use crate::{Modifier, ModifierType};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bias {
    Before,
    After,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Anchor {
    pub id: Dot,
    pub bias: Bias,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SpanOp {
    AddSpan {
        start: Anchor,
        end: Anchor,
        modifier: Modifier,
    },
    RemoveSpan {
        start: Anchor,
        end: Anchor,
        modifier_type: ModifierType,
    },
}

impl From<Bias> for editor_crdt::sequence::Bias {
    fn from(b: Bias) -> Self {
        match b {
            Bias::Before => editor_crdt::sequence::Bias::Before,
            Bias::After => editor_crdt::sequence::Bias::After,
        }
    }
}

impl SpanOp {
    pub fn anchors(&self) -> (Anchor, Anchor) {
        match self {
            SpanOp::AddSpan { start, end, .. } | SpanOp::RemoveSpan { start, end, .. } => {
                (*start, *end)
            }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpanLog {
    ops: imbl::HashMap<Dot, SpanOp>,
}

impl SpanLog {
    pub fn new() -> Self {
        Self {
            ops: imbl::HashMap::new(),
        }
    }

    pub fn apply(&self, id: Dot, op: SpanOp) -> Result<Self, CrdtError> {
        if let Some(existing) = self.ops.get(&id) {
            if *existing != op {
                return Err(CrdtError::DotConflict { dot: id });
            }
            return Ok(self.clone());
        }
        Ok(Self {
            ops: self.ops.update(id, op),
        })
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Dot, &SpanOp)> + '_ {
        self.ops.iter()
    }

    pub fn get(&self, dot: Dot) -> Option<&SpanOp> {
        self.ops.get(&dot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn anc(actor: u64, clock: u64, bias: Bias) -> Anchor {
        Anchor {
            id: Dot::new(actor, clock),
            bias,
        }
    }

    #[test]
    fn bias_converts_to_crdt_bias() {
        use editor_crdt::sequence::Bias as CrdtBias;
        assert_eq!(CrdtBias::from(Bias::Before), CrdtBias::Before);
        assert_eq!(CrdtBias::from(Bias::After), CrdtBias::After);
    }

    #[test]
    fn span_op_anchors_returns_start_end() {
        let s = anc(1, 0, Bias::Before);
        let e = anc(1, 5, Bias::After);
        let add = SpanOp::AddSpan {
            start: s,
            end: e,
            modifier: Modifier::Bold,
        };
        assert_eq!(add.anchors(), (s, e));
    }

    fn add(a: u64, c: u64, m: Modifier) -> SpanOp {
        SpanOp::AddSpan {
            start: anc(a, c, Bias::Before),
            end: anc(a, c + 1, Bias::After),
            modifier: m,
        }
    }

    #[test]
    fn apply_add_then_len_one() {
        let log = SpanLog::new()
            .apply(Dot::new(1, 0), add(1, 0, Modifier::Bold))
            .unwrap();
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn apply_same_dot_same_op_idempotent() {
        let op = add(1, 0, Modifier::Bold);
        let a = SpanLog::new().apply(Dot::new(1, 0), op.clone()).unwrap();
        let b = a.apply(Dot::new(1, 0), op).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn apply_same_dot_diff_op_conflicts() {
        let d = Dot::new(1, 0);
        let a = SpanLog::new().apply(d, add(1, 0, Modifier::Bold)).unwrap();
        let err = a.apply(d, add(1, 0, Modifier::Italic)).unwrap_err();
        assert_eq!(err, editor_crdt::CrdtError::DotConflict { dot: d });
    }

    #[test]
    fn add_and_remove_coexist() {
        let log = SpanLog::new()
            .apply(Dot::new(1, 0), add(1, 0, Modifier::Bold))
            .unwrap()
            .apply(
                Dot::new(1, 1),
                SpanOp::RemoveSpan {
                    start: anc(1, 0, Bias::Before),
                    end: anc(1, 1, Bias::After),
                    modifier_type: ModifierType::Bold,
                },
            )
            .unwrap();
        assert_eq!(log.len(), 2);
    }

    fn arb_modifier() -> impl Strategy<Value = Modifier> {
        prop_oneof![
            Just(Modifier::Bold),
            Just(Modifier::Italic),
            any::<u32>().prop_map(|v| Modifier::FontSize { value: v }),
            ".*".prop_map(|s| Modifier::Link { href: s }),
        ]
    }

    fn arb_span_op() -> impl Strategy<Value = SpanOp> {
        let anchor = (
            any::<u64>(),
            any::<u64>(),
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
            (anchor.clone(), anchor, arb_modifier()).prop_map(|(s, e, m)| {
                SpanOp::RemoveSpan {
                    start: s,
                    end: e,
                    modifier_type: m.as_type(),
                }
            }),
        ]
    }

    fn paired(ops: &[SpanOp]) -> Vec<(Dot, SpanOp)> {
        ops.iter()
            .enumerate()
            .map(|(i, op)| (Dot::new((i as u64 % 4) + 1, i as u64), op.clone()))
            .collect()
    }

    fn permute<T: Clone>(items: &[T], seed: u64) -> Vec<T> {
        let mut indexed: Vec<(u64, T)> = items
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let mut z = (i as u64).wrapping_add(seed.wrapping_mul(0x9E3779B97F4A7C15));
                z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
                z ^= z >> 31;
                (z, x.clone())
            })
            .collect();
        indexed.sort_by_key(|(k, _)| *k);
        indexed.into_iter().map(|(_, x)| x).collect()
    }

    fn apply_all(pairs: &[(Dot, SpanOp)]) -> SpanLog {
        let mut log = SpanLog::new();
        for (d, op) in pairs {
            log = log
                .apply(*d, op.clone())
                .expect("distinct dots never conflict");
        }
        log
    }

    proptest! {
        #[test]
        fn span_log_converges_under_permutation(
            ops in prop::collection::vec(arb_span_op(), 0..32),
            seed in any::<u64>(),
        ) {
            let pairs = paired(&ops);
            let permuted = permute(&pairs, seed);
            prop_assert_eq!(apply_all(&pairs), apply_all(&permuted));
        }

        #[test]
        fn span_log_idempotent_under_permutation(
            ops in prop::collection::vec(arb_span_op(), 0..32),
            seed in any::<u64>(),
        ) {
            let pairs = paired(&ops);
            let once = apply_all(&pairs);
            let doubled: Vec<(Dot, SpanOp)> = pairs.iter().flat_map(|p| [p.clone(), p.clone()]).collect();
            let twice = apply_all(&permute(&doubled, seed));
            prop_assert_eq!(once, twice);
        }
    }
}
