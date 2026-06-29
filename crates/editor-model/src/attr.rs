use std::collections::BTreeMap;

use editor_crdt::{CrdtError, Dot};
use serde::{Deserialize, Serialize};

use crate::{Modifier, ModifierType};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, editor_macros::Wire)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModifierAttrOp {
    #[wire(n(0))]
    SetModifier {
        #[wire(n(0))]
        target: Dot,
        #[wire(n(1))]
        modifier: Modifier,
    },
    #[wire(n(1))]
    ClearModifier {
        #[wire(n(0))]
        target: Dot,
        #[wire(n(1))]
        key: ModifierType,
    },
}

impl ModifierAttrOp {
    pub fn target_key(&self) -> (Dot, ModifierType) {
        match self {
            ModifierAttrOp::SetModifier { target, modifier } => (*target, modifier.as_type()),
            ModifierAttrOp::ClearModifier { target, key } => (*target, *key),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModifierAttrLog {
    ops: imbl::HashMap<Dot, ModifierAttrOp>,
}

impl ModifierAttrLog {
    pub fn new() -> Self {
        Self {
            ops: imbl::HashMap::new(),
        }
    }

    pub fn apply(&self, id: Dot, op: ModifierAttrOp) -> Result<Self, CrdtError> {
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

    pub fn modifiers_of(&self, target: Dot) -> BTreeMap<ModifierType, Modifier> {
        let mut by_type: BTreeMap<ModifierType, (Dot, Option<Modifier>)> = BTreeMap::new();
        for (op_dot, op) in &self.ops {
            let (t, k) = op.target_key();
            if t != target {
                continue;
            }
            let value = match op {
                ModifierAttrOp::SetModifier { modifier, .. } => Some(modifier.clone()),
                ModifierAttrOp::ClearModifier { .. } => None,
            };
            let win = by_type.get(&k).is_none_or(|(cur, _)| op_dot > cur);
            if win {
                by_type.insert(k, (*op_dot, value));
            }
        }
        by_type
            .into_iter()
            .filter_map(|(k, (_, v))| v.map(|m| (k, m)))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.ops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Dot, &ModifierAttrOp)> + '_ {
        self.ops.iter()
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn round_trip<T: editor_crdt::wire::Wire>(value: &T) -> editor_crdt::wire::WireResult<T> {
        use editor_crdt::wire::{CollectCtx, DecCtx, EncCtx, WireError};
        let mut cc = CollectCtx::new();
        value.collect(&mut cc);
        let (table, baselines) = cc.finalize();
        let ec = EncCtx::from_table(&table, baselines.clone());
        let dc = DecCtx {
            actor_table: table,
            baselines,
        };
        let mut buf = Vec::new();
        value.encode(&ec, &mut buf)?;
        let mut slice = &buf[..];
        let out = T::decode(&dc, &mut slice)?;
        if !slice.is_empty() {
            return Err(WireError::TrailingBytes {
                remaining: slice.len(),
            });
        }
        Ok(out)
    }

    #[test]
    fn modifier_attr_op_wire_round_trips() {
        let set = ModifierAttrOp::SetModifier {
            target: Dot::new(1, 0),
            modifier: Modifier::FontSize { value: 1600 },
        };
        let clear = ModifierAttrOp::ClearModifier {
            target: Dot::new(2, 3),
            key: ModifierType::Bold,
        };
        assert_eq!(round_trip(&set).unwrap(), set);
        assert_eq!(round_trip(&clear).unwrap(), clear);
    }

    #[test]
    fn target_key_derives_from_modifier() {
        let set = ModifierAttrOp::SetModifier {
            target: Dot::new(1, 0),
            modifier: Modifier::Bold,
        };
        assert_eq!(set.target_key(), (Dot::new(1, 0), ModifierType::Bold));
    }

    fn set(t: u64, m: Modifier) -> ModifierAttrOp {
        ModifierAttrOp::SetModifier {
            target: Dot::new(1, t),
            modifier: m,
        }
    }

    #[test]
    fn modifiers_of_returns_set_value() {
        let log = ModifierAttrLog::new()
            .apply(Dot::new(2, 0), set(1, Modifier::FontSize { value: 1600 }))
            .unwrap();
        let m = log.modifiers_of(Dot::new(1, 1));
        assert_eq!(
            m.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
    }

    #[test]
    fn clear_with_higher_dot_removes() {
        let target = Dot::new(1, 1);
        let log = ModifierAttrLog::new()
            .apply(
                Dot::new(2, 0),
                ModifierAttrOp::SetModifier {
                    target,
                    modifier: Modifier::Bold,
                },
            )
            .unwrap()
            .apply(
                Dot::new(3, 0),
                ModifierAttrOp::ClearModifier {
                    target,
                    key: ModifierType::Bold,
                },
            )
            .unwrap();
        assert!(
            !log.modifiers_of(target).contains_key(&ModifierType::Bold),
            "higher-Dot Clear가 제거"
        );
    }

    #[test]
    fn lww_higher_dot_value_wins() {
        let target = Dot::new(1, 1);
        let log = ModifierAttrLog::new()
            .apply(
                Dot::new(2, 0),
                ModifierAttrOp::SetModifier {
                    target,
                    modifier: Modifier::FontSize { value: 1200 },
                },
            )
            .unwrap()
            .apply(
                Dot::new(3, 0),
                ModifierAttrOp::SetModifier {
                    target,
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap();
        assert_eq!(
            log.modifiers_of(target).get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 1600 })
        );
    }

    #[test]
    fn apply_same_dot_diff_op_conflicts() {
        let d = Dot::new(2, 0);
        let a = ModifierAttrLog::new()
            .apply(d, set(1, Modifier::Bold))
            .unwrap();
        let err = a.apply(d, set(1, Modifier::Italic)).unwrap_err();
        assert_eq!(err, editor_crdt::CrdtError::DotConflict { dot: d });
    }

    #[test]
    fn modifiers_of_isolates_target() {
        let log = ModifierAttrLog::new()
            .apply(Dot::new(2, 0), set(1, Modifier::Bold))
            .unwrap()
            .apply(Dot::new(2, 1), set(2, Modifier::Italic))
            .unwrap();
        assert!(
            log.modifiers_of(Dot::new(1, 1))
                .contains_key(&ModifierType::Bold)
        );
        assert!(
            !log.modifiers_of(Dot::new(1, 1))
                .contains_key(&ModifierType::Italic)
        );
    }

    #[test]
    fn apply_same_dot_same_op_idempotent() {
        let op = set(1, Modifier::Bold);
        let a = ModifierAttrLog::new()
            .apply(Dot::new(2, 0), op.clone())
            .unwrap();
        let b = a.apply(Dot::new(2, 0), op).unwrap();
        assert_eq!(a, b);
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

    fn arb_modifier() -> impl Strategy<Value = Modifier> {
        prop_oneof![
            Just(Modifier::Bold),
            Just(Modifier::Italic),
            any::<u32>().prop_map(|v| Modifier::FontSize { value: v })
        ]
    }

    fn arb_attr_op() -> impl Strategy<Value = ModifierAttrOp> {
        (0u64..4, arb_modifier(), any::<bool>()).prop_map(|(t, m, clear)| {
            let target = Dot::new(1, t);
            if clear {
                ModifierAttrOp::ClearModifier {
                    target,
                    key: m.as_type(),
                }
            } else {
                ModifierAttrOp::SetModifier {
                    target,
                    modifier: m,
                }
            }
        })
    }

    fn apply_all(pairs: &[(Dot, ModifierAttrOp)]) -> ModifierAttrLog {
        let mut log = ModifierAttrLog::new();
        for (d, op) in pairs {
            log = log
                .apply(*d, op.clone())
                .expect("distinct dots never conflict");
        }
        log
    }

    fn reference_modifiers_of(
        log: &ModifierAttrLog,
        target: Dot,
    ) -> BTreeMap<ModifierType, Modifier> {
        let mut bucket: BTreeMap<ModifierType, Vec<(Dot, Option<Modifier>)>> = BTreeMap::new();
        for (op_dot, op) in log.iter() {
            let (t, k, value) = match op {
                ModifierAttrOp::SetModifier { target, modifier } => {
                    (*target, modifier.as_type(), Some(modifier.clone()))
                }
                ModifierAttrOp::ClearModifier { target, key } => (*target, *key, None),
            };
            if t != target {
                continue;
            }
            bucket.entry(k).or_default().push((*op_dot, value));
        }
        let mut out = BTreeMap::new();
        for (k, mut v) in bucket {
            v.sort_by_key(|(d, _)| *d);
            if let Some((_, Some(m))) = v.last() {
                out.insert(k, m.clone());
            }
        }
        out
    }

    proptest! {
        #[test]
        fn attr_log_converges_under_permutation(
            ops in prop::collection::vec(arb_attr_op(), 0..24),
            seed in any::<u64>(),
        ) {
            let pairs: Vec<(Dot, ModifierAttrOp)> =
                ops.iter().enumerate().map(|(i, op)| (Dot::new(9, i as u64), op.clone())).collect();
            prop_assert_eq!(apply_all(&pairs), apply_all(&permute(&pairs, seed)));
        }

        #[test]
        fn attr_log_idempotent_under_permutation(
            ops in prop::collection::vec(arb_attr_op(), 0..24),
            seed in any::<u64>(),
        ) {
            let pairs: Vec<(Dot, ModifierAttrOp)> =
                ops.iter().enumerate().map(|(i, op)| (Dot::new(9, i as u64), op.clone())).collect();
            let once = apply_all(&pairs);
            let doubled: Vec<(Dot, ModifierAttrOp)> = pairs.iter().flat_map(|p| [p.clone(), p.clone()]).collect();
            prop_assert_eq!(once, apply_all(&permute(&doubled, seed)));
        }

        #[test]
        fn modifiers_of_matches_reference(ops in prop::collection::vec(arb_attr_op(), 0..24)) {
            let pairs: Vec<(Dot, ModifierAttrOp)> =
                ops.iter().enumerate().map(|(i, op)| (Dot::new(9, i as u64), op.clone())).collect();
            let log = apply_all(&pairs);
            for t in 0u64..4 {
                let target = Dot::new(1, t);
                prop_assert_eq!(log.modifiers_of(target), reference_modifiers_of(&log, target));
            }
        }
    }
}
