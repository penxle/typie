//! Temporary perf probe for internal-selection DnD `Over` judgment across
//! block-spanning selections.
//! Run: cargo test -p editor-core --profile profiling perf_dnd_over -- --ignored --nocapture
//! Profile: cargo test -p editor-core --profile profiling perf_dnd_over --no-run
//!          samply record <target/profiling/deps/editor_core-*> perf_dnd_over_profile_target --ignored --nocapture

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use editor_crdt::Dot;
use editor_model::{
    PlainDoc, PlainNode, PlainNodeEntry, PlainParagraphNode, PlainRootNode, PlainTextNode,
};
use editor_state::test_utils::build_state_from_plain;
use editor_state::{Position, Selection, State};

use crate::editor::Editor;
use crate::message::*;

fn text_entry(text: String) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Text(PlainTextNode { text }),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: vec![],
    }
}

fn para_entry(text: String) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Paragraph(PlainParagraphNode {}),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: vec![text_entry(text)],
    }
}

fn root_entry(children: Vec<PlainNodeEntry>) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Root(PlainRootNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children,
    }
}

fn para_text(i: usize, words: usize) -> String {
    (0..words)
        .map(|w| format!("w{i:04}x{w:03}"))
        .collect::<Vec<_>>()
        .join(" ")
}

struct Fixture {
    state: State,
    paras: Vec<Dot>,
    len: usize,
}

fn build_fixture(n: usize, words: usize) -> Fixture {
    let paras: Vec<PlainNodeEntry> = (0..n).map(|i| para_entry(para_text(i, words))).collect();
    let (state, handles) = build_state_from_plain(PlainDoc {
        root: root_entry(paras),
    });
    let paras = (0..n).map(|i| handles[&vec![i]]).collect();
    let len = para_text(0, words).chars().count();
    Fixture { state, paras, len }
}

fn over_point(editor: &Editor, para: Dot) -> (usize, f32, f32) {
    let metrics = editor
        .view()
        .cursor_metrics(editor.state(), &Position::new(para, 0))
        .expect("cursor metrics");
    (
        metrics.page_idx,
        metrics.caret.x,
        metrics.caret.y + metrics.caret.height * 0.5,
    )
}

struct Sample {
    over: Vec<Duration>,
    allowed: bool,
}

fn measure(
    fixture: &Fixture,
    from: (usize, usize),
    to: (usize, usize),
    targets: &[usize],
    rounds: usize,
) -> Sample {
    let mut editor = Editor::new_test(fixture.state.clone());
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                Position::new(fixture.paras[from.0], from.1),
                Position::new(fixture.paras[to.0], to.1),
            ),
        },
    });
    editor.apply(Message::Dnd {
        op: DndOp::StartInternalSelection,
    });
    let points: Vec<_> = targets
        .iter()
        .map(|&t| over_point(&editor, fixture.paras[t]))
        .collect();
    let mut over = Vec::with_capacity(rounds);
    let mut allowed = false;
    for round in 0..rounds {
        let (page, x, y) = points[round % points.len()];
        let t = Instant::now();
        editor.apply(Message::Dnd {
            op: DndOp::Over {
                page,
                x,
                y,
                reuse_node_id: None,
                modifiers: InputModifiers::default(),
            },
        });
        over.push(t.elapsed());
        allowed = editor.drop_indicator_for_test().is_some();
    }
    editor.apply(Message::Dnd { op: DndOp::End });
    Sample { over, allowed }
}

fn median(vals: &mut [Duration]) -> Duration {
    vals.sort();
    vals[vals.len() / 2]
}

type Scenario = (&'static str, (usize, usize), (usize, usize));

fn scenarios(len: usize) -> Vec<Scenario> {
    vec![
        ("1 block partial", (2, 5), (2, len - 5)),
        ("2 blocks partial", (2, len / 2), (3, len / 2)),
        ("2 blocks whole", (2, 0), (3, len)),
        ("5 blocks whole", (2, 0), (6, len)),
        ("20 blocks whole", (2, 0), (21, len)),
    ]
}

#[test]
#[ignore]
fn perf_dnd_over_internal_selection() {
    const ROUNDS: usize = 5;
    for (n, words) in [(100usize, 8usize), (300, 8), (1000, 8)] {
        let fixture = build_fixture(n, words);
        let targets = [n / 2, n / 2 + 1];
        for (label, from, to) in scenarios(fixture.len) {
            let mut sample = measure(&fixture, from, to, &targets, ROUNDS);
            eprintln!(
                "[n={n} words={words}] {label:<18} over median {:>12?} | max {:>12?} | allowed={}",
                median(&mut sample.over),
                sample.over.iter().max().copied().unwrap_or_default(),
                sample.allowed,
            );
        }
    }
}

fn measure_delete_selection(
    fixture: &Fixture,
    from: (usize, usize),
    to: (usize, usize),
) -> Duration {
    let mut editor = Editor::new_test(fixture.state.clone());
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    editor.apply(Message::Selection {
        op: SelectionOp::Set {
            selection: Selection::new(
                Position::new(fixture.paras[from.0], from.1),
                Position::new(fixture.paras[to.0], to.1),
            ),
        },
    });
    let t = Instant::now();
    editor.apply(Message::Deletion {
        op: DeletionOp::Selection,
    });
    t.elapsed()
}

#[test]
#[ignore]
fn perf_dnd_over_plain_delete_comparison() {
    for (n, words) in [(100usize, 8usize), (300, 8), (1000, 8)] {
        let fixture = build_fixture(n, words);
        for (label, from, to) in scenarios(fixture.len) {
            let mut samples: Vec<Duration> = (0..5)
                .map(|_| measure_delete_selection(&fixture, from, to))
                .collect();
            eprintln!(
                "[n={n} words={words}] {label:<18} delete_selection median {:>12?}",
                median(&mut samples),
            );
        }
    }
}

#[test]
#[ignore]
fn perf_dnd_over_profile_target() {
    let rounds: usize = std::env::var("PERF_DND_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);
    let n: usize = std::env::var("PERF_DND_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);
    let fixture = build_fixture(n, 8);
    let targets = [n / 2, n / 2 + 1];
    let mut sample = measure(&fixture, (2, 0), (3, fixture.len), &targets, rounds);
    eprintln!(
        "[profile n={n} rounds={rounds}] 2 blocks whole over median {:?} | allowed={}",
        median(&mut sample.over),
        sample.allowed,
    );
}
