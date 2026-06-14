use editor_common::Rect;
use peniko::color::PremulRgba8;
use std::sync::Arc;

use crate::sink::RenderSink;
use crate::types::{Color, Image, Path, Stroke, Transform};

const TRANSPARENT: PremulRgba8 = PremulRgba8 {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};

pub struct CpuSink {
    ctx: vello_cpu::RenderContext,
    resources: vello_cpu::Resources,
    pixmap: vello_cpu::Pixmap,
    width: u16,
    height: u16,
    has_pending_vello: bool,
}

impl CpuSink {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            ctx: Self::create_context(width, height),
            resources: vello_cpu::Resources::new(),
            pixmap: vello_cpu::Pixmap::new(width, height),
            width,
            height,
            has_pending_vello: false,
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.ctx = Self::create_context(width, height);
            self.pixmap = vello_cpu::Pixmap::new(width, height);
            self.has_pending_vello = false;
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

    fn flush_pending_vello(&mut self) {
        if !self.has_pending_vello {
            return;
        }
        self.ctx.flush();
        self.ctx
            .composite_to_pixmap_at_offset(&self.resources, &mut self.pixmap, 0, 0);
        self.ctx.reset();
        self.has_pending_vello = false;
    }

    fn blit_glyph(&mut self, image: &Image, dst_x: i32, dst_y: i32) {
        let iw = image.width as i32;
        let ih = image.height as i32;
        if iw <= 0 || ih <= 0 {
            return;
        }
        let pw = self.width as i32;
        let ph = self.height as i32;

        let x0 = dst_x.max(0);
        let y0 = dst_y.max(0);
        let x1 = (dst_x + iw).min(pw);
        let y1 = (dst_y + ih).min(ph);
        if x0 >= x1 || y0 >= y1 {
            return;
        }

        let pitch = pw as usize;
        let src_pitch = (iw as usize) * 4;
        let src = &image.data;
        let dst = self.pixmap.data_mut();

        for y in y0..y1 {
            let src_row = ((y - dst_y) as usize) * src_pitch;
            let dst_row = (y as usize) * pitch;
            for x in x0..x1 {
                let si = src_row + ((x - dst_x) as usize) * 4;
                let sa = src[si + 3] as u32;
                if sa == 0 {
                    continue;
                }
                let di = dst_row + x as usize;
                if sa == 255 {
                    dst[di] = PremulRgba8 {
                        r: src[si],
                        g: src[si + 1],
                        b: src[si + 2],
                        a: 255,
                    };
                } else {
                    let inv = 255 - sa;
                    let d = dst[di];
                    dst[di] = PremulRgba8 {
                        r: (src[si] as u32 + ((d.r as u32 * inv) >> 8)).min(255) as u8,
                        g: (src[si + 1] as u32 + ((d.g as u32 * inv) >> 8)).min(255) as u8,
                        b: (src[si + 2] as u32 + ((d.b as u32 * inv) >> 8)).min(255) as u8,
                        a: (sa + ((d.a as u32 * inv) >> 8)).min(255) as u8,
                    };
                }
            }
        }
    }

    pub fn flush_to(&mut self, dst: &mut [u8]) {
        self.flush_pending_vello();

        for (d, s) in dst
            .chunks_exact_mut(4)
            .zip(self.pixmap.data_mut().iter_mut())
        {
            let a = s.a;
            if a == 255 {
                d[0] = s.r;
                d[1] = s.g;
                d[2] = s.b;
                d[3] = 255;
            } else if a == 0 {
                d[0] = 0;
                d[1] = 0;
                d[2] = 0;
                d[3] = 0;
            } else {
                let a32 = a as u32;
                d[0] = ((s.r as u32 * 255 + a32 / 2) / a32).min(255) as u8;
                d[1] = ((s.g as u32 * 255 + a32 / 2) / a32).min(255) as u8;
                d[2] = ((s.b as u32 * 255 + a32 / 2) / a32).min(255) as u8;
                d[3] = a;
            }
            *s = TRANSPARENT;
        }
    }
}

impl RenderSink for CpuSink {
    fn pixel_size(&self) -> (u32, u32) {
        (self.width as u32, self.height as u32)
    }

    fn fill_rect(&mut self, rect: Rect, color: Color, transform: Transform) {
        self.ctx.set_transform(transform.into());
        self.ctx.set_paint(crate::types::to_peniko(color));
        let r = kurbo::Rect::new(
            rect.x as f64,
            rect.y as f64,
            (rect.x + rect.width) as f64,
            (rect.y + rect.height) as f64,
        );
        self.ctx.fill_rect(&r);
        self.has_pending_vello = true;
    }

    fn fill_path(&mut self, path: &Path, color: Color, transform: Transform) {
        self.ctx.set_transform(transform.into());
        self.ctx.set_paint(crate::types::to_peniko(color));
        self.ctx.fill_path(&kurbo::BezPath::from(path));
        self.has_pending_vello = true;
    }

    fn stroke_path(&mut self, path: &Path, color: Color, stroke: &Stroke, transform: Transform) {
        self.ctx.set_transform(transform.into());
        self.ctx.set_paint(crate::types::to_peniko(color));
        let mut ks = kurbo::Stroke::new(stroke.width as f64);
        ks.start_cap = stroke.cap.into();
        ks.end_cap = stroke.cap.into();
        ks.join = stroke.join.into();
        self.ctx.set_stroke(ks);
        self.ctx.stroke_path(&kurbo::BezPath::from(path));
        self.has_pending_vello = true;
    }

    fn draw_glyph(&mut self, image: &Image, dst_x: i32, dst_y: i32) {
        self.flush_pending_vello();
        self.blit_glyph(image, dst_x, dst_y);
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
        // Nearest + 정수 translate 조건에서 byte-exact 1:1 복사 (POC 검증)
        let image_brush = vello_cpu::Image {
            image: image_source,
            sampler: peniko::ImageSampler {
                x_extend: peniko::Extend::Pad,
                y_extend: peniko::Extend::Pad,
                quality: peniko::ImageQuality::Low,
                alpha: 1.0,
            },
        };

        self.ctx.set_transform(transform.into());
        self.ctx.set_paint(image_brush);
        self.ctx.fill_rect(&kurbo::Rect::new(
            0.0,
            0.0,
            image.width as f64,
            image.height as f64,
        ));
        self.has_pending_vello = true;
    }
}
