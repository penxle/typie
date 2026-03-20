use crate::global::{GLOBALS, TextBrush};
use crate::layout::elements::{BackgroundSegment, LineElement, RubySegment, build_metrics};
use crate::layout::{
    Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy, PositionedNode, StrutMetrics,
    StrutRequest, measure_strut,
};
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule, parse_styles};
use crate::model::{Annotation, Node, PendingStylesDecor, PreeditDecor, Style};
use crate::schema::Expand;
use crate::types::{BoxConstraints, Point, Size};
use crate::utils::{
    LengthUnit, build_char_to_byte_offsets, byte_to_char_offset_with_map, char_to_byte_offset,
    char_to_byte_offset_with_map, convert_length,
};
use macros::Codec;
use parley::style::*;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

fn map_range_with_preedit(
    (start, end): (usize, usize),
    preedit: Option<(usize, usize)>,
    expand: &Expand,
) -> (usize, usize) {
    let Some((preedit_offset, preedit_len)) = preedit else {
        return (start, end);
    };

    if preedit_offset < start {
        (start + preedit_len, end + preedit_len)
    } else if preedit_offset == start {
        match expand {
            Expand::Before | Expand::Both => (start, end + preedit_len),
            Expand::After | Expand::None => (start + preedit_len, end + preedit_len),
        }
    } else if preedit_offset < end {
        (start, end + preedit_len)
    } else if preedit_offset == end {
        match expand {
            Expand::After | Expand::Both => (start, end + preedit_len),
            Expand::Before | Expand::None => (start, end),
        }
    } else {
        (start, end)
    }
}

fn preedit_for_node<'a>(ctx: &'a LayoutContext<'a>) -> Option<&'a PreeditDecor> {
    ctx.decorations
        .preedit
        .as_ref()
        .filter(|preedit| preedit.node_id == ctx.node.node_id())
}

fn pending_styles_for_node<'a>(ctx: &'a LayoutContext<'a>) -> Option<&'a PendingStylesDecor> {
    let ps = &ctx.decorations.pending_styles;
    if ps.node_id == ctx.node.node_id() {
        Some(ps)
    } else {
        None
    }
}

