use serde::{Deserialize, Serialize};

use crate::{CrdtError, Dot, ToPlain};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, editor_macros::Wire)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LwwRegOp<T> {
    #[wire(n(0))]
    Set {
        #[wire(n(0))]
        value: T,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, editor_macros::Wire)]
pub struct LwwReg<T> {
    #[wire(n(0))]
    last_set: Option<Dot>,
    #[wire(n(1))]
    value: T,
}

impl<T: Clone + PartialEq> LwwReg<T> {
    pub fn with_value(initial: T) -> Self {
        Self {
            last_set: None,
            value: initial,
        }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn last_set(&self) -> Option<Dot> {
        self.last_set
    }

    pub fn apply(&self, id: Dot, op: LwwRegOp<T>) -> Result<Self, CrdtError> {
        let LwwRegOp::Set { value } = op;
        match self.last_set {
            Some(current) if id == current => {
                if value != self.value {
                    return Err(CrdtError::DotConflict { dot: id });
                }
                Ok(self.clone())
            }
            Some(current) if id < current => Ok(self.clone()),
            _ => Ok(Self {
                last_set: Some(id),
                value,
            }),
        }
    }
}

impl<T: Clone + PartialEq> ToPlain for LwwReg<T> {
    type Plain = T;
    fn to_plain(&self) -> T {
        self.value.clone()
    }
}

impl<T: Clone + PartialEq + Default> LwwReg<T> {
    pub fn new() -> Self {
        Self::with_value(T::default())
    }
}

impl<T: Clone + PartialEq + Default> Default for LwwReg<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lwwreg_and_usize_wire_round_trip() {
        use crate::wire::{CollectCtx, DecCtx, EncCtx, Wire};
        fn rt<T: Wire + PartialEq + std::fmt::Debug>(v: &T) -> T {
            let mut cc = CollectCtx::new();
            v.collect(&mut cc);
            let (t, b) = cc.finalize();
            let ec = EncCtx::from_table(&t, b.clone());
            let dc = DecCtx {
                actor_table: t,
                baselines: b,
            };
            let mut buf = Vec::new();
            v.encode(&ec, &mut buf).unwrap();
            let mut s = &buf[..];
            T::decode(&dc, &mut s).unwrap()
        }
        assert_eq!(rt(&12345usize), 12345usize);
        let a = LwwReg::with_value(7u32)
            .apply(Dot::new(1, 0), LwwRegOp::Set { value: 42 })
            .unwrap();
        assert_eq!(rt(&a), a);
        let b = LwwReg::with_value(Some("x".to_string()));
        assert_eq!(rt(&b), b);
    }

    #[test]
    fn with_value_initial_state() {
        let reg: LwwReg<u32> = LwwReg::with_value(42);
        assert_eq!(reg.get(), &42);
        assert_eq!(reg.last_set(), None);
    }

    #[test]
    fn new_uses_default() {
        let reg: LwwReg<u32> = LwwReg::new();
        assert_eq!(reg.get(), &0);
        assert_eq!(reg.last_set(), None);
    }

    #[test]
    fn default_impl_matches_new() {
        let a: LwwReg<u32> = LwwReg::new();
        let b: LwwReg<u32> = LwwReg::default();
        assert_eq!(a, b);
    }

    #[test]
    fn apply_first_set_wins_against_initial() {
        let reg = LwwReg::with_value(0u32);
        let dot = Dot::new(7, 3);
        let next = reg.apply(dot, LwwRegOp::Set { value: 99 }).unwrap();
        assert_eq!(next.get(), &99);
        assert_eq!(next.last_set(), Some(dot));
    }

    #[test]
    fn silent_skip_on_lower_dot() {
        let reg = LwwReg::with_value(0u32);
        let high = Dot::new(1, 10);
        let low = Dot::new(1, 5);
        let after_high = reg.apply(high, LwwRegOp::Set { value: 99 }).unwrap();
        let after_low = after_high.apply(low, LwwRegOp::Set { value: 77 }).unwrap();
        assert_eq!(after_low.get(), &99);
        assert_eq!(after_low.last_set(), Some(high));
    }

