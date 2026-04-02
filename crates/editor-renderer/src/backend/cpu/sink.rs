use editor_common::Rect;
use std::sync::Arc;

use crate::sink::RenderSink;
use crate::types::{Color, Image, Path, Stroke, Transform};

pub struct CpuSink {
    ctx: vello_cpu::RenderContext,
    width: u16,
    height: u16,
}

impl CpuSink {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            ctx: Self::create_context(width, height),
            width,
            height,
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.ctx = Self::create_context(width, height);
        }
    }

    pub fn create_context(width: u16, height: u16) -> vello_cpu::RenderContext {
        let settings = vello_cpu::RenderSettings {
            level: vello_cpu::Level::new(),
            num_threads: 0,
            render_mode: vello_cpu::RenderMode::OptimizeSpeed,
        };

        vello_cpu::RenderContext::new_with(width, height, settings)
    }

    pub fn flush_to(&mut self, dst: &mut [u8]) {
        self.ctx.flush();
        self.ctx.render_to_buffer(
            dst,
            self.width,
            self.height,
            vello_cpu::RenderMode::OptimizeSpeed,
        );

        self.ctx.reset();
    }
}

impl RenderSink for CpuSink {
    fn fill_rect(&mut self, rect: Rect, color: Color, transform: Transform) {
        self.ctx.set_transform(transform.into());
        self.ctx.set_paint(peniko::color::AlphaColor::from(color));
        let r = kurbo::Rect::new(
            rect.x as f64,
            rect.y as f64,
            (rect.x + rect.width) as f64,
            (rect.y + rect.height) as f64,
        );
        self.ctx.fill_rect(&r);
    }

    fn fill_path(&mut self, path: &Path, color: Color, transform: Transform) {
        self.ctx.set_transform(transform.into());
        self.ctx.set_paint(peniko::color::AlphaColor::from(color));
        self.ctx.fill_path(&kurbo::BezPath::from(path));
    }

    fn stroke_path(&mut self, path: &Path, color: Color, stroke: &Stroke, transform: Transform) {
        self.ctx.set_transform(transform.into());
        self.ctx.set_paint(peniko::color::AlphaColor::from(color));
        self.ctx.set_stroke(kurbo::Stroke::new(stroke.width as f64));
        self.ctx.stroke_path(&kurbo::BezPath::from(path));
    }

    fn draw_image(&mut self, image: &Image, _rect: Rect, transform: Transform) {
        if image.width == 0 || image.height == 0 {
            return;
        }

        let premul_pixels: Vec<peniko::color::PremulRgba8> = image
            .data
            .chunks_exact(4)
            .map(|c| peniko::color::PremulRgba8 {
                r: c[0],
                g: c[1],
                b: c[2],
                a: c[3],
            })
            .collect();

        let pixmap =
            vello_cpu::Pixmap::from_parts(premul_pixels, image.width as u16, image.height as u16);
        let image_source = vello_cpu::ImageSource::Pixmap(Arc::new(pixmap));
        let image_brush = vello_cpu::Image {
            image: image_source,
            sampler: Default::default(),
        };

        self.ctx.set_transform(transform.into());
        self.ctx.set_paint(image_brush);
        self.ctx.fill_rect(&kurbo::Rect::new(
            0.0,
            0.0,
            image.width as f64,
            image.height as f64,
        ));
    }
}
