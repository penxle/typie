use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use editor::model::{Node, NodeId, Text, TextNode};
use editor::runtime::{Direction, Message, PasteMode, Runtime, State};
use editor::state::{Position, Selection, compute_selection_aggregates};
use editor::transact;
use editor::transaction::Transaction;
use editor::types::{Affinity, Theme};
use rustc_hash::FxHashMap;
use std::rc::Rc;
use std::time::Duration;

const BATCH: BatchSize = BatchSize::SmallInput;

fn test_theme() -> Theme {
    let tokens: &[(&str, u32)] = &[
        ("selection", 0x33_80_ff_ff),
        ("text.black", 0x18_18_1b_ff),
        ("ui.text.default", 0x18_18_1b_ff),
        ("ui.text.muted", 0x8c_8c_8d_ff),
        ("ui.text.subtle", 0xc5_c5_c6_ff),
        ("ui.border.default", 0xe4_e4_e7_ff),
        ("ui.surface.default", 0xff_ff_ff_ff),
        ("ui.surface.subtle", 0xf4_f4_f5_ff),
        ("ui.surface.dark", 0x18_18_1b_ff),
        ("ui.accent.brand.default", 0x33_80_ff_ff),
        ("ui.blockquote.message-sent", 0xe7_f3_f8_ff),
        ("ui.blockquote.message-received", 0xf4_f4_f5_ff),
    ];
    let colors: FxHashMap<String, u32> = tokens.iter().map(|&(k, v)| (k.to_string(), v)).collect();
    Theme { colors }
}

fn runtime_with_paragraphs(count: usize) -> Runtime {
    editor::test_utils::init_test_env();

    let doc = Rc::new(editor::model::Doc::new());
    let initial_state = State::new(
        doc,
        Selection::collapsed(Position::new(NodeId::ROOT, 0, Affinity::default())),
    );

    let state = transact!(initial_state, |tr| {
        let root_id = NodeId::ROOT;

        for index in 0..count {
            let paragraph_id = NodeId::new();
            tr.doc()
                .node(root_id)
                .unwrap()
                .as_mut()
                .insert_child_with_id(index, paragraph_id, Node::Paragraph(Default::default()))
                .unwrap();

            let text_id = NodeId::new();
            let text = Text::from(format!("Paragraph {}", index));
            tr.doc()
                .node(paragraph_id)
                .unwrap()
                .as_mut()
                .insert_child_with_id(
                    0,
                    text_id,
                    Node::Text(TextNode {
                        text,
                        ..Default::default()
                    }),
                )
                .unwrap();
        }
    });

    let mut runtime = Runtime::new(800.0, 1.0, state);
    runtime.update(Message::Initialize {
        theme: test_theme(),
    });
    runtime.tick();
    runtime.flush();
    runtime
}

fn prepared_runtime(count: usize) -> Runtime {
    let mut runtime = runtime_with_paragraphs(count);
    runtime.layout();
    runtime
}

