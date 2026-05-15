use std::borrow::Cow;

use editor_common::{EdgeInsets, Rect};

use crate::style::Alignment;
use editor_model::{Doc, Node, NodeRef};
use parley::setting::{FontFeature, Tag};
use parley::style::{FontFamily, FontFamilyName, FontFeatures, FontWeight, LineHeight, TextStyle};

use crate::glyph_run::{Glyph, GlyphRun, GraphemeSpan, Synthesis, TextDecoration};
use crate::measure::Measurer;
use crate::measure::container::{PaddedLayoutConfig, layout_padded};
use crate::measure::text::resolve::{ResolvedTextStyle, resolve_text_style};
use crate::measure::text::strut::compute_strut;
use crate::measure::{MeasuredContent, MeasuredNode};
use crate::style::{Decoration, DecorationData};
use crate::view_state::ViewState;

use super::line_geometry::{LineStrutExpansion, expand_first_line, first_line_info};

const MARKER_RECT_MIN_RATIO: f32 = 1.25;
const MARKER_OUTER_GAP_RATIO: f32 = 0.5;

pub fn measure_list_item(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> MeasuredNode {
    let base_style = resolve_text_style(node);
    let marker_font_size = list_item_max_font_size(node, &base_style);
    // Only font_size is bumped: a stylized descendant text must not change the marker's typeface.
    let marker_style = ResolvedTextStyle {
        font_size: marker_font_size,
        ..base_style.clone()
    };

    let marker_strut = {
        let mut resource = measurer.resource.lock().unwrap();
        compute_strut(&mut resource, &marker_style)
    };
    let (marker_ascent, marker_descent) = marker_strut
        .as_ref()
        .map(|s| (s.ascent, s.descent))
        .unwrap_or((marker_font_size * 0.8, marker_font_size * 0.2));

    let marker_shape = shape_marker(measurer, node, &marker_style);

    let measured_glyph_width = marker_shape
        .as_ref()
        .map(|s| s.glyph_runs.iter().map(|r| r.width).sum::<f32>())
        .unwrap_or(0.0);
    let marker_rect_width = measured_glyph_width.max(marker_font_size * MARKER_RECT_MIN_RATIO);
    let outer_gap = marker_font_size * MARKER_OUTER_GAP_RATIO;

    let padding = EdgeInsets {
        left: marker_rect_width + outer_gap,
        ..EdgeInsets::ZERO
    };

    let measured = layout_padded(
        measurer,
        doc,
        node,
        width,
        view_state,
        PaddedLayoutConfig {
            padding,
            border: EdgeInsets::ZERO,
            scope: false,
            alignment: Alignment::Start,
            // padding.left is the bullet marker slot, not a visual envelope —
            // selection rendering must not include it in a block rect.
            is_visual_container: false,
        },
    );

    let expansion = LineStrutExpansion {
        ascent: marker_ascent,
        descent: marker_descent,
        min_line_height: marker_font_size * marker_style.line_height,
    };
    let (mut measured, line_top, line_height, line_baseline) =
        match expand_first_line(&measured, &expansion) {
            Some(e) => (e.tree, e.top, e.height, e.baseline),
            None => {
                let fallback_height = (marker_font_size * marker_style.line_height)
                    .max(marker_ascent + marker_descent);
                let fallback_baseline =
                    (fallback_height - (marker_ascent + marker_descent)).max(0.0) / 2.0
                        + marker_ascent;
                let top = first_line_info(&measured).map(|i| i.top).unwrap_or(0.0);
                (measured, top, fallback_height, fallback_baseline)
            }
        };

    let marker_data = match marker_shape {
        Some(shape) => apply_baseline(shape.glyph_runs, line_baseline),
        None => DecorationData::Bullet,
    };

    if let MeasuredContent::Box(ref mut b) = measured.content {
        b.style.decorations.push(Decoration {
            id: 0,
            rect: Rect {
                x: 0.0,
                y: line_top,
                width: marker_rect_width,
                height: line_height,
            },
            data: marker_data,
        });
    }

    measured
}

fn list_item_max_font_size(node: &NodeRef<'_>, base: &ResolvedTextStyle) -> f32 {
    let mut max = base.font_size;
    for desc in node.descendants() {
        if matches!(desc.node(), Node::Text(_)) {
            let fs = resolve_text_style(&desc).font_size;
            if fs > max {
                max = fs;
            }
        }
    }
    max
}

struct MarkerShape {
    // glyph y is baseline-relative; caller adds the host line's baseline.
    glyph_runs: Vec<GlyphRun>,
}

fn shape_marker(
    measurer: &mut Measurer,
    node: &NodeRef<'_>,
    style: &ResolvedTextStyle,
) -> Option<MarkerShape> {
    let parent = node.parent()?;
    let text = match parent.node() {
        Node::OrderedList(_) => format!("{}.", node.index().unwrap_or(0) + 1),
        // Bullets are rendered as a path, not as a shaped glyph.
        _ => return None,
    };

    let mut resource = measurer.resource.lock().unwrap();
    let font_id = resource.font_registry.intern(&style.font_family);
    let font_family_name = resource
        .font_registry
        .family_name_opt(font_id)
        .unwrap_or_default()
        .to_owned();

    let parley_style = TextStyle {
        font_family: FontFamily::Single(FontFamilyName::Named(Cow::Owned(font_family_name))),
        font_size: style.font_size,
        line_height: LineHeight::FontSizeRelative(style.line_height),
        font_weight: FontWeight::new(style.font_weight as f32),
        font_features: FontFeatures::List(Cow::Borrowed(&[FontFeature {
            tag: Tag::new(b"tnum"),
            value: 1,
        }])),
        ..TextStyle::default()
    };

    let resource = &mut *resource;
    let mut builder =
        resource
            .layout_context
            .style_run_builder(&mut resource.font_context, &text, 1.0, false);
    let idx = builder.push_style(parley_style);
    builder.push_style_run(idx, 0..text.len());
    let mut layout = builder.build(&text);
    layout.break_all_lines(None);

    let mut glyph_runs = Vec::new();
    let line = layout.lines().next()?;
    for item in line.items() {
        if let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item {
            let run = glyph_run.run();
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
                        y: g.y,
                    }
                })
                .collect();

            let run_advance = glyph_run.advance();
            glyph_runs.push(GlyphRun {
                family_id: font_id,
                weight: style.font_weight,
                font_size: run.font_size(),
                synthesis: Synthesis::default(),
                color: String::new(),
                background_color: None,
                glyphs,
                decoration: TextDecoration::default(),
                node_id: node.id(),
                offset: 0,
                text: text.clone(),
                x: 0.0,
                width: run_advance,
                graphemes: vec![GraphemeSpan {
                    advance: run_advance,
                    codepoints: text.chars().count() as u8,
                }],
            });
        }
    }

    Some(MarkerShape { glyph_runs })
}

