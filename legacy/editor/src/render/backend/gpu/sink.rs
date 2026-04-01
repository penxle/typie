use kurbo::{Affine, BezPath, Rect, Stroke};
use peniko::{Brush, Fill};
use vello::Scene;

use crate::render::glyph::Glyph;
use crate::render::glyph::rasterize::{RasterizedGlyph, rasterize_glyphs};
use crate::render::glyph::scale::image::{Content, Image};
use crate::render::sink::RenderSink;
use parley::FontData;
use peniko;
use std::sync::Arc;

pub struct GpuSink {
    scene: Scene,
}

impl GpuSink {
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
        }
    }

    pub fn into_scene(self) -> Scene {
        self.scene
    }

    fn add_image_to_scene(&mut self, image: &Image, x: f32, y: f32) {
        let p = &image.placement;
        if p.width == 0 || p.height == 0 {
            return;
        }

        let rgba_data = match image.content {
            Content::Mask => {
                let mut rgba = Vec::with_capacity(image.data.len() * 4);
                for &alpha in &image.data {
                    rgba.push(255);
                    rgba.push(255);
                    rgba.push(255);
                    rgba.push(alpha);
                }
                rgba
            }
            Content::Color | Content::SubpixelMask => image.data.clone(),
        };

        let blob = peniko::Blob::new(Arc::new(rgba_data));
        let image_data = peniko::ImageData {
            data: blob,
            format: peniko::ImageFormat::Rgba8,
            alpha_type: peniko::ImageAlphaType::AlphaPremultiplied,
            width: p.width,
            height: p.height,
        };
        let image_brush = peniko::ImageBrush::new(image_data);

        let blit_x = x + p.left as f32;
        let blit_y = y - p.top as f32;
        let affine = Affine::translate((blit_x as f64, blit_y as f64));
        self.scene.draw_image(&image_brush, affine);
    }
}

impl RenderSink for GpuSink {
    fn fill_rect(&mut self, rect: Rect, brush: &Brush, transform: Affine) {
        self.scene
            .fill(Fill::NonZero, transform, brush, None, &rect);
    }

    fn fill_path(&mut self, path: &BezPath, brush: &Brush, fill: Fill, transform: Affine) {
        self.scene.fill(fill, transform, brush, None, path);
    }

    fn stroke_path(&mut self, path: &BezPath, brush: &Brush, stroke: &Stroke, transform: Affine) {
        self.scene.stroke(stroke, transform, brush, None, path);
    }

    fn draw_text(
        &mut self,
        _text: &str,
        font: &FontData,
        font_size: f32,
        brush: &Brush,
        transform: Affine,
        glyph_transform: Option<Affine>,
        embolden: bool,
        glyphs: &[Glyph],
    ) {
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
                    path,
                    brush,
                    fill,
                    transform,
                } => {
                    self.scene.fill(fill, transform, brush, None, &path);
                }
                RasterizedGlyph::Bitmap { image, x, y } => {
                    self.add_image_to_scene(&image, x, y);
                }
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use peniko::color::AlphaColor;

    fn solid_brush(r: u8, g: u8, b: u8) -> Brush {
        Brush::Solid(AlphaColor::from_rgba8(r, g, b, 255))
    }

    #[test]
    fn fill_rect_adds_to_scene() {
        let mut sink = GpuSink::new();
        let rect = Rect::new(10.0, 20.0, 110.0, 70.0);
        sink.fill_rect(rect, &solid_brush(255, 0, 0), Affine::IDENTITY);
        let scene = sink.into_scene();
        assert!(!scene.encoding().is_empty());
    }

    #[test]
    fn fill_path_adds_to_scene() {
        let mut sink = GpuSink::new();
        let mut bp = BezPath::new();
        bp.move_to((0.0, 0.0));
        bp.line_to((100.0, 0.0));
        bp.line_to((50.0, 80.0));
        bp.close_path();
        sink.fill_path(
            &bp,
            &solid_brush(0, 255, 0),
            Fill::NonZero,
            Affine::IDENTITY,
        );
        let scene = sink.into_scene();
        assert!(!scene.encoding().is_empty());
    }

    #[test]
    fn stroke_path_adds_to_scene() {
        let mut sink = GpuSink::new();
        let mut bp = BezPath::new();
        bp.move_to((0.0, 0.0));
        bp.line_to((100.0, 100.0));
        let stroke = Stroke::new(2.0);
        sink.stroke_path(&bp, &solid_brush(0, 0, 255), &stroke, Affine::IDENTITY);
        let scene = sink.into_scene();
        assert!(!scene.encoding().is_empty());
    }

    #[test]
    fn empty_scene_when_no_ops() {
        let sink = GpuSink::new();
        let scene = sink.into_scene();
        assert!(scene.encoding().is_empty());
    }

    #[test]
    fn transform_applied_to_fill_rect() {
        let mut sink = GpuSink::new();
        let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
        let transform = Affine::translate((50.0, 50.0));
        sink.fill_rect(rect, &solid_brush(128, 128, 128), transform);
        let scene = sink.into_scene();
        assert!(!scene.encoding().is_empty());
    }
}
