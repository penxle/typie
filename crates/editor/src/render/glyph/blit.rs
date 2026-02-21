#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;
use tiny_skia::PixmapMut;

#[inline(always)]
fn pack_rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    u32::from_ne_bytes([r, g, b, a])
}

#[inline(always)]
fn blend_packed(src: u32, dst: u32, scale: u32, inv_scale: u32) -> u32 {
    let rb = ((src & 0x00FF00FF) * scale + (dst & 0x00FF00FF) * inv_scale) >> 8;
    let ag = (((src >> 8) & 0x00FF00FF) * scale + ((dst >> 8) & 0x00FF00FF) * inv_scale) >> 8;
    (rb & 0x00FF00FF) | ((ag & 0x00FF00FF) << 8)
}

#[inline(always)]
fn scale_packed(pixel: u32, scale256: u32) -> u32 {
    let rb = ((pixel & 0x00FF00FF) * scale256) >> 8;
    let ag = (((pixel >> 8) & 0x00FF00FF) * scale256) >> 8;
    (rb & 0x00FF00FF) | ((ag & 0x00FF00FF) << 8)
}

#[inline(always)]
fn add_sat_u8x4(a: u32, b: u32) -> u32 {
    let s = a.wrapping_add(b);
    let carries = ((a & b) | ((a | b) & !s)) & 0x80808080;
    let overflow_mask = carries | carries.wrapping_sub(carries >> 7);
    s | overflow_mask
}

#[inline(always)]
unsafe fn as_pixels_mut(data: &mut [u8]) -> &mut [u32] {
    unsafe { std::slice::from_raw_parts_mut(data.as_mut_ptr() as *mut u32, data.len() >> 2) }
}

#[inline(always)]
unsafe fn as_pixels(data: &[u8]) -> &[u32] {
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u32, data.len() >> 2) }
}

pub(crate) fn blit_mask_d32_a8(
    dst: &mut PixmapMut,
    mask_data: &[u8],
    mask_width: u32,
    mask_height: u32,
    dst_x: i32,
    dst_y: i32,
    color_r: u8,
    color_g: u8,
    color_b: u8,
    color_a: u8,
) {
    if color_a == 0 {
        return;
    }

    if color_a == 255 {
        blit_mask_opaque(
            dst,
            mask_data,
            mask_width,
            mask_height,
            dst_x,
            dst_y,
            color_r,
            color_g,
            color_b,
        );
    } else {
        blit_mask_general(
            dst,
            mask_data,
            mask_width,
            mask_height,
            dst_x,
            dst_y,
            color_r,
            color_g,
            color_b,
            color_a,
        );
    }
}

fn blit_mask_opaque(
    dst: &mut PixmapMut,
    mask_data: &[u8],
    mask_width: u32,
    mask_height: u32,
    dst_x: i32,
    dst_y: i32,
    color_r: u8,
    color_g: u8,
    color_b: u8,
) {
    let (sx, sy, dx, dy, w, h) = compute_clip(dst, mask_width, mask_height, dst_x, dst_y);
    if w <= 0 || h <= 0 {
        return;
    }

    let mask_stride = mask_width as usize;
    let dst_width = dst.width() as usize;
    let dst_pixels = unsafe { as_pixels_mut(dst.data_mut()) };
    let src = pack_rgba(color_r, color_g, color_b, 255);
    let w = w as usize;
    let h = h as usize;
    let sx = sx as usize;
    let sy = sy as usize;
    let dx = dx as usize;
    let dy = dy as usize;

    for row in 0..h {
        let mask_off = (sy + row) * mask_stride + sx;
        let dst_off = (dy + row) * dst_width + dx;
        unsafe {
            row_blend_opaque(
                dst_pixels.as_mut_ptr().add(dst_off),
                mask_data.as_ptr().add(mask_off),
                src,
                w,
            );
        }
    }
}

