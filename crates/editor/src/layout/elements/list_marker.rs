use crate::global::GLOBALS;
use crate::render::{GlyphRenderer, Render, RenderContext, glyph::Glyph};
use parley::style::{FontFamily, FontStack, StyleProperty};
use parley::swash;
use std::fmt;
use tiny_skia::{Paint, PixmapMut, Transform};

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
}

impl fmt::Debug for ListMarkerElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListMarkerElement")
            .field("marker_type", &self.marker_type)
            .field("baseline", &self.baseline)
            .field("line_mid", &self.line_mid)
            .field("marker_width", &self.marker_width)
            .finish()
    }
}

impl ListMarkerElement {
    pub fn new(
        marker_type: ListMarkerType,
        baseline: f32,
        line_mid: f32,
        marker_width: f32,
    ) -> Self {
        Self {
            marker_type,
            baseline,
            line_mid,
            marker_width,
        }
    }
}

impl Render for ListMarkerElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext<'_>,
    ) {
        match &self.marker_type {
            ListMarkerType::Bullet => {
                self.render_bullet(pixmap, transform, ctx);
            }
            ListMarkerType::Ordered(index) => {
                self.render_ordered_marker(*index, pixmap, glyph_renderer, transform, ctx);
            }
        }
    }
}

impl ListMarkerElement {
    fn render_bullet(&self, pixmap: &mut PixmapMut, transform: Transform, ctx: &RenderContext) {
        let color = ctx.theme.color("ui.text.default");
        let paint = Paint {
            shader: tiny_skia::Shader::SolidColor(color),
            anti_alias: true,
            ..Paint::default()
        };

        let x = self.marker_width - BULLET_SIZE - BULLET_OFFSET;
        let y = self.line_mid - BULLET_SIZE / 2.0;
        let rect = tiny_skia::Rect::from_xywh(x, y, BULLET_SIZE, BULLET_SIZE).unwrap();

        pixmap.fill_rect(rect, &paint, transform, None);
    }

    fn render_ordered_marker(
        &self,
        index: usize,
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        let text = format!("{}.", index);
        let scale = ctx.scale_factor as f32;

        let color = ctx.theme.color("ui.text.default");
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;

        GLOBALS.with(|globals| {
            let globals = globals.borrow();
            let mut lcx = globals.parley_layout_context.borrow_mut();
            let mut fcx = globals.parley_font_context.borrow_mut();

            let mut builder = lcx.ranged_builder(&mut fcx, &text, 1.0, false);

            builder.push_default(StyleProperty::FontSize(MARKER_FONT_SIZE));
            builder.push_default(StyleProperty::FontStack(FontStack::Single(
                FontFamily::Named(ctx.doc.default_attrs().font_family().into()),
            )));
            builder.push_default(StyleProperty::FontFeatures(
                parley::style::FontSettings::List(std::borrow::Cow::Borrowed(&[swash::Setting {
                    tag: swash::tag_from_bytes(b"tnum"),
                    value: 1,
                }])),
            ));

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

                        let glyphs: Vec<_> = glyph_data
                            .into_iter()
                            .map(|(id, glyph_x, glyph_y)| Glyph {
                                id,
                                x: run_x + align_offset + glyph_x,
                                y: self.baseline + glyph_y,
                            })
                            .collect();

                        glyph_renderer.draw_glyphs(
                            pixmap,
                            &run.font(),
                            run.font_size() * scale,
                            &paint,
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
