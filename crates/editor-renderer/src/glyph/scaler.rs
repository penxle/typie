use skrifa::instance::{LocationRef, NormalizedCoord, Size};
use skrifa::outline::DrawSettings;
use skrifa::{FontRef, GlyphId, MetadataProvider};

use super::RasterizedGlyph;
use super::bitmap::rasterize_bitmap;
use super::color::rasterize_color_outline;
use super::hinting::HintingCache;
use super::outline::Outline;
use super::outline_pen::OutlineWriter;
use super::path::outline_to_path;

pub const EMBOLDEN_RATIO: f32 = 1.0 / 64.0;

pub struct ScaleContext {
    pub outline: Outline,
    pub hinting_cache: HintingCache,
}

impl ScaleContext {
    pub fn new() -> Self {
        Self {
            outline: Outline::new(),
            hinting_cache: HintingCache::new(),
        }
    }
}

pub fn rasterize_glyph(
    ctx: &mut ScaleContext,
    font_data: &[u8],
    glyph_id: u32,
    font_size: f32,
    embolden: bool,
    skew: Option<f32>,
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

    let try_outline_before_bitmap = has_skew || embolden;

    if font.color_glyphs().get(gid).is_some()
        && let Some(image) = rasterize_color_outline(font_data, glyph_id, font_size)
    {
        return Some(RasterizedGlyph::Bitmap(image));
    }

    if try_outline_before_bitmap {
        if let Some(result) = try_outline(ctx, &font, gid, quantized_size, embolden_amount, skew) {
            return Some(result);
        }

        if let Some(image) = rasterize_bitmap(font_data, glyph_id, font_size) {
            return Some(RasterizedGlyph::Bitmap(image));
        }
    } else {
        if let Some(image) = rasterize_bitmap(font_data, glyph_id, font_size) {
            return Some(RasterizedGlyph::Bitmap(image));
        }

        if let Some(result) = try_outline(ctx, &font, gid, quantized_size, embolden_amount, skew) {
            return Some(result);
        }
    }

    None
}

fn try_outline(
    ctx: &mut ScaleContext,
    font: &FontRef<'_>,
    gid: GlyphId,
    quantized_size: f32,
    embolden_amount: f32,
    skew: Option<f32>,
) -> Option<RasterizedGlyph> {
    let outlines = font.outline_glyphs();
    let outline_glyph = outlines.get(gid)?;

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
    let mut writer = OutlineWriter(&mut ctx.outline);
    outline_glyph.draw(settings, &mut writer).ok()?;

    if ctx.outline.is_empty() {
        return None;
    }

    if embolden_amount > 0.0 {
        ctx.outline.embolden(embolden_amount, embolden_amount);
    }

    if let Some(angle) = skew {
        let kx = (angle as f64).to_radians().tan() as f32;
        let skew_transform = zeno::Transform {
            xx: 1.0,
            yx: kx,
            xy: 0.0,
            yy: 1.0,
            x: 0.0,
            y: 0.0,
        };
        ctx.outline.transform(&skew_transform);
    }

    let path = outline_to_path(&ctx.outline, 0.0, 0.0);
    Some(RasterizedGlyph::Path(path))
}