fn collect_mapped_font_runs(
    text: &str,
    start_offset: usize,
    family: &str,
    weight: u16,
    font_mappings: &FxHashMap<(Arc<str>, u16, u32), (Arc<str>, u16)>,
    font_interner: &std::collections::HashMap<String, Arc<str>>,
) -> Vec<(usize, usize, Arc<str>, u16)> {
    let interned_primary = font_interner
        .get(family)
        .cloned()
        .unwrap_or_else(|| Arc::from(family));

    let mut runs = Vec::new();
    let mut current_resolved: Option<(Arc<str>, u16)> = None;
    let mut range_start = start_offset;

    for (i, ch) in text.chars().enumerate() {
        let char_idx = start_offset + i;
        let cp = ch as u32;
        let resolved = font_mappings
            .get(&(interned_primary.clone(), weight, cp))
            .cloned()
            .unwrap_or_else(|| (interned_primary.clone(), weight));

        let is_same = current_resolved
            .as_ref()
            .is_some_and(|prev| prev.0 == resolved.0 && prev.1 == resolved.1);

        if !is_same {
            if let Some((prev_family, prev_weight)) = current_resolved.take() {
                runs.push((range_start, char_idx, prev_family, prev_weight));
            }
            current_resolved = Some(resolved);
            range_start = char_idx;
        }
    }

    if let Some((prev_family, prev_weight)) = current_resolved {
        runs.push((
            range_start,
            start_offset + text.chars().count(),
            prev_family,
            prev_weight,
        ));
    }

    runs
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct StrutFontDefaults {
    family: String,
    weight: u16,
    font_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DeclaredStrutRun {
    start_offset: usize,
    end_offset: usize,
    defaults: StrutFontDefaults,
}

fn resolve_strut_font_defaults(
    ctx: &LayoutContext,
    has_preedit: bool,
    is_text_empty: bool,
    cascade_family: &str,
    cascade_weight: u16,
    cascade_font_size: u32,
) -> StrutFontDefaults {
    let cascade_defaults = StrutFontDefaults {
        family: cascade_family.to_string(),
        weight: cascade_weight,
        font_size: cascade_font_size,
    };

    if has_preedit || is_text_empty {
        return cascade_defaults;
    }

    for child in ctx.node.children() {
        match child.node() {
            Some(Node::HardBreak(_)) => break,
            Some(Node::Text(node)) => {
                for segment in node.text.get_segments() {
                    if segment.text.is_empty() {
                        continue;
                    }

                    let family = segment
                        .styles
                        .iter()
                        .find_map(|style| match style {
                            Style::FontFamily(family) => Some(family.family.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| cascade_defaults.family.clone());
                    let weight = segment
                        .styles
                        .iter()
                        .find_map(|style| match style {
                            Style::FontWeight(weight) => Some(weight.weight),
                            _ => None,
                        })
                        .unwrap_or(cascade_defaults.weight);
                    let font_size = segment
                        .styles
                        .iter()
                        .find_map(|style| match style {
                            Style::FontSize(size) => Some(size.size),
                            _ => None,
                        })
                        .unwrap_or(cascade_defaults.font_size);

                    return StrutFontDefaults {
                        family,
                        weight,
                        font_size,
                    };
                }
            }
            _ => {}
        }
    }

    cascade_defaults
}

fn resolve_declared_segment_strut_defaults(
    styles: &[Style],
    cascade_defaults: &StrutFontDefaults,
) -> StrutFontDefaults {
    let family = styles
        .iter()
        .find_map(|style| match style {
            Style::FontFamily(family) => Some(family.family.clone()),
            _ => None,
        })
        .unwrap_or_else(|| cascade_defaults.family.clone());
    let weight = styles
        .iter()
        .find_map(|style| match style {
            Style::FontWeight(weight) => Some(weight.weight),
            _ => None,
        })
        .unwrap_or(cascade_defaults.weight);
    let font_size = styles
        .iter()
        .find_map(|style| match style {
            Style::FontSize(size) => Some(size.size),
            _ => None,
        })
        .unwrap_or(cascade_defaults.font_size);

    StrutFontDefaults {
        family,
        weight,
        font_size,
    }
}

fn resolve_strut_request<'a>(
    defaults: &'a StrutFontDefaults,
    extra_styles: Option<&[Style]>,
    extra_style_default_font_size: u32,
) -> StrutRequest<'a> {
    let mut request = StrutRequest {
        family: &defaults.family,
        weight: defaults.weight,
        font_size_px: convert_length(
            defaults.font_size as f32 / 100.0,
            LengthUnit::Pt,
            LengthUnit::Px,
        ),
        style: FontStyle::Normal,
    };

    let Some(styles) = extra_styles else {
        return request;
    };

    let effective_font_size = styles
        .iter()
        .find_map(|style| {
            if let Style::FontSize(font_size) = style {
                Some(font_size.size)
            } else {
                None
            }
        })
        .unwrap_or(extra_style_default_font_size);
    request.font_size_px = convert_length(
        effective_font_size as f32 / 100.0,
        LengthUnit::Pt,
        LengthUnit::Px,
    );

    for style in styles {
        match style {
            Style::FontWeight(font_weight) => request.weight = font_weight.weight,
            Style::Italic(_) => request.style = FontStyle::Italic,
            _ => {}
        }
    }

    request
}

fn extract_ruby_segments(ctx: &LayoutContext) -> Vec<RubySegment> {
    let mut ruby_segments = Vec::new();
    let mut offset = 0;

    let preedit = preedit_for_node(ctx);
    let preedit_info = preedit.map(|preedit| (preedit.offset, preedit.text.chars().count()));

    for child in ctx.node.children() {
        if let Some(Node::Text(node)) = child.node() {
            let segments = node.text.get_segments();

            for segment in segments {
                let segment_len = segment.text.chars().count();
                let base_start = offset;
                let base_end = offset + segment_len;

                for annotation in &segment.annotations {
                    if let Annotation::Ruby(ruby_ann) = annotation {
                        let (start, end) = map_range_with_preedit(
                            (base_start, base_end),
                            preedit_info,
                            &Expand::None,
                        );

                        ruby_segments.push(RubySegment {
                            start_offset: start,
                            end_offset: end,
                            ruby_text: ruby_ann.text.clone(),
                        });
                        break;
                    }
                }

                offset += segment_len;
            }
        } else if let Some(Node::HardBreak(_)) = child.node() {
            offset += 1;
        }
    }

    let mut merged: Vec<RubySegment> = Vec::new();
    for segment in ruby_segments {
        if let Some(last) = merged.last_mut() {
            if last.ruby_text == segment.ruby_text && last.end_offset >= segment.start_offset {
                last.end_offset = last.end_offset.max(segment.end_offset);
                continue;
            }
        }
        merged.push(segment);
    }
    merged
}

fn extract_background_segments(ctx: &LayoutContext) -> Vec<BackgroundSegment> {
    let mut background_segments = Vec::new();
    let mut offset = 0;

    let preedit = preedit_for_node(ctx);
    let preedit_info = preedit.map(|preedit| (preedit.offset, preedit.text.chars().count()));

    for child in ctx.node.children() {
        if let Some(Node::Text(node)) = child.node() {
            let segments = node.text.get_segments();

            for segment in segments {
                let segment_len = segment.text.chars().count();
                let base_start = offset;
                let base_end = offset + segment_len;

                for style in &segment.styles {
                    if let Style::BackgroundColor(bg_style) = style {
                        if bg_style.has_color() {
                            let (start, end) = map_range_with_preedit(
                                (base_start, base_end),
                                preedit_info,
                                &Expand::None,
                            );

                            background_segments.push(BackgroundSegment {
                                start_offset: start,
                                end_offset: end,
                                color_key: bg_style.color.clone(),
                            });
                            break;
                        }
                    }
                }

                offset += segment_len;
            }
        } else if let Some(Node::HardBreak(_)) = child.node() {
            offset += 1;
        }
    }

    if let Some(preedit) = preedit {
        let pending = pending_styles_for_node(ctx);
        if let Some(ps) = pending {
            let preedit_start = preedit.offset;
            let preedit_end = preedit_start + preedit.text.chars().count();

            for style in &ps.styles {
                if let Style::BackgroundColor(bg_style) = style {
                    if bg_style.has_color() {
                        background_segments.push(BackgroundSegment {
                            start_offset: preedit_start,
                            end_offset: preedit_end,
                            color_key: bg_style.color.clone(),
                        });
                    }
                }
            }
        }
    }

    background_segments.sort_by_key(|s| s.start_offset);

    let mut merged: Vec<BackgroundSegment> = Vec::new();
    for segment in background_segments {
        if let Some(last) = merged.last_mut() {
            if last.color_key == segment.color_key && last.end_offset >= segment.start_offset {
                last.end_offset = last.end_offset.max(segment.end_offset);
                continue;
            }
        }
        merged.push(segment);
    }
    merged
}

fn apply_style_to_builder(
    builder: &mut parley::RangedBuilder<'_, TextBrush>,
    style: &Style,
    range: std::ops::Range<usize>,
    font_size: u32,
) {
    if range.start >= range.end {
        return;
    }

    match style {
        Style::FontFamily(_) => {} // Handled by mapping-based font resolution
        Style::FontSize(m) => builder.push(
            StyleProperty::FontSize(convert_length(
                m.size as f32 / 100.0,
                LengthUnit::Pt,
                LengthUnit::Px,
            )),
            range,
        ),
        Style::FontWeight(m) => builder.push(
            StyleProperty::FontWeight(FontWeight::new(m.weight as f32)),
            range,
        ),
        Style::LetterSpacing(m) => {
            let font_size_px =
                convert_length(font_size as f32 / 100.0, LengthUnit::Pt, LengthUnit::Px);
            builder.push(
                StyleProperty::LetterSpacing((m.spacing as f32 / 100.0) * font_size_px),
                range,
            )
        }
        Style::Bold(_) => {} // Handled via TextBrush.embolden
        Style::Italic(_) => builder.push(StyleProperty::FontStyle(FontStyle::Italic), range),
        Style::Strikethrough(_) => builder.push(StyleProperty::Strikethrough(true), range),
        Style::Underline(_) => builder.push(StyleProperty::Underline(true), range),
        Style::TextColor(_) => {} // Handled via TextBrush.color
        Style::BackgroundColor(_) => {}
    }
}

fn apply_annotation_to_builder(
    builder: &mut parley::RangedBuilder<'_, TextBrush>,
    annotation: &Annotation,
    range: std::ops::Range<usize>,
) {
    if range.start >= range.end {
        return;
    }

    match annotation {
        Annotation::Link(_) => {
            builder.push(StyleProperty::Underline(true), range.clone());
            builder.push(
                StyleProperty::Brush(TextBrush {
                    color: "ui.text.faint".to_string(),
                    ..Default::default()
                }),
                range,
            );
        }
        Annotation::Ruby(_) => {}
    }
}

fn apply_pending_styles_to_builder(
    builder: &mut parley::RangedBuilder<'_, TextBrush>,
    styles: &[Style],
    range: std::ops::Range<usize>,
    default_font_size: u32,
) {
    if range.start >= range.end {
        return;
    }

    let font_size = styles
        .iter()
        .find_map(|style| {
            if let Style::FontSize(font_size) = style {
                Some(font_size.size)
            } else {
                None
            }
        })
        .unwrap_or(default_font_size);

    let has_embolden = styles.iter().any(|style| matches!(style, Style::Bold(_)));
    for style in styles {
        if has_embolden && let Style::FontWeight(weight_style) = style {
            let target_weight = weight_style.weight.max(700) as f32;
            builder.push(
                StyleProperty::FontWeight(FontWeight::new(target_weight)),
                range.clone(),
            );
            continue;
        }
        apply_style_to_builder(builder, style, range.clone(), font_size);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

impl std::fmt::Display for TextAlign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TextAlign::Left => "left",
            TextAlign::Center => "center",
            TextAlign::Right => "right",
            TextAlign::Justify => "justify",
        };
        f.write_str(s)
    }
}

impl std::str::FromStr for TextAlign {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "center" => TextAlign::Center,
            "right" => TextAlign::Right,
            "justify" => TextAlign::Justify,
            _ => TextAlign::Left,
        })
    }
}

