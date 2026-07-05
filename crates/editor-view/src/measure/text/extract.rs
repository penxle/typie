#[derive(Debug, Clone, Copy)]
pub struct LineHeightConfig {
    pub line_height_ratio: f32,
    pub base_font_size: f32,
}

use std::collections::BTreeMap;
use std::ops::Range;

use editor_common::StrExt;
use editor_model::{Modifier, ModifierType, OwnModifier};
use icu_segmenter::GraphemeClusterSegmenter;
use parley::Layout;

use editor_resource::TextBrush;

use super::inline::{TabMark, TextRun};
use super::strut::StrutMetrics;
use super::style_run::StyleRun;
use crate::glyph_run::GlyphRun;
use crate::glyph_run::{Glyph, GraphemeSpan, Synthesis, TextDecoration};

const LINK_COLOR: &str = "text.blue";
const ITALIC_SKEW_DEGREES: f32 = 14.0;

pub(crate) struct ExtractedLine {
    pub height: f32,
    pub baseline: f32,
    pub ascent: f32,
    pub descent: f32,
    pub glyph_runs: Vec<GlyphRun>,
    pub tab_gaps_raw: Vec<(u64, f32, f32)>,
    pub is_phantom: bool,
    pub content_edge_x: Option<f32>,
}

#[derive(Debug, Clone, Copy)]
struct LineTypographyMetrics {
    ascent: f32,
    descent: f32,
    max_run_font_size: f32,
}

fn segment_graphemes(
    cluster_text: &str,
    cluster_advance: f32,
    segmenter: &GraphemeClusterSegmenter,
) -> Vec<GraphemeSpan> {
    let boundaries: Vec<usize> = segmenter
        .as_borrowed()
        .segment_str(cluster_text)
        .filter(|&b| b > 0)
        .collect();
    let count = boundaries.len();
    if count == 0 {
        return vec![];
    }
    let per_grapheme = cluster_advance / count as f32;
    let mut spans = Vec::with_capacity(count);
    let mut prev = 0;
    for b in boundaries {
        let grapheme = &cluster_text[prev..b];
        spans.push(GraphemeSpan {
            advance: per_grapheme,
            codepoints: grapheme.char_count() as u8,
        });
        prev = b;
    }
    spans
}

fn max_run_font_size(line: &parley::layout::Line<TextBrush>, base_font_size: f32) -> f32 {
    line.items()
        .filter_map(|item| match item {
            parley::PositionedLayoutItem::GlyphRun(gr) => Some(gr.run().font_size()),
            _ => None,
        })
        .fold(base_font_size, f32::max)
}

fn resolve_line_typography_metrics(
    metrics: parley::layout::LineMetrics,
    strut: &StrutMetrics,
    base_font_size: f32,
    max_run_font_size: f32,
) -> LineTypographyMetrics {
    let safe_base_font_size = base_font_size.max(1.0);
    let line_font_size = max_run_font_size.max(safe_base_font_size);

    let mut ascent = (strut.ascent.max(0.0) / safe_base_font_size) * line_font_size;
    let mut descent = (strut.descent.max(0.0) / safe_base_font_size) * line_font_size;

    if ascent <= 0.0 && descent <= 0.0 {
        ascent = strut.ascent.max(0.0);
        descent = strut.descent.max(0.0);
    }
    if (!ascent.is_finite() || ascent <= 0.0) && metrics.ascent > 0.0 {
        ascent = metrics.ascent;
    }
    if (!descent.is_finite() || descent <= 0.0) && metrics.descent > 0.0 {
        descent = metrics.descent;
    }

    LineTypographyMetrics {
        ascent,
        descent,
        max_run_font_size,
    }
}

