use editor_common::StrExt;
use editor_model::{Doc, Modifier, ModifierType, NodeId, NodeRef};
use editor_resource::TextBrush;
use icu_segmenter::GraphemeClusterSegmenter;
use parley::Layout;

use super::strut::StrutMetrics;
use super::style_run::StyleRun;
use super::text_run::TextRun;
use crate::glyph_run::{Glyph, GlyphRun, GraphemeSpan, Synthesis, TextDecoration};
use crate::measure::resolve::resolve_inherited;

pub struct ExtractedLine {
    pub height: f32,
    pub baseline: f32,
    pub ascent: f32,
    pub descent: f32,
    pub glyph_runs: Vec<GlyphRun>,
}

pub struct LineHeightConfig {
    pub line_height_ratio: f32,
    pub base_font_size: f32,
}

const ITALIC_SKEW_DEGREES: f32 = 14.0;

fn resolve_synthesis(doc: &Doc, node_id: NodeId) -> Synthesis {
    let (bold, italic) = doc
        .node(node_id)
        .map(|node_ref| {
            let bold = resolve_inherited(&node_ref, ModifierType::Bold).is_some();
            let italic = resolve_inherited(&node_ref, ModifierType::Italic).is_some();
            (bold, italic)
        })
        .unwrap_or_default();

    Synthesis {
        embolden: bold,
        skew: if italic {
            Some(ITALIC_SKEW_DEGREES)
        } else {
            None
        },
    }
}

const LINK_COLOR: &str = "text.blue";

fn has_link_modifier(node_ref: &NodeRef<'_>) -> bool {
    node_ref
        .modifiers()
        .any(|m| matches!(m, Modifier::Link { .. }))
}

fn resolve_decoration(doc: &Doc, node_id: NodeId) -> TextDecoration {
    doc.node(node_id)
        .map(|node_ref| {
            let underline = has_link_modifier(&node_ref)
                || resolve_inherited(&node_ref, ModifierType::Underline).is_some();
            TextDecoration {
                underline,
                strikethrough: resolve_inherited(&node_ref, ModifierType::Strikethrough).is_some(),
            }
        })
        .unwrap_or_default()
}

fn resolve_text_colors(doc: &Doc, node_id: NodeId) -> (String, Option<String>) {
    let node_ref = doc.node(node_id);

    let color = node_ref
        .as_ref()
        .map(resolve_text_color)
        .unwrap_or_else(|| "text.black".to_string());

    let background_color = node_ref.as_ref().and_then(|nr| {
        resolve_inherited(nr, ModifierType::BackgroundColor).and_then(|m| match m {
            Modifier::BackgroundColor { value } if value != "none" => Some(format!("bg.{value}")),
            _ => None,
        })
    });

    (color, background_color)
}