fn default_line_height() -> u32 {
    160
}

const LINE_HEIGHTS: &[u32] = &[80, 100, 120, 140, 160, 180, 200, 220];

fn snap_line_height(v: u32) -> u32 {
    let mut best = LINE_HEIGHTS[0];
    let mut best_dist = u32::MAX;
    for &lh in LINE_HEIGHTS {
        let d = if v >= lh { v - lh } else { lh - v };
        if d < best_dist {
            best_dist = d;
            best = lh;
        }
    }
    best
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Codec)]
#[cfg_attr(feature = "wasm", derive(tsify::Tsify))]
pub struct ParagraphNode {
    #[serde(default)]
    pub align: TextAlign,
    /// × 100 (e.g. 160% → 160)
    #[serde(default = "default_line_height")]
    pub line_height: u32,
}

impl Default for ParagraphNode {
    fn default() -> Self {
        Self {
            align: TextAlign::default(),
            line_height: default_line_height(),
        }
    }
}

impl std::hash::Hash for ParagraphNode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.align.hash(state);
        self.line_height.hash(state);
    }
}

impl ParagraphNode {
    fn build_style(&self) -> String {
        let mut s = Vec::new();
        if self.align != TextAlign::Left {
            s.push(format!("text-align:{}", self.align));
        }
        if self.line_height != 160 {
            s.push(format!("line-height:{}", self.line_height as f32 / 100.0));
        }
        s.join(";")
    }
}

impl NodeHtmlCodec for ParagraphNode {
    fn to_dom(&self) -> Option<DomSpec> {
        let style = self.build_style();
        let spec = if style.is_empty() {
            DomSpec::el("p").hole()
        } else {
            DomSpec::el("p").style(style).hole()
        };
        Some(spec)
    }

    fn parse_rules() -> Vec<NodeParseRule> {
        vec![
            NodeParseRule::simple("p", |elem| {
                let mut n = ParagraphNode::default();
                if let Some(style) = elem.value().attr("style") {
                    let m = parse_styles(style);
                    if let Some(a) = m.get("text-align") {
                        n.align = a.parse().unwrap_or_default();
                    }
                    if let Some(lh) = m.get("line-height") {
                        n.line_height = snap_line_height(
                            (lh.parse::<f32>().unwrap_or(1.6) * 100.0).round() as u32,
                        );
                    }
                }
                Some(Node::Paragraph(n))
            }),
            NodeParseRule::new(
                "div",
                10,
                |elem| {
                    let has_non_whitespace_text = elem.children().any(|child| {
                        matches!(child.value(), scraper::Node::Text(t) if !t.text.trim().is_empty())
                    });
                    let child_elements: Vec<_> = elem
                        .children()
                        .filter_map(|child| scraper::ElementRef::wrap(child))
                        .collect();
                    let all_children_are_block = !child_elements.is_empty()
                        && child_elements.iter().all(|c| {
                            matches!(
                                c.value().name(),
                                "div" | "p" | "blockquote" | "ul" | "ol" | "li" | "br"
                            )
                        });

                    if !has_non_whitespace_text && all_children_are_block {
                        return false;
                    }

                    elem.value().attr("data-page-break").is_none()
                        && elem.value().attr("class") != Some("fold-content")
                },
                |elem| {
                    let mut n = ParagraphNode::default();
                    if let Some(style) = elem.value().attr("style") {
                        let m = parse_styles(style);
                        if let Some(a) = m.get("text-align") {
                            n.align = a.parse().unwrap_or_default();
                        }
                        if let Some(lh) = m.get("line-height") {
                            n.line_height = snap_line_height(
                                (lh.parse::<f32>().unwrap_or(1.6) * 100.0).round() as u32,
                            );
                        }
                    }
                    Some(Node::Paragraph(n))
                },
            ),
        ]
    }
}