pub(crate) fn extract_lines(
    text: &str,
    layout: &Layout<TextBrush>,
    style_runs: &[StyleRun],
    runs: &[TextRun],
    strut: &StrutMetrics,
    height_config: LineHeightConfig,
    grapheme_segmenter: &GraphemeClusterSegmenter,
    tab_boxes: &[(TabMark, f32)],
) -> Vec<ExtractedLine> {
    let LineHeightConfig {
        line_height_ratio,
        base_font_size,
    } = height_config;
    let mut lines = Vec::new();
    let content_width = layout.layout_max_advance();

    for line in layout.lines() {
        let metrics = line.metrics();
        let hung_clamp_x = (metrics.trailing_whitespace > 0.0
            && metrics.offset + metrics.advance > content_width)
            .then_some(content_width);
        let typography = resolve_line_typography_metrics(
            *metrics,
            strut,
            base_font_size,
            max_run_font_size(&line, base_font_size),
        );
        let ascent = typography.ascent;
        let descent = typography.descent;
        let content_height = (ascent + descent).max(0.0);
        let line_box_height =
            (typography.max_run_font_size * line_height_ratio).max(content_height);
        let leading = (line_box_height - content_height).max(0.0);
        let baseline = leading / 2.0 + ascent;

        let mut glyph_runs = Vec::new();
        let mut tab_gaps_raw = Vec::new();
        let mut shift = 0.0_f32;
        let line_origin = metrics.offset;

        for item in line.items() {
            let glyph_run = match item {
                parley::PositionedLayoutItem::GlyphRun(glyph_run) => glyph_run,
                parley::PositionedLayoutItem::InlineBox(b) => {
                    let (_, tab_px) = &tab_boxes[b.id as usize];
                    let cur_x = b.x + shift;
                    const TAB_EPS: f32 = 0.01;
                    let rem = (cur_x - line_origin).rem_euclid(*tab_px);
                    let pad = if rem < TAB_EPS || rem > tab_px - TAB_EPS {
                        *tab_px
                    } else {
                        tab_px - rem
                    };
                    tab_gaps_raw.push((b.id, cur_x, pad));
                    shift += pad - tab_px;
                    continue;
                }
            };

            {
                let run = glyph_run.run();
                let font_size = run.font_size();
                let safe_base = height_config.base_font_size.max(1.0);
                let run_cursor_ascent = (strut.ascent.max(0.0) / safe_base) * font_size;
                let run_cursor_descent = (strut.descent.max(0.0) / safe_base) * font_size;

                let run_x = glyph_run.offset();
                let mut glyph_x_advance = 0.0;
                let glyphs: Vec<Glyph> = glyph_run
                    .glyphs()
                    .map(|g| {
                        let gx = glyph_x_advance + g.x;
                        glyph_x_advance += g.advance;
                        Glyph {
                            id: g.id,
                            x: run_x + gx + shift,
                            y: baseline + g.y,
                        }
                    })
                    .collect();

                let target_style_index = glyph_run.glyphs().next().map(|g| g.style_index());

                let mut run_text = String::new();
                let mut graphemes = Vec::new();
                let mut first_byte_start = None;

                for cluster in run.visual_clusters() {
                    let cluster_style_index = cluster.glyphs().next().map(|g| g.style_index());
                    if cluster_style_index != target_style_index {
                        continue;
                    }

                    let cluster_range = cluster.text_range();
                    let cluster_text = &text[cluster_range.clone()];
                    let advance = cluster.advance();

                    if first_byte_start.is_none() {
                        first_byte_start = Some(cluster_range.start);
                    }

                    run_text.push_str(cluster_text);
                    graphemes.extend(segment_graphemes(cluster_text, advance, grapheme_segmenter));
                }

                let byte_start = first_byte_start.unwrap_or(0);
                let run_index = glyph_run.style().brush.run_index;
                let src = &runs[run_index];
                let char_offset = text[src.byte_range.start..byte_start].char_count();
                let run_char_count = run_text.chars().count();
                let offset_range: Range<usize> = (src.offset_range.start + char_offset)
                    ..(src.offset_range.start + char_offset + run_char_count);

                let synthesis = resolve_synthesis(src.effective);
                let decoration = resolve_decoration(src.own_modifiers, src.effective);
                let (color, background_color) = resolve_colors(src.own_modifiers, src.effective);
                let link = resolve_link(src.own_modifiers);

                let (family_id, weight) = style_runs
                    .iter()
                    .find(|sr| sr.byte_range.contains(&byte_start))
                    .map(|sr| (sr.family, sr.weight))
                    .unwrap_or((0, 400));

                let run_advance = glyph_run.advance();

                glyph_runs.push(GlyphRun {
                    family_id,
                    weight,
                    font_size,
                    synthesis,
                    color,
                    background_color,
                    glyphs,
                    decoration,
                    offset_range,
                    link,
                    text: run_text,
                    x: run_x + shift,
                    width: run_advance,
                    graphemes,
                    cursor_ascent: run_cursor_ascent,
                    cursor_descent: run_cursor_descent,
                });
            }
        }

        lines.push(ExtractedLine {
            height: line_box_height,
            baseline,
            ascent,
            descent,
            tab_gaps_raw,
            glyph_runs,
            is_phantom: false,
            content_edge_x: hung_clamp_x,
        });
    }

    if lines.len() > 1
        && lines
            .last()
            .is_some_and(|l| l.glyph_runs.is_empty() && l.tab_gaps_raw.is_empty())
    {
        let last = lines.last_mut().unwrap();
        last.height = 0.0;
        last.ascent = 0.0;
        last.descent = 0.0;
        last.baseline = 0.0;
        last.is_phantom = true;
        last.content_edge_x = None;
    } else if lines.last().is_some_and(|l| l.content_edge_x.is_some()) {
        lines.push(ExtractedLine {
            height: 0.0,
            baseline: 0.0,
            ascent: 0.0,
            descent: 0.0,
            glyph_runs: vec![],
            tab_gaps_raw: vec![],
            is_phantom: true,
            content_edge_x: None,
        });
    }

    lines
}

