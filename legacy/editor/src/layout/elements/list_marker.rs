use crate::global::GLOBALS;
use crate::model::{LIST_ITEM_MARKER_GAP, NodeId};
use crate::render::sink::RenderSink;
use crate::render::{Render, RenderParams, RenderPhase, glyph::Glyph};
use crate::types::theme::Color;
use kurbo::{Affine, Rect};
use parley::setting::{FontFeature, Tag};
use parley::style::{FontFamily, FontFamilyName, FontFeatures, StyleProperty};
use peniko::Brush;
use std::fmt;

const MARKER_FONT_SIZE: f32 = 14.0;
const BULLET_SIZE: f32 = 4.0;
const BULLET_OFFSET: f32 = 4.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListMarkerType {
    Bullet,
    Ordered(usize),
}

#[derive(Clone, PartialEq)]
pub struct ListMarkerElement {
    pub marker_type: ListMarkerType,
    pub baseline: f32,
    pub line_mid: f32,
    pub marker_width: f32,
    pub selection_node_id: NodeId,
    pub selection_width: f32,
    pub selection_height: f32,
}

impl fmt::Debug for ListMarkerElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListMarkerElement")
            .field("marker_type", &self.marker_type)
            .field("baseline", &self.baseline)
            .field("line_mid", &self.line_mid)
            .field("marker_width", &self.marker_width)
            .field("selection_node_id", &self.selection_node_id)
            .field("selection_width", &self.selection_width)
            .field("selection_height", &self.selection_height)
            .finish()
    }
}

impl ListMarkerElement {
    pub fn new(
        marker_type: ListMarkerType,
        baseline: f32,
        line_mid: f32,
        marker_width: f32,
        selection_node_id: NodeId,
        selection_width: f32,
        selection_height: f32,
    ) -> Self {
        Self {
            marker_type,
            baseline,
            line_mid,
            marker_width,
            selection_node_id,
            selection_width,
            selection_height,
        }
    }
}

impl Render for ListMarkerElement {
    fn render(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        self.paint_to(sink, transform, ctx);
    }
}

impl ListMarkerElement {
    fn paint_to(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        match ctx.phase {
            RenderPhase::Content => {
                let color = ctx.theme.color("ui.text.default");
                match &self.marker_type {
                    ListMarkerType::Bullet => self.render_bullet(sink, transform, color),
                    ListMarkerType::Ordered(index) => {
                        self.render_ordered_marker(*index, sink, transform, ctx, color);
                    }
                }
            }
            RenderPhase::Selection => {
                self.render_selection_background(sink, transform, ctx);
            }
            _ => {}
        }
    }

    fn render_selection_background(
        &self,
        sink: &mut dyn RenderSink,
        transform: Affine,
        ctx: &RenderParams<'_>,
    ) {
        let Some(rect) = self.selection_background_rect(ctx) else {
            return;
        };

        let brush = ctx.selection_paint();
        sink.fill_rect(rect, &brush, transform);
    }

    fn selection_background_rect(&self, ctx: &RenderParams<'_>) -> Option<Rect> {
        if !ctx.is_block_selected(self.selection_node_id) {
            return None;
        }

        let width = self
            .selection_width
            .max(self.marker_width + LIST_ITEM_MARKER_GAP);
        Some(Rect::new(
            0.0,
            0.0,
            width as f64,
            self.selection_height as f64,
        ))
    }

    fn render_bullet(&self, sink: &mut dyn RenderSink, transform: Affine, color: Color) {
        let brush = Brush::Solid(color);

        let x = self.marker_width - BULLET_SIZE - BULLET_OFFSET;
        let y = self.line_mid - BULLET_SIZE / 2.0;
        let rect = Rect::new(
            x as f64,
            y as f64,
            (x + BULLET_SIZE) as f64,
            (y + BULLET_SIZE) as f64,
        );

        sink.fill_rect(rect, &brush, transform);
    }

    fn render_ordered_marker(
        &self,
        index: usize,
        sink: &mut dyn RenderSink,
        transform: Affine,
        ctx: &RenderParams,
        color: Color,
    ) {
        let text = format!("{}.", index);
        let scale = ctx.scale_factor as f32;

        let brush = Brush::Solid(color);

        GLOBALS.with(|globals| {
            let globals = globals.borrow();
            let mut lcx = globals.parley_layout_context.borrow_mut();
            let mut fcx = globals.parley_font_context.borrow_mut();

            let mut builder = lcx.ranged_builder(&mut fcx, &text, 1.0, false);

            builder.push_default(StyleProperty::FontSize(MARKER_FONT_SIZE));
            builder.push_default(StyleProperty::FontFamily(FontFamily::Single(
                FontFamilyName::Named(ctx.doc.default_attrs().font_family().into()),
            )));
            builder.push_default(StyleProperty::FontFeatures(FontFeatures::List(
                std::borrow::Cow::Borrowed(&[FontFeature {
                    tag: Tag::new(b"tnum"),
                    value: 1,
                }]),
            )));

            let mut layout = builder.build(&text);
            layout.break_all_lines(None);

            if let Some(line) = layout.lines().next() {
                for item in line.items() {
                    if let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                        let run = glyph_run.run();
                        let run_x = glyph_run.offset();

                        let mut glyph_data = Vec::new();
                        let mut x_advance = 0.0;
                        let mut run_width: f32 = 0.0;

                        for g in glyph_run.glyphs() {
                            let glyph_x = x_advance + g.x;
                            run_width = run_width.max(glyph_x + g.advance);
                            x_advance += g.advance;
                            glyph_data.push((g.id, glyph_x, g.y));
                        }

                        let align_offset = self.marker_width - run_x - run_width;
                        let text_range = run.text_range();

                        let run_text = if text_range.start < text_range.end {
                            text.get(text_range).unwrap_or("")
                        } else {
                            ""
                        };

                        let glyphs: Vec<_> = glyph_data
                            .into_iter()
                            .map(|(id, glyph_x, glyph_y)| Glyph {
                                id,
                                x: run_x + align_offset + glyph_x,
                                y: self.baseline + glyph_y,
                            })
                            .collect();

                        sink.draw_text(
                            run_text,
                            &run.font(),
                            run.font_size() * scale,
                            &brush,
                            transform,
                            None,
                            false,
                            &glyphs,
                        );
                    }
                }
            }
        });
    }
}
