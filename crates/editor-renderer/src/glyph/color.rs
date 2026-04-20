use skrifa::color::ColorPalettes;
use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::DrawSettings;
use skrifa::raw::TableProvider;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use zeno::{Placement, Point};

use super::mask::rasterize_outline_to_mask;
use super::outline_pen::OutlineWriter;
use super::scaler::ScaleContext;
use super::{Content, RasterizedGlyph};

const FOREGROUND_FALLBACK: [u8; 4] = [128, 128, 128, 255];

pub fn rasterize_color_outline(
    ctx: &mut ScaleContext,
    font_data: &[u8],
    glyph_id: u32,
    font_size: f32,
    subpixel_offset_x: f32,
) -> Option<RasterizedGlyph> {
    let font = FontRef::from_index(font_data, 0).ok()?;
    let layers = read_colr_v0_layers(&font, glyph_id)?;
    if layers.is_empty() {
        return None;
    }

    let palette = read_palette(&font);
    let size = Size::new(font_size);
    let coords: &[NormalizedCoord] = &[];

    let (base_x, base_y, width, height) =
        compute_union_bounds(ctx, &font, size, coords, &layers, subpixel_offset_x)?;

    let mut canvas = vec![0u8; (width * height * 4) as usize];

    let outlines = font.outline_glyphs();
    for (layer_gid, color_index) in &layers {
        let og = outlines.get(GlyphId::new(u32::from(*layer_gid)))?;
        ctx.outline.clear();
        og.draw(
            DrawSettings::unhinted(size, LocationRef::new(coords)),
            &mut OutlineWriter(&mut ctx.outline),
        )
        .ok()?;
        if ctx.outline.is_empty() {
            continue;
        }

        let placement = rasterize_outline_to_mask(
            &ctx.outline,
            &mut ctx.scratch.zeno,
            subpixel_offset_x,
            None,
            &mut ctx.scratch.bitmap_0,
        );
        if placement.width == 0 || placement.height == 0 {
            continue;
        }

        let color = resolve_color(*color_index, palette.as_deref());
        blit_mask_onto_canvas(
            &ctx.scratch.bitmap_0,
            placement,
            base_x,
            base_y,
            width,
            height,
            color,
            &mut canvas,
        );
    }

    Some(RasterizedGlyph {
        data: canvas,
        width,
        height,
        placement_left: base_x,
        placement_top: base_y + height as i32,
        content: Content::Color,
    })
}

fn read_colr_v0_layers(font: &FontRef<'_>, glyph_id: u32) -> Option<Vec<(u16, u16)>> {
    let colr = font.colr().ok()?;
    let range = colr.v0_base_glyph(GlyphId::new(glyph_id)).ok()??;
    let mut out = Vec::with_capacity(range.len());
    for i in range {
        let (layer_gid, palette_index) = colr.v0_layer(i).ok()?;
        out.push((layer_gid.to_u16(), palette_index));
    }
    Some(out)
}

fn read_palette(font: &FontRef<'_>) -> Option<Vec<[u8; 4]>> {
    let palettes = ColorPalettes::new(font);
    let palette = palettes.get(0)?;
    Some(
        palette
            .colors()
            .iter()
            .map(|c| [c.red(), c.green(), c.blue(), c.alpha()])
            .collect(),
    )
}

fn resolve_color(palette_index: u16, palette: Option<&[[u8; 4]]>) -> [u8; 4] {
    if palette_index == 0xFFFF {
        return FOREGROUND_FALLBACK;
    }
    palette
        .and_then(|p| p.get(palette_index as usize).copied())
        .unwrap_or(FOREGROUND_FALLBACK)
}

