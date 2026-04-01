// Vendored from swash 0.2.6 (MIT/Apache-2.0), adapted for skrifa 0.40.
// Scaling, hinting and rasterization of visual glyph representations.

#![allow(dead_code)]

pub mod image;
pub mod outline;

mod bitmap;
mod color;
mod hinting_cache;

use hinting_cache::HintingCache;
use image::*;
use outline::*;
use skrifa::{
    GlyphId, MetadataProvider,
    bitmap::{BitmapData, BitmapStrikes, MaskData, Origin},
    instance::{NormalizedCoord as SkrifaNormalizedCoord, Size as SkrifaSize},
    outline::OutlineGlyphCollection,
    raw::TableProvider,
};

use color::ColorProxy;
use zeno::Placement;
use zeno::{Format, Mask, Origin as ZenoOrigin, Point, Scratch, Style, Transform, Vector};

pub use bitmap::decode_png;

/// Index of a color palette.
pub type PaletteIndex = u16;

/// Index of a bitmap strike.
pub type StrikeIndex = u32;

/// Bitmap strike selection mode.
#[derive(Copy, Clone, Debug)]
pub enum StrikeWith {
    /// Load a bitmap only if the exact size is available.
    ExactSize,
    /// Load a bitmap of the best available size.
    BestFit,
    /// Loads a bitmap of the largest size available.
    LargestSize,
    /// Load a bitmap from the specified strike.
    Index(StrikeIndex),
}

/// Glyph sources for the renderer.
#[derive(Copy, Clone, Debug)]
pub enum Source {
    /// Scalable outlines.
    Outline,
    /// Layered color scalable outlines.
    ColorOutline(PaletteIndex),
    /// Embedded alpha bitmaps.
    Bitmap(StrikeWith),
    /// Embedded color bitmaps.
    ColorBitmap(StrikeWith),
}

impl Default for Source {
    fn default() -> Self {
        Self::Outline
    }
}

/// Context that manages caches and scratch buffers for scaling.
pub struct ScaleContext {
    state: State,
    hinting_cache: HintingCache,
    coords: Vec<SkrifaNormalizedCoord>,
}

struct State {
    scratch0: Vec<u8>,
    scratch1: Vec<u8>,
    outline: Outline,
    rcx: Scratch,
}

impl ScaleContext {
    /// Creates a new scaling context.
    pub fn new() -> Self {
        Self {
            state: State {
                scratch0: Vec::new(),
                scratch1: Vec::new(),
                outline: Outline::new(),
                rcx: Scratch::new(),
            },
            hinting_cache: HintingCache::default(),
            coords: Vec::new(),
        }
    }

    /// Creates a new builder for constructing a scaler with this context
    /// and the specified skrifa FontRef.
    pub fn builder<'a>(&'a mut self, font: skrifa::FontRef<'a>, id: [u64; 2]) -> ScalerBuilder<'a> {
        ScalerBuilder::new(self, font, id)
    }
}

impl Default for ScaleContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for configuring a scaler.
pub struct ScalerBuilder<'a> {
    state: &'a mut State,
    hinting_cache: &'a mut HintingCache,
    font: skrifa::FontRef<'a>,
    outlines: Option<OutlineGlyphCollection<'a>>,
    color: ColorProxy,
    upem: u16,
    id: [u64; 2],
    coords: &'a mut Vec<SkrifaNormalizedCoord>,
    size: f32,
    hint: bool,
}

impl<'a> ScalerBuilder<'a> {
    fn new(context: &'a mut ScaleContext, font: skrifa::FontRef<'a>, id: [u64; 2]) -> Self {
        let outlines = Some(font.outline_glyphs());
        let upem = font.head().ok().map(|h| h.units_per_em()).unwrap_or(1);
        let font_data = font.data().as_bytes();
        let color = ColorProxy::from_font_data(font_data, 0);
        Self {
            state: &mut context.state,
            hinting_cache: &mut context.hinting_cache,
            font,
            outlines,
            color,
            upem,
            id,
            coords: &mut context.coords,
            size: 0.,
            hint: false,
        }
    }

    /// Specifies the font size in pixels per em.
    pub fn size(mut self, ppem: f32) -> Self {
        self.size = ppem.max(0.);
        self
    }

    /// Specifies whether to apply hinting to outlines.
    pub fn hint(mut self, yes: bool) -> Self {
        self.hint = yes;
        self
    }

