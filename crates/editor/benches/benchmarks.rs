use criterion::{Criterion, criterion_group, criterion_main};
use editor::layout::{Layout, LayoutCache, LayoutContext, Paginator};
use editor::model::{Decorations, Doc, Node, NodeId, Text, TextNode};
use editor::runtime::{Direction, Message, PasteMode, Runtime, State, ViewStates};
use editor::state::{
    Position, Selection, build_selection_decorations, compute_selection_aggregates,
};
use editor::transact;
use editor::transaction::Transaction;
use editor::types::{Affinity, BoxConstraints, Size, Theme};
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

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
    runtime
}

fn prepared_runtime(count: usize) -> Runtime {
    let mut runtime = runtime_with_paragraphs(count);
    runtime.layout();
    runtime
}

fn navigate_to_start(runtime: &mut Runtime) {
    runtime.update(Message::Navigate {
        direction: Direction::DocumentStart,
        extend: false,
    });
    runtime.tick();
}

fn navigate_to_end(runtime: &mut Runtime) {
    runtime.update(Message::Navigate {
        direction: Direction::DocumentEnd,
        extend: false,
    });
    runtime.tick();
}

fn navigate_to_middle(runtime: &mut Runtime) {
    navigate_to_start(runtime);
    for _ in 0..500 {
        runtime.update(Message::Navigate {
            direction: Direction::Down,
            extend: false,
        });
    }
    runtime.tick();
}

fn select_lines(runtime: &mut Runtime, count: usize) {
    for _ in 0..count {
        runtime.update(Message::Navigate {
            direction: Direction::Down,
            extend: true,
        });
    }
    runtime.tick();
}

// --- Existing benchmarks ---

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
    group
        .sample_size(10)
        .measurement_time(Duration::from_secs(10));

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
    group
        .sample_size(10)
        .measurement_time(Duration::from_secs(10));

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
                runtime.layout();
                runtime
            },
            |mut runtime| {
                runtime.update(Message::Paste {
                    html: None,
                    text: paste_text.clone(),
                    mode: PasteMode::Auto,
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

// --- New benchmarks ---

fn bench_input_positions(c: &mut Criterion) {
    let mut group = c.benchmark_group("input_positions");
    group.sample_size(10);

    group.bench_function("input_at_document_start", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_start(&mut runtime);
                runtime
            },
            |mut runtime| {
                runtime.update(Message::Input {
                    text: "a".to_string(),
                });
                runtime.tick();
                runtime.render_page(0);
                runtime.flush();
            },
        );
    });

    group.bench_function("input_at_document_middle", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_middle(&mut runtime);
                runtime
            },
            |mut runtime| {
                runtime.update(Message::Input {
                    text: "a".to_string(),
                });
                runtime.tick();
                let page_idx = runtime.pages().len() / 2;
                runtime.render_page(page_idx);
                runtime.flush();
            },
        );
    });

    group.finish();
}

fn bench_delete_backward(c: &mut Criterion) {
    let mut group = c.benchmark_group("delete_backward");
    group.sample_size(10);

    group.bench_function("delete_backward_at_end", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_end(&mut runtime);
                runtime
            },
            |mut runtime| {
                runtime.update(Message::DeleteBackward);
                runtime.tick();
                let page_count = runtime.pages().len();
                runtime.render_page(page_count - 1);
                runtime.flush();
            },
        );
    });

    group.bench_function("delete_backward_at_start", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_start(&mut runtime);
                runtime.update(Message::Navigate {
                    direction: Direction::Down,
                    extend: false,
                });
                runtime.update(Message::Navigate {
                    direction: Direction::LineStart,
                    extend: false,
                });
                runtime.tick();
                runtime
            },
            |mut runtime| {
                runtime.update(Message::DeleteBackward);
                runtime.tick();
                runtime.render_page(0);
                runtime.flush();
            },
        );
    });

    group.bench_function("delete_backward_at_middle", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_middle(&mut runtime);
                runtime
            },
            |mut runtime| {
                runtime.update(Message::DeleteBackward);
                runtime.tick();
                let page_idx = runtime.pages().len() / 2;
                runtime.render_page(page_idx);
                runtime.flush();
            },
        );
    });

    group.finish();
}

