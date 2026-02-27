mod blit;
mod internal;
mod scale;

use parley::FontData;
use rustc_hash::FxHashMap;
use scale::image::{Content, Image};
use scale::{Render, ScaleContext, Source, StrikeWith};
use skrifa::{FontRef, GlyphId};
use std::hash::{Hash, Hasher};
use tiny_skia::{Paint, Path, PathBuilder, PixmapMut, Transform};
use zeno::{Vector, Verb};

const SUBPIXEL_POS_BITS: u32 = 2;
const SUBPIXEL_POS_COUNT: u32 = 1 << SUBPIXEL_POS_BITS;
const SUBPIXEL_ROUND: f32 = 1.0 / ((SUBPIXEL_POS_COUNT << 1) as f32);
const SUBPIXEL_MASK: u32 = SUBPIXEL_POS_COUNT - 1;

const EMBOLDEN_RATIO: f32 = 1.0 / 64.0;

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
    embolden: bool,
    subpixel_x: u8,
    subpixel_y: u8,
}

enum CachedGlyph {
    Rendered(Image),
    None,
}

pub struct GlyphRenderer {
    cache: FxHashMap<GlyphCacheKey, CachedGlyph>,
    scale_context: ScaleContext,
}

impl GlyphRenderer {
    pub fn new() -> Self {
        Self {
            cache: FxHashMap::default(),
            scale_context: ScaleContext::new(),
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
        embolden: bool,
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

        let embolden_amount = if embolden {
            quantized_size * EMBOLDEN_RATIO
        } else {
            0.0
        };
        let render_sources = if has_skew || embolden {
            [
                Source::ColorOutline(0),
                Source::Outline,
                Source::ColorBitmap(StrikeWith::BestFit),
                Source::Bitmap(StrikeWith::BestFit),
            ]
        } else {
            [
                Source::ColorOutline(0),
                Source::ColorBitmap(StrikeWith::BestFit),
                Source::Outline,
                Source::Bitmap(StrikeWith::BestFit),
            ]
        };

        for glyph in glyphs {
            if glyph.id == 0 {
                continue;
            }

            let glyph_x = tx + glyph.x * sx;
            let glyph_y = ty + glyph.y * transform.sy;

            let fract_x = glyph_x - glyph_x.floor();
            let subpixel_x = quantize_subpixel(fract_x.abs());
            let subpixel_y = 0u8;

            let cache_key = GlyphCacheKey {
                font_hash,
                glyph_id: glyph.id,
                size_q4,
                has_skew,
                embolden,
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

                let glyph_id = GlyphId::new(glyph.id);
                let subpixel_offset_x = subpixel_x as f32 * (1.0 / SUBPIXEL_POS_COUNT as f32);
                let subpixel_offset_y = subpixel_y as f32 * (1.0 / SUBPIXEL_POS_COUNT as f32);

                let id = [font_hash, 0];
                let mut scaler = self
                    .scale_context
                    .builder(font_ref.clone(), id)
                    .size(quantized_size)
                    .hint(true)
                    .build();

                // zeno's transform_point: x' = x*xx + y*yx, y' = x*xy + y*yy
                // tiny_skia: x' = x*sx + y*kx, y' = x*ky + y*sy
                let skew_transform = glyph_transform.map(|t| zeno::Transform {
                    xx: t.sx,
                    yx: t.kx,
                    xy: t.ky,
                    yy: t.sy,
                    x: 0.0,
                    y: 0.0,
                });

                let cached = Render::new(&render_sources)
                    .offset(Vector::new(subpixel_offset_x, subpixel_offset_y))
                    .embolden(embolden_amount)
                    .transform(skew_transform)
                    .render(&mut scaler, glyph_id);

                let cached = match cached {
                    Some(image) => CachedGlyph::Rendered(image),
                    None => CachedGlyph::None,
                };
                self.cache.insert(cache_key, cached);
            }

            match self.cache.get(&cache_key) {
                Some(CachedGlyph::Rendered(image)) => {
                    let p = &image.placement;
                    match image.content {
                        Content::Mask => {
                            let blit_x = glyph_x.floor() as i32 + p.left;
                            let blit_y = glyph_y.floor() as i32 - p.top;
                            blit::blit_mask_d32_a8(
                                pixmap,
                                &image.data,
                                p.width,
                                p.height,
                                blit_x,
                                blit_y,
                                color_r,
                                color_g,
                                color_b,
                                color_a,
                            );
                        }
                        Content::Color | Content::SubpixelMask => {
                            let blit_x = glyph_x.floor() as i32 + p.left;
                            let blit_y = glyph_y.floor() as i32 - p.top;
                            blit::blit_color(
                                pixmap,
                                &image.data,
                                p.width,
                                p.height,
                                blit_x,
                                blit_y,
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn for_each_glyph_outline<F>(
        &mut self,
        font: &FontData,
        font_size: f32,
        transform: Transform,
        glyph_transform: Option<Transform>,
        embolden: bool,
        glyphs: &[Glyph],
        mut f: F,
    ) where
        F: FnMut(&Path),
    {
        let font_data = font.data.as_ref();
        let font_ref = match FontRef::new(font_data) {
            Ok(font_ref) => font_ref,
            Err(e) => {
                error!(
                    "[GlyphRenderer] FontRef::new failed for outline export: {:?}, font_data.len={}",
                    e,
                    font_data.len()
                );
                return;
            }
        };

        let font_hash = calculate_font_hash(font_data);
        let size_q4 = (font_size * 4.0).round() as u32;
        let quantized_size = size_q4 as f32 / 4.0;
        let embolden_amount = if embolden {
            quantized_size * EMBOLDEN_RATIO
        } else {
            0.0
        };

        let skew_transform = glyph_transform.map(|t| zeno::Transform {
            xx: t.sx,
            yx: t.kx,
            xy: t.ky,
            yy: t.sy,
            x: 0.0,
            y: 0.0,
        });

        for glyph in glyphs {
            if glyph.id == 0 {
                continue;
            }

            let glyph_point = map_transform(transform, glyph.x, glyph.y);
            let fract_x = glyph_point.x - glyph_point.x.floor();
            let subpixel_x = quantize_subpixel(fract_x.abs());
            let subpixel_offset_x = subpixel_x as f32 * (1.0 / SUBPIXEL_POS_COUNT as f32);

            let mut scaler = self
                .scale_context
                .builder(font_ref.clone(), [font_hash, 0])
                .size(quantized_size)
                .hint(true)
                .build();

            let Some(outline) =
                scaler.scale_outline(GlyphId::new(glyph.id), skew_transform, embolden_amount)
            else {
                continue;
            };

            for layer_idx in 0..outline.len() {
                let Some(layer) = outline.get(layer_idx) else {
                    continue;
                };

                let Some(path) = build_outline_layer_path(
                    layer.points(),
                    layer.verbs(),
                    glyph_point.x + subpixel_offset_x,
                    glyph_point.y,
                ) else {
                    continue;
                };

                f(&path);
            }
        }
    }
}

fn map_transform(transform: Transform, x: f32, y: f32) -> tiny_skia::Point {
    let mut point = tiny_skia::Point::from_xy(x, y);
    transform.map_point(&mut point);
    point
}

fn map_outline_point(point: zeno::Point, origin_x: f32, origin_y: f32) -> (f32, f32) {
    (origin_x + point.x, origin_y - point.y)
}

fn build_outline_layer_path(
    points: &[zeno::Point],
    verbs: &[Verb],
    origin_x: f32,
    origin_y: f32,
) -> Option<Path> {
    let mut pb = PathBuilder::new();
    let mut point_idx = 0usize;

    for verb in verbs {
        match verb {
            Verb::MoveTo => {
                let point = points.get(point_idx)?;
                point_idx += 1;
                let (x, y) = map_outline_point(*point, origin_x, origin_y);
                pb.move_to(x, y);
            }
            Verb::LineTo => {
                let point = points.get(point_idx)?;
                point_idx += 1;
                let (x, y) = map_outline_point(*point, origin_x, origin_y);
                pb.line_to(x, y);
            }
            Verb::QuadTo => {
                let ctrl = points.get(point_idx)?;
                let point = points.get(point_idx + 1)?;
                point_idx += 2;
                let (cx, cy) = map_outline_point(*ctrl, origin_x, origin_y);
                let (x, y) = map_outline_point(*point, origin_x, origin_y);
                pb.quad_to(cx, cy, x, y);
            }
            Verb::CurveTo => {
                let ctrl1 = points.get(point_idx)?;
                let ctrl2 = points.get(point_idx + 1)?;
                let point = points.get(point_idx + 2)?;
                point_idx += 3;
                let (c1x, c1y) = map_outline_point(*ctrl1, origin_x, origin_y);
                let (c2x, c2y) = map_outline_point(*ctrl2, origin_x, origin_y);
                let (x, y) = map_outline_point(*point, origin_x, origin_y);
                pb.cubic_to(c1x, c1y, c2x, c2y, x, y);
            }
            Verb::Close => {
                pb.close();
            }
        }
    }

    pb.finish()
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
