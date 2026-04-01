use crate::layout::elements::SplitEdges;
use crate::layout::elements::{CalloutBackgroundElement, CalloutIconElement};
use crate::model::CalloutVariant;
use crate::model::{CALLOUT_BORDER_RADIUS, CALLOUT_BORDER_WIDTH};
use crate::render::sink::RenderSink;
use crate::render::{Render, RenderParams, RenderPhase};
use kurbo::{Affine, BezPath, Cap, Join, Stroke};
use macros::svg_icon_path;
use peniko::{Brush, Fill};

const ICON_SIZE: f32 = 20.0;
const ICON_STROKE_WIDTH: f32 = 1.5;

impl Render for CalloutBackgroundElement {
    fn render(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        self.paint_to(sink, transform, ctx);
    }
}

impl CalloutBackgroundElement {
    fn paint_to(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        let color_key = format!("ui.callout.{}", self.variant);
        let is_selected = ctx.is_block_selected(self.node_id);

        match ctx.phase {
            RenderPhase::Background => {
                let bg_color = ctx.theme.color_with_alpha(&color_key, 8);
                let bg_brush = Brush::Solid(bg_color);

                let (tl, tr, br, bl) = corner_radii(CALLOUT_BORDER_RADIUS, &self.split_edges);

                let path = build_rounded_rect_corners(
                    0.0,
                    0.0,
                    self.size.width,
                    self.size.height,
                    tl,
                    tr,
                    br,
                    bl,
                );
                sink.fill_path(&path, &bg_brush, Fill::NonZero, transform);
            }
            RenderPhase::Content => {
                let border_color = ctx.theme.color(&color_key);
                let border_brush = Brush::Solid(border_color);

                let stroke = Stroke::new(CALLOUT_BORDER_WIDTH as f64);

                let mb = CALLOUT_BORDER_WIDTH / 2.0;
                let inner_radius = (CALLOUT_BORDER_RADIUS - mb).max(0.0);
                let (tl_inner, tr_inner, br_inner, bl_inner) =
                    corner_radii(inner_radius, &self.split_edges);

                let path = build_partial_border(
                    mb,
                    mb,
                    self.size.width - CALLOUT_BORDER_WIDTH,
                    self.size.height - CALLOUT_BORDER_WIDTH,
                    tl_inner,
                    tr_inner,
                    br_inner,
                    bl_inner,
                    &self.split_edges,
                );
                sink.stroke_path(&path, &border_brush, &stroke, transform);
            }
            RenderPhase::Selection => {
                if !is_selected {
                    return;
                }

                let selection_brush = ctx.selection_paint();

                let (tl, tr, br, bl) = corner_radii(CALLOUT_BORDER_RADIUS, &self.split_edges);
                let path = build_rounded_rect_corners(
                    0.0,
                    0.0,
                    self.size.width,
                    self.size.height,
                    tl,
                    tr,
                    br,
                    bl,
                );
                sink.fill_path(&path, &selection_brush, Fill::NonZero, transform);
            }
        }
    }
}

impl Render for CalloutIconElement {
    fn render(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        self.paint_to(sink, transform, ctx);
    }
}

impl CalloutIconElement {
    fn paint_to(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        if let RenderPhase::Content = ctx.phase {
            let color_key = format!("ui.callout.{}", self.variant);
            let icon_color = ctx.theme.color(&color_key);

            let icon_brush = Brush::Solid(icon_color);

            let icon_stroke = Stroke::new(ICON_STROKE_WIDTH as f64)
                .with_caps(Cap::Round)
                .with_join(Join::Round);

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
                sink.stroke_path(&path, &icon_brush, &icon_stroke, transform);
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
) -> BezPath {
    let mut bp = BezPath::new();

    bp.move_to(((x + top_left) as f64, y as f64));
    bp.line_to(((x + width - top_right) as f64, y as f64));
    if top_right > 0.0 {
        bp.quad_to(
            ((x + width) as f64, y as f64),
            ((x + width) as f64, (y + top_right) as f64),
        );
    }
    bp.line_to(((x + width) as f64, (y + height - bottom_right) as f64));
    if bottom_right > 0.0 {
        bp.quad_to(
            ((x + width) as f64, (y + height) as f64),
            ((x + width - bottom_right) as f64, (y + height) as f64),
        );
    }
    bp.line_to(((x + bottom_left) as f64, (y + height) as f64));
    if bottom_left > 0.0 {
        bp.quad_to(
            (x as f64, (y + height) as f64),
            (x as f64, (y + height - bottom_left) as f64),
        );
    }
    bp.line_to((x as f64, (y + top_left) as f64));
    if top_left > 0.0 {
        bp.quad_to((x as f64, y as f64), ((x + top_left) as f64, y as f64));
    }
    bp.close_path();

    bp
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
) -> BezPath {
    let mut bp = BezPath::new();

    if split.top && split.bottom {
        bp.move_to((x as f64, y as f64));
        bp.line_to((x as f64, (y + height) as f64));
        bp.move_to(((x + width) as f64, y as f64));
        bp.line_to(((x + width) as f64, (y + height) as f64));
    } else if split.top {
        bp.move_to((x as f64, y as f64));
        bp.line_to((x as f64, (y + height - bottom_left) as f64));
        if bottom_left > 0.0 {
            bp.quad_to(
                (x as f64, (y + height) as f64),
                ((x + bottom_left) as f64, (y + height) as f64),
            );
        }
        bp.line_to(((x + width - bottom_right) as f64, (y + height) as f64));
        if bottom_right > 0.0 {
            bp.quad_to(
                ((x + width) as f64, (y + height) as f64),
                ((x + width) as f64, (y + height - bottom_right) as f64),
            );
        }
        bp.line_to(((x + width) as f64, y as f64));
    } else if split.bottom {
        bp.move_to((x as f64, (y + height) as f64));
        bp.line_to((x as f64, (y + top_left) as f64));
        if top_left > 0.0 {
            bp.quad_to((x as f64, y as f64), ((x + top_left) as f64, y as f64));
        }
        bp.line_to(((x + width - top_right) as f64, y as f64));
        if top_right > 0.0 {
            bp.quad_to(
                ((x + width) as f64, y as f64),
                ((x + width) as f64, (y + top_right) as f64),
            );
        }
        bp.line_to(((x + width) as f64, (y + height) as f64));
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

    bp
}