    /// Builds a scaler for the current configuration.
    pub fn build(self) -> Scaler<'a> {
        let skrifa_size = if self.size != 0.0 && self.upem != 0 {
            SkrifaSize::new(self.size)
        } else {
            SkrifaSize::unscaled()
        };
        let hinting_instance = match (self.hint, &self.outlines) {
            (true, Some(outlines)) => {
                let key = hinting_cache::HintingKey {
                    id: self.id,
                    outlines,
                    size: skrifa_size,
                    coords: self.coords,
                };
                self.hinting_cache.get(&key)
            }
            _ => None,
        };
        Scaler {
            state: self.state,
            font: self.font,
            outlines: self.outlines,
            hinting_instance,
            color: self.color,
            coords: &self.coords[..],
            size: self.size,
            skrifa_size,
        }
    }
}

/// Scales outline and bitmap glyphs.
pub struct Scaler<'a> {
    state: &'a mut State,
    font: skrifa::FontRef<'a>,
    outlines: Option<OutlineGlyphCollection<'a>>,
    hinting_instance: Option<&'a skrifa::outline::HintingInstance>,
    color: ColorProxy,
    coords: &'a [SkrifaNormalizedCoord],
    size: f32,
    skrifa_size: SkrifaSize,
}

impl<'a> Scaler<'a> {
    /// Returns true if scalable glyph outlines are available.
    pub fn has_outlines(&self) -> bool {
        self.outlines
            .as_ref()
            .map(|outlines| outlines.format().is_some())
            .unwrap_or_default()
    }

    /// Scales a glyph outline and returns it as vector geometry.
    pub fn scale_outline(
        &mut self,
        glyph_id: GlyphId,
        transform: Option<Transform>,
        embolden: f32,
    ) -> Option<Outline> {
        if !self.has_outlines() {
            return None;
        }

        self.state.outline.clear();
        if !self.scale_outline_impl(glyph_id, None, None) {
            return None;
        }

        if embolden != 0.0 {
            self.state.outline.embolden(embolden, embolden);
        }

        if let Some(transform) = transform {
            self.state.outline.transform(&transform);
        }

        Some(self.state.outline.clone())
    }

    /// Returns true if scalable color glyph outlines are available.
    pub fn has_color_outlines(&self) -> bool {
        self.color.colr != 0 && self.color.cpal != 0
    }

    fn scale_outline_impl(
        &mut self,
        glyph_id: GlyphId,
        color_index: Option<u16>,
        outline: Option<&mut Outline>,
    ) -> bool {
        let mut outline = match outline {
            Some(x) => x,
            _ => &mut self.state.outline,
        };
        if let Some(outlines) = &self.outlines {
            if let Some(glyph) = outlines.get(glyph_id) {
                outline.begin_layer(color_index);
                let settings: skrifa::outline::DrawSettings =
                    if let Some(hinting_instance) = &self.hinting_instance {
                        (*hinting_instance).into()
                    } else {
                        (
                            self.skrifa_size,
                            skrifa::instance::LocationRef::new(self.coords),
                        )
                            .into()
                    };
                if glyph
                    .draw(settings, &mut OutlineWriter(&mut outline))
                    .is_ok()
                {
                    outline.maybe_close();
                    outline.finish();
                    return true;
                }
            }
        }
        false
    }

    fn scale_color_outline_impl(&mut self, glyph_id: GlyphId) -> bool {
        if !self.has_color_outlines() {
            return false;
        }
        let layers = match self
            .color
            .layers(self.font.data().as_bytes(), glyph_id.to_u32() as u16)
        {
            Some(layers) => layers,
            _ => return false,
        };
        self.state.outline.clear();
        for i in 0..layers.len() {
            let layer = match layers.get(i) {
                Some(layer) => layer,
                _ => return false,
            };
            if !self.scale_outline_impl(
                GlyphId::new(layer.glyph_id as u32),
                layer.color_index,
                None,
            ) {
                return false;
            }
        }
        true
    }

    /// Returns true if color bitmaps are available.
    pub fn has_color_bitmaps(&self) -> bool {
        BitmapStrikes::new(&self.font).iter().next().is_some()
    }