impl Layout for ParagraphNode {
    fn layout(&self, ctx: &LayoutContext, constraints: BoxConstraints) -> LayoutNode {
        use crate::tracing::TRACER;
        use opentelemetry::KeyValue;
        use opentelemetry::trace::{Tracer, mark_span_as_active};

        let mut para_span = TRACER.start("layout.paragraph");
        opentelemetry::trace::Span::set_attribute(
            &mut para_span,
            KeyValue::new("node_id", ctx.node.node_id().to_string()),
        );
        let _para_guard = mark_span_as_active(para_span);

        let mut text = ctx
            .node
            .children()
            .filter_map(|child| match child.node()? {
                Node::Text(node) => Some(node.text.to_string()),
                Node::HardBreak(_) => Some("\n".to_string()),
                _ => None,
            })
            .collect::<String>();

        let preedit = preedit_for_node(ctx);
        let preedit_info = preedit.map(|preedit| (preedit.offset, preedit.text.chars().count()));
        if let Some(preedit) = preedit {
            let idx = char_to_byte_offset(&text, preedit.offset);
            text.insert_str(idx, &preedit.text);
        }

        let is_text_empty = text.is_empty();
        if text.is_empty() {
            text = "\u{200B}".to_string();
        }

        let pending_styles = pending_styles_for_node(ctx);
        let char_to_byte = build_char_to_byte_offsets(&text);

        let line_height = self.line_height as f32 / 100.0;

        // Resolve cascade font defaults (style_overrides → cascade_attrs chain → root default_attrs)
        let (cascade_family, cascade_weight, cascade_font_size) = ctx.resolve_cascade_font();

        let layout = GLOBALS.with(|globals| {
            use parley::style::*;
            let globals = globals.borrow();

            let mut lcx = globals.parley_layout_context.borrow_mut();
            let mut fcx = globals.parley_font_context.borrow_mut();

            let mut builder = lcx.ranged_builder(&mut fcx, &text, 1.0, false);

            builder.push_default(StyleProperty::FontFamily(FontFamily::Single(
                FontFamilyName::Named(cascade_family.clone().into()),
            )));
            builder.push_default(StyleProperty::FontWeight(FontWeight::new(
                cascade_weight as f32,
            )));
            builder.push_default(StyleProperty::FontSize(convert_length(
                cascade_font_size as f32 / 100.0,
                LengthUnit::Pt,
                LengthUnit::Px,
            )));
            builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
                line_height,
            )));
            builder.push_default(StyleProperty::FontFeatures(FontFeatures::Source(
                Cow::Owned("\"ss05\" 1, \"cv12\" 1, \"ss18\" 1".to_string()),
            )));

            builder.push_default(StyleProperty::OverflowWrap(OverflowWrap::Anywhere));
            builder.push_default(StyleProperty::WordBreak(WordBreak::BreakAll));

            let font_mappings = globals.font_mappings.borrow();
            let font_interner = globals.font_family_interner.borrow();
            let mut declared_strut_runs = Vec::new();
            let cascade_defaults = StrutFontDefaults {
                family: cascade_family.clone(),
                weight: cascade_weight,
                font_size: cascade_font_size,
            };

