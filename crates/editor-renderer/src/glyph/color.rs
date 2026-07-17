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
            // 분할 로딩에서 미도착 레이어는 loca 길이 > 0 이지만 0으로 채워져 빈
            // 윤곽으로 파싱된다. 부분 합성을 캐시에 남기는 대신 None 으로 물러나
            // 청크 도착 후 font_version 재시도에 맡긴다. loca 길이 0 은 원래 빈
            // 레이어이므로 기존대로 건너뛴다.
            if glyf_len(&font, u32::from(*layer_gid)) > 0 {
                return None;
            }
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
        data: canvas.into(),
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

fn glyf_len(font: &FontRef<'_>, gid: u32) -> usize {
    let Ok(loca) = font.loca(None) else {
        return 0;
    };
    let start = loca.get_raw(gid as usize).unwrap_or(0) as usize;
    let end = loca.get_raw(gid as usize + 1).unwrap_or(0) as usize;
    end.saturating_sub(start)
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

    for (dy, sy) in (dest_y..).zip(source_y..source_end_y) {
        let src_row = &mask[sy * source_w..];
        let dst_row = &mut canvas[dy * dest_pitch..];
        let mut dx = dest_x * 4;
        for &src_byte in &src_row[source_x..source_end_x] {
            let a = (src_byte as u32 * color_a) >> 8;
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

#[cfg(test)]
mod tests {
    use super::{Content, ScaleContext, rasterize_color_outline};

    fn push_u16(v: &mut Vec<u8>, x: u16) {
        v.extend_from_slice(&x.to_be_bytes());
    }

    fn push_u32(v: &mut Vec<u8>, x: u32) {
        v.extend_from_slice(&x.to_be_bytes());
    }

    fn push_i16(v: &mut Vec<u8>, x: i16) {
        v.extend_from_slice(&x.to_be_bytes());
    }

    fn square_glyph() -> Vec<u8> {
        let mut g = Vec::new();
        push_i16(&mut g, 1);
        for v in [0i16, 0, 100, 100] {
            push_i16(&mut g, v);
        }
        push_u16(&mut g, 3);
        push_u16(&mut g, 0);
        g.extend_from_slice(&[0x01, 0x01, 0x01, 0x01]);
        for dx in [0i16, 100, 0, -100] {
            push_i16(&mut g, dx);
        }
        for dy in [0i16, 0, 100, 0] {
            push_i16(&mut g, dy);
        }
        while g.len() % 4 != 0 {
            g.push(0);
        }
        g
    }

    /// gid0/1: 빈 글리프, gid2/3: 사각형 레이어, gid4: loca 길이 0 인 원래 빈 레이어.
    /// COLR v0: base gid1 → layers [gid2, gid3, gid4].
    fn synthetic_colr_font() -> (Vec<u8>, std::ops::Range<usize>) {
        let sq = square_glyph();
        let sq_len = sq.len() as u32;

        let mut glyf = Vec::new();
        glyf.extend_from_slice(&sq);
        glyf.extend_from_slice(&sq);

        let mut loca = Vec::new();
        for off in [0, 0, 0, sq_len, sq_len * 2, sq_len * 2] {
            push_u32(&mut loca, off);
        }

        let mut head = Vec::new();
        push_u32(&mut head, 0x0001_0000);
        push_u32(&mut head, 0);
        push_u32(&mut head, 0);
        push_u32(&mut head, 0x5F0F_3CF5);
        push_u16(&mut head, 0);
        push_u16(&mut head, 1000);
        head.extend_from_slice(&[0; 16]);
        for v in [0i16, 0, 100, 100] {
            push_i16(&mut head, v);
        }
        push_u16(&mut head, 0);
        push_u16(&mut head, 8);
        push_i16(&mut head, 2);
        push_i16(&mut head, 1);
        push_i16(&mut head, 0);

        let mut maxp = Vec::new();
        push_u32(&mut maxp, 0x0000_5000);
        push_u16(&mut maxp, 5);

        let mut hhea = Vec::new();
        push_u32(&mut hhea, 0x0001_0000);
        for v in [800i16, -200, 0] {
            push_i16(&mut hhea, v);
        }
        push_u16(&mut hhea, 100);
        for v in [0i16, 0, 100, 1, 0, 0, 0, 0, 0, 0, 0] {
            push_i16(&mut hhea, v);
        }
        push_u16(&mut hhea, 1);

        let mut hmtx = Vec::new();
        push_u16(&mut hmtx, 100);
        push_i16(&mut hmtx, 0);
        for _ in 0..4 {
            push_i16(&mut hmtx, 0);
        }

        let mut colr = Vec::new();
        push_u16(&mut colr, 0);
        push_u16(&mut colr, 1);
        push_u32(&mut colr, 14);
        push_u32(&mut colr, 20);
        push_u16(&mut colr, 3);
        push_u16(&mut colr, 1);
        push_u16(&mut colr, 0);
        push_u16(&mut colr, 3);
        for (g, p) in [(2u16, 0u16), (3, 1), (4, 2)] {
            push_u16(&mut colr, g);
            push_u16(&mut colr, p);
        }

        let tables: [(&[u8; 4], &[u8]); 7] = [
            (b"COLR", &colr),
            (b"glyf", &glyf),
            (b"head", &head),
            (b"hhea", &hhea),
            (b"hmtx", &hmtx),
            (b"loca", &loca),
            (b"maxp", &maxp),
        ];

        let mut font = Vec::new();
        push_u32(&mut font, 0x0001_0000);
        push_u16(&mut font, tables.len() as u16);
        push_u16(&mut font, 0);
        push_u16(&mut font, 0);
        push_u16(&mut font, 0);
        let mut off = 12 + 16 * tables.len() as u32;
        let mut offsets = Vec::new();
        for (tag, data) in &tables {
            font.extend_from_slice(*tag);
            push_u32(&mut font, 0);
            push_u32(&mut font, off);
            push_u32(&mut font, data.len() as u32);
            offsets.push(off as usize);
            off += data.len().next_multiple_of(4) as u32;
        }
        let glyf_start = offsets[1];
        for (_, data) in &tables {
            font.extend_from_slice(data);
            while font.len() % 4 != 0 {
                font.push(0);
            }
        }
        let gid3_range = glyf_start + sq.len()..glyf_start + sq.len() * 2;
        (font, gid3_range)
    }

    #[test]
    fn colr_full_layers_rasterize() {
        let (font, _) = synthetic_colr_font();
        let mut ctx = ScaleContext::new();
        let raster = rasterize_color_outline(&mut ctx, &font, 1, 16.0, 0.0).expect("full layers");
        assert_eq!(raster.content, Content::Color);
        assert!(raster.width > 0 && raster.height > 0);
    }

    #[test]
    fn colr_unloaded_layer_returns_none() {
        let (mut font, gid3_range) = synthetic_colr_font();
        font[gid3_range].fill(0);
        let mut ctx = ScaleContext::new();
        assert!(rasterize_color_outline(&mut ctx, &font, 1, 16.0, 0.0).is_none());
    }
}
