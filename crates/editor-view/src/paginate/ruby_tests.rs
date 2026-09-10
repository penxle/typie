use editor_common::EdgeInsets;
use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
use editor_model::{
    AliasLog, Anchor, Bias, DocLogs, DocView, Modifier, ModifierAttrLog, ModifierAttrOp,
    NodeAttrLog, NodeType, SeqItem, SpanLog, SpanOp, project_document,
};
use editor_resource::Resource;

use crate::measure::context::MeasureContext;
use crate::measure::nodes::dispatch::measure_node;
use crate::measure::types::MeasuredTree;

use super::paginator::Paginator;
use super::types::{LayoutContent, LayoutLine, LayoutNode, PaginatedLayout};

fn document(paragraphs: &[(&str, Option<&str>)], line_height: u32, gap: u32) -> DocLogs {
    wrapped_document(paragraphs, line_height, gap, &[])
}

fn wrapped_document(
    paragraphs: &[(&str, Option<&str>)],
    line_height: u32,
    gap: u32,
    wrappers: &[NodeType],
) -> DocLogs {
    let mut events = Vec::new();
    let mut spans = SpanLog::new();
    let mut parents = vec![Dot::ROOT];
    for node_type in wrappers {
        let pos = events.len();
        let id = Dot::new(1, pos as u64 + 1);
        events.push(InputEvent {
            id,
            parents: if pos == 0 {
                vec![]
            } else {
                vec![Dot::new(1, pos as u64)]
            },
            op: ListOp::Ins {
                pos,
                item: SeqItem::Block {
                    node_type: *node_type,
                    parents: parents.clone(),
                    attrs: vec![],
                },
            },
        });
        parents.push(id);
    }
    for (text, ruby) in paragraphs {
        let para = Dot::new(1, events.len() as u64 + 1);
        let first_char = Dot::new(1, para.clock + 1);
        let items = std::iter::once(SeqItem::Block {
            node_type: NodeType::Paragraph,
            parents: parents.clone(),
            attrs: vec![],
        })
        .chain(text.chars().map(SeqItem::Char));
        for item in items {
            let pos = events.len();
            events.push(InputEvent {
                id: Dot::new(1, pos as u64 + 1),
                parents: if pos == 0 {
                    vec![]
                } else {
                    vec![Dot::new(1, pos as u64)]
                },
                op: ListOp::Ins { pos, item },
            });
        }
        if let Some(ruby) = ruby {
            spans = spans
                .apply(
                    para,
                    SpanOp::AddSpan {
                        start: Anchor {
                            id: first_char,
                            bias: Bias::Before,
                        },
                        end: Anchor {
                            id: Dot::new(1, events.len() as u64),
                            bias: Bias::After,
                        },
                        modifier: Modifier::Ruby {
                            text: (*ruby).to_string(),
                        },
                    },
                )
                .unwrap();
        }
    }
    let mut block_modifiers = ModifierAttrLog::new();
    for (i, modifier) in [
        Modifier::LineHeight { value: line_height },
        Modifier::BlockGap { value: gap },
    ]
    .into_iter()
    .enumerate()
    {
        block_modifiers = block_modifiers
            .apply(
                Dot::new(2, i as u64 + 1),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier,
                },
            )
            .unwrap();
    }
    DocLogs {
        seq: build_oplog(&events),
        spans,
        block_modifiers,
        node_attrs: NodeAttrLog::new(),
        node_carries: ModifierAttrLog::new(),
        aliases: AliasLog::new(),
    }
}

fn layout(doc: &DocLogs, paginator: Paginator) -> PaginatedLayout {
    let pd = project_document(doc).unwrap();
    let view = DocView::new(&pd);
    let measured = measure_node(
        &mut crate::measure::Measurer::new(),
        &view.root().unwrap(),
        300.0,
        &MeasureContext::default(),
        &mut Resource::new_test(),
    );
    paginator.paginate(MeasuredTree { root: measured })
}

fn lines(node: &LayoutNode) -> Vec<(&LayoutNode, &LayoutLine)> {
    match &node.content {
        LayoutContent::Line(line) if !line.is_phantom => vec![(node, line)],
        LayoutContent::Box(b) => b.children.iter().flat_map(lines).collect(),
        _ => vec![],
    }
}

fn ruby_top(node: &LayoutNode, line: &LayoutLine) -> f32 {
    node.rect.y
        + line
            .ruby_annotations
            .iter()
            .map(|ruby| ruby.baseline_y - ruby.ascent)
            .reduce(f32::min)
            .expect("ruby annotation")
}

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "got {actual}, expected {expected}"
    );
}