            let mut offset = 0;
            for child in ctx.node.children() {
                match child.node() {
                    Some(Node::Text(node)) => {
                        let segments = node.text.get_segments();
                        let mut segment_offset = 0;

                        for segment in segments {
                            let segment_len = segment.text.chars().count();
                            let base_start = offset + segment_offset;
                            let base_end = base_start + segment_len;
                            let segment_defaults = resolve_declared_segment_strut_defaults(
                                &segment.styles,
                                &cascade_defaults,
                            );
                            let segment_font_size = segment_defaults.font_size;

                            if segment_len > 0 {
                                declared_strut_runs.push(DeclaredStrutRun {
                                    start_offset: base_start,
                                    end_offset: base_end,
                                    defaults: segment_defaults.clone(),
                                });
                            }

                            // Build TextBrush from segment styles (color + embolden)
                            let has_embolden =
                                segment.styles.iter().any(|s| matches!(s, Style::Bold(_)));
                            let text_color = segment
                                .styles
                                .iter()
                                .find_map(|s| match s {
                                    Style::TextColor(m) => Some(format!("text.{}", m.color)),
                                    _ => None,
                                })
                                .unwrap_or_default();

                            {
                                let (start, end) = map_range_with_preedit(
                                    (base_start, base_end),
                                    preedit_info,
                                    &Expand::None,
                                );
                                let range = char_to_byte_offset_with_map(&char_to_byte, start)
                                    ..char_to_byte_offset_with_map(&char_to_byte, end);

                                if has_embolden || !text_color.is_empty() {
                                    builder.push(
                                        StyleProperty::Brush(TextBrush {
                                            color: text_color,
                                            embolden: has_embolden,
                                        }),
                                        range,
                                    );
                                }
                            }

                            // Push remaining styles (FontSize, FontWeight, LetterSpacing, etc.)
                            for style in &segment.styles {
                                let (start, end) = map_range_with_preedit(
                                    (base_start, base_end),
                                    preedit_info,
                                    &Expand::None,
                                );
                                let range = char_to_byte_offset_with_map(&char_to_byte, start)
                                    ..char_to_byte_offset_with_map(&char_to_byte, end);

                                apply_style_to_builder(
                                    &mut builder,
                                    style,
                                    range,
                                    segment_font_size,
                                );
                            }

                            // Mapping-based font family resolution (single pass)
                            let segment_family = segment_defaults.family.as_str();
                            let segment_weight = segment_defaults.weight;

                            for (run_start, run_end, resolved_family, resolved_weight) in
                                collect_mapped_font_runs(
                                    &segment.text,
                                    base_start,
                                    segment_family,
                                    segment_weight,
                                    &font_mappings,
                                    &font_interner,
                                )
                            {
                                let (start, end) = map_range_with_preedit(
                                    (run_start, run_end),
                                    preedit_info,
                                    &Expand::None,
                                );
                                let byte_range = char_to_byte_offset_with_map(&char_to_byte, start)
                                    ..char_to_byte_offset_with_map(&char_to_byte, end);
                                builder.push(
                                    StyleProperty::FontFamily(FontFamily::Single(
                                        FontFamilyName::Named(resolved_family.to_string().into()),
                                    )),
                                    byte_range.clone(),
                                );
                                if resolved_weight != segment_weight {
                                    builder.push(
                                        StyleProperty::FontWeight(FontWeight::new(
                                            resolved_weight as f32,
                                        )),
                                        byte_range,
                                    );
                                }
                            }

                            for annotation in &segment.annotations {
                                let (start, end) = map_range_with_preedit(
                                    (base_start, base_end),
                                    preedit_info,
                                    &Expand::None,
                                );
                                let range = char_to_byte_offset_with_map(&char_to_byte, start)
                                    ..char_to_byte_offset_with_map(&char_to_byte, end);

                                apply_annotation_to_builder(&mut builder, annotation, range);
                            }

                            segment_offset += segment_len;
                        }

                        offset += segment_offset;
                    }
                    Some(Node::HardBreak(_)) => {
                        offset += 1;
                    }
                    _ => continue,
                }
            }

            if let (Some(preedit), Some(ps)) = (preedit, pending_styles) {
                let preedit_start = preedit.offset;
                let preedit_end = preedit_start + preedit.text.chars().count();
                let range = char_to_byte_offset_with_map(&char_to_byte, preedit_start)
                    ..char_to_byte_offset_with_map(&char_to_byte, preedit_end);
                apply_pending_styles_to_builder(&mut builder, &ps.styles, range, 1200);

                let preedit_defaults =
                    resolve_declared_segment_strut_defaults(&ps.styles, &cascade_defaults);
                for (run_start, run_end, resolved_family, resolved_weight) in
                    collect_mapped_font_runs(
                        &preedit.text,
                        preedit_start,
                        &preedit_defaults.family,
                        preedit_defaults.weight,
                        &font_mappings,
                        &font_interner,
                    )
                {
                    let byte_range = char_to_byte_offset_with_map(&char_to_byte, run_start)
                        ..char_to_byte_offset_with_map(&char_to_byte, run_end);
                    builder.push(
                        StyleProperty::FontFamily(FontFamily::Single(FontFamilyName::Named(
                            resolved_family.to_string().into(),
                        ))),
                        byte_range.clone(),
                    );
                    if resolved_weight != preedit_defaults.weight {
                        builder.push(
                            StyleProperty::FontWeight(FontWeight::new(resolved_weight as f32)),
                            byte_range,
                        );
                    }
                }
            }

            drop(font_mappings);
            drop(font_interner);

            if is_text_empty && let Some(ps) = pending_styles {
                apply_pending_styles_to_builder(&mut builder, &ps.styles, 0..text.len(), 1200);
            }

            let parent_is_root = ctx
                .node
                .parent()
                .map(|parent| matches!(parent.node(), Some(Node::Root(_))))
                .unwrap_or(false);
            let indent = if parent_is_root {
                (ctx.settings.paragraph_indent as f32 / 100.0 * 16.0).max(0.0)
            } else {
                0.0
            };

            let mut layout = {
                let _s = mark_span_as_active(TRACER.start("layout.paragraph.shape_text"));
                builder.build(&text)
            };

            if matches!(self.align, TextAlign::Left | TextAlign::Justify if indent > 0.0) {
                layout.indent(indent, parley::IndentOptions::default());
            }

            {
                let _s = mark_span_as_active(TRACER.start("layout.paragraph.break_lines"));
                layout.break_all_lines(Some(constraints.max_width));
                layout.align(
                    Some(constraints.max_width),
                    match self.align {
                        TextAlign::Left => parley::Alignment::Left,
                        TextAlign::Center => parley::Alignment::Center,
                        TextAlign::Right => parley::Alignment::Right,
                        TextAlign::Justify => parley::Alignment::Justify,
                    },
                    parley::AlignmentOptions::default(),
                );
            }

