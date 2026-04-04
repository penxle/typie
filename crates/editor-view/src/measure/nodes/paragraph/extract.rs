use editor_model::{Doc, Modifier, ModifierType, NodeId};
use editor_resource::TextBrush;
use parley::Layout;

use super::strut::StrutMetrics;
use super::style_run::StyleRun;
use crate::glyph_run::{Glyph, GlyphRun, Synthesis};
use crate::measure::resolve::resolve_inherited;

pub struct ExtractedLine {
    pub height: f32,
    pub baseline: f32,
    pub glyph_runs: Vec<GlyphRun>,
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

fn resolve_text_colors(doc: &Doc, node_id: NodeId) -> (String, Option<String>) {
    let color = doc
        .node(node_id)
        .and_then(|node_ref| {
            resolve_inherited(&node_ref, ModifierType::TextColor).and_then(|m| match m {
                Modifier::TextColor { value } => Some(value.clone()),
                _ => None,
            })
        })
        .unwrap_or_else(|| "text.default".to_string());

    let background_color = doc.node(node_id).and_then(|node_ref| {
        resolve_inherited(&node_ref, ModifierType::BackgroundColor).and_then(|m| match m {
            Modifier::BackgroundColor { value } => Some(value.clone()),
            _ => None,
        })
    });

    (color, background_color)
}

pub fn extract_lines(
    doc: &Doc,
    text: &str,
    layout: &Layout<TextBrush>,
    style_runs: &[StyleRun],
    strut: &StrutMetrics,
    line_height_ratio: f32,
    base_font_size: f32,
) -> Vec<ExtractedLine> {
    let mut lines = Vec::new();

    for line in layout.lines() {
        let metrics = line.metrics();

        let ascent = metrics.ascent.max(strut.ascent);
        let descent = metrics.descent.max(strut.descent);
        let content_height = ascent + descent;

        let line_box_height = (base_font_size * line_height_ratio).max(content_height);
        let leading = (line_box_height - content_height).max(0.0);
        let baseline = leading / 2.0 + ascent;

        let mut glyph_runs = Vec::new();
        let mut x = metrics.offset;

        for item in line.items() {
            match item {
                parley::PositionedLayoutItem::GlyphRun(glyph_run) => {
                    let run = glyph_run.run();
                    let font_size = run.font_size();

                    // Capture glyph positions (line-relative)
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

                    // Compute char_advances from visual clusters
                    let mut run_first = true;

                    for cluster in run.visual_clusters() {
                        let node_id = cluster.first_style().brush.node_id;
                        let cluster_range = cluster.text_range();
                        let cluster_text = &text[cluster_range.clone()];
                        let advance = cluster.advance();

                        let char_count = cluster_text.chars().count();
                        let per_char = if char_count > 0 {
                            advance / char_count as f32
                        } else {
                            0.0
                        };

                        let extend = !run_first
                            && glyph_runs
                                .last()
                                .map(|gr: &GlyphRun| gr.node_id == node_id)
                                .unwrap_or(false);

                        if extend {
                            let gr: &mut GlyphRun = glyph_runs.last_mut().unwrap();
                            gr.text.push_str(cluster_text);
                            gr.width += advance;
                            for _ in 0..char_count {
                                gr.char_advances.push(per_char);
                            }
                        } else {
                            let byte_start = cluster_range.start;
                            let char_offset = text[..byte_start].chars().count();
                            let mut char_advances = Vec::with_capacity(char_count);
                            for _ in 0..char_count {
                                char_advances.push(per_char);
                            }

                            let synthesis = resolve_synthesis(doc, node_id);
                            let (color, background_color) = resolve_text_colors(doc, node_id);

                            let (font_id, font_weight) = style_runs
                                .iter()
                                .find(|sr| sr.byte_range.contains(&byte_start))
                                .map(|sr| (sr.family, sr.weight))
                                .unwrap_or((0, 400));

                            glyph_runs.push(GlyphRun {
                                font_id,
                                font_weight,
                                font_size,
                                synthesis,
                                color,
                                background_color,
                                glyphs: glyphs.clone(),
                                node_id,
                                offset: char_offset,
                                text: cluster_text.to_string(),
                                x,
                                width: advance,
                                char_advances,
                            });
                        }

                        run_first = false;
                        x += advance;
                    }
                }
                _ => {}
            }
        }

        lines.push(ExtractedLine {
            height: line_box_height,
            baseline,
            glyph_runs,
        });
    }

    lines
}
