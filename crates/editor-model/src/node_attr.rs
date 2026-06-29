use editor_crdt::{CrdtError, Dot};
use serde::{Deserialize, Serialize};

use crate::{ModelError, Node, NodeAttr, NodeType};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, editor_macros::Wire)]
pub struct NodeAttrOp {
    #[wire(n(0))]
    pub target: Dot,
    #[wire(n(1))]
    pub attr: NodeAttr,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NodeAttrLog {
    ops: imbl::HashMap<Dot, NodeAttrOp>,
}

impl NodeAttrLog {
    pub fn new() -> Self {
        Self {
            ops: imbl::HashMap::new(),
        }
    }

    pub fn apply(&self, id: Dot, op: NodeAttrOp) -> Result<Self, CrdtError> {
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

    pub fn iter(&self) -> impl Iterator<Item = (&Dot, &NodeAttrOp)> + '_ {
        self.ops.iter()
    }

    pub fn project(
        &self,
        node_type_of: impl Fn(Dot) -> Option<NodeType>,
    ) -> imbl::HashMap<Dot, Node> {
        let mut out: imbl::HashMap<Dot, Node> = imbl::HashMap::new();
        for (op_id, op) in &self.ops {
            let Some(node_type) = node_type_of(op.target) else {
                continue;
            };
            let mut node = out
                .get(&op.target)
                .cloned()
                .unwrap_or_else(|| node_type.into_node());
            if fold_op(&mut node, *op_id, &op.attr) {
                out.insert(op.target, node);
            }
        }
        out
    }

