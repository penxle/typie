use parley::FontData;
use rustc_hash::FxHashMap;
use skrifa::bitmap::{BitmapData, BitmapStrikes, Origin};
use skrifa::outline::OutlinePen;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use std::hash::{Hash, Hasher};
use tiny_skia::{
    ColorU8, FillRule, FilterQuality, Paint, PathBuilder, Pixmap, PixmapMut, PixmapPaint, Transform,
};

const GAMMA: f64 = 1.8;
const ALPHA: f64 = 0.85;

const fn generate_alpha_lut() -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut i = 0u32;
    while i < 256 {
        let normalized = i as f64 / 255.0;
        let boosted = pow_approx(normalized, ALPHA);
        let result = boosted * 255.0 + 0.5;
        table[i as usize] = if result > 255.0 { 255 } else { result as u8 };
        i += 1;
    }
    table
}

const ALPHA_LUT: [u8; 256] = generate_alpha_lut();

const fn generate_gamma_to_linear() -> [u16; 256] {
    let mut table = [0u16; 256];
    let mut i = 0u32;
    while i < 256 {
        let srgb = i as f64 / 255.0;
        let linear = pow_approx(srgb, GAMMA);
        table[i as usize] = (linear * 4095.0 + 0.5) as u16;
        i += 1;
    }
    table
}

const fn generate_linear_to_gamma() -> [u8; 4096] {
    let mut table = [0u8; 4096];
    let mut i = 0u32;
    while i < 4096 {
        let linear = i as f64 / 4095.0;
        let srgb = pow_approx(linear, 1.0 / GAMMA);
        let clamped = srgb * 255.0 + 0.5;
        table[i as usize] = if clamped > 255.0 { 255 } else { clamped as u8 };
        i += 1;
    }
    table
}

const SRGB_TO_LINEAR: [u16; 256] = generate_gamma_to_linear();
const LINEAR_TO_SRGB: [u8; 4096] = generate_linear_to_gamma();

const fn pow_approx(base: f64, exp: f64) -> f64 {
    exp_approx(exp * ln_approx(base))
}

const fn ln_approx(x: f64) -> f64 {
    if x <= 0.0 {
        return -1000.0;
    }
    let mut val = x;
    let mut log2 = 0i32;
    while val >= 2.0 {
        val /= 2.0;
        log2 += 1;
    }
    while val < 1.0 {
        val *= 2.0;
        log2 -= 1;
    }
    let y = val - 1.0;
    let ln2 = 0.693147180559945;
    let poly = y * (1.0 - y * (0.5 - y * (0.333333 - y * 0.25)));
    (log2 as f64) * ln2 + poly
}

const fn exp_approx(x: f64) -> f64 {
    if x < -20.0 {
        return 0.0;
    }
    if x > 20.0 {
        return 1e9;
    }
    let ln2 = 0.693147180559945;
    let k = (x / ln2) as i32;
    let r = x - (k as f64) * ln2;
    let exp_r = 1.0 + r * (1.0 + r * (0.5 + r * (0.166667 + r * (0.041667 + r * 0.008333))));
    let mut result = exp_r;
    if k >= 0 {
        let mut i = 0;
        while i < k {
            result *= 2.0;
            i += 1;
        }
    } else {
        let mut i = 0;
        while i < -k {
            result /= 2.0;
            i += 1;
        }
    }
    result
}

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

struct MaskGlyph {
    data: Vec<u8>,
    width: u32,
    height: u32,
    offset_x: f32,
    offset_y: f32,
}

struct ColorGlyph {
    pixmap: Pixmap,
    offset_x: f32,
    offset_y: f32,
}

enum CachedGlyph {
    Mask(MaskGlyph),
    Color(ColorGlyph),
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