            let (default_strut, per_line_struts) = {
                let ps_styles = pending_styles.map(|ps| &ps.styles[..]);
                let strut_defaults = resolve_strut_font_defaults(
                    ctx,
                    preedit.is_some(),
                    is_text_empty,
                    &cascade_family,
                    cascade_weight,
                    cascade_font_size,
                );
                let default_strut = if let Some(styles) =
                    ps_styles.filter(|_| preedit.is_some() || is_text_empty)
                {
                    measure_strut(
                        &mut fcx,
                        resolve_strut_request(
                            &strut_defaults,
                            Some(styles),
                            strut_defaults.font_size,
                        ),
                    )
                } else {
                    measure_strut(&mut fcx, resolve_strut_request(&strut_defaults, None, 0))
                };

                let per_line_struts: Option<Vec<StrutMetrics>> =
                    if preedit.is_some() || is_text_empty {
                        None
                    } else {
                        let mut cache: FxHashMap<StrutFontDefaults, StrutMetrics> =
                            FxHashMap::default();
                        let metrics = layout
                            .lines()
                            .map(|line| {
                                let text_range = line.text_range();
                                let line_start =
                                    byte_to_char_offset_with_map(&char_to_byte, text_range.start);
                                let line_end =
                                    byte_to_char_offset_with_map(&char_to_byte, text_range.end);

                                let Some(run) = declared_strut_runs.iter().find(|run| {
                                    run.start_offset < line_end && run.end_offset > line_start
                                }) else {
                                    return default_strut;
                                };

                                if let Some(metrics) = cache.get(&run.defaults) {
                                    return *metrics;
                                }

                                let metrics = measure_strut(
                                    &mut fcx,
                                    resolve_strut_request(&run.defaults, None, 0),
                                );
                                cache.insert(run.defaults.clone(), metrics);
                                metrics
                            })
                            .collect::<Vec<_>>();
                        Some(metrics)
                    };

                (default_strut, per_line_struts)
            };

