use editor_common::{Color, Rect};

use crate::backend::cpu::raster::{self, RasterScratch};
use crate::damage::IRect;
use crate::sink::RenderSink;
use crate::types::{Image, Path, Stroke, Transform};

fn write_unpremult(dst4: &mut [u8], src4: &[u8]) {
    let a = src4[3];
    if a == 255 {
        dst4.copy_from_slice(&[src4[0], src4[1], src4[2], 255]);
    } else if a == 0 {
        dst4.copy_from_slice(&[0, 0, 0, 0]);
    } else {
        let a32 = a as u32;
        dst4[0] = ((src4[0] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
        dst4[1] = ((src4[1] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
        dst4[2] = ((src4[2] as u32 * 255 + a32 / 2) / a32).min(255) as u8;
        dst4[3] = a;
    }
}

pub struct CpuSink {
    buf: Vec<u8>,
    width: u16,
    height: u16,
    scratch: RasterScratch,
    clip: Option<IRect>,
}

impl CpuSink {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            buf: vec![0u8; width as usize * height as usize * 4],
            width,
            height,
            scratch: RasterScratch::new(),
            clip: None,
        }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.buf = vec![0u8; width as usize * height as usize * 4];
            self.clip = None;
        }
    }

    pub fn set_clip(&mut self, clip: Option<IRect>) {
        self.clip = clip;
    }

    fn sink_bounds(&self) -> IRect {
        IRect {
            x0: 0,
            y0: 0,
            x1: self.width as i32,
            y1: self.height as i32,
        }
    }

    pub fn clear_rect(&mut self, r: IRect) {
        let Some(r) = r.intersect(self.sink_bounds()) else {
            return;
        };
        let pitch = self.width as usize * 4;
        for y in r.y0..r.y1 {
            let row = y as usize * pitch;
            for x in r.x0..r.x1 {
                self.buf[row + x as usize * 4..][..4].fill(0);
            }
        }
    }

    pub fn read_back_rect(&self, dst: &mut [u8], dst_stride: usize, r: IRect) {
        debug_assert!(dst_stride >= r.width() as usize * 4);
        debug_assert!(dst.len() >= r.height() as usize * dst_stride);
        let sb = self.sink_bounds();
        let pitch = self.width as usize * 4;
        for y in r.y0..r.y1 {
            for x in r.x0..r.x1 {
                let d_off = (y - r.y0) as usize * dst_stride + (x - r.x0) as usize * 4;
                let d = &mut dst[d_off..d_off + 4];
                if x < sb.x0 || x >= sb.x1 || y < sb.y0 || y >= sb.y1 {
                    d.copy_from_slice(&[0, 0, 0, 0]);
                    continue;
                }
                let s = &self.buf[y as usize * pitch + x as usize * 4..][..4];
                write_unpremult(d, s);
            }
        }
    }

    pub fn read_back_rect_absolute(&self, dst: &mut [u8], dst_stride: usize, r: IRect) {
        let Some(rc) = r.intersect(self.sink_bounds()) else {
            return;
        };
        debug_assert!(dst_stride >= self.width as usize * 4);
        debug_assert!(dst.len() >= rc.y1 as usize * dst_stride);
        let pitch = self.width as usize * 4;
        for y in rc.y0..rc.y1 {
            for x in rc.x0..rc.x1 {
                let d_off = y as usize * dst_stride + x as usize * 4;
                let s = &self.buf[y as usize * pitch + x as usize * 4..][..4];
                write_unpremult(&mut dst[d_off..d_off + 4], s);
            }
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
        let (x0, y0, x1, y1) = if let Some(c) = self.clip {
            (x0.max(c.x0), y0.max(c.y0), x1.min(c.x1), y1.min(c.y1))
        } else {
            (x0, y0, x1, y1)
        };
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

    fn blit_mask_at(&mut self, placement: zeno::Placement, color: Color) {
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
        let (x0, y0, x1, y1) = if let Some(c) = self.clip {
            (x0.max(c.x0), y0.max(c.y0), x1.min(c.x1), y1.min(c.y1))
        } else {
            (x0, y0, x1, y1)
        };
        if x0 >= x1 || y0 >= y1 {
            return;
        }

        let pitch = pw as usize * 4;
        let mask_pitch = iw as usize;
        let mask = raster::mask(&self.scratch);
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
        self.blit_mask_at(placement, color);
    }

    fn fill_path(&mut self, path: &Path, color: Color, transform: Transform) {
        let placement = raster::rasterize_fill_to_mask(&mut self.scratch, path, transform);
        self.blit_mask_at(placement, color);
    }

    fn stroke_path(&mut self, path: &Path, color: Color, stroke: &Stroke, transform: Transform) {
        let placement =
            raster::rasterize_stroke_to_mask(&mut self.scratch, path, stroke, transform);
        self.blit_mask_at(placement, color);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::damage::IRect;
    use crate::types::Transform;
    use editor_common::Color;

    fn opaque(sink: &mut CpuSink, r: editor_common::Rect) {
        sink.fill_rect(r, Color::new(255, 0, 0, 255), Transform::IDENTITY);
    }

    #[test]
    fn clip_confines_writes() {
        let mut s = CpuSink::new(10, 10);
        s.set_clip(Some(IRect {
            x0: 0,
            y0: 0,
            x1: 5,
            y1: 10,
        }));
        opaque(&mut s, editor_common::Rect::from_xywh(0.0, 0.0, 10.0, 10.0));
        s.set_clip(None);
        let mut dst = vec![0u8; 10 * 10 * 4];
        s.read_back_rect(
            &mut dst,
            10 * 4,
            IRect {
                x0: 0,
                y0: 0,
                x1: 10,
                y1: 10,
            },
        );
        let a = ((0 * 10 + 2) * 4) as usize;
        let b = ((0 * 10 + 7) * 4) as usize;
        assert_ne!(dst[a + 3], 0);
        assert_eq!(dst[b + 3], 0);
    }

    #[test]
    fn buffer_is_retained_across_readback() {
        let mut s = CpuSink::new(4, 4);
        opaque(&mut s, editor_common::Rect::from_xywh(0.0, 0.0, 4.0, 4.0));
        let mut dst1 = vec![0u8; 4 * 4 * 4];
        s.read_back_rect(
            &mut dst1,
            4 * 4,
            IRect {
                x0: 0,
                y0: 0,
                x1: 4,
                y1: 4,
            },
        );
        let mut dst2 = vec![0u8; 4 * 4 * 4];
        s.read_back_rect(
            &mut dst2,
            4 * 4,
            IRect {
                x0: 0,
                y0: 0,
                x1: 4,
                y1: 4,
            },
        );
        assert_eq!(dst1, dst2);
    }

    #[test]
    fn clear_rect_zeros_only_subrect() {
        let mut s = CpuSink::new(4, 4);
        opaque(&mut s, editor_common::Rect::from_xywh(0.0, 0.0, 4.0, 4.0));
        s.clear_rect(IRect {
            x0: 0,
            y0: 0,
            x1: 2,
            y1: 4,
        });
        let mut dst = vec![0u8; 4 * 4 * 4];
        s.read_back_rect(
            &mut dst,
            4 * 4,
            IRect {
                x0: 0,
                y0: 0,
                x1: 4,
                y1: 4,
            },
        );
        assert_eq!(dst[(0 * 4 + 1) * 4 + 3], 0);
        assert_ne!(dst[(0 * 4 + 3) * 4 + 3], 0);
    }

    #[test]
    fn read_back_rect_extracts_offset_subregion_from_larger_sink() {
        let mut s = CpuSink::new(8, 6);
        opaque(&mut s, editor_common::Rect::from_xywh(0.0, 0.0, 8.0, 6.0));
        s.fill_rect(
            editor_common::Rect::from_xywh(3.0, 2.0, 3.0, 2.0),
            Color::new(0, 255, 0, 255),
            Transform::IDENTITY,
        );

        let r = IRect {
            x0: 3,
            y0: 2,
            x1: 6,
            y1: 4,
        };
        let w = r.width() as usize;
        let h = r.height() as usize;
        let mut dst = vec![9u8; w * h * 4];
        s.read_back_rect(&mut dst, w * 4, r);

        for y in 0..h {
            for x in 0..w {
                let off = y * (w * 4) + x * 4;
                assert!(
                    dst[off + 1] > dst[off] && dst[off + 3] != 0,
                    "expected green at ({x},{y}), got {:?}",
                    &dst[off..off + 4]
                );
            }
        }

        let mut edge = vec![0u8; 4];
        s.read_back_rect(
            &mut edge,
            4,
            IRect {
                x0: 2,
                y0: 2,
                x1: 3,
                y1: 3,
            },
        );
        assert!(
            edge[0] > edge[1] && edge[3] != 0,
            "expected red at edge pixel, got {edge:?}"
        );
    }

    #[test]
    fn read_back_rect_out_of_bounds_source_keeps_destination_placement() {
        let mut s = CpuSink::new(4, 4);
        opaque(&mut s, editor_common::Rect::from_xywh(0.0, 0.0, 4.0, 4.0));
        let r = IRect {
            x0: -2,
            y0: 0,
            x1: 2,
            y1: 1,
        };
        let mut dst = vec![7u8; 4 * 4];
        s.read_back_rect(&mut dst, 4 * 4, r);
        assert_eq!(&dst[0..4], &[0, 0, 0, 0]);
        assert_ne!(dst[2 * 4 + 3], 0);
    }

    #[test]
    fn read_back_rect_absolute_places_subrect_at_absolute_position() {
        let mut s = CpuSink::new(10, 10);
        opaque(&mut s, editor_common::Rect::from_xywh(3.0, 8.0, 3.0, 1.0));

        let stride = 10 * 4;
        let mut dst = vec![0u8; 10 * 10 * 4];
        s.read_back_rect_absolute(
            &mut dst,
            stride,
            IRect {
                x0: 3,
                y0: 8,
                x1: 6,
                y1: 9,
            },
        );

        for y in 0..10 {
            for x in 0..10 {
                let off = y * stride + x * 4;
                let expect_opaque = y == 8 && (3..6).contains(&x);
                assert_eq!(
                    dst[off + 3] != 0,
                    expect_opaque,
                    "unexpected pixel state at ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn read_back_rect_absolute_clips_out_of_bounds_rect_without_panic() {
        let mut s = CpuSink::new(10, 10);
        opaque(&mut s, editor_common::Rect::from_xywh(0.0, 0.0, 10.0, 10.0));

        let stride = 10 * 4;
        let mut dst = vec![0u8; 10 * 10 * 4];
        s.read_back_rect_absolute(
            &mut dst,
            stride,
            IRect {
                x0: 8,
                y0: 8,
                x1: 20,
                y1: 20,
            },
        );

        for y in 0..10 {
            for x in 0..10 {
                let off = y * stride + x * 4;
                let expect_opaque = y >= 8 && x >= 8;
                assert_eq!(
                    dst[off + 3] != 0,
                    expect_opaque,
                    "unexpected pixel state at ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn read_back_rect_absolute_fully_outside_is_noop() {
        let mut s = CpuSink::new(10, 10);
        opaque(&mut s, editor_common::Rect::from_xywh(0.0, 0.0, 10.0, 10.0));

        let stride = 10 * 4;
        let mut dst = vec![0u8; 10 * 10 * 4];
        s.read_back_rect_absolute(
            &mut dst,
            stride,
            IRect {
                x0: 20,
                y0: 20,
                x1: 30,
                y1: 30,
            },
        );

        assert!(dst.iter().all(|&b| b == 0));
    }

    #[test]
    fn read_back_rect_absolute_negative_origin_clips_without_panic() {
        let mut s = CpuSink::new(10, 10);
        opaque(&mut s, editor_common::Rect::from_xywh(0.0, 0.0, 10.0, 10.0));

        let stride = 10 * 4;
        let mut dst = vec![0u8; 10 * 10 * 4];
        s.read_back_rect_absolute(
            &mut dst,
            stride,
            IRect {
                x0: -5,
                y0: -5,
                x1: 3,
                y1: 3,
            },
        );

        for y in 0..10 {
            for x in 0..10 {
                let off = y * stride + x * 4;
                let expect_opaque = y < 3 && x < 3;
                assert_eq!(
                    dst[off + 3] != 0,
                    expect_opaque,
                    "unexpected pixel state at ({x},{y})"
                );
            }
        }
    }
}
