use std::borrow::Cow;

use editor_common::{EdgeInsets, Rect};

use crate::style::Alignment;
use editor_model::{Doc, Node, NodeRef};
use parley::setting::{FontFeature, Tag};
use parley::style::{FontFamily, FontFamilyName, FontFeatures, FontWeight, TextStyle};

use crate::glyph_run::{Glyph, GlyphRun, GraphemeSpan, Synthesis};
use crate::measure::Measurer;
use crate::measure::container::{PaddedLayoutConfig, layout_padded};
use crate::measure::text::resolve::resolve_text_style;
use crate::measure::{MeasuredContent, MeasuredNode};
use crate::style::{Decoration, DecorationData};
use crate::view_state::ViewState;

const LIST_ITEM_MARKER_WIDTH: f32 = 20.0;
const LIST_ITEM_MARKER_GAP: f32 = 8.0;

const MARKER_FONT_SIZE: f32 = 14.0;

fn resolve_marker_data(measurer: &mut Measurer, node: &NodeRef<'_>) -> DecorationData {
    let parent = match node.parent() {
        Some(p) => p,
        None => return DecorationData::Text("\u{2022}".to_string()),
    };

    match parent.node() {
        Node::OrderedList(_) => {
            let index = node.index().unwrap_or(0);
            let text = format!("{}.", index + 1);
            shape_marker_label(measurer, node, &text)
        }
        _ => DecorationData::Text("\u{2022}".to_string()),
    }
}

fn shape_marker_label(measurer: &mut Measurer, node: &NodeRef<'_>, text: &str) -> DecorationData {
    let base_style = resolve_text_style(node);

    let mut resource = measurer.resource.lock().unwrap();
    let font_id = resource.font_registry.intern(&base_style.font_family);
    let font_family_name = resource
        .font_registry
        .resolve_opt(font_id)
        .unwrap_or_default()
        .to_owned();

    let style = TextStyle {
        font_family: FontFamily::Single(FontFamilyName::Named(Cow::Owned(font_family_name))),
        font_size: MARKER_FONT_SIZE,
        font_weight: FontWeight::new(400.0),
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
            .style_run_builder(&mut resource.font_context, text, 1.0, false);
    let idx = builder.push_style(style);
    builder.push_style_run(idx, 0..text.len());
    let mut layout = builder.build(text);
    layout.break_all_lines(None);

    let mut glyph_runs = Vec::new();

    if let Some(line) = layout.lines().next() {
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
                let font_size = run.font_size();

                glyph_runs.push(GlyphRun {
                    font_id,
                    font_weight: 400,
                    font_size,
                    synthesis: Synthesis::default(),
                    color: String::new(),
                    background_color: None,
                    glyphs,
                    node_id: node.id(),
                    offset: 0,
                    text: text.to_string(),
                    x: 0.0,
                    width: run_advance,
                    graphemes: vec![GraphemeSpan {
                        advance: run_advance,
                        codepoints: text.chars().count() as u8,
                    }],
                });
            }
        }
    }

    DecorationData::Glyphs(glyph_runs)
}

pub fn measure_list_item(
    measurer: &mut Measurer,
    doc: &Doc,
    node: &NodeRef<'_>,
    width: f32,
    view_state: &ViewState,
) -> MeasuredNode {
    let padding = EdgeInsets {
        left: LIST_ITEM_MARKER_WIDTH + LIST_ITEM_MARKER_GAP,
        ..EdgeInsets::ZERO
    };

    let mut measured = layout_padded(
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
        },
    );

    if let MeasuredContent::Box(ref mut b) = measured.content {
        b.style.decorations.push(Decoration {
            id: 0,
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: LIST_ITEM_MARKER_WIDTH,
                height: LIST_ITEM_MARKER_WIDTH,
            },
            data: resolve_marker_data(measurer, node),
        });
    }

    measured
}

#[cfg(test)]
mod tests {
    use editor_macros::doc;

    use super::*;

    #[test]
    fn applies_left_indent() {
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

        assert_eq!(b.style.padding.left, 28.0);
        assert_eq!(result.width, 300.0);
    }

    #[test]
    fn ordered_list_uses_glyphs() {
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

        assert!(
            matches!(&b.style.decorations[0].data, DecorationData::Glyphs(runs) if !runs.is_empty())
        );
    }

    #[test]
    fn bullet_list_uses_bullet() {
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

        assert!(matches!(&b.style.decorations[0].data, DecorationData::Text(s) if s == "\u{2022}"));
    }
}
