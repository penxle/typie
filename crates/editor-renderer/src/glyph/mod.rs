mod bitmap;
mod cache;
mod color;
mod hinting;
mod mask;
mod outline;
mod outline_pen;
mod scaler;
mod scratch;

pub use cache::GlyphCache;
pub use scaler::ScaleContext;

use crate::types::Transform as RenderTransform;
use cache::GlyphCacheKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Content {
    Mask,
    Color,
}

#[derive(Debug, Clone)]
pub struct RasterizedGlyph {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub placement_left: i32,
    pub placement_top: i32,
    pub content: Content,
}

#[derive(Debug, Clone)]
pub struct PositionedGlyph {
    pub raster: RasterizedGlyph,
    pub blit_x: i32,
    pub blit_y: i32,
}

/// base_transform 의 2×3 매트릭스로 logical (x, y) 를 매핑한다.
#[inline]
fn map_point(t: RenderTransform, x: f32, y: f32) -> (f32, f32) {
    let [a, b, c, d, e, f] = t.m;
    (a * x + c * y + e, b * x + d * y + f)
}

pub fn rasterize(
    run: &editor_view::glyph_run::GlyphRun,
    fonts: &editor_resource::FontRegistry,
    scale_ctx: &mut ScaleContext,
    cache: &mut GlyphCache,
    scale_factor: f32,
    base_transform: RenderTransform,
) -> Vec<PositionedGlyph> {
    let Some(font_data) = fonts.font_data(run.font_id, run.font_weight) else {
        return Vec::new();
    };
    let font_version = fonts.font_version(run.font_id, run.font_weight);
    let scaled_font_size = run.font_size * scale_factor;
    let has_skew = run.synthesis.skew.is_some();
    let embolden = run.synthesis.embolden;

    let mut out = Vec::with_capacity(run.glyphs.len());
    for g in &run.glyphs {
        if g.id == 0 {
            continue;
        }

        // base_transform 은 renderer::ContentVisitor 에서 root_transform =
        // Transform::scale(scale_factor) 로 시작해 누적되므로 이미 device-pixel
        // 좌표계다. 여기서 scale_factor 를 다시 곱하면 이중 적용이 된다.
        let (glyph_x_device, glyph_y_device) = map_point(base_transform, g.x, g.y);

        let snapped_x = (glyph_x_device * 4.0).round() / 4.0;
        let subpixel_x = ((snapped_x - snapped_x.floor()) * 4.0) as u8;

        let key = GlyphCacheKey::new(
            run.font_id,
            g.id,
            scaled_font_size,
            has_skew,
            embolden,
            subpixel_x,
        );

        let raster = match cache.get(&key, font_version) {
            Some(entry) => entry.clone(),
            None => {
                let r = scaler::rasterize_glyph(
                    scale_ctx,
                    font_data,
                    g.id,
                    scaled_font_size,
                    embolden,
                    run.synthesis.skew,
                    subpixel_x as f32 / 4.0,
                );
                cache.insert(key, r.clone(), font_version);
                r
            }
        };

        let Some(raster) = raster else { continue };

        let blit_x = snapped_x.floor() as i32 + raster.placement_left;
        let blit_y = glyph_y_device.floor() as i32 - raster.placement_top;

        out.push(PositionedGlyph {
            raster,
            blit_x,
            blit_y,
        });
    }
    out
}