fn apply_baseline(mut glyph_runs: Vec<GlyphRun>, baseline: f32) -> DecorationData {
    for run in &mut glyph_runs {
        for g in &mut run.glyphs {
            g.y += baseline;
        }
    }
    DecorationData::Glyphs(glyph_runs)
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;

    #[test]
    fn applies_left_indent_with_default_font() {
        let (doc, li1) = doc! {
            root {
                bullet_list {
                    li1: list_item {
                        paragraph
                    }
                }
            }
        };

        let node = doc.node(li1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_list_item(&mut measurer, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        // 16px * (MARKER_RECT_MIN_RATIO + MARKER_OUTER_GAP_RATIO) = 16 * 1.75 = 28.
        assert!(
            (b.style.padding.left - 28.0).abs() < 0.01,
            "expected padding.left ≈ 28, got {}",
            b.style.padding.left,
        );
        assert_eq!(result.width, 300.0);
    }

    #[test]
    fn ordered_list_emits_glyph_runs() {
        let (doc, li1) = doc! {
            root {
                ordered_list {
                    li1: list_item {
                        paragraph { text("first") }
                    }
                }
            }
        };

        let node = doc.node(li1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_list_item(&mut measurer, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        assert!(matches!(
            &b.style.decorations[0].data,
            DecorationData::Glyphs(runs) if !runs.is_empty(),
        ));
    }

    #[test]
    fn bullet_decoration_rect_covers_first_line() {
        // Renderer centers the bullet within the marker rect; rect must match first line bounds.
        let (doc, li1) = doc! {
            root {
                bullet_list {
                    li1: list_item {
                        paragraph { text("item") }
                    }
                }
            }
        };

        let node = doc.node(li1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_list_item(&mut measurer, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };
        let MeasuredContent::Box(ref paragraph) = b.children[0].content else {
            panic!()
        };
        let first_line_height = paragraph.children[0].height;
        let marker = b.style.decorations.first().expect("marker decoration");

        assert!(matches!(marker.data, DecorationData::Bullet));
        assert_eq!(marker.rect.y, 0.0);
        assert!(
            (marker.rect.height - first_line_height).abs() < 0.01,
            "marker rect height {} should match first line height {}",
            marker.rect.height,
            first_line_height,
        );
    }

    #[test]
    fn bullet_list_uses_bullet_data() {
        let (doc, li1) = doc! {
            root {
                bullet_list {
                    li1: list_item {
                        paragraph { text("item") }
                    }
                }
            }
        };

        let node = doc.node(li1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_list_item(&mut measurer, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        assert!(matches!(
            &b.style.decorations[0].data,
            DecorationData::Bullet
        ));
    }

    #[test]
    fn marker_glyph_y_anchored_to_first_line_baseline() {
        let (doc, li1) = doc! {
            root {
                ordered_list {
                    li1: list_item {
                        paragraph { text("hello") }
                    }
                }
            }
        };

        let node = doc.node(li1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_list_item(&mut measurer, &doc, &node, 300.0, &ViewState::new());
        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };

        let MeasuredContent::Box(ref paragraph) = b.children[0].content else {
            panic!("first child should be a paragraph box");
        };
        let MeasuredContent::Line(first_line) = &paragraph.children[0].content else {
            panic!("first line should be a Line");
        };
        let line_baseline = first_line.baseline;

        let marker = b.style.decorations.first().expect("marker decoration");
        let DecorationData::Glyphs(runs) = &marker.data else {
            panic!("ordered marker should be glyphs");
        };
        for run in runs {
            for g in &run.glyphs {
                assert!(
                    (g.y - line_baseline).abs() < 0.5,
                    "marker glyph y {} should track line baseline {}",
                    g.y,
                    line_baseline,
                );
            }
        }
    }

    #[test]
    fn marker_rect_width_contains_measured_glyphs_at_large_font() {
        let (doc, li1) = doc! {
            root {
                ordered_list {
                    li1: list_item {
                        paragraph { text("123412341234") [font_size(9600)] }
                    }
                }
            }
        };

        let node = doc.node(li1).unwrap();
        let mut measurer = Measurer::new_test();
        let result = measure_list_item(&mut measurer, &doc, &node, 1200.0, &ViewState::new());
        let MeasuredContent::Box(ref b) = result.content else {
            panic!()
        };
        let marker = b.style.decorations.first().expect("marker decoration");
        let DecorationData::Glyphs(runs) = &marker.data else {
            panic!("ordered marker should be glyphs");
        };
        let total: f32 = runs.iter().map(|r| r.width).sum();
        assert!(
            total <= marker.rect.width + 0.01,
            "measured marker width {} must fit inside marker rect width {}",
            total,
            marker.rect.width,
        );
    }

    #[test]
    fn marker_size_tracks_largest_text_in_any_paragraph() {
        let (doc, li_small_first) = doc! {
            root {
                ordered_list {
                    li_small_first: list_item {
                        paragraph { text("small") }
                        paragraph { text("BIG") [font_size(4800)] }
                    }
                }
            }
        };
        let (doc2, li_only_small) = doc! {
            root {
                ordered_list {
                    li_only_small: list_item {
                        paragraph { text("small") }
                    }
                }
            }
        };

        let mut measurer = Measurer::new_test();
        let res_mixed = measure_list_item(
            &mut measurer,
            &doc,
            &doc.node(li_small_first).unwrap(),
            600.0,
            &ViewState::new(),
        );
        let res_small = measure_list_item(
            &mut measurer,
            &doc2,
            &doc2.node(li_only_small).unwrap(),
            600.0,
            &ViewState::new(),
        );
        let MeasuredContent::Box(ref b_mixed) = res_mixed.content else {
            panic!()
        };
        let MeasuredContent::Box(ref b_small) = res_small.content else {
            panic!()
        };
        assert!(
            b_mixed.style.padding.left > b_small.style.padding.left,
            "padding (= marker_rect_width + gap) should grow with the largest descendant \
             font_size: mixed={} small={}",
            b_mixed.style.padding.left,
            b_small.style.padding.left,
        );
    }

    #[test]
    fn first_line_expands_to_house_large_marker() {
        let (doc, li1) = doc! {
            root {
                ordered_list {
                    li1: list_item {
                        paragraph { text("small") }
                        paragraph { text("BIG") [font_size(4800)] }
                    }
                }
            }
        };
        let (doc2, li_only_small) = doc! {
            root {
                ordered_list {
                    li_only_small: list_item {
                        paragraph { text("small") }
                    }
                }
            }
        };

        let mut measurer = Measurer::new_test();
        let res = measure_list_item(
            &mut measurer,
            &doc,
            &doc.node(li1).unwrap(),
            600.0,
            &ViewState::new(),
        );
        let res_small = measure_list_item(
            &mut measurer,
            &doc2,
            &doc2.node(li_only_small).unwrap(),
            600.0,
            &ViewState::new(),
        );

        let MeasuredContent::Box(ref b) = res.content else {
            panic!()
        };
        let MeasuredContent::Box(ref paragraph) = b.children[0].content else {
            panic!()
        };
        let mixed_first_line_h = paragraph.children[0].height;

        let MeasuredContent::Box(ref b_small) = res_small.content else {
            panic!()
        };
        let MeasuredContent::Box(ref paragraph_small) = b_small.children[0].content else {
            panic!()
        };
        let small_first_line_h = paragraph_small.children[0].height;

        assert!(
            mixed_first_line_h > small_first_line_h,
            "first line in the mixed-font list item must grow to fit the marker \
             (mixed={mixed_first_line_h}, small={small_first_line_h})",
        );
    }
}
