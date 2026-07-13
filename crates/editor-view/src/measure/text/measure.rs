pub(crate) fn empty_caret_x_for(align: Alignment, indent: f32, width: f32) -> f32 {
    match align {
        Alignment::Left | Alignment::Justify => indent,
        Alignment::Center => width / 2.0,
        Alignment::Right => width,
    }
}

use std::ops::Range;
use std::sync::Arc;

use editor_crdt::Dot;
use editor_model::{Alignment, ChildView, Modifier, NodeView};
use editor_resource::Resource;

use crate::glyph_run::RubyAnnotation;

use super::extract::LineHeightConfig;
use super::extract::{ExtractedLine, extract_lines, resolve_link};
use super::inline::{
    RubyGroup, Segment, TabMark, TextRun, collect_text_runs, identify_ruby_groups, split_segments,
};
use super::layout::build_layout;
use super::resolve::{ResolvedTextStyle, apply_pending_to_style, style_from_effective_modifiers};
use super::ruby::build_ruby_annotations;
use super::ruby::ruby_extra_top;
use super::seg_cache::{self, SegmentCache};
use super::strut::compute_strut;
use super::style_run::resolve_style_runs;
use super::tab_metric::tab_px;
use crate::glyph_run::GlyphRun;

#[derive(Debug, Clone, PartialEq)]
pub struct TabGap {
    pub offset_index: usize,
    pub x: f32,
    pub width: f32,
    pub link: Option<String>,
}

/// The new (eg-walker) measured-line output. Mirrors `MeasuredLine` with the
/// identity in the projected coordinate: `node: Dot`, `offset_range`,
/// `Vec<GlyphRun>`, `Vec<TabGap>`. Carries its own `height` (the old
/// height lived on the wrapper `MeasuredNode`; d-3-3 wraps these later).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MeasuredLine {
    pub node: Dot,
    pub height: f32,
    pub baseline: f32,
    pub ascent: f32,
    pub descent: f32,
    pub cursor_ascent: f32,
    pub cursor_descent: f32,
    pub glyph_runs: Vec<GlyphRun>,
    pub ruby_annotations: Vec<RubyAnnotation>,
    pub empty_caret_x: f32,
    pub offset_range: Option<Range<usize>>,
    pub tab_gaps: Vec<TabGap>,
    pub is_phantom: bool,
    pub content_edge_x: Option<f32>,
}

