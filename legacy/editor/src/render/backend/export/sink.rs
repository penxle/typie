use crate::render::glyph::Glyph;
use crate::render::glyph::rasterize::{RasterizedGlyph, rasterize_glyphs};
use crate::render::glyph::scale::image::Content;
use crate::render::sink::RenderSink;
use kurbo::{Affine, BezPath, PathEl, Rect, Stroke};
use parley::FontData;
use peniko::{Brush, Fill};
use serde::Serialize;

// ── Vector serialization types ───────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportFillRule {
    Winding,
    EvenOdd,
}

impl From<Fill> for ExportFillRule {
    fn from(value: Fill) -> Self {
        match value {
            Fill::NonZero => Self::Winding,
            Fill::EvenOdd => Self::EvenOdd,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportLineCap {
    Butt,
    Round,
    Square,
}

impl From<kurbo::Cap> for ExportLineCap {
    fn from(value: kurbo::Cap) -> Self {
        match value {
            kurbo::Cap::Butt => Self::Butt,
            kurbo::Cap::Round => Self::Round,
            kurbo::Cap::Square => Self::Square,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportLineJoin {
    Miter,
    Round,
    Bevel,
}

impl From<kurbo::Join> for ExportLineJoin {
    fn from(value: kurbo::Join) -> Self {
        match value {
            kurbo::Join::Miter { .. } => Self::Miter,
            kurbo::Join::Round => Self::Round,
            kurbo::Join::Bevel => Self::Bevel,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ExportPathCommand {
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
pub enum ExportOp {
    FillPath {
        path: Vec<ExportPathCommand>,
        color: [u8; 4],
        fill_rule: ExportFillRule,
    },
    StrokePath {
        path: Vec<ExportPathCommand>,
        color: [u8; 4],
        width: f32,
        line_cap: ExportLineCap,
        line_join: ExportLineJoin,
    },
    DrawImage {
        data: Vec<u8>,
        width: u32,
        height: u32,
        x: f32,
        y: f32,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTextOp {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub size: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPage {
    pub width: f32,
    pub height: f32,
    pub ops: Vec<ExportOp>,
    pub text_ops: Vec<ExportTextOp>,
}

// ── ExportSink ───────────────────────────────────────────────────────

pub struct ExportSink {
    ops: Vec<ExportOp>,
    text_ops: Vec<ExportTextOp>,
}

impl ExportSink {
    pub fn new() -> Self {
        Self {
            ops: Vec::new(),
            text_ops: Vec::new(),
        }
    }

    pub fn into_parts(self) -> (Vec<ExportOp>, Vec<ExportTextOp>) {
        (self.ops, self.text_ops)
    }
}

impl RenderSink for ExportSink {
    fn fill_rect(&mut self, rect: Rect, brush: &Brush, transform: Affine) {
        let mut bp = BezPath::new();
        bp.move_to((rect.x0, rect.y0));
        bp.line_to((rect.x1, rect.y0));
        bp.line_to((rect.x1, rect.y1));
        bp.line_to((rect.x0, rect.y1));
        bp.close_path();
        self.fill_path(&bp, brush, Fill::NonZero, transform);
    }

    fn fill_path(&mut self, path: &BezPath, brush: &Brush, fill: Fill, transform: Affine) {
        let commands = commands_from_bezpath(path, transform);
        if commands.is_empty() {
            return;
        }
        self.ops.push(ExportOp::FillPath {
            path: commands,
            color: brush_rgba(brush),
            fill_rule: fill.into(),
        });
    }

    fn stroke_path(&mut self, path: &BezPath, brush: &Brush, stroke: &Stroke, transform: Affine) {
        let commands = commands_from_bezpath(path, transform);
        if commands.is_empty() {
            return;
        }
        self.ops.push(ExportOp::StrokePath {
            path: commands,
            color: brush_rgba(brush),
            width: stroke.width as f32,
            line_cap: stroke.start_cap.into(),
            line_join: stroke.join.into(),
        });
    }

    fn draw_text(
        &mut self,
        text: &str,
        font: &FontData,
        font_size: f32,
        brush: &Brush,
        transform: Affine,
        glyph_transform: Option<Affine>,
        embolden: bool,
        glyphs: &[Glyph],
    ) {
        // Record text for web rendering
        if !text.is_empty() && font_size.is_finite() && font_size > 0.0 {
            let (x, y) = glyphs.first().map_or((0.0, 0.0), |g| (g.x, g.y));
            let point = transform * kurbo::Point::new(x as f64, y as f64);
            let px = point.x as f32;
            let py = point.y as f32;
            if px.is_finite() && py.is_finite() {
                self.text_ops.push(ExportTextOp {
                    text: text.to_string(),
                    x: px,
                    y: py,
                    size: font_size,
                });
            }
        }

        // Render glyphs
        let color = brush_rgba(brush);

        rasterize_glyphs(
            font,
            font_size,
            brush,
            transform,
            glyph_transform,
            embolden,
            glyphs,
            |g| match g {
                RasterizedGlyph::Path {
                    path, transform, ..
                } => {
                    let commands = commands_from_bezpath(&path, transform);
                    if !commands.is_empty() {
                        self.ops.push(ExportOp::FillPath {
                            path: commands,
                            color,
                            fill_rule: ExportFillRule::Winding,
                        });
                    }
                }
                RasterizedGlyph::Bitmap { image, x, y } => {
                    let p = &image.placement;
                    if p.width == 0 || p.height == 0 {
                        return;
                    }

                    let blit_x = x + p.left as f32;
                    let blit_y = y - p.top as f32;

                    let rgba_data = match image.content {
                        Content::Mask => {
                            let mut rgba = Vec::with_capacity(image.data.len() * 4);
                            for &alpha in &image.data {
                                let a = ((alpha as u16 * color[3] as u16) / 255) as u8;
                                rgba.push(color[0]);
                                rgba.push(color[1]);
                                rgba.push(color[2]);
                                rgba.push(a);
                            }
                            rgba
                        }
                        Content::Color | Content::SubpixelMask => image.data,
                    };

                    self.ops.push(ExportOp::DrawImage {
                        data: rgba_data,
                        width: p.width,
                        height: p.height,
                        x: blit_x,
                        y: blit_y,
                    });
                }
            },
        );
    }
}

// ── Helper functions ─────────────────────────────────────────────────

pub fn brush_rgba(brush: &Brush) -> [u8; 4] {
    match brush {
        Brush::Solid(color) => {
            let rgba = color.to_rgba8();
            [rgba.r, rgba.g, rgba.b, rgba.a]
        }
        _ => [0, 0, 0, 255],
    }
}

fn commands_from_bezpath(path: &BezPath, transform: Affine) -> Vec<ExportPathCommand> {
    let mut commands = Vec::new();
    for el in path.elements() {
        match *el {
            PathEl::MoveTo(p) => {
                let p = transform * p;
                commands.push(ExportPathCommand::MoveTo {
                    x: p.x as f32,
                    y: p.y as f32,
                });
            }
            PathEl::LineTo(p) => {
                let p = transform * p;
                commands.push(ExportPathCommand::LineTo {
                    x: p.x as f32,
                    y: p.y as f32,
                });
            }
            PathEl::QuadTo(ctrl, p) => {
                let ctrl = transform * ctrl;
                let p = transform * p;
                commands.push(ExportPathCommand::QuadTo {
                    cx: ctrl.x as f32,
                    cy: ctrl.y as f32,
                    x: p.x as f32,
                    y: p.y as f32,
                });
            }
            PathEl::CurveTo(c1, c2, p) => {
                let c1 = transform * c1;
                let c2 = transform * c2;
                let p = transform * p;
                commands.push(ExportPathCommand::CubicTo {
                    c1x: c1.x as f32,
                    c1y: c1.y as f32,
                    c2x: c2.x as f32,
                    c2y: c2.y as f32,
                    x: p.x as f32,
                    y: p.y as f32,
                });
            }
            PathEl::ClosePath => {
                commands.push(ExportPathCommand::ClosePath);
            }
        }
    }
    commands
}
