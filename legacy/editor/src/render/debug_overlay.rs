use super::backend::cpu::PixelBufMut;
use super::diagnostics::DebugFrame;
use super::geometry::LayoutRect;

const DEBUG_MARKER_SIZE_PX: f32 = 8.0;
const DEBUG_MARKER_MARGIN_PX: f32 = 2.0;

pub(super) fn render_debug_overlay(
    buf: &mut PixelBufMut,
    scale_factor: f64,
    frame: &DebugFrame,
    show_render: bool,
    show_layout: bool,
) {
    let scale = scale_factor as f32;

    if show_render && (!frame.render_rects.is_empty() || !frame.overflow_rects.is_empty()) {
        let (fill_color, stroke_color) = if frame.full_repaint {
            ([255, 77, 77, 36], [255, 77, 77, 220])
        } else {
            ([255, 179, 0, 28], [255, 179, 0, 220])
        };

        for rect in &frame.render_rects {
            fill_layout_rect(buf, *rect, fill_color, scale);
        }
        for rect in &frame.render_rects {
            draw_debug_rect_outline(buf, *rect, stroke_color, scale);
        }

        let overflow_fill = [236, 72, 153, 24];
        let overflow_stroke = [236, 72, 153, 220];
        for rect in &frame.overflow_rects {
            fill_layout_rect(buf, *rect, overflow_fill, scale);
        }
        for rect in &frame.overflow_rects {
            draw_debug_rect_outline(buf, *rect, overflow_stroke, scale);
        }
    }

    if show_layout && !frame.layout_rects.is_empty() {
        let (fill_color, stroke_color) = if frame.full_relayout {
            ([59, 130, 246, 24], [59, 130, 246, 220])
        } else {
            ([14, 165, 233, 20], [14, 165, 233, 220])
        };

        for rect in &frame.layout_rects {
            fill_layout_rect(buf, *rect, fill_color, scale);
        }
        for rect in &frame.layout_rects {
            draw_debug_rect_outline(buf, *rect, stroke_color, scale);
        }
    }

    let render_marker_color = if !show_render {
        None
    } else if frame.cache_reused && frame.overflow_rects.is_empty() {
        Some([16, 185, 129, 255])
    } else if frame.full_repaint {
        Some([255, 77, 77, 255])
    } else if !frame.render_rects.is_empty() || !frame.overflow_rects.is_empty() {
        Some([255, 179, 0, 255])
    } else {
        None
    };

    if let Some(color) = render_marker_color {
        draw_debug_marker(buf, color, scale, 0);
    }

    let layout_marker_color = if !show_layout {
        None
    } else if frame.layout_reused {
        Some([16, 185, 129, 255])
    } else if frame.full_relayout {
        Some([59, 130, 246, 255])
    } else if !frame.layout_rects.is_empty() {
        Some([14, 165, 233, 255])
    } else {
        None
    };

    if let Some(color) = layout_marker_color {
        draw_debug_marker(buf, color, scale, 1);
    }
}

/// Premultiply an RGBA8 color.
fn premul(color: [u8; 4]) -> [u8; 4] {
    let a = color[3] as u16;
    if a == 255 {
        return color;
    }
    if a == 0 {
        return [0, 0, 0, 0];
    }
    [
        ((color[0] as u16 * a + 127) / 255) as u8,
        ((color[1] as u16 * a + 127) / 255) as u8,
        ((color[2] as u16 * a + 127) / 255) as u8,
        color[3],
    ]
}

/// Fill a layout-coordinate rect onto the pixel buffer with src-over blending.
fn fill_layout_rect(buf: &mut PixelBufMut, rect: LayoutRect, color: [u8; 4], scale: f32) {
    let pm = premul(color);
    if pm[3] == 0 {
        return;
    }

    let x0 = (rect.x * scale).floor().max(0.0) as usize;
    let y0 = (rect.y * scale).floor().max(0.0) as usize;
    let x1 = (rect.right() * scale).ceil().min(buf.width() as f32) as usize;
    let y1 = (rect.bottom() * scale).ceil().min(buf.height() as f32) as usize;
    if x0 >= x1 || y0 >= y1 {
        return;
    }

    let stride = buf.width() as usize * 4;
    let data = buf.data_mut();
    let inv_alpha = 255u16 - pm[3] as u16;

    for row in y0..y1 {
        let row_start = row * stride + x0 * 4;
        let row_end = row * stride + x1 * 4;
        let row_slice = &mut data[row_start..row_end];
        if pm[3] == 255 {
            let mut i = 0;
            while i < row_slice.len() {
                row_slice[i] = pm[0];
                row_slice[i + 1] = pm[1];
                row_slice[i + 2] = pm[2];
                row_slice[i + 3] = pm[3];
                i += 4;
            }
        } else {
            let mut i = 0;
            while i < row_slice.len() {
                let dr = row_slice[i] as u16;
                let dg = row_slice[i + 1] as u16;
                let db = row_slice[i + 2] as u16;
                let da = row_slice[i + 3] as u16;
                row_slice[i] = (pm[0] as u16 + ((dr * inv_alpha + 127) / 255)).min(255) as u8;
                row_slice[i + 1] = (pm[1] as u16 + ((dg * inv_alpha + 127) / 255)).min(255) as u8;
                row_slice[i + 2] = (pm[2] as u16 + ((db * inv_alpha + 127) / 255)).min(255) as u8;
                row_slice[i + 3] = (pm[3] as u16 + ((da * inv_alpha + 127) / 255)).min(255) as u8;
                i += 4;
            }
        }
    }
}

fn draw_debug_rect_outline(buf: &mut PixelBufMut, rect: LayoutRect, color: [u8; 4], scale: f32) {
    let mut thickness = (1.0 / scale).max(0.25);
    thickness = thickness.min(rect.width * 0.5).min(rect.height * 0.5);
    if thickness <= 0.0 {
        return;
    }

    if let Some(top) = LayoutRect::from_xywh(rect.x, rect.y, rect.width, thickness) {
        fill_layout_rect(buf, top, color, scale);
    }
    if let Some(bottom) = LayoutRect::from_xywh(
        rect.x,
        rect.y + rect.height - thickness,
        rect.width,
        thickness,
    ) {
        fill_layout_rect(buf, bottom, color, scale);
    }
    if let Some(left) = LayoutRect::from_xywh(rect.x, rect.y, thickness, rect.height) {
        fill_layout_rect(buf, left, color, scale);
    }
    if let Some(right) = LayoutRect::from_xywh(
        rect.x + rect.width - thickness,
        rect.y,
        thickness,
        rect.height,
    ) {
        fill_layout_rect(buf, right, color, scale);
    }
}

fn draw_debug_marker(buf: &mut PixelBufMut, color: [u8; 4], scale: f32, slot: usize) {
    let size = (DEBUG_MARKER_SIZE_PX / scale).max(0.5 / scale);
    let margin = DEBUG_MARKER_MARGIN_PX / scale;
    let x = margin + slot as f32 * (size + margin);
    let y = margin;

    if let Some(marker_rect) = LayoutRect::from_xywh(x, y, size, size) {
        fill_layout_rect(buf, marker_rect, color, scale);
        draw_debug_rect_outline(buf, marker_rect, [0, 0, 0, 220], scale);
    }
}
