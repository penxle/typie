use editor_common::Rect;
use editor_resource::FontRegistry;
use editor_view::glyph_run::GlyphRun;

use crate::types::Color;
use crate::types::{Image, Path, Stroke, Transform};

pub trait RenderSink {
    fn pixel_size(&self) -> (u32, u32);
    fn fill_rect(&mut self, rect: Rect, color: Color, transform: Transform);
    fn fill_path(&mut self, path: &Path, color: Color, transform: Transform);
    fn stroke_path(&mut self, path: &Path, color: Color, stroke: &Stroke, transform: Transform);
    fn draw_image(&mut self, image: &Image, rect: Rect, transform: Transform);
    fn draw_glyph(&mut self, image: &Image, dst_x: i32, dst_y: i32) {
        let rect = Rect::from_xywh(0.0, 0.0, image.width as f32, image.height as f32);
        let transform = Transform::IDENTITY.translate(dst_x as f32, dst_y as f32);
        self.draw_image(image, rect, transform);
    }
    fn draw_glyph_run(
        &mut self,
        _run: &GlyphRun,
        _color: Color,
        _transform: Transform,
        _fonts: &FontRegistry,
    ) {
    }
}
