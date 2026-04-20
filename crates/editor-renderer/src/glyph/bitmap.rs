use skrifa::bitmap::{BitmapData, BitmapStrikes, MaskData, Origin};
use skrifa::instance::Size;
use skrifa::{FontRef, GlyphId};
use zune_png::PngDecoder;
use zune_png::zune_core::bytestream::ZCursor;
use zune_png::zune_core::colorspace::ColorSpace;
use zune_png::zune_core::options::DecoderOptions;

use super::scaler::ScaleContext;
use super::{Content, RasterizedGlyph};

pub fn rasterize_bitmap(
    ctx: &mut ScaleContext,
    font_data: &[u8],
    glyph_id: u32,
    font_size: f32,
) -> Option<RasterizedGlyph> {
    let font = FontRef::from_index(font_data, 0).ok()?;
    let strikes = BitmapStrikes::new(&font);
    let gid = GlyphId::new(glyph_id);

    let bg = strikes.glyph_for_size(Size::new(font_size), gid)?;
    let src_w = bg.width;
    let src_h = bg.height;
    if src_w == 0 || src_h == 0 {
        return None;
    }

    let ppem = bg.ppem_y;
    let scale = if font_size != 0.0 && ppem != 0.0 {
        font_size / ppem
    } else {
        1.0
    };
    let dst_w = ((src_w as f32) * scale).ceil() as u32;
    let dst_h = ((src_h as f32) * scale).ceil() as u32;
    if dst_w == 0 || dst_h == 0 {
        return None;
    }

    let (bearing_x, bearing_y) = match bg.placement_origin {
        Origin::TopLeft => (bg.inner_bearing_x, -bg.inner_bearing_y),
        Origin::BottomLeft => (bg.inner_bearing_x, -(bg.inner_bearing_y - src_h as f32)),
    };
    let placement_left = (bearing_x * scale) as i32;
    let placement_top = -(bearing_y * scale) as i32;

    let need_resize = (scale - 1.0).abs() > f32::EPSILON;

    match &bg.data {
        BitmapData::Mask(mask) => {
            let alpha = decode_bitmap_mask(mask, src_w, src_h)?;
            let data = if need_resize {
                resize_mitchell_alpha(ctx, &alpha, src_w, src_h, dst_w, dst_h)?
            } else {
                alpha
            };
            Some(RasterizedGlyph {
                data,
                width: dst_w,
                height: dst_h,
                placement_left,
                placement_top,
                content: Content::Mask,
            })
        }
        BitmapData::Png(png) => {
            let mut rgba = decode_png_to_rgba(png)?;
            premultiply_rgba_inplace(&mut rgba);
            let data = if need_resize {
                resize_mitchell_rgba(ctx, &rgba, src_w, src_h, dst_w, dst_h)?
            } else {
                rgba
            };
            Some(RasterizedGlyph {
                data,
                width: dst_w,
                height: dst_h,
                placement_left,
                placement_top,
                content: Content::Color,
            })
        }
        BitmapData::Bgra(bgra) => {
            let mut rgba = decode_bgra_to_rgba(bgra, src_w, src_h)?;
            premultiply_rgba_inplace(&mut rgba);
            let data = if need_resize {
                resize_mitchell_rgba(ctx, &rgba, src_w, src_h, dst_w, dst_h)?
            } else {
                rgba
            };
            Some(RasterizedGlyph {
                data,
                width: dst_w,
                height: dst_h,
                placement_left,
                placement_top,
                content: Content::Color,
            })
        }
    }
}

fn decode_bitmap_mask(mask: &MaskData<'_>, width: u32, height: u32) -> Option<Vec<u8>> {
    let pixel_count = (width as usize) * (height as usize);

    let bpp = match mask.bpp {
        1 | 2 | 4 | 8 => mask.bpp as usize,
        _ => return None,
    };

    let max_sample = ((1u16 << bpp) - 1) as u16;
    let row_bits = width as usize * bpp;
    let stride_bits = if mask.is_packed {
        row_bits
    } else {
        row_bits.next_multiple_of(8)
    };
    let total_bits = stride_bits * height as usize;
    let required_bytes = total_bits.div_ceil(8);
    if mask.data.len() < required_bytes {
        return None;
    }

    let mut out = vec![0u8; pixel_count];
    let mut cursor = 0usize;
    for y in 0..height as usize {
        let row_base = y * stride_bits;
        for x in 0..width as usize {
            let sample = read_packed_sample(mask.data, row_base + x * bpp, bpp);
            out[cursor] = if bpp == 8 {
                sample
            } else {
                ((sample as u16 * 255 + max_sample / 2) / max_sample) as u8
            };
            cursor += 1;
        }
    }
    Some(out)
}