#[test]
fn page_margin_is_used_before_adding_only_the_missing_clearance() {
    let plain = document(&[("abc", None)], 160, 0);
    let ruby = document(&[("abc", Some("annotation"))], 160, 0);
    for margin in [40.0, 1.0, 0.0] {
        let paginator = || Paginator::paginated(380.0, 500.0, EdgeInsets::all(margin));
        let plain = layout(&plain, paginator());
        let ruby = layout(&ruby, paginator());
        let (plain_node, plain_line) = lines(&plain.tree.root)[0];
        let (ruby_node, ruby_line) = lines(&ruby.tree.root)[0];
        close(ruby_node.rect.height, plain_node.rect.height);
        close(ruby_line.baseline, plain_line.baseline);
        if margin == 40.0 {
            close(ruby_node.rect.y, plain_node.rect.y);
            assert!(ruby_top(ruby_node, ruby_line) < ruby.pages[0].content_y_start);
        } else {
            close(ruby_top(ruby_node, ruby_line), 2.0);
        }
    }
}

#[test]
fn adjacent_lines_share_all_available_leading_and_paragraph_spacing() {
    for gap in [0, 100] {
        let plain = document(&[("abc", None), ("def", None)], 240, gap);
        let ruby = document(&[("abc", None), ("def", Some("annotation"))], 240, gap);
        let paginator = || Paginator::continuous(380.0, 100_000.0, EdgeInsets::all(40.0));
        let plain = layout(&plain, paginator());
        let ruby = layout(&ruby, paginator());
        let plain_lines = lines(&plain.tree.root);
        let ruby_lines = lines(&ruby.tree.root);
        for ((plain_node, _), (ruby_node, _)) in plain_lines.iter().zip(&ruby_lines) {
            assert_eq!(ruby_node.rect, plain_node.rect);
        }
        let (previous, previous_line) = ruby_lines[0];
        let (current, current_line) = ruby_lines[1];
        assert!(
            ruby_top(current, current_line)
                >= previous.rect.y + previous_line.baseline + previous_line.descent + 2.0 - 0.001
        );
    }
}

#[test]
fn tight_lines_add_only_the_missing_space() {
    let state = document(&[("abc", None), ("def", Some("annotation"))], 100, 0);
    let layout = layout(
        &state,
        Paginator::continuous(380.0, 100_000.0, EdgeInsets::all(40.0)),
    );
    let lines = lines(&layout.tree.root);
    let (previous, previous_line) = lines[0];
    let (current, current_line) = lines[1];
    close(
        ruby_top(current, current_line),
        previous.rect.y + previous_line.baseline + previous_line.descent + 2.0,
    );
}

#[test]
fn indexed_previous_content_bottom_includes_earlier_tall_text() {
    let mut doc = document(
        &[("a", None), ("b", None), ("c", Some("annotation"))],
        50,
        0,
    );
    doc.spans = doc
        .spans
        .apply(
            Dot::new(3, 1),
            SpanOp::AddSpan {
                start: Anchor {
                    id: Dot::new(1, 2),
                    bias: Bias::Before,
                },
                end: Anchor {
                    id: Dot::new(1, 2),
                    bias: Bias::After,
                },
                modifier: Modifier::FontSize { value: 6000 },
            },
        )
        .unwrap();
    let layout = layout(
        &doc,
        Paginator::continuous(380.0, 100_000.0, EdgeInsets::all(40.0)),
    );
    let all_lines = lines(&layout.tree.root);
    let first_bottom = all_lines[0].0.rect.y + all_lines[0].1.baseline + all_lines[0].1.descent;
    let second_bottom = all_lines[1].0.rect.y + all_lines[1].1.baseline + all_lines[1].1.descent;
    assert!(first_bottom > second_bottom);
    let ruby_node = all_lines[2].1.node;
    let index = crate::query::layout_index::LayoutIndex::new(layout.tree, &layout.pages);
    close(
        index.box_entry(&ruby_node).unwrap().previous_content_bottom,
        first_bottom,
    );
}

#[test]
fn automatic_page_break_recomputes_clearance_from_the_new_page_margin() {
    let doc = document(&[("abc", None), ("def", Some("annotation"))], 100, 0);
    let continuous = layout(
        &doc,
        Paginator::continuous(380.0, 100_000.0, EdgeInsets::all(40.0)),
    );
    let line_height = lines(&continuous.tree.root)[0].0.rect.height;
    let paginated = layout(
        &doc,
        Paginator::paginated(380.0, 80.0 + 2.0 * line_height + 0.5, EdgeInsets::all(40.0)),
    );
    assert_eq!(paginated.pages.len(), 2);
    let (node, line) = lines(&paginated.tree.root)[1];
    close(node.rect.y, paginated.pages[1].content_y_start);
    assert!(ruby_top(node, line) >= paginated.pages[1].y_start + 2.0);
    assert!(ruby_top(node, line) < paginated.pages[1].content_y_start);
    for (i, page) in paginated.pages.iter().enumerate() {
        crate::page_fragment::build_page_fragment_tree(&paginated.tree, i, page);
    }
}