fn resolve_text_color(node_ref: &NodeRef<'_>) -> String {
    let mut ancestors = node_ref.ancestors();
    if let Some(self_node) = ancestors.next() {
        if let Some(Modifier::TextColor { value }) = self_node
            .modifiers_with_style()
            .find(|m| matches!(m, Modifier::TextColor { .. }))
        {
            return format!("text.{value}");
        }
        if has_link_modifier(&self_node) {
            return LINK_COLOR.to_string();
        }
    }
    for ancestor in ancestors {
        if let Some(Modifier::TextColor { value }) = ancestor
            .modifiers_with_style()
            .find(|m| matches!(m, Modifier::TextColor { .. }))
        {
            return format!("text.{value}");
        }
    }
    "text.black".to_string()
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

pub fn extract_lines(
    doc: &Doc,
    text: &str,
    layout: &Layout<TextBrush>,
    style_runs: &[StyleRun],
    text_runs: &[TextRun],
    strut: &StrutMetrics,
    height_config: LineHeightConfig,
    grapheme_segmenter: &GraphemeClusterSegmenter,
) -> Vec<ExtractedLine> {
    let LineHeightConfig {
        line_height_ratio,
        base_font_size,
    } = height_config;
    let mut lines = Vec::new();

    for line in layout.lines() {
        let metrics = line.metrics();

        let ascent = metrics.ascent.max(strut.ascent);
        let descent = metrics.descent.max(strut.descent);
        let content_height = ascent + descent;

        let max_run_font_size = line
            .items()
            .filter_map(|item| match item {
                parley::PositionedLayoutItem::GlyphRun(gr) => Some(gr.run().font_size()),
                _ => None,
            })
            .fold(base_font_size, f32::max);
        let line_box_height = (max_run_font_size * line_height_ratio).max(content_height);
        let leading = (line_box_height - content_height).max(0.0);
        let baseline = leading / 2.0 + ascent;

        let mut glyph_runs = Vec::new();
        let mut x = metrics.offset;

        for item in line.items() {
            if let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                let run = glyph_run.run();
                let font_size = run.font_size();

                let run_x = glyph_run.offset();
                let mut glyph_x_advance = 0.0;
                let glyphs: Vec<Glyph> = glyph_run
                    .glyphs()
                    .map(|g| {
                        let gx = glyph_x_advance + g.x;
                        glyph_x_advance += g.advance;
                        Glyph {
                            id: g.id,
                            x: run_x + gx,
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
                let node_id = glyph_run.style().brush.node_id;
                let node_byte_start = text_runs
                    .iter()
                    .find(|tr| tr.node_id == node_id)
                    .map(|tr| tr.byte_range.start)
                    .unwrap_or(0);
                let char_offset = text[node_byte_start..byte_start].char_count();
                let synthesis = resolve_synthesis(doc, node_id);
                let decoration = resolve_decoration(doc, node_id);
                let (color, background_color) = resolve_text_colors(doc, node_id);

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
                    node_id,
                    offset: char_offset,
                    text: run_text,
                    x,
                    width: run_advance,
                    graphemes,
                });

                x += run_advance;
            }
        }

        lines.push(ExtractedLine {
            height: line_box_height,
            baseline,
            ascent,
            descent,
            glyph_runs,
        });
    }

    lines
}
#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;

    fn segmenter() -> GraphemeClusterSegmenter {
        GraphemeClusterSegmenter::new().static_to_owned()
    }

    #[test]
    fn resolve_text_colors_default_uses_root_text_color() {
        let (doc, t1) = doc! {
            root { paragraph { t1: text("hello") } }
        };
        let (color, bg) = resolve_text_colors(&doc, t1);
        assert_eq!(color, "text.black");
        assert_eq!(bg, None);
    }

    #[test]
    fn resolve_text_colors_prefixes_modifier_values() {
        let (doc, t1) = doc! {
            root {
                paragraph {
                    t1: text("hello") [
                        text_color("red".to_string()),
                        background_color("yellow".to_string())
                    ]
                }
            }
        };
        let (color, bg) = resolve_text_colors(&doc, t1);
        assert_eq!(color, "text.red");
        assert_eq!(bg.as_deref(), Some("bg.yellow"));
    }

    #[test]
    fn resolve_text_colors_filters_none_background() {
        let (doc, t1) = doc! {
            root {
                paragraph {
                    t1: text("hello") [background_color("none".to_string())]
                }
            }
        };
        let (_color, bg) = resolve_text_colors(&doc, t1);
        assert_eq!(bg, None);
    }

    #[test]
    fn text_color_message_sent_with_seeded_root_modifier() {
        let (doc, t1) = doc! {
            root {
                blockquote(variant: BlockquoteVariant::MessageSent) {
                    paragraph { t1: text("hello") }
                }
            }
        };
        let (color, _) = resolve_text_colors(&doc, t1);
        assert_eq!(color, "text.bright");
    }

    #[test]
    fn text_color_message_sent_overrides_explicit_root_modifier() {
        let (doc, t1) = doc! {
            root [text_color("red".to_string())] {
                blockquote(variant: BlockquoteVariant::MessageSent) {
                    paragraph { t1: text("hello") }
                }
            }
        };
        let (color, _) = resolve_text_colors(&doc, t1);
        assert_eq!(color, "text.bright");
    }

    #[test]
    fn text_color_message_sent_respects_explicit_paragraph_modifier() {
        let (doc, t1) = doc! {
            root {
                blockquote(variant: BlockquoteVariant::MessageSent) {
                    paragraph [text_color("red".to_string())] {
                        t1: text("hello")
                    }
                }
            }
        };
        let (color, _) = resolve_text_colors(&doc, t1);
        assert_eq!(color, "text.red");
    }

    #[test]
    fn text_color_message_received_uses_root() {
        let (doc, t1) = doc! {
            root {
                blockquote(variant: BlockquoteVariant::MessageReceived) {
                    paragraph { t1: text("hello") }
                }
            }
        };
        let (color, _) = resolve_text_colors(&doc, t1);
        assert_eq!(color, "text.black");
    }

    #[test]
    fn text_color_message_sent_through_nested_container() {
        let (doc, t1) = doc! {
            root {
                blockquote(variant: BlockquoteVariant::MessageSent) {
                    bullet_list {
                        list_item {
                            paragraph { t1: text("hello") }
                        }
                    }
                }
            }
        };
        let (color, _) = resolve_text_colors(&doc, t1);
        assert_eq!(color, "text.bright");
    }

    #[test]
    fn text_color_inherits_from_root_outside_message_sent() {
        let (doc, t1) = doc! {
            root [text_color("red".to_string())] {
                paragraph { t1: text("hello") }
            }
        };
        let (color, _) = resolve_text_colors(&doc, t1);
        assert_eq!(color, "text.red");
    }

    #[test]
    fn resolve_decoration_none_by_default() {
        let (doc, t1) = doc! {
            root { paragraph { t1: text("hello") } }
        };
        let d = resolve_decoration(&doc, t1);
        assert!(!d.underline);
        assert!(!d.strikethrough);
    }

    #[test]
    fn resolve_decoration_underline_only() {
        let (doc, t1) = doc! {
            root { paragraph { t1: text("hello") [underline] } }
        };
        let d = resolve_decoration(&doc, t1);
        assert!(d.underline);
        assert!(!d.strikethrough);
    }

    #[test]
    fn resolve_decoration_strikethrough_only() {
        let (doc, t1) = doc! {
            root { paragraph { t1: text("hello") [strikethrough] } }
        };
        let d = resolve_decoration(&doc, t1);
        assert!(!d.underline);
        assert!(d.strikethrough);
    }

    #[test]
    fn resolve_decoration_both() {
        let (doc, t1) = doc! {
            root { paragraph { t1: text("hello") [underline, strikethrough] } }
        };
        let d = resolve_decoration(&doc, t1);
        assert!(d.underline);
        assert!(d.strikethrough);
    }

    #[test]
    fn resolve_decoration_inherits_from_ancestor() {
        let (doc, t1) = doc! {
            root [underline] {
                paragraph { t1: text("hello") }
            }
        };
        let d = resolve_decoration(&doc, t1);
        assert!(d.underline);
        assert!(!d.strikethrough);
    }

    #[test]
    fn resolve_decoration_inherits_strikethrough_from_ancestor() {
        let (doc, t1) = doc! {
            root [strikethrough] {
                paragraph { t1: text("hello") }
            }
        };
        let d = resolve_decoration(&doc, t1);
        assert!(!d.underline);
        assert!(d.strikethrough);
    }

    #[test]
    fn link_modifier_underlines_text() {
        let (doc, t1) = doc! {
            root {
                paragraph {
                    t1: text("hello") [link(href: "https://example.com".into())]
                }
            }
        };
        let d = resolve_decoration(&doc, t1);
        assert!(d.underline);
    }

    #[test]
    fn link_modifier_uses_link_color() {
        let (doc, t1) = doc! {
            root {
                paragraph {
                    t1: text("hello") [link(href: "https://example.com".into())]
                }
            }
        };
        let (color, _) = resolve_text_colors(&doc, t1);
        assert_eq!(color, LINK_COLOR);
    }

    #[test]
    fn explicit_text_color_overrides_link_color() {
        let (doc, t1) = doc! {
            root {
                paragraph {
                    t1: text("hello") [
                        link(href: "https://example.com".into()),
                        text_color("red".to_string())
                    ]
                }
            }
        };
        let (color, _) = resolve_text_colors(&doc, t1);
        assert_eq!(color, "text.red");
    }

    #[test]
    fn link_color_overrides_inherited_text_color() {
        let (doc, t1) = doc! {
            root [text_color("red".to_string())] {
                paragraph {
                    t1: text("hello") [link(href: "https://example.com".into())]
                }
            }
        };
        let (color, _) = resolve_text_colors(&doc, t1);
        assert_eq!(color, LINK_COLOR);
    }

    #[test]
    fn link_underline_combines_with_explicit_underline() {
        let (doc, t1) = doc! {
            root {
                paragraph {
                    t1: text("hello") [
                        link(href: "https://example.com".into()),
                        underline
                    ]
                }
            }
        };
        let d = resolve_decoration(&doc, t1);
        assert!(d.underline);
    }

    #[test]
    fn segment_single_ascii_char() {
        let spans = segment_graphemes("h", 10.0, &segmenter());
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].codepoints, 1);
        assert_eq!(spans[0].advance, 10.0);
    }

    #[test]
    fn segment_multiple_ascii_chars() {
        let spans = segment_graphemes("ab", 20.0, &segmenter());
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].codepoints, 1);
        assert_eq!(spans[0].advance, 10.0);
        assert_eq!(spans[1].codepoints, 1);
        assert_eq!(spans[1].advance, 10.0);
    }

    #[test]
    fn segment_multi_codepoint_grapheme() {
        // 👨‍👩 = U+1F468 U+200D U+1F469 (3 codepoints, 1 grapheme cluster)
        let text = "\u{1F468}\u{200D}\u{1F469}";
        let spans = segment_graphemes(text, 20.0, &segmenter());
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].codepoints, 3);
        assert_eq!(spans[0].advance, 20.0);
    }

    #[test]
    fn segment_no_zero_codepoint_span() {
        let spans = segment_graphemes("hello", 50.0, &segmenter());
        assert!(spans.iter().all(|s| s.codepoints > 0));
        assert_eq!(spans.len(), 5);
    }
}
