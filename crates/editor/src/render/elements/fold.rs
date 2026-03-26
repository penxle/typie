use crate::layout::elements::{FoldContentElement, FoldTitleElement, FoldTitleIconElement};
use crate::model::{FOLD_BORDER_RADIUS, FOLD_BORDER_WIDTH};
use crate::render::sink::RenderSink;
use crate::render::{Render, RenderParams, RenderPhase};
use kurbo::{Affine, BezPath, Cap, Join, Stroke};
use macros::svg_icon_path;
use peniko::{Brush, Fill};

const CHEVRON_SIZE: f32 = 20.0;
const CHEVRON_STROKE_WIDTH: f32 = 1.5;

impl Render for FoldTitleIconElement {
    fn render(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        self.paint_to(sink, transform, ctx);
    }
}

impl FoldTitleIconElement {
    fn paint_to(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        if let RenderPhase::Content = ctx.phase {
            let color = ctx.theme.color("ui.text.faint");
            let brush = Brush::Solid(color);

            let stroke = Stroke::new(CHEVRON_STROKE_WIDTH as f64)
                .with_caps(Cap::Round)
                .with_join(Join::Round);

            let cx = self.size.width / 2.0;
            let cy = self.size.height / 2.0;

            let path = if self.expanded {
                svg_icon_path!("lucide/chevron-up", CHEVRON_SIZE, cx, cy)
            } else {
                svg_icon_path!("lucide/chevron-down", CHEVRON_SIZE, cx, cy)
            };

            if let Some(path) = path {
                sink.stroke_path(&path, &brush, &stroke, transform);
            }
        }
    }
}

impl Render for FoldTitleElement {
    fn render(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        self.paint_to(sink, transform, ctx);
    }
}

impl FoldTitleElement {
    fn paint_to(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        let inner_radius = (FOLD_BORDER_RADIUS - FOLD_BORDER_WIDTH).max(0.0);
        let (top_left_radius, top_right_radius, bottom_right_radius, bottom_left_radius) =
            if self.expanded {
                (inner_radius, inner_radius, 0.0, 0.0)
            } else {
                (inner_radius, inner_radius, inner_radius, inner_radius)
            };

        let path = build_rounded_rect(
            0.0,
            0.0,
            self.size.width,
            self.size.height,
            top_left_radius,
            top_right_radius,
            bottom_right_radius,
            bottom_left_radius,
        );

        match ctx.phase {
            RenderPhase::Background => {
                let brush = Brush::Solid(ctx.theme.color("ui.surface.muted"));
                sink.fill_path(&path, &brush, Fill::NonZero, transform);
            }
            RenderPhase::Selection => {
                if ctx.is_block_selected(self.fold_id) {
                    let color = if ctx.is_focused {
                        ctx.theme.color_with_alpha("selection", 77)
                    } else {
                        ctx.theme.color_with_alpha("ui.surface.dark", 32)
                    };
                    let sel_brush = Brush::Solid(color);
                    sink.fill_path(&path, &sel_brush, Fill::NonZero, transform);
                }
            }
            RenderPhase::Content => {
                let border_brush = Brush::Solid(ctx.theme.color("ui.border.default"));
                let stroke = Stroke::new(FOLD_BORDER_WIDTH as f64);

                if !self.expanded {
                    let border_path = build_rounded_rect(
                        FOLD_BORDER_WIDTH / 2.0,
                        FOLD_BORDER_WIDTH / 2.0,
                        self.size.width - FOLD_BORDER_WIDTH,
                        self.size.height - FOLD_BORDER_WIDTH,
                        top_left_radius,
                        top_right_radius,
                        bottom_right_radius,
                        bottom_left_radius,
                    );
                    sink.stroke_path(&border_path, &border_brush, &stroke, transform);
                } else {
                    let mut bp = BezPath::new();
                    bp.move_to(((FOLD_BORDER_WIDTH / 2.0) as f64, self.size.height as f64));
                    bp.line_to(((FOLD_BORDER_WIDTH / 2.0) as f64, FOLD_BORDER_RADIUS as f64));
                    bp.quad_to(
                        (
                            (FOLD_BORDER_WIDTH / 2.0) as f64,
                            (FOLD_BORDER_WIDTH / 2.0) as f64,
                        ),
                        (FOLD_BORDER_RADIUS as f64, (FOLD_BORDER_WIDTH / 2.0) as f64),
                    );
                    bp.line_to((
                        (self.size.width - FOLD_BORDER_RADIUS) as f64,
                        (FOLD_BORDER_WIDTH / 2.0) as f64,
                    ));
                    bp.quad_to(
                        (
                            (self.size.width - FOLD_BORDER_WIDTH / 2.0) as f64,
                            (FOLD_BORDER_WIDTH / 2.0) as f64,
                        ),
                        (
                            (self.size.width - FOLD_BORDER_WIDTH / 2.0) as f64,
                            FOLD_BORDER_RADIUS as f64,
                        ),
                    );
                    bp.line_to((
                        (self.size.width - FOLD_BORDER_WIDTH / 2.0) as f64,
                        self.size.height as f64,
                    ));
                    sink.stroke_path(&bp, &border_brush, &stroke, transform);
                }
            }
        }
    }
}

