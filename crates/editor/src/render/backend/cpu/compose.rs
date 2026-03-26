//! CPU pixel compositing, selection overlay, content layer compositing, blend utilities.

use crate::layout::Page;
use crate::model::{Doc, SelectionDecor};
use crate::render::backend::cpu::blend::{
    blend_row_const_src_over_lut, blend_row_const_src_over_opaque, blend_row_src_over,
    build_const_src_over_lut,
};
use crate::render::backend::cpu::pixel_buf::{PixelBuf, PixelBufMut};
use crate::render::geometry::{LayoutRect, collect_non_overlapping_pixel_rects};
use crate::render::renderer::{SelectionOverlayData, should_promote_full_repaint};
use crate::render::selection_overlay_color;
use crate::types::theme::Color;
use crate::types::{Point, Theme};

// ── Selection overlay & compositing ──────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub(in crate::render) fn render_selection_overlay(
    buf: &mut PixelBufMut,
    scratch_buf: &mut PixelBuf,
    scale_factor: f64,
    theme: &Theme,
    is_focused: bool,
    page: &Page,
    selections: &[SelectionDecor],
    doc: &Doc,
    selection_data: &SelectionOverlayData,
) {
    if selections.is_empty() || selection_data.clip_rects.is_empty() {
        return;
    }

    let scale = scale_factor as f32;
    if scale <= 0.0 {
        return;
    }

    let canvas_width = buf.width() as f32 / scale;
    let canvas_height = buf.height() as f32 / scale;
    if !selection_data.has_non_text_selection && !selection_data.text_paint_rects.is_empty() {
        let color = selection_overlay_color(theme, is_focused);
        fill_layout_rects_src_over(buf, &selection_data.text_paint_rects, scale_factor, color);
        return;
    }

    if should_promote_full_repaint(&selection_data.clip_rects, canvas_width, canvas_height) {
        super::layers::render_selection_phase(
            buf,
            scale_factor,
            theme,
            is_focused,
            page,
            selections,
            doc,
            None,
            Point::zero(),
        );
        return;
    }

    let clip_pixel_rects = collect_non_overlapping_pixel_rects(
        &selection_data.clip_rects,
        scale,
        buf.width(),
        buf.height(),
    );
    for pixel_rect in clip_pixel_rects {
        super::layers::render_selection_phase_clipped(
            buf,
            scratch_buf,
            scale_factor,
            theme,
            is_focused,
            page,
            selections,
            doc,
            pixel_rect.to_layout_rect(scale),
        );
    }
}

pub(in crate::render) fn composite_cached_content_layer_clipped(
    buf: &mut PixelBufMut,
    content_layer: &PixelBuf,
    clip_rects: &[LayoutRect],
    scale_factor: f64,
) {
    if clip_rects.is_empty() {
        return;
    }

    let scale = scale_factor as f32;
    if scale <= 0.0 {
        return;
    }

    let max_width = buf.width().min(content_layer.width());
    let max_height = buf.height().min(content_layer.height());
    let src_stride = content_layer.width() as usize * 4;
    let dst_stride = buf.width() as usize * 4;
    let src_data = content_layer.data();
    let dst_data = buf.data_mut();
    let pixel_rects = collect_non_overlapping_pixel_rects(clip_rects, scale, max_width, max_height);

    for pixel_rect in pixel_rects {
        let row_bytes = pixel_rect.width as usize * 4;
        let x_offset = pixel_rect.x as usize * 4;
        let y_start = pixel_rect.y as usize;
        for row in 0..pixel_rect.height as usize {
            let y = y_start + row;
            let src_offset = y * src_stride + x_offset;
            let dst_offset = y * dst_stride + x_offset;
            let src_slice = &src_data[src_offset..src_offset + row_bytes];
            let dst_slice = &mut dst_data[dst_offset..dst_offset + row_bytes];
            blend_row_src_over(src_slice, dst_slice);
        }
    }
}

pub(in crate::render) fn fill_layout_rects_src_over(
    buf: &mut PixelBufMut,
    rects: &[LayoutRect],
    scale_factor: f64,
    color: Color,
) {
    if rects.is_empty() {
        return;
    }

    let scale = scale_factor as f32;
    if scale <= 0.0 {
        return;
    }

    let premul = color.premultiply().to_rgba8();
    let src = [premul.r, premul.g, premul.b, premul.a];
    let src_alpha = src[3];
    if src_alpha == 0 {
        return;
    }
    let mut lut_r = [0u8; 256];
    let mut lut_g = [0u8; 256];
    let mut lut_b = [0u8; 256];
    let mut lut_a = [0u8; 256];
    if src_alpha != 255 {
        build_const_src_over_lut(src, &mut lut_r, &mut lut_g, &mut lut_b, &mut lut_a);
    }

    let max_width = buf.width();
    let max_height = buf.height();
    let stride = buf.width() as usize * 4;
    let data = buf.data_mut();
    let pixel_rects = collect_non_overlapping_pixel_rects(rects, scale, max_width, max_height);

    for pixel_rect in pixel_rects {
        let row_bytes = pixel_rect.width as usize * 4;
        let x_offset = pixel_rect.x as usize * 4;
        let y_start = pixel_rect.y as usize;
        for row in 0..pixel_rect.height as usize {
            let y = y_start + row;
            let row_offset = y * stride + x_offset;
            let row_slice = &mut data[row_offset..row_offset + row_bytes];
            if src_alpha == 255 {
                blend_row_const_src_over_opaque(row_slice, src);
            } else {
                blend_row_const_src_over_lut(row_slice, &lut_r, &lut_g, &lut_b, &lut_a);
            }
        }
    }
}