fn blit_mask_general(
    dst: &mut PixmapMut,
    mask_data: &[u8],
    mask_width: u32,
    mask_height: u32,
    dst_x: i32,
    dst_y: i32,
    color_r: u8,
    color_g: u8,
    color_b: u8,
    color_a: u8,
) {
    let (sx, sy, dx, dy, w, h) = compute_clip(dst, mask_width, mask_height, dst_x, dst_y);
    if w <= 0 || h <= 0 {
        return;
    }

    let mask_stride = mask_width as usize;
    let dst_width = dst.width() as usize;
    let dst_pixels = unsafe { as_pixels_mut(dst.data_mut()) };
    let w = w as usize;
    let h = h as usize;
    let sx = sx as usize;
    let sy = sy as usize;
    let dx = dx as usize;
    let dy = dy as usize;

    let a256 = color_a as u32 + 1;
    let pm_r = (color_r as u32 * a256) >> 8;
    let pm_g = (color_g as u32 * a256) >> 8;
    let pm_b = (color_b as u32 * a256) >> 8;
    let pm_a = color_a as u32;
    let pm_packed = pack_rgba(pm_r as u8, pm_g as u8, pm_b as u8, pm_a as u8);

    for row in 0..h {
        let mask_off = (sy + row) * mask_stride + sx;
        let dst_off = (dy + row) * dst_width + dx;
        unsafe {
            row_blend_general(
                dst_pixels.as_mut_ptr().add(dst_off),
                mask_data.as_ptr().add(mask_off),
                pm_packed,
                pm_a,
                w,
            );
        }
    }
}

