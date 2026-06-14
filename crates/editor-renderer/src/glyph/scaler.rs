use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::DrawSettings;
use skrifa::raw::TableProvider;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use zeno::Transform as ZTransform;
use zeno::{Point, Verb};

use super::bitmap::rasterize_bitmap;
use super::color::rasterize_color_outline;
use super::hinting::HintingCache;
use super::mask::rasterize_outline_to_mask;
use super::outline::Outline;
use super::outline_pen::OutlineWriter;
use super::scratch::GlyphScratch;
use super::{Content, RasterizedGlyph, SvgPathGlyph};
use crate::types::{Path, PathElement};

pub const EMBOLDEN_RATIO: f32 = 1.0 / 64.0;

pub struct ScaleContext {
    pub outline: Outline,
    pub hinting_cache: HintingCache,
    pub scratch: GlyphScratch,
}

impl ScaleContext {
    pub fn new() -> Self {
        Self {
            outline: Outline::new(),
            hinting_cache: HintingCache::new(),
            scratch: GlyphScratch::new(),
        }
    }
}

impl Default for ScaleContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn rasterize_glyph(
    ctx: &mut ScaleContext,
    font_data: &[u8],
    glyph_id: u32,
    font_size: f32,
    embolden: bool,
    skew: Option<f32>,
    subpixel_offset_x: f32,
) -> Option<RasterizedGlyph> {
    let has_skew = skew.is_some();

    let size_q4 = (font_size * 4.0).round() as u32;
    let quantized_size = size_q4 as f32 / 4.0;

    let embolden_amount = if embolden {
        quantized_size * EMBOLDEN_RATIO
    } else {
        0.0
    };

    let skew_transform = skew.map(|angle| {
        let kx = (angle as f64).to_radians().tan() as f32;
        ZTransform {
            xx: 1.0,
            yx: kx,
            xy: 0.0,
            yy: 1.0,
            x: 0.0,
            y: 0.0,
        }
    });

    let try_outline_before_bitmap = has_skew || embolden;

    if let Some(img) =
        rasterize_color_outline(ctx, font_data, glyph_id, quantized_size, subpixel_offset_x)
    {
        return Some(img);
    }

    if try_outline_before_bitmap {
        if let Some(r) = try_outline_raster(
            ctx,
            font_data,
            glyph_id,
            quantized_size,
            embolden_amount,
            skew_transform,
            subpixel_offset_x,
        ) {
            return Some(r);
        }
        if let Some(r) = rasterize_bitmap(ctx, font_data, glyph_id, quantized_size) {
            return Some(r);
        }
        return None;
    }

    if let Some(r) = rasterize_bitmap(ctx, font_data, glyph_id, quantized_size) {
        return Some(r);
    }
    try_outline_raster(
        ctx,
        font_data,
        glyph_id,
        quantized_size,
        embolden_amount,
        skew_transform,
        subpixel_offset_x,
    )
}

pub fn svg_path_glyph(
    ctx: &mut ScaleContext,
    font_data: &[u8],
    glyph_id: u32,
    font_size: f32,
    embolden: bool,
    skew: Option<f32>,
    subpixel_offset_x: f32,
) -> Option<SvgPathGlyph> {
    let size_q4 = (font_size * 4.0).round() as u32;
    let quantized_size = size_q4 as f32 / 4.0;
    let embolden_amount = if embolden {
        quantized_size * EMBOLDEN_RATIO
    } else {
        0.0
    };
    let skew_transform = skew.map(|angle| {
        let kx = (angle as f64).to_radians().tan() as f32;
        ZTransform {
            xx: 1.0,
            yx: kx,
            xy: 0.0,
            yy: 1.0,
            x: 0.0,
            y: 0.0,
        }
    });

    try_outline_svg_path(
        ctx,
        font_data,
        glyph_id,
        quantized_size,
        embolden_amount,
        skew_transform,
        subpixel_offset_x,
    )
}

fn try_outline_raster(
    ctx: &mut ScaleContext,
    font_data: &[u8],
    glyph_id: u32,
    quantized_size: f32,
    embolden_amount: f32,
    skew_transform: Option<ZTransform>,
    subpixel_offset_x: f32,
) -> Option<RasterizedGlyph> {
    let font = FontRef::from_index(font_data, 0).ok()?;
    let gid = GlyphId::new(glyph_id);
    let outlines = font.outline_glyphs();
    let og = outlines.get(gid)?;

    let size = Size::new(quantized_size);
    let coords: &[NormalizedCoord] = &[];

    let font_bytes = font.data().as_bytes();
    let id = [font_bytes.as_ptr() as u64, font_bytes.len() as u64];

    let settings = if let Some(instance) = ctx.hinting_cache.get(id, &outlines, size, coords) {
        DrawSettings::hinted(instance, false)
    } else {
        DrawSettings::unhinted(size, LocationRef::new(coords))
    };

    ctx.outline.clear();
    og.draw(settings, &mut OutlineWriter(&mut ctx.outline))
        .ok()?;

    if ctx.outline.is_empty() {
        return None;
    }

    if embolden_amount > 0.0 {
        ctx.outline.embolden(embolden_amount, embolden_amount);
    }

    // skew 는 legacy 와 동일하게 점 좌표에 직접 적용 (Mask transform 인자가 아님).
    if let Some(t) = skew_transform {
        ctx.outline.transform(&t);
    }

    let mask_buf = &mut ctx.scratch.bitmap_0;
    let placement = rasterize_outline_to_mask(
        &ctx.outline,
        &mut ctx.scratch.zeno,
        subpixel_offset_x,
        None,
        mask_buf,
    );

    if placement.width == 0 || placement.height == 0 {
        return None;
    }

    Some(RasterizedGlyph {
        data: mask_buf.clone(),
        width: placement.width,
        height: placement.height,
        placement_left: placement.left,
        placement_top: placement.top,
        content: Content::Mask,
    })
}

