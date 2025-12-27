use crate::global::GLOBALS;
use crate::layout::elements::{LineElement, build_metrics};
use crate::layout::{Element, Layout, LayoutContext, LayoutNode, PageBreakPolicy, PositionedNode};
use crate::model::html::{DomSpec, NodeHtmlCodec, NodeParseRule, parse_styles};
use crate::model::{FontFamilyMark, Mark, Node, PreeditDecor};
use crate::schema::Expand;
use crate::types::{BoxConstraints, Point, Size};
use crate::utils::{LengthUnit, char_to_byte_offset, convert_length};
use macros::Codec;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::rc::Rc;
use tsify::Tsify;

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

fn extract_ruby_segments(ctx: &LayoutContext) -> Vec<crate::layout::elements::RubySegment> {
    use crate::layout::elements::RubySegment;
    use crate::model::{Mark, Node};

    let mut ruby_segments = Vec::new();
    let mut offset = 0;

    let preedit = preedit_for_node(ctx);
    let preedit_info = preedit.map(|preedit| (preedit.offset, preedit.text.chars().count()));
    let preedit_has_explicit_marks = preedit.map(|p| p.marks.is_some()).unwrap_or(false);

    let schema = ctx.node.schema();

    for child in ctx.node.children() {
        if let Node::Text(node) = child.node() {
            let segments = node.text.get_rich_text_segments();

            for (segment_text, segment_marks) in segments {
                let segment_len = segment_text.chars().count();
                let base_start = offset;
                let base_end = offset + segment_len;

                for mark in segment_marks {
                    if let Mark::Ruby(ref ruby_mark) = mark {
                        let expand = if preedit_has_explicit_marks {
                            &Expand::None
                        } else {
                            &schema.mark_spec(mark.as_type()).expand
                        };
                        let (start, end) =
                            map_range_with_preedit((base_start, base_end), preedit_info, expand);

                        ruby_segments.push(RubySegment {
                            start_offset: start,
                            end_offset: end,
                            ruby_text: ruby_mark.text.clone(),
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

    if let Some(preedit) = preedit {
        if let Some(marks) = &preedit.marks {
            let preedit_start = preedit.offset;
            let preedit_end = preedit_start + preedit.text.chars().count();

            for mark in marks {
                if let Mark::Ruby(ruby_mark) = mark {
                    ruby_segments.push(RubySegment {
                        start_offset: preedit_start,
                        end_offset: preedit_end,
                        ruby_text: ruby_mark.text.clone(),
                    });
                }
            }
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

fn extract_background_segments(
    ctx: &LayoutContext,
) -> Vec<crate::layout::elements::BackgroundSegment> {
    use crate::layout::elements::BackgroundSegment;
    use crate::model::{Mark, Node};

    let mut background_segments = Vec::new();
    let mut offset = 0;

    let preedit = preedit_for_node(ctx);
    let preedit_info = preedit.map(|preedit| (preedit.offset, preedit.text.chars().count()));
    let preedit_has_explicit_marks = preedit.map(|p| p.marks.is_some()).unwrap_or(false);

    let schema = ctx.node.schema();

    for child in ctx.node.children() {
        if let Node::Text(node) = child.node() {
            let segments = node.text.get_rich_text_segments();

            for (segment_text, segment_marks) in segments {
                let segment_len = segment_text.chars().count();
                let base_start = offset;
                let base_end = offset + segment_len;

                for mark in segment_marks {
                    if let Mark::BackgroundColor(ref bg_mark) = mark {
                        let expand = if preedit_has_explicit_marks {
                            &Expand::None
                        } else {
                            &schema.mark_spec(mark.as_type()).expand
                        };
                        let (start, end) =
                            map_range_with_preedit((base_start, base_end), preedit_info, expand);

                        background_segments.push(BackgroundSegment {
                            start_offset: start,
                            end_offset: end,
                            color_key: bg_mark.key.clone(),
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

    if let Some(preedit) = preedit {
        if let Some(marks) = &preedit.marks {
            let preedit_start = preedit.offset;
            let preedit_end = preedit_start + preedit.text.chars().count();

            for mark in marks {
                if let Mark::BackgroundColor(bg_mark) = mark {
                    background_segments.push(BackgroundSegment {
                        start_offset: preedit_start,
                        end_offset: preedit_end,
                        color_key: bg_mark.key.clone(),
                    });
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

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Hash, Serialize, Deserialize, Codec, Tsify,
)]
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

fn default_line_height() -> f32 {
    1.6
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Codec, Tsify)]
pub struct ParagraphNode {
    #[serde(default)]
    pub align: TextAlign,
    #[serde(default = "default_line_height")]
    pub line_height: f32,
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
        self.line_height.to_bits().hash(state);
    }
}

impl ParagraphNode {
    pub fn reset_attributes(&mut self) -> bool {
        let mut changed = false;
        if self.align != TextAlign::default() {
            self.align = TextAlign::default();
            changed = true;
        }
        if self.line_height != default_line_height() {
            self.line_height = default_line_height();
            changed = true;
        }

        changed
    }

    fn build_style(&self) -> String {
        let mut s = Vec::new();
        if self.align != TextAlign::Left {
            s.push(format!("text-align:{}", self.align));
        }
        if (self.line_height - 1.6).abs() > 0.01 {
            s.push(format!("line-height:{}", self.line_height));
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
                        n.line_height = lh.parse().unwrap_or(1.6);
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
                            n.line_height = lh.parse().unwrap_or(1.6);
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
        let preedit_has_explicit_marks = preedit.map(|p| p.marks.is_some()).unwrap_or(false);

        if let Some(preedit) = preedit {
            let idx = char_to_byte_offset(&text, preedit.offset);
            text.insert_str(idx, &preedit.text);
        }

        let is_text_empty = text.is_empty();
        if text.is_empty() {
            text = "\u{200B}".to_string();
        }

        let line_height = self.line_height;
        let layout = GLOBALS.with(|globals| {
            use parley::style::*;

            let globals = globals.borrow();

            let mut lcx = globals.parley_layout_context.borrow_mut();
            let mut fcx = globals.parley_font_context.borrow_mut();

            let setup_defaults = |builder: &mut parley::RangedBuilder<'_, String>| {
                builder.push_default(StyleProperty::FontStack(FontStack::Single(
                    FontFamily::Named(FontFamilyMark::default().family.into()),
                )));
                builder.push_default(StyleProperty::FontSize(16.0));
                builder.push_default(StyleProperty::FontWeight(FontWeight::new(400.0)));
                builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
                    line_height,
                )));
                builder.push_default(StyleProperty::LetterSpacing(0.0));

                builder.push_default(StyleProperty::FontFeatures(FontSettings::Source(
                    Cow::Owned("\"ss05\" 1, \"cv12\" 1, \"ss18\" 1".to_string()),
                )));
            };

            let mut builder = lcx.ranged_builder(&mut fcx, &text, 1.0, false);

            let apply_mark = |builder: &mut parley::RangedBuilder<'_, String>,
                              mark: &Mark,
                              range: std::ops::Range<usize>,
                              font_size: f32| {
                match mark {
                    Mark::FontFamily(m) => builder.push(
                        StyleProperty::FontStack(FontStack::Single(FontFamily::Named(
                            m.family.clone().into(),
                        ))),
                        range,
                    ),
                    Mark::FontSize(m) => builder.push(
                        StyleProperty::FontSize(convert_length(
                            m.size,
                            LengthUnit::Pt,
                            LengthUnit::Px,
                        )),
                        range,
                    ),
                    Mark::FontWeight(m) => builder.push(
                        StyleProperty::FontWeight(FontWeight::new(m.weight as f32)),
                        range,
                    ),
                    Mark::LetterSpacing(m) => {
                        let font_size_px =
                            convert_length(font_size, LengthUnit::Pt, LengthUnit::Px);
                        builder.push(
                            StyleProperty::LetterSpacing(m.spacing * font_size_px),
                            range,
                        )
                    }
                    Mark::Italic(_) => {
                        builder.push(StyleProperty::FontStyle(FontStyle::Italic), range)
                    }
                    Mark::Strikethrough(_) => {
                        builder.push(StyleProperty::Strikethrough(true), range)
                    }
                    Mark::Underline(_) => builder.push(StyleProperty::Underline(true), range),
                    Mark::TextColor(m) => builder.push(
                        StyleProperty::Brush(format!("text.{}", m.key.clone())),
                        range,
                    ),
                    Mark::Ruby(_) => {
                        // Parley가 아직 ruby를 지원하지 않으므로 layout 단계가 아닌 rendering 단계에서 처리
                        // https://github.com/linebender/parley/issues/255
                    }
                    Mark::BackgroundColor(_) => {
                        // Parley가 background를 지원하지 않으므로 rendering 단계에서 처리
                    }
                }
            };

            let parent_is_root = ctx
                .node
                .parent()
                .map(|parent| matches!(parent.node(), Node::Root(_)))
                .unwrap_or(false);
            let indent = if parent_is_root {
                (ctx.settings.paragraph_indent * 16.0).max(0.0)
            } else {
                0.0
            };

            builder.push_default(StyleProperty::OverflowWrap(OverflowWrap::Anywhere));
            builder.push_default(match self.align {
                TextAlign::Justify => StyleProperty::WordBreak(WordBreakStrength::KeepAll),
                _ => StyleProperty::WordBreak(WordBreakStrength::BreakAll),
            });

            setup_defaults(&mut builder);

            match self.align {
                TextAlign::Left | TextAlign::Justify if indent > 0.0 => {
                    builder.push_inline_box(parley::InlineBox {
                        id: 0,
                        index: 0,
                        width: indent,
                        height: 0.0,
                    });
                }
                _ => {}
            }

            let schema = ctx.node.schema();
            let mut offset = 0;
            for child in ctx.node.children() {
                match child.node() {
                    Node::Text(node) => {
                        let segments = node.text.get_rich_text_segments();
                        let mut segment_offset = 0;

                        for (segment_text, segment_marks) in segments {
                            let segment_len = segment_text.chars().count();
                            let base_start = offset + segment_offset;
                            let base_end = base_start + segment_len;

                            let segment_font_size = segment_marks
                                .iter()
                                .find_map(|m| {
                                    if let Mark::FontSize(fs) = m {
                                        Some(fs.size)
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(12.0);

                            for mark in &segment_marks {
                                let expand = if preedit_has_explicit_marks {
                                    &Expand::None
                                } else {
                                    &schema.mark_spec(mark.as_type()).expand
                                };
                                let (start, end) = map_range_with_preedit(
                                    (base_start, base_end),
                                    preedit_info,
                                    expand,
                                );
                                let range = char_to_byte_offset(&text, start)
                                    ..char_to_byte_offset(&text, end);

                                apply_mark(&mut builder, mark, range, segment_font_size);
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
                if let Some(marks) = &preedit.marks {
                    let preedit_start = preedit.offset;
                    let preedit_end = preedit_start + preedit.text.chars().count();
                    let range = char_to_byte_offset(&text, preedit_start)
                        ..char_to_byte_offset(&text, preedit_end);

                    let preedit_font_size = marks
                        .iter()
                        .find_map(|m| {
                            if let Mark::FontSize(fs) = m {
                                Some(fs.size)
                            } else {
                                None
                            }
                        })
                        .unwrap_or(12.0);

                    for mark in marks {
                        apply_mark(&mut builder, mark, range.clone(), preedit_font_size);
                    }
                }
            }

            let mut layout = builder.build(&text);
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

            let mut dummy_builder = lcx.ranged_builder(&mut fcx, "\u{200B}", 1.0, false);
            setup_defaults(&mut dummy_builder);

            let mut dummy_layout = dummy_builder.build("\u{200B}");
            dummy_layout.break_all_lines(None);
            let dummy_line = dummy_layout.lines().next().unwrap();
            let dummy_metrics = dummy_line.metrics();
            let default_height = dummy_metrics.ascent + dummy_metrics.descent;

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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutCache;
    use crate::model::{Decorations, PreeditDecor};
    use crate::types::BoxConstraints;
    use std::cell::RefCell;

    #[test]
    fn layout_handles_multibyte_mark_ranges() {
        let mut p = id!();

        let state = state! {
            doc {
                @p paragraph {
                    text(marks: [font_size(24.0)]) { "ㅁㄴㅇㄹ" }
                }
            }
            selection { (p, 0) }
        };

        let doc = &state.doc;
        let para = doc.node(p).unwrap();
        let settings = doc.settings();
        let decorations = Decorations::default();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = crate::runtime::ViewStates::default();
        let ctx = LayoutContext::new(&para, &settings, &decorations, 1.0, &view_states, &cache);
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
    fn preedit_from_other_node_does_not_shift_mark_range() {
        let mut p1 = id!();
        let mut p2 = id!();

        let mut decorations = Decorations::default();
        decorations.preedit = Some(PreeditDecor {
            node_id: p2,
            offset: 1,
            text: "가나".into(),
            marks: None,
        });

        let state = state! {
            doc {
                @p1 paragraph {
                    text(marks: [font_weight(700)]) { "abcd" }
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
        let cache = RefCell::new(LayoutCache::new());
        let view_states = crate::runtime::ViewStates::default();
        let ctx = LayoutContext::new(&para, &settings, &decorations, 1.0, &view_states, &cache);

        let preedit_info =
            preedit_for_node(&ctx).map(|preedit| (preedit.offset, preedit.text.chars().count()));

        let mapped = map_range_with_preedit((0, 1), preedit_info, &Expand::After);
        assert_eq!(mapped, (0, 1));
    }

    #[test]
    fn extract_background_segments_includes_preedit_marks() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(marks: [bg_color("red")]) { "ABC" }
                }
            }
            selection { (p, 0) }
        };

        let mut decorations = Decorations::default();
        decorations.preedit = Some(PreeditDecor {
            node_id: p,
            offset: 1,
            text: "XY".into(),
            marks: Some(vec![Mark::BackgroundColor(
                crate::model::BackgroundColorMark { key: "blue".into() },
            )]),
        });

        let doc = &state.doc;
        let para = doc.node(p).unwrap();
        let settings = doc.settings();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = crate::runtime::ViewStates::default();
        let ctx = LayoutContext::new(&para, &settings, &decorations, 1.0, &view_states, &cache);

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
    fn extract_ruby_segments_expands_with_preedit() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text(marks: [ruby("ruby_text")]) { "ABC" }
                }
            }
            selection { (p, 0) }
        };

        let mut decorations = Decorations::default();
        decorations.preedit = Some(PreeditDecor {
            node_id: p,
            offset: 1,
            text: "XY".into(),
            marks: None,
        });

        let doc = &state.doc;
        let para = doc.node(p).unwrap();
        let settings = doc.settings();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = crate::runtime::ViewStates::default();
        let ctx = LayoutContext::new(&para, &settings, &decorations, 1.0, &view_states, &cache);

        let segments = extract_ruby_segments(&ctx);

        assert_eq!(segments.len(), 1);

        assert_eq!(segments[0].start_offset, 0);
        assert_eq!(segments[0].end_offset, 5);
        assert_eq!(segments[0].ruby_text, "ruby_text");
    }
}
