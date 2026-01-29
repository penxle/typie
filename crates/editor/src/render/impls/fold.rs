use crate::layout::elements::{FoldContentElement, FoldTitleBackgroundElement, FoldTitleElement};
use crate::model::{FOLD_BORDER_RADIUS, FOLD_BORDER_WIDTH};
use crate::render::{GlyphRenderer, Render, RenderContext, RenderPhase};
use macros::svg_icon_path;
use tiny_skia::{Paint, Path, PathBuilder, PixmapMut, Stroke, Transform};

const CHEVRON_SIZE: f32 = 20.0;
const CHEVRON_STROKE_WIDTH: f32 = 1.5;

impl Render for FoldTitleElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        _glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        if ctx.phase != RenderPhase::Content {
            return;
        }

        let color = ctx.theme.color("ui.text.subtle");
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;

        let stroke = Stroke {
            width: CHEVRON_STROKE_WIDTH,
            line_cap: tiny_skia::LineCap::Round,
            line_join: tiny_skia::LineJoin::Round,
            ..Stroke::default()
        };

        let cx = self.size.width / 2.0;
        let cy = self.size.height / 2.0;

        let path = if self.expanded {
            svg_icon_path!("lucide/chevron-down", CHEVRON_SIZE, cx, cy)
        } else {
            svg_icon_path!("lucide/chevron-up", CHEVRON_SIZE, cx, cy)
        };

        if let Some(path) = path {
            pixmap.stroke_path(&path, &paint, &stroke, transform, None);
        }
    }
}

impl Render for FoldTitleBackgroundElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        _glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext,
    ) {
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
                if let Some(ref path) = path {
                    let mut paint = Paint::default();
                    paint.set_color(ctx.theme.color("ui.surface.subtle"));
                    paint.anti_alias = true;
                    pixmap.fill_path(&path, &paint, tiny_skia::FillRule::Winding, transform, None);
                }

                let mut border_paint = Paint::default();
                border_paint.set_color(ctx.theme.color("ui.border.default"));
                border_paint.anti_alias = true;
                let stroke = Stroke {
                    width: FOLD_BORDER_WIDTH,
                    ..Stroke::default()
                };

                if !self.expanded {
                    if let Some(ref path) = build_rounded_rect(
                        FOLD_BORDER_WIDTH / 2.0,
                        FOLD_BORDER_WIDTH / 2.0,
                        self.size.width - FOLD_BORDER_WIDTH,
                        self.size.height - FOLD_BORDER_WIDTH,
                        top_left_radius,
                        top_right_radius,
                        bottom_right_radius,
                        bottom_left_radius,
                    ) {
                        pixmap.stroke_path(&path, &border_paint, &stroke, transform, None);
                    }
                } else {
                    let mut pb = PathBuilder::new();
                    pb.move_to(FOLD_BORDER_WIDTH / 2.0, self.size.height);
                    pb.line_to(FOLD_BORDER_WIDTH / 2.0, FOLD_BORDER_RADIUS);
                    pb.quad_to(
                        FOLD_BORDER_WIDTH / 2.0,
                        FOLD_BORDER_WIDTH / 2.0,
                        FOLD_BORDER_RADIUS,
                        FOLD_BORDER_WIDTH / 2.0,
                    );
                    pb.line_to(
                        self.size.width - FOLD_BORDER_RADIUS,
                        FOLD_BORDER_WIDTH / 2.0,
                    );
                    pb.quad_to(
                        self.size.width - FOLD_BORDER_WIDTH / 2.0,
                        FOLD_BORDER_WIDTH / 2.0,
                        self.size.width - FOLD_BORDER_WIDTH / 2.0,
                        FOLD_BORDER_RADIUS,
                    );
                    pb.line_to(self.size.width - FOLD_BORDER_WIDTH / 2.0, self.size.height);
                    if let Some(path) = pb.finish() {
                        pixmap.stroke_path(&path, &border_paint, &stroke, transform, None);
                    }
                }
            }
            RenderPhase::Selection => {
                let is_selected = ctx.selections.iter().any(|s| {
                    if let crate::model::SelectionDecor::Fold { node_id } = s {
                        *node_id == self.fold_id
                    } else {
                        false
                    }
                });

                if is_selected {
                    if let Some(ref path) = path {
                        let color = if ctx.is_focused {
                            ctx.theme.color_with_alpha("selection", 77)
                        } else {
                            ctx.theme.color_with_alpha("ui.surface.dark", 32)
                        };
                        let mut sel_paint = Paint::default();
                        sel_paint.set_color(color);
                        pixmap.fill_path(
                            &path,
                            &sel_paint,
                            tiny_skia::FillRule::Winding,
                            transform,
                            None,
                        );
                    }
                }
            }
            RenderPhase::Content => {}
        }
    }
}

