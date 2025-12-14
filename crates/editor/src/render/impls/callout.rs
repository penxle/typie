use crate::layout::elements::{CalloutBackgroundElement, CalloutIconElement};
use crate::model::CalloutType;
use crate::model::{CALLOUT_BORDER_RADIUS, CALLOUT_BORDER_WIDTH};
use crate::render::{GlyphRenderer, Render, RenderContext};
use tiny_skia::{Color, Paint, PathBuilder, PixmapMut, Stroke, Transform};

const ICON_SIZE: f32 = 20.0;
const ICON_STROKE_WIDTH: f32 = 1.5;

impl Render for CalloutBackgroundElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        _glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        _ctx: &RenderContext,
    ) {
        let (r, g, b) = self.callout_type.color();
        let border_color = Color::from_rgba8(r, g, b, 255);

        let bg_color = Color::from_rgba8(r, g, b, 8); // ~3% of 255

        let mut bg_paint = Paint::default();
        bg_paint.set_color(bg_color);
        bg_paint.anti_alias = true;

        if let Some(path) = build_rounded_rect(
            0.0,
            0.0,
            self.size.width,
            self.size.height,
            CALLOUT_BORDER_RADIUS,
        ) {
            pixmap.fill_path(
                &path,
                &bg_paint,
                tiny_skia::FillRule::Winding,
                transform,
                None,
            );
        }

        let mut border_paint = Paint::default();
        border_paint.set_color(border_color);
        border_paint.anti_alias = true;

        let stroke = Stroke {
            width: CALLOUT_BORDER_WIDTH,
            ..Stroke::default()
        };

        let mb = CALLOUT_BORDER_WIDTH / 2.0;
        if let Some(path) = build_rounded_rect(
            mb,
            mb,
            self.size.width - CALLOUT_BORDER_WIDTH,
            self.size.height - CALLOUT_BORDER_WIDTH,
            CALLOUT_BORDER_RADIUS - mb,
        ) {
            pixmap.stroke_path(&path, &border_paint, &stroke, transform, None);
        }
    }
}

impl Render for CalloutIconElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        _glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        _ctx: &RenderContext,
    ) {
        let (r, g, b) = self.callout_type.color();
        let icon_color = Color::from_rgba8(r, g, b, 255);

        let mut icon_paint = Paint::default();
        icon_paint.set_color(icon_color);
        icon_paint.anti_alias = true;

        let icon_stroke = Stroke {
            width: ICON_STROKE_WIDTH,
            line_cap: tiny_skia::LineCap::Round,
            line_join: tiny_skia::LineJoin::Round,
            ..Stroke::default()
        };

        let cx = self.size.width / 2.0;
        let cy = self.size.height / 2.0;

        if let Some(path) = build_icon(self.callout_type, cx, cy, ICON_SIZE) {
            pixmap.stroke_path(&path, &icon_paint, &icon_stroke, transform, None);
        }
    }
}

fn build_rounded_rect(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();

    pb.move_to(x + radius, y);
    pb.line_to(x + width - radius, y);
    pb.quad_to(x + width, y, x + width, y + radius);
    pb.line_to(x + width, y + height - radius);
    pb.quad_to(x + width, y + height, x + width - radius, y + height);
    pb.line_to(x + radius, y + height);
    pb.quad_to(x, y + height, x, y + height - radius);
    pb.line_to(x, y + radius);
    pb.quad_to(x, y, x + radius, y);
    pb.close();

    pb.finish()
}

// TODO: svg
fn build_icon(callout_type: CalloutType, cx: f32, cy: f32, size: f32) -> Option<tiny_skia::Path> {
    let r = size / 2.0;
    let mut pb = PathBuilder::new();

    match callout_type {
        CalloutType::Info => {
            draw_circle(&mut pb, cx, cy, r);

            // Draw "i" - dot and line
            pb.move_to(cx, cy - r * 0.4);
            pb.line_to(cx, cy - r * 0.35);
            pb.move_to(cx, cy - r * 0.1);
            pb.line_to(cx, cy + r * 0.4);
        }
        CalloutType::Success => {
            draw_circle(&mut pb, cx, cy, r);

            // Draw checkmark
            let check_size = r * 0.5;
            pb.move_to(cx - check_size * 0.6, cy);
            pb.line_to(cx - check_size * 0.1, cy + check_size * 0.4);
            pb.line_to(cx + check_size * 0.6, cy - check_size * 0.4);
        }
        CalloutType::Warning => {
            draw_circle(&mut pb, cx, cy, r);

            // Draw "!"
            pb.move_to(cx, cy - r * 0.4);
            pb.line_to(cx, cy + r * 0.15);
            pb.move_to(cx, cy + r * 0.35);
            pb.line_to(cx, cy + r * 0.4);
        }
        CalloutType::Danger => {
            let tri_h = r * 1.8;
            let tri_w = tri_h * 1.1;

            pb.move_to(cx, cy - tri_h * 0.45);
            pb.line_to(cx + tri_w * 0.5, cy + tri_h * 0.45);
            pb.line_to(cx - tri_w * 0.5, cy + tri_h * 0.45);
            pb.close();

            // Draw "!"
            pb.move_to(cx, cy - r * 0.2);
            pb.line_to(cx, cy + r * 0.15);
            pb.move_to(cx, cy + r * 0.3);
            pb.line_to(cx, cy + r * 0.35);
        }
    }

    pb.finish()
}

fn draw_circle(pb: &mut PathBuilder, cx: f32, cy: f32, r: f32) {
    // Approximate circle with bezier curves
    let k = 0.5522847498; // Magic number for bezier circle approximation
    let kr = k * r;

    pb.move_to(cx + r, cy);
    pb.cubic_to(cx + r, cy + kr, cx + kr, cy + r, cx, cy + r);
    pb.cubic_to(cx - kr, cy + r, cx - r, cy + kr, cx - r, cy);
    pb.cubic_to(cx - r, cy - kr, cx - kr, cy - r, cx, cy - r);
    pb.cubic_to(cx + kr, cy - r, cx + r, cy - kr, cx + r, cy);
}