fn read_packed_sample(data: &[u8], bit_start: usize, bpp: usize) -> u8 {
    let mut sample = 0u8;
    for bit in 0..bpp {
        let bit_index = bit_start + bit;
        let byte = data[bit_index / 8];
        let shift = 7 - (bit_index % 8);
        sample = (sample << 1) | ((byte >> shift) & 1);
    }
    sample
}

fn decode_png_to_rgba(png_data: &[u8]) -> Option<Vec<u8>> {
    let options = DecoderOptions::default()
        .png_set_add_alpha_channel(true)
        .png_set_strip_to_8bit(true);
    let mut decoder = PngDecoder::new_with_options(ZCursor::new(png_data), options);
    let pixels = decoder.decode_raw().ok()?;
    let (w, h) = decoder.dimensions()?;
    let colorspace = decoder.colorspace()?;
    match colorspace {
        ColorSpace::RGBA => Some(pixels),
        ColorSpace::RGB => {
            let mut rgba = Vec::with_capacity(w * h * 4);
            for chunk in pixels.chunks_exact(3) {
                rgba.extend_from_slice(chunk);
                rgba.push(255);
            }
            Some(rgba)
        }
        ColorSpace::LumaA => {
            let mut rgba = Vec::with_capacity(w * h * 4);
            for chunk in pixels.chunks_exact(2) {
                let (l, a) = (chunk[0], chunk[1]);
                rgba.extend_from_slice(&[l, l, l, a]);
            }
            Some(rgba)
        }
        ColorSpace::Luma => {
            let mut rgba = Vec::with_capacity(w * h * 4);
            for &l in &pixels {
                rgba.extend_from_slice(&[l, l, l, 255]);
            }
            Some(rgba)
        }
        _ => None,
    }
}

fn decode_bgra_to_rgba(data: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let expected = (width as usize) * (height as usize) * 4;
    if data.len() < expected {
        return None;
    }
    let mut rgba = vec![0u8; expected];
    for (i, chunk) in data[..expected].chunks_exact(4).enumerate() {
        let off = i * 4;
        rgba[off] = chunk[2];
        rgba[off + 1] = chunk[1];
        rgba[off + 2] = chunk[0];
        rgba[off + 3] = chunk[3];
    }
    Some(rgba)
}

fn premultiply_rgba_inplace(pixels: &mut [u8]) {
    for px in pixels.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a == 0 {
            px[0] = 0;
            px[1] = 0;
            px[2] = 0;
        } else if a < 255 {
            px[0] = (px[0] as u32 * a / 255) as u8;
            px[1] = (px[1] as u32 * a / 255) as u8;
            px[2] = (px[2] as u32 * a / 255) as u8;
        }
    }
}

pub(crate) fn resize_mitchell_alpha(
    ctx: &mut ScaleContext,
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
) -> Option<Vec<u8>> {
    let scratch_size = (dst_w as usize) * (src_h as usize);
    ctx.scratch.bitmap_1.clear();
    ctx.scratch.bitmap_1.resize(scratch_size, 0);
    let mut target = vec![0u8; (dst_w as usize) * (dst_h as usize)];
    if resample(
        src,
        src_w,
        src_h,
        1,
        &mut target,
        dst_w,
        dst_h,
        &mut ctx.scratch.bitmap_1,
        2.0,
        &mitchell,
    ) {
        Some(target)
    } else {
        None
    }
}

pub(crate) fn resize_mitchell_rgba(
    ctx: &mut ScaleContext,
    src: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
) -> Option<Vec<u8>> {
    let scratch_size = (dst_w as usize) * (src_h as usize) * 4;
    ctx.scratch.bitmap_1.clear();
    ctx.scratch.bitmap_1.resize(scratch_size, 0);
    let mut target = vec![0u8; (dst_w as usize) * (dst_h as usize) * 4];
    if resample(
        src,
        src_w,
        src_h,
        4,
        &mut target,
        dst_w,
        dst_h,
        &mut ctx.scratch.bitmap_1,
        2.0,
        &mitchell,
    ) {
        Some(target)
    } else {
        None
    }
}