    fn scale_bitmap_impl(
        &mut self,
        glyph_id: GlyphId,
        color: bool,
        strike: StrikeWith,
        image: &mut Image,
    ) -> Option<bool> {
        image.clear();
        let size = self.size;
        let strikes = BitmapStrikes::new(&self.font);

        let bitmap_glyph = match strike {
            StrikeWith::BestFit | StrikeWith::ExactSize => {
                if size == 0. {
                    return None;
                }
                let skrifa_size = SkrifaSize::new(size);
                strikes.glyph_for_size(skrifa_size, glyph_id)?
            }
            StrikeWith::LargestSize => {
                let mut best = None;
                for s in strikes.iter() {
                    if let Some(g) = s.get(glyph_id) {
                        match &best {
                            None => best = Some(g),
                            Some(prev) => {
                                if g.ppem_y > prev.ppem_y {
                                    best = Some(g);
                                }
                            }
                        }
                    }
                }
                best?
            }
            StrikeWith::Index(_) => {
                return None;
            }
        };

        let src_width = bitmap_glyph.width;
        let src_height = bitmap_glyph.height;
        if src_width == 0 || src_height == 0 {
            return None;
        }

        let ppem = bitmap_glyph.ppem_y as f32;
        let scale = if size != 0. && ppem != 0. {
            size / ppem
        } else {
            1.0
        };

        let (bearing_x, bearing_y) = match bitmap_glyph.placement_origin {
            Origin::TopLeft => (bitmap_glyph.inner_bearing_x, -bitmap_glyph.inner_bearing_y),
            Origin::BottomLeft => (
                bitmap_glyph.inner_bearing_x,
                -(bitmap_glyph.inner_bearing_y - src_height as f32),
            ),
        };

        // Decode bitmap data to either RGBA or alpha mask.
        let src_buf_size = (src_width * src_height * 4) as usize;
        self.state.scratch0.clear();
        self.state.scratch0.resize(src_buf_size, 0);

        let decoded = match &bitmap_glyph.data {
            BitmapData::Png(png_data) if color => {
                self.state.scratch1.clear();
                let (dw, dh) =
                    decode_png(png_data, &mut self.state.scratch1, &mut self.state.scratch0)?;
                // Premultiply alpha
                premultiply_rgba(&mut self.state.scratch0[..(dw * dh * 4) as usize]);
                Some((dw, dh))
            }
            BitmapData::Bgra(data) if color => {
                // Convert BGRA to premultiplied RGBA
                for (i, chunk) in data.chunks_exact(4).enumerate() {
                    let (b, g, r, a) = (chunk[0], chunk[1], chunk[2], chunk[3]);
                    let off = i * 4;
                    if a == 255 {
                        self.state.scratch0[off] = r;
                        self.state.scratch0[off + 1] = g;
                        self.state.scratch0[off + 2] = b;
                        self.state.scratch0[off + 3] = a;
                    } else if a == 0 {
                        self.state.scratch0[off..off + 4].fill(0);
                    } else {
                        let a16 = a as u16;
                        self.state.scratch0[off] = ((r as u16 * a16) / 255) as u8;
                        self.state.scratch0[off + 1] = ((g as u16 * a16) / 255) as u8;
                        self.state.scratch0[off + 2] = ((b as u16 * a16) / 255) as u8;
                        self.state.scratch0[off + 3] = a;
                    }
                }
                Some((src_width, src_height))
            }
            BitmapData::Mask(mask_data) if !color => {
                let decoded_len = (src_width * src_height) as usize;
                self.state.scratch0.clear();
                self.state.scratch0.resize(decoded_len, 0);
                if !decode_bitmap_mask(mask_data, src_width, src_height, &mut self.state.scratch0) {
                    return None;
                }

                let dst_width = ((src_width as f32) * scale).ceil() as u32;
                let dst_height = ((src_height as f32) * scale).ceil() as u32;
                if dst_width == 0 || dst_height == 0 {
                    return None;
                }

                if scale != 1.0 {
                    image.data.resize((dst_width * dst_height) as usize, 0);
                    if !bitmap::resize(
                        &self.state.scratch0,
                        src_width,
                        src_height,
                        1,
                        &mut image.data,
                        dst_width,
                        dst_height,
                        bitmap::Filter::Mitchell,
                        Some(&mut self.state.scratch1),
                    ) {
                        return None;
                    }
                } else {
                    image.data.clear();
                    image.data.extend_from_slice(&self.state.scratch0);
                }

                let left = (bearing_x * scale) as i32;
                let top = -(bearing_y * scale) as i32;
                image.placement = Placement {
                    left,
                    top,
                    width: dst_width,
                    height: dst_height,
                };
                image.content = Content::Mask;
                image.source = match color {
                    true => Source::ColorBitmap(strike),
                    false => Source::Bitmap(strike),
                };
                return Some(true);
            }
            _ => return None,
        }?;

        let (dw, dh) = decoded;
        let dst_width = ((dw as f32) * scale).ceil() as u32;
        let dst_height = ((dh as f32) * scale).ceil() as u32;

        if dst_width == 0 || dst_height == 0 {
            return None;
        }

        if scale != 1.0 {
            image.data.resize((dst_width * dst_height * 4) as usize, 0);
            if !bitmap::resize(
                &self.state.scratch0[..(dw * dh * 4) as usize],
                dw,
                dh,
                4,
                &mut image.data,
                dst_width,
                dst_height,
                bitmap::Filter::Mitchell,
                Some(&mut self.state.scratch1),
            ) {
                return None;
            }
        } else {
            image.data.clear();
            image
                .data
                .extend_from_slice(&self.state.scratch0[..(dw * dh * 4) as usize]);
        }

        let left = (bearing_x * scale) as i32;
        let top = -(bearing_y * scale) as i32;
        image.placement = Placement {
            left,
            top,
            width: dst_width,
            height: dst_height,
        };
        image.source = match color {
            true => Source::ColorBitmap(strike),
            false => Source::Bitmap(strike),
        };
        image.content = Content::Color;
        Some(true)
    }
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

/// Builder type for rendering a glyph into an image.
pub struct Render<'a> {
    sources: &'a [Source],
    format: Format,
    offset: Point,
    transform: Option<Transform>,
    embolden: f32,
    foreground: [u8; 4],
    style: Style<'a>,
}

