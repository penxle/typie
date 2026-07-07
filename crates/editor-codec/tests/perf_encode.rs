//! Perf gate (기록용, 차단 아님) — 대형 히스토리에 대해 신 코덱(wire v2)의 encode 산출
//! 바이트 크기·decode+그래프 구축 시간을 계측한다.
//! Run: cargo test -p editor-codec --release perf_encode -- --ignored --nocapture

use std::time::Instant;

use editor_codec::{ReencodableChangesets, decode_changesets, encode_changesets};
use editor_crdt::{Changeset, Dot, ListOp, Op, OpGraph};
use editor_model::{EditOp, NodeType, SeqItem, split_logs};

fn timed<T>(label: &str, f: impl FnOnce() -> T) -> (T, std::time::Duration) {
    let t = Instant::now();
    let out = f();
    let elapsed = t.elapsed();
    eprintln!("{label}: {elapsed:.2?}");
    (out, elapsed)
}

/// One paragraph followed by `n` sequential char inserts, split into
/// changesets of `per_changeset` ops each (matching real push/pull batching).
fn synth_history(n: usize, per_changeset: usize) -> Vec<Changeset<EditOp>> {
    let mut ops = Vec::with_capacity(n + 1);
    ops.push(Op {
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
    });
    let text: String = "가나다라마 hello world 바사아자차 "
        .chars()
        .cycle()
        .take(n)
        .collect();
    for (i, ch) in text.chars().enumerate() {
        let clock = 1 + i as u64;
        ops.push(Op {
            id: Dot::new(1, clock),
            parents: vec![Dot::new(1, clock - 1)],
            payload: EditOp::Seq(ListOp::Ins {
                pos: 1 + i,
                item: SeqItem::Char(ch),
            }),
        });
    }
    ops.chunks(per_changeset)
        .map(|c| Changeset { ops: c.to_vec() })
        .collect()
}

#[test]
#[ignore]
fn perf_encode_decode_large_history() {
    let css = synth_history(20_000, 64);
    let op_count: usize = css.iter().map(|c| c.ops.len()).sum();
    eprintln!("history: {op_count} ops across {} changesets", css.len());

    let (new_bytes, _new_encode_time) = timed("new codec encode", || {
        encode_changesets(ReencodableChangesets::from_local_ops(css.clone())).unwrap()
    });
    eprintln!(
        "new codec bytes: {} ({:.3} B/op)",
        new_bytes.len(),
        new_bytes.len() as f64 / op_count as f64
    );

    timed("new codec decode+graph+split", || {
        let decoded = decode_changesets(&new_bytes).unwrap().into_graph_input();
        let g = OpGraph::from_changesets(decoded).unwrap();
        split_logs(&g).unwrap()
    });
}
