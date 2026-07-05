//! Temporary perf instrumentation for select-all + inline modifier latency.
//! Run: cargo test -p editor-core --release perf_modifier -- --ignored --nocapture

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use editor_model::{
    Modifier, ModifierType, PlainDoc, PlainNode, PlainNodeEntry, PlainParagraphNode, PlainRootNode,
    PlainTextNode,
};
use editor_resource::{FontFamily, FontFamilySource, FontWeight, Resource};
use editor_state::test_utils::build_state_from_plain;
use editor_state::{State, flat_size};

use crate::editor::Editor;
use crate::message::*;

fn plain_text(text: String) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Text(PlainTextNode { text }),
        modifiers: BTreeMap::new(),
        marker: None,
        children: vec![],
    }
}

fn plain_para(text: String) -> PlainNodeEntry {
    PlainNodeEntry {
        node: PlainNode::Paragraph(PlainParagraphNode::default()),
        modifiers: BTreeMap::new(),
        marker: None,
        children: vec![plain_text(text)],
    }
}

fn build_large_state(paras: usize, chars_per_para: usize) -> State {
    let text: String = "가나다라마 hello world 바사아자차 "
        .chars()
        .cycle()
        .take(chars_per_para)
        .collect();
    let mut root_modifiers = BTreeMap::new();
    root_modifiers.insert(
        ModifierType::FontFamily,
        Modifier::FontFamily {
            value: "Pretendard".to_string(),
        },
    );
    root_modifiers.insert(
        ModifierType::FontWeight,
        Modifier::FontWeight { value: 400 },
    );
    let plain = PlainDoc {
        root: PlainNodeEntry {
            node: PlainNode::Root(PlainRootNode::default()),
            modifiers: root_modifiers,
            marker: None,
            children: (0..paras).map(|_| plain_para(text.clone())).collect(),
        },
    };
    let (state, _handles) = build_state_from_plain(plain);
    state
}

fn make_resource() -> Resource {
    let mut resource = Resource::new_test();
    resource.set_fonts(
        [
            ("Pretendard", vec![400u16, 700]),
            ("Paperlogy", vec![400u16, 700]),
        ]
        .into_iter()
        .map(|(name, weights)| FontFamily {
            name: name.to_string(),
            source: FontFamilySource::Default,
            weights: weights
                .into_iter()
                .map(|value| FontWeight {
                    value,
                    hash: format!("{name}-{value}"),
                    chunks: vec![vec![0x0000, 0xFFFF]],
                })
                .collect(),
        })
        .collect(),
    );
    resource
}

fn timed<T>(label: &str, f: impl FnOnce() -> T) -> T {
    let (out, _elapsed) = timed_dur(label, f);
    out
}

fn timed_dur<T>(label: &str, f: impl FnOnce() -> T) -> (T, std::time::Duration) {
    let t = Instant::now();
    let out = f();
    let elapsed = t.elapsed();
    eprintln!("{label}: {elapsed:.2?}");
    (out, elapsed)
}

#[test]
#[ignore]
fn perf_modifier_editor_level() {
    let state = timed("build 50k state", || build_large_state(50, 1000));
    let resource = Arc::new(Mutex::new(make_resource()));
    let mut editor = timed("Editor::new_test (initial layout)", || {
        Editor::new_test_with_resource(state, resource)
    });

    let n = flat_size(&editor.state().view());
    eprintln!("flat size: {n}");

    timed("select all (SetFlat)", || {
        editor.apply(Message::Selection {
            op: SelectionOp::SetFlat { start: 0, end: n },
        })
    });

    timed("toggle italic ON (apply = messages + tick)", || {
        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Italic,
            },
        })
    });

    timed("toggle italic OFF", || {
        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Italic,
            },
        })
    });

    timed("toggle bold ON", || {
        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Bold,
            },
        })
    });

    timed("toggle bold OFF", || {
        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Bold,
            },
        })
    });

    timed("set font family (Paperlogy)", || {
        editor.apply(Message::Modifier {
            op: ModifierOp::Set {
                modifier: Modifier::FontFamily {
                    value: "Paperlogy".to_string(),
                },
            },
        })
    });
}

#[test]
#[ignore]
fn perf_modifier_state_level() {
    let state = build_large_state(50, 1000);
    let resource = Arc::new(Mutex::new(make_resource()));
    let mut editor = Editor::new_test_with_resource(state, resource);
    let n = flat_size(&editor.state().view());

    let seg_index = &editor.state.projected.projected().seg_index;
    eprintln!(
        "seg index (50k-char doc): {} segs across {} blocks",
        seg_index.total_segs(),
        seg_index.block_count()
    );

    editor.apply(Message::Selection {
        op: SelectionOp::SetFlat { start: 0, end: n },
    });

    // --- decompose the generic Toggle path (handle_modifier_op) ---
    timed(
        "state_observably_changed (transact_observable gate)",
        || {
            let mut probe = editor_transaction::Transaction::new(&editor.state);
            editor_commands::toggle_modifier(&mut probe, ModifierType::Italic).unwrap();
            editor_state::state_observably_changed(&editor.state, probe.state())
        },
    );

    // sub-decompose the command itself on a fresh transaction
    let span_op_elapsed = {
        let tr = editor_transaction::Transaction::new(&editor.state);
        let view = timed("  command: tr.view()", || tr.view());
        let rs = timed("  command: selection.resolve", || {
            editor.state.selection.unwrap().resolve(&view).unwrap()
        });
        timed("  command: resolve_modifier_state_in_range", || {
            editor_state::resolve_modifier_state_in_range(&rs)
        });
        drop(view);

        let mut tr = tr;
        // first/last leaf dots across the doc
        let (first, last) = {
            let view = tr.view();
            let root = view.root().unwrap();
            let mut leaves = root.descendants().filter_map(|c| match c {
                editor_model::ChildView::Leaf(l) => Some(l.dot()),
                editor_model::ChildView::Block(_) => None,
            });
            let first = leaves.next().unwrap();
            let last = leaves.last().unwrap();
            (first, last)
        };
        let (_, span_op_elapsed) =
            timed_dur("  command: add_span_modifier (italic, whole doc)", || {
                tr.add_span_modifier(first, last, Modifier::Italic).unwrap()
            });
        timed("  command: tr.commit", || tr.commit());
        span_op_elapsed
    };

    // --- real editor transact + tick, timed separately ---
    timed("transact_observable (toggle_modifier italic ON)", || {
        editor
            .transact_observable(|tr| {
                editor_commands::toggle_modifier(tr, ModifierType::Italic)?;
                Ok(())
            })
            .unwrap()
    });
    timed("tick after transact (reconcile/layout)", || {
        editor.tick().unwrap()
    });

    // 2ms = 2x headroom over the <1ms goal, so this stays non-flaky while
    // still catching an O(N) regression. Asserted last so every timing above
    // prints even when this fails (the test is #[ignore]d; run explicitly).
    assert!(
        span_op_elapsed < std::time::Duration::from_millis(2),
        "span op must be O(segments): {span_op_elapsed:?}"
    );
}
