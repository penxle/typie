use editor_common::Rect;
use editor_resource::FontRegistry;
use editor_view::glyph_run::GlyphRun;

use crate::sink::RenderSink;
use crate::types::{Color, Image, Path, Stroke, Transform};

pub struct TranslatedSink<'a> {
    inner: &'a mut dyn RenderSink,
    dx: f32,
    dy: f32,
}

impl<'a> TranslatedSink<'a> {
    pub fn new(inner: &'a mut dyn RenderSink, dx: f32, dy: f32) -> Self {
        Self { inner, dx, dy }
    }

    fn shift(&self, transform: Transform) -> Transform {
        transform.translate_device(self.dx, self.dy)
    }
}

impl RenderSink for TranslatedSink<'_> {
    fn pixel_size(&self) -> (u32, u32) {
        self.inner.pixel_size()
    }

    fn fill_rect(&mut self, rect: Rect, color: Color, transform: Transform) {
        let transform = self.shift(transform);
        self.inner.fill_rect(rect, color, transform);
    }

    fn fill_path(&mut self, path: &Path, color: Color, transform: Transform) {
        let transform = self.shift(transform);
        self.inner.fill_path(path, color, transform);
    }

    fn stroke_path(&mut self, path: &Path, color: Color, stroke: &Stroke, transform: Transform) {
        let transform = self.shift(transform);
        self.inner.stroke_path(path, color, stroke, transform);
    }

    fn draw_image(&mut self, image: &Image, rect: Rect, transform: Transform) {
        let transform = self.shift(transform);
        self.inner.draw_image(image, rect, transform);
    }

    fn draw_glyph(&mut self, image: &Image, dst_x: i32, dst_y: i32) {
        self.inner
            .draw_glyph(image, dst_x + self.dx as i32, dst_y + self.dy as i32);
    }

    fn draw_glyph_run(
        &mut self,
        run: &GlyphRun,
        color: Color,
        transform: Transform,
        fonts: &FontRegistry,
    ) {
        let transform = self.shift(transform);
        self.inner.draw_glyph_run(run, color, transform, fonts);
    }
}

#[cfg(test)]
mod tests {
    use editor_common::Rect;

    use super::*;
    use crate::backend::cpu::CpuSink;
    use crate::types::{Color, Transform};

    fn red() -> Color {
        Color::new(255, 0, 0, 255)
    }

    #[test]
    fn fill_rect_is_shifted_by_device_offset() {
        let mut direct = CpuSink::new(16, 16);
        direct.fill_rect(
            Rect::from_xywh(4.0, 3.0, 4.0, 4.0),
            red(),
            Transform::IDENTITY,
        );

        let mut base = CpuSink::new(16, 16);
        let mut translated = TranslatedSink::new(&mut base, 3.0, 2.0);
        translated.fill_rect(
            Rect::from_xywh(1.0, 1.0, 4.0, 4.0),
            red(),
            Transform::IDENTITY,
        );

        assert_eq!(base.pixels(), direct.pixels());
    }

    #[test]
    fn device_shift_applies_after_local_transform() {
        let mut direct = CpuSink::new(16, 16);
        direct.fill_rect(
            Rect::from_xywh(1.0, 1.0, 2.0, 2.0),
            red(),
            Transform::scale(2.0).translate_device(-1.0, -2.0),
        );

        let mut base = CpuSink::new(16, 16);
        let mut translated = TranslatedSink::new(&mut base, -1.0, -2.0);
        translated.fill_rect(
            Rect::from_xywh(1.0, 1.0, 2.0, 2.0),
            red(),
            Transform::scale(2.0),
        );

        assert_eq!(base.pixels(), direct.pixels());
    }

    #[test]
    fn draw_glyph_offsets_integer_destination() {
        let image = crate::types::Image {
            width: 2,
            height: 2,
            data: std::sync::Arc::<[u8]>::from(vec![255u8; 2 * 2 * 4]),
            glyph: None,
        };

        let mut direct = CpuSink::new(16, 16);
        direct.draw_glyph(&image, 5, 7);

        let mut base = CpuSink::new(16, 16);
        let mut translated = TranslatedSink::new(&mut base, 2.0, 3.0);
        translated.draw_glyph(&image, 3, 4);

        assert_eq!(base.pixels(), direct.pixels());
    }
}
