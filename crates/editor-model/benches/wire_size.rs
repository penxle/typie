//! Pre/post wire-format-redesign bench. 6 scenarios; outputs CSV-like rows.
//!
//! Run: `cargo run --release --example wire_size_bench -p editor-model`

use std::time::Instant;

use editor_crdt::{Changeset, OpGraph, OrMapOp, PlacementId, RgaOp, TextOp};
use editor_model::{DocOp, NodeId, NodeType};

fn main() {
    println!("scenario,raw_bytes,zstd_bytes,encode_us,decode_us");
    typing_ascii_1k();
    typing_hangul_1k();
    paste_10k();
    accumulated_100k();
    mixed_edit();
    single_keystroke_push();
}

fn build_typing(actor: u64, paragraph: NodeId, text: &str) -> OpGraph<DocOp> {
    let mut g = OpGraph::with_actor(actor);
    let mut prev: Option<PlacementId> = None;
    for ch in text.chars() {
        let payload = DocOp::Text {
            node_id: paragraph,
            op: TextOp::InsertChar { after: prev, ch },
        };
        let (ng, op) = g.add(payload).unwrap();
        prev = Some(PlacementId(op.id));
        g = ng;
    }
    g.commit()
}

fn measure(label: &str, css: Vec<Changeset<DocOp>>) {
    let t = Instant::now();
    let bytes = editor_crdt::wire::encode(&css).unwrap();
    let encode_us = t.elapsed().as_micros();
    let zstd_bytes = bytes.len();
    let t = Instant::now();
    let _: Vec<Changeset<DocOp>> = editor_crdt::wire::decode(&bytes).unwrap();
    let decode_us = t.elapsed().as_micros();
    println!(
        "{label},{},{},{},{}",
        bytes.len(),
        zstd_bytes,
        encode_us,
        decode_us
    );
}

fn typing_ascii_1k() {
    let para = NodeId::new();
    let g = build_typing(1, para, &"a".repeat(1000));
    measure("typing-ascii-1k", g.changesets_as_vec());
}

fn typing_hangul_1k() {
    let para = NodeId::new();
    let g = build_typing(1, para, &"가".repeat(1000));
    measure("typing-hangul-1k", g.changesets_as_vec());
}

fn paste_10k() {
    let para = NodeId::new();
    let g = build_typing(1, para, &"a".repeat(10_000));
    measure("paste-10k", g.changesets_as_vec());
}

fn accumulated_100k() {
    let para = NodeId::new();
    let g = build_typing(1, para, &"x".repeat(100_000));
    measure("accumulated-100k", g.changesets_as_vec());
}

fn mixed_edit() {
    let mut g = OpGraph::with_actor(1);
    let para = NodeId::new();
    g = g
        .add(DocOp::Presence {
            node_id: para,
            op: OrMapOp::Set {
                key: para,
                value: NodeType::Paragraph,
            },
        })
        .unwrap()
        .0;
    g = g
        .add(DocOp::Children {
            node_id: NodeId::ROOT,
            op: RgaOp::Insert {
                after: None,
                value: para,
            },
        })
        .unwrap()
        .0;
    let mut prev: Option<PlacementId> = None;
    for ch in "Hello, world!".chars() {
        let (ng, op) = g
            .add(DocOp::Text {
                node_id: para,
                op: TextOp::InsertChar { after: prev, ch },
            })
            .unwrap();
        prev = Some(PlacementId(op.id));
        g = ng;
    }
    measure("mixed-edit", g.commit().changesets_as_vec());
}

fn single_keystroke_push() {
    let para = NodeId::new();
    let g = build_typing(1, para, "a");
    measure("single-keystroke-push", g.changesets_as_vec());
}