fn compute_union_bounds(
    ctx: &mut ScaleContext,
    font: &FontRef<'_>,
    size: Size,
    coords: &[NormalizedCoord],
    layers: &[(u16, u16)],
    subpixel_offset_x: f32,
) -> Option<(i32, i32, u32, u32)> {
    let outlines = font.outline_glyphs();
    let mut union: Option<(f32, f32, f32, f32)> = None;

    for (layer_gid, _) in layers {
        let og = outlines.get(GlyphId::new(u32::from(*layer_gid)))?;
        ctx.outline.clear();
        og.draw(
            DrawSettings::unhinted(size, LocationRef::new(coords)),
            &mut OutlineWriter(&mut ctx.outline),
        )
        .ok()?;
        if ctx.outline.is_empty() {
            continue;
        }

        let (min, max) = outline_point_bounds(ctx.outline.points())?;
        union = Some(match union {
            None => (min.x, min.y, max.x, max.y),
            Some((lx, ly, hx, hy)) => (lx.min(min.x), ly.min(min.y), hx.max(max.x), hy.max(max.y)),
        });
    }

    let (min_x, min_y, max_x, max_y) = union?;
    // 서브픽셀 오프셋은 x축에만 적용 (legacy 와 동일). y 는 floor 대신 ceil 로
    // 비대칭 처리 — 폰트 좌표계의 y-up 과 픽셀 그리드 정렬을 위한 기존 규약.
    let base_x = (min_x + subpixel_offset_x).floor() as i32;
    let base_y = min_y.ceil() as i32;
    let width = (max_x - min_x).ceil() as u32;
    let height = (max_y - min_y).ceil() as u32;
    if width == 0 || height == 0 {
        return None;
    }
    Some((base_x, base_y, width, height))
}

fn outline_point_bounds(points: &[Point]) -> Option<(Point, Point)> {
    if points.is_empty() {
        return None;
    }
    let mut min = points[0];
    let mut max = points[0];
    for p in &points[1..] {
        if p.x < min.x {
            min.x = p.x;
        }
        if p.y < min.y {
            min.y = p.y;
        }
        if p.x > max.x {
            max.x = p.x;
        }
        if p.y > max.y {
            max.y = p.y;
        }
    }
    Some((min, max))
}

fn blit_mask_onto_canvas(
    mask: &[u8],
    placement: Placement,
    base_x: i32,
    base_y: i32,
    canvas_w: u32,
    canvas_h: u32,
    color: [u8; 4],
    canvas: &mut [u8],
) {
    let mask_w = placement.width;
    let mask_h = placement.height;
    if mask_w == 0 || mask_h == 0 {
        return;
    }
    let dst_x = placement.left.wrapping_sub(base_x);
    let dst_y = (canvas_h as i32 + base_y).wrapping_sub(placement.top);

    let source_w = mask_w as usize;
    let source_h = mask_h as usize;
    let dest_w = canvas_w as usize;
    let dest_h = canvas_h as usize;

    let source_x = if dst_x < 0 { -dst_x as usize } else { 0 };
    let source_y = if dst_y < 0 { -dst_y as usize } else { 0 };
    if source_x >= source_w || source_y >= source_h {
        return;
    }
    let dest_x = if dst_x < 0 { 0 } else { dst_x as usize };
    let dest_y = if dst_y < 0 { 0 } else { dst_y as usize };
    if dest_x >= dest_w || dest_y >= dest_h {
        return;
    }

    let source_end_x = source_w.min(dest_w - dest_x + source_x);
    let source_end_y = source_h.min(dest_h - dest_y + source_y);
    let dest_pitch = dest_w * 4;
    let color_a = color[3] as u32;

    let mut dy = dest_y;
    for sy in source_y..source_end_y {
        let src_row = &mask[sy * source_w..];
        let dst_row = &mut canvas[dy * dest_pitch..];
        dy += 1;
        let mut dx = dest_x * 4;
        for sx in source_x..source_end_x {
            let a = (src_row[sx] as u32 * color_a) >> 8;
            if a >= 255 {
                dst_row[dx] = color[0];
                dst_row[dx + 1] = color[1];
                dst_row[dx + 2] = color[2];
                dst_row[dx + 3] = 255;
            } else if a != 0 {
                let inverse_a = 255 - a;
                for i in 0..3 {
                    let d = dst_row[dx + i] as u32;
                    let c = ((inverse_a * d) + (a * color[i] as u32)) >> 8;
                    dst_row[dx + i] = c as u8;
                }
                let d = dst_row[dx + 3] as u32;
                let c = ((inverse_a * d) + a * 255) >> 8;
                dst_row[dx + 3] = c as u8;
            }
            dx += 4;
        }
    }
}
