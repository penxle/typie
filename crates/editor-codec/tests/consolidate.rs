mod common;

mod tests {
    use editor_codec::{ReencodableChangesets, consolidate_stream, encode_changesets};
    use editor_crdt::{Changeset, Dot, ListOp, Op};
    use editor_model::{EditOp, NodeType, SeqItem};
    use proptest::prelude::*;

    use super::common;

    fn bundle(ops: Vec<Op<EditOp>>) -> Vec<u8> {
        encode_changesets(ReencodableChangesets::from_local_ops(vec![Changeset {
            ops,
        }]))
        .unwrap()
    }

    fn char_op(clock: u64, parents: Vec<Dot>, ch: char) -> Op<EditOp> {
        Op {
            id: Dot::new(1, clock),
            parents,
            payload: EditOp::Seq(ListOp::Ins {
                pos: clock as usize,
                item: SeqItem::Char(ch),
            }),
        }
    }

    fn stream(bundles: &[Vec<u8>]) -> Vec<u8> {
        bundles.concat()
    }

    fn assert_changesets_equal(a: &[Changeset<EditOp>], b: &[Changeset<EditOp>]) {
        assert_eq!(a.len(), b.len());
        for (cs_a, cs_b) in a.iter().zip(b) {
            assert_eq!(cs_a.ops.len(), cs_b.ops.len());
            for (op_a, op_b) in cs_a.ops.iter().zip(&cs_b.ops) {
                assert_eq!(op_a.id, op_b.id);
                assert_eq!(op_a.parents, op_b.parents);
                assert_eq!(
                    editor_codec::convert::to_durable_op(&op_a.payload).unwrap(),
                    editor_codec::convert::to_durable_op(&op_b.payload).unwrap()
                );
            }
        }
    }

    #[test]
    fn consolidation_preserves_changeset_list() {
        let b0 = bundle(vec![Op {
            id: Dot::new(1, 0),
            parents: vec![],
            payload: EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }),
        }]);
        let b1 = bundle(vec![char_op(1, vec![Dot::new(1, 0)], 'a')]);
        let b2 = bundle(vec![char_op(2, vec![Dot::new(1, 1)], 'b')]);
        let s = stream(&[b0.clone(), b1.clone(), b2.clone()]);

        let c = consolidate_stream(&s).unwrap().expect("3개 병합");
        assert_eq!(c.consumed, 3);
        assert_eq!(c.consumed_bytes, s.len());

        let merged = editor_codec::decode_changesets(&c.payload)
            .unwrap()
            .into_graph_input();
        let original = editor_codec::decode_changeset_stream(&s)
            .unwrap()
            .into_graph_input();
        assert_changesets_equal(&merged, &original);
    }

    #[test]
    fn consolidation_stops_at_unknown_bearing_bundle() {
        let b0 = bundle(vec![char_op(0, vec![], 'a')]);
        let b1 = bundle(vec![char_op(1, vec![Dot::new(1, 0)], 'b')]);
        let b2 = common::synth_unknown_bundle();
        let b3 = bundle(vec![char_op(2, vec![Dot::new(1, 1)], 'c')]);
        let s = stream(&[b0.clone(), b1.clone(), b2, b3]);

        let c = consolidate_stream(&s).unwrap().expect("접두 2개만 병합");
        assert_eq!(c.consumed, 2, "unknown-보유 번들에서 멈춰야 한다");
        assert_eq!(c.consumed_bytes, b0.len() + b1.len());
    }

    #[test]
    fn stops_at_record_tail_bundle() {
        let b0 = bundle(vec![char_op(0, vec![], 'a')]);
        let b1 = bundle(vec![char_op(1, vec![Dot::new(1, 0)], 'b')]);
        let b2 = common::synth_record_tail_bundle();
        let s = stream(&[b0.clone(), b1.clone(), b2]);

        let c = consolidate_stream(&s).unwrap().expect("접두 2개만 병합");
        assert_eq!(c.consumed, 2, "record_tail 번들에서 멈춰야 한다");
        assert_eq!(c.consumed_bytes, b0.len() + b1.len());
    }

    #[test]
    fn fenced_envelope_is_boundary_not_error() {
        let b0 = bundle(vec![char_op(0, vec![], 'a')]);
        let b1 = bundle(vec![char_op(1, vec![Dot::new(1, 0)], 'b')]);
        let b2 = common::synth_fenced_envelope();
        let s = stream(&[b0.clone(), b1.clone(), b2]);

        let c = consolidate_stream(&s)
            .unwrap()
            .expect("접두 2개만 병합 — 에러 아님");
        assert_eq!(c.consumed, 2, "Fenced envelope는 경계이지 에러가 아니다");
        assert_eq!(c.consumed_bytes, b0.len() + b1.len());
    }

    #[test]
    fn single_bundle_is_noop() {
        let b0 = bundle(vec![char_op(0, vec![], 'a')]);
        assert!(consolidate_stream(&b0).unwrap().is_none());
    }

    #[test]
    fn unknown_first_bundle_is_noop() {
        let b0 = common::synth_unknown_bundle();
        let b1 = bundle(vec![char_op(0, vec![], 'a')]);
        let s = stream(&[b0, b1]);
        assert!(consolidate_stream(&s).unwrap().is_none());
    }

    fn linear_char_ops(total: usize) -> Vec<Op<EditOp>> {
        (0..total)
            .map(|i| {
                let parents = if i == 0 {
                    vec![]
                } else {
                    vec![Dot::new(1, (i - 1) as u64)]
                };
                let ch = char::from(b'a' + (i % 26) as u8);
                Op {
                    id: Dot::new(1, i as u64),
                    parents,
                    payload: EditOp::Seq(ListOp::Ins {
                        pos: i,
                        item: SeqItem::Char(ch),
                    }),
                }
            })
            .collect()
    }

    proptest! {
        #[test]
        fn consolidation_is_semantically_identity(
            splits in proptest::collection::vec(1usize..5, 1..6),
        ) {
            let total: usize = splits.iter().sum();
            let ops = linear_char_ops(total);

            let mut idx = 0;
            let mut bundles = Vec::with_capacity(splits.len());
            for &n in &splits {
                bundles.push(bundle(ops[idx..idx + n].to_vec()));
                idx += n;
            }
            let s = stream(&bundles);

            let original = editor_codec::decode_changeset_stream(&s).unwrap().into_graph_input();

            match consolidate_stream(&s).unwrap() {
                Some(c) => {
                    prop_assert_eq!(c.consumed, bundles.len());
                    prop_assert_eq!(c.consumed_bytes, s.len());
                    let merged = editor_codec::decode_changesets(&c.payload).unwrap().into_graph_input();
                    prop_assert_eq!(merged.len(), original.len());
                    for (m, o) in merged.iter().zip(&original) {
                        prop_assert_eq!(m.ops.len(), o.ops.len());
                        for (a, b) in m.ops.iter().zip(&o.ops) {
                            prop_assert_eq!(a.id, b.id);
                            prop_assert_eq!(a.parents.clone(), b.parents.clone());
                            prop_assert_eq!(
                                editor_codec::convert::to_durable_op(&a.payload).unwrap(),
                                editor_codec::convert::to_durable_op(&b.payload).unwrap()
                            );
                        }
                    }
                }
                None => {
                    prop_assert_eq!(bundles.len(), 1);
                }
            }
        }
    }
}
