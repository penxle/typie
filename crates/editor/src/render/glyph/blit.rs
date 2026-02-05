use std::sync::OnceLock;

use tiny_skia::PixmapMut;

use super::scaler::{ColorImage, MaskImage};

const LIN_BITS: u32 = 13;
const LIN_SCALE: u32 = 1 << LIN_BITS;
const LIN_TABLE_SIZE: usize = LIN_SCALE as usize + 1;

struct LinearTables {
    s2l: [u16; 256],
    l2s: [u8; LIN_TABLE_SIZE],
}

fn linear_tables() -> &'static LinearTables {
    static TABLES: OnceLock<LinearTables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut s2l = [0u16; 256];
        for i in 0..256 {
            let s = i as f64 / 255.0;
            let lin = if s <= 0.04045 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            };
            s2l[i] = (lin * LIN_SCALE as f64 + 0.5) as u16;
        }

        let mut l2s = [0u8; LIN_TABLE_SIZE];
        for i in 0..LIN_TABLE_SIZE {
            let lin = i as f64 / LIN_SCALE as f64;
            let s = if lin <= 0.0031308 {
                lin * 12.92
            } else {
                1.055 * lin.powf(1.0 / 2.4) - 0.055
            };
            l2s[i] = (s * 255.0 + 0.5).min(255.0) as u8;
        }

        LinearTables { s2l, l2s }
    })
}

#[inline(always)]
fn alpha_mul(value: u32, scale256: u32) -> u32 {
    (value * scale256) >> 8
}

pub(crate) fn blit_mask_d32_a8(
    dst: &mut PixmapMut,
    mask: &MaskImage,
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

    let tables = linear_tables();

    if color_r == 0 && color_g == 0 && color_b == 0 && color_a == 255 {
        blit_mask_black(dst, mask, dst_x, dst_y, tables);
        return;
    }

    if color_a == 255 {
        blit_mask_opaque(dst, mask, dst_x, dst_y, color_r, color_g, color_b, tables);
    } else {
        blit_mask_general(
            dst, mask, dst_x, dst_y, color_r, color_g, color_b, color_a, tables,
        );
    }
}

fn blit_mask_opaque(
    dst: &mut PixmapMut,
    mask: &MaskImage,
    dst_x: i32,
    dst_y: i32,
    color_r: u8,
    color_g: u8,
    color_b: u8,
    tables: &LinearTables,
) {
    let (src_x_start, src_y_start, dst_x_start, dst_y_start, copy_width, copy_height) =
        compute_clip(dst, mask, dst_x, dst_y);
    if copy_width <= 0 || copy_height <= 0 {
        return;
    }

    let mask_data = &mask.data;
    let mask_stride = mask.width as usize;
    let dst_stride = (dst.width() as usize) * 4;
    let dst_data = dst.data_mut();

    let src_lin_r = tables.s2l[color_r as usize] as u32;
    let src_lin_g = tables.s2l[color_g as usize] as u32;
    let src_lin_b = tables.s2l[color_b as usize] as u32;

    for row in 0..copy_height as usize {
        let mask_row_start = (src_y_start as usize + row) * mask_stride + src_x_start as usize;
        let dst_row_start = (dst_y_start as usize + row) * dst_stride + dst_x_start as usize * 4;

        for col in 0..copy_width as usize {
            let vmask = mask_data[mask_row_start + col] as u32;
            if vmask == 0 {
                continue;
            }

            let idx = dst_row_start + col * 4;

            if vmask == 255 {
                dst_data[idx] = color_r;
                dst_data[idx + 1] = color_g;
                dst_data[idx + 2] = color_b;
                dst_data[idx + 3] = 255;
                continue;
            }

            let scale = vmask + 1;
            let inv_scale = 256 - vmask;

            let dst_lin_r = tables.s2l[dst_data[idx] as usize] as u32;
            let dst_lin_g = tables.s2l[dst_data[idx + 1] as usize] as u32;
            let dst_lin_b = tables.s2l[dst_data[idx + 2] as usize] as u32;

            let r = ((src_lin_r * scale + dst_lin_r * inv_scale) >> 8).min(LIN_SCALE);
            let g = ((src_lin_g * scale + dst_lin_g * inv_scale) >> 8).min(LIN_SCALE);
            let b = ((src_lin_b * scale + dst_lin_b * inv_scale) >> 8).min(LIN_SCALE);

            dst_data[idx] = tables.l2s[r as usize];
            dst_data[idx + 1] = tables.l2s[g as usize];
            dst_data[idx + 2] = tables.l2s[b as usize];
            dst_data[idx + 3] =
                (alpha_mul(255, scale) + alpha_mul(dst_data[idx + 3] as u32, inv_scale)) as u8;
        }
    }
}

