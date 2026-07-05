use editor_crdt::{CrdtError, Dot, LwwReg, LwwRegOp};
use serde::{Deserialize, Serialize};

use crate::Marker;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, editor_macros::Wire)]
pub struct NodeLwwOp<T> {
    #[wire(n(0))]
    pub target: Dot,
    #[wire(n(1))]
    pub op: LwwRegOp<T>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NodeLwwLog<T> {
    ops: imbl::HashMap<Dot, NodeLwwOp<T>>,
}

pub type NodeMarkerLog = NodeLwwLog<Option<Marker>>;

impl<T: Clone + PartialEq> NodeLwwLog<T> {
    pub fn new() -> Self {
        Self {
            ops: imbl::HashMap::new(),
        }
    }

    pub fn apply(&self, id: Dot, op: NodeLwwOp<T>) -> Result<Self, CrdtError> {
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

    pub fn iter(&self) -> impl Iterator<Item = (&Dot, &NodeLwwOp<T>)> + '_ {
        self.ops.iter()
    }
}

fn fold_lww<T: Clone + PartialEq>(reg: LwwReg<T>, op_id: Dot, op: LwwRegOp<T>) -> LwwReg<T> {
    match reg.apply(op_id, op) {
        Ok(next) => next,
        Err(e) => {
            debug_assert!(false, "NodeLwwLog fold: unexpected {e:?}");
            reg
        }
    }
}

impl<T: Clone + PartialEq + Default> NodeLwwLog<T> {
    pub fn value_of(&self, target: Dot) -> T {
        let mut reg: LwwReg<T> = LwwReg::new();
        for (op_id, entry) in &self.ops {
            if entry.target != target {
                continue;
            }
            reg = fold_lww(reg, *op_id, entry.op.clone());
        }
        reg.get().clone()
    }

