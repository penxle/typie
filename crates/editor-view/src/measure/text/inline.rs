use std::collections::BTreeMap;
use std::ops::Range;

use editor_model::{InlineKind, Modifier, ModifierType, NodeType, NodeView, OwnModifier};

use super::resolve::{ResolvedTextStyle, style_from_effective_modifiers};

pub(crate) struct TextRun<'a> {
    pub byte_range: Range<usize>,
    pub offset_range: Range<usize>,
    pub own_modifiers: &'a BTreeMap<ModifierType, OwnModifier>,
    pub effective: &'a BTreeMap<ModifierType, Modifier>,
    pub style: ResolvedTextStyle,
}

pub(crate) struct TabMark<'a> {
    pub offset_index: usize,
    pub byte_offset: usize,
    pub own_modifiers: &'a BTreeMap<ModifierType, OwnModifier>,
    pub effective: &'a BTreeMap<ModifierType, Modifier>,
    pub style: ResolvedTextStyle,
}

pub(crate) fn collect_text_runs<'a>(
    node: &NodeView<'a>,
) -> (String, Vec<TextRun<'a>>, Vec<TabMark<'a>>) {
    let text = node.inline_text();
    let mut runs: Vec<TextRun<'a>> = Vec::new();
    let mut tabs: Vec<TabMark<'a>> = Vec::new();
    let mut byte_cursor = 0usize;
    let mut open: Option<usize> = None;
    for (offset, item) in node.inline().into_iter().enumerate() {
        match &item.kind {
            InlineKind::Char { byte_range, .. } => {
                byte_cursor = byte_range.end;
                let mergeable = match open {
                    Some(i) => {
                        let r = &runs[i];
                        r.own_modifiers == item.own_modifiers
                            && r.effective == item.effective
                            && r.offset_range.end == offset
                    }
                    None => false,
                };
                if mergeable {
                    let r = &mut runs[open.unwrap()];
                    r.byte_range.end = byte_range.end;
                    r.offset_range.end = offset + 1;
                } else {
                    open = Some(runs.len());
                    runs.push(TextRun {
                        byte_range: byte_range.clone(),
                        offset_range: offset..offset + 1,
                        own_modifiers: item.own_modifiers,
                        effective: item.effective,
                        style: style_from_effective_modifiers(
                            &item.effective.values().cloned().collect::<Vec<_>>(),
                        ),
                    });
                }
            }
            InlineKind::Atom(NodeType::Tab) => {
                tabs.push(TabMark {
                    offset_index: offset,
                    byte_offset: byte_cursor,
                    own_modifiers: item.own_modifiers,
                    effective: item.effective,
                    style: style_from_effective_modifiers(
                        &item.effective.values().cloned().collect::<Vec<_>>(),
                    ),
                });
                open = None;
            }
            InlineKind::Atom(_) => {
                open = None;
            }
        }
    }
    (text, runs, tabs)
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Segment {
    Text {
        offset_range: Range<usize>,
        byte_range: Range<usize>,
    },
    Empty {
        offset_range: Range<usize>,
    },
}

pub(crate) fn split_segments(node: &NodeView) -> Vec<Segment> {
    let mut segments: Vec<Segment> = Vec::new();
    let mut seg_start_offset = 0usize;
    let mut seg_start_byte = 0usize;
    let mut offset = 0usize;
    let mut byte = 0usize;
    let mut seg_has_flow = false;
    let mut last_was_break = false;
    let mut count = 0usize;
    for item in node.inline() {
        count += 1;
        match &item.kind {
            InlineKind::Char { byte_range, .. } => {
                byte = byte_range.end;
                seg_has_flow = true;
                last_was_break = false;
            }
            InlineKind::Atom(NodeType::HardBreak) => {
                if seg_has_flow {
                    segments.push(Segment::Text {
                        offset_range: seg_start_offset..offset,
                        byte_range: seg_start_byte..byte,
                    });
                } else {
                    segments.push(Segment::Empty {
                        offset_range: seg_start_offset..seg_start_offset,
                    });
                }
                seg_start_offset = offset + 1;
                seg_start_byte = byte;
                seg_has_flow = false;
                last_was_break = true;
            }
            InlineKind::Atom(NodeType::Tab) => {
                seg_has_flow = true;
                last_was_break = false;
            }
            InlineKind::Atom(_) => {
                last_was_break = false;
            }
        }
        offset += 1;
    }
    if count == 0 {
        segments.push(Segment::Empty { offset_range: 0..0 });
    } else if last_was_break {
        segments.push(Segment::Empty {
            offset_range: seg_start_offset..seg_start_offset,
        });
    } else {
        segments.push(Segment::Text {
            offset_range: seg_start_offset..offset,
            byte_range: seg_start_byte..byte,
        });
    }
    segments
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RubyGroup {
    pub text: String,
    pub offset_range: Range<usize>,
    pub total_base_chars: usize,
}

pub(crate) fn identify_ruby_groups(node: &NodeView) -> Vec<RubyGroup> {
    fn flush(current: &mut Option<RubyGroup>, groups: &mut Vec<RubyGroup>) {
        if let Some(g) = current.take()
            && !g.text.is_empty()
            && g.total_base_chars > 0
        {
            groups.push(g);
        }
    }
    let mut groups: Vec<RubyGroup> = Vec::new();
    let mut current: Option<RubyGroup> = None;
    for (offset, item) in node.inline().into_iter().enumerate() {
        match &item.kind {
            InlineKind::Char { .. } => {
                let ruby_text: Option<&str> =
                    item.own_modifiers
                        .get(&ModifierType::Ruby)
                        .and_then(|o| match &o.value {
                            Modifier::Ruby { text } => Some(text.as_str()),
                            _ => None,
                        });
                match ruby_text {
                    Some(t) if !t.is_empty() => match current.as_mut() {
                        Some(g) if g.text == t => {
                            g.total_base_chars += 1;
                            g.offset_range.end = offset + 1;
                        }
                        _ => {
                            flush(&mut current, &mut groups);
                            current = Some(RubyGroup {
                                text: t.to_owned(),
                                offset_range: offset..offset + 1,
                                total_base_chars: 1,
                            });
                        }
                    },
                    _ => flush(&mut current, &mut groups),
                }
            }
            InlineKind::Atom(_) => flush(&mut current, &mut groups),
        }
    }
    flush(&mut current, &mut groups);
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        Anchor, AtomLeaf, Bias, DocLogs, DocView, Modifier, ModifierAttrLog, ModifierAttrOp,
        ModifierType, NodeAttrLog, NodeType, SeqItem, SpanLog, SpanOp, project_document,
    };

    fn logs(items: &[(Dot, SeqItem)]) -> DocLogs {
        let mut ev = Vec::new();
        let mut prev: Option<Dot> = None;
        for (i, (id, item)) in items.iter().enumerate() {
            ev.push(InputEvent {
                id: *id,
                parents: prev.into_iter().collect(),
                op: ListOp::Ins {
                    pos: i,
                    item: item.clone(),
                },
            });
            prev = Some(*id);
        }
        DocLogs {
            seq: build_oplog(&ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_carries: ModifierAttrLog::new(),
        }
    }

    // Root > Paragraph with `children` appended as leaves. Char/atom leaf i is Dot(1, 2 + i).
    fn build_logs(children: Vec<SeqItem>) -> DocLogs {
        let root = Dot::ROOT;
        let p = Dot::new(1, 1);
        let mut items = vec![(
            p,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
            },
        )];
        for (i, c) in children.into_iter().enumerate() {
            items.push((Dot::new(1, 2 + i as u64), c));
        }
        logs(&items)
    }

    fn ch(c: char) -> SeqItem {
        SeqItem::Char(c)
    }
    fn tab() -> SeqItem {
        SeqItem::Atom(AtomLeaf::Tab)
    }
    fn hb() -> SeqItem {
        SeqItem::Atom(AtomLeaf::HardBreak)
    }
    fn pb() -> SeqItem {
        SeqItem::Atom(AtomLeaf::PageBreak)
    }
    fn leaf(i: u64) -> Dot {
        Dot::new(1, 2 + i)
    }
    fn anc(d: Dot, bias: Bias) -> Anchor {
        Anchor { id: d, bias }
    }

    #[test]
    fn single_run_plain_chars() {
        let pd = project_document(&build_logs(vec![ch('a'), ch('b'), ch('c')])).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let (text, runs, tabs) = collect_text_runs(&para);
        assert_eq!(text, "abc");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].offset_range, 0..3);
        assert_eq!(runs[0].byte_range, 0..3);
        assert!(tabs.is_empty());
    }

    #[test]
    fn pair_coalesce_splits_at_bold_span_mid_word() {
        // "abcdef"; bold on chars c,d (leaf 2,3 == Dot(1,4),(1,5)) -> 3 runs.
        let mut l = build_logs(vec![ch('a'), ch('b'), ch('c'), ch('d'), ch('e'), ch('f')]);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(50, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(2), Bias::Before),
                    end: anc(leaf(3), Bias::After),
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let (_text, runs, _tabs) = collect_text_runs(&para);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0].offset_range, 0..2);
        assert_eq!(runs[1].offset_range, 2..4);
        assert_eq!(runs[2].offset_range, 4..6);
        assert_eq!(runs[0].byte_range, 0..2);
        assert_eq!(runs[1].byte_range, 2..4);
        assert_eq!(runs[2].byte_range, 4..6);
        assert!(runs[1].own_modifiers.contains_key(&ModifierType::Bold));
        assert!(!runs[0].own_modifiers.contains_key(&ModifierType::Bold));
    }

    #[test]
    fn color_only_span_still_splits_run() {
        // Identical font everywhere; a TextColor span on the middle char must still split.
        let mut l = build_logs(vec![ch('a'), ch('b'), ch('c')]);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(51, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(1), Bias::Before),
                    end: anc(leaf(1), Bias::After),
                    modifier: Modifier::TextColor {
                        value: "red".to_string(),
                    },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let (_t, runs, _tabs) = collect_text_runs(&para);
        assert_eq!(runs.len(), 3);
        assert_eq!(
            runs[1].effective.get(&ModifierType::TextColor),
            Some(&Modifier::TextColor {
                value: "red".to_string()
            })
        );
    }

    #[test]
    fn offset_byte_diverge_and_no_cross_tab_merge() {
        // "ab" Tab "cd": identical (empty) own/effective on all four chars,
        // yet the Tab must split them into TWO runs (offset guard + reset).
        let pd =
            project_document(&build_logs(vec![ch('a'), ch('b'), tab(), ch('c'), ch('d')])).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let (text, runs, tabs) = collect_text_runs(&para);
        assert_eq!(text, "abcd");
        assert_eq!(runs.len(), 2);
        // run before the Tab
        assert_eq!(runs[0].offset_range, 0..2);
        assert_eq!(runs[0].byte_range, 0..2);
        // run after the Tab: byte continues at 2 (char-only) but offset starts at 3 (Tab is offset 2)
        assert_eq!(runs[1].offset_range, 3..5);
        assert_eq!(runs[1].byte_range, 2..4);
        assert_eq!(tabs.len(), 1);
        assert_eq!(tabs[0].offset_index, 2);
        assert_eq!(tabs[0].byte_offset, 2);
    }

    #[test]
    fn provenance_own_vs_inherited_font_size() {
        // root block FontSize(1600) -> char b inherits; char a ALSO owns the same size via span.
        // Same effective, different own -> 2 runs; only run[0] carries own FontSize.
        let mut l = build_logs(vec![ch('a'), ch('b')]);
        l.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::new(52, 1),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap();
        l.spans = SpanLog::new()
            .apply(
                Dot::new(53, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(0), Bias::Before),
                    end: anc(leaf(0), Bias::After),
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let (_t, runs, _tabs) = collect_text_runs(&para);
        assert_eq!(runs.len(), 2);
        assert!(runs[0].own_modifiers.contains_key(&ModifierType::FontSize)); // own
        assert!(!runs[1].own_modifiers.contains_key(&ModifierType::FontSize)); // inherited only
        assert!(runs[0].effective.contains_key(&ModifierType::FontSize));
        assert!(runs[1].effective.contains_key(&ModifierType::FontSize));
    }

    #[test]
    fn decoration_underline_exposed_via_effective() {
        // char a owns Underline via span; char b has none (non-inheritable, and a
        // paragraph block Underline record no longer reaches carriers).
        // The d-3-2 decoration contract reads effective[Underline] directly.
        let mut l = build_logs(vec![ch('a'), ch('b')]);
        l.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::new(54, 1),
                ModifierAttrOp::SetModifier {
                    target: Dot::new(1, 1),
                    modifier: Modifier::Underline,
                },
            )
            .unwrap();
        l.spans = SpanLog::new()
            .apply(
                Dot::new(55, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(0), Bias::Before),
                    end: anc(leaf(0), Bias::After),
                    modifier: Modifier::Underline,
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let (_t, runs, _tabs) = collect_text_runs(&para);
        assert_eq!(runs.len(), 2);
        assert!(runs[0].effective.get(&ModifierType::Underline).is_some());
        assert!(runs[1].effective.get(&ModifierType::Underline).is_none());
        assert!(runs[0].own_modifiers.contains_key(&ModifierType::Underline));
        assert!(!runs[1].own_modifiers.contains_key(&ModifierType::Underline));
    }

    #[test]
    fn style_wires_effective_into_resolved_text_style() {
        // root block FontSize(1600) + FontWeight(700) -> inherited by the char.
        // style.font_size == 16pt in px (1600/100 * 96/72); style.font_weight == 700.
        let mut l = build_logs(vec![ch('a')]);
        l.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::new(56, 1),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::FontSize { value: 1600 },
                },
            )
            .unwrap()
            .apply(
                Dot::new(7, 1),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::FontWeight { value: 700 },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let (_t, runs, _tabs) = collect_text_runs(&para);
        assert_eq!(runs.len(), 1);
        assert!((runs[0].style.font_size - 16.0 * 96.0 / 72.0).abs() < 0.01);
        assert_eq!(runs[0].style.font_weight, 700);
    }

    #[test]
    fn set_tab_carries_its_own_effective_and_style() {
        // chars a, TAB(own FontSize 2400 via span), b. Tab.effective[FontSize] == 2400;
        // style.font_size == 24pt in px. Proves the tab carries its own resolved font.
        let mut l = build_logs(vec![ch('a'), tab(), ch('b')]);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(60, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(1), Bias::Before), // the Tab is leaf index 1 == Dot(1,3)
                    end: anc(leaf(1), Bias::After),
                    modifier: Modifier::FontSize { value: 2400 },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let (_t, _runs, tabs) = collect_text_runs(&para);
        assert_eq!(tabs.len(), 1);
        assert_eq!(
            tabs[0].effective.get(&ModifierType::FontSize),
            Some(&Modifier::FontSize { value: 2400 })
        );
        assert!((tabs[0].style.font_size - 24.0 * 96.0 / 72.0).abs() < 0.01);
    }

    #[test]
    fn segments_split_at_hard_break_with_both_coords() {
        let pd = project_document(&build_logs(vec![ch('a'), hb(), ch('b')])).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let segs = split_segments(&para);
        assert_eq!(
            segs,
            vec![
                Segment::Text {
                    offset_range: 0..1,
                    byte_range: 0..1
                },
                Segment::Text {
                    offset_range: 2..3,
                    byte_range: 1..2
                },
            ]
        );
    }

    #[test]
    fn segments_trailing_hard_break_yields_trailing_empty() {
        let pd = project_document(&build_logs(vec![ch('a'), hb()])).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let segs = split_segments(&para);
        assert_eq!(
            segs,
            vec![
                Segment::Text {
                    offset_range: 0..1,
                    byte_range: 0..1
                },
                Segment::Empty { offset_range: 2..2 },
            ]
        );
    }

    #[test]
    fn segments_empty_paragraph_is_single_empty() {
        let pd = project_document(&build_logs(vec![])).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let segs = split_segments(&para);
        assert_eq!(segs, vec![Segment::Empty { offset_range: 0..0 }]);
    }

    #[test]
    fn segments_tab_only_is_text_with_empty_byte_range() {
        let pd = project_document(&build_logs(vec![tab()])).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let segs = split_segments(&para);
        assert_eq!(
            segs,
            vec![Segment::Text {
                offset_range: 0..1,
                byte_range: 0..0
            },]
        );
        let (_t, _runs, tabs) = collect_text_runs(&para);
        assert_eq!(tabs.len(), 1);
    }

    #[test]
    fn segments_page_break_only_is_text_with_empty_byte_no_tab() {
        let pd = project_document(&build_logs(vec![pb()])).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let segs = split_segments(&para);
        assert_eq!(
            segs,
            vec![Segment::Text {
                offset_range: 0..1,
                byte_range: 0..0
            },]
        );
        let (_t, _runs, tabs) = collect_text_runs(&para);
        assert!(tabs.is_empty());
    }

    // Helper: chain N ruby AddSpans, each over a single leaf index, onto a fresh SpanLog.
    // Each (leaf_index, text) pair becomes one AddSpan under op-id (5, i).
    fn ruby_spans(pairs: &[(u64, &str)]) -> SpanLog {
        let mut s = SpanLog::new();
        for (i, (li, text)) in pairs.iter().enumerate() {
            s = s
                .apply(
                    Dot::new(5, i as u64),
                    SpanOp::AddSpan {
                        start: anc(leaf(*li), Bias::Before),
                        end: anc(leaf(*li), Bias::After),
                        modifier: Modifier::Ruby {
                            text: text.to_string(),
                        },
                    },
                )
                .unwrap();
        }
        s
    }

    #[test]
    fn ruby_contiguous_same_text_is_one_group() {
        // "ab" both with ruby "x" -> one group spanning offset 0..2, total_base_chars 2.
        let mut l = build_logs(vec![ch('a'), ch('b')]);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(64, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(0), Bias::Before),
                    end: anc(leaf(1), Bias::After),
                    modifier: Modifier::Ruby {
                        text: "x".to_string(),
                    },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let groups = identify_ruby_groups(&para);
        assert_eq!(
            groups,
            vec![RubyGroup {
                text: "x".to_string(),
                offset_range: 0..2,
                total_base_chars: 2,
            }]
        );
    }

    #[test]
    fn ruby_different_text_splits_into_two_groups() {
        let mut l = build_logs(vec![ch('a'), ch('b')]);
        l.spans = ruby_spans(&[(0, "x"), (1, "y")]);
        let pd = project_document(&l).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let groups = identify_ruby_groups(&para);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].text, "x");
        assert_eq!(groups[1].text, "y");
    }

    #[test]
    fn ruby_non_ruby_char_between_breaks_group() {
        // a(ruby x) b(no ruby) c(ruby x) -> two separate groups.
        let mut l = build_logs(vec![ch('a'), ch('b'), ch('c')]);
        l.spans = ruby_spans(&[(0, "x"), (2, "x")]);
        let pd = project_document(&l).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let groups = identify_ruby_groups(&para);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].offset_range, 0..1);
        assert_eq!(groups[1].offset_range, 2..3);
    }

    #[test]
    fn ruby_offset_key_contains_run_offset_range() {
        // The d-3-2 glyph->group key is run.offset_range ⊆ group.offset_range.
        let mut l = build_logs(vec![ch('a'), ch('b')]);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(65, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(0), Bias::Before),
                    end: anc(leaf(1), Bias::After),
                    modifier: Modifier::Ruby {
                        text: "x".to_string(),
                    },
                },
            )
            .unwrap();
        let pd = project_document(&l).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let groups = identify_ruby_groups(&para);
        let (_t, runs, _tabs) = collect_text_runs(&para);
        assert_eq!(groups.len(), 1);
        let g = &groups[0];
        // every run over these base chars is contained in the group's offset span
        for r in &runs {
            assert!(
                g.offset_range.start <= r.offset_range.start
                    && r.offset_range.end <= g.offset_range.end
            );
        }
        assert_eq!(
            g.total_base_chars,
            g.offset_range.end - g.offset_range.start
        );
    }

    #[test]
    fn ruby_groups_survive_hard_break_as_two_groups_in_two_segments() {
        // ruby("x") HardBreak ruby("x"): the HardBreak splits the group; the two groups'
        // offset_ranges sit in segment 1 and segment 2 respectively.
        let mut l = build_logs(vec![ch('a'), hb(), ch('b')]);
        l.spans = ruby_spans(&[(0, "x"), (2, "x")]); // char a (leaf 0), char b (leaf 2; leaf 1 is HardBreak)
        let pd = project_document(&l).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let groups = identify_ruby_groups(&para);
        let segs = split_segments(&para);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].offset_range, 0..1);
        assert_eq!(groups[1].offset_range, 2..3);
        // group 0 sits in segment 0 (0..1), group 1 in segment 1 (2..3)
        assert_eq!(
            segs[0],
            Segment::Text {
                offset_range: 0..1,
                byte_range: 0..1
            }
        );
        assert_eq!(
            segs[1],
            Segment::Text {
                offset_range: 2..3,
                byte_range: 1..2
            }
        );
    }
}