fn blit_mask_general(
    dst: &mut PixmapMut,
    mask: &MaskImage,
    dst_x: i32,
    dst_y: i32,
    color_r: u8,
    color_g: u8,
    color_b: u8,
    color_a: u8,
    tables: &LinearTables,
) {
    let (src_x_start, src_y_start, dst_x_start, dst_y_start, copy_width, copy_height) =
        compute_clip(dst, mask, dst_x, dst_y);
    if copy_width <= 0 || copy_height <= 0 {
        return;
    }

    let mask_data = &mask.data;
    let mask_stride = mask.width as usize;
    let dst_stride = (dst.width() as usize) * 4;
    let dst_data = dst.data_mut();

    let src_lin_r = tables.s2l[color_r as usize] as u32;
    let src_lin_g = tables.s2l[color_g as usize] as u32;
    let src_lin_b = tables.s2l[color_b as usize] as u32;
    let a256 = color_a as u32 + 1;
    let pm_lin_r = (src_lin_r * a256) >> 8;
    let pm_lin_g = (src_lin_g * a256) >> 8;
    let pm_lin_b = (src_lin_b * a256) >> 8;
    let pm_a = color_a as u32;

    for row in 0..copy_height as usize {
        let mask_row_start = (src_y_start as usize + row) * mask_stride + src_x_start as usize;
        let dst_row_start = (dst_y_start as usize + row) * dst_stride + dst_x_start as usize * 4;

        for col in 0..copy_width as usize {
            let vmask = mask_data[mask_row_start + col] as u32;
            if vmask == 0 {
                continue;
            }

            let idx = dst_row_start + col * 4;
            let vmask256 = vmask + 1;
            let inv_scale = 256 - alpha_mul(pm_a, vmask256);

            let dst_lin_r = tables.s2l[dst_data[idx] as usize] as u32;
            let dst_lin_g = tables.s2l[dst_data[idx + 1] as usize] as u32;
            let dst_lin_b = tables.s2l[dst_data[idx + 2] as usize] as u32;

            let r = ((pm_lin_r * vmask256 + dst_lin_r * inv_scale) >> 8).min(LIN_SCALE);
            let g = ((pm_lin_g * vmask256 + dst_lin_g * inv_scale) >> 8).min(LIN_SCALE);
            let b = ((pm_lin_b * vmask256 + dst_lin_b * inv_scale) >> 8).min(LIN_SCALE);

            dst_data[idx] = tables.l2s[r as usize];
            dst_data[idx + 1] = tables.l2s[g as usize];
            dst_data[idx + 2] = tables.l2s[b as usize];
            dst_data[idx + 3] =
                (alpha_mul(pm_a, vmask256) + alpha_mul(dst_data[idx + 3] as u32, inv_scale)) as u8;
        }
    }
}