pub(crate) fn own_values(
    own: &BTreeMap<ModifierType, OwnModifier>,
    ty: ModifierType,
) -> Option<&Modifier> {
    own.get(&ty).map(|o| &o.value)
}

pub(crate) fn resolve_synthesis(eff: &BTreeMap<ModifierType, Modifier>) -> Synthesis {
    Synthesis {
        embolden: eff.contains_key(&ModifierType::Bold),
        skew: eff
            .contains_key(&ModifierType::Italic)
            .then_some(ITALIC_SKEW_DEGREES),
    }
}

pub(crate) fn resolve_decoration(
    own: &BTreeMap<ModifierType, OwnModifier>,
    eff: &BTreeMap<ModifierType, Modifier>,
) -> TextDecoration {
    TextDecoration {
        underline: own_values(own, ModifierType::Link).is_some()
            || eff.contains_key(&ModifierType::Underline),
        strikethrough: eff.contains_key(&ModifierType::Strikethrough),
    }
}

pub(crate) fn resolve_colors(
    own: &BTreeMap<ModifierType, OwnModifier>,
    eff: &BTreeMap<ModifierType, Modifier>,
) -> (String, Option<String>) {
    let color = if let Some(Modifier::TextColor { value }) =
        own.get(&ModifierType::TextColor).map(|o| &o.value)
    {
        format!("text.{value}")
    } else if own_values(own, ModifierType::Link).is_some() {
        LINK_COLOR.to_string()
    } else if let Some(Modifier::TextColor { value }) = eff.get(&ModifierType::TextColor) {
        format!("text.{value}")
    } else {
        "text.black".to_string()
    };
    let background = match eff.get(&ModifierType::BackgroundColor) {
        Some(Modifier::BackgroundColor { value }) => Some(format!("bg.{value}")),
        _ => None,
    };
    (color, background)
}

