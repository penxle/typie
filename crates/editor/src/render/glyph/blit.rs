use tiny_skia::PixmapMut;

use super::scaler::{ColorImage, MaskImage};

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

    if color_r == 0 && color_g == 0 && color_b == 0 && color_a == 255 {
        blit_mask_black(dst, mask, dst_x, dst_y);
        return;
    }

    if color_a == 255 {
        blit_mask_opaque(dst, mask, dst_x, dst_y, color_r, color_g, color_b);
    } else {
        blit_mask_general(dst, mask, dst_x, dst_y, color_r, color_g, color_b, color_a);
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

    let sr = color_r as u32;
    let sg = color_g as u32;
    let sb = color_b as u32;

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
            let inv_scale = 256 - scale;

            dst_data[idx] = ((sr * scale + dst_data[idx] as u32 * inv_scale) >> 8) as u8;
            dst_data[idx + 1] = ((sg * scale + dst_data[idx + 1] as u32 * inv_scale) >> 8) as u8;
            dst_data[idx + 2] = ((sb * scale + dst_data[idx + 2] as u32 * inv_scale) >> 8) as u8;
            dst_data[idx + 3] =
                ((255u32 * scale + dst_data[idx + 3] as u32 * inv_scale) >> 8) as u8;
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

    let a256 = color_a as u32 + 1;
    let pm_r = alpha_mul(color_r as u32, a256);
    let pm_g = alpha_mul(color_g as u32, a256);
    let pm_b = alpha_mul(color_b as u32, a256);
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
            let eff_a = alpha_mul(pm_a, vmask256);
            let inv_scale = 256 - eff_a;

            let src_r = alpha_mul(pm_r, vmask256);
            let src_g = alpha_mul(pm_g, vmask256);
            let src_b = alpha_mul(pm_b, vmask256);

            dst_data[idx] = (src_r + alpha_mul(dst_data[idx] as u32, inv_scale)).min(255) as u8;
            dst_data[idx + 1] =
                (src_g + alpha_mul(dst_data[idx + 1] as u32, inv_scale)).min(255) as u8;
            dst_data[idx + 2] =
                (src_b + alpha_mul(dst_data[idx + 2] as u32, inv_scale)).min(255) as u8;
            dst_data[idx + 3] =
                (eff_a + alpha_mul(dst_data[idx + 3] as u32, inv_scale)).min(255) as u8;
        }
    }
}

fn blit_mask_black(dst: &mut PixmapMut, mask: &MaskImage, dst_x: i32, dst_y: i32) {
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

            let inv_scale = 256 - (vmask + 1);

            dst_data[idx] = alpha_mul(dst_data[idx] as u32, inv_scale) as u8;
            dst_data[idx + 1] = alpha_mul(dst_data[idx + 1] as u32, inv_scale) as u8;
            dst_data[idx + 2] = alpha_mul(dst_data[idx + 2] as u32, inv_scale) as u8;
            dst_data[idx + 3] =
                ((255u32 * (vmask + 1) + dst_data[idx + 3] as u32 * inv_scale) >> 8) as u8;
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
