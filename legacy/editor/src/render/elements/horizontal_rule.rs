use crate::layout::elements::HorizontalRuleElement;
use crate::model::HorizontalRuleVariant;
use crate::render::sink::RenderSink;
use crate::render::{Render, RenderParams, RenderPhase};
use kurbo::{Affine, BezPath, Cap, Circle, Join, Rect, Shape, Stroke};
use peniko::{Brush, Fill};

const LINE_HEIGHT: f32 = 1.0;
const SHAPE_SIZE_LARGE: f32 = 10.0;
const SHAPE_SIZE_SMALL: f32 = 8.0;
const SHAPE_GAP: f32 = 8.0;

impl Render for HorizontalRuleElement {
    fn render(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        self.paint_to(sink, transform, ctx);
    }
}

impl HorizontalRuleElement {
    fn paint_to(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>) {
        let is_selected = ctx.is_block_selected(self.node_id);

        match ctx.phase {
            RenderPhase::Background => {}
            RenderPhase::Selection => {
                if is_selected {
                    let brush = ctx.selection_paint();
                    let rect = Rect::new(0.0, 0.0, self.size.width as f64, self.size.height as f64);
                    sink.fill_rect(rect, &brush, transform);
                }
            }
            RenderPhase::Content => {
                let color = ctx.theme.color("ui.text.default");
                let brush = Brush::Solid(color);

                match self.variant {
                    HorizontalRuleVariant::Line => self.render_line(sink, transform, &brush),
                    HorizontalRuleVariant::DashedLine => {
                        self.render_dashed_line(sink, transform, &brush)
                    }
                    HorizontalRuleVariant::CircleLine => {
                        self.render_circle_line(sink, transform, &brush)
                    }
                    HorizontalRuleVariant::DiamondLine => {
                        self.render_diamond_line(sink, transform, &brush)
                    }
                    HorizontalRuleVariant::Circle => self.render_circle(sink, transform, &brush),
                    HorizontalRuleVariant::Diamond => self.render_diamond(sink, transform, &brush),
                    HorizontalRuleVariant::ThreeCircles => {
                        self.render_three_circles(sink, transform, &brush)
                    }
                    HorizontalRuleVariant::ThreeDiamonds => {
                        self.render_three_diamonds(sink, transform, &brush)
                    }
                    HorizontalRuleVariant::Zigzag => self.render_zigzag(sink, transform, &brush),
                }
            }
        }
    }

    fn center(&self) -> (f32, f32) {
        (self.size.width / 2.0, self.size.height / 2.0)
    }

    fn circle_path(cx: f32, cy: f32, r: f32) -> BezPath {
        Circle::new((cx as f64, cy as f64), r as f64).to_path(0.1)
    }

    fn diamond_path(cx: f32, cy: f32, r: f32) -> BezPath {
        let mut bp = BezPath::new();
        bp.move_to((cx as f64, (cy - r) as f64));
        bp.line_to(((cx + r) as f64, cy as f64));
        bp.line_to((cx as f64, (cy + r) as f64));
        bp.line_to(((cx - r) as f64, cy as f64));
        bp.close_path();
        bp
    }

    fn fill_path_bp(
        &self,
        sink: &mut dyn RenderSink,
        transform: Affine,
        brush: &Brush,
        path: &BezPath,
    ) {
        sink.fill_path(path, brush, Fill::NonZero, transform);
    }

    fn stroke_path_bp(
        &self,
        sink: &mut dyn RenderSink,
        transform: Affine,
        brush: &Brush,
        path: &BezPath,
    ) {
        let stroke = Stroke::new(1.0);
        sink.stroke_path(path, brush, &stroke, transform);
    }

    fn fill_rect_bp(
        &self,
        sink: &mut dyn RenderSink,
        transform: Affine,
        brush: &Brush,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    ) {
        let rect = Rect::new(x as f64, y as f64, (x + w) as f64, (y + h) as f64);
        sink.fill_rect(rect, brush, transform);
    }

    fn render_line(&self, sink: &mut dyn RenderSink, transform: Affine, brush: &Brush) {
        let y = (self.size.height - LINE_HEIGHT) / 2.0;
        self.fill_rect_bp(sink, transform, brush, 0.0, y, self.size.width, LINE_HEIGHT);
    }

    fn render_dashed_line(&self, sink: &mut dyn RenderSink, transform: Affine, brush: &Brush) {
        let y = self.size.height / 2.0 - LINE_HEIGHT / 2.0;
        let segment_width: f32 = 16.0;
        let dash_width: f32 = segment_width * 0.5;
        let mut x: f32 = 0.0;

        while x < self.size.width {
            let w = dash_width.min(self.size.width - x);
            self.fill_rect_bp(sink, transform, brush, x, y, w, LINE_HEIGHT);
            x += segment_width;
        }
    }

