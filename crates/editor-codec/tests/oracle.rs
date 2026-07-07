use editor_codec::{ReencodableChangesets, decode_changesets, encode_changesets};
use editor_crdt::{Changeset, Dot, ListOp, Op, OpGraph};
use editor_model::{
    Anchor, Bias, EditOp, Modifier, ModifierAttrOp, ModifierType, NodeType, SeqItem, SpanOp,
    project_document, split_logs,
};
use proptest::prelude::*;

fn arb_history() -> impl Strategy<Value = Vec<Changeset<EditOp>>> {
    // Linear-chain generator: one paragraph plus char inserts/deletes/spans/block
    // modifiers/carries assembled directly as a `Changeset`, within the same
    // valid-range constraints editor-state's warm-cold generators use. ~30 ops,
    // a single changeset — parallel/concurrent structure is already covered by
    // the vnext/bundle tests; this oracle's job is the codec round trip's
    // meaning-preservation, not concurrency.
    proptest::collection::vec((0u8..8, any::<u8>(), any::<char>()), 1..30).prop_map(|actions| {
        let mut ops = Vec::new();
        let mut count = 0usize;
        let mut clock = 0u64;
        let push = |ops: &mut Vec<Op<EditOp>>, clock: &mut u64, payload: EditOp| {
            let parents = if *clock == 0 {
                vec![]
            } else {
                vec![Dot::new(1, *clock - 1)]
            };
            ops.push(Op {
                id: Dot::new(1, *clock),
                parents,
                payload,
            });
            *clock += 1;
        };
        push(
            &mut ops,
            &mut clock,
            EditOp::Seq(ListOp::Ins {
                pos: 0,
                item: SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![Dot::ROOT],
                    attrs: vec![],
                },
            }),
        );
        count += 1;
        let para = Dot::new(1, 0);
        let mut live: Vec<Dot> = Vec::new();
        for (kind, a, ch) in actions {
            match kind {
                0..=2 => {
                    let pos = 1 + (a as usize) % count;
                    let id = Dot::new(1, clock);
                    push(
                        &mut ops,
                        &mut clock,
                        EditOp::Seq(ListOp::Ins {
                            pos,
                            item: SeqItem::Char(ch),
                        }),
                    );
                    live.push(id);
                    count += 1;
                }
                3 if count > 1 => {
                    let pos = 1 + (a as usize) % (count - 1);
                    push(
                        &mut ops,
                        &mut clock,
                        EditOp::Seq(ListOp::Del { pos, len: 1 }),
                    );
                    count -= 1;
                }
                4 if !live.is_empty() => {
                    let anchor = live[(a as usize) % live.len()];
                    push(
                        &mut ops,
                        &mut clock,
                        EditOp::Span(SpanOp::AddSpan {
                            start: Anchor {
                                id: anchor,
                                bias: Bias::Before,
                            },
                            end: Anchor {
                                id: anchor,
                                bias: Bias::After,
                            },
                            modifier: if a & 1 == 0 {
                                Modifier::Bold
                            } else {
                                Modifier::FontSize {
                                    value: 1200 + a as u32,
                                }
                            },
                        }),
                    );
                }
                5 if !live.is_empty() => {
                    let anchor = live[(a as usize) % live.len()];
                    push(
                        &mut ops,
                        &mut clock,
                        EditOp::Span(SpanOp::RemoveSpan {
                            start: Anchor {
                                id: anchor,
                                bias: Bias::Before,
                            },
                            end: Anchor {
                                id: anchor,
                                bias: Bias::After,
                            },
                            modifier_type: ModifierType::Bold,
                        }),
                    );
                }
                6 => {
                    let op = if a & 1 == 0 {
                        ModifierAttrOp::SetModifier {
                            target: para,
                            modifier: Modifier::LineHeight {
                                value: 100 + a as u32,
                            },
                        }
                    } else {
                        ModifierAttrOp::ClearModifier {
                            target: para,
                            key: ModifierType::LineHeight,
                        }
                    };
                    push(&mut ops, &mut clock, EditOp::BlockModifier(op));
                }
                7 => {
                    let op = if a & 1 == 0 {
                        ModifierAttrOp::SetModifier {
                            target: para,
                            modifier: Modifier::FontSize {
                                value: 1000 + a as u32,
                            },
                        }
                    } else {
                        ModifierAttrOp::ClearModifier {
                            target: para,
                            key: ModifierType::FontSize,
                        }
                    };
                    push(&mut ops, &mut clock, EditOp::NodeCarry(op));
                }
                _ => {}
            }
        }
        vec![Changeset { ops }]
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, ..ProptestConfig::default() })]
    #[test]
    fn round_trip_preserves_projection(css in arb_history()) {
        let bytes = encode_changesets(ReencodableChangesets::from_local_ops(css.clone())).unwrap();
        let decoded = decode_changesets(&bytes).unwrap().into_graph_input();
        let g1 = OpGraph::from_changesets(css).unwrap();
        let g2 = OpGraph::from_changesets(decoded).unwrap();
        let p1 = project_document(&split_logs(&g1).unwrap()).unwrap();
        let p2 = project_document(&split_logs(&g2).unwrap()).unwrap();
        prop_assert_eq!(p1, p2, "코덱 왕복이 투영을 바꿨다");
    }
}
