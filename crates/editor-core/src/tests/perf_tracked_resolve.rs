use std::time::{Duration, Instant};

use editor_crdt::Dot;
use editor_model::{
    PlainDoc, PlainNode, PlainNodeEntry, PlainParagraphNode, PlainRootNode, PlainTextNode,
};
use editor_state::test_utils::build_state_from_plain;
use editor_state::{Position, Selection, StableSelection, State};
use editor_transaction::Transaction;

use crate::tracked_range::{TrackedRange, TrackedRangeRegistry};

const PARAS: usize = 50;
const CHARS_PER_PARA: usize = 1000;
const RANGE_COUNT: usize = 200;
const SPAN_LEN: usize = 3;
const KEYSTROKES: usize = 100;

fn plain_text(text: String) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Text(PlainTextNode { text }),
        modifiers: Default::default(),
        carry: Vec::new(),
        children: vec![],
    }
}

fn plain_para(text: String) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Paragraph(PlainParagraphNode::default()),
        modifiers: Default::default(),
        carry: Vec::new(),
        children: vec![plain_text(text)],
    }
}

fn build_large_state(paras: usize, chars_per_para: usize) -> (State, Vec<Dot>) {
    let text: String = "abcdefghij ".chars().cycle().take(chars_per_para).collect();
    let plain = PlainDoc {
        root: PlainNodeEntry {
            node: PlainNode::Root(PlainRootNode::default()),
            modifiers: Default::default(),
            carry: Vec::new(),
            children: (0..paras).map(|_| plain_para(text.clone())).collect(),
        },
    };
    let (state, handles) = build_state_from_plain(plain);
    let para_dots: Vec<Dot> = (0..paras)
        .map(|i| *handles.get(&vec![i]).expect("paragraph handle must exist"))
        .collect();
    (state, para_dots)
}

fn make_tracked_ranges(
    state: &State,
    para_dots: &[Dot],
    count: usize,
    span_len: usize,
) -> TrackedRangeRegistry {
    let view = state.view();
    let mut reg = TrackedRangeRegistry::new();
    for i in 0..count {
        let para = para_dots[i % para_dots.len()];
        let slot = i / para_dots.len();
        let start = slot * (span_len + 1);
        let sel = Selection::new(
            Position::new(para, start),
            Position::new(para, start + span_len),
        );
        let stable = StableSelection::capture(&sel, &view);
        reg.add(TrackedRange::new(
            format!("r{i}"),
            "g".into(),
            stable,
            String::new(),
            state,
        ));
    }
    reg
}

fn locate_all(state: &State, reg: &TrackedRangeRegistry) -> usize {
    reg.iter().filter(|r| r.locate(state).is_some()).count()
}

fn run_keystrokes(
    mut state: State,
    reg: &TrackedRangeRegistry,
    cursor: Dot,
    start_offset: usize,
    keystrokes: usize,
) -> (Duration, usize) {
    let mut total = Duration::ZERO;
    let mut last_hits = 0;
    for offset in (start_offset..).take(keystrokes) {
        let mut tr = Transaction::new(&state);
        tr.insert_text(cursor, offset, "x").unwrap();
        let (next_state, ..) = tr.commit();
        state = next_state;

        let t = Instant::now();
        last_hits = locate_all(&state, reg);
        total += t.elapsed();
    }
    (total, last_hits)
}

#[test]
#[ignore]
fn perf_tracked_resolve_no_move() {
    let (state, para_dots) = build_large_state(PARAS, CHARS_PER_PARA);
    let reg = make_tracked_ranges(&state, &para_dots, RANGE_COUNT, SPAN_LEN);
    let cursor = para_dots[0];

    let (total, hits) = run_keystrokes(state, &reg, cursor, CHARS_PER_PARA, KEYSTROKES);
    eprintln!(
        "perf_tracked_resolve_no_move: {KEYSTROKES} keystrokes, total locate {total:?}, avg {:?}",
        total / KEYSTROKES as u32
    );
    assert_eq!(
        hits, RANGE_COUNT,
        "all ranges must resolve when nothing moved"
    );
    assert!(
        total < Duration::from_millis(140),
        "tracked range resolve regressed (no-move, hash-miss fast path): {total:?}"
    );
}

#[test]
#[ignore]
fn perf_tracked_resolve_after_move() {
    let (state0, para_dots) = build_large_state(PARAS, CHARS_PER_PARA);
    let reg = make_tracked_ranges(&state0, &para_dots, RANGE_COUNT, SPAN_LEN);

    let moved = para_dots[PARAS / 2];
    let mut tr = Transaction::new(&state0);
    tr.move_node(moved, Dot::ROOT, 0).unwrap();
    let (state, ..) = tr.commit();

    let cursor = para_dots[0];
    let (total, hits) = run_keystrokes(state, &reg, cursor, CHARS_PER_PARA, KEYSTROKES);
    eprintln!(
        "perf_tracked_resolve_after_move: {KEYSTROKES} keystrokes, total locate {total:?}, avg {:?}",
        total / KEYSTROKES as u32
    );
    assert_eq!(
        hits, RANGE_COUNT,
        "the moved paragraph's ranges must still resolve (dead anchors alias to the new dot)"
    );
    assert!(
        total < Duration::from_millis(150),
        "tracked range resolve regressed (after move, dead-anchor scan path): {total:?}"
    );
}
