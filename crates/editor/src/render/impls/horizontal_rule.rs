use crate::layout::elements::HorizontalRuleElement;
use crate::model::HorizontalRuleVariant;
use crate::render::{GlyphRenderer, Render, RenderContext, RenderPhase};
use crate::state::position_helpers::calculate_offset_before_child;
use tiny_skia::{Paint, PathBuilder, PixmapMut, Rect, Stroke, Transform};

const LINE_HEIGHT: f32 = 1.0;
const SHAPE_SIZE_LARGE: f32 = 10.0;
const SHAPE_SIZE_SMALL: f32 = 8.0;
const SHAPE_GAP: f32 = 8.0;

impl Render for HorizontalRuleElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        _glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        let is_selected = if let Some(selection) = ctx
            .selections
            .iter()
            .find(|sel| sel.node_id() == self.parent_id)
        {
            if let Some(parent) = ctx.doc.node(self.parent_id) {
                let offset = calculate_offset_before_child(&parent, self.node_id);
                selection.start_offset() <= offset && offset < selection.end_offset()
            } else {
                false
            }
        } else {
            false
        };

        match ctx.phase {
            RenderPhase::Background => {
                let color = ctx.theme.color("ui.text.default");
                let mut paint = Paint::default();
                paint.set_color(color);
                paint.anti_alias = true;

                match self.variant {
                    HorizontalRuleVariant::Line => self.render_line(pixmap, transform, &paint),
                    HorizontalRuleVariant::DashedLine => {
                        self.render_dashed_line(pixmap, transform, &paint)
                    }
                    HorizontalRuleVariant::CircleLine => {
                        self.render_circle_line(pixmap, transform, &paint)
                    }
                    HorizontalRuleVariant::DiamondLine => {
                        self.render_diamond_line(pixmap, transform, &paint)
                    }
                    HorizontalRuleVariant::Circle => self.render_circle(pixmap, transform, &paint),
                    HorizontalRuleVariant::Diamond => {
                        self.render_diamond(pixmap, transform, &paint)
                    }
                    HorizontalRuleVariant::ThreeCircles => {
                        self.render_three_circles(pixmap, transform, &paint)
                    }
                    HorizontalRuleVariant::ThreeDiamonds => {
                        self.render_three_diamonds(pixmap, transform, &paint)
                    }
                    HorizontalRuleVariant::Zigzag => self.render_zigzag(pixmap, transform, &paint),
                }
            }
            RenderPhase::Selection => {
                if is_selected {
                    let color = if ctx.is_focused {
                        ctx.theme.color_with_alpha("selection", 77)
                    } else {
                        ctx.theme.color_with_alpha("selection", 48)
                    };
                    let mut paint = Paint::default();
                    paint.set_color(color);

                    if let Some(rect) = Rect::from_xywh(0.0, 0.0, self.size.width, self.size.height)
                    {
                        pixmap.fill_rect(rect, &paint, transform, None);
                    }
                }
            }
            RenderPhase::Content => {}
        }
    }
}

impl HorizontalRuleElement {
    fn center(&self) -> (f32, f32) {
        (self.size.width / 2.0, self.size.height / 2.0)
    }

    fn circle_path(cx: f32, cy: f32, r: f32) -> Option<tiny_skia::Path> {
        let mut pb = PathBuilder::new();
        pb.push_circle(cx, cy, r);
        pb.finish()
    }

    fn diamond_path(cx: f32, cy: f32, r: f32) -> Option<tiny_skia::Path> {
        let mut pb = PathBuilder::new();
        pb.move_to(cx, cy - r);
        pb.line_to(cx + r, cy);
        pb.line_to(cx, cy + r);
        pb.line_to(cx - r, cy);
        pb.close();
        pb.finish()
    }

    fn fill_path(
        &self,
        pixmap: &mut PixmapMut,
        transform: Transform,
        paint: &Paint,
        path: Option<tiny_skia::Path>,
    ) {
        if let Some(path) = path {
            pixmap.fill_path(&path, paint, tiny_skia::FillRule::Winding, transform, None);
        }
    }

    fn stroke_path(
        &self,
        pixmap: &mut PixmapMut,
        transform: Transform,
        paint: &Paint,
        path: Option<tiny_skia::Path>,
    ) {
        if let Some(path) = path {
            let stroke = Stroke {
                width: 1.0,
                ..Default::default()
            };
            pixmap.stroke_path(&path, paint, &stroke, transform, None);
        }
    }

    fn fill_rect(
        &self,
        pixmap: &mut PixmapMut,
        transform: Transform,
        paint: &Paint,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        if let Some(rect) = Rect::from_xywh(x, y, w, h) {
            pixmap.fill_rect(rect, paint, transform, None);
        }
    }

    fn render_line(&self, pixmap: &mut PixmapMut, transform: Transform, paint: &Paint) {
        let y = (self.size.height - LINE_HEIGHT) / 2.0;
        self.fill_rect(
            pixmap,
            transform,
            paint,
            0.0,
            y,
            self.size.width,
            LINE_HEIGHT,
        );
    }