    fn render_circle_line(&self, sink: &mut dyn RenderSink, transform: Affine, brush: &Brush) {
        let (cx, cy) = self.center();
        let shape_half = (SHAPE_SIZE_LARGE / 2.0) + 10.0;
        let line_y = cy - LINE_HEIGHT / 2.0;
        let container_half = self.size.width / 4.0;
        let line_width = container_half - shape_half;

        self.fill_rect_bp(
            sink,
            transform,
            brush,
            cx - container_half,
            line_y,
            line_width,
            LINE_HEIGHT,
        );
        self.fill_rect_bp(
            sink,
            transform,
            brush,
            cx + shape_half,
            line_y,
            line_width,
            LINE_HEIGHT,
        );
        let path = Self::circle_path(cx, cy, SHAPE_SIZE_LARGE / 2.0);
        self.fill_path_bp(sink, transform, brush, &path);
    }

    fn render_diamond_line(&self, sink: &mut dyn RenderSink, transform: Affine, brush: &Brush) {
        let (cx, cy) = self.center();
        let shape_half = (SHAPE_SIZE_LARGE / 2.0) + 10.0;
        let line_y = cy - LINE_HEIGHT / 2.0;
        let container_half = self.size.width / 4.0;
        let line_width = container_half - shape_half;

        self.fill_rect_bp(
            sink,
            transform,
            brush,
            cx - container_half,
            line_y,
            line_width,
            LINE_HEIGHT,
        );
        self.fill_rect_bp(
            sink,
            transform,
            brush,
            cx + shape_half,
            line_y,
            line_width,
            LINE_HEIGHT,
        );
        let path = Self::diamond_path(cx, cy, SHAPE_SIZE_LARGE / 2.0);
        self.stroke_path_bp(sink, transform, brush, &path);
    }

    fn render_circle(&self, sink: &mut dyn RenderSink, transform: Affine, brush: &Brush) {
        let (cx, cy) = self.center();
        let path = Self::circle_path(cx, cy, SHAPE_SIZE_LARGE / 2.0);
        self.fill_path_bp(sink, transform, brush, &path);
    }

    fn render_diamond(&self, sink: &mut dyn RenderSink, transform: Affine, brush: &Brush) {
        let (cx, cy) = self.center();
        let path = Self::diamond_path(cx, cy, SHAPE_SIZE_LARGE / 2.0);
        self.stroke_path_bp(sink, transform, brush, &path);
    }

    fn render_three_circles(&self, sink: &mut dyn RenderSink, transform: Affine, brush: &Brush) {
        let (cx, cy) = self.center();
        let r = SHAPE_SIZE_SMALL / 2.0;
        let gap = SHAPE_GAP + SHAPE_SIZE_SMALL;
        for offset in [-gap, 0.0, gap] {
            let path = Self::circle_path(cx + offset, cy, r);
            self.fill_path_bp(sink, transform, brush, &path);
        }
    }

    fn render_three_diamonds(&self, sink: &mut dyn RenderSink, transform: Affine, brush: &Brush) {
        let (cx, cy) = self.center();
        let r = SHAPE_SIZE_SMALL / 2.0;
        let gap = SHAPE_GAP + SHAPE_SIZE_SMALL;
        for offset in [-gap, 0.0, gap] {
            let path = Self::diamond_path(cx + offset, cy, r);
            self.stroke_path_bp(sink, transform, brush, &path);
        }
    }

    fn render_zigzag(&self, sink: &mut dyn RenderSink, transform: Affine, brush: &Brush) {
        let (cx, cy) = self.center();
        const POINTS: usize = 8;
        const SEGMENT_WIDTH: f32 = 8.0;
        const AMPLITUDE: f32 = 4.0;

        let total_width = (POINTS - 1) as f32 * SEGMENT_WIDTH;
        let start_x = cx - total_width / 2.0;

        let mut bp = BezPath::new();
        for i in 0..POINTS {
            let x = start_x + i as f32 * SEGMENT_WIDTH;
            let y = if i % 2 == 0 {
                cy + AMPLITUDE
            } else {
                cy - AMPLITUDE
            };
            if i == 0 {
                bp.move_to((x as f64, y as f64));
            } else {
                bp.line_to((x as f64, y as f64));
            }
        }

        let stroke = Stroke::new(1.0)
            .with_caps(Cap::Round)
            .with_join(Join::Round);
        sink.stroke_path(&bp, brush, &stroke, transform);
    }
}