impl Render for FoldContentElement {
    fn render(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        self.paint_to(sink, transform, ctx);
    }
}

impl FoldContentElement {
    fn paint_to(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        let mb = FOLD_BORDER_WIDTH / 2.0;

        match ctx.phase {
            RenderPhase::Background => {}
            RenderPhase::Selection => {
                if ctx.is_block_selected(self.fold_id) {
                    let color = if ctx.is_focused {
                        ctx.theme.color_with_alpha("selection", 77)
                    } else {
                        ctx.theme.color_with_alpha("ui.surface.dark", 32)
                    };
                    let sel_brush = Brush::Solid(color);

                    let mut bp = BezPath::new();
                    if self.split_edges.top && self.split_edges.bottom {
                        bp.move_to((mb as f64, 0.0));
                        bp.line_to((mb as f64, self.size.height as f64));
                        bp.line_to(((self.size.width - mb) as f64, self.size.height as f64));
                        bp.line_to(((self.size.width - mb) as f64, 0.0));
                        bp.close_path();
                    } else if self.split_edges.bottom {
                        bp.move_to((mb as f64, 0.0));
                        bp.line_to((mb as f64, self.size.height as f64));
                        bp.line_to(((self.size.width - mb) as f64, self.size.height as f64));
                        bp.close_path();
                    } else {
                        bp.move_to((mb as f64, 0.0));
                        bp.line_to((mb as f64, (self.size.height - FOLD_BORDER_RADIUS) as f64));
                        bp.quad_to(
                            (mb as f64, (self.size.height - mb) as f64),
                            (FOLD_BORDER_RADIUS as f64, (self.size.height - mb) as f64),
                        );
                        bp.line_to((
                            (self.size.width - FOLD_BORDER_RADIUS) as f64,
                            (self.size.height - mb) as f64,
                        ));
                        bp.quad_to(
                            (
                                (self.size.width - mb) as f64,
                                (self.size.height - mb) as f64,
                            ),
                            (
                                (self.size.width - mb) as f64,
                                (self.size.height - FOLD_BORDER_RADIUS) as f64,
                            ),
                        );
                        bp.line_to(((self.size.width - mb) as f64, 0.0));
                        bp.close_path();
                    }

                    sink.fill_path(&bp, &sel_brush, Fill::NonZero, transform);
                }
            }
            RenderPhase::Content => {
                let brush = Brush::Solid(ctx.theme.color("ui.border.default"));
                let stroke = Stroke::new(FOLD_BORDER_WIDTH as f64);

                let mut bp = BezPath::new();
                if self.split_edges.top && self.split_edges.bottom {
                    bp.move_to((mb as f64, 0.0));
                    bp.line_to((mb as f64, self.size.height as f64));
                    bp.move_to(((self.size.width - mb) as f64, 0.0));
                    bp.line_to(((self.size.width - mb) as f64, self.size.height as f64));
                } else if self.split_edges.bottom {
                    bp.move_to((mb as f64, 0.0));
                    bp.line_to((mb as f64, self.size.height as f64));
                    bp.move_to(((self.size.width - mb) as f64, 0.0));
                    bp.line_to(((self.size.width - mb) as f64, self.size.height as f64));
                } else {
                    bp.move_to((mb as f64, 0.0));
                    bp.line_to((mb as f64, (self.size.height - FOLD_BORDER_RADIUS) as f64));
                    bp.quad_to(
                        (mb as f64, (self.size.height - mb) as f64),
                        (FOLD_BORDER_RADIUS as f64, (self.size.height - mb) as f64),
                    );
                    bp.line_to((
                        (self.size.width - FOLD_BORDER_RADIUS) as f64,
                        (self.size.height - mb) as f64,
                    ));
                    bp.quad_to(
                        (
                            (self.size.width - mb) as f64,
                            (self.size.height - mb) as f64,
                        ),
                        (
                            (self.size.width - mb) as f64,
                            (self.size.height - FOLD_BORDER_RADIUS) as f64,
                        ),
                    );
                    bp.line_to(((self.size.width - mb) as f64, 0.0));
                }

                sink.stroke_path(&bp, &brush, &stroke, transform);
            }
        }
    }
}

fn build_rounded_rect(
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