    #[test]
    fn same_dot_same_value_idempotent() {
        let reg = LwwReg::with_value(0u32);
        let dot = Dot::new(1, 5);
        let after = reg.apply(dot, LwwRegOp::Set { value: 42 }).unwrap();
        let again = after.apply(dot, LwwRegOp::Set { value: 42 }).unwrap();
        assert_eq!(after, again);
    }

    #[test]
    fn same_dot_different_value_returns_dot_conflict() {
        let reg = LwwReg::with_value(0u32);
        let dot = Dot::new(1, 5);
        let after = reg.apply(dot, LwwRegOp::Set { value: 42 }).unwrap();
        let err = after.apply(dot, LwwRegOp::Set { value: 99 }).unwrap_err();
        assert_eq!(err, CrdtError::DotConflict { dot });
    }

    #[test]
    fn winner_higher_clock_wins() {
        let reg = LwwReg::with_value(0u32);
        let lo = Dot::new(5, 2);
        let hi = Dot::new(1, 10);
        let r1 = reg.apply(lo, LwwRegOp::Set { value: 11 }).unwrap();
        let r2 = r1.apply(hi, LwwRegOp::Set { value: 22 }).unwrap();
        assert_eq!(r2.get(), &22);
        assert_eq!(r2.last_set(), Some(hi));
    }

    #[test]
    fn winner_actor_tiebreak_on_equal_clock() {
        let reg = LwwReg::with_value(0u32);
        let lo_actor = Dot::new(1, 5);
        let hi_actor = Dot::new(2, 5);
        let r1 = reg.apply(lo_actor, LwwRegOp::Set { value: 11 }).unwrap();
        let r2 = r1.apply(hi_actor, LwwRegOp::Set { value: 22 }).unwrap();
        assert_eq!(r2.get(), &22);
        assert_eq!(r2.last_set(), Some(hi_actor));
    }

    #[test]
    fn winner_same_actor_higher_clock_wins() {
        let reg = LwwReg::with_value(0u32);
        let early = Dot::new(5, 1);
        let late = Dot::new(5, 7);
        let r1 = reg.apply(early, LwwRegOp::Set { value: 11 }).unwrap();
        let r2 = r1.apply(late, LwwRegOp::Set { value: 22 }).unwrap();
        assert_eq!(r2.get(), &22);
        assert_eq!(r2.last_set(), Some(late));
    }

    #[test]
    fn three_step_chain_last_wins() {
        let reg = LwwReg::with_value(0u32);
        let d1 = Dot::new(1, 1);
        let d2 = Dot::new(1, 2);
        let d3 = Dot::new(1, 3);
        let r = reg
            .apply(d1, LwwRegOp::Set { value: 10 })
            .unwrap()
            .apply(d2, LwwRegOp::Set { value: 20 })
            .unwrap()
            .apply(d3, LwwRegOp::Set { value: 30 })
            .unwrap();
        assert_eq!(r.get(), &30);
        assert_eq!(r.last_set(), Some(d3));
    }

    #[test]
    fn interleaved_actors_converge_to_max_dot() {
        let reg = LwwReg::with_value(0u32);
        let a1c1 = Dot::new(1, 1);
        let a2c2 = Dot::new(2, 2);
        let a1c3 = Dot::new(1, 3);
        let a2c4 = Dot::new(2, 4);
        let r = reg
            .apply(a1c1, LwwRegOp::Set { value: 11 })
            .unwrap()
            .apply(a2c2, LwwRegOp::Set { value: 22 })
            .unwrap()
            .apply(a1c3, LwwRegOp::Set { value: 33 })
            .unwrap()
            .apply(a2c4, LwwRegOp::Set { value: 44 })
            .unwrap();
        assert_eq!(r.get(), &44);
        assert_eq!(r.last_set(), Some(a2c4));
    }

    #[test]
    fn lwwreg_to_plain_returns_winner_value() {
        let r = LwwReg::with_value(0u32)
            .apply(Dot::new(1, 0), LwwRegOp::Set { value: 42 })
            .unwrap();
        assert_eq!(r.to_plain(), 42);
    }

    #[test]
    fn lwwreg_to_plain_returns_initial_when_unmodified() {
        let r = LwwReg::with_value(7u32);
        assert_eq!(r.to_plain(), 7);
    }
}