#[inline(always)]
unsafe fn blend_pixel_opaque(dst: *mut u32, mask_val: u8, src: u32) {
    unsafe {
        if mask_val == 255 {
            *dst = src;
        } else {
            let scale = mask_val as u32 + 1;
            *dst = blend_packed(src, *dst, scale, 256 - scale);
        }
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn row_blend_opaque(mut dst: *mut u32, mut mask: *const u8, src: u32, mut width: usize) {
    unsafe {
        let src_vec = vdupq_n_u32(src);

        while width >= 16 {
            let m16 = vld1q_u8(mask);
            if vmaxvq_u8(m16) == 0 {
                dst = dst.add(16);
                mask = mask.add(16);
                width -= 16;
                continue;
            }
            if vminvq_u8(m16) == 255 {
                vst1q_u32(dst, src_vec);
                vst1q_u32(dst.add(4), src_vec);
                vst1q_u32(dst.add(8), src_vec);
                vst1q_u32(dst.add(12), src_vec);
                dst = dst.add(16);
                mask = mask.add(16);
                width -= 16;
                continue;
            }
            for i in 0..16 {
                let m = *mask.add(i);
                if m != 0 {
                    blend_pixel_opaque(dst.add(i), m, src);
                }
            }
            dst = dst.add(16);
            mask = mask.add(16);
            width -= 16;
        }

        row_blend_opaque_scalar(dst, mask, src, width);
    }
}

#[cfg(not(target_arch = "aarch64"))]
unsafe fn row_blend_opaque(dst: *mut u32, mask: *const u8, src: u32, width: usize) {
    unsafe { row_blend_opaque_scalar(dst, mask, src, width) }
}

#[inline(always)]
unsafe fn row_blend_opaque_scalar(dst: *mut u32, mask: *const u8, src: u32, width: usize) {
    unsafe {
        let mut col = 0;
        while col + 4 <= width {
            let m4 = (mask.add(col) as *const u32).read_unaligned();
            if m4 == 0 {
                col += 4;
                continue;
            }
            let bytes = m4.to_ne_bytes();
            for i in 0..4 {
                if bytes[i] != 0 {
                    blend_pixel_opaque(dst.add(col + i), bytes[i], src);
                }
            }
            col += 4;
        }
        while col < width {
            let m = *mask.add(col);
            if m != 0 {
                blend_pixel_opaque(dst.add(col), m, src);
            }
            col += 1;
        }
    }
}

#[cfg(target_arch = "aarch64")]
unsafe fn row_blend_general(
    mut dst: *mut u32,
    mut mask: *const u8,
    pm_packed: u32,
    pm_a: u32,
    mut width: usize,
) {
    unsafe {
        while width >= 16 {
            let m16 = vld1q_u8(mask);
            if vmaxvq_u8(m16) == 0 {
                dst = dst.add(16);
                mask = mask.add(16);
                width -= 16;
                continue;
            }
            for i in 0..16 {
                let m = *mask.add(i);
                if m != 0 {
                    blend_pixel_general(dst.add(i), m, pm_packed, pm_a);
                }
            }
            dst = dst.add(16);
            mask = mask.add(16);
            width -= 16;
        }

        row_blend_general_scalar(dst, mask, pm_packed, pm_a, width);
    }
}

#[cfg(not(target_arch = "aarch64"))]
unsafe fn row_blend_general(
    dst: *mut u32,
    mask: *const u8,
    pm_packed: u32,
    pm_a: u32,
    width: usize,
) {
    unsafe { row_blend_general_scalar(dst, mask, pm_packed, pm_a, width) }
}

#[inline(always)]
unsafe fn blend_pixel_general(dst: *mut u32, mask_val: u8, pm_packed: u32, pm_a: u32) {
    unsafe {
        let vmask256 = mask_val as u32 + 1;
        let src = scale_packed(pm_packed, vmask256);
        let eff_a = (pm_a * vmask256) >> 8;
        let inv_scale = 256 - eff_a;
        *dst = add_sat_u8x4(src, scale_packed(*dst, inv_scale));
    }
}

#[inline(always)]
unsafe fn row_blend_general_scalar(
    dst: *mut u32,
    mask: *const u8,
    pm_packed: u32,
    pm_a: u32,
    width: usize,
) {
    unsafe {
        let mut col = 0;
        while col + 4 <= width {
            let m4 = (mask.add(col) as *const u32).read_unaligned();
            if m4 == 0 {
                col += 4;
                continue;
            }
            let bytes = m4.to_ne_bytes();
            for i in 0..4 {
                if bytes[i] != 0 {
                    blend_pixel_general(dst.add(col + i), bytes[i], pm_packed, pm_a);
                }
            }
            col += 4;
        }
        while col < width {
            let m = *mask.add(col);
            if m != 0 {
                blend_pixel_general(dst.add(col), m, pm_packed, pm_a);
            }
            col += 1;
        }
    }
}

pub(crate) fn blit_color(
    dst: &mut PixmapMut,
    src_data: &[u8],
    src_width: u32,
    src_height: u32,
    dst_x: i32,
    dst_y: i32,
) {
    let dst_width = dst.width() as i32;
    let dst_height = dst.height() as i32;
    let src_w = src_width as i32;
    let src_h = src_height as i32;

    let src_x_start = if dst_x < 0 { -dst_x } else { 0 };
    let src_y_start = if dst_y < 0 { -dst_y } else { 0 };
    let dst_x_start = dst_x.max(0);
    let dst_y_start = dst_y.max(0);

    let copy_width = (src_w - src_x_start).min(dst_width - dst_x_start);
    let copy_height = (src_h - src_y_start).min(dst_height - dst_y_start);

    if copy_width <= 0 || copy_height <= 0 {
        return;
    }

    let src_pixels = unsafe { as_pixels(src_data) };
    let dst_pixels = unsafe { as_pixels_mut(dst.data_mut()) };
    let src_stride = src_width as usize;
    let dst_stride = dst_width as usize;
    let w = copy_width as usize;
    let h = copy_height as usize;
    let sxs = src_x_start as usize;
    let sys = src_y_start as usize;
    let dxs = dst_x_start as usize;
    let dys = dst_y_start as usize;

    let alpha_byte_idx = u32::from_ne_bytes([0, 0, 0, 255]).trailing_zeros() as usize / 8;
    let alpha_shift = alpha_byte_idx * 8;

    for row in 0..h {
        let src_row = (sys + row) * src_stride + sxs;
        let dst_row = (dys + row) * dst_stride + dxs;

        unsafe {
            let sp = src_pixels.as_ptr().add(src_row);
            let dp = dst_pixels.as_mut_ptr().add(dst_row);

            let mut col = 0;
            while col < w {
                let src_pixel = *sp.add(col);
                let src_a = (src_pixel >> alpha_shift) & 0xFF;

                if src_a == 0 {
                    col += 1;
                    continue;
                }

                if src_a == 255 {
                    *dp.add(col) = src_pixel;
                    col += 1;
                    continue;
                }

                let inv_scale = 256 - (src_a + 1);
                *dp.add(col) = add_sat_u8x4(src_pixel, scale_packed(*dp.add(col), inv_scale));
                col += 1;
            }
        }
    }
}

fn compute_clip(
    dst: &PixmapMut,
    mask_width: u32,
    mask_height: u32,
    dst_x: i32,
    dst_y: i32,
) -> (i32, i32, i32, i32, i32, i32) {
    let dst_width = dst.width() as i32;
    let dst_height = dst.height() as i32;
    let mask_width = mask_width as i32;
    let mask_height = mask_height as i32;

    let src_x_start = if dst_x < 0 { -dst_x } else { 0 };
    let src_y_start = if dst_y < 0 { -dst_y } else { 0 };
    let dst_x_start = dst_x.max(0);
    let dst_y_start = dst_y.max(0);

    let copy_width = (mask_width - src_x_start).min(dst_width - dst_x_start);
    let copy_height = (mask_height - src_y_start).min(dst_height - dst_y_start);

    (
        src_x_start,
        src_y_start,
        dst_x_start,
        dst_y_start,
        copy_width,
        copy_height,
    )
}