fn bench_insert_newline(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_newline");
    group.sample_size(10);

    group.bench_function("split_paragraph_at_middle", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_middle(&mut runtime);
                runtime
            },
            |mut runtime| {
                runtime.update(Message::InsertNewline);
                runtime.tick();
                let page_idx = runtime.pages().len() / 2;
                runtime.render_page(page_idx);
                runtime.flush();
            },
        );
    });

    group.finish();
}

fn bench_navigation(c: &mut Criterion) {
    let mut group = c.benchmark_group("navigation");
    group.sample_size(10);

    group.bench_function("navigate_down_from_start", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_start(&mut runtime);
                runtime
            },
            |mut runtime| {
                runtime.update(Message::Navigate {
                    direction: Direction::Down,
                    extend: false,
                });
                runtime.tick();
            },
        );
    });

    group.bench_function("navigate_up_from_end", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_end(&mut runtime);
                runtime
            },
            |mut runtime| {
                runtime.update(Message::Navigate {
                    direction: Direction::Up,
                    extend: false,
                });
                runtime.tick();
            },
        );
    });

    group.bench_function("navigate_right", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_middle(&mut runtime);
                runtime
            },
            |mut runtime| {
                runtime.update(Message::Navigate {
                    direction: Direction::Right,
                    extend: false,
                });
                runtime.tick();
            },
        );
    });

    group.bench_function("navigate_word_right", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_middle(&mut runtime);
                runtime
            },
            |mut runtime| {
                runtime.update(Message::Navigate {
                    direction: Direction::WordRight,
                    extend: false,
                });
                runtime.tick();
            },
        );
    });

    group.bench_function("navigate_document_end", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_start(&mut runtime);
                runtime
            },
            |mut runtime| {
                runtime.update(Message::Navigate {
                    direction: Direction::DocumentEnd,
                    extend: false,
                });
                runtime.tick();
            },
        );
    });

    group.bench_function("navigate_page_down", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_start(&mut runtime);
                runtime
            },
            |mut runtime| {
                runtime.update(Message::Navigate {
                    direction: Direction::PageDown,
                    extend: false,
                });
                runtime.tick();
            },
        );
    });

    group.finish();
}

fn bench_selection_extend(c: &mut Criterion) {
    let mut group = c.benchmark_group("selection_extend");
    group.sample_size(10);

    group.bench_function("extend_selection_100_lines", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_start(&mut runtime);
                runtime
            },
            |mut runtime| {
                for _ in 0..100 {
                    runtime.update(Message::Navigate {
                        direction: Direction::Down,
                        extend: true,
                    });
                }
                runtime.tick();
                runtime.render_page(0);
            },
        );
    });

    group.bench_function("extend_selection_full", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_start(&mut runtime);
                runtime
            },
            |mut runtime| {
                runtime.update(Message::Navigate {
                    direction: Direction::DocumentEnd,
                    extend: true,
                });
                runtime.tick();
                runtime.render_page(0);
            },
        );
    });

    group.finish();
}

fn bench_formatting(c: &mut Criterion) {
    let mut group = c.benchmark_group("formatting");
    group.sample_size(10);

    group.bench_function("toggle_bold_100_lines", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_start(&mut runtime);
                select_lines(&mut runtime, 100);
                runtime
            },
            |mut runtime| {
                runtime.update(Message::ToggleBold);
                runtime.tick();
                runtime.render_page(0);
                runtime.flush();
            },
        );
    });

    group.bench_function("toggle_bold_full", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                runtime.update(Message::SelectAll);
                runtime.tick();
                runtime
            },
            |mut runtime| {
                runtime.update(Message::ToggleBold);
                runtime.tick();
                runtime.render_page(0);
                runtime.flush();
            },
        );
    });

    group.bench_function("clear_formatting", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                runtime.update(Message::SelectAll);
                runtime.tick();
                runtime.update(Message::ToggleBold);
                runtime.tick();
                runtime.update(Message::SelectAll);
                runtime.tick();
                runtime
            },
            |mut runtime| {
                runtime.update(Message::ClearFormatting);
                runtime.tick();
                runtime.render_page(0);
                runtime.flush();
            },
        );
    });

    group.finish();
}