#[cfg(test)]
mod proptests {
    use std::collections::HashSet;

    use super::*;
    use crate::test_utils::permute;
    use proptest::prelude::*;

    fn apply_all<T>(initial: T, ops: &[(Dot, LwwRegOp<T>)]) -> LwwReg<T>
    where
        T: Clone + PartialEq,
    {
        ops.iter()
            .fold(LwwReg::with_value(initial), |reg, (id, op)| {
                reg.apply(*id, op.clone())
                    .expect("arb_unique_ops produced a duplicate Dot — generator bug")
            })
    }

    fn arb_op() -> impl Strategy<Value = (Dot, LwwRegOp<u32>)> {
        (any::<u64>(), any::<u64>(), any::<u32>())
            .prop_map(|(actor, clock, value)| (Dot::new(actor, clock), LwwRegOp::Set { value }))
    }

    fn arb_unique_ops() -> impl Strategy<Value = Vec<(Dot, LwwRegOp<u32>)>> {
        prop::collection::vec(arb_op(), 0..30).prop_map(|ops| {
            let mut seen = HashSet::new();
            ops.into_iter()
                .filter(|(dot, _)| seen.insert(*dot))
                .collect()
        })
    }

    proptest! {
        #[test]
        fn convergence_under_permutation(
            ops in arb_unique_ops(),
            seed in any::<u64>(),
        ) {
            let a = apply_all(0u32, &ops);
            let permuted: Vec<_> = permute(&ops, seed);
            let b = apply_all(0u32, &permuted);
            prop_assert_eq!(a, b);
        }

        #[test]
        fn idempotency_under_permutation(
            ops in arb_unique_ops(),
            seed in any::<u64>(),
        ) {
            // Duplicate every op (same Dot, same value). Across permutations,
            // duplicates hit each `apply` case: case 1 (id == current winner,
            // value-equal idempotent), case 2 (id < current, silent skip),
            // case 3 (duplicate arrives before original is overtaken). Property
            // is `single == doubled` regardless of which case each duplicate hits.
            let mut doubled = ops.clone();
            doubled.extend(ops.iter().cloned());
            let single = apply_all(0u32, &ops);
            let doubled_permuted: Vec<_> = permute(&doubled, seed);
            let twice = apply_all(0u32, &doubled_permuted);
            prop_assert_eq!(single, twice);
        }

        #[test]
        fn winner_matches_max_dot(ops in arb_unique_ops()) {
            let init = 0u32;
            let ours = apply_all(init, &ops);

            // Independent reference: pick the op with the maximum Dot
            // (clock primary, actor tiebreak — `Dot::cmp` ordering).
            let reference = ops
                .iter()
                .max_by_key(|(d, _)| *d)
                .map(|(d, op)| {
                    let LwwRegOp::Set { value } = op;
                    (Some(*d), *value)
                })
                .unwrap_or((None, init));

            prop_assert_eq!(ours.last_set(), reference.0);
            prop_assert_eq!(*ours.get(), reference.1);
        }

        #[test]
        fn multi_actor_concurrent_set(
            actors in prop::collection::vec(any::<u64>(), 2..10)
                .prop_filter("actors must be unique", |actors| {
                    let mut sorted = actors.clone();
                    sorted.sort_unstable();
                    sorted.dedup();
                    sorted.len() == actors.len()
                }),
            clock in any::<u64>(),
            base_value in any::<u32>(),
        ) {
            // Every actor sets at the same clock — winner = max(actor) actor.
            let ops: Vec<_> = actors
                .iter()
                .enumerate()
                .map(|(i, &actor)| {
                    (Dot::new(actor, clock), LwwRegOp::Set {
                        value: base_value.wrapping_add(i as u32),
                    })
                })
                .collect();

            let reg = apply_all(0u32, &ops);
            let max_actor = *actors.iter().max().unwrap();
            let winner_idx = actors.iter().position(|&a| a == max_actor).unwrap();
            let expected_value = base_value.wrapping_add(winner_idx as u32);

            prop_assert_eq!(reg.last_set(), Some(Dot::new(max_actor, clock)));
            prop_assert_eq!(*reg.get(), expected_value);
        }
    }
}
