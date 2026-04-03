mod bitmap;
mod cache;
mod color;
mod hinting;
mod outline;
mod outline_pen;
mod path;
mod scaler;

pub use cache::GlyphCache;
pub use scaler::ScaleContext;

use editor_resource::FontRegistry;

use crate::types::{Image, Path};
use cache::GlyphCacheKey;
use scaler::rasterize_glyph;

#[derive(Debug, Clone)]
pub enum RasterizedGlyph {
    Path(Path),
    Bitmap(Image),
}

#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    pub raster: RasterizedGlyph,
    pub x: f32,
    pub y: f32,
}

struct GlyphInput {
    id: u32,
    x: f32,
    y: f32,
}

struct RasterizeInput<'a> {
    font_id: u16,
    font_weight: u16,
    font_size: f32,
    embolden: bool,
    skew: Option<f32>,
    glyphs: &'a [GlyphInput],
}

pub fn rasterize(
    run: &editor_view::glyph_run::GlyphRun,
    fonts: &FontRegistry,
    scale_ctx: &mut ScaleContext,
    cache: &mut GlyphCache,
    scale_factor: f32,
) -> Vec<PositionedGlyph> {
    let glyphs: Vec<GlyphInput> = run
        .glyphs
        .iter()
        .map(|g| GlyphInput {
            id: g.id,
            x: g.x,
            y: g.y,
        })
        .collect();
    let input = RasterizeInput {
        font_id: run.font_id,
        font_weight: run.font_weight,
        font_size: run.font_size,
        embolden: run.synthesis.embolden,
        skew: run.synthesis.skew,
        glyphs: &glyphs,
    };
    rasterize_inner(&input, fonts, scale_ctx, cache, scale_factor)
}

fn rasterize_inner(
    input: &RasterizeInput,
    fonts: &FontRegistry,
    scale_ctx: &mut ScaleContext,
    cache: &mut GlyphCache,
    scale_factor: f32,
) -> Vec<PositionedGlyph> {
    let Some(font_data) = fonts.font_data(input.font_id, input.font_weight) else {
        return Vec::new();
    };

    let font_version = fonts.font_version(input.font_id, input.font_weight);
    let embolden = input.embolden;
    let has_skew = input.skew.is_some();
    let scaled_font_size = input.font_size * scale_factor;

    input
        .glyphs
        .iter()
        .filter_map(|glyph| {
            if glyph.id == 0 {
                return None;
            }

            let snapped_x = glyph.x.floor();
            let snapped_y = glyph.y.floor();

            let key = GlyphCacheKey::new(
                input.font_id,
                glyph.id,
                scaled_font_size,
                has_skew,
                embolden,
            );

            let result = match cache.get(&key, font_version) {
                Some(cached) => cached.clone(),
                None => {
                    let result = rasterize_glyph(
                        scale_ctx,
                        font_data,
                        glyph.id,
                        scaled_font_size,
                        embolden,
                        input.skew,
                    );
                    cache.insert(key, result.clone(), font_version);
                    result
                }
            };

            result.map(|r| PositionedGlyph {
                raster: r,
                x: snapped_x,
                y: snapped_y,
            })
        })
        .collect()
}