fn bench_undo_redo(c: &mut Criterion) {
    let mut group = c.benchmark_group("undo_redo");
    group.sample_size(10);

    group.bench_function("undo_after_input", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_end(&mut runtime);
                runtime.update(Message::Input {
                    text: "hello world".to_string(),
                });
                runtime.tick();
                runtime.flush();
                runtime
            },
            |mut runtime| {
                runtime.update(Message::Undo);
                runtime.tick();
                let page_count = runtime.pages().len();
                runtime.render_page(page_count - 1);
                runtime.flush();
            },
        );
    });

    group.bench_function("undo_after_delete", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                runtime.update(Message::SelectAll);
                runtime.tick();
                runtime.update(Message::DeleteBackward);
                runtime.tick();
                while !runtime.state().garbage_ids.is_empty() {
                    runtime.flush();
                }
                runtime
            },
            |mut runtime| {
                runtime.update(Message::Undo);
                runtime.tick();
                runtime.render_page(0);
                runtime.flush();
            },
        );
    });

    group.finish();
}

fn bench_layout(c: &mut Criterion) {
    let mut group = c.benchmark_group("layout");
    group.sample_size(10);

    group.bench_function("layout_full", |b| {
        b.iter_with_setup(
            || runtime_with_paragraphs(1_000),
            |mut runtime| {
                runtime.layout();
            },
        );
    });

    group.bench_function("layout_cached", |b| {
        b.iter_with_setup(
            || prepared_runtime(1_000),
            |mut runtime| {
                runtime.layout();
            },
        );
    });

    group.bench_function("layout_after_single_edit", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_middle(&mut runtime);
                runtime.update(Message::Input {
                    text: "x".to_string(),
                });
                runtime.tick();
                runtime
            },
            |mut runtime| {
                runtime.layout();
            },
        );
    });

    group.finish();
}

fn bench_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("render");
    group.sample_size(10);

    group.bench_function("render_page_no_selection", |b| {
        b.iter_with_setup(
            || prepared_runtime(1_000),
            |mut runtime| {
                runtime.render_page(0);
            },
        );
    });

    group.bench_function("render_page_with_selection", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                runtime.update(Message::SelectAll);
                runtime.tick();
                runtime
            },
            |mut runtime| {
                runtime.render_page(0);
            },
        );
    });

    group.finish();
}

fn bench_resize(c: &mut Criterion) {
    let mut group = c.benchmark_group("resize");
    group.sample_size(10);

    group.bench_function("resize_viewport", |b| {
        b.iter_with_setup(
            || prepared_runtime(1_000),
            |mut runtime| {
                runtime.update(Message::Resize {
                    width: 600.0,
                    height: 800.0,
                    scale_factor: 1.0,
                });
                runtime.tick();
                runtime.render_page(0);
            },
        );
    });

    group.finish();
}

