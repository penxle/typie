use super::blend::{
    blend_row_const_src_over_lut, blend_row_const_src_over_opaque, build_const_src_over_lut,
};
use crate::types::Theme;
use tiny_skia::{Color, Paint, PixmapMut, Rect, Transform};

pub(crate) fn selection_overlay_color(theme: &Theme, is_focused: bool) -> Color {
    if is_focused {
        theme.color_with_alpha("selection", 77)
    } else {
        theme.color_with_alpha("selection", 48)
    }
}

pub(crate) fn selection_overlay_paint(theme: &Theme, is_focused: bool) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(selection_overlay_color(theme, is_focused));
    paint.anti_alias = true;
    paint
}

pub(crate) fn fill_rect_src_over_fast(
    pixmap: &mut PixmapMut,
    rect: Rect,
    transform: Transform,
    color: Color,
) -> bool {
    if transform.kx.abs() > f32::EPSILON
        || transform.ky.abs() > f32::EPSILON
        || transform.sx <= 0.0
        || transform.sy <= 0.0
    {
        return false;
    }

    let left = rect.left() * transform.sx + transform.tx;
    let top = rect.top() * transform.sy + transform.ty;
    let right = rect.right() * transform.sx + transform.tx;
    let bottom = rect.bottom() * transform.sy + transform.ty;

    let x0 = left.floor().max(0.0).min(pixmap.width() as f32) as u32;
    let y0 = top.floor().max(0.0).min(pixmap.height() as f32) as u32;
    let x1 = right.ceil().max(0.0).min(pixmap.width() as f32) as u32;
    let y1 = bottom.ceil().max(0.0).min(pixmap.height() as f32) as u32;

    if x1 <= x0 || y1 <= y0 {
        return true;
    }

    let premul = color.premultiply().to_color_u8();
    let src = [premul.red(), premul.green(), premul.blue(), premul.alpha()];
    let src_alpha = src[3];
    if src_alpha == 0 {
        return true;
    }

    let stride = pixmap.width() as usize * 4;
    let row_bytes = (x1 - x0) as usize * 4;
    let x_offset = x0 as usize * 4;
    let y_start = y0 as usize;
    let y_end = y1 as usize;
    let data = pixmap.data_mut();

    if src_alpha == 255 {
        for row in y_start..y_end {
            let row_offset = row * stride + x_offset;
            blend_row_const_src_over_opaque(&mut data[row_offset..row_offset + row_bytes], src);
        }
        return true;
    }

    let mut lut_r = [0u8; 256];
    let mut lut_g = [0u8; 256];
    let mut lut_b = [0u8; 256];
    let mut lut_a = [0u8; 256];
    build_const_src_over_lut(src, &mut lut_r, &mut lut_g, &mut lut_b, &mut lut_a);

    for row in y_start..y_end {
        let row_offset = row * stride + x_offset;
        blend_row_const_src_over_lut(
            &mut data[row_offset..row_offset + row_bytes],
            &lut_r,
            &lut_g,
            &lut_b,
            &lut_a,
        );
    }

    true
}