    pub fn attrs_of(&self, target: Dot, node_type: NodeType) -> Node {
        let mut node = node_type.into_node();
        for (op_id, op) in &self.ops {
            if op.target != target {
                continue;
            }
            fold_op(&mut node, *op_id, &op.attr);
        }
        node
    }
}

fn fold_op(node: &mut Node, op_id: Dot, attr: &NodeAttr) -> bool {
    match node.apply_attr(op_id, attr) {
        Ok(()) => true,
        Err(ModelError::AttrNodeKindMismatch) => false,
        Err(other) => {
            debug_assert!(false, "NodeAttrLog fold: unexpected {other:?}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CalloutNodeAttr, CalloutVariant};

    fn op(target: u64, attr: NodeAttr) -> NodeAttrOp {
        NodeAttrOp {
            target: Dot::new(1, target),
            attr,
        }
    }

    fn callout(v: CalloutVariant) -> NodeAttr {
        NodeAttr::Callout {
            attr: CalloutNodeAttr::Variant(v),
        }
    }

    #[test]
    fn apply_stores_op() {
        let log = NodeAttrLog::new()
            .apply(Dot::new(2, 0), op(1, callout(CalloutVariant::Warning)))
            .unwrap();
        assert_eq!(log.len(), 1);
        assert!(!log.is_empty());
    }

    #[test]
    fn apply_same_dot_same_op_idempotent() {
        let o = op(1, callout(CalloutVariant::Warning));
        let a = NodeAttrLog::new().apply(Dot::new(2, 0), o.clone()).unwrap();
        let b = a.apply(Dot::new(2, 0), o).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn apply_same_dot_diff_op_conflicts() {
        let d = Dot::new(2, 0);
        let a = NodeAttrLog::new()
            .apply(d, op(1, callout(CalloutVariant::Warning)))
            .unwrap();
        let err = a
            .apply(d, op(1, callout(CalloutVariant::Danger)))
            .unwrap_err();
        assert_eq!(err, CrdtError::DotConflict { dot: d });
    }

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
    fn node_attr_op_wire_round_trips() {
        let op = NodeAttrOp {
            target: Dot::new(1, 0),
            attr: NodeAttr::Callout {
                attr: CalloutNodeAttr::Variant(CalloutVariant::Warning),
            },
        };
        assert_eq!(round_trip(&op).unwrap(), op);
    }

    fn callout_variant(node: &Node) -> CalloutVariant {
        match node {
            Node::Callout(c) => *c.variant.get(),
            other => panic!("expected Callout, got {other:?}"),
        }
    }

    #[test]
    fn attrs_of_applies_single_set() {
        let target = Dot::new(1, 1);
        let log = NodeAttrLog::new()
            .apply(Dot::new(2, 0), op(1, callout(CalloutVariant::Warning)))
            .unwrap();
        let node = log.attrs_of(target, NodeType::Callout);
        assert_eq!(callout_variant(&node), CalloutVariant::Warning);
    }

    #[test]
    fn attrs_of_lww_higher_dot_wins() {
        let target = Dot::new(1, 1);
        let log = NodeAttrLog::new()
            .apply(Dot::new(2, 0), op(1, callout(CalloutVariant::Warning)))
            .unwrap()
            .apply(Dot::new(3, 0), op(1, callout(CalloutVariant::Danger)))
            .unwrap();
        assert_eq!(
            callout_variant(&log.attrs_of(target, NodeType::Callout)),
            CalloutVariant::Danger
        );
    }

    #[test]
    fn attrs_of_no_ops_returns_default() {
        let log = NodeAttrLog::new();
        assert_eq!(
            callout_variant(&log.attrs_of(Dot::new(1, 9), NodeType::Callout)),
            CalloutVariant::Info
        );
    }

    #[test]
    fn attrs_of_isolates_target() {
        let log = NodeAttrLog::new()
            .apply(Dot::new(2, 0), op(1, callout(CalloutVariant::Warning)))
            .unwrap()
            .apply(Dot::new(2, 1), op(2, callout(CalloutVariant::Danger)))
            .unwrap();
        assert_eq!(
            callout_variant(&log.attrs_of(Dot::new(1, 1), NodeType::Callout)),
            CalloutVariant::Warning
        );
    }

    #[test]
    fn attrs_of_skips_kind_mismatched_op_dormant() {
        use crate::{LayoutMode, RootNodeAttr};
        let target = Dot::new(1, 1);
        let log = NodeAttrLog::new()
            .apply(
                Dot::new(2, 0),
                op(
                    1,
                    NodeAttr::Root {
                        attr: RootNodeAttr::LayoutMode(LayoutMode::Continuous { max_width: 600 }),
                    },
                ),
            )
            .unwrap();
        assert_eq!(
            callout_variant(&log.attrs_of(target, NodeType::Callout)),
            CalloutVariant::Info
        );
    }

    #[test]
    fn attrs_of_supports_atom_node() {
        use crate::ImageNodeAttr;
        let target = Dot::new(1, 5);
        let log = NodeAttrLog::new()
            .apply(
                Dot::new(2, 0),
                NodeAttrOp {
                    target,
                    attr: NodeAttr::Image {
                        attr: ImageNodeAttr::Proportion(150),
                    },
                },
            )
            .unwrap();
        match log.attrs_of(target, NodeType::Image) {
            Node::Image(img) => assert_eq!(*img.proportion.get(), 150),
            other => panic!("expected Image, got {other:?}"),
        }
    }

    #[test]
    fn project_accumulates_per_node() {
        let log = NodeAttrLog::new()
            .apply(Dot::new(2, 0), op(1, callout(CalloutVariant::Warning)))
            .unwrap()
            .apply(Dot::new(2, 1), op(2, callout(CalloutVariant::Danger)))
            .unwrap();
        let projected = log.project(|_| Some(NodeType::Callout));
        assert_eq!(projected.len(), 2);
        assert_eq!(
            callout_variant(projected.get(&Dot::new(1, 1)).unwrap()),
            CalloutVariant::Warning
        );
        assert_eq!(
            callout_variant(projected.get(&Dot::new(1, 2)).unwrap()),
            CalloutVariant::Danger
        );
    }

    #[test]
    fn project_omits_attr_less_nodes() {
        let log = NodeAttrLog::new()
            .apply(Dot::new(2, 0), op(1, callout(CalloutVariant::Warning)))
            .unwrap();
        let projected = log.project(|_| Some(NodeType::Callout));
        assert!(projected.get(&Dot::new(1, 1)).is_some());
        assert!(projected.get(&Dot::new(1, 7)).is_none());
    }

    #[test]
    fn project_skips_unknown_target_dormant() {
        let log = NodeAttrLog::new()
            .apply(Dot::new(2, 0), op(1, callout(CalloutVariant::Warning)))
            .unwrap();
        let projected = log.project(|_| None);
        assert!(projected.is_empty());
    }

    #[test]
    fn project_matches_attrs_of() {
        let target = Dot::new(1, 1);
        let log = NodeAttrLog::new()
            .apply(Dot::new(2, 0), op(1, callout(CalloutVariant::Warning)))
            .unwrap()
            .apply(Dot::new(3, 0), op(1, callout(CalloutVariant::Danger)))
            .unwrap();
        let projected = log.project(|_| Some(NodeType::Callout));
        assert_eq!(
            projected.get(&target).unwrap(),
            &log.attrs_of(target, NodeType::Callout)
        );
    }

    #[test]
    fn project_skips_kind_mismatched_op_dormant() {
        use crate::{LayoutMode, RootNodeAttr};
        let target = Dot::new(1, 1);
        let log = NodeAttrLog::new()
            .apply(
                Dot::new(2, 0),
                NodeAttrOp {
                    target,
                    attr: NodeAttr::Root {
                        attr: RootNodeAttr::LayoutMode(LayoutMode::Continuous { max_width: 600 }),
                    },
                },
            )
            .unwrap();
        let projected = log.project(|_| Some(NodeType::Callout));
        assert!(
            projected.get(&target).is_none(),
            "mismatch-only target은 dormant → 부재여야 함"
        );
    }
}

#[cfg(test)]
mod proptests {
    use std::collections::HashSet;

    use proptest::prelude::*;

    use super::*;
    use crate::{CalloutNodeAttr, CalloutVariant, TableNodeAttr};

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

    fn arb_proportion_ops() -> impl Strategy<Value = Vec<(Dot, u32)>> {
        prop::collection::vec((any::<u64>(), any::<u64>(), any::<u32>()), 0..30).prop_map(|raw| {
            let mut seen = HashSet::new();
            raw.into_iter()
                .map(|(actor, clock, value)| (Dot::new(actor, clock), value))
                .filter(|(dot, _)| seen.insert(*dot))
                .collect()
        })
    }

    fn build(pairs: &[(Dot, u32)]) -> NodeAttrLog {
        let mut log = NodeAttrLog::new();
        for (dot, value) in pairs {
            let op = NodeAttrOp {
                target: Dot::new(1, 1),
                attr: NodeAttr::Table {
                    attr: TableNodeAttr::Proportion(*value),
                },
            };
            log = log.apply(*dot, op).expect("distinct dots never conflict");
        }
        log
    }

    fn projected_proportion(log: &NodeAttrLog) -> u32 {
        match log.attrs_of(Dot::new(1, 1), NodeType::Table) {
            Node::Table(t) => *t.proportion.get(),
            other => panic!("expected Table, got {other:?}"),
        }
    }

    fn arb_node_attr() -> impl Strategy<Value = NodeAttr> {
        prop_oneof![
            prop_oneof![
                Just(CalloutVariant::Info),
                Just(CalloutVariant::Warning),
                Just(CalloutVariant::Danger),
            ]
            .prop_map(|v| NodeAttr::Callout {
                attr: CalloutNodeAttr::Variant(v),
            }),
            any::<u32>().prop_map(|p| NodeAttr::Table {
                attr: TableNodeAttr::Proportion(p),
            }),
        ]
    }

    fn arb_node_attr_op() -> impl Strategy<Value = NodeAttrOp> {
        (0u64..4, arb_node_attr()).prop_map(|(t, attr)| NodeAttrOp {
            target: Dot::new(1, t),
            attr,
        })
    }

    fn apply_all(pairs: &[(Dot, NodeAttrOp)]) -> NodeAttrLog {
        let mut log = NodeAttrLog::new();
        for (d, op) in pairs {
            log = log
                .apply(*d, op.clone())
                .expect("distinct dots never conflict");
        }
        log
    }

    proptest! {
        #[test]
        fn node_attr_log_converges_under_permutation(
            ops in prop::collection::vec(arb_node_attr_op(), 0..24),
            seed in any::<u64>(),
        ) {
            let pairs: Vec<(Dot, NodeAttrOp)> = ops
                .iter()
                .enumerate()
                .map(|(i, op)| (Dot::new(9, i as u64), op.clone()))
                .collect();
            prop_assert_eq!(apply_all(&pairs), apply_all(&permute(&pairs, seed)));
        }

        #[test]
        fn node_attr_log_idempotent_under_permutation(
            ops in prop::collection::vec(arb_node_attr_op(), 0..24),
            seed in any::<u64>(),
        ) {
            let pairs: Vec<(Dot, NodeAttrOp)> = ops
                .iter()
                .enumerate()
                .map(|(i, op)| (Dot::new(9, i as u64), op.clone()))
                .collect();
            let once = apply_all(&pairs);
            let doubled: Vec<(Dot, NodeAttrOp)> =
                pairs.iter().flat_map(|p| [p.clone(), p.clone()]).collect();
            prop_assert_eq!(once, apply_all(&permute(&doubled, seed)));
        }

        #[test]
        fn attrs_of_matches_max_dot_reference(ops in arb_proportion_ops()) {
            let log = build(&ops);
            let reference = ops
                .iter()
                .max_by_key(|(d, _)| *d)
                .map(|(_, v)| *v)
                .unwrap_or(100);
            prop_assert_eq!(projected_proportion(&log), reference);
        }
    }
}
