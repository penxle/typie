mod blit;
mod pen;
mod scaler;

use parley::FontData;
use pen::TinySkiaPen;
use rustc_hash::FxHashMap;
use scaler::{ColorImage, MaskImage};
use skrifa::bitmap::BitmapStrikes;
use skrifa::instance::Size;
use skrifa::raw::TableProvider;
use skrifa::raw::tables::postscript::dict;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use std::hash::{Hash, Hasher};
use tiny_skia::{Paint, PixmapMut, Transform};

const SUBPIXEL_POS_BITS: u32 = 2;
const SUBPIXEL_POS_COUNT: u32 = 1 << SUBPIXEL_POS_BITS;
const SUBPIXEL_ROUND: f32 = 1.0 / ((SUBPIXEL_POS_COUNT << 1) as f32);
const SUBPIXEL_MASK: u32 = SUBPIXEL_POS_COUNT - 1;

const DARKEN_PARAMS: [(f32, f32); 4] = [
    (500.0, 400.0),
    (1000.0, 275.0),
    (1667.0, 275.0),
    (2333.0, 0.0),
];

const DEFAULT_STEM_WIDTH_PER_1000: f32 = 75.0;
const DARKEN_STRENGTH: f32 = 0.25;

#[derive(Clone, Copy)]
pub struct Glyph {
    pub id: u32,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphCacheKey {
    font_hash: u64,
    glyph_id: u32,
    size_q4: u32,
    has_skew: bool,
    subpixel_x: u8,
    subpixel_y: u8,
}

enum CachedGlyph {
    Mask(MaskImage),
    Color(ColorImage),
    None,
}

pub struct GlyphRenderer {
    cache: FxHashMap<GlyphCacheKey, CachedGlyph>,
}

impl GlyphRenderer {
    pub fn new() -> Self {
        Self {
            cache: FxHashMap::default(),
        }
    }