        for glyph in glyphs {
            if glyph.id == 0 {
                continue;
            }

            let glyph_x = transform.tx + glyph.x * transform.sx;
            let glyph_y = transform.ty + glyph.y * transform.sy;

            let subpixel_x = ((glyph_x.fract().abs() * 4.0).round() as u8) % 4;
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
                let font_ref =
                    font_ref_lazy.get_or_insert_with(|| FontRef::new(font_data).unwrap());

                let glyph_id = GlyphId::new(glyph.id);
                let size = skrifa::instance::Size::new(quantized_size);
                let outlines = font_ref.outline_glyphs();
                let bitmap_strikes = BitmapStrikes::new(font_ref);

                let subpixel_offset_x = subpixel_x as f32 * 0.25;
                let subpixel_offset_y = subpixel_y as f32 * 0.25;

                let cached = if let Some(mask) = rasterize_outline_glyph(
                    &outlines,
                    glyph_id,
                    size,
                    glyph_transform,
                    subpixel_offset_x,
                    subpixel_offset_y,
                ) {
                    CachedGlyph::Mask(mask)
                } else if let Some(color) =
                    rasterize_bitmap_glyph(&bitmap_strikes, glyph_id, size, quantized_size)
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
                    blit_mask_with_color(
                        pixmap, mask, blit_x, blit_y, color_r, color_g, color_b, color_a,
                    );
                }
                Some(CachedGlyph::Color(color)) => {
                    let blit_x = (glyph_x + color.offset_x).round() as i32;
                    let blit_y = (glyph_y + color.offset_y).round() as i32;
                    blit_with_alpha(pixmap, &color.pixmap, blit_x, blit_y);
                }
                _ => {}
            }
        }
    }
}

fn calculate_font_hash(font_data: &[u8]) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    font_data.as_ptr().hash(&mut hasher);
    font_data.len().hash(&mut hasher);
    hasher.finish()
}

fn rasterize_outline_glyph(
    outlines: &skrifa::outline::OutlineGlyphCollection,
    glyph_id: GlyphId,
    size: skrifa::instance::Size,
    skew_transform: Option<Transform>,
    subpixel_x: f32,
    subpixel_y: f32,
) -> Option<MaskGlyph> {
    let glyph = outlines.get(glyph_id)?;
    let mut pen = TinySkiaPen::new();

    glyph.draw(size, &mut pen).ok()?;

    let path = pen.finish()?;

    let bounds = path.bounds();

    let skew = skew_transform.map(|t| t.kx).unwrap_or(0.0);
    let skew_amount = skew.abs();
    let extra_width = (bounds.height() * skew_amount).ceil();

    let padding = 2.0;

    let baseline_in_pixmap = (bounds.bottom() + padding + subpixel_y).ceil();
    let top_space = (-bounds.top() + padding).ceil();

    let final_width = (bounds.width() + extra_width + padding * 2.0 + subpixel_x).ceil() as u32;
    let final_height = (baseline_in_pixmap + top_space) as u32;

    if final_width == 0 || final_height == 0 || final_width > 512 || final_height > 512 {
        return None;
    }

    let oversample_width = final_width * 3;
    let oversample_height = final_height * 2;
    let mut glyph_pixmap = Pixmap::new(oversample_width, oversample_height)?;

    let min_skew_x = if skew >= 0.0 {
        skew * bounds.top()
    } else {
        skew * bounds.bottom()
    };

    let offset_x = (-bounds.left() - min_skew_x + padding + subpixel_x) * 3.0;
    let offset_y = baseline_in_pixmap * 2.0;

    let mut render_transform = Transform::from_translate(offset_x, offset_y);
    render_transform = render_transform.pre_scale(3.0, -2.0);

    if let Some(skew) = skew_transform {
        render_transform = render_transform.pre_concat(skew);
    }

    let mut white_paint = Paint::default();
    white_paint.set_color_rgba8(255, 255, 255, 255);

    glyph_pixmap.fill_path(
        &path,
        &white_paint,
        FillRule::Winding,
        render_transform,
        None,
    );

    let oversampled_alpha: Vec<u8> = glyph_pixmap
        .data()
        .iter()
        .skip(3)
        .step_by(4)
        .copied()
        .collect();

    let mut alpha_data = Vec::with_capacity((final_width * final_height) as usize);
    for y in 0..final_height {
        for x in 0..final_width {
            let y0 = y * 2;
            let y1 = y * 2 + 1;
            let x0 = x * 3;

            let row0_base = (y0 * oversample_width + x0) as usize;
            let row1_base = (y1 * oversample_width + x0) as usize;

            let a00 = oversampled_alpha[row0_base] as u32;
            let a01 = oversampled_alpha[row0_base + 1] as u32;
            let a02 = oversampled_alpha[row0_base + 2] as u32;
            let a10 = oversampled_alpha[row1_base] as u32;
            let a11 = oversampled_alpha[row1_base + 1] as u32;
            let a12 = oversampled_alpha[row1_base + 2] as u32;

            let avg = ((a00 + a01 + a02 + a10 + a11 + a12 + 3) / 6) as u8;
            alpha_data.push(ALPHA_LUT[avg as usize]);
        }
    }

    Some(MaskGlyph {
        data: alpha_data,
        width: final_width,
        height: final_height,
        offset_x: bounds.left() + min_skew_x - padding - subpixel_x,
        offset_y: -baseline_in_pixmap,
    })
}