fn blit_mask_black(
    dst: &mut PixmapMut,
    mask: &MaskImage,
    dst_x: i32,
    dst_y: i32,
    tables: &LinearTables,
) {
    let (src_x_start, src_y_start, dst_x_start, dst_y_start, copy_width, copy_height) =
        compute_clip(dst, mask, dst_x, dst_y);
    if copy_width <= 0 || copy_height <= 0 {
        return;
    }

    let mask_data = &mask.data;
    let mask_stride = mask.width as usize;
    let dst_stride = (dst.width() as usize) * 4;
    let dst_data = dst.data_mut();

    for row in 0..copy_height as usize {
        let mask_row_start = (src_y_start as usize + row) * mask_stride + src_x_start as usize;
        let dst_row_start = (dst_y_start as usize + row) * dst_stride + dst_x_start as usize * 4;

        for col in 0..copy_width as usize {
            let vmask = mask_data[mask_row_start + col] as u32;
            if vmask == 0 {
                continue;
            }

            let idx = dst_row_start + col * 4;

            if vmask == 255 {
                dst_data[idx] = 0;
                dst_data[idx + 1] = 0;
                dst_data[idx + 2] = 0;
                dst_data[idx + 3] = 255;
                continue;
            }

            let inv_scale = 256 - vmask;

            let dst_lin_r = tables.s2l[dst_data[idx] as usize] as u32;
            let dst_lin_g = tables.s2l[dst_data[idx + 1] as usize] as u32;
            let dst_lin_b = tables.s2l[dst_data[idx + 2] as usize] as u32;

            dst_data[idx] = tables.l2s[((dst_lin_r * inv_scale) >> 8) as usize];
            dst_data[idx + 1] = tables.l2s[((dst_lin_g * inv_scale) >> 8) as usize];
            dst_data[idx + 2] = tables.l2s[((dst_lin_b * inv_scale) >> 8) as usize];
            dst_data[idx + 3] = (vmask + alpha_mul(dst_data[idx + 3] as u32, inv_scale)) as u8;
        }
    }
}

pub(crate) fn blit_color(dst: &mut PixmapMut, color: &ColorImage, dst_x: i32, dst_y: i32) {
    let dst_width = dst.width() as i32;
    let dst_height = dst.height() as i32;
    let src_width = color.pixmap.width() as i32;
    let src_height = color.pixmap.height() as i32;

    let src_x_start = if dst_x < 0 { -dst_x } else { 0 };
    let src_y_start = if dst_y < 0 { -dst_y } else { 0 };
    let dst_x_start = dst_x.max(0);
    let dst_y_start = dst_y.max(0);

    let copy_width = (src_width - src_x_start).min(dst_width - dst_x_start);
    let copy_height = (src_height - src_y_start).min(dst_height - dst_y_start);

    if copy_width <= 0 || copy_height <= 0 {
        return;
    }

    let src_data = color.pixmap.data();
    let dst_data = dst.data_mut();
    let src_stride = (src_width * 4) as usize;
    let dst_stride = (dst_width * 4) as usize;

    for row in 0..copy_height as usize {
        let src_row_offset = (src_y_start as usize + row) * src_stride + (src_x_start as usize) * 4;
        let dst_row_offset = (dst_y_start as usize + row) * dst_stride + (dst_x_start as usize) * 4;

        for col in 0..copy_width as usize {
            let si = src_row_offset + col * 4;
            let di = dst_row_offset + col * 4;

            let src_a = src_data[si + 3] as u32;
            if src_a == 0 {
                continue;
            }

            if src_a == 255 {
                dst_data[di] = src_data[si];
                dst_data[di + 1] = src_data[si + 1];
                dst_data[di + 2] = src_data[si + 2];
                dst_data[di + 3] = 255;
                continue;
            }

            let inv_scale = 256 - (src_a + 1);

            dst_data[di] = (src_data[si] as u32 + alpha_mul(dst_data[di] as u32, inv_scale)) as u8;
            dst_data[di + 1] =
                (src_data[si + 1] as u32 + alpha_mul(dst_data[di + 1] as u32, inv_scale)) as u8;
            dst_data[di + 2] =
                (src_data[si + 2] as u32 + alpha_mul(dst_data[di + 2] as u32, inv_scale)) as u8;
            dst_data[di + 3] =
                (src_a + alpha_mul(dst_data[di + 3] as u32, inv_scale)).min(255) as u8;
        }
    }
}

fn compute_clip(
    dst: &PixmapMut,
    mask: &MaskImage,
    dst_x: i32,
    dst_y: i32,
) -> (i32, i32, i32, i32, i32, i32) {
    let dst_width = dst.width() as i32;
    let dst_height = dst.height() as i32;
    let mask_width = mask.width as i32;
    let mask_height = mask.height as i32;

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