impl Render for FoldContentElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        _glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        use crate::render::RenderPhase;

        let mb = FOLD_BORDER_WIDTH / 2.0;

        match ctx.phase {
            RenderPhase::Background => {
                let mut paint = Paint::default();
                paint.set_color(ctx.theme.color("ui.border.default"));
                paint.anti_alias = true;
                let stroke = Stroke {
                    width: FOLD_BORDER_WIDTH,
                    ..Stroke::default()
                };

                let mut pb = PathBuilder::new();
                if self.split_edges.top && self.split_edges.bottom {
                    pb.move_to(mb, 0.0);
                    pb.line_to(mb, self.size.height);
                    pb.move_to(self.size.width - mb, 0.0);
                    pb.line_to(self.size.width - mb, self.size.height);
                } else if self.split_edges.bottom {
                    pb.move_to(mb, 0.0);
                    pb.line_to(mb, self.size.height);
                    pb.move_to(self.size.width - mb, 0.0);
                    pb.line_to(self.size.width - mb, self.size.height);
                } else {
                    pb.move_to(mb, 0.0);
                    pb.line_to(mb, self.size.height - FOLD_BORDER_RADIUS);
                    pb.quad_to(
                        mb,
                        self.size.height - mb,
                        FOLD_BORDER_RADIUS,
                        self.size.height - mb,
                    );
                    pb.line_to(self.size.width - FOLD_BORDER_RADIUS, self.size.height - mb);
                    pb.quad_to(
                        self.size.width - mb,
                        self.size.height - mb,
                        self.size.width - mb,
                        self.size.height - FOLD_BORDER_RADIUS,
                    );
                    pb.line_to(self.size.width - mb, 0.0);
                }

                if let Some(path) = pb.finish() {
                    pixmap.stroke_path(&path, &paint, &stroke, transform, None);
                }
            }
            RenderPhase::Selection => {
                let is_selected = ctx.selections.iter().any(|s| {
                    if let crate::model::SelectionDecor::Fold { node_id } = s {
                        *node_id == self.fold_id
                    } else {
                        false
                    }
                });

                if is_selected {
                    let color = if ctx.is_focused {
                        ctx.theme.color_with_alpha("selection", 77)
                    } else {
                        ctx.theme.color_with_alpha("ui.surface.dark", 32)
                    };
                    let mut sel_paint = Paint::default();
                    sel_paint.set_color(color);

                    let mut pb = PathBuilder::new();
                    if self.split_edges.top && self.split_edges.bottom {
                        pb.move_to(mb, 0.0);
                        pb.line_to(mb, self.size.height);
                        pb.line_to(self.size.width - mb, self.size.height);
                        pb.line_to(self.size.width - mb, 0.0);
                        pb.close();
                    } else if self.split_edges.bottom {
                        pb.move_to(mb, 0.0);
                        pb.line_to(mb, self.size.height);
                        pb.line_to(self.size.width - mb, self.size.height);
                        pb.close();
                    } else {
                        pb.move_to(mb, 0.0);
                        pb.line_to(mb, self.size.height - FOLD_BORDER_RADIUS);
                        pb.quad_to(
                            mb,
                            self.size.height - mb,
                            FOLD_BORDER_RADIUS,
                            self.size.height - mb,
                        );
                        pb.line_to(self.size.width - FOLD_BORDER_RADIUS, self.size.height - mb);
                        pb.quad_to(
                            self.size.width - mb,
                            self.size.height - mb,
                            self.size.width - mb,
                            self.size.height - FOLD_BORDER_RADIUS,
                        );
                        pb.line_to(self.size.width - mb, 0.0);
                        pb.close();
                    }

                    if let Some(path) = pb.finish() {
                        pixmap.fill_path(
                            &path,
                            &sel_paint,
                            tiny_skia::FillRule::Winding,
                            transform,
                            None,
                        );
                    }
                }
            }
            RenderPhase::Content => {}
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
) -> Option<Path> {
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