fn bench_editing(c: &mut Criterion) {
    let mut group = c.benchmark_group("editing");
    group
        .sample_size(10)
        .measurement_time(Duration::from_secs(20));

    group.bench_function("input", |b| {
        b.iter_batched_ref(
            || {
                let mut runtime = prepared_runtime(1_000);
                runtime.update(Message::Navigate {
                    direction: Direction::DocumentEnd,
                    extend: false,
                });
                runtime.tick();
                runtime
            },
            |runtime| {
                runtime.update(Message::Input {
                    text: "a".to_string(),
                });
                runtime.tick();
                let page_count = runtime.pages().len();
                runtime.render_page(page_count - 1);
                runtime.flush();
            },
            BATCH,
        );
    });

    group.bench_function("delete_all", |b| {
        b.iter_batched_ref(
            || {
                let mut runtime = prepared_runtime(1_000);
                runtime.update(Message::SelectAll);
                runtime.tick();
                runtime
            },
            |runtime| {
                runtime.update(Message::DeleteBackward);
                runtime.tick();
                runtime.render_page(0);
                runtime.flush();
            },
            BATCH,
        );
    });

    group.bench_function("paste_large_text", |b| {
        let paragraph_count = 1000;
        let mut paste_text = String::new();
        for i in 0..paragraph_count {
            if i > 0 {
                paste_text.push('\n');
            }
            paste_text.push_str(&format!("Paragraph {} with text.", i));
        }

        b.iter_batched_ref(
            || {
                editor::test_utils::init_test_env();

                let doc = Rc::new(editor::model::Doc::new());
                let initial_state = State::new(
                    doc,
                    Selection::collapsed(Position::new(NodeId::ROOT, 0, Affinity::default())),
                );

                let p_id = NodeId::new();
                let state = transact!(initial_state, |tr| {
                    tr.doc()
                        .node(NodeId::ROOT)
                        .unwrap()
                        .as_mut()
                        .insert_child_with_id(0, p_id, Node::Paragraph(Default::default()))
                        .unwrap();
                });

                let state = transact!(state, |tr| {
                    tr.set_selection(Selection::collapsed(Position::new(
                        p_id,
                        0,
                        Affinity::default(),
                    )));
                });

                let mut runtime = Runtime::new(800.0, 1.0, state);
                runtime.update(Message::Initialize {
                    theme: test_theme(),
                });
                runtime.tick();
                runtime.flush();
                runtime.layout();
                runtime
            },
            |runtime| {
                runtime.update(Message::Paste {
                    html: None,
                    text: paste_text.clone(),
                    mode: PasteMode::Auto,
                });
                runtime.tick();
                runtime.render_page(0);
                runtime.flush();
            },
            BATCH,
        );
    });

    group.bench_function("toggle_bold", |b| {
        b.iter_batched_ref(
            || {
                let mut runtime = prepared_runtime(1_000);
                runtime.update(Message::SelectAll);
                runtime.tick();
                runtime
            },
            |runtime| {
                runtime.update(Message::ToggleBold);
                runtime.tick();
                runtime.render_page(0);
                runtime.flush();
            },
            BATCH,
        );
    });

    group.bench_function("undo_after_delete", |b| {
        b.iter_batched_ref(
            || {
                let mut runtime = prepared_runtime(1_000);
                runtime.update(Message::SelectAll);
                runtime.tick();
                runtime.update(Message::DeleteBackward);
                runtime.tick();
                runtime.flush();
                runtime
            },
            |runtime| {
                runtime.update(Message::Undo);
                runtime.tick();
                runtime.render_page(0);
                runtime.flush();
            },
            BATCH,
        );
    });

    group.finish();
}

fn bench_commit(c: &mut Criterion) {
    let mut group = c.benchmark_group("commit");
    group.sample_size(50);

    group.bench_function("with_structure_change", |b| {
        b.iter_batched_ref(
            || runtime_with_paragraphs(1_000),
            |runtime| {
                let state = runtime.state();
                let mut tr = Transaction::new(state);
                tr.push_effect(editor::runtime::Effect::StructureChanged);
                tr.commit().unwrap();
            },
            BATCH,
        );
    });

    group.finish();
}

fn bench_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("render");
    group
        .sample_size(10)
        .measurement_time(Duration::from_secs(10));

    group.bench_function("with_selection", |b| {
        b.iter_batched_ref(
            || {
                let mut runtime = prepared_runtime(1_000);
                runtime.update(Message::SelectAll);
                runtime.tick();
                runtime
            },
            |runtime| {
                runtime.render_page(0);
            },
            BATCH,
        );
    });

    group.bench_function("resize_viewport", |b| {
        b.iter_batched_ref(
            || prepared_runtime(1_000),
            |runtime| {
                runtime.update(Message::Resize {
                    width: 600.0,
                    height: 800.0,
                    scale_factor: 1.0,
                });
                runtime.tick();
                runtime.render_page(0);
            },
            BATCH,
        );
    });

    group.finish();
}

fn bench_data_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_access");
    group
        .sample_size(10)
        .measurement_time(Duration::from_secs(10));

    group.bench_function("doc_to_plain_text", |b| {
        b.iter_batched_ref(
            || prepared_runtime(1_000),
            |runtime| {
                let state = runtime.state();
                state.doc.to_plain_text()
            },
            BATCH,
        );
    });

    group.bench_function("selection_aggregates", |b| {
        b.iter_batched_ref(
            || {
                let mut runtime = prepared_runtime(1_000);
                runtime.update(Message::SelectAll);
                runtime.tick();
                runtime
            },
            |runtime| {
                let state = runtime.state();
                let (from, to) = state.selection.as_sorted(&state.doc).unwrap();
                let block_ids: Vec<NodeId> = state.doc.iter_blocks().map(|(id, _)| id).collect();
                compute_selection_aggregates(&state.doc, &block_ids, from, to)
            },
            BATCH,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_editing,
    bench_commit,
    bench_render,
    bench_data_access,
);
criterion_main!(benches);