pub(crate) fn build_strut_only_line(
    node: Dot,
    base_style: &ResolvedTextStyle,
    width: f32,
    align: Alignment,
    indent: f32,
    offset_range: Range<usize>,
    resource: &mut Resource,
) -> MeasuredLine {
    let strut =
        compute_strut(resource, base_style).expect("strut layout should have one line and run");
    let ascent = strut.ascent;
    let descent = strut.descent;
    let content_height = ascent + descent;
    let line_box_height = (base_style.font_size * base_style.line_height).max(content_height);
    let leading = (line_box_height - content_height).max(0.0);
    let baseline = leading / 2.0 + ascent;
    MeasuredLine {
        node,
        height: line_box_height,
        baseline,
        ascent,
        descent,
        cursor_ascent: strut.ascent,
        cursor_descent: strut.descent,
        glyph_runs: vec![],
        ruby_annotations: vec![],
        empty_caret_x: empty_caret_x_for(align, indent, width),
        offset_range: Some(offset_range),
        tab_gaps: vec![],
        is_phantom: false,
        content_edge_x: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn measure_segment<'a>(
    node: &NodeView<'a>,
    seg: &Segment,
    text: &str,
    runs: &[TextRun<'a>],
    tabs: &[TabMark<'a>],
    width: f32,
    align: Alignment,
    indent: f32,
    base_style: &ResolvedTextStyle,
    ruby_groups: &[RubyGroup],
    resource: &mut Resource,
) -> Vec<MeasuredLine> {
    let para = node.id();
    let (seg_off, seg_byte) = match seg {
        Segment::Empty { offset_range } => {
            return vec![build_strut_only_line(
                para,
                base_style,
                width,
                align,
                indent,
                offset_range.clone(),
                resource,
            )];
        }
        Segment::Text {
            offset_range,
            byte_range,
        } => (offset_range, byte_range),
    };

    let seg_text = &text[seg_byte.clone()];
    let seg_runs: Vec<TextRun<'a>> = runs
        .iter()
        .filter(|r| seg_off.start <= r.offset_range.start && r.offset_range.end <= seg_off.end)
        .map(|r| TextRun {
            byte_range: (r.byte_range.start - seg_byte.start)..(r.byte_range.end - seg_byte.start),
            offset_range: r.offset_range.clone(),
            own_modifiers: r.own_modifiers,
            effective: r.effective,
            style: r.style.clone(),
        })
        .collect();
    let seg_tabs: Vec<TabMark<'a>> = tabs
        .iter()
        .filter(|t| seg_off.start <= t.offset_index && t.offset_index < seg_off.end)
        .map(|t| TabMark {
            offset_index: t.offset_index,
            byte_offset: t.byte_offset - seg_byte.start,
            own_modifiers: t.own_modifiers,
            effective: t.effective,
            style: t.style.clone(),
        })
        .collect();

    if seg_text.is_empty() && seg_tabs.is_empty() {
        return vec![build_strut_only_line(
            para,
            base_style,
            width,
            align,
            indent,
            seg_off.clone(),
            resource,
        )];
    }

    let strut =
        compute_strut(resource, base_style).expect("strut layout should have one line and run");
    let style_runs = resolve_style_runs(seg_text, &seg_runs, &mut resource.font_registry);
    let tab_boxes: Vec<(TabMark<'a>, f32)> = seg_tabs
        .into_iter()
        .map(|t| {
            let px = tab_px(&t.style, resource);
            (t, px)
        })
        .collect();
    let layout = build_layout(
        seg_text,
        &style_runs,
        align,
        indent,
        width,
        resource,
        &tab_boxes,
    );
    let segmenters = Arc::clone(&resource.segmenters);
    let lines = extract_lines(
        seg_text,
        &layout,
        &style_runs,
        &seg_runs,
        &strut,
        LineHeightConfig {
            line_height_ratio: base_style.line_height,
            base_font_size: base_style.font_size,
        },
        &segmenters.grapheme,
        &tab_boxes,
    );

    let mut group_offsets = vec![0usize; ruby_groups.len()];
    let mut line_outputs: Vec<(ExtractedLine, Vec<RubyAnnotation>)> =
        Vec::with_capacity(lines.len());
    for line in lines {
        let ruby = build_ruby_annotations(&line, width, ruby_groups, &mut group_offsets, resource);
        line_outputs.push((line, ruby));
    }

    let n = line_outputs.len();
    line_outputs
        .into_iter()
        .enumerate()
        .map(|(i, (line, ruby_annotations))| {
            let extra_top = ruby_extra_top(line.baseline, line.ascent, &ruby_annotations);
            let new_ascent = line.ascent + extra_top;
            let new_baseline = line.baseline + extra_top;
            let new_height = line.height + extra_top;

            let line_offset_range = if n == 1 {
                Some(seg_off.clone())
            } else if i == 0 {
                Some(seg_off.start..seg_off.start)
            } else if i + 1 == n {
                Some(seg_off.end..seg_off.end)
            } else {
                None
            };

            let glyph_runs = line
                .glyph_runs
                .into_iter()
                .map(|mut r| {
                    if extra_top != 0.0 {
                        for g in &mut r.glyphs {
                            g.y += extra_top;
                        }
                    }
                    r
                })
                .collect::<Vec<_>>();

            let ruby_annotations = ruby_annotations
                .into_iter()
                .map(|mut a| {
                    a.baseline_y += extra_top;
                    for run in &mut a.glyph_runs {
                        for g in &mut run.glyphs {
                            g.y += extra_top;
                        }
                    }
                    a
                })
                .collect::<Vec<_>>();

            let tab_gaps: Vec<TabGap> = line
                .tab_gaps_raw
                .iter()
                .map(|(id, x, pad)| TabGap {
                    offset_index: tab_boxes[*id as usize].0.offset_index,
                    x: *x,
                    width: *pad,
                    link: resolve_link(tab_boxes[*id as usize].0.own_modifiers),
                })
                .collect();

            MeasuredLine {
                node: para,
                height: new_height,
                baseline: new_baseline,
                ascent: new_ascent,
                descent: line.descent,
                cursor_ascent: strut.ascent,
                cursor_descent: strut.descent,
                glyph_runs,
                ruby_annotations,
                empty_caret_x: empty_caret_x_for(align, indent, width),
                offset_range: line_offset_range,
                tab_gaps,
                is_phantom: line.is_phantom,
                content_edge_x: line.content_edge_x,
            }
        })
        .collect()
}

pub(crate) fn measure_paragraph(
    node: &NodeView,
    width: f32,
    align: Alignment,
    indent: f32,
    pending: Option<&editor_state::PendingModifiers>,
    mut seg_cache: Option<&mut SegmentCache>,
    resource: &mut Resource,
) -> (Vec<MeasuredLine>, f32) {
    let mut base_style = style_from_effective_modifiers(
        &node
            .effective()
            .values()
            .cloned()
            .collect::<Vec<Modifier>>(),
    );
    let no_char_leaves = node
        .children()
        .all(|c| !matches!(&c, ChildView::Leaf(lv) if lv.as_char().is_some()));
    if no_char_leaves {
        let carry: editor_state::PendingModifiers = node
            .carry_modifiers()
            .into_values()
            .map(|modifier| editor_state::PendingModifier::Set { modifier })
            .collect();
        apply_pending_to_style(&mut base_style, &carry);
        if let Some(m) = pending {
            apply_pending_to_style(&mut base_style, m);
        }
    }
    let (text, runs, tabs) = collect_text_runs(node);
    let segments = split_segments(node);
    // Ruby groups depend only on the paragraph, not the segment — compute once here
    // rather than re-scanning the whole paragraph inside every `measure_segment`
    // (that made a paragraph with `S` hard-break segments `O(S · paragraph)`).
    let ruby_groups = identify_ruby_groups(node);

    let node_id = node.id();
    let mut lines: Vec<MeasuredLine> = Vec::new();
    for (i, seg) in segments.iter().enumerate() {
        let seg_indent = if i == 0 { indent } else { 0.0 };
        // Only text segments are cache-worthy; empty (hard-break) segments just emit a
        // cheap strut line. A text segment's shaped output depends solely on its own
        // content (hard breaks prevent cross-segment reflow), so a content-hash cache
        // lets an edit re-shape only the changed segment.
        if let Segment::Text {
            offset_range,
            byte_range,
        } = seg
        {
            let hash = seg_cache::segment_hash(
                &text[byte_range.clone()],
                offset_range,
                &runs,
                &tabs,
                width,
                align,
                seg_indent,
                &base_style,
            );
            let seg_start = offset_range.start;
            if let Some(cache) = seg_cache.as_deref_mut()
                && let Some(reused) = cache.get(node_id, i, hash, seg_start)
            {
                lines.extend(reused);
                continue;
            }
            let measured = measure_segment(
                node,
                seg,
                &text,
                &runs,
                &tabs,
                width,
                align,
                seg_indent,
                &base_style,
                &ruby_groups,
                resource,
            );
            if let Some(cache) = seg_cache.as_deref_mut() {
                cache.put(node_id, i, hash, &measured, seg_start);
            }
            lines.extend(measured);
        } else {
            lines.extend(measure_segment(
                node,
                seg,
                &text,
                &runs,
                &tabs,
                width,
                align,
                seg_indent,
                &base_style,
                &ruby_groups,
                resource,
            ));
        }
    }
    if let Some(cache) = seg_cache {
        cache.prune(node_id, segments.len());
    }

    let total_height: f32 = lines.iter().map(|l| l.height).sum();
    (lines, total_height)
}

#[cfg(test)]
mod tests {
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, Anchor, AtomLeaf, Bias, DocLogs, DocView, Modifier, ModifierAttrLog,
        ModifierAttrOp, NodeAttrLog, NodeType, SeqItem, SpanLog, SpanOp, project_document,
    };
    use editor_resource::Resource;

    use super::*;

    fn base() -> ResolvedTextStyle {
        ResolvedTextStyle {
            font_family: String::new(),
            font_weight: 400,
            font_size: 16.0,
            letter_spacing: 0.0,
            line_height: 1.6,
        }
    }

    #[test]
    fn strut_only_line_has_height_and_no_glyphs() {
        let mut res = Resource::new_test();
        let node = Dot::new(1, 1);
        let line =
            build_strut_only_line(node, &base(), 100.0, Alignment::Left, 0.0, 0..0, &mut res);
        assert!(line.height > 0.0);
        assert!(line.glyph_runs.is_empty());
        assert!(line.ruby_annotations.is_empty());
        assert_eq!(line.offset_range, Some(0..0));
        assert!(!line.is_phantom);
        assert_eq!(line.node, node);
        assert_eq!(line.empty_caret_x, 0.0);
    }

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
            aliases: AliasLog::new(),
        }
    }

    fn build_logs(children: Vec<SeqItem>) -> DocLogs {
        let root = Dot::ROOT;
        let p = Dot::new(1, 1);
        let mut items = vec![(
            p,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
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
    fn hb() -> SeqItem {
        SeqItem::Atom(AtomLeaf::HardBreak)
    }
    fn tab() -> SeqItem {
        SeqItem::Atom(AtomLeaf::Tab)
    }
    fn leaf(i: u64) -> Dot {
        Dot::new(1, 2 + i)
    }
    fn anc(d: Dot, b: Bias) -> Anchor {
        Anchor { id: d, bias: b }
    }

    fn measure(l: &DocLogs, width: f32) -> (Vec<MeasuredLine>, f32) {
        let pd = project_document(l).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let mut res = Resource::new_test();
        measure_paragraph(&para, width, Alignment::Left, 0.0, None, None, &mut res)
    }

    fn glyph_runs(lines: &[MeasuredLine]) -> Vec<&GlyphRun> {
        lines.iter().flat_map(|l| l.glyph_runs.iter()).collect()
    }

    #[test]
    fn seg_cache_reuse_matches_uncached() {
        use super::seg_cache::SegmentCache;
        let logs = build_logs(vec![
            ch('a'),
            ch('b'),
            hb(),
            ch('c'),
            ch('d'),
            ch('e'),
            hb(),
            ch('f'),
        ]);
        let pd = project_document(&logs).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let mut res = Resource::new_test();

        let uncached =
            measure_paragraph(&para, 200.0, Alignment::Left, 0.0, None, None, &mut res).0;

        let mut cache = SegmentCache::default();
        // First pass populates the cache (all segments measured + stored relative).
        let first = measure_paragraph(
            &para,
            200.0,
            Alignment::Left,
            0.0,
            None,
            Some(&mut cache),
            &mut res,
        )
        .0;
        // Second pass reuses every segment from the cache (rebased back to absolute).
        let reused = measure_paragraph(
            &para,
            200.0,
            Alignment::Left,
            0.0,
            None,
            Some(&mut cache),
            &mut res,
        )
        .0;

        assert_eq!(uncached.len(), reused.len(), "line count");
        for (i, (u, r)) in uncached.iter().zip(&reused).enumerate() {
            assert_eq!(u.offset_range, r.offset_range, "line {i} offset_range");
            assert_eq!(u.glyph_runs.len(), r.glyph_runs.len(), "line {i} run count");
            for (ug, rg) in u.glyph_runs.iter().zip(&r.glyph_runs) {
                assert_eq!(ug.offset_range, rg.offset_range, "line {i} run offset");
                assert_eq!(ug.text, rg.text, "line {i} run text");
                assert!((ug.x - rg.x).abs() < 1e-3, "line {i} run x");
            }
            assert_eq!(
                u.tab_gaps
                    .iter()
                    .map(|t| t.offset_index)
                    .collect::<Vec<_>>(),
                r.tab_gaps
                    .iter()
                    .map(|t| t.offset_index)
                    .collect::<Vec<_>>(),
                "line {i} tab offsets",
            );
            assert!((u.height - r.height).abs() < 1e-3, "line {i} height");
        }
        // Uncached and first-cached pass must also agree.
        assert_eq!(uncached.len(), first.len());
    }

    #[test]
    fn seg_cache_reuse_distinguishes_tab_count_changes() {
        use super::seg_cache::SegmentCache;

        let one_tab = build_logs(vec![tab()]);
        let two_tabs = build_logs(vec![tab(), tab()]);
        let mut cache = SegmentCache::default();
        let mut res = Resource::new_test();

        let one_tab_pd = project_document(&one_tab).unwrap();
        let one_tab_view = DocView::new(&one_tab_pd);
        let one_tab_para = one_tab_view.root().unwrap().child_blocks().next().unwrap();
        measure_paragraph(
            &one_tab_para,
            200.0,
            Alignment::Left,
            0.0,
            None,
            Some(&mut cache),
            &mut res,
        );

        let two_tabs_pd = project_document(&two_tabs).unwrap();
        let two_tabs_view = DocView::new(&two_tabs_pd);
        let two_tabs_para = two_tabs_view.root().unwrap().child_blocks().next().unwrap();
        let uncached = measure_paragraph(
            &two_tabs_para,
            200.0,
            Alignment::Left,
            0.0,
            None,
            None,
            &mut res,
        )
        .0;
        let reused = measure_paragraph(
            &two_tabs_para,
            200.0,
            Alignment::Left,
            0.0,
            None,
            Some(&mut cache),
            &mut res,
        )
        .0;

        let tab_offsets = |lines: &[MeasuredLine]| -> Vec<usize> {
            lines
                .iter()
                .flat_map(|line| line.tab_gaps.iter().map(|gap| gap.offset_index))
                .collect()
        };
        assert_eq!(tab_offsets(&uncached), vec![0, 1]);
        assert_eq!(tab_offsets(&reused), tab_offsets(&uncached));
    }

    #[test]
    fn single_line_text() {
        let (lines, height) = measure(&build_logs(vec![ch('a'), ch('b'), ch('c')]), 1.0e6);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].offset_range, Some(0..3));
        assert!(height > 0.0);
        let grs = glyph_runs(&lines);
        assert!(!grs.is_empty());
        assert!(
            grs.iter()
                .all(|g| g.offset_range.start < g.offset_range.end && g.offset_range.end <= 3)
        );
        assert_eq!(grs.iter().map(|g| g.offset_range.start).min(), Some(0));
        assert_eq!(grs.iter().map(|g| g.offset_range.end).max(), Some(3));
    }

    #[test]
    fn empty_paragraph_is_one_strut_line() {
        let (lines, height) = measure(&build_logs(vec![]), 1.0e6);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].glyph_runs.is_empty());
        assert_eq!(lines[0].offset_range, Some(0..0));
        assert!(height > 0.0);
        assert!(!lines[0].is_phantom);
    }

    #[test]
    fn hard_break_two_segments_offsets_absolute() {
        let (lines, _h) = measure(&build_logs(vec![ch('a'), hb(), ch('b')]), 1.0e6);
        assert!(lines.len() >= 2);
        let grs = glyph_runs(&lines);
        assert!(
            grs.iter().any(|g| g.offset_range.start == 2),
            "the 'b' run must be at absolute offset 2, not byte 0"
        );
        assert!(
            lines
                .iter()
                .any(|l| matches!(&l.offset_range, Some(r) if r.start <= 1))
        );
        assert!(
            lines
                .iter()
                .any(|l| matches!(&l.offset_range, Some(r) if r.end >= 2))
        );
    }

    #[test]
    fn tab_yields_tab_gap_at_absolute_offset() {
        let (lines, _h) = measure(&build_logs(vec![ch('a'), tab(), ch('b')]), 1.0e6);
        let gaps: Vec<&TabGap> = lines.iter().flat_map(|l| l.tab_gaps.iter()).collect();
        assert_eq!(gaps.len(), 1);
        assert_eq!(
            gaps[0].offset_index, 1,
            "the Tab is at paragraph-absolute offset 1"
        );
        assert!(gaps[0].width > 0.0);
    }

    #[test]
    fn soft_wrap_interior_line_owns_no_boundary() {
        let text: Vec<SeqItem> = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".chars().map(ch).collect();
        let (lines, _h) = measure(&build_logs(text), 20.0);
        assert!(lines.len() > 1, "narrow width must wrap");
        assert!(lines.first().unwrap().offset_range.is_some());
        assert!(lines.last().unwrap().offset_range.is_some());
        if lines.len() > 2 {
            assert!(
                lines[1..lines.len() - 1]
                    .iter()
                    .all(|l| l.offset_range.is_none())
            );
        }
    }

    #[test]
    fn ruby_inflates_line_and_carries_annotations() {
        let plain = measure(&build_logs(vec![ch('\u{6F22}'), ch('\u{5B57}')]), 1.0e6).0;
        let mut l = build_logs(vec![ch('\u{6F22}'), ch('\u{5B57}')]);
        l.spans = SpanLog::new()
            .apply(
                Dot::ROOT,
                SpanOp::AddSpan {
                    start: anc(leaf(0), Bias::Before),
                    end: anc(leaf(1), Bias::After),
                    modifier: Modifier::Ruby {
                        text: "かんじ".to_string(),
                    },
                },
            )
            .unwrap();
        let ruby = measure(&l, 1.0e6).0;
        assert!(
            !ruby[0].ruby_annotations.is_empty(),
            "ruby annotations present"
        );
        let d_ascent = ruby[0].ascent - plain[0].ascent;
        let d_baseline = ruby[0].baseline - plain[0].baseline;
        let d_height = ruby[0].height - plain[0].height;
        assert!(
            d_ascent > 0.0,
            "ruby strictly inflates the line (extra_top > 0)"
        );
        assert!(
            (d_ascent - d_baseline).abs() < 1e-3 && (d_ascent - d_height).abs() < 1e-3,
            "extra_top added uniformly to ascent/baseline/height",
        );
    }

    #[test]
    fn render_fields_flow_end_to_end() {
        let mut l = build_logs(vec![ch('a')]);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(50, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(0), Bias::Before),
                    end: anc(leaf(0), Bias::After),
                    modifier: Modifier::TextColor {
                        value: "red".to_string(),
                    },
                },
            )
            .unwrap()
            .apply(
                Dot::new(51, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(0), Bias::Before),
                    end: anc(leaf(0), Bias::After),
                    modifier: Modifier::Bold,
                },
            )
            .unwrap();
        let (lines, _h) = measure(&l, 1.0e6);
        let grs = glyph_runs(&lines);
        assert!(
            grs.iter()
                .any(|g| g.color == "text.red" && g.synthesis.embolden)
        );
    }

    fn measure_with_pending(
        l: &DocLogs,
        width: f32,
        pending: Option<&editor_state::PendingModifiers>,
    ) -> (Vec<MeasuredLine>, f32) {
        let pd = project_document(l).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let mut res = Resource::new_test();
        measure_paragraph(&para, width, Alignment::Left, 0.0, pending, None, &mut res)
    }

    fn font_size_span(spans: SpanLog, op_seq: u64, target: Dot, value: u32) -> SpanLog {
        spans
            .apply(
                Dot::new(70, op_seq),
                SpanOp::AddSpan {
                    start: anc(target, Bias::Before),
                    end: anc(target, Bias::After),
                    modifier: Modifier::FontSize { value },
                },
            )
            .unwrap()
    }

    #[test]
    fn line_height_scales_with_span_font_size() {
        let height_for = |value: u32| {
            let mut l = build_logs(vec![ch('a')]);
            l.spans = font_size_span(SpanLog::new(), 1, leaf(0), value);
            measure(&l, 1.0e6).1
        };
        let h6 = height_for(600);
        let h12 = height_for(1200);
        let h24 = height_for(2400);
        assert!(
            (h6 - h12 / 2.0).abs() < 0.01,
            "6pt line must be half of 12pt (h6={h6}, h12={h12})"
        );
        assert!(
            (h24 - h12 * 2.0).abs() < 0.01,
            "24pt line must be double of 12pt (h24={h24}, h12={h12})"
        );
    }

    #[test]
    fn span_below_document_default_scales() {
        let mut l = build_logs(vec![ch('a')]);
        l.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::new(60, 1),
                ModifierAttrOp::SetModifier {
                    target: Dot::ROOT,
                    modifier: Modifier::FontSize { value: 2400 },
                },
            )
            .unwrap();
        l.spans = font_size_span(SpanLog::new(), 1, leaf(0), 1200);
        let (_, h) = measure(&l, 1.0e6);

        let mut l12 = build_logs(vec![ch('a')]);
        l12.spans = font_size_span(SpanLog::new(), 1, leaf(0), 1200);
        let (_, h12) = measure(&l12, 1.0e6);

        assert!(
            (h - h12).abs() < 0.01,
            "12pt span in a 24pt-default document must match a plain 12pt line (h={h}, h12={h12})"
        );
    }

    #[test]
    fn mixed_size_line_uses_max_run() {
        let mut l = build_logs(vec![ch('a'), ch('b')]);
        l.spans = font_size_span(
            font_size_span(SpanLog::new(), 1, leaf(0), 600),
            2,
            leaf(1),
            1000,
        );
        let (lines, h) = measure(&l, 1.0e6);
        assert_eq!(lines.len(), 1);

        let mut l10 = build_logs(vec![ch('b')]);
        l10.spans = font_size_span(SpanLog::new(), 1, leaf(0), 1000);
        let (_, h10) = measure(&l10, 1.0e6);

        assert!(
            (h - h10).abs() < 0.01,
            "mixed 6pt+10pt line must match a 10pt-only line (h={h}, h10={h10})"
        );

        let (_, h_default) = measure(&build_logs(vec![ch('b')]), 1.0e6);
        assert!(
            h < h_default,
            "10pt frame must be below the document-default frame (h={h}, h_default={h_default})"
        );
    }

    #[test]
    fn trailing_empty_segment_keeps_paragraph_size() {
        let mut l = build_logs(vec![ch('a'), hb()]);
        l.spans = font_size_span(SpanLog::new(), 1, leaf(0), 600);
        let (lines, _) = measure(&l, 1.0e6);
        assert_eq!(lines.len(), 2);

        let (empty_lines, _) = measure(&build_logs(vec![]), 1.0e6);
        assert!(
            (lines[1].height - empty_lines[0].height).abs() < 0.01,
            "empty segment keeps the paragraph-size strut (got {}, want {})",
            lines[1].height,
            empty_lines[0].height
        );
        assert!(
            lines[0].height < lines[1].height,
            "6pt text line must be shorter than the paragraph-size strut line (text={}, strut={})",
            lines[0].height,
            lines[1].height
        );
    }

    #[test]
    fn empty_paragraph_pending_font_size_grows_strut() {
        let l = build_logs(vec![]);
        let (lines_base, h0) = measure_with_pending(&l, 1.0e6, None);
        assert_eq!(lines_base.len(), 1);

        let big: editor_state::PendingModifiers = vec![editor_state::PendingModifier::Set {
            modifier: Modifier::FontSize { value: 9600 },
        }];
        let (lines_big, h1) = measure_with_pending(&l, 1.0e6, Some(&big));
        assert_eq!(lines_big.len(), 1);

        assert!(
            h1 > h0,
            "strut must grow with bigger pending font-size (h0={h0}, h1={h1})"
        );
    }

    fn with_carry(mut l: DocLogs, modifier: Modifier) -> DocLogs {
        l.node_carries = ModifierAttrLog::new()
            .apply(
                Dot::new(50, 1),
                ModifierAttrOp::SetModifier {
                    target: Dot::new(1, 1),
                    modifier,
                },
            )
            .unwrap();
        l
    }

    #[test]
    fn empty_paragraph_carry_font_size_grows_strut() {
        let (_, h0) = measure(&build_logs(vec![]), 1.0e6);

        let l = with_carry(build_logs(vec![]), Modifier::FontSize { value: 9600 });
        let (lines, h1) = measure(&l, 1.0e6);
        assert_eq!(lines.len(), 1);

        assert!(
            h1 > h0,
            "strut must grow with bigger carry font-size (h0={h0}, h1={h1})"
        );
    }

    #[test]
    fn non_empty_paragraph_carry_font_size_gate_unchanged() {
        let (_, h0) = measure(&build_logs(vec![ch('a')]), 1.0e6);

        let l = with_carry(
            build_logs(vec![ch('a')]),
            Modifier::FontSize { value: 9600 },
        );
        let (_, h1) = measure(&l, 1.0e6);

        assert!(
            (h1 - h0).abs() < 0.01,
            "carry must not apply to a paragraph with Char leaves (h0={h0}, h1={h1})"
        );
    }

    #[test]
    fn empty_paragraph_pending_overrides_carry() {
        let l = with_carry(build_logs(vec![]), Modifier::FontSize { value: 9600 });
        let (_, h_carry) = measure(&l, 1.0e6);

        let small: editor_state::PendingModifiers = vec![editor_state::PendingModifier::Set {
            modifier: Modifier::FontSize { value: 1200 },
        }];
        let (_, h_pending) = measure_with_pending(&l, 1.0e6, Some(&small));

        assert!(
            h_pending < h_carry,
            "pending must win over carry (carry={h_carry}, pending={h_pending})"
        );
    }

    #[test]
    fn non_empty_paragraph_pending_font_size_gate_unchanged() {
        let l = build_logs(vec![ch('a')]);
        let (_, h0) = measure_with_pending(&l, 1.0e6, None);

        let big: editor_state::PendingModifiers = vec![editor_state::PendingModifier::Set {
            modifier: Modifier::FontSize { value: 9600 },
        }];
        let (_, h1) = measure_with_pending(&l, 1.0e6, Some(&big));

        assert!(
            (h1 - h0).abs() < 0.01,
            "pending must not apply to a paragraph with Char leaves (h0={h0}, h1={h1})"
        );
    }
}
