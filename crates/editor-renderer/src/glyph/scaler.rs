use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::DrawSettings;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use zeno::Transform as ZTransform;

use super::bitmap::rasterize_bitmap;
use super::color::rasterize_color_outline;
use super::hinting::HintingCache;
use super::mask::rasterize_outline_to_mask;
use super::outline::Outline;
use super::outline_pen::OutlineWriter;
use super::scratch::GlyphScratch;
use super::{Content, RasterizedGlyph};

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
    let font = FontRef::from_index(font_data, 0).ok()?;
    let gid = GlyphId::new(glyph_id);
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
        if let Some(r) = try_outline(
            ctx,
            &font,
            gid,
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
    try_outline(
        ctx,
        &font,
        gid,
        quantized_size,
        embolden_amount,
        skew_transform,
        subpixel_offset_x,
    )
}

fn try_outline(
    ctx: &mut ScaleContext,
    font: &FontRef<'_>,
    gid: GlyphId,
    quantized_size: f32,
    embolden_amount: f32,
    skew_transform: Option<ZTransform>,
    subpixel_offset_x: f32,
) -> Option<RasterizedGlyph> {
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