impl<'a> Render<'a> {
    /// Creates a new builder for configuring rendering using the specified
    /// prioritized list of sources.
    pub fn new(sources: &'a [Source]) -> Self {
        Self {
            sources,
            format: Format::Alpha,
            offset: Point::new(0., 0.),
            transform: None,
            embolden: 0.,
            foreground: [128, 128, 128, 255],
            style: Style::default(),
        }
    }

    /// Specifies the target format for rasterizing an outline.
    pub fn format(&mut self, format: Format) -> &mut Self {
        self.format = format;
        self
    }

    /// Specifies the path style to use when rasterizing an outline.
    pub fn style(&mut self, style: impl Into<Style<'a>>) -> &mut Self {
        self.style = style.into();
        self
    }

    /// Specifies an additional offset to apply when rasterizing an outline.
    pub fn offset(&mut self, offset: Vector) -> &mut Self {
        self.offset = offset;
        self
    }

    /// Specifies a transformation matrix to apply when rasterizing an outline.
    pub fn transform(&mut self, transform: Option<Transform>) -> &mut Self {
        self.transform = transform;
        self
    }

    /// Specifies the strength of a faux bold transform to apply when
    /// rasterizing an outline.
    pub fn embolden(&mut self, strength: f32) -> &mut Self {
        self.embolden = strength;
        self
    }

    /// Specifies an RGBA color to use when rasterizing layers of a color
    /// outline that do not directly reference a palette color.
    pub fn default_color(&mut self, color: [u8; 4]) -> &mut Self {
        self.foreground = color;
        self
    }