fn bench_document_traversal(c: &mut Criterion) {
    let mut group = c.benchmark_group("document_traversal");
    group.sample_size(50);

    group.bench_function("to_spellcheck_text", |b| {
        b.iter_with_setup(
            || prepared_runtime(1_000),
            |runtime| runtime.state().doc.to_spellcheck_text(),
        );
    });

    group.bench_function("iter_blocks", |b| {
        b.iter_with_setup(
            || prepared_runtime(1_000),
            |runtime| {
                let doc = &runtime.state().doc;
                let mut count = 0usize;
                for _ in doc.iter_blocks() {
                    count += 1;
                }
                count
            },
        );
    });

    group.bench_function("iter_segments", |b| {
        b.iter_with_setup(
            || prepared_runtime(1_000),
            |runtime| {
                let doc = &runtime.state().doc;
                let mut count = 0usize;
                for _ in doc.iter_segments() {
                    count += 1;
                }
                count
            },
        );
    });

    group.finish();
}

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");
    group.sample_size(10);

    group.bench_function("search_query", |b| {
        b.iter_with_setup(
            || prepared_runtime(1_000),
            |mut runtime| {
                runtime.update(Message::Search {
                    query: "Paragraph".to_string(),
                    match_whole_word: false,
                });
                runtime.tick();
                runtime.render_page(0);
            },
        );
    });

    group.bench_function("find_next", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                runtime.update(Message::Search {
                    query: "Paragraph".to_string(),
                    match_whole_word: false,
                });
                runtime.tick();
                runtime
            },
            |mut runtime| {
                runtime.update(Message::FindNext);
                runtime.tick();
            },
        );
    });

    group.bench_function("replace_all", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                runtime.update(Message::Search {
                    query: "Paragraph".to_string(),
                    match_whole_word: false,
                });
                runtime.tick();
                runtime
            },
            |mut runtime| {
                runtime.update(Message::ReplaceAll {
                    replacement: "Section".to_string(),
                });
                runtime.tick();
                runtime.render_page(0);
                runtime.flush();
            },
        );
    });

    group.finish();
}

fn bench_garbage_collection(c: &mut Criterion) {
    let mut group = c.benchmark_group("garbage_collection");
    group
        .sample_size(10)
        .measurement_time(Duration::from_secs(10));

    group.bench_function("flush_after_delete_all", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                runtime.update(Message::SelectAll);
                runtime.tick();
                runtime.update(Message::DeleteBackward);
                runtime.tick();
                runtime
            },
            |mut runtime| {
                while !runtime.state().garbage_ids.is_empty() {
                    runtime.flush();
                }
            },
        );
    });

    group.finish();
}

fn bench_selection_snapshot(c: &mut Criterion) {
    let mut group = c.benchmark_group("selection_snapshot");
    group.sample_size(10);

    group.bench_function("selection_aggregates_full", |b| {
        b.iter_with_setup(
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
        );
    });

    group.bench_function("selection_aggregates_partial", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_start(&mut runtime);
                select_lines(&mut runtime, 100);
                runtime
            },
            |runtime| {
                let state = runtime.state();
                let (from, to) = state.selection.as_sorted(&state.doc).unwrap();
                let block_ids: Vec<NodeId> = state.doc.iter_blocks().map(|(id, _)| id).collect();
                compute_selection_aggregates(&state.doc, &block_ids, from, to)
            },
        );
    });

    group.finish();
}

fn bench_continuous_typing(c: &mut Criterion) {
    let mut group = c.benchmark_group("continuous_typing");
    group.sample_size(10);

    group.bench_function("type_10_chars", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                navigate_to_middle(&mut runtime);
                runtime
            },
            |mut runtime| {
                for ch in "abcdefghij".chars() {
                    runtime.update(Message::Input {
                        text: ch.to_string(),
                    });
                    runtime.tick();
                    let page_idx = runtime.pages().len() / 2;
                    runtime.render_page(page_idx);
                }
                runtime.flush();
            },
        );
    });

    group.finish();
}

