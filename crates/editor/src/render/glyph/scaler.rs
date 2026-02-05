use skrifa::GlyphId;
use skrifa::bitmap::{BitmapData, BitmapStrikes, Origin};
use skrifa::instance::Size;
use skrifa::outline::OutlineGlyphCollection;
use tiny_skia::{ColorU8, FillRule, FilterQuality, Paint, Pixmap, PixmapPaint, Transform};

use super::pen::TinySkiaPen;

pub(crate) struct MaskImage {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub offset_x: f32,
    pub offset_y: f32,
}

pub(crate) struct ColorImage {
    pub pixmap: Pixmap,
    pub offset_x: f32,
    pub offset_y: f32,
}

pub(crate) fn rasterize_outline(
    outlines: &OutlineGlyphCollection,
    glyph_id: GlyphId,
    size: Size,
    skew_transform: Option<Transform>,
    subpixel_x: f32,
    subpixel_y: f32,
    darken_x: f32,
    darken_y: f32,
) -> Option<MaskImage> {
    let glyph = outlines.get(glyph_id)?;
    let mut pen = TinySkiaPen::new();
    glyph.draw(size, &mut pen).ok()?;
    let path = pen.finish_emboldened(darken_x, darken_y)?;

    let bounds = path.bounds();

    let skew = skew_transform.map(|t| t.kx).unwrap_or(0.0);
    let extra_width = (bounds.height() * skew.abs()).ceil();

    let padding = 1.0;

    let baseline_in_pixmap = (bounds.bottom() + padding + subpixel_y).ceil();
    let top_space = (-bounds.top() + padding).ceil();

    let width = (bounds.width() + extra_width + padding * 2.0 + subpixel_x).ceil() as u32;
    let height = (baseline_in_pixmap + top_space) as u32;

    if width == 0 || height == 0 || width > 512 || height > 512 {
        return None;
    }

    let mut glyph_pixmap = Pixmap::new(width, height)?;

    let min_skew_x = if skew >= 0.0 {
        skew * bounds.top()
    } else {
        skew * bounds.bottom()
    };

    let offset_x = -bounds.left() - min_skew_x + padding + subpixel_x;
    let offset_y = baseline_in_pixmap;

    let mut render_transform = Transform::from_translate(offset_x, offset_y);
    render_transform = render_transform.pre_scale(1.0, -1.0);

    if let Some(skew_t) = skew_transform {
        render_transform = render_transform.pre_concat(skew_t);
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

    let pixel_count = (width * height) as usize;
    let pixmap_data = glyph_pixmap.data();
    let mut alpha_data = Vec::with_capacity(pixel_count);

    for i in 0..pixel_count {
        alpha_data.push(pixmap_data[i * 4 + 3]);
    }

    Some(MaskImage {
        data: alpha_data,
        width,
        height,
        offset_x: bounds.left() + min_skew_x - padding - subpixel_x,
        offset_y: -baseline_in_pixmap,
    })
}

pub(crate) fn rasterize_bitmap(
    strikes: &BitmapStrikes,
    glyph_id: GlyphId,
    size: Size,
    font_size: f32,
) -> Option<ColorImage> {
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

    Some(ColorImage {
        pixmap: dst_pixmap,
        offset_x: bearing_x * scale,
        offset_y: bearing_y * scale,
    })
}