    fn render_dashed_line(&self, pixmap: &mut PixmapMut, transform: Transform, paint: &Paint) {
        let y = self.size.height / 2.0 - LINE_HEIGHT / 2.0;
        let segment_width: f32 = 16.0;
        let dash_width: f32 = segment_width * 0.5;
        let mut x: f32 = 0.0;

        while x < self.size.width {
            let w = dash_width.min(self.size.width - x);
            self.fill_rect(pixmap, transform, paint, x, y, w, LINE_HEIGHT);
            x += segment_width;
        }
    }

    fn render_circle_line(&self, pixmap: &mut PixmapMut, transform: Transform, paint: &Paint) {
        let (cx, cy) = self.center();
        let shape_half = (SHAPE_SIZE_LARGE / 2.0) + 10.0;
        let line_y = cy - LINE_HEIGHT / 2.0;
        let container_half = self.size.width / 4.0;
        let line_width = container_half - shape_half;

        self.fill_rect(
            pixmap,
            transform,
            paint,
            cx - container_half,
            line_y,
            line_width,
            LINE_HEIGHT,
        );
        self.fill_rect(
            pixmap,
            transform,
            paint,
            cx + shape_half,
            line_y,
            line_width,
            LINE_HEIGHT,
        );
        self.fill_path(
            pixmap,
            transform,
            paint,
            Self::circle_path(cx, cy, SHAPE_SIZE_LARGE / 2.0),
        );
    }

    fn render_diamond_line(&self, pixmap: &mut PixmapMut, transform: Transform, paint: &Paint) {
        let (cx, cy) = self.center();
        let shape_half = (SHAPE_SIZE_LARGE / 2.0) + 10.0;
        let line_y = cy - LINE_HEIGHT / 2.0;
        let container_half = self.size.width / 4.0;
        let line_width = container_half - shape_half;

        self.fill_rect(
            pixmap,
            transform,
            paint,
            cx - container_half,
            line_y,
            line_width,
            LINE_HEIGHT,
        );
        self.fill_rect(
            pixmap,
            transform,
            paint,
            cx + shape_half,
            line_y,
            line_width,
            LINE_HEIGHT,
        );
        self.stroke_path(
            pixmap,
            transform,
            paint,
            Self::diamond_path(cx, cy, SHAPE_SIZE_LARGE / 2.0),
        );
    }

    fn render_circle(&self, pixmap: &mut PixmapMut, transform: Transform, paint: &Paint) {
        let (cx, cy) = self.center();
        self.fill_path(
            pixmap,
            transform,
            paint,
            Self::circle_path(cx, cy, SHAPE_SIZE_LARGE / 2.0),
        );
    }

    fn render_diamond(&self, pixmap: &mut PixmapMut, transform: Transform, paint: &Paint) {
        let (cx, cy) = self.center();
        self.stroke_path(
            pixmap,
            transform,
            paint,
            Self::diamond_path(cx, cy, SHAPE_SIZE_LARGE / 2.0),
        );
    }

    fn render_three_circles(&self, pixmap: &mut PixmapMut, transform: Transform, paint: &Paint) {
        let (cx, cy) = self.center();
        let r = SHAPE_SIZE_SMALL / 2.0;
        let gap = SHAPE_GAP + SHAPE_SIZE_SMALL;
        for offset in [-gap, 0.0, gap] {
            self.fill_path(
                pixmap,
                transform,
                paint,
                Self::circle_path(cx + offset, cy, r),
            );
        }
    }

    fn render_three_diamonds(&self, pixmap: &mut PixmapMut, transform: Transform, paint: &Paint) {
        let (cx, cy) = self.center();
        let r = SHAPE_SIZE_SMALL / 2.0;
        let gap = SHAPE_GAP + SHAPE_SIZE_SMALL;
        for offset in [-gap, 0.0, gap] {
            self.stroke_path(
                pixmap,
                transform,
                paint,
                Self::diamond_path(cx + offset, cy, r),
            );
        }
    }

    fn render_zigzag(&self, pixmap: &mut PixmapMut, transform: Transform, paint: &Paint) {
        let (cx, cy) = self.center();
        const POINTS: usize = 8;
        const SEGMENT_WIDTH: f32 = 8.0;
        const AMPLITUDE: f32 = 4.0;

        let total_width = (POINTS - 1) as f32 * SEGMENT_WIDTH;
        let start_x = cx - total_width / 2.0;

        let mut pb = PathBuilder::new();
        for i in 0..POINTS {
            let x = start_x + i as f32 * SEGMENT_WIDTH;
            let y = if i % 2 == 0 {
                cy + AMPLITUDE
            } else {
                cy - AMPLITUDE
            };
            if i == 0 {
                pb.move_to(x, y);
            } else {
                pb.line_to(x, y);
            }
        }

        if let Some(path) = pb.finish() {
            let stroke = Stroke {
                width: 1.0,
                line_cap: tiny_skia::LineCap::Round,
                line_join: tiny_skia::LineJoin::Round,
                ..Default::default()
            };
            pixmap.stroke_path(&path, paint, &stroke, transform, None);
        }
    }
}
