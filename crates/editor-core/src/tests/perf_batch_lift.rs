use std::time::{Duration, Instant};

use editor_model::{
    Modifier, ModifierType, NodeType, PlainBulletListNode, PlainDoc, PlainListItemNode, PlainNode,
    PlainNodeEntry, PlainParagraphNode, PlainRootNode, PlainTextNode,
};
use editor_state::test_utils::build_state_from_plain;
use editor_state::{Position, Selection, State};

use crate::editor::Editor;
use crate::message::*;

fn plain_text_entry(text: String, bold: bool) -> PlainNodeEntry {
    let mut e = PlainNodeEntry {
        node: PlainNode::Text(PlainTextNode { text }),
        modifiers: std::collections::BTreeMap::new(),
        carry: Vec::new(),
        children: vec![],
    };
    if bold {
        e.modifiers.insert(ModifierType::Bold, Modifier::Bold);
    }
    e
}

fn item_para(i: usize, styled: bool) -> (PlainNodeEntry, usize) {
    let text = format!("item{i}");
    let len = text.chars().count();
    (
        PlainNodeEntry {
            node: PlainNode::Paragraph(PlainParagraphNode {}),
            modifiers: std::collections::BTreeMap::new(),
            carry: Vec::new(),
            children: vec![plain_text_entry(text, styled)],
        },
        len,
    )
}

fn list_item_entry(child: PlainNodeEntry) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::ListItem(PlainListItemNode::default()),
        modifiers: std::collections::BTreeMap::new(),
        carry: Vec::new(),
        children: vec![child],
    }
}

fn bullet_list_entry(items: Vec<PlainNodeEntry>) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::BulletList(PlainBulletListNode::default()),
        modifiers: std::collections::BTreeMap::new(),
        carry: Vec::new(),
        children: items,
    }
}

fn root_entry(children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Root(PlainRootNode::default()),
        modifiers: std::collections::BTreeMap::new(),
        carry: Vec::new(),
        children,
    }
}

/// A flat `n`-item bullet list at the document root, with a range selection
/// spanning every item's own paragraph — the shape `Message::List { op:
/// ListOp::Outdent }` resolves to a single multi-item `lift_list_items` batch
/// (or, for `n == 1`, the collapsed-selection single-item path).
fn build_flat_lift_fixture(n: usize, styled: bool) -> State {
    assert!(n > 0);
    let mut last_len = 0usize;
    let items: Vec<PlainNodeEntry> = (0..n)
        .map(|i| {
            let (para, len) = item_para(i, styled);
            last_len = len;
            list_item_entry(para)
        })
        .collect();
    let (mut state, handles) = build_state_from_plain(PlainDoc {
        root: root_entry(vec![bullet_list_entry(items)]),
    });
    let first = handles[&vec![0usize, 0usize, 0usize]];
    let last = handles[&vec![0usize, n - 1, 0usize]];
    state.selection = Some(Selection::new(
        Position::new(first, 0),
        Position::new(last, last_len),
    ));
    state
}

fn editor_from_state(state: &State) -> Editor {
    Editor::new_test(state.clone())
}

fn outdent(editor: &mut Editor) {
    editor.apply(Message::List {
        op: ListOp::Outdent,
    });
}

fn assert_lifted(editor: &Editor, n: usize) {
    let view = editor.state().view();
    let root = view.root().expect("root exists");
    let paragraphs = root
        .child_blocks()
        .filter(|b| b.node_type() == NodeType::Paragraph)
        .count();
    assert_eq!(
        paragraphs, n,
        "lift must promote every item's own paragraph to root (n={n})"
    );
}

/// Wall time of applying `Outdent` once to each of `calls` fresh editors built
/// from `state` — lift is a one-shot transformation (the fixture cannot be
/// re-lifted on the same editor), so the "steady-state" measurement pools
/// fresh editors instead of repeating the op on one editor.
fn fresh_wall(state: &State, calls: usize) -> Duration {
    let mut pool: Vec<Editor> = (0..calls).map(|_| editor_from_state(state)).collect();
    let t = Instant::now();
    for e in &mut pool {
        outdent(std::hint::black_box(e));
    }
    t.elapsed() / calls as u32
}

fn median_duration(vals: &mut [Duration]) -> Duration {
    vals.sort();
    vals[vals.len() / 2]
}

fn median_f64(vals: &mut [f64]) -> f64 {
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    vals[vals.len() / 2]
}

#[test]
#[ignore]
fn perf_batch_lift() {
    const N_LARGE: usize = 60;

    // Warmup (compiler/allocator/cache) — discarded.
    for _ in 0..3 {
        let mut e = editor_from_state(&build_flat_lift_fixture(N_LARGE, false));
        outdent(&mut e);
        std::hint::black_box(&e);
    }

    // `projection_passes` — a single deterministic measurement (not sampled):
    // the counter only tracks successful reproject/reproject_window/
    // reproject_from_tree/reproject_after_delete calls, so it is identical
    // across repeated runs of the same fixture.
    {
        let mut e = editor_from_state(&build_flat_lift_fixture(N_LARGE, false));
        let before = e.state().projected.projection_passes();
        outdent(&mut e);
        let after = e.state().projected.projection_passes();
        assert_lifted(&e, N_LARGE);
        eprintln!(
            "[flat n={N_LARGE}] projection_passes {} (bound: n*1 + cleanup(<=2) + const(<=3) = {})",
            after - before,
            N_LARGE + 2 + 3
        );
    }

    // Paired 20-round median-of-ratios: n=60 vs n=1, interleaved per round to
    // cancel timing drift — gives both the absolute n=60 wall (compared
    // against the pre-batch baseline for the >=4x gate) and an internal
    // scaling sanity ratio.
    let fixture_large = build_flat_lift_fixture(N_LARGE, false);
    let fixture_1 = build_flat_lift_fixture(1, false);
    let mut ratios: Vec<f64> = Vec::new();
    let mut wall_large: Vec<Duration> = Vec::new();
    let mut wall_1: Vec<Duration> = Vec::new();
    for _ in 0..20 {
        let w1 = fresh_wall(&fixture_1, 10);
        let wl = fresh_wall(&fixture_large, 10);
        if w1.as_secs_f64() > 0.0 {
            ratios.push(wl.as_secs_f64() / w1.as_secs_f64());
        }
        wall_1.push(w1);
        wall_large.push(wl);
    }
    eprintln!(
        "[flat n={N_LARGE} vs n=1] wall n=60 median {:?} | wall n=1 median {:?} | ratio(median-of-ratios) {:.2}",
        median_duration(&mut wall_large),
        median_duration(&mut wall_1),
        if ratios.is_empty() {
            0.0
        } else {
            median_f64(&mut ratios)
        },
    );

    // No-regression cells: small item counts (2, 3) and a styled 3-item case
    // (exercises the "+1 flush per styled run" cost from a few Span ops).
    for (label, n, styled) in [
        ("n=2", 2usize, false),
        ("n=3", 3, false),
        ("styled n=3", 3, true),
    ] {
        let fixture = build_flat_lift_fixture(n, styled);
        {
            let mut e = editor_from_state(&fixture);
            outdent(&mut e);
            assert_lifted(&e, n);
        }
        let mut samples: Vec<Duration> = (0..5).map(|_| fresh_wall(&fixture, 10)).collect();
        eprintln!("[{label}] wall median {:?}", median_duration(&mut samples));
    }
}