fn resample<F>(
    image: &[u8],
    width: u32,
    height: u32,
    channels: u32,
    target: &mut [u8],
    target_width: u32,
    target_height: u32,
    scratch: &mut [u8],
    support: f32,
    filter: &F,
) -> bool
where
    F: Fn(f32) -> f32,
{
    let tmp_width = target_width;
    let tmp_height = height;
    let s = 1. / 255.;
    if channels == 1 {
        sample_dir(
            &|x, y| [0., 0., 0., image[(y * width + x) as usize] as f32 * s],
            width,
            height,
            target_width,
            filter,
            support,
            &mut |x, y, p| scratch[(y * tmp_width + x) as usize] = (p[3] * 255.) as u8,
        );
        sample_dir(
            &|y, x| [0., 0., 0., scratch[(y * tmp_width + x) as usize] as f32 * s],
            tmp_height,
            tmp_width,
            target_height,
            filter,
            support,
            &mut |y, x, p| target[(y * target_width + x) as usize] = (p[3] * 255.) as u8,
        );
        true
    } else if channels == 4 {
        sample_dir(
            &|x, y| {
                let row = (y * width * channels + x * channels) as usize;
                [
                    image[row] as f32 * s,
                    image[row + 1] as f32 * s,
                    image[row + 2] as f32 * s,
                    image[row + 3] as f32 * s,
                ]
            },
            width,
            height,
            target_width,
            filter,
            support,
            &mut |x, y, p| {
                let row = (y * target_width * channels + x * channels) as usize;
                scratch[row] = (p[0] * 255.) as u8;
                scratch[row + 1] = (p[1] * 255.) as u8;
                scratch[row + 2] = (p[2] * 255.) as u8;
                scratch[row + 3] = (p[3] * 255.) as u8;
            },
        );
        sample_dir(
            &|y, x| {
                let row = (y * tmp_width * channels + x * channels) as usize;
                [
                    scratch[row] as f32 * s,
                    scratch[row + 1] as f32 * s,
                    scratch[row + 2] as f32 * s,
                    scratch[row + 3] as f32 * s,
                ]
            },
            tmp_height,
            tmp_width,
            target_height,
            filter,
            support,
            &mut |y, x, p| {
                let row = (y * target_width * channels + x * channels) as usize;
                target[row] = (p[0] * 255.) as u8;
                target[row + 1] = (p[1] * 255.) as u8;
                target[row + 2] = (p[2] * 255.) as u8;
                target[row + 3] = (p[3] * 255.) as u8;
            },
        );
        true
    } else {
        false
    }
}

fn sample_dir<Input, Output, F>(
    input: &Input,
    width: u32,
    height: u32,
    new_width: u32,
    filter: &F,
    support: f32,
    output: &mut Output,
) where
    Input: Fn(u32, u32) -> [f32; 4],
    Output: FnMut(u32, u32, &[f32; 4]),
    F: Fn(f32) -> f32,
{
    const MAX_WEIGHTS: usize = 64;
    let mut weights = [0f32; MAX_WEIGHTS];
    let mut num_weights;
    let ratio = width as f32 / new_width as f32;
    let sratio = ratio.max(1.);
    let src_support = support * sratio;
    let isratio = 1. / sratio;
    for outx in 0..new_width {
        let inx = (outx as f32 + 0.5) * ratio;
        let left = (inx - src_support).floor() as i32;
        let mut left = left.max(0).min(width as i32 - 1) as usize;
        let right = (inx + src_support).ceil() as i32;
        let mut right = right.max(left as i32 + 1).min(width as i32) as usize;
        let inx = inx - 0.5;
        while right - left > MAX_WEIGHTS {
            right -= 1;
            left += 1;
        }
        num_weights = 0;
        let mut sum = 0.;
        for i in left..right {
            let w = filter((i as f32 - inx) * isratio);
            weights[num_weights] = w;
            num_weights += 1;
            sum += w;
        }
        let isum = 1. / sum;
        let weights = &weights[..num_weights];
        for y in 0..height {
            let mut accum = [0f32; 4];
            for (i, w) in weights.iter().enumerate() {
                let p = input((left + i) as u32, y);
                let a = p[3];
                accum[0] += p[0] * w * a;
                accum[1] += p[1] * w * a;
                accum[2] += p[2] * w * a;
                accum[3] += p[3] * w;
            }
            if accum[3] != 0. {
                let a = 1. / accum[3];
                accum[0] *= a;
                accum[1] *= a;
                accum[2] *= a;
                accum[3] *= isum;
            }
            output(outx, y, &accum);
        }
    }
}

fn mitchell(x: f32) -> f32 {
    let x = x.abs();
    if x < 1. {
        ((16. + x * x * (21. * x - 36.)) / 18.).abs()
    } else if x < 2. {
        ((32. + x * (-60. + x * (36. - 7. * x))) / 18.).abs()
    } else {
        0.
    }
}
