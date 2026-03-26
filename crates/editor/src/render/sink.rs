use crate::render::glyph::Glyph;
use kurbo::{Affine, BezPath, Rect, Stroke};
use parley::FontData;
use peniko::{Brush, Fill};

// ── RenderSink trait ────────────────────────────────────────────────

pub trait RenderSink {
    fn fill_rect(&mut self, rect: Rect, brush: &Brush, transform: Affine);
    fn fill_path(&mut self, path: &BezPath, brush: &Brush, fill: Fill, transform: Affine);
    fn stroke_path(&mut self, path: &BezPath, brush: &Brush, stroke: &Stroke, transform: Affine);
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
    );
}
