use crate::render::glyph::Glyph;
use crate::render::glyph::rasterize::{RasterizedGlyph, rasterize_glyphs};
use crate::render::glyph::scale::image::{Content, Image};
use crate::render::sink::RenderSink;
use kurbo::{Affine, BezPath, Rect, Stroke};
use parley::FontData;
use peniko::color::PremulRgba8;
use peniko::{Brush, Fill};
use std::sync::Arc;
use vello_cpu::RenderContext;

pub struct CpuSink {
    ctx: RenderContext,
}

impl CpuSink {
    pub fn new(width: u16, height: u16) -> Self {
        let settings = vello_cpu::RenderSettings {
            level: vello_cpu::Level::new(),
            num_threads: 0,
            render_mode: vello_cpu::RenderMode::OptimizeSpeed,
        };
        Self {
            ctx: RenderContext::new_with(width, height, settings),
        }
    }

    pub fn flush_to(&mut self, dst: &mut [u8], width: u16, height: u16) {
        self.ctx.flush();
        self.ctx
            .render_to_buffer(dst, width, height, vello_cpu::RenderMode::OptimizeSpeed);
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.ctx.reset();
    }
}

/// peniko::Brush → vello_cpu set_paint용 색상 추출
fn set_paint_from_brush(ctx: &mut RenderContext, brush: &Brush) {
    match brush {
        Brush::Solid(color) => {
            ctx.set_paint(*color);
        }
        _ => {
            ctx.set_paint(peniko::color::AlphaColor::from_rgba8(0, 0, 0, 255));
        }
    }
}

impl RenderSink for CpuSink {
    fn fill_rect(&mut self, rect: Rect, brush: &Brush, transform: Affine) {
        self.ctx.set_transform(transform);
        set_paint_from_brush(&mut self.ctx, brush);
        self.ctx.fill_rect(&rect);
    }

    fn fill_path(&mut self, path: &BezPath, brush: &Brush, fill: Fill, transform: Affine) {
        self.ctx.set_transform(transform);
        set_paint_from_brush(&mut self.ctx, brush);
        self.ctx.set_fill_rule(fill);
        self.ctx.fill_path(path);
    }

    fn stroke_path(&mut self, path: &BezPath, brush: &Brush, stroke: &Stroke, transform: Affine) {
        self.ctx.set_transform(transform);
        set_paint_from_brush(&mut self.ctx, brush);
        self.ctx.set_stroke(stroke.clone());
        self.ctx.stroke_path(path);
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
        set_paint_from_brush(&mut self.ctx, brush);

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
                    self.ctx.set_transform(transform);
                    self.ctx.fill_path(&path);
                }
                RasterizedGlyph::Bitmap { image, x, y } => {
                    self.add_image(&image, x, y);
                }
            },
        );
    }
}

impl CpuSink {
    fn add_image(&mut self, image: &Image, x: f32, y: f32) {
        let p = &image.placement;
        if p.width == 0 || p.height == 0 {
            return;
        }

        // RGBA u8 → PremulRgba8 벡터 변환
        let premul_pixels: Vec<PremulRgba8> = match image.content {
            Content::Mask => image
                .data
                .iter()
                .map(|&alpha| PremulRgba8 {
                    r: alpha,
                    g: alpha,
                    b: alpha,
                    a: alpha,
                })
                .collect(),
            Content::Color | Content::SubpixelMask => image
                .data
                .chunks_exact(4)
                .map(|c| PremulRgba8 {
                    r: c[0],
                    g: c[1],
                    b: c[2],
                    a: c[3],
                })
                .collect(),
        };

        let pixmap = vello_cpu::Pixmap::from_parts(premul_pixels, p.width as u16, p.height as u16);
        let image_source = vello_cpu::ImageSource::Pixmap(Arc::new(pixmap));
        let image_brush = vello_cpu::Image {
            image: image_source,
            sampler: Default::default(),
        };

        let blit_x = x + p.left as f32;
        let blit_y = y - p.top as f32;
        self.ctx
            .set_transform(Affine::translate((blit_x as f64, blit_y as f64)));
        self.ctx.set_paint(image_brush);
        self.ctx
            .fill_rect(&Rect::new(0.0, 0.0, p.width as f64, p.height as f64));
    }
}