    pub fn project(&self) -> imbl::HashMap<Dot, T> {
        let mut regs: imbl::HashMap<Dot, LwwReg<T>> = imbl::HashMap::new();
        for (op_id, entry) in &self.ops {
            let reg = regs
                .get(&entry.target)
                .cloned()
                .unwrap_or_else(|| LwwReg::new());
            regs.insert(entry.target, fold_lww(reg, *op_id, entry.op.clone()));
        }
        regs.into_iter()
            .map(|(dot, reg)| (dot, reg.get().clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
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

    fn lww_op(target: u64, value: Option<&str>) -> NodeLwwOp<Option<String>> {
        NodeLwwOp {
            target: Dot::new(1, target),
            op: LwwRegOp::Set {
                value: value.map(String::from),
            },
        }
    }

    #[test]
    fn apply_stores_op() {
        let log: NodeLwwLog<Option<String>> = NodeLwwLog::new()
            .apply(Dot::new(2, 0), lww_op(1, Some("a")))
            .unwrap();
        assert_eq!(log.len(), 1);
        assert!(!log.is_empty());
    }

    #[test]
    fn apply_same_dot_same_op_idempotent() {
        let o = lww_op(1, Some("a"));
        let a: NodeLwwLog<Option<String>> =
            NodeLwwLog::new().apply(Dot::new(2, 0), o.clone()).unwrap();
        let b = a.apply(Dot::new(2, 0), o).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn apply_same_dot_diff_op_conflicts() {
        let d = Dot::new(2, 0);
        let a: NodeLwwLog<Option<String>> =
            NodeLwwLog::new().apply(d, lww_op(1, Some("a"))).unwrap();
        let err = a.apply(d, lww_op(1, Some("b"))).unwrap_err();
        assert_eq!(err, CrdtError::DotConflict { dot: d });
    }

    #[test]
    fn lww_op_wire_round_trips() {
        let op: NodeLwwOp<Option<String>> = NodeLwwOp {
            target: Dot::new(1, 0),
            op: LwwRegOp::Set {
                value: Some("style-1".to_string()),
            },
        };
        assert_eq!(round_trip(&op).unwrap(), op);
    }

    #[test]
    fn marker_op_wire_round_trips() {
        let op: NodeLwwOp<Option<Marker>> = NodeLwwOp {
            target: Dot::new(2, 3),
            op: LwwRegOp::Set {
                value: Some(Marker { modifiers: vec![] }),
            },
        };
        assert_eq!(round_trip(&op).unwrap(), op);
    }

    #[test]
    fn value_of_applies_single_set() {
        let target = Dot::new(1, 1);
        let log: NodeLwwLog<Option<String>> = NodeLwwLog::new()
            .apply(Dot::new(2, 0), lww_op(1, Some("a")))
            .unwrap();
        assert_eq!(log.value_of(target), Some("a".to_string()));
    }

    #[test]
    fn value_of_no_ops_is_none() {
        let log: NodeLwwLog<Option<String>> = NodeLwwLog::new();
        assert_eq!(log.value_of(Dot::new(1, 9)), None);
    }

    #[test]
    fn value_of_lww_higher_dot_wins() {
        let target = Dot::new(1, 1);
        let log: NodeLwwLog<Option<String>> = NodeLwwLog::new()
            .apply(Dot::new(2, 0), lww_op(1, Some("a")))
            .unwrap()
            .apply(Dot::new(3, 0), lww_op(1, Some("b")))
            .unwrap();
        assert_eq!(log.value_of(target), Some("b".to_string()));
    }

    #[test]
    fn value_of_isolates_target() {
        let log: NodeLwwLog<Option<String>> = NodeLwwLog::new()
            .apply(Dot::new(2, 0), lww_op(1, Some("a")))
            .unwrap()
            .apply(Dot::new(2, 1), lww_op(2, Some("b")))
            .unwrap();
        assert_eq!(log.value_of(Dot::new(1, 1)), Some("a".to_string()));
    }

    #[test]
    fn value_of_supports_marker_alias() {
        let target = Dot::new(1, 1);
        let m = Marker { modifiers: vec![] };
        let log: NodeMarkerLog = NodeLwwLog::new()
            .apply(
                Dot::new(2, 0),
                NodeLwwOp {
                    target,
                    op: LwwRegOp::Set {
                        value: Some(m.clone()),
                    },
                },
            )
            .unwrap();
        assert_eq!(log.value_of(target), Some(m));
    }

    #[test]
    fn project_accumulates_per_node() {
        let log: NodeLwwLog<Option<String>> = NodeLwwLog::new()
            .apply(Dot::new(2, 0), lww_op(1, Some("a")))
            .unwrap()
            .apply(Dot::new(2, 1), lww_op(2, Some("b")))
            .unwrap();
        let projected = log.project();
        assert_eq!(projected.len(), 2);
        assert_eq!(projected.get(&Dot::new(1, 1)), Some(&Some("a".to_string())));
        assert_eq!(projected.get(&Dot::new(1, 2)), Some(&Some("b".to_string())));
    }

    #[test]
    fn project_omits_no_op_nodes() {
        let log: NodeLwwLog<Option<String>> = NodeLwwLog::new()
            .apply(Dot::new(2, 0), lww_op(1, Some("a")))
            .unwrap();
        let projected = log.project();
        assert!(projected.get(&Dot::new(1, 1)).is_some());
        assert!(projected.get(&Dot::new(1, 7)).is_none());
    }

    #[test]
    fn project_retains_explicit_set_none() {
        let a = Dot::new(1, 1);
        let log: NodeLwwLog<Option<String>> = NodeLwwLog::new()
            .apply(Dot::new(2, 0), lww_op(1, Some("a")))
            .unwrap()
            .apply(Dot::new(3, 0), lww_op(1, None))
            .unwrap();
        let projected = log.project();
        assert_eq!(projected.get(&a), Some(&None));
        assert_eq!(projected.get(&Dot::new(1, 9)), None);
    }

    #[test]
    fn project_matches_value_of() {
        let target = Dot::new(1, 1);
        let log: NodeLwwLog<Option<String>> = NodeLwwLog::new()
            .apply(Dot::new(2, 0), lww_op(1, Some("a")))
            .unwrap()
            .apply(Dot::new(3, 0), lww_op(1, Some("b")))
            .unwrap();
        let projected = log.project();
        assert_eq!(projected.get(&target), Some(&log.value_of(target)));
    }

    #[test]
    fn project_supports_marker_alias() {
        let target = Dot::new(1, 1);
        let m = Marker { modifiers: vec![] };
        let log: NodeMarkerLog = NodeLwwLog::new()
            .apply(
                Dot::new(2, 0),
                NodeLwwOp {
                    target,
                    op: LwwRegOp::Set {
                        value: Some(m.clone()),
                    },
                },
            )
            .unwrap();
        let projected = log.project();
        assert_eq!(projected.get(&target), Some(&Some(m)));
    }
}

#[cfg(test)]
mod proptests {
    use std::collections::HashSet;

    use proptest::prelude::*;

    use super::*;

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

    fn arb_style_ops() -> impl Strategy<Value = Vec<(Dot, Option<String>)>> {
        prop::collection::vec((any::<u64>(), any::<u64>(), any::<Option<String>>()), 0..30)
            .prop_map(|raw| {
                let mut seen = HashSet::new();
                raw.into_iter()
                    .map(|(actor, clock, value)| (Dot::new(actor, clock), value))
                    .filter(|(dot, _)| seen.insert(*dot))
                    .collect()
            })
    }

    fn build(pairs: &[(Dot, Option<String>)]) -> NodeLwwLog<Option<String>> {
        let mut log = NodeLwwLog::new();
        for (dot, value) in pairs {
            let op = NodeLwwOp {
                target: Dot::new(1, 1),
                op: LwwRegOp::Set {
                    value: value.clone(),
                },
            };
            log = log.apply(*dot, op).expect("distinct dots never conflict");
        }
        log
    }

    fn arb_style_op() -> impl Strategy<Value = NodeLwwOp<Option<String>>> {
        (0u64..4, any::<Option<String>>()).prop_map(|(t, value)| NodeLwwOp {
            target: Dot::new(1, t),
            op: LwwRegOp::Set { value },
        })
    }

    fn apply_all(pairs: &[(Dot, NodeLwwOp<Option<String>>)]) -> NodeLwwLog<Option<String>> {
        let mut log = NodeLwwLog::new();
        for (d, op) in pairs {
            log = log
                .apply(*d, op.clone())
                .expect("distinct dots never conflict");
        }
        log
    }

    proptest! {
        #[test]
        fn node_lww_log_converges_under_permutation(
            ops in prop::collection::vec(arb_style_op(), 0..24),
            seed in any::<u64>(),
        ) {
            let pairs: Vec<(Dot, NodeLwwOp<Option<String>>)> = ops
                .iter()
                .enumerate()
                .map(|(i, op)| (Dot::new(9, i as u64), op.clone()))
                .collect();
            prop_assert_eq!(apply_all(&pairs), apply_all(&permute(&pairs, seed)));
        }

        #[test]
        fn node_lww_log_idempotent_under_permutation(
            ops in prop::collection::vec(arb_style_op(), 0..24),
            seed in any::<u64>(),
        ) {
            let pairs: Vec<(Dot, NodeLwwOp<Option<String>>)> = ops
                .iter()
                .enumerate()
                .map(|(i, op)| (Dot::new(9, i as u64), op.clone()))
                .collect();
            let once = apply_all(&pairs);
            let doubled: Vec<(Dot, NodeLwwOp<Option<String>>)> =
                pairs.iter().flat_map(|p| [p.clone(), p.clone()]).collect();
            prop_assert_eq!(once, apply_all(&permute(&doubled, seed)));
        }

        #[test]
        fn value_of_matches_max_dot_reference(ops in arb_style_ops()) {
            let log = build(&ops);
            let reference = ops
                .iter()
                .max_by_key(|(d, _)| *d)
                .map(|(_, v)| v.clone())
                .unwrap_or(None);
            prop_assert_eq!(log.value_of(Dot::new(1, 1)), reference);
        }
    }
}
