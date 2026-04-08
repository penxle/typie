use crate::layout::elements::SplitEdges;
use crate::layout::elements::{CalloutBackgroundElement, CalloutIconElement};
use crate::model::CalloutVariant;
use crate::model::{CALLOUT_BORDER_RADIUS, CALLOUT_BORDER_WIDTH};
use crate::render::outline::ElementSink;
use crate::render::{GlyphRenderer, Outline, RasterSink, Render, RenderContext, RenderPhase};
use macros::svg_icon_path;
use tiny_skia::{Paint, PathBuilder, PixmapMut, Stroke, Transform};

const ICON_SIZE: f32 = 20.0;
const ICON_STROKE_WIDTH: f32 = 1.5;

impl Render for CalloutBackgroundElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        let mut sink = RasterSink::new(pixmap, glyph_renderer);
        self.paint_to(&mut sink, transform, ctx);
    }
}

impl Outline for CalloutBackgroundElement {
    fn outline(&self, sink: &mut dyn ElementSink, transform: Transform, ctx: &RenderContext<'_>) {
        self.paint_to(sink, transform, ctx);
    }
}

impl CalloutBackgroundElement {
    fn paint_to(&self, sink: &mut dyn ElementSink, transform: Transform, ctx: &RenderContext<'_>) {
        let color_key = format!("ui.callout.{}", self.variant);
        let is_selected = ctx.is_block_selected(self.node_id);

        match ctx.phase {
            RenderPhase::Background => {
                let bg_color = ctx.theme.color_with_alpha(&color_key, 8);

                let mut bg_paint = Paint::default();
                bg_paint.set_color(bg_color);
                bg_paint.anti_alias = true;

                let (tl, tr, br, bl) = corner_radii(CALLOUT_BORDER_RADIUS, &self.split_edges);

                if let Some(path) = build_rounded_rect_corners(
                    0.0,
                    0.0,
                    self.size.width,
                    self.size.height,
                    tl,
                    tr,
                    br,
                    bl,
                ) {
                    sink.fill_path(&path, &bg_paint, tiny_skia::FillRule::Winding, transform);
                }
            }
            RenderPhase::Content => {
                let border_color = ctx.theme.color(&color_key);
                let mut border_paint = Paint::default();
                border_paint.set_color(border_color);
                border_paint.anti_alias = true;

                let stroke = Stroke {
                    width: CALLOUT_BORDER_WIDTH,
                    ..Stroke::default()
                };

                let mb = CALLOUT_BORDER_WIDTH / 2.0;
                let inner_radius = (CALLOUT_BORDER_RADIUS - mb).max(0.0);
                let (tl_inner, tr_inner, br_inner, bl_inner) =
                    corner_radii(inner_radius, &self.split_edges);

                if let Some(path) = build_partial_border(
                    mb,
                    mb,
                    self.size.width - CALLOUT_BORDER_WIDTH,
                    self.size.height - CALLOUT_BORDER_WIDTH,
                    tl_inner,
                    tr_inner,
                    br_inner,
                    bl_inner,
                    &self.split_edges,
                ) {
                    sink.stroke_path(&path, &border_paint, &stroke, transform);
                }
            }
            RenderPhase::Selection => {
                if !is_selected {
                    return;
                }

                let selection_paint = ctx.selection_paint();

                let (tl, tr, br, bl) = corner_radii(CALLOUT_BORDER_RADIUS, &self.split_edges);
                if let Some(path) = build_rounded_rect_corners(
                    0.0,
                    0.0,
                    self.size.width,
                    self.size.height,
                    tl,
                    tr,
                    br,
                    bl,
                ) {
                    sink.fill_path(
                        &path,
                        &selection_paint,
                        tiny_skia::FillRule::Winding,
                        transform,
                    );
                }
            }
        }
    }
}

impl Render for CalloutIconElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        let mut sink = RasterSink::new(pixmap, glyph_renderer);
        self.paint_to(&mut sink, transform, ctx);
    }
}

impl Outline for CalloutIconElement {
    fn outline(&self, sink: &mut dyn ElementSink, transform: Transform, ctx: &RenderContext<'_>) {
        self.paint_to(sink, transform, ctx);
    }
}

impl CalloutIconElement {
    fn paint_to(&self, sink: &mut dyn ElementSink, transform: Transform, ctx: &RenderContext<'_>) {
        if let RenderPhase::Content = ctx.phase {
            let color_key = format!("ui.callout.{}", self.variant);
            let icon_color = ctx.theme.color(&color_key);

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

            let path = match self.variant {
                CalloutVariant::Info => svg_icon_path!("lucide/info", ICON_SIZE, cx, cy),
                CalloutVariant::Success => svg_icon_path!("lucide/circle-check", ICON_SIZE, cx, cy),
                CalloutVariant::Warning => svg_icon_path!("lucide/circle-alert", ICON_SIZE, cx, cy),
                CalloutVariant::Danger => {
                    svg_icon_path!("lucide/triangle-alert", ICON_SIZE, cx, cy)
                }
            };

            if let Some(path) = path {
                sink.stroke_path(&path, &icon_paint, &icon_stroke, transform);
            }
        }
    }
}

fn corner_radii(radius: f32, split: &SplitEdges) -> (f32, f32, f32, f32) {
    let top_radius = if split.top { 0.0 } else { radius };
    let bottom_radius = if split.bottom { 0.0 } else { radius };
    (top_radius, top_radius, bottom_radius, bottom_radius)
}

fn build_rounded_rect_corners(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    top_left: f32,
    top_right: f32,
    bottom_right: f32,
    bottom_left: f32,
) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();

    pb.move_to(x + top_left, y);
    pb.line_to(x + width - top_right, y);
    if top_right > 0.0 {
        pb.quad_to(x + width, y, x + width, y + top_right);
    }
    pb.line_to(x + width, y + height - bottom_right);
    if bottom_right > 0.0 {
        pb.quad_to(x + width, y + height, x + width - bottom_right, y + height);
    }
    pb.line_to(x + bottom_left, y + height);
    if bottom_left > 0.0 {
        pb.quad_to(x, y + height, x, y + height - bottom_left);
    }
    pb.line_to(x, y + top_left);
    if top_left > 0.0 {
        pb.quad_to(x, y, x + top_left, y);
    }
    pb.close();

    pb.finish()
}

fn build_partial_border(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    top_left: f32,
    top_right: f32,
    bottom_right: f32,
    bottom_left: f32,
    split: &SplitEdges,
) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();

    if split.top && split.bottom {
        pb.move_to(x, y);
        pb.line_to(x, y + height);
        pb.move_to(x + width, y);
        pb.line_to(x + width, y + height);
    } else if split.top {
        pb.move_to(x, y);
        pb.line_to(x, y + height - bottom_left);
        if bottom_left > 0.0 {
            pb.quad_to(x, y + height, x + bottom_left, y + height);
        }
        pb.line_to(x + width - bottom_right, y + height);
        if bottom_right > 0.0 {
            pb.quad_to(x + width, y + height, x + width, y + height - bottom_right);
        }
        pb.line_to(x + width, y);
    } else if split.bottom {
        pb.move_to(x, y + height);
        pb.line_to(x, y + top_left);
        if top_left > 0.0 {
            pb.quad_to(x, y, x + top_left, y);
        }
        pb.line_to(x + width - top_right, y);
        if top_right > 0.0 {
            pb.quad_to(x + width, y, x + width, y + top_right);
        }
        pb.line_to(x + width, y + height);
    } else {
        return build_rounded_rect_corners(
            x,
            y,
            width,
            height,
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        );
    }

    pb.finish()
}
