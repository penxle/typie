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
use editor_view::fragment::GlyphRun;

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

pub fn rasterize(
    run: &GlyphRun,
    fonts: &FontRegistry,
    scale_ctx: &mut ScaleContext,
    cache: &mut GlyphCache,
    scale_factor: f32,
) -> Vec<PositionedGlyph> {
    let Some(font_data) = fonts.font_data(run.font_id, run.font_weight) else {
        return Vec::new();
    };

    let font_version = fonts.font_version(run.font_id, run.font_weight);
    let embolden = run.synthesis.embolden;
    let has_skew = run.synthesis.skew.is_some();
    let scaled_font_size = run.font_size * scale_factor;

    run.glyphs
        .iter()
        .filter_map(|glyph| {
            if glyph.id == 0 {
                return None;
            }

            let snapped_x = glyph.x.floor();
            let snapped_y = glyph.y.floor();

            let key =
                GlyphCacheKey::new(run.font_id, glyph.id, scaled_font_size, has_skew, embolden);

            let result = match cache.get(&key, font_version) {
                Some(cached) => cached.clone(),
                None => {
                    let result = rasterize_glyph(
                        scale_ctx,
                        font_data,
                        glyph.id,
                        scaled_font_size,
                        embolden,
                        run.synthesis.skew,
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