fn rasterize_bitmap_glyph(
    strikes: &BitmapStrikes,
    glyph_id: GlyphId,
    size: skrifa::instance::Size,
    font_size: f32,
) -> Option<ColorGlyph> {
    let bitmap_glyph = strikes.glyph_for_size(size, glyph_id)?;

    let src_width = bitmap_glyph.width;
    let src_height = bitmap_glyph.height;

    if src_width == 0 || src_height == 0 {
        return None;
    }

    let ppem = bitmap_glyph.ppem_y as f32;
    let scale = font_size / ppem;

    let (bearing_x, bearing_y) = match bitmap_glyph.placement_origin {
        Origin::TopLeft => (bitmap_glyph.inner_bearing_x, -bitmap_glyph.inner_bearing_y),
        Origin::BottomLeft => (
            bitmap_glyph.inner_bearing_x,
            -(bitmap_glyph.inner_bearing_y - src_height as f32),
        ),
    };

    let dst_width = ((src_width as f32) * scale).ceil() as u32;
    let dst_height = ((src_height as f32) * scale).ceil() as u32;

    if dst_width == 0 || dst_height == 0 {
        return None;
    }

    let src_pixmap = match &bitmap_glyph.data {
        BitmapData::Png(png_data) => {
            let decoded = image::load_from_memory_with_format(png_data, image::ImageFormat::Png)
                .ok()?
                .to_rgba8();
            let mut pixmap = Pixmap::new(decoded.width(), decoded.height())?;
            for (i, pixel) in decoded.pixels().enumerate() {
                let color =
                    ColorU8::from_rgba(pixel[0], pixel[1], pixel[2], pixel[3]).premultiply();
                pixmap.pixels_mut()[i] = color;
            }
            pixmap
        }
        BitmapData::Bgra(data) => {
            let mut pixmap = Pixmap::new(src_width, src_height)?;
            for (i, chunk) in data.chunks_exact(4).enumerate() {
                let color =
                    ColorU8::from_rgba(chunk[2], chunk[1], chunk[0], chunk[3]).premultiply();
                pixmap.pixels_mut()[i] = color;
            }
            pixmap
        }
        BitmapData::Mask(mask_data) => {
            let mut pixmap = Pixmap::new(src_width, src_height)?;
            for (i, &alpha) in mask_data.data.iter().enumerate() {
                let color = ColorU8::from_rgba(0, 0, 0, alpha).premultiply();
                pixmap.pixels_mut()[i] = color;
            }
            pixmap
        }
    };

    let mut dst_pixmap = Pixmap::new(dst_width, dst_height)?;

    let scale_transform = Transform::from_scale(scale, scale);
    let blit_paint = PixmapPaint {
        quality: FilterQuality::Bilinear,
        ..PixmapPaint::default()
    };

    dst_pixmap.draw_pixmap(
        0,
        0,
        src_pixmap.as_ref(),
        &blit_paint,
        scale_transform,
        None,
    );

    Some(ColorGlyph {
        pixmap: dst_pixmap,
        offset_x: bearing_x * scale,
        offset_y: bearing_y * scale,
    })
}