fn bench_layout_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("layout_pipeline");
    group.sample_size(10);

    group.bench_function("1_crdt_children_collect", |b| {
        b.iter_with_setup(
            || prepared_runtime(1_000),
            |runtime| {
                let doc = &runtime.state().doc;
                let root = doc.node(NodeId::ROOT).unwrap();
                let children: Vec<_> = root.children().collect();
                for child in &children {
                    let _grandchildren: Vec<_> = child.children().collect();
                }
                children.len()
            },
        );
    });

    group.bench_function("2_crdt_node_decode", |b| {
        b.iter_with_setup(
            || prepared_runtime(1_000),
            |runtime| {
                let doc = &runtime.state().doc;
                let root = doc.node(NodeId::ROOT).unwrap();
                let children: Vec<_> = root.children().collect();
                for child in &children {
                    let _ = child.node();
                    for grandchild in child.children() {
                        let _ = grandchild.node();
                    }
                }
                children.len()
            },
        );
    });

    group.bench_function("3_tree_layout_uncached", |b| {
        b.iter_with_setup(
            || prepared_runtime(1_000),
            |runtime| {
                let doc = &runtime.state().doc;
                let settings = doc.settings();
                let root = doc.node(NodeId::ROOT).unwrap();
                let cache = RefCell::new(LayoutCache::new());
                let decorations = Decorations::default();
                let view_states = ViewStates::default();
                let constraints =
                    BoxConstraints::loose(Size::new(800.0 - 96.0 * 2.0, f32::INFINITY));
                let ctx =
                    LayoutContext::new(&root, &settings, &decorations, 1.0, &view_states, &cache);
                root.node().layout(&ctx, constraints)
            },
        );
    });

    group.bench_function("4_tree_layout_cached", |b| {
        b.iter_with_setup(
            || {
                let runtime = prepared_runtime(1_000);
                let doc = runtime.state().doc.clone();
                let settings = doc.settings();
                let cache = RefCell::new(LayoutCache::new());
                {
                    let root = doc.node(NodeId::ROOT).unwrap();
                    let decorations = Decorations::default();
                    let view_states = ViewStates::default();
                    let constraints =
                        BoxConstraints::loose(Size::new(800.0 - 96.0 * 2.0, f32::INFINITY));
                    let ctx = LayoutContext::new(
                        &root,
                        &settings,
                        &decorations,
                        1.0,
                        &view_states,
                        &cache,
                    );
                    let _ = root.node().layout(&ctx, constraints);
                }
                (doc, settings, cache)
            },
            |(doc, settings, cache): (Rc<Doc>, _, _)| {
                let root = doc.node(NodeId::ROOT).unwrap();
                let decorations = Decorations::default();
                let view_states = ViewStates::default();
                let constraints =
                    BoxConstraints::loose(Size::new(800.0 - 96.0 * 2.0, f32::INFINITY));
                let ctx =
                    LayoutContext::new(&root, &settings, &decorations, 1.0, &view_states, &cache);
                root.node().layout(&ctx, constraints)
            },
        );
    });

    group.bench_function("5_pagination", |b| {
        b.iter_with_setup(
            || {
                let runtime = prepared_runtime(1_000);
                let doc = runtime.state().doc.clone();
                let settings = doc.settings();
                let root = doc.node(NodeId::ROOT).unwrap();
                let cache = RefCell::new(LayoutCache::new());
                let decorations = Decorations::default();
                let view_states = ViewStates::default();
                let constraints =
                    BoxConstraints::loose(Size::new(800.0 - 96.0 * 2.0, f32::INFINITY));
                let ctx =
                    LayoutContext::new(&root, &settings, &decorations, 1.0, &view_states, &cache);
                let layout_root = root.node().layout(&ctx, constraints);
                (layout_root, settings)
            },
            |(layout_root, settings)| {
                let paginator =
                    Paginator::new(800.0, f32::INFINITY, 96.0, 96.0, 96.0, settings.layout_mode);
                paginator.paginate(layout_root)
            },
        );
    });

    group.bench_function("6_render_only", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                runtime.layout();
                runtime
            },
            |mut runtime| runtime.render_page(0),
        );
    });

    group.bench_function("7_selection_snapshot", |b| {
        b.iter_with_setup(
            || {
                let mut runtime = prepared_runtime(1_000);
                runtime.update(Message::SelectAll);
                runtime.tick();
                let doc = runtime.state().doc.clone();
                let selection = runtime.state().selection.clone();
                (doc, selection)
            },
            |(doc, selection)| build_selection_decorations(&doc, &selection, None),
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
    bench_input_positions,
    bench_delete_backward,
    bench_insert_newline,
    bench_navigation,
    bench_selection_extend,
    bench_formatting,
    bench_undo_redo,
    bench_layout,
    bench_render,
    bench_resize,
    bench_document_traversal,
    bench_search,
    bench_garbage_collection,
    bench_selection_snapshot,
    bench_continuous_typing,
    bench_layout_pipeline,
);
criterion_main!(benches);
