use super::geometry::CacheRect;
use super::paint_diagnostics::PaintDebugFrame;
use tiny_skia::{Color, PixmapMut, Rect, Transform};

const DEBUG_MARKER_SIZE_PX: f32 = 8.0;
const DEBUG_MARKER_MARGIN_PX: f32 = 2.0;

pub(super) fn render_debug_overlay(
    pixmap: &mut PixmapMut,
    scale_factor: f64,
    frame: &PaintDebugFrame,
    show_render: bool,
    show_layout: bool,
) {
    let scale = scale_factor as f32;
    let transform = Transform::from_scale(scale, scale);

    if show_render && !frame.render_rects.is_empty() {
        let (fill_color, stroke_color) = if frame.full_repaint {
            (
                Color::from_rgba8(255, 77, 77, 36),
                Color::from_rgba8(255, 77, 77, 220),
            )
        } else {
            (
                Color::from_rgba8(255, 179, 0, 28),
                Color::from_rgba8(255, 179, 0, 220),
            )
        };

        let mut fill_paint = tiny_skia::Paint::default();
        fill_paint.set_color(fill_color);
        for rect in &frame.render_rects {
            if let Some(layout_rect) = Rect::from_xywh(rect.x, rect.y, rect.width, rect.height) {
                pixmap.fill_rect(layout_rect, &fill_paint, transform, None);
            }
        }

        for rect in &frame.render_rects {
            draw_debug_rect_outline(pixmap, *rect, stroke_color, transform, scale);
        }
    }

    if show_layout && !frame.layout_rects.is_empty() {
        let (fill_color, stroke_color) = if frame.full_relayout {
            (
                Color::from_rgba8(59, 130, 246, 24),
                Color::from_rgba8(59, 130, 246, 220),
            )
        } else {
            (
                Color::from_rgba8(14, 165, 233, 20),
                Color::from_rgba8(14, 165, 233, 220),
            )
        };

        let mut fill_paint = tiny_skia::Paint::default();
        fill_paint.set_color(fill_color);
        for rect in &frame.layout_rects {
            if let Some(layout_rect) = Rect::from_xywh(rect.x, rect.y, rect.width, rect.height) {
                pixmap.fill_rect(layout_rect, &fill_paint, transform, None);
            }
        }

        for rect in &frame.layout_rects {
            draw_debug_rect_outline(pixmap, *rect, stroke_color, transform, scale);
        }
    }

    let render_marker_color = if !show_render {
        None
    } else if frame.cache_reused {
        Some(Color::from_rgba8(16, 185, 129, 255))
    } else if frame.full_repaint {
        Some(Color::from_rgba8(255, 77, 77, 255))
    } else if !frame.render_rects.is_empty() {
        Some(Color::from_rgba8(255, 179, 0, 255))
    } else {
        None
    };

    if let Some(color) = render_marker_color {
        draw_debug_marker(pixmap, color, transform, scale, 0);
    }

    let layout_marker_color = if !show_layout {
        None
    } else if frame.layout_reused {
        Some(Color::from_rgba8(16, 185, 129, 255))
    } else if frame.full_relayout {
        Some(Color::from_rgba8(59, 130, 246, 255))
    } else if !frame.layout_rects.is_empty() {
        Some(Color::from_rgba8(14, 165, 233, 255))
    } else {
        None
    };

    if let Some(color) = layout_marker_color {
        draw_debug_marker(pixmap, color, transform, scale, 1);
    }
}

fn draw_debug_rect_outline(
    pixmap: &mut PixmapMut,
    rect: CacheRect,
    color: Color,
    transform: Transform,
    scale: f32,
) {
    let mut paint = tiny_skia::Paint::default();
    paint.set_color(color);

    let mut thickness = (1.0 / scale).max(0.25);
    thickness = thickness.min(rect.width * 0.5).min(rect.height * 0.5);
    if thickness <= 0.0 {
        return;
    }

    let top = Rect::from_xywh(rect.x, rect.y, rect.width, thickness);
    let bottom = Rect::from_xywh(
        rect.x,
        rect.y + rect.height - thickness,
        rect.width,
        thickness,
    );
    let left = Rect::from_xywh(rect.x, rect.y, thickness, rect.height);
    let right = Rect::from_xywh(
        rect.x + rect.width - thickness,
        rect.y,
        thickness,
        rect.height,
    );

    for segment in [top, bottom, left, right].into_iter().flatten() {
        pixmap.fill_rect(segment, &paint, transform, None);
    }
}

fn draw_debug_marker(
    pixmap: &mut PixmapMut,
    color: Color,
    transform: Transform,
    scale: f32,
    slot: usize,
) {
    let size = (DEBUG_MARKER_SIZE_PX / scale).max(0.5 / scale);
    let margin = DEBUG_MARKER_MARGIN_PX / scale;
    let x = margin + slot as f32 * (size + margin);
    let y = margin;

    if let Some(marker_rect) = Rect::from_xywh(x, y, size, size) {
        let mut fill = tiny_skia::Paint::default();
        fill.set_color(color);
        pixmap.fill_rect(marker_rect, &fill, transform, None);

        draw_debug_rect_outline(
            pixmap,
            CacheRect {
                x,
                y,
                width: size,
                height: size,
            },
            Color::from_rgba8(0, 0, 0, 220),
            transform,
            scale,
        );
    }
}