    pub fn draw_glyphs(
        &mut self,
        pixmap: &mut PixmapMut,
        font: &FontData,
        font_size: f32,
        paint: &Paint,
        transform: Transform,
        glyph_transform: Option<Transform>,
        glyphs: &[Glyph],
    ) {
        let font_data = font.data.as_ref();
        let font_hash = calculate_font_hash(font_data);
        let size_q4 = (font_size * 4.0).round() as u32;
        let quantized_size = size_q4 as f32 / 4.0;
        let has_skew = glyph_transform.is_some();

        let mut font_ref_lazy: Option<FontRef> = None;

        let color = match &paint.shader {
            tiny_skia::Shader::SolidColor(c) => *c,
            _ => tiny_skia::Color::BLACK,
        };

        let color_r = (color.red() * 255.0) as u8;
        let color_g = (color.green() * 255.0) as u8;
        let color_b = (color.blue() * 255.0) as u8;
        let color_a = (color.alpha() * 255.0) as u8;

        let tx = transform.tx;
        let ty = transform.ty;
        let sx = transform.sx;
        let sy = transform.sy;

        let mut darken_xy: Option<(f32, f32)> = None;

        for glyph in glyphs {
            if glyph.id == 0 {
                continue;
            }

            let glyph_x = tx + glyph.x * sx;
            let glyph_y = ty + glyph.y * sy;

            let fract_x = glyph_x - glyph_x.floor();
            let subpixel_x = quantize_subpixel(fract_x.abs());
            let subpixel_y = 0u8;

            let cache_key = GlyphCacheKey {
                font_hash,
                glyph_id: glyph.id,
                size_q4,
                has_skew,
                subpixel_x,
                subpixel_y,
            };

            if !self.cache.contains_key(&cache_key) {
                let font_ref = match font_ref_lazy {
                    Some(ref f) => f,
                    None => match FontRef::new(font_data) {
                        Ok(f) => font_ref_lazy.insert(f),
                        Err(e) => {
                            error!(
                                "[GlyphRenderer] FontRef::new failed: {:?}, font_data.len={}",
                                e,
                                font_data.len()
                            );
                            continue;
                        }
                    },
                };

                let (dx, dy) = *darken_xy.get_or_insert_with(|| {
                    let units_per_em = font_ref.head().unwrap().units_per_em();
                    let dpi_scale = sx;
                    let logical_ppem = quantized_size / dpi_scale;
                    let (stdvw, stdhw) = get_stem_widths(font_ref, units_per_em);
                    let darken_x = compute_stem_darkening(logical_ppem, units_per_em, stdvw)
                        * dpi_scale
                        * DARKEN_STRENGTH;
                    let darken_y = compute_stem_darkening(logical_ppem, units_per_em, stdhw)
                        * dpi_scale
                        * DARKEN_STRENGTH;
                    (darken_x, darken_y)
                });

                let glyph_id = GlyphId::new(glyph.id);
                let size = skrifa::instance::Size::new(quantized_size);
                let outlines = font_ref.outline_glyphs();
                let bitmap_strikes = BitmapStrikes::new(font_ref);

                let subpixel_offset_x = subpixel_x as f32 * (1.0 / SUBPIXEL_POS_COUNT as f32);
                let subpixel_offset_y = subpixel_y as f32 * (1.0 / SUBPIXEL_POS_COUNT as f32);

                let cached = if let Some(mask) = scaler::rasterize_outline(
                    &outlines,
                    glyph_id,
                    size,
                    glyph_transform,
                    subpixel_offset_x,
                    subpixel_offset_y,
                    dx,
                    dy,
                ) {
                    CachedGlyph::Mask(mask)
                } else if let Some(color) =
                    scaler::rasterize_bitmap(&bitmap_strikes, glyph_id, size, quantized_size)
                {
                    CachedGlyph::Color(color)
                } else {
                    CachedGlyph::None
                };
                self.cache.insert(cache_key, cached);
            }

            match self.cache.get(&cache_key) {
                Some(CachedGlyph::Mask(mask)) => {
                    let blit_x = (glyph_x + mask.offset_x).floor() as i32;
                    let blit_y = (glyph_y + mask.offset_y).floor() as i32;
                    blit::blit_mask_d32_a8(
                        pixmap, mask, blit_x, blit_y, color_r, color_g, color_b, color_a,
                    );
                }
                Some(CachedGlyph::Color(color)) => {
                    let blit_x = (glyph_x + color.offset_x).round() as i32;
                    let blit_y = (glyph_y + color.offset_y).round() as i32;
                    blit::blit_color(pixmap, color, blit_x, blit_y);
                }
                _ => {}
            }
        }
    }
}

fn get_stem_widths(font_ref: &FontRef, units_per_em: u16) -> (f32, f32) {
    let em_ratio = 1000.0 / units_per_em as f32;

    if let Some((v, h)) = read_cff_stem_widths(font_ref, em_ratio) {
        if v != h {
            return (v, h);
        }
    }

    if let Some((v, h)) = measure_stem_widths_from_glyphs(font_ref, units_per_em) {
        return (v * em_ratio, h * em_ratio);
    }

    (DEFAULT_STEM_WIDTH_PER_1000, DEFAULT_STEM_WIDTH_PER_1000)
}

fn read_cff_stem_widths(font_ref: &FontRef, em_ratio: f32) -> Option<(f32, f32)> {
    let cff = font_ref.cff().ok()?;
    let top_dict_data = cff.top_dicts().get(0).ok()?;
    let offset_data = cff.offset_data();

    for entry in dict::entries(top_dict_data, None).flatten() {
        if let dict::Entry::PrivateDictRange(range) = entry {
            if let Some(private_data) = offset_data.as_bytes().get(range) {
                let mut stdvw = DEFAULT_STEM_WIDTH_PER_1000;
                let mut stdhw = DEFAULT_STEM_WIDTH_PER_1000;
                for priv_entry in dict::entries(private_data, None).flatten() {
                    match priv_entry {
                        dict::Entry::StdVw(w) => stdvw = w.to_f64() as f32 * em_ratio,
                        dict::Entry::StdHw(w) => stdhw = w.to_f64() as f32 * em_ratio,
                        _ => {}
                    }
                }
                return Some((stdvw, stdhw));
            }
        }
    }

    None
}

fn measure_stem_widths_from_glyphs(font_ref: &FontRef, _units_per_em: u16) -> Option<(f32, f32)> {
    let charmap = font_ref.charmap();
    let outlines = font_ref.outline_glyphs();
    let size = Size::unscaled();

    let vertical_glyph_id = charmap.map('ㅣ')?;
    let horizontal_glyph_id = charmap.map('ㅡ')?;

    let mut v_pen = TinySkiaPen::new();
    outlines
        .get(vertical_glyph_id)?
        .draw(size, &mut v_pen)
        .ok()?;
    let stem_v = v_pen.measure_width_at_mid_y()?;

    let mut h_pen = TinySkiaPen::new();
    outlines
        .get(horizontal_glyph_id)?
        .draw(size, &mut h_pen)
        .ok()?;
    let stem_h = h_pen.measure_height_at_mid_x()?;

    if stem_v > 0.0 && stem_h > 0.0 {
        Some((stem_v, stem_h))
    } else {
        None
    }
}

fn compute_stem_darkening(ppem: f32, units_per_em: u16, stem_width_per_1000: f32) -> f32 {
    let ppem = ppem.max(4.0);
    let em_ratio = 1000.0 / units_per_em as f32;
    if em_ratio < 0.01 {
        return 0.0;
    }

    let scaled_stem = stem_width_per_1000 * ppem;

    let (x1, y1) = DARKEN_PARAMS[0];
    let (x2, y2) = DARKEN_PARAMS[1];
    let (x3, y3) = DARKEN_PARAMS[2];
    let (x4, y4) = DARKEN_PARAMS[3];

    let darken_amount = if scaled_stem < x1 {
        y1 / ppem
    } else if scaled_stem < x2 {
        let xdelta = x2 - x1;
        if xdelta == 0.0 {
            y2 / ppem
        } else {
            let x = stem_width_per_1000 - x1 / ppem;
            x * (y2 - y1) / xdelta + y1 / ppem
        }
    } else if scaled_stem < x3 {
        let xdelta = x3 - x2;
        if xdelta == 0.0 {
            y3 / ppem
        } else {
            let x = stem_width_per_1000 - x2 / ppem;
            x * (y3 - y2) / xdelta + y2 / ppem
        }
    } else if scaled_stem < x4 {
        let xdelta = x4 - x3;
        if xdelta == 0.0 {
            y4 / ppem
        } else {
            let x = stem_width_per_1000 - x3 / ppem;
            x * (y4 - y3) / xdelta + y3 / ppem
        }
    } else {
        y4 / ppem
    };

    let darken_font_units = darken_amount / em_ratio;
    darken_font_units * ppem / units_per_em as f32
}

fn quantize_subpixel(fract: f32) -> u8 {
    let biased = fract + SUBPIXEL_ROUND;
    let fixed = (biased * (SUBPIXEL_POS_COUNT as f32)) as u32;
    (fixed & SUBPIXEL_MASK) as u8
}

fn calculate_font_hash(font_data: &[u8]) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    font_data.as_ptr().hash(&mut hasher);
    font_data.len().hash(&mut hasher);
    crate::global::font_version(font_data.as_ptr()).hash(&mut hasher);
    hasher.finish()
}
