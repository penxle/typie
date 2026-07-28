use std::collections::BTreeMap;
use std::hint::black_box;
use std::time::{Duration, Instant};

use editor_common::{DecorationStyle, Underline, UnderlineStyle};
use editor_crdt::Dot;
use editor_model::{
    PlainDoc, PlainNode, PlainNodeEntry, PlainParagraphNode, PlainRootNode, PlainTextNode,
};
use editor_state::test_utils::build_state_from_plain;
use editor_state::{Position, Selection};

use crate::editor::Editor;
use crate::message::*;

const CHARS_PER_PARA: usize = 120;
const SPAN_LEN: usize = 4;
const KEYSTROKES: usize = 20;

fn plain_text(text: String) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Text(PlainTextNode { text }),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: vec![],
    }
}

fn plain_para(text: String) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Paragraph(PlainParagraphNode::default()),
        modifiers: BTreeMap::new(),
        carry: Vec::new(),
        children: vec![plain_text(text)],
    }
}

fn build_editor(paras: usize) -> (Editor, Vec<Dot>) {
    let text: String = "abcdefghij ".chars().cycle().take(CHARS_PER_PARA).collect();
    let plain = PlainDoc {
        root: PlainNodeEntry {
            node: PlainNode::Root(PlainRootNode::default()),
            modifiers: BTreeMap::new(),
            carry: Vec::new(),
            children: (0..paras).map(|_| plain_para(text.clone())).collect(),
        },
    };
    let (mut state, handles) = build_state_from_plain(plain);
    let para_dots: Vec<Dot> = (0..paras)
        .map(|i| *handles.get(&vec![i]).expect("paragraph handle must exist"))
        .collect();
    state.selection = Some(Selection::collapsed(Position::new(
        para_dots[0],
        CHARS_PER_PARA,
    )));
    let mut editor = Editor::new_test(state);
    editor.apply(Message::System {
        event: SystemEvent::Initialize,
    });
    (editor, para_dots)
}

fn add_spellcheck_ranges(editor: &mut Editor, para_dots: &[Dot], count: usize) {
    for i in 0..count {
        let para = para_dots[i % para_dots.len()];
        let slot = i / para_dots.len();
        let start = slot * (SPAN_LEN + 5);
        assert!(
            start + SPAN_LEN <= CHARS_PER_PARA,
            "range fixture exceeds paragraph length"
        );
        let sel = Selection::new(
            Position::new(para, start),
            Position::new(para, start + SPAN_LEN),
        );
        editor.apply(Message::TrackedRange {
            op: TrackedRangeOp::Add {
                id: format!("spell-{i}"),
                group: "spellcheck".into(),
                selection: sel,
                metadata: String::new(),
                invalidate_on_text_change: true,
            },
        });
    }
    editor.apply(Message::TrackedRange {
        op: TrackedRangeOp::SetGroupDecoration {
            group: "spellcheck".into(),
            style: DecorationStyle {
                background: None,
                underline: Some(Underline {
                    color: "spellcheck".into(),
                    style: UnderlineStyle::Wavy,
                    thickness: 1.0,
                }),
                ..Default::default()
            },
            enabled: true,
            z_index: 0,
        },
    });
}

/// Mirrors `editor-ffi`'s `public_tracked_range`, which the website host runs
/// for every range on every `StateField::TrackedRanges` change (= every
/// keystroke while any range exists).
fn host_query(editor: &Editor) -> usize {
    let state = editor.state();
    let view = state.view();
    let mut resolved_count = 0;
    for range in editor.tracked_ranges().iter() {
        let Some(sel) = range.locate(state) else {
            continue;
        };
        let Some(resolved) = sel.resolve(&view) else {
            continue;
        };
        let rects = editor.view().selection_rects(&resolved);
        let text = resolved.collect_text();
        black_box((rects, text));
        resolved_count += 1;
    }
    resolved_count
}

struct KeystrokeCost {
    tick: Duration,
    query: Duration,
    marks: Duration,
}

fn run_keystrokes(editor: &mut Editor, expected_ranges: usize) -> KeystrokeCost {
    let mut cost = KeystrokeCost {
        tick: Duration::ZERO,
        query: Duration::ZERO,
        marks: Duration::ZERO,
    };
    for _ in 0..KEYSTROKES {
        let _ = editor.enqueue_request(vec![Message::Insertion {
            op: InsertionOp::Text { text: "x".into() },
        }]);
        let t = Instant::now();
        editor.tick().expect("tick");
        cost.tick += t.elapsed();

        let t = Instant::now();
        let resolved = host_query(editor);
        cost.query += t.elapsed();
        assert_eq!(resolved, expected_ranges, "all fixture ranges must resolve");

        let t = Instant::now();
        let marks = editor.tracked_decoration_marks_for_test();
        cost.marks += t.elapsed();
        if expected_ranges > 0 {
            assert!(!marks.is_empty(), "decoration group must produce marks");
        }
        black_box(marks);
    }
    cost
}

fn report(label: &str, cost: &KeystrokeCost) {
    let per = |d: Duration| d / KEYSTROKES as u32;
    eprintln!(
        "{label}: per-keystroke tick {:?}, host tracked_ranges query {:?}, decoration marks {:?}",
        per(cost.tick),
        per(cost.query),
        per(cost.marks),
    );
}

#[test]
#[ignore]
fn perf_spellcheck_scales_with_range_count() {
    let paras = 100;
    for ranges in [0usize, 75, 300, 600] {
        let (mut editor, para_dots) = build_editor(paras);
        add_spellcheck_ranges(&mut editor, &para_dots, ranges);
        let cost = run_keystrokes(&mut editor, ranges);
        report(
            &format!(
                "{paras} paras ({} chars) x {ranges} ranges",
                paras * CHARS_PER_PARA
            ),
            &cost,
        );
    }
}

#[test]
#[ignore]
fn perf_spellcheck_scales_with_doc_size() {
    let ranges = 300;
    for paras in [25usize, 50, 100, 200] {
        let (mut editor, para_dots) = build_editor(paras);
        add_spellcheck_ranges(&mut editor, &para_dots, ranges);
        let cost = run_keystrokes(&mut editor, ranges);
        report(
            &format!(
                "{paras} paras ({} chars) x {ranges} ranges",
                paras * CHARS_PER_PARA
            ),
            &cost,
        );
    }
}

#[test]
#[ignore]
fn perf_spellcheck_single_range_rects_cost() {
    let paras = 100;
    let (mut editor, para_dots) = build_editor(paras);
    add_spellcheck_ranges(&mut editor, &para_dots, 1);

    let state = editor.state();
    let view = state.view();
    let range = editor.tracked_ranges().iter().next().expect("one range");
    let sel = range.locate(state).expect("locate");
    let resolved = sel.resolve(&view).expect("resolve");

    let iters = 1000u32;
    let t = Instant::now();
    for _ in 0..iters {
        black_box(editor.view().selection_rects(&resolved));
    }
    let rects_total = t.elapsed();

    let t = Instant::now();
    for _ in 0..iters {
        black_box(resolved.collect_text());
    }
    let text_total = t.elapsed();

    let t = Instant::now();
    for _ in 0..iters {
        black_box(range.locate(state));
    }
    let locate_total = t.elapsed();

    eprintln!(
        "one {SPAN_LEN}-char range in a {}-char doc: locate {:?}, selection_rects {:?}, collect_text {:?} per call",
        paras * CHARS_PER_PARA,
        locate_total / iters,
        rects_total / iters,
        text_total / iters,
    );
}