            (layout, default_strut, per_line_struts)
        });

        let (layout, default_strut, per_line_struts) = layout;
        let layout = Rc::new(layout);
        let metrics = {
            let _s = mark_span_as_active(TRACER.start("layout.paragraph.build_metrics"));
            build_metrics(
                &layout,
                &text,
                ctx.scale_factor,
                default_strut,
                per_line_struts.as_deref(),
                line_height,
            )
        };

        let ruby_segments = extract_ruby_segments(ctx);
        let background_segments = extract_background_segments(ctx);

        let has_page_break = ctx
            .node
            .children()
            .last()
            .map(|child| matches!(child.node(), Some(Node::PageBreak(_))))
            .unwrap_or(false);

        let mut children = Vec::new();
        let mut y_offset = 0.0;

        let text_rc: Rc<str> = Rc::from(text);
        let preedit = preedit_for_node(ctx).cloned();
        for (line_idx, metric) in metrics.iter().enumerate() {
            let line_ruby_segments: Vec<_> = ruby_segments
                .iter()
                .filter_map(|seg| seg.split(metric.start_offset, metric.end_offset))
                .collect();
            let line_background_segments: Vec<_> = background_segments
                .iter()
                .filter_map(|seg| seg.split(metric.start_offset, metric.end_offset))
                .collect();

            let is_last_line = line_idx == metrics.len() - 1;
            let line_has_page_break = has_page_break && is_last_line;

            let line_element = LineElement::build(
                ctx.node.node_id(),
                Size::new(constraints.max_width, metric.height + metric.leading),
                line_idx,
                layout.clone(),
                metric.clone(),
                preedit.clone(),
                is_text_empty,
                text_rc.clone(),
                line_ruby_segments.clone(),
                line_background_segments.clone(),
                line_has_page_break,
            );

            children.push(PositionedNode {
                position: Point::new(0.0, y_offset),
                node: Rc::new(LayoutNode {
                    size: line_element.size,
                    element: Some(Element::Line(line_element)),
                    children: None,
                    page_break_policy: PageBreakPolicy::Avoid,
                    render_hints: Default::default(),
                    scope_id: None,
                }),
            });

            y_offset += metric.height + metric.leading;
        }

        let content_width = metrics
            .iter()
            .map(|m| m.content_width)
            .fold(0.0f32, |a, b| a.max(b));

        LayoutNode {
            size: Size::new(content_width, y_offset),
            element: None,
            children: Some(children),
            page_break_policy: PageBreakPolicy::Auto,
            render_hints: Default::default(),
            scope_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Element, LayoutCache, LayoutNode};
    use crate::model::{
        Attr, BackgroundColorStyle, Decorations, DefaultAttrs, FontFamilyStyle, FontSizeStyle,
        FontWeightStyle, PendingStylesDecor, PreeditDecor,
    };
    use crate::runtime::ViewStates;
    use crate::types::BoxConstraints;
    use rustc_hash::FxHashMap;
    use std::{cell::RefCell, collections::HashMap, sync::Arc};

    #[test]
    fn mapped_font_runs_include_preedit_codepoints() {
        let mut font_mappings = FxHashMap::default();
        font_mappings.insert(
            (Arc::<str>::from("Pretendard"), 400, '한' as u32),
            (Arc::<str>::from("Paperlogy"), 700),
        );

        let font_interner = HashMap::from([
            ("Pretendard".to_string(), Arc::<str>::from("Pretendard")),
            ("Paperlogy".to_string(), Arc::<str>::from("Paperlogy")),
        ]);

        let runs =
            collect_mapped_font_runs("한글", 3, "Pretendard", 400, &font_mappings, &font_interner);

        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].0, 3);
        assert_eq!(runs[0].1, 4);
        assert_eq!(&*runs[0].2, "Paperlogy");
        assert_eq!(runs[0].3, 700);
        assert_eq!(runs[1].0, 4);
        assert_eq!(runs[1].1, 5);
        assert_eq!(&*runs[1].2, "Pretendard");
        assert_eq!(runs[1].3, 400);
    }

    #[test]
    fn layout_handles_multibyte_style_ranges() {
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [font_size(2400)]) { "ㅁㄴㅇㄹ" }
                }
            }
            selection { (p, 0) }
        };

        let doc = &state.doc;
        let para = doc.node(p).unwrap();
        let settings = doc.settings();
        let default_attrs = DefaultAttrs::default();
        let decorations = Decorations::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let ctx = LayoutContext::new(
            &para,
            &settings,
            &default_attrs,
            &decorations,
            1.0,
            &view_states,
            &cache,
        );
        let constraints = BoxConstraints::new(0.0, 800.0, 0.0, f32::INFINITY);

        if let Some(Node::Paragraph(paragraph)) = para.node() {
            paragraph.layout(&ctx, constraints);
        } else {
            panic!("paragraph node expected");
        }
    }

    #[test]
    fn layout_skips_empty_preedit_range_when_pending_styles_exist() {
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text { "AB" }
                }
            }
            selection { (p, 1) }
        };

        let mut decorations = Decorations::default();
        decorations.preedit = Some(PreeditDecor {
            node_id: p,
            offset: 1,
            text: "".into(),
        });
        decorations.pending_styles = PendingStylesDecor {
            node_id: p,
            styles: vec![Style::FontWeight(FontWeightStyle { weight: 700 })],
        };

        let doc = &state.doc;
        let para = doc.node(p).unwrap();
        let settings = doc.settings();
        let default_attrs = DefaultAttrs::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let ctx = LayoutContext::new(
            &para,
            &settings,
            &default_attrs,
            &decorations,
            1.0,
            &view_states,
            &cache,
        );
        let constraints = BoxConstraints::new(0.0, 800.0, 0.0, f32::INFINITY);

        if let Some(Node::Paragraph(paragraph)) = para.node() {
            paragraph.layout(&ctx, constraints);
        } else {
            panic!("paragraph node expected");
        }
    }

    #[test]
    fn layout_hard_break_leading_line_height_is_stable_with_pending_styles() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    hard_break {}
                    text(styles: [font_size(1800)]) { "ㅁㄴㅇㄹ" }
                }
            }
            selection { (p, 0) }
        };

        let doc = &state.doc;
        let para = doc.node(p).unwrap();
        let settings = doc.settings();
        let default_attrs = DefaultAttrs::default();
        let view_states = ViewStates::default();
        let constraints = BoxConstraints::new(0.0, 800.0, 0.0, f32::INFINITY);

        let layout_with_decorations = |decorations: Decorations| -> LayoutNode {
            let cache = RefCell::new(LayoutCache::new());
            let ctx = LayoutContext::new(
                &para,
                &settings,
                &default_attrs,
                &decorations,
                1.0,
                &view_states,
                &cache,
            );

            if let Some(Node::Paragraph(paragraph)) = para.node() {
                paragraph.layout(&ctx, constraints)
            } else {
                panic!("paragraph node expected");
            }
        };

        let line_box_height = |layout: &LayoutNode, line_idx: usize| -> f32 {
            let line = layout
                .children
                .as_ref()
                .and_then(|children| {
                    children
                        .get(line_idx)
                        .and_then(|child| match child.node.element.as_ref() {
                            Some(Element::Line(line)) => Some(line),
                            _ => None,
                        })
                })
                .expect("line element expected");
            line.metric.height + line.metric.leading
        };

        let baseline_layout = layout_with_decorations(Decorations::default());
        let mut pending_decorations = Decorations::default();
        pending_decorations.pending_styles = PendingStylesDecor {
            node_id: p,
            styles: vec![Style::FontSize(FontSizeStyle { size: 1800 })],
        };
        let pending_layout = layout_with_decorations(pending_decorations);

        let baseline_first_line_height = line_box_height(&baseline_layout, 0);
        let pending_first_line_height = line_box_height(&pending_layout, 0);
        let eps = 0.01;
        assert!(
            (baseline_first_line_height - pending_first_line_height).abs() <= eps,
            "first line before hard-break should not change with pending styles: baseline={}, pending={}",
            baseline_first_line_height,
            pending_first_line_height
        );
    }

    #[test]
    fn strut_defaults_prefer_visible_text_styles_over_hidden_cascade_font() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [font_family("Pretendard")]) { "프리텐다드" }
                }
            }
            selection { (p, 0) }
        };
        let state = transact!(state, |tr| {
            tr.set_cascade_attrs(
                p,
                &Attr::from_styles(&[Style::FontFamily(FontFamilyStyle {
                    family: "Paperlogy".to_string(),
                })]),
            )
            .unwrap();
        });

        let doc = &state.doc;
        let para = doc.node(p).unwrap();
        let settings = doc.settings();
        let default_attrs = doc.default_attrs();
        let decorations = Decorations::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let ctx = LayoutContext::new(
            &para,
            &settings,
            &default_attrs,
            &decorations,
            1.0,
            &view_states,
            &cache,
        );
        let (cascade_family, cascade_weight, cascade_font_size) = ctx.resolve_cascade_font();

        let strut_defaults = resolve_strut_font_defaults(
            &ctx,
            false,
            false,
            &cascade_family,
            cascade_weight,
            cascade_font_size,
        );

        assert_eq!(strut_defaults.family, "Pretendard");
        assert_eq!(strut_defaults.weight, cascade_weight);
        assert_eq!(strut_defaults.font_size, cascade_font_size);
    }

    #[test]
    fn strut_defaults_ignore_later_text_after_leading_hard_break() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    hard_break {}
                    text(styles: [font_family("Pretendard"), font_size(1800)]) { "가나다" }
                }
            }
            selection { (p, 0) }
        };
        let state = transact!(state, |tr| {
            tr.set_cascade_attrs(
                p,
                &Attr::from_styles(&[Style::FontFamily(FontFamilyStyle {
                    family: "Paperlogy".to_string(),
                })]),
            )
            .unwrap();
        });

        let doc = &state.doc;
        let para = doc.node(p).unwrap();
        let settings = doc.settings();
        let default_attrs = doc.default_attrs();
        let decorations = Decorations::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let ctx = LayoutContext::new(
            &para,
            &settings,
            &default_attrs,
            &decorations,
            1.0,
            &view_states,
            &cache,
        );
        let (cascade_family, cascade_weight, cascade_font_size) = ctx.resolve_cascade_font();

        let strut_defaults = resolve_strut_font_defaults(
            &ctx,
            false,
            false,
            &cascade_family,
            cascade_weight,
            cascade_font_size,
        );

        assert_eq!(strut_defaults.family, "Paperlogy");
        assert_eq!(strut_defaults.weight, cascade_weight);
        assert_eq!(strut_defaults.font_size, cascade_font_size);
    }

    #[test]
    fn map_range_with_preedit_accounts_for_insertion_before_range() {
        let mapped = map_range_with_preedit((6, 11), Some((2, 3)), &Expand::After);
        assert_eq!(mapped, (9, 14));
    }

    #[test]
    fn map_range_with_preedit_keeps_start_when_insertion_inside_range() {
        let mapped = map_range_with_preedit((2, 6), Some((3, 2)), &Expand::After);
        assert_eq!(mapped, (2, 8));
    }

    #[test]
    fn map_range_with_preedit_at_start_with_expand_before() {
        let mapped = map_range_with_preedit((3, 5), Some((3, 2)), &Expand::Before);
        assert_eq!(mapped, (3, 7));
    }

    #[test]
    fn map_range_with_preedit_at_start_with_expand_after() {
        let mapped = map_range_with_preedit((3, 5), Some((3, 2)), &Expand::After);
        assert_eq!(mapped, (5, 7));
    }

    #[test]
    fn map_range_with_preedit_at_start_with_expand_both() {
        let mapped = map_range_with_preedit((3, 5), Some((3, 2)), &Expand::Both);
        assert_eq!(mapped, (3, 7));
    }

    #[test]
    fn map_range_with_preedit_at_start_with_expand_none() {
        let mapped = map_range_with_preedit((3, 5), Some((3, 2)), &Expand::None);
        assert_eq!(mapped, (5, 7));
    }

    #[test]
    fn map_range_with_preedit_keeps_range_when_insertion_after() {
        let mapped = map_range_with_preedit((0, 3), Some((5, 2)), &Expand::After);
        assert_eq!(mapped, (0, 3));
    }

    #[test]
    fn map_range_with_preedit_at_end_with_expand_after() {
        let mapped = map_range_with_preedit((3, 5), Some((5, 2)), &Expand::After);
        assert_eq!(mapped, (3, 7));
    }

    #[test]
    fn map_range_with_preedit_at_end_with_expand_before() {
        let mapped = map_range_with_preedit((3, 5), Some((5, 2)), &Expand::Before);
        assert_eq!(mapped, (3, 5));
    }

    #[test]
    fn map_range_with_preedit_at_end_with_expand_both() {
        let mapped = map_range_with_preedit((3, 5), Some((5, 2)), &Expand::Both);
        assert_eq!(mapped, (3, 7));
    }

    #[test]
    fn map_range_with_preedit_at_end_with_expand_none() {
        let mapped = map_range_with_preedit((3, 5), Some((5, 2)), &Expand::None);
        assert_eq!(mapped, (3, 5));
    }

    #[test]
    fn preedit_from_other_node_does_not_shift_style_range() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut decorations = Decorations::default();
        decorations.preedit = Some(PreeditDecor {
            node_id: p2,
            offset: 1,
            text: "가나".into(),
        });

        let state = state! {
            doc {
                @p1 paragraph {
                    text(styles: [font_weight(700)]) { "abcd" }
                }
                @p2 paragraph {
                    text { "efgh" }
                }
            }
            selection { (p1, 0) }
        };

        let doc = &state.doc;
        let para = doc.node(p1).unwrap();
        let settings = doc.settings();
        let default_attrs = DefaultAttrs::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let ctx = LayoutContext::new(
            &para,
            &settings,
            &default_attrs,
            &decorations,
            1.0,
            &view_states,
            &cache,
        );

        let preedit_info =
            preedit_for_node(&ctx).map(|preedit| (preedit.offset, preedit.text.chars().count()));

        let mapped = map_range_with_preedit((0, 1), preedit_info, &Expand::After);
        assert_eq!(mapped, (0, 1));
    }

    #[test]
    fn extract_background_segments_includes_preedit_styles() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(styles: [bg_color("red")]) { "ABC" }
                }
            }
            selection { (p, 0) }
        };

        let mut decorations = Decorations::default();
        decorations.preedit = Some(PreeditDecor {
            node_id: p,
            offset: 1,
            text: "XY".into(),
        });
        decorations.pending_styles = PendingStylesDecor {
            node_id: p,
            styles: vec![Style::BackgroundColor(BackgroundColorStyle {
                color: "blue".into(),
            })],
        };

        let doc = &state.doc;
        let para = doc.node(p).unwrap();
        let settings = doc.settings();
        let default_attrs = DefaultAttrs::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let ctx = LayoutContext::new(
            &para,
            &settings,
            &default_attrs,
            &decorations,
            1.0,
            &view_states,
            &cache,
        );

        let segments = extract_background_segments(&ctx);

        assert_eq!(segments.len(), 2);

        assert_eq!(segments[0].start_offset, 0);
        assert_eq!(segments[0].end_offset, 5);
        assert_eq!(segments[0].color_key, "red");

        assert_eq!(segments[1].start_offset, 1);
        assert_eq!(segments[1].end_offset, 3);
        assert_eq!(segments[1].color_key, "blue");
    }

    #[test]
    fn extract_ruby_segments_returns_empty_without_annotations() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text { "ABC" }
                }
            }
            selection { (p, 0) }
        };

        let mut decorations = Decorations::default();
        decorations.preedit = Some(PreeditDecor {
            node_id: p,
            offset: 1,
            text: "XY".into(),
        });

        let doc = &state.doc;
        let para = doc.node(p).unwrap();
        let settings = doc.settings();
        let default_attrs = DefaultAttrs::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let ctx = LayoutContext::new(
            &para,
            &settings,
            &default_attrs,
            &decorations,
            1.0,
            &view_states,
            &cache,
        );

        let segments = extract_ruby_segments(&ctx);

        assert_eq!(segments.len(), 0);
    }
}