// ── Free functions ───────────────────────────────────────────────────

pub(in crate::render) fn composite_scratch_region_src_over(
    dst: &mut PixelBufMut,
    src: &PixelBuf,
    src_width: u32,
    src_height: u32,
    dst_x: u32,
    dst_y: u32,
) {
    if src_width == 0 || src_height == 0 {
        return;
    }

    let copy_width = src_width.min(dst.width().saturating_sub(dst_x));
    let copy_height = src_height.min(dst.height().saturating_sub(dst_y));
    if copy_width == 0 || copy_height == 0 {
        return;
    }

    let src_stride = src.width() as usize * 4;
    let dst_stride = dst.width() as usize * 4;
    let row_bytes = copy_width as usize * 4;
    let src_data = src.data();
    let dst_data = dst.data_mut();
    for row in 0..copy_height as usize {
        let src_offset = row * src_stride;
        let dst_offset = (dst_y as usize + row) * dst_stride + dst_x as usize * 4;
        let src_slice = &src_data[src_offset..src_offset + row_bytes];
        let dst_slice = &mut dst_data[dst_offset..dst_offset + row_bytes];
        blend_row_src_over(src_slice, dst_slice);
    }
}

/// Composite `src` onto `dst` at (dst_x, dst_y) using src-over blending.
pub(in crate::render) fn composite_src_over(
    dst: &mut PixelBufMut,
    src: &PixelBuf,
    dst_x: i32,
    dst_y: i32,
) {
    let src_w = src.width() as i32;
    let src_h = src.height() as i32;
    let dst_w = dst.width() as i32;
    let dst_h = dst.height() as i32;

    let sx_start = if dst_x < 0 { -dst_x } else { 0 };
    let sy_start = if dst_y < 0 { -dst_y } else { 0 };
    let dx_start = dst_x.max(0);
    let dy_start = dst_y.max(0);

    let copy_w = (src_w - sx_start).min(dst_w - dx_start);
    let copy_h = (src_h - sy_start).min(dst_h - dy_start);
    if copy_w <= 0 || copy_h <= 0 {
        return;
    }

    let src_stride = src.width() as usize * 4;
    let dst_stride = dst.width() as usize * 4;
    let row_bytes = copy_w as usize * 4;
    let src_data = src.data();
    let dst_data = dst.data_mut();

    for row in 0..copy_h as usize {
        let src_off = (sy_start as usize + row) * src_stride + sx_start as usize * 4;
        let dst_off = (dy_start as usize + row) * dst_stride + dx_start as usize * 4;
        let src_slice = &src_data[src_off..src_off + row_bytes];
        let dst_slice = &mut dst_data[dst_off..dst_off + row_bytes];
        blend_row_src_over(src_slice, dst_slice);
    }
}

/// Fill a pixel-aligned rectangle with a premultiplied RGBA color using src-over blending.
pub(in crate::render) fn fill_pixel_rect_src_over(
    buf: &mut PixelBufMut,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color_pm: [u8; 4],
) {
    if color_pm[3] == 0 {
        return;
    }

    let buf_w = buf.width() as i32;
    let buf_h = buf.height() as i32;

    let x0 = x.max(0) as usize;
    let y0 = y.max(0) as usize;
    let x1 = ((x + w as i32).min(buf_w)) as usize;
    let y1 = ((y + h as i32).min(buf_h)) as usize;
    if x0 >= x1 || y0 >= y1 {
        return;
    }

    let stride = buf.width() as usize * 4;
    let data = buf.data_mut();
    let inv_alpha = 255u16 - color_pm[3] as u16;

    for row in y0..y1 {
        let row_start = row * stride + x0 * 4;
        let row_end = row * stride + x1 * 4;
        let row_slice = &mut data[row_start..row_end];
        if color_pm[3] == 255 {
            let mut i = 0;
            while i < row_slice.len() {
                row_slice[i] = color_pm[0];
                row_slice[i + 1] = color_pm[1];
                row_slice[i + 2] = color_pm[2];
                row_slice[i + 3] = color_pm[3];
                i += 4;
            }
        } else {
            let mut i = 0;
            while i < row_slice.len() {
                let dr = row_slice[i] as u16;
                let dg = row_slice[i + 1] as u16;
                let db = row_slice[i + 2] as u16;
                let da = row_slice[i + 3] as u16;
                row_slice[i] = (color_pm[0] as u16 + ((dr * inv_alpha + 127) / 255)).min(255) as u8;
                row_slice[i + 1] =
                    (color_pm[1] as u16 + ((dg * inv_alpha + 127) / 255)).min(255) as u8;
                row_slice[i + 2] =
                    (color_pm[2] as u16 + ((db * inv_alpha + 127) / 255)).min(255) as u8;
                row_slice[i + 3] =
                    (color_pm[3] as u16 + ((da * inv_alpha + 127) / 255)).min(255) as u8;
                i += 4;
            }
        }
    }
}
