use crate::global::GLOBALS;
use crate::layout::elements::{BackgroundSegment, LineElement, RubySegment, build_metrics};
use crate::layout::{Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy, PositionedNode};
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule, parse_styles};
use crate::model::{Annotation, Node, PendingStylesDecor, PreeditDecor, Style};
use crate::schema::Expand;
use crate::types::{BoxConstraints, Point, Size};
use crate::utils::{
    LengthUnit, build_char_to_byte_offsets, char_to_byte_offset, char_to_byte_offset_with_map,
    convert_length,
};
use macros::Codec;
use parley::style::*;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::rc::Rc;

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

fn extract_ruby_segments(ctx: &LayoutContext) -> Vec<RubySegment> {
    let mut ruby_segments = Vec::new();
    let mut offset = 0;

    let preedit = preedit_for_node(ctx);
    let preedit_info = preedit.map(|preedit| (preedit.offset, preedit.text.chars().count()));

    for child in ctx.node.children() {
        if let Node::Text(node) = child.node() {
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
        } else if let Node::HardBreak(_) = child.node() {
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
        if let Node::Text(node) = child.node() {
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
        } else if let Node::HardBreak(_) = child.node() {
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
    builder: &mut parley::RangedBuilder<'_, String>,
    style: &Style,
    range: std::ops::Range<usize>,
    font_size: u32,
) {
    match style {
        Style::FontFamily(m) => builder.push(
            StyleProperty::FontFamily(FontFamily::Single(FontFamilyName::Named(
                m.family.clone().into(),
            ))),
            range,
        ),
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
        Style::Bold(_) => {}
        Style::Italic(_) => builder.push(StyleProperty::FontStyle(FontStyle::Italic), range),
        Style::Strikethrough(_) => builder.push(StyleProperty::Strikethrough(true), range),
        Style::Underline(_) => builder.push(StyleProperty::Underline(true), range),
        Style::TextColor(m) => {
            builder.push(StyleProperty::Brush(format!("text.{}", m.color)), range)
        }
        Style::BackgroundColor(_) => {}
    }
}

fn apply_annotation_to_builder(
    builder: &mut parley::RangedBuilder<'_, String>,
    annotation: &Annotation,
    range: std::ops::Range<usize>,
) {
    match annotation {
        Annotation::Link(_) => {
            builder.push(StyleProperty::Underline(true), range.clone());
            builder.push(StyleProperty::Brush("ui.text.faint".to_string()), range);
        }
        Annotation::Ruby(_) => {}
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
        let mut text = ctx
            .node
            .children()
            .filter_map(|child| match child.node() {
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

            let mut offset = 0;
            for child in ctx.node.children() {
                match child.node() {
                    Node::Text(node) => {
                        let segments = node.text.get_segments();
                        let mut segment_offset = 0;

                        for segment in segments {
                            let segment_len = segment.text.chars().count();
                            let base_start = offset + segment_offset;
                            let base_end = base_start + segment_len;

                            let segment_font_size = segment
                                .styles
                                .iter()
                                .find_map(|s| {
                                    if let Style::FontSize(fs) = s {
                                        Some(fs.size)
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(1200);

                            let has_embolden =
                                segment.styles.iter().any(|s| matches!(s, Style::Bold(_)));

                            for style in &segment.styles {
                                let (start, end) = map_range_with_preedit(
                                    (base_start, base_end),
                                    preedit_info,
                                    &Expand::None,
                                );
                                let range = char_to_byte_offset_with_map(&char_to_byte, start)
                                    ..char_to_byte_offset_with_map(&char_to_byte, end);

                                if has_embolden && let Style::FontWeight(weight_style) = style {
                                    let target_weight = weight_style.weight.max(700) as f32;
                                    builder.push(
                                        StyleProperty::FontWeight(FontWeight::new(target_weight)),
                                        range,
                                    );
                                    continue;
                                }

                                apply_style_to_builder(
                                    &mut builder,
                                    style,
                                    range,
                                    segment_font_size,
                                );
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
                    Node::HardBreak(_) => {
                        offset += 1;
                    }
                    _ => continue,
                }
            }

            if let Some(preedit) = preedit {
                if let Some(ps) = pending_styles {
                    let preedit_start = preedit.offset;
                    let preedit_end = preedit_start + preedit.text.chars().count();
                    let range = char_to_byte_offset_with_map(&char_to_byte, preedit_start)
                        ..char_to_byte_offset_with_map(&char_to_byte, preedit_end);

                    let preedit_font_size = ps
                        .styles
                        .iter()
                        .find_map(|s| {
                            if let Style::FontSize(fs) = s {
                                Some(fs.size)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(1200);

                    let has_embolden = ps.styles.iter().any(|s| matches!(s, Style::Bold(_)));
                    for style in &ps.styles {
                        if has_embolden && let Style::FontWeight(weight_style) = style {
                            let target_weight = weight_style.weight.max(700) as f32;
                            builder.push(
                                StyleProperty::FontWeight(FontWeight::new(target_weight)),
                                range.clone(),
                            );
                            continue;
                        }
                        apply_style_to_builder(
                            &mut builder,
                            style,
                            range.clone(),
                            preedit_font_size,
                        );
                    }
                }
            }

            if is_text_empty {
                if let Some(ps) = pending_styles {
                    let range = 0..text.len();
                    let font_size = ps
                        .styles
                        .iter()
                        .find_map(|s| {
                            if let Style::FontSize(fs) = s {
                                Some(fs.size)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(1200);
                    let has_embolden = ps.styles.iter().any(|s| matches!(s, Style::Bold(_)));
                    for style in &ps.styles {
                        if has_embolden && let Style::FontWeight(weight_style) = style {
                            let target_weight = weight_style.weight.max(700) as f32;
                            builder.push(
                                StyleProperty::FontWeight(FontWeight::new(target_weight)),
                                range.clone(),
                            );
                            continue;
                        }
                        apply_style_to_builder(&mut builder, style, range.clone(), font_size);
                    }
                }
            }

            let parent_is_root = ctx
                .node
                .parent()
                .map(|parent| matches!(parent.node(), Node::Root(_)))
                .unwrap_or(false);
            let indent = if parent_is_root {
                (ctx.settings.paragraph_indent as f32 / 100.0 * 16.0).max(0.0)
            } else {
                0.0
            };

            let mut layout = builder.build(&text);

            if matches!(self.align, TextAlign::Left | TextAlign::Justify if indent > 0.0) {
                layout.indent(indent, parley::IndentOptions::default());
            }

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

            let default_height = {
                let ps_styles = pending_styles.map(|ps| &ps.styles[..]);

                let mut dummy_builder = lcx.ranged_builder(&mut fcx, "\u{200B}", 1.0, false);
                dummy_builder.push_default(StyleProperty::FontFamily(FontFamily::Single(
                    FontFamilyName::Named(cascade_family.clone().into()),
                )));
                dummy_builder.push_default(StyleProperty::FontWeight(FontWeight::new(
                    cascade_weight as f32,
                )));
                dummy_builder.push_default(StyleProperty::FontSize(convert_length(
                    cascade_font_size as f32 / 100.0,
                    LengthUnit::Pt,
                    LengthUnit::Px,
                )));
                dummy_builder.push_default(StyleProperty::LineHeight(
                    LineHeight::FontSizeRelative(line_height),
                ));
                dummy_builder.push_default(StyleProperty::FontFeatures(FontFeatures::Source(
                    Cow::Owned("\"ss05\" 1, \"cv12\" 1, \"ss18\" 1".to_string()),
                )));

                if let Some(styles) = ps_styles {
                    let range = 0.."\u{200B}".len();
                    let font_size = styles
                        .iter()
                        .find_map(|s| {
                            if let Style::FontSize(fs) = s {
                                Some(fs.size)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(1200);
                    for style in styles {
                        apply_style_to_builder(&mut dummy_builder, style, range.clone(), font_size);
                    }
                }

                let mut dummy_layout = dummy_builder.build("\u{200B}");
                dummy_layout.break_all_lines(None);
                let dummy_line = dummy_layout.lines().next().unwrap();
                let dummy_metrics = dummy_line.metrics();
                dummy_metrics.ascent + dummy_metrics.descent
            };

            (layout, default_height)
        });

        let (layout, default_height) = layout;
        let layout = Rc::new(layout);
        let metrics = build_metrics(&layout, &text, ctx.scale_factor, default_height);

        let ruby_segments = extract_ruby_segments(ctx);
        let background_segments = extract_background_segments(ctx);

        let has_page_break = ctx
            .node
            .children()
            .last()
            .map(|child| matches!(child.node(), Node::PageBreak(_)))
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
    use crate::layout::LayoutCache;
    use crate::model::{
        BackgroundColorStyle, Decorations, DefaultAttrs, PendingStylesDecor, PreeditDecor,
    };
    use crate::runtime::ViewStates;
    use crate::types::BoxConstraints;
    use std::cell::RefCell;

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

        if let Node::Paragraph(paragraph) = para.node() {
            paragraph.layout(&ctx, constraints);
        } else {
            panic!("paragraph node expected");
        }
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