fn try_outline_svg_path(
    ctx: &mut ScaleContext,
    font_data: &[u8],
    glyph_id: u32,
    quantized_size: f32,
    embolden_amount: f32,
    skew_transform: Option<ZTransform>,
    subpixel_offset_x: f32,
) -> Option<SvgPathGlyph> {
    let font = FontRef::from_index(font_data, 0).ok()?;
    let gid = GlyphId::new(glyph_id);

    let has_svg_glyph = font
        .svg()
        .ok()
        .and_then(|svg| svg.glyph_data(gid).ok().flatten())
        .is_some();
    if !has_svg_glyph {
        return None;
    }

    let outlines = font.outline_glyphs();
    let og = outlines.get(gid)?;

    let size = Size::new(quantized_size);
    let coords: &[NormalizedCoord] = &[];

    let font_bytes = font.data().as_bytes();
    let id = [font_bytes.as_ptr() as u64, font_bytes.len() as u64];

    let settings = if let Some(instance) = ctx.hinting_cache.get(id, &outlines, size, coords) {
        DrawSettings::hinted(instance, false)
    } else {
        DrawSettings::unhinted(size, LocationRef::new(coords))
    };

    ctx.outline.clear();
    og.draw(settings, &mut OutlineWriter(&mut ctx.outline))
        .ok()?;

    if ctx.outline.is_empty() {
        return None;
    }

    if embolden_amount > 0.0 {
        ctx.outline.embolden(embolden_amount, embolden_amount);
    }

    if let Some(t) = skew_transform {
        ctx.outline.transform(&t);
    }

    build_svg_path_glyph(&ctx.outline, subpixel_offset_x)
}

fn build_svg_path_glyph(outline: &Outline, subpixel_offset_x: f32) -> Option<SvgPathGlyph> {
    let (min, max) = outline_point_bounds(outline.points())?;
    let placement_left = (min.x + subpixel_offset_x).floor() as i32;
    let placement_top = max.y.ceil() as i32;
    let mut points = outline.points().iter();
    let mut elements = Vec::with_capacity(outline.verbs().len());

    for verb in outline.verbs() {
        match verb {
            Verb::MoveTo => {
                let p = *points.next()?;
                elements.push(PathElement::MoveTo {
                    x: p.x - placement_left as f32,
                    y: placement_top as f32 - p.y,
                });
            }
            Verb::LineTo => {
                let p = *points.next()?;
                elements.push(PathElement::LineTo {
                    x: p.x - placement_left as f32,
                    y: placement_top as f32 - p.y,
                });
            }
            Verb::QuadTo => {
                let c = *points.next()?;
                let p = *points.next()?;
                elements.push(PathElement::QuadTo {
                    x1: c.x - placement_left as f32,
                    y1: placement_top as f32 - c.y,
                    x: p.x - placement_left as f32,
                    y: placement_top as f32 - p.y,
                });
            }
            Verb::CurveTo => {
                let c1 = *points.next()?;
                let c2 = *points.next()?;
                let p = *points.next()?;
                elements.push(PathElement::CurveTo {
                    x1: c1.x - placement_left as f32,
                    y1: placement_top as f32 - c1.y,
                    x2: c2.x - placement_left as f32,
                    y2: placement_top as f32 - c2.y,
                    x: p.x - placement_left as f32,
                    y: placement_top as f32 - p.y,
                });
            }
            Verb::Close => elements.push(PathElement::Close),
        }
    }

    if elements.is_empty() {
        return None;
    }

    Some(SvgPathGlyph {
        path: Path { elements },
        placement_left,
        placement_top,
    })
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

#[cfg(test)]
mod tests {
    use super::build_svg_path_glyph;
    use crate::glyph::outline::Outline;
    use crate::types::PathElement;
    use zeno::Point;

    #[test]
    fn build_svg_path_glyph_flips_y_and_localizes_points() {
        // outline glyph 좌표를 SVG path 캐시용 local 좌표계로 변환하는지 확인한다.
        let mut outline = Outline::new();
        outline.move_to(Point::new(12.0, 24.0));
        outline.line_to(Point::new(16.0, 20.0));
        outline.close();

        let glyph = build_svg_path_glyph(&outline, 0.0).expect("path glyph must exist");

        assert!(matches!(
            glyph.path.elements[0],
            PathElement::MoveTo { x, y } if x == 0.0 && y == 0.0
        ));
        assert!(matches!(
            glyph.path.elements[1],
            PathElement::LineTo { x, y } if x == 4.0 && y == 4.0
        ));
        assert!(matches!(glyph.path.elements[2], PathElement::Close));
    }
}
