//! Generated changesets exercise wire syntactic round-trip only; semantic validity
//! (e.g. anchor NodeId existing in the doc) is not guaranteed by `arb_doc_op_sequence`.

use editor_crdt::{Dot, OpGraph, OrMapOp, RgaOp, TextOp, wire};
use editor_model::{DocOp, Modifier, ModifierType, NodeId, NodeType};
use proptest::prelude::*;

fn arb_doc_op_sequence(
    max_ops: usize,
) -> impl Strategy<Value = Vec<editor_crdt::Changeset<DocOp>>> {
    proptest::collection::vec(arb_doc_op(), 0..=max_ops).prop_map(build_changesets)
}

fn build_changesets(payloads: Vec<DocOp>) -> Vec<editor_crdt::Changeset<DocOp>> {
    let mut g = OpGraph::with_actor(1);
    for p in payloads {
        if let Ok((ng, _)) = g.add(p) {
            g = ng;
        }
    }
    g.commit().changesets().to_vec()
}

fn arb_doc_op() -> impl Strategy<Value = DocOp> {
    prop_oneof![
        (any::<char>(), prop::option::of(arb_dot())).prop_map(|(ch, after)| DocOp::Text {
            node_id: NodeId::new(),
            op: TextOp::InsertChar { after, ch },
        }),
        Just(()).prop_map(|_| {
            let id = NodeId::new();
            DocOp::Presence {
                node_id: id,
                op: OrMapOp::Set {
                    key: id,
                    value: NodeType::Paragraph,
                },
            }
        }),
        Just(()).prop_map(|_| DocOp::Modifier {
            node_id: NodeId::new(),
            op: OrMapOp::Set {
                key: ModifierType::Bold,
                value: Modifier::Bold
            },
        }),
        prop::option::of(arb_dot()).prop_map(|after| DocOp::Children {
            node_id: NodeId::new(),
            op: RgaOp::Insert {
                after,
                value: NodeId::new()
            },
        }),
    ]
}

fn arb_dot() -> impl Strategy<Value = Dot> {
    (any::<u64>(), any::<u64>()).prop_map(|(a, c)| Dot::new(a, c))
}

proptest! {
    #[test]
    fn encode_decode_round_trip(css in arb_doc_op_sequence(50)) {
        let bytes = wire::encode(&css).unwrap();
        let decoded: Vec<editor_crdt::Changeset<DocOp>> = wire::decode(&bytes).unwrap();
        prop_assert_eq!(decoded, css);
    }

    #[test]
    fn encode_is_deterministic(css in arb_doc_op_sequence(50)) {
        let a = wire::encode(&css).unwrap();
        let b = wire::encode(&css).unwrap();
        prop_assert_eq!(a, b);
    }

    #[test]
    fn decode_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..1024)) {
        let _ = wire::decode::<DocOp>(&bytes);
    }
}