    /// Renders the specified glyph using the current configuration into
    /// the provided image.
    pub fn render_into(&self, scaler: &mut Scaler, glyph_id: GlyphId, image: &mut Image) -> bool {
        for source in self.sources {
            match source {
                Source::Outline => {
                    if !scaler.has_outlines() {
                        continue;
                    }
                    scaler.state.outline.clear();
                    if scaler.scale_outline_impl(glyph_id, None, None) {
                        let state = &mut scaler.state;
                        let rcx = &mut state.rcx;
                        let outline = &mut state.outline;
                        if self.embolden != 0. {
                            outline.embolden(self.embolden, self.embolden);
                        }
                        let placement = Mask::with_scratch(outline.path(), rcx)
                            .format(self.format)
                            .origin(ZenoOrigin::BottomLeft)
                            .style(self.style)
                            .offset(self.offset)
                            .render_offset(self.offset)
                            .transform(self.transform)
                            .inspect(|fmt, w, h| {
                                image.data.resize(fmt.buffer_size(w, h), 0);
                            })
                            .render_into(&mut image.data[..], None);
                        image.placement = placement;
                        image.content = if self.format == Format::Alpha {
                            Content::Mask
                        } else {
                            Content::SubpixelMask
                        };
                        image.source = Source::Outline;
                        return true;
                    }
                }
                Source::ColorOutline(palette_index) => {
                    if !scaler.has_color_outlines() {
                        continue;
                    }
                    scaler.state.outline.clear();
                    if scaler.scale_color_outline_impl(glyph_id) {
                        let font_data = scaler.font.data().as_bytes();
                        let color_proxy = &scaler.color;
                        let state = &mut scaler.state;
                        let scratch = &mut state.scratch0;
                        let rcx = &mut state.rcx;
                        let outline = &mut state.outline;
                        if let Some(transform) = &self.transform {
                            outline.transform(transform);
                        }
                        let palette = color_proxy.palette(font_data, *palette_index);
                        let total_bounds = outline.bounds();
                        let base_x = (total_bounds.min.x + self.offset.x).floor() as i32;
                        let base_y = (total_bounds.min.y + self.offset.y).ceil() as i32;
                        let base_w = total_bounds.width().ceil() as u32;
                        let base_h = total_bounds.height().ceil() as u32;

                        image.data.resize((base_w * base_h * 4) as usize, 0);
                        image.placement.left = base_x;
                        image.placement.top = base_h as i32 + base_y;
                        image.placement.width = total_bounds.width().ceil() as u32;
                        image.placement.height = total_bounds.height().ceil() as u32;

                        let mut ok = true;
                        for i in 0..outline.len() {
                            let layer = match outline.get(i) {
                                Some(layer) => layer,
                                _ => {
                                    ok = false;
                                    break;
                                }
                            };

                            scratch.clear();
                            let placement = Mask::with_scratch(layer.path(), rcx)
                                .origin(ZenoOrigin::BottomLeft)
                                .style(self.style)
                                .offset(self.offset)
                                .render_offset(self.offset)
                                .inspect(|fmt, w, h| {
                                    scratch.resize(fmt.buffer_size(w, h), 0);
                                })
                                .render_into(&mut scratch[..], None);
                            let color = layer
                                .color_index()
                                .and_then(|i| palette.map(|p| p.get(i)))
                                .unwrap_or(self.foreground);
                            bitmap::blit(
                                &scratch[..],
                                placement.width,
                                placement.height,
                                placement.left.wrapping_sub(base_x),
                                (base_h as i32 + base_y).wrapping_sub(placement.top),
                                color,
                                &mut image.data,
                                base_w,
                                base_h,
                            );
                        }
                        if ok {
                            image.source = Source::ColorOutline(*palette_index);
                            image.content = Content::Color;
                            return true;
                        }
                    }
                }
                Source::Bitmap(mode) => {
                    if scaler.scale_bitmap_impl(glyph_id, false, *mode, image) == Some(true) {
                        return true;
                    }
                }
                Source::ColorBitmap(mode) => {
                    if scaler.scale_bitmap_impl(glyph_id, true, *mode, image) == Some(true) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// 글리프 소스를 판별한다. zeno 래스터라이제이션 없이 소스 타입만 결정한다.
    ///
    /// - `Source::Outline`: 아웃라인 존재 확인만 수행. `image.data`는 비어있음.
    /// - `Source::ColorOutline`: 래스터라이즈된 이미지 반환 (GPU에서도 이미지로 합성 필요).
    /// - `Source::Bitmap` / `Source::ColorBitmap`: 디코딩된 이미지 반환.
    pub fn detect_source(&self, scaler: &mut Scaler, glyph_id: GlyphId, image: &mut Image) -> bool {
        for source in self.sources {
            match source {
                Source::Outline => {
                    if !scaler.has_outlines() {
                        continue;
                    }
                    scaler.state.outline.clear();
                    if scaler.scale_outline_impl(glyph_id, None, None) {
                        image.source = Source::Outline;
                        return true;
                    }
                }
                Source::ColorOutline(palette_index) => {
                    if !scaler.has_color_outlines() {
                        continue;
                    }
                    // ColorOutline은 래스터라이즈 필요 — render_into와 동일 경로
                    scaler.state.outline.clear();
                    if scaler.scale_color_outline_impl(glyph_id) {
                        let font_data = scaler.font.data().as_bytes();
                        let color_proxy = &scaler.color;
                        let state = &mut scaler.state;
                        let scratch = &mut state.scratch0;
                        let rcx = &mut state.rcx;
                        let outline = &mut state.outline;
                        if let Some(transform) = &self.transform {
                            outline.transform(transform);
                        }
                        let palette = color_proxy.palette(font_data, *palette_index);
                        let total_bounds = outline.bounds();
                        let base_x = (total_bounds.min.x + self.offset.x).floor() as i32;
                        let base_y = (total_bounds.min.y + self.offset.y).ceil() as i32;
                        let base_w = total_bounds.width().ceil() as u32;
                        let base_h = total_bounds.height().ceil() as u32;

                        image.data.resize((base_w * base_h * 4) as usize, 0);
                        image.placement.left = base_x;
                        image.placement.top = base_h as i32 + base_y;
                        image.placement.width = total_bounds.width().ceil() as u32;
                        image.placement.height = total_bounds.height().ceil() as u32;

                        let mut ok = true;
                        for i in 0..outline.len() {
                            let layer = match outline.get(i) {
                                Some(layer) => layer,
                                _ => {
                                    ok = false;
                                    break;
                                }
                            };

                            scratch.clear();
                            let placement = Mask::with_scratch(layer.path(), rcx)
                                .origin(ZenoOrigin::BottomLeft)
                                .style(self.style)
                                .offset(self.offset)
                                .render_offset(self.offset)
                                .inspect(|fmt, w, h| {
                                    scratch.resize(fmt.buffer_size(w, h), 0);
                                })
                                .render_into(&mut scratch[..], None);
                            let color = layer
                                .color_index()
                                .and_then(|i| palette.map(|p| p.get(i)))
                                .unwrap_or(self.foreground);
                            bitmap::blit(
                                &scratch[..],
                                placement.width,
                                placement.height,
                                placement.left.wrapping_sub(base_x),
                                (base_h as i32 + base_y).wrapping_sub(placement.top),
                                color,
                                &mut image.data,
                                base_w,
                                base_h,
                            );
                        }
                        if ok {
                            image.source = Source::ColorOutline(*palette_index);
                            image.content = Content::Color;
                            return true;
                        }
                    }
                }
                Source::Bitmap(mode) => {
                    if scaler.scale_bitmap_impl(glyph_id, false, *mode, image) == Some(true) {
                        return true;
                    }
                }
                Source::ColorBitmap(mode) => {
                    if scaler.scale_bitmap_impl(glyph_id, true, *mode, image) == Some(true) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Renders the specified glyph using the current configuration.
    pub fn render(&self, scaler: &mut Scaler, glyph_id: GlyphId) -> Option<Image> {
        let mut image = Image::new();
        if self.render_into(scaler, glyph_id, &mut image) {
            Some(image)
        } else {
            None
        }
    }
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

#[cfg(test)]
mod tests {
    use super::decode_bitmap_mask;
    use skrifa::bitmap::MaskData;

    #[test]
    fn decode_bitmap_mask_1bpp_unpacked() {
        let mask = MaskData {
            bpp: 1,
            is_packed: false,
            data: &[0b1010_0101],
        };
        let mut out = [0u8; 8];
        assert!(decode_bitmap_mask(&mask, 8, 1, &mut out));
        assert_eq!(out, [255, 0, 255, 0, 0, 255, 0, 255]);
    }

    #[test]
    fn decode_bitmap_mask_2bpp_unpacked() {
        let mask = MaskData {
            bpp: 2,
            is_packed: false,
            data: &[0b0001_1011],
        };
        let mut out = [0u8; 4];
        assert!(decode_bitmap_mask(&mask, 4, 1, &mut out));
        assert_eq!(out, [0, 85, 170, 255]);
    }

    #[test]
    fn decode_bitmap_mask_packed_and_unpacked_match() {
        let mut packed_out = [0u8; 6];
        let packed = MaskData {
            bpp: 1,
            is_packed: true,
            data: &[0b1010_1100],
        };
        assert!(decode_bitmap_mask(&packed, 3, 2, &mut packed_out));

        let mut unpacked_out = [0u8; 6];
        let unpacked = MaskData {
            bpp: 1,
            is_packed: false,
            data: &[0b1010_0000, 0b0110_0000],
        };
        assert!(decode_bitmap_mask(&unpacked, 3, 2, &mut unpacked_out));

        assert_eq!(packed_out, [255, 0, 255, 0, 255, 255]);
        assert_eq!(unpacked_out, packed_out);
    }
}