#[test]
fn callout_and_table_cell_use_padding_before_growing() {
    for wrappers in [
        &[NodeType::Callout][..],
        &[NodeType::Table, NodeType::TableRow, NodeType::TableCell][..],
    ] {
        let plain = wrapped_document(&[("abc", None)], 160, 0, wrappers);
        let ruby = wrapped_document(&[("abc", Some("annotation"))], 160, 0, wrappers);
        let paginator = || Paginator::continuous(380.0, 100_000.0, EdgeInsets::all(40.0));
        let plain = layout(&plain, paginator());
        let ruby = layout(&ruby, paginator());
        let (plain_node, _) = lines(&plain.tree.root)[0];
        let (ruby_node, ruby_line) = lines(&ruby.tree.root)[0];
        close(ruby_node.rect.height, plain_node.rect.height);
        if wrappers[0] == NodeType::Callout {
            assert_eq!(ruby_node.rect, plain_node.rect);
        } else {
            close(
                ruby_top(ruby_node, ruby_line),
                ruby.tree.root.rect.y + 1.0 + 2.0,
            );
            let delta = ruby_node.rect.y - plain_node.rect.y;
            assert!(delta > 0.0);
            close(
                ruby.tree.root.rect.height - plain.tree.root.rect.height,
                delta,
            );
        }
    }
}

#[test]
fn wrapped_lines_with_ruby_keep_two_pixels_between_annotations_and_base_text() {
    let doc = document(
        &[(
            "abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz abcdefghijklmnopqrstuvwxyz",
            Some(
                "annotation annotation annotation annotation annotation annotation annotation annotation",
            ),
        )],
        100,
        0,
    );
    let layout = layout(
        &doc,
        Paginator::continuous(380.0, 100_000.0, EdgeInsets::all(40.0)),
    );
    let lines = lines(&layout.tree.root);
    assert!(lines.len() > 2);
    for pair in lines.windows(2) {
        let (previous, previous_line) = pair[0];
        let (current, current_line) = pair[1];
        close(
            ruby_top(current, current_line),
            previous.rect.y + previous_line.baseline + previous_line.descent + 2.0,
        );
    }
}

#[test]
fn ruby_does_not_change_hit_testing_in_previous_lines_leading() {
    let doc = document(&[("abc", None), ("def", Some("annotation"))], 240, 0);
    let layout = layout(
        &doc,
        Paginator::continuous(380.0, 100_000.0, EdgeInsets::all(40.0)),
    );
    let all_lines = lines(&layout.tree.root);
    let (node, line) = all_lines[1];
    let x = node.rect.x + line.ruby_annotations[0].x + 1.0;
    let y = ruby_top(node, line) + 1.0;
    assert!(y < all_lines[0].0.rect.bottom());
    let expected_node = all_lines[0].1.node;
    let index = crate::query::layout_index::LayoutIndex::new(layout.tree, &layout.pages);
    let selection = crate::query::hit_test::hit_test(&index, 0, x, y).unwrap();
    assert_eq!(selection.head.node, expected_node);
    let previous_selection = crate::query::hit_test::hit_test(&index, 0, x, 50.0).unwrap();
    let projected = project_document(&doc).unwrap();
    let rects = crate::query::hit_test::cursor_hit_rects(
        &index,
        &DocView::new(&projected),
        &previous_selection,
    );
    assert!(rects.iter().any(|rect| rect.rect.contains(x, y)));
}

#[test]
fn continuous_page_fragments_include_ruby_crossing_the_rendering_boundary() {
    use crate::page_fragment::{PageFragmentContent, PageFragmentNode};
    fn contains_ruby(node: &PageFragmentNode) -> bool {
        match &node.content {
            PageFragmentContent::Line(line) => !line.ruby_annotations.is_empty(),
            PageFragmentContent::Box(b) => b.children.iter().any(contains_ruby),
            _ => false,
        }
    }
    let doc = document(&[("abc", None), ("def", Some("annotation"))], 240, 0);
    let layout = layout(
        &doc,
        Paginator::continuous(380.0, 20.0, EdgeInsets::all(40.0)),
    );
    assert_eq!(layout.pages.len(), 2);
    let (node, line) = lines(&layout.tree.root)[1];
    assert!(ruby_top(node, line) < layout.pages[1].y_start);
    for (i, page) in layout.pages.iter().enumerate() {
        let fragment = crate::page_fragment::build_page_fragment_tree(&layout.tree, i, page);
        assert!(
            contains_ruby(fragment.root.as_ref().unwrap()),
            "missing ruby on rendering page {i}"
        );
    }
}