struct TinySkiaPen {
    builder: PathBuilder,
}

impl TinySkiaPen {
    fn new() -> Self {
        Self {
            builder: PathBuilder::new(),
        }
    }

    fn finish(self) -> Option<tiny_skia::Path> {
        self.builder.finish()
    }
}

impl OutlinePen for TinySkiaPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.builder.quad_to(x1, y1, x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.builder.cubic_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

fn blit_mask_with_color(
    dst: &mut PixmapMut,
    mask: &MaskGlyph,
    dst_x: i32,
    dst_y: i32,
    src_r: u8,
    src_g: u8,
    src_b: u8,
    src_a: u8,
) {
    let dst_width = dst.width() as i32;
    let dst_height = dst.height() as i32;
    let src_width = mask.width as i32;
    let src_height = mask.height as i32;

    let src_x_start = if dst_x < 0 { -dst_x } else { 0 };
    let src_y_start = if dst_y < 0 { -dst_y } else { 0 };

    let dst_x_start = dst_x.max(0);
    let dst_y_start = dst_y.max(0);

    let copy_width = (src_width - src_x_start).min(dst_width - dst_x_start);
    let copy_height = (src_height - src_y_start).min(dst_height - dst_y_start);

    if copy_width <= 0 || copy_height <= 0 || src_a == 0 {
        return;
    }

    let mask_data = &mask.data;
    let dst_data = dst.data_mut();
    let src_stride = src_width as usize;
    let dst_stride = dst_width as usize * 4;
    let copy_width_usize = copy_width as usize;

    let is_opaque_color = src_a == 255;

    let src_r_linear = SRGB_TO_LINEAR[src_r as usize] as u32;
    let src_g_linear = SRGB_TO_LINEAR[src_g as usize] as u32;
    let src_b_linear = SRGB_TO_LINEAR[src_b as usize] as u32;
    let color_alpha = src_a as u32;

    for row in 0..copy_height as usize {
        let src_row_start = (src_y_start as usize + row) * src_stride + src_x_start as usize;
        let dst_row_start = (dst_y_start as usize + row) * dst_stride + (dst_x_start as usize) * 4;

        let mask_row = &mask_data[src_row_start..src_row_start + copy_width_usize];
        let dst_row = &mut dst_data[dst_row_start..dst_row_start + copy_width_usize * 4];

        let mut col = 0;
        while col < copy_width_usize {
            let mask_alpha = mask_row[col];

            if mask_alpha == 0 {
                col += 1;
                continue;
            }

            let dst_idx = col * 4;

            if mask_alpha == 255 && is_opaque_color {
                dst_row[dst_idx] = src_r;
                dst_row[dst_idx + 1] = src_g;
                dst_row[dst_idx + 2] = src_b;
                dst_row[dst_idx + 3] = 255;
                col += 1;
                continue;
            }

            let combined_alpha = if is_opaque_color {
                mask_alpha as u32
            } else {
                (color_alpha * mask_alpha as u32 + 128) >> 8
            };

            if combined_alpha == 0 {
                col += 1;
                continue;
            }

            let dst_a = dst_row[dst_idx + 3];

            if combined_alpha >= 255 {
                dst_row[dst_idx] = src_r;
                dst_row[dst_idx + 1] = src_g;
                dst_row[dst_idx + 2] = src_b;
                dst_row[dst_idx + 3] = 255;
            } else {
                let dst_r_linear = SRGB_TO_LINEAR[dst_row[dst_idx] as usize] as u32;
                let dst_g_linear = SRGB_TO_LINEAR[dst_row[dst_idx + 1] as usize] as u32;
                let dst_b_linear = SRGB_TO_LINEAR[dst_row[dst_idx + 2] as usize] as u32;

                let inv_alpha = 255 - combined_alpha;

                let result_r = ((src_r_linear * combined_alpha + dst_r_linear * inv_alpha + 128)
                    >> 8)
                    .min(4095);
                let result_g = ((src_g_linear * combined_alpha + dst_g_linear * inv_alpha + 128)
                    >> 8)
                    .min(4095);
                let result_b = ((src_b_linear * combined_alpha + dst_b_linear * inv_alpha + 128)
                    >> 8)
                    .min(4095);

                dst_row[dst_idx] = LINEAR_TO_SRGB[result_r as usize];
                dst_row[dst_idx + 1] = LINEAR_TO_SRGB[result_g as usize];
                dst_row[dst_idx + 2] = LINEAR_TO_SRGB[result_b as usize];
                dst_row[dst_idx + 3] =
                    ((combined_alpha + ((dst_a as u32 * inv_alpha + 128) >> 8)).min(255)) as u8;
            }

            col += 1;
        }
    }
}

fn blit_with_alpha(dst: &mut PixmapMut, src: &Pixmap, dst_x: i32, dst_y: i32) {
    let dst_width = dst.width() as i32;
    let dst_height = dst.height() as i32;
    let src_width = src.width() as i32;
    let src_height = src.height() as i32;

    let src_x_start = if dst_x < 0 { -dst_x } else { 0 };
    let src_y_start = if dst_y < 0 { -dst_y } else { 0 };

    let dst_x_start = dst_x.max(0);
    let dst_y_start = dst_y.max(0);

    let copy_width = (src_width - src_x_start).min(dst_width - dst_x_start);
    let copy_height = (src_height - src_y_start).min(dst_height - dst_y_start);

    if copy_width <= 0 || copy_height <= 0 {
        return;
    }

    let src_data = src.data();
    let dst_data = dst.data_mut();
    let src_stride = (src_width * 4) as usize;
    let dst_stride = (dst_width * 4) as usize;

    for row in 0..copy_height as usize {
        let src_row_offset = (src_y_start as usize + row) * src_stride + (src_x_start as usize) * 4;
        let dst_row_offset = (dst_y_start as usize + row) * dst_stride + (dst_x_start as usize) * 4;

        let mut src_idx = src_row_offset;
        let mut dst_idx = dst_row_offset;

        for _ in 0..copy_width {
            let src_a = src_data[src_idx + 3] as u32;

            if src_a == 255 {
                dst_data[dst_idx] = src_data[src_idx];
                dst_data[dst_idx + 1] = src_data[src_idx + 1];
                dst_data[dst_idx + 2] = src_data[src_idx + 2];
                dst_data[dst_idx + 3] = 255;
            } else if src_a > 0 {
                let src_r_linear = SRGB_TO_LINEAR[src_data[src_idx] as usize] as u32;
                let src_g_linear = SRGB_TO_LINEAR[src_data[src_idx + 1] as usize] as u32;
                let src_b_linear = SRGB_TO_LINEAR[src_data[src_idx + 2] as usize] as u32;

                let dst_r_linear = SRGB_TO_LINEAR[dst_data[dst_idx] as usize] as u32;
                let dst_g_linear = SRGB_TO_LINEAR[dst_data[dst_idx + 1] as usize] as u32;
                let dst_b_linear = SRGB_TO_LINEAR[dst_data[dst_idx + 2] as usize] as u32;
                let dst_a = dst_data[dst_idx + 3] as u32;

                let inv_a = 255 - src_a;

                let result_r_linear = (src_r_linear + (dst_r_linear * inv_a + 127) / 255).min(4095);
                let result_g_linear = (src_g_linear + (dst_g_linear * inv_a + 127) / 255).min(4095);
                let result_b_linear = (src_b_linear + (dst_b_linear * inv_a + 127) / 255).min(4095);

                dst_data[dst_idx] = LINEAR_TO_SRGB[result_r_linear as usize];
                dst_data[dst_idx + 1] = LINEAR_TO_SRGB[result_g_linear as usize];
                dst_data[dst_idx + 2] = LINEAR_TO_SRGB[result_b_linear as usize];
                dst_data[dst_idx + 3] = ((src_a + (dst_a * inv_a + 127) / 255).min(255)) as u8;
            }

            src_idx += 4;
            dst_idx += 4;
        }
    }
}