pub(crate) fn resolve_link(own: &BTreeMap<ModifierType, OwnModifier>) -> Option<String> {
    match own_values(own, ModifierType::Link) {
        Some(Modifier::Link { href }) => Some(href.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        Anchor, AtomLeaf, Bias, DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeMarkerLog,
        NodeType, SeqItem, SpanLog, SpanOp, project_document,
    };
    use editor_resource::Resource;

    use super::super::inline::collect_text_runs;
    use super::super::layout::build_layout;
    use super::super::resolve::ResolvedTextStyle;
    use super::super::strut::compute_strut;
    use super::super::style_run::resolve_style_runs;
    use super::super::tab_metric::tab_px;
    use editor_model::Alignment;

    use super::*;

    fn own(pairs: Vec<(ModifierType, Modifier)>) -> BTreeMap<ModifierType, OwnModifier> {
        pairs
            .into_iter()
            .map(|(t, m)| (t, OwnModifier { value: m }))
            .collect()
    }
    fn eff(pairs: Vec<(ModifierType, Modifier)>) -> BTreeMap<ModifierType, Modifier> {
        pairs.into_iter().collect()
    }
    fn tc(v: &str) -> Modifier {
        Modifier::TextColor {
            value: v.to_string(),
        }
    }
    fn bg(v: &str) -> Modifier {
        Modifier::BackgroundColor {
            value: v.to_string(),
        }
    }
    fn link(h: &str) -> Modifier {
        Modifier::Link {
            href: h.to_string(),
        }
    }

    #[test]
    fn color_own_text_color_wins() {
        let o = own(vec![(ModifierType::TextColor, tc("red"))]);
        let e = eff(vec![(ModifierType::TextColor, tc("red"))]);
        assert_eq!(resolve_colors(&o, &e).0, "text.red");
    }

    #[test]
    fn color_own_text_color_beats_own_link() {
        // own TextColor AND own no-style Link both present → TextColor wins (branch 1 before
        // branch 2). The link is still resolved separately (resolve_link), and decoration
        // still underlines. Guards against a reordered color cascade.
        let o = own(vec![
            (ModifierType::TextColor, tc("red")),
            (ModifierType::Link, link("h")),
        ]);
        let e = eff(vec![
            (ModifierType::TextColor, tc("red")),
            (ModifierType::Link, link("h")),
        ]);
        assert_eq!(resolve_colors(&o, &e).0, "text.red"); // NOT LINK_COLOR
        assert_eq!(resolve_link(&o), Some("h".to_string())); // link still resolved
        assert!(resolve_decoration(&o, &e).underline); // and still underlines
    }

    #[test]
    fn color_own_values_link_uses_link_color() {
        let o = own(vec![(ModifierType::Link, link("h"))]);
        let e = eff(vec![(ModifierType::TextColor, tc("blue"))]); // inherited, but link beats it
        assert_eq!(resolve_colors(&o, &e).0, LINK_COLOR);
    }

    #[test]
    fn color_inherited_effective_text_color() {
        let o = own(vec![]);
        let e = eff(vec![(ModifierType::TextColor, tc("green"))]);
        assert_eq!(resolve_colors(&o, &e).0, "text.green");
    }

    #[test]
    fn color_default_black() {
        assert_eq!(resolve_colors(&own(vec![]), &eff(vec![])).0, "text.black");
    }

    #[test]
    fn background_effective_none_suppress_prefix() {
        assert_eq!(
            resolve_colors(
                &own(vec![]),
                &eff(vec![(ModifierType::BackgroundColor, bg("yellow"))])
            )
            .1,
            Some("bg.yellow".to_string())
        );
        assert_eq!(resolve_colors(&own(vec![]), &eff(vec![])).1, None);
    }

    #[test]
    fn decoration_underline_via_link_or_effective() {
        // own-no-style Link → underline
        let d = resolve_decoration(&own(vec![(ModifierType::Link, link("h"))]), &eff(vec![]));
        assert!(d.underline);
        // effective Underline → underline
        let d = resolve_decoration(
            &own(vec![]),
            &eff(vec![(ModifierType::Underline, Modifier::Underline)]),
        );
        assert!(d.underline);
        // neither → no underline
        assert!(!resolve_decoration(&own(vec![]), &eff(vec![])).underline);
    }

    #[test]
    fn decoration_strikethrough_from_effective() {
        let d = resolve_decoration(
            &own(vec![]),
            &eff(vec![(ModifierType::Strikethrough, Modifier::Strikethrough)]),
        );
        assert!(d.strikethrough);
        assert!(!resolve_decoration(&own(vec![]), &eff(vec![])).strikethrough);
    }

    #[test]
    fn synthesis_bold_italic_from_effective() {
        let s = resolve_synthesis(&eff(vec![(ModifierType::Bold, Modifier::Bold)]));
        assert!(s.embolden && s.skew.is_none());
        let s = resolve_synthesis(&eff(vec![(ModifierType::Italic, Modifier::Italic)]));
        assert!(!s.embolden && s.skew == Some(ITALIC_SKEW_DEGREES));
        let s = resolve_synthesis(&eff(vec![
            (ModifierType::Bold, Modifier::Bold),
            (ModifierType::Italic, Modifier::Italic),
        ]));
        assert!(s.embolden && s.skew == Some(ITALIC_SKEW_DEGREES));
        let s = resolve_synthesis(&eff(vec![]));
        assert!(!s.embolden && s.skew.is_none());
    }

    #[test]
    fn link_own_href() {
        assert_eq!(
            resolve_link(&own(vec![(ModifierType::Link, link("u"))])),
            Some("u".to_string())
        );
        assert_eq!(resolve_link(&own(vec![])), None);
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
            node_markers: NodeMarkerLog::new(),
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
    fn leaf(i: u64) -> Dot {
        Dot::new(1, 2 + i)
    }
    fn anc(d: Dot, b: Bias) -> Anchor {
        Anchor { id: d, bias: b }
    }

    fn pipeline_extract(l: &DocLogs) -> Vec<ExtractedLine> {
        let pd = project_document(l).unwrap();
        let view = DocView::new(&pd);
        let para = view.root().unwrap().child_blocks().next().unwrap();
        let (text, runs, tabs) = collect_text_runs(&para);
        let mut resource = Resource::new_test();
        let base = ResolvedTextStyle {
            font_family: String::new(),
            font_weight: 400,
            font_size: 16.0,
            letter_spacing: 0.0,
            line_height: 1.6,
        };
        let strut = compute_strut(&mut resource, &base).expect("strut");
        let style_runs = resolve_style_runs(&text, &runs, &mut resource.font_registry);
        let tab_boxes: Vec<(TabMark, f32)> = tabs
            .into_iter()
            .map(|t| {
                let px = tab_px(&t.style, &mut resource);
                (t, px)
            })
            .collect();
        let layout = build_layout(
            &text,
            &style_runs,
            Alignment::Left,
            0.0,
            1.0e6,
            &mut resource,
            &tab_boxes,
        );
        let segmenters = Arc::clone(&resource.segmenters);
        drop(resource);
        extract_lines(
            &text,
            &layout,
            &style_runs,
            &runs,
            &strut,
            LineHeightConfig {
                line_height_ratio: base.line_height,
                base_font_size: base.font_size,
            },
            &segmenters.grapheme,
            &tab_boxes,
        )
    }

    fn glyph_runs(lines: &[ExtractedLine]) -> Vec<&GlyphRun> {
        lines.iter().flat_map(|l| l.glyph_runs.iter()).collect()
    }

    #[test]
    fn render_fields_from_effective() {
        let mut l = build_logs(vec![ch('a')]);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(50, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(0), Bias::Before),
                    end: anc(leaf(0), Bias::After),
                    modifier: editor_model::Modifier::TextColor {
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
                    modifier: editor_model::Modifier::Bold,
                },
            )
            .unwrap();
        let lines = pipeline_extract(&l);
        let grs = glyph_runs(&lines);
        assert!(!grs.is_empty());
        assert!(
            grs.iter()
                .any(|g| g.color == "text.red" && g.synthesis.embolden)
        );
    }

    #[test]
    fn offset_range_is_paragraph_absolute_with_leading_atom() {
        let l = build_logs(vec![
            SeqItem::Atom(AtomLeaf::Tab),
            ch('a'),
            ch('b'),
            ch('c'),
        ]);
        let lines = pipeline_extract(&l);
        let grs = glyph_runs(&lines);
        assert!(!grs.is_empty(), "expected at least one text glyph run");
        assert!(grs.iter().all(|g| g.offset_range.start >= 1));
        assert!(grs.iter().any(|g| g.offset_range == (1..4)));
    }

    #[test]
    fn link_carried_and_link_color() {
        let mut l = build_logs(vec![ch('a')]);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(52, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(0), Bias::Before),
                    end: anc(leaf(0), Bias::After),
                    modifier: editor_model::Modifier::Link {
                        href: "https://x".to_string(),
                    },
                },
            )
            .unwrap();
        let lines = pipeline_extract(&l);
        let grs = glyph_runs(&lines);
        assert!(
            grs.iter()
                .any(|g| g.link.as_deref() == Some("https://x") && g.color == LINK_COLOR)
        );
    }

    #[test]
    fn two_runs_distinct_render_by_offset_range() {
        let mut l = build_logs(vec![ch('a'), ch('b')]);
        l.spans = SpanLog::new()
            .apply(
                Dot::new(53, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(0), Bias::Before),
                    end: anc(leaf(0), Bias::After),
                    modifier: editor_model::Modifier::TextColor {
                        value: "red".to_string(),
                    },
                },
            )
            .unwrap()
            .apply(
                Dot::new(54, 1),
                SpanOp::AddSpan {
                    start: anc(leaf(1), Bias::Before),
                    end: anc(leaf(1), Bias::After),
                    modifier: editor_model::Modifier::TextColor {
                        value: "blue".to_string(),
                    },
                },
            )
            .unwrap();
        let lines = pipeline_extract(&l);
        let grs = glyph_runs(&lines);
        let color_at = |off: usize| -> Option<&str> {
            grs.iter()
                .find(|g| g.offset_range.contains(&off))
                .map(|g| g.color.as_str())
        };
        assert_eq!(color_at(0), Some("text.red"));
        assert_eq!(color_at(1), Some("text.blue"));
    }
}
