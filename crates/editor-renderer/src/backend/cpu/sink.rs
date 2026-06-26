use editor_common::{Color, Rect};

use crate::backend::cpu::raster::{self, RasterScratch};
use crate::sink::RenderSink;
use crate::types::{Image, Path, Stroke, Transform};

pub struct CpuSink {
    buf: Vec<u8>,
    width: u16,
    height: u16,
    scratch: RasterScratch,
}

impl CpuSink {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            buf: vec![0u8; width as usize * height as usize * 4],
            width,
            height,
            scratch: RasterScratch::new(),
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.buf = vec![0u8; width as usize * height as usize * 4];
        }
    }

    fn blit_premul_at(&mut self, src: &[u8], iw: i32, ih: i32, dst_x: i32, dst_y: i32) {
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

        let pitch = pw as usize * 4;
        let src_pitch = (iw as usize) * 4;
        let dst = &mut self.buf;

        for y in y0..y1 {
            let src_row = ((y - dst_y) as usize) * src_pitch;
            let dst_row = (y as usize) * pitch;
            for x in x0..x1 {
                let si = src_row + ((x - dst_x) as usize) * 4;
                let sa = src[si + 3] as u32;
                if sa == 0 {
                    continue;
                }
                let di = dst_row + (x as usize) * 4;
                if sa == 255 {
                    dst[di] = src[si];
                    dst[di + 1] = src[si + 1];
                    dst[di + 2] = src[si + 2];
                    dst[di + 3] = 255;
                } else {
                    let inv = 255 - sa;
                    dst[di] = (src[si] as u32 + ((dst[di] as u32 * inv) >> 8)).min(255) as u8;
                    dst[di + 1] =
                        (src[si + 1] as u32 + ((dst[di + 1] as u32 * inv) >> 8)).min(255) as u8;
                    dst[di + 2] =
                        (src[si + 2] as u32 + ((dst[di + 2] as u32 * inv) >> 8)).min(255) as u8;
                    dst[di + 3] = (sa + ((dst[di + 3] as u32 * inv) >> 8)).min(255) as u8;
                }
            }
        }
    }

    fn blit_mask_at(&mut self, mask: &[u8], placement: zeno::Placement, color: Color) {
        let iw = placement.width as i32;
        let ih = placement.height as i32;
        if iw <= 0 || ih <= 0 {
            return;
        }
        let dst_x = placement.left;
        let dst_y = placement.top;

        let pw = self.width as i32;
        let ph = self.height as i32;

        let x0 = dst_x.max(0);
        let y0 = dst_y.max(0);
        let x1 = (dst_x + iw).min(pw);
        let y1 = (dst_y + ih).min(ph);
        if x0 >= x1 || y0 >= y1 {
            return;
        }

        let pitch = pw as usize * 4;
        let mask_pitch = iw as usize;
        let dst = &mut self.buf;

        for y in y0..y1 {
            let mask_row = ((y - dst_y) as usize) * mask_pitch;
            let dst_row = (y as usize) * pitch;
            for x in x0..x1 {
                let m = mask[mask_row + (x - dst_x) as usize];
                let [pr, pg, pb, pa] = raster::premul_pixel(m, color);
                let sa = pa as u32;
                if sa == 0 {
                    continue;
                }
                let di = dst_row + (x as usize) * 4;
                if sa == 255 {
                    dst[di] = pr;
                    dst[di + 1] = pg;
                    dst[di + 2] = pb;
                    dst[di + 3] = 255;
                } else {
                    let inv = 255 - sa;
                    dst[di] = (pr as u32 + ((dst[di] as u32 * inv) >> 8)).min(255) as u8;
                    dst[di + 1] = (pg as u32 + ((dst[di + 1] as u32 * inv) >> 8)).min(255) as u8;
                    dst[di + 2] = (pb as u32 + ((dst[di + 2] as u32 * inv) >> 8)).min(255) as u8;
                    dst[di + 3] = (sa + ((dst[di + 3] as u32 * inv) >> 8)).min(255) as u8;
                }
            }
        }
    }

    pub fn flush_to(&mut self, dst: &mut [u8]) {
        for (d, s) in dst.chunks_exact_mut(4).zip(self.buf.chunks_exact_mut(4)) {
            let a = s[3];
            if a == 255 {
                d[0] = s[0];
                d[1] = s[1];
                d[2] = s[2];
                d[3] = 255;
            } else if a == 0 {
                d[0] = 0;
                d[1] = 0;
                d[2] = 0;
                d[3] = 0;
            } else {
                let a32 = a as u32;
                d[0] = ((s[0] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
                d[1] = ((s[1] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
                d[2] = ((s[2] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
                d[3] = a;
            }
            s[0] = 0;
            s[1] = 0;
            s[2] = 0;
            s[3] = 0;
        }
    }
}

impl RenderSink for CpuSink {
    fn pixel_size(&self) -> (u32, u32) {
        (self.width as u32, self.height as u32)
    }

    fn fill_rect(&mut self, rect: Rect, color: Color, transform: Transform) {
        let path = crate::types::Path::rect(rect);
        let placement = raster::rasterize_fill_to_mask(&mut self.scratch, &path, transform);
        let mask = raster::mask(&self.scratch).to_vec();
        self.blit_mask_at(&mask, placement, color);
    }

    fn fill_path(&mut self, path: &Path, color: Color, transform: Transform) {
        let placement = raster::rasterize_fill_to_mask(&mut self.scratch, path, transform);
        let mask = raster::mask(&self.scratch).to_vec();
        self.blit_mask_at(&mask, placement, color);
    }

    fn stroke_path(&mut self, path: &Path, color: Color, stroke: &Stroke, transform: Transform) {
        let placement =
            raster::rasterize_stroke_to_mask(&mut self.scratch, path, stroke, transform);
        let mask = raster::mask(&self.scratch).to_vec();
        self.blit_mask_at(&mask, placement, color);
    }

    fn draw_image(&mut self, _image: &Image, _rect: Rect, _transform: Transform) {}

    fn draw_glyph(&mut self, image: &Image, dst_x: i32, dst_y: i32) {
        self.blit_premul_at(
            &image.data,
            image.width as i32,
            image.height as i32,
            dst_x,
            dst_y,
        );
    }
}
