use crate::render::GlyphRenderer;
use crate::render::glyph::Glyph;
use parley::FontData;
use serde::Serialize;
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, PathSegment, PixmapMut, Rect,
    Stroke, Transform,
};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VectorFillRule {
    Winding,
    EvenOdd,
}

impl From<FillRule> for VectorFillRule {
    fn from(value: FillRule) -> Self {
        match value {
            FillRule::Winding => Self::Winding,
            FillRule::EvenOdd => Self::EvenOdd,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VectorLineCap {
    Butt,
    Round,
    Square,
}

impl From<LineCap> for VectorLineCap {
    fn from(value: LineCap) -> Self {
        match value {
            LineCap::Butt => Self::Butt,
            LineCap::Round => Self::Round,
            LineCap::Square => Self::Square,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VectorLineJoin {
    Miter,
    Round,
    Bevel,
}

impl From<LineJoin> for VectorLineJoin {
    fn from(value: LineJoin) -> Self {
        match value {
            LineJoin::Miter => Self::Miter,
            LineJoin::MiterClip => Self::Miter,
            LineJoin::Round => Self::Round,
            LineJoin::Bevel => Self::Bevel,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum VectorPathCommand {
    MoveTo {
        x: f32,
        y: f32,
    },
    LineTo {
        x: f32,
        y: f32,
    },
    QuadTo {
        cx: f32,
        cy: f32,
        x: f32,
        y: f32,
    },
    CubicTo {
        c1x: f32,
        c1y: f32,
        c2x: f32,
        c2y: f32,
        x: f32,
        y: f32,
    },
    ClosePath,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum VectorOp {
    FillPath {
        path: Vec<VectorPathCommand>,
        color: [u8; 4],
        fill_rule: VectorFillRule,
    },
    StrokePath {
        path: Vec<VectorPathCommand>,
        color: [u8; 4],
        width: f32,
        line_cap: VectorLineCap,
        line_join: VectorLineJoin,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorTextOp {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub size: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VectorPage {
    pub width: f32,
    pub height: f32,
    pub ops: Vec<VectorOp>,
    pub text_ops: Vec<VectorTextOp>,
}

pub trait ElementSink {
    fn fill_rect(&mut self, rect: Rect, paint: &Paint, transform: Transform);
    fn fill_path(&mut self, path: &Path, paint: &Paint, fill_rule: FillRule, transform: Transform);
    fn stroke_path(&mut self, path: &Path, paint: &Paint, stroke: &Stroke, transform: Transform);
    fn draw_text_layer(&mut self, text: &str, font_size: f32, x: f32, y: f32, transform: Transform);
    fn draw_glyphs(
        &mut self,
        font: &FontData,
        font_size: f32,
        paint: &Paint,
        transform: Transform,
        glyph_transform: Option<Transform>,
        embolden: bool,
        glyphs: &[Glyph],
    );
}

pub struct RasterSink<'a, 'p> {
    pixmap: &'a mut PixmapMut<'p>,
    glyph_renderer: &'a mut GlyphRenderer,
}

impl<'a, 'p> RasterSink<'a, 'p> {
    pub fn new(pixmap: &'a mut PixmapMut<'p>, glyph_renderer: &'a mut GlyphRenderer) -> Self {
        Self {
            pixmap,
            glyph_renderer,
        }
    }
}

impl ElementSink for RasterSink<'_, '_> {
    fn fill_rect(&mut self, rect: Rect, paint: &Paint, transform: Transform) {
        self.pixmap.fill_rect(rect, paint, transform, None);
    }

    fn fill_path(&mut self, path: &Path, paint: &Paint, fill_rule: FillRule, transform: Transform) {
        self.pixmap
            .fill_path(path, paint, fill_rule, transform, None);
    }

    fn stroke_path(&mut self, path: &Path, paint: &Paint, stroke: &Stroke, transform: Transform) {
        self.pixmap
            .stroke_path(path, paint, stroke, transform, None);
    }

    fn draw_text_layer(
        &mut self,
        _text: &str,
        _font_size: f32,
        _x: f32,
        _y: f32,
        _transform: Transform,
    ) {
    }

    fn draw_glyphs(
        &mut self,
        font: &FontData,
        font_size: f32,
        paint: &Paint,
        transform: Transform,
        glyph_transform: Option<Transform>,
        embolden: bool,
        glyphs: &[Glyph],
    ) {
        self.glyph_renderer.draw_glyphs(
            self.pixmap,
            font,
            font_size,
            paint,
            transform,
            glyph_transform,
            embolden,
            glyphs,
        );
    }
}

pub struct VectorSink {
    ops: Vec<VectorOp>,
    text_ops: Vec<VectorTextOp>,
    glyph_renderer: GlyphRenderer,
}

impl VectorSink {
    pub fn new() -> Self {
        Self {
            ops: Vec::new(),
            text_ops: Vec::new(),
            glyph_renderer: GlyphRenderer::new(),
        }
    }

    pub fn into_parts(self) -> (Vec<VectorOp>, Vec<VectorTextOp>) {
        (self.ops, self.text_ops)
    }
}

impl ElementSink for VectorSink {
    fn fill_rect(&mut self, rect: Rect, paint: &Paint, transform: Transform) {
        let mut pb = PathBuilder::new();
        let left = rect.left();
        let top = rect.top();
        let right = rect.right();
        let bottom = rect.bottom();
        pb.move_to(left, top);
        pb.line_to(right, top);
        pb.line_to(right, bottom);
        pb.line_to(left, bottom);
        pb.close();

        if let Some(path) = pb.finish() {
            self.fill_path(&path, paint, FillRule::Winding, transform);
        }
    }

    fn fill_path(&mut self, path: &Path, paint: &Paint, fill_rule: FillRule, transform: Transform) {
        let commands = commands_from_path(path, transform);
        if commands.is_empty() {
            return;
        }

        self.ops.push(VectorOp::FillPath {
            path: commands,
            color: paint_rgba(paint),
            fill_rule: fill_rule.into(),
        });
    }

    fn stroke_path(&mut self, path: &Path, paint: &Paint, stroke: &Stroke, transform: Transform) {
        let commands = commands_from_path(path, transform);
        if commands.is_empty() {
            return;
        }

        self.ops.push(VectorOp::StrokePath {
            path: commands,
            color: paint_rgba(paint),
            width: stroke.width,
            line_cap: stroke.line_cap.into(),
            line_join: stroke.line_join.into(),
        });
    }

    fn draw_text_layer(
        &mut self,
        text: &str,
        font_size: f32,
        x: f32,
        y: f32,
        transform: Transform,
    ) {
        if text.is_empty() || !font_size.is_finite() || font_size <= 0.0 {
            return;
        }

        let point = map_point(transform, tiny_skia::Point::from_xy(x, y));
        if !point.x.is_finite() || !point.y.is_finite() {
            return;
        }

        self.text_ops.push(VectorTextOp {
            text: text.to_string(),
            x: point.x,
            y: point.y,
            size: font_size,
        });
    }

    fn draw_glyphs(
        &mut self,
        font: &FontData,
        font_size: f32,
        paint: &Paint,
        transform: Transform,
        glyph_transform: Option<Transform>,
        embolden: bool,
        glyphs: &[Glyph],
    ) {
        let color = paint_rgba(paint);
        self.glyph_renderer.for_each_glyph_outline(
            font,
            font_size,
            transform,
            glyph_transform,
            embolden,
            glyphs,
            |path| {
                let commands = commands_from_path(path, Transform::identity());
                if commands.is_empty() {
                    return;
                }

                self.ops.push(VectorOp::FillPath {
                    path: commands,
                    color,
                    fill_rule: VectorFillRule::Winding,
                });
            },
        );
    }
}

fn paint_rgba(paint: &Paint) -> [u8; 4] {
    match &paint.shader {
        tiny_skia::Shader::SolidColor(color) => color_rgba(*color),
        _ => color_rgba(Color::BLACK),
    }
}

fn color_rgba(color: Color) -> [u8; 4] {
    [
        (color.red() * 255.0).round() as u8,
        (color.green() * 255.0).round() as u8,
        (color.blue() * 255.0).round() as u8,
        (color.alpha() * 255.0).round() as u8,
    ]
}

fn map_point(transform: Transform, mut point: tiny_skia::Point) -> tiny_skia::Point {
    transform.map_point(&mut point);
    point
}

fn commands_from_path(path: &Path, transform: Transform) -> Vec<VectorPathCommand> {
    let mut commands = Vec::new();

    for segment in path.segments() {
        match segment {
            PathSegment::MoveTo(point) => {
                let point = map_point(transform, point);
                commands.push(VectorPathCommand::MoveTo {
                    x: point.x,
                    y: point.y,
                });
            }
            PathSegment::LineTo(point) => {
                let point = map_point(transform, point);
                commands.push(VectorPathCommand::LineTo {
                    x: point.x,
                    y: point.y,
                });
            }
            PathSegment::QuadTo(ctrl, point) => {
                let ctrl = map_point(transform, ctrl);
                let point = map_point(transform, point);
                commands.push(VectorPathCommand::QuadTo {
                    cx: ctrl.x,
                    cy: ctrl.y,
                    x: point.x,
                    y: point.y,
                });
            }
            PathSegment::CubicTo(ctrl1, ctrl2, point) => {
                let ctrl1 = map_point(transform, ctrl1);
                let ctrl2 = map_point(transform, ctrl2);
                let point = map_point(transform, point);
                commands.push(VectorPathCommand::CubicTo {
                    c1x: ctrl1.x,
                    c1y: ctrl1.y,
                    c2x: ctrl2.x,
                    c2y: ctrl2.y,
                    x: point.x,
                    y: point.y,
                });
            }
            PathSegment::Close => {
                commands.push(VectorPathCommand::ClosePath);
            }
        }
    }

    commands
}
