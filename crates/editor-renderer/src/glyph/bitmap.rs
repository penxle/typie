use skrifa::bitmap::{BitmapData, BitmapStrikes, MaskData};
use skrifa::instance::Size;
use skrifa::{FontRef, GlyphId};
use zune_png::PngDecoder;
use zune_png::zune_core::bytestream::ZCursor;
use zune_png::zune_core::colorspace::ColorSpace;
use zune_png::zune_core::options::DecoderOptions;

use crate::types::Image;

pub fn rasterize_bitmap(font_data: &[u8], glyph_id: u32, font_size: f32) -> Option<Image> {
    let font = FontRef::from_index(font_data, 0).ok()?;
    let strikes = BitmapStrikes::new(&font);
    let gid = GlyphId::new(glyph_id);

    let bitmap_glyph = strikes.glyph_for_size(Size::new(font_size), gid)?;

    let src_w = bitmap_glyph.width;
    let src_h = bitmap_glyph.height;
    if src_w == 0 || src_h == 0 {
        return None;
    }

    let mut pixels = match &bitmap_glyph.data {
        BitmapData::Png(png_data) => decode_png_to_rgba(png_data)?,
        BitmapData::Bgra(data) => decode_bgra_to_rgba(data, src_w, src_h)?,
        BitmapData::Mask(mask) => decode_mask_to_rgba(mask, src_w, src_h)?,
    };

    premultiply_rgba(&mut pixels);

    let ppem = bitmap_glyph.ppem_y;
    let scale = if font_size != 0.0 && ppem != 0.0 {
        font_size / ppem
    } else {
        1.0
    };

    let (final_data, final_w, final_h) = if (scale - 1.0).abs() > f32::EPSILON {
        let dst_w = ((src_w as f32) * scale).ceil() as u32;
        let dst_h = ((src_h as f32) * scale).ceil() as u32;
        if dst_w == 0 || dst_h == 0 {
            return None;
        }
        let resized = resize_rgba(&pixels, src_w, src_h, dst_w, dst_h)?;
        (resized, dst_w, dst_h)
    } else {
        (pixels, src_w, src_h)
    };

    Some(Image {
        data: final_data,
        width: final_w,
        height: final_h,
    })
}

fn decode_png_to_rgba(png_data: &[u8]) -> Option<Vec<u8>> {
    let options = DecoderOptions::default()
        .png_set_add_alpha_channel(true)
        .png_set_strip_to_8bit(true);
    let mut decoder = PngDecoder::new_with_options(ZCursor::new(png_data), options);
    let pixels = decoder.decode_raw().ok()?;
    let (w, h) = decoder.dimensions()?;
    let colorspace = decoder.colorspace()?;
    let channels = colorspace.num_components();

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
        _ => {
            if channels == 4 {
                Some(pixels)
            } else {
                None
            }
        }
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
        rgba[off] = chunk[2]; // R
        rgba[off + 1] = chunk[1]; // G
        rgba[off + 2] = chunk[0]; // B
        rgba[off + 3] = chunk[3]; // A
    }

    Some(rgba)
}

fn decode_mask_to_rgba(mask: &MaskData<'_>, width: u32, height: u32) -> Option<Vec<u8>> {
    let pixel_count = (width as usize) * (height as usize);
    let mut alpha = vec![0u8; pixel_count];
    if !decode_bitmap_mask(mask, width, height, &mut alpha) {
        return None;
    }

    let mut rgba = vec![0u8; pixel_count * 4];
    for (i, &a) in alpha.iter().enumerate() {
        let off = i * 4;
        rgba[off] = 255;
        rgba[off + 1] = 255;
        rgba[off + 2] = 255;
        rgba[off + 3] = a;
    }

    Some(rgba)
}

fn decode_bitmap_mask(mask: &MaskData<'_>, width: u32, height: u32, target: &mut [u8]) -> bool {
    let pixel_count = width as usize * height as usize;
    if target.len() < pixel_count {
        return false;
    }

    let bpp = match mask.bpp {
        1 | 2 | 4 | 8 => mask.bpp as usize,
        _ => return false,
    };

    let max_sample = (1u16 << bpp) - 1;
    let row_bits = width as usize * bpp;
    let stride_bits = if mask.is_packed {
        row_bits
    } else {
        row_bits.next_multiple_of(8)
    };

    let total_bits = stride_bits * height as usize;
    let required_bytes = total_bits.div_ceil(8);
    if mask.data.len() < required_bytes {
        return false;
    }

    let mut out = 0usize;
    for y in 0..height as usize {
        let row_base = y * stride_bits;
        for x in 0..width as usize {
            let sample = read_packed_sample(mask.data, row_base + x * bpp, bpp);
            target[out] = if bpp == 8 {
                sample
            } else {
                ((sample as u16 * 255 + max_sample / 2) / max_sample) as u8
            };
            out += 1;
        }
    }

    true
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

fn premultiply_rgba(data: &mut [u8]) {
    for chunk in data.chunks_exact_mut(4) {
        let a = chunk[3] as u16;
        if a == 0 {
            chunk[0] = 0;
            chunk[1] = 0;
            chunk[2] = 0;
        } else if a < 255 {
            chunk[0] = ((chunk[0] as u16 * a) / 255) as u8;
            chunk[1] = ((chunk[1] as u16 * a) / 255) as u8;
            chunk[2] = ((chunk[2] as u16 * a) / 255) as u8;
        }
    }
}

fn resize_rgba(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Option<Vec<u8>> {
    let src_w = src_w as usize;
    let src_h = src_h as usize;
    let dst_w = dst_w as usize;
    let dst_h = dst_h as usize;

    let src_pixels: Vec<resize::px::RGBA<u8>> = src
        .chunks_exact(4)
        .map(|c| resize::px::RGBA {
            r: c[0],
            g: c[1],
            b: c[2],
            a: c[3],
        })
        .collect();

    let mut dst_pixels = vec![
        resize::px::RGBA {
            r: 0,
            g: 0,
            b: 0,
            a: 0
        };
        dst_w * dst_h
    ];

    let mut resizer = resize::new(
        src_w,
        src_h,
        dst_w,
        dst_h,
        resize::Pixel::RGBA8,
        resize::Type::Lanczos3,
    )
    .ok()?;
    resizer.resize(&src_pixels, &mut dst_pixels).ok()?;

    let dst_bytes: Vec<u8> = dst_pixels
        .iter()
        .flat_map(|px| [px.r, px.g, px.b, px.a])
        .collect();

    Some(dst_bytes)
}
