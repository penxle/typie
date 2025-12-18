use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;
use editor::model::{Node, NodeId, Text, TextNode};
use editor::runtime::{Direction, Message, Runtime, State};
use editor::state::{Position, Selection};
use editor::transaction::Transaction;
use editor::types::Affinity;
use editor::transact;
use std::rc::Rc;

fn runtime_with_paragraphs(count: usize) -> Runtime {
    editor::test_utils::init_test_icu();

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

    Runtime::new(800.0, 1.0, state)
}

fn bench_input(c: &mut Criterion) {
    let mut group = c.benchmark_group("input");
    group.sample_size(10);

    group.bench_function("input_long_document", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = runtime_with_paragraphs(1_000);
                runtime.layout();
                runtime.update(Message::Navigate {
                    direction: Direction::DocumentEnd,
                    extend: false,
                });
                runtime.tick();
                runtime
            },
            |mut runtime| {
                runtime.update(Message::Input {
                    text: "a".to_string(),
                });
                runtime.tick();
                let page_count = runtime.pages().len();
                runtime.render_page(page_count - 1);
                runtime.flush();
            },
        );
    });

    group.finish();
}

fn bench_select_all(c: &mut Criterion) {
    let mut group = c.benchmark_group("select_all");
    group.sample_size(10);

    group.bench_function("select_all_complete", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = runtime_with_paragraphs(1_000);
                runtime.layout();
                runtime
            },
            |mut runtime| {
                runtime.update(Message::SelectAll);
                runtime.tick();
                runtime.render_page(0);
                runtime.flush();
            },
        );
    });

    group.finish();
}

fn bench_delete_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete_selection");
    group.sample_size(10).measurement_time(Duration::from_secs(10));

    group.bench_function("delete_all", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = runtime_with_paragraphs(1_000);
                runtime.layout();
                runtime.update(Message::SelectAll);
                runtime.tick();
                runtime
            },
            |mut runtime| {
                runtime.update(Message::DeleteBackward);
                runtime.tick();
                runtime.render_page(0);
                while !runtime.state().garbage_ids.is_empty() {
                    runtime.flush();
                }
            },
        );
    });

    group.finish();
}

fn bench_paste(c: &mut Criterion) {
    let mut group = c.benchmark_group("paste");
    group.sample_size(10).measurement_time(Duration::from_secs(10));

    group.bench_function("paste_large_text", |b| {
        let paragraph_count = 1000;
        let mut paste_text = String::new();
        for i in 0..paragraph_count {
            if i > 0 {
                paste_text.push('\n');
            }
            paste_text.push_str(&format!("Paragraph {} with text.", i));
        }

        b.iter_with_setup(
            || {
                editor::test_utils::init_test_icu();

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
                runtime.layout();
                runtime
            },
            |mut runtime| {
                runtime.update(Message::Paste {
                    fragment: None,
                    html: None,
                    text: paste_text.clone(),
                });
                runtime.tick();
                runtime.render_page(0);
                runtime.flush();
            },
        );
    });

    group.finish();
}

fn bench_commit(c: &mut Criterion) {
    let mut group = c.benchmark_group("commit");
    group.sample_size(50);

    group.bench_function("commit_with_structure_change", |b| {
        b.iter_with_setup(
            || runtime_with_paragraphs(1_000),
            |runtime| {
                let state = runtime.state();
                let mut tr = Transaction::new(state);
                tr.push_effect(editor::runtime::Effect::StructureChanged);
                tr.commit().unwrap()
            },
        );
    });

    group.bench_function("commit_without_structure_change", |b| {
        b.iter_with_setup(
            || runtime_with_paragraphs(1_000),
            |runtime| {
                let state = runtime.state();
                let mut tr = Transaction::new(state);
                tr.push_effect(editor::runtime::Effect::NodeChanged {
                    node_id: NodeId::ROOT,
                });
                tr.commit().unwrap()
            },
        );
    });

    group.finish();
}

fn bench_get_text(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_text");
    group.sample_size(100);

    group.bench_function("doc_to_plain_text", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = runtime_with_paragraphs(1_000);
                runtime.layout();
                runtime
            },
            |runtime| {
                let state = runtime.state();
                state.doc.to_plain_text()
            },
        );
    });

    group.bench_function("selection_to_plain_text_full", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = runtime_with_paragraphs(1_000);
                runtime.layout();
                runtime.update(Message::SelectAll);
                runtime.tick();
                runtime
            },
            |runtime| {
                let state = runtime.state();
                state.selection.to_plain_text(&state.doc)
            },
        );
    });

    group.bench_function("selection_to_plain_text_partial", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = runtime_with_paragraphs(1_000);
                runtime.layout();
                runtime.update(Message::Navigate {
                    direction: Direction::DocumentStart,
                    extend: false,
                });
                for _ in 0..100 {
                    runtime.update(Message::Navigate {
                        direction: Direction::Down,
                        extend: true,
                    });
                }
                runtime.tick();
                runtime
            },
            |runtime| {
                let state = runtime.state();
                state.selection.to_plain_text(&state.doc)
            },
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_input,
    bench_select_all,
    bench_delete_selection,
    bench_paste,
    bench_commit,
    bench_get_text,
);
criterion_main!(benches);
