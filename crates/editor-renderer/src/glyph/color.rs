use skrifa::color::Transform;
use skrifa::color::{Brush, ColorPainter, CompositeMode, PaintCachedColorGlyph, PaintError};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::DrawSettings;
use skrifa::raw::TableProvider;
use skrifa::raw::types::BoundingBox;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use zeno::{Command, Format, Mask, Placement, Point, Scratch, Verb};

use super::outline::Outline;
use super::outline_pen::OutlineWriter;
use crate::types::Image;

pub fn rasterize_color_outline(font_data: &[u8], glyph_id: u32, font_size: f32) -> Option<Image> {
    let font = FontRef::from_index(font_data, 0).ok()?;
    let color_glyphs = font.color_glyphs();
    let gid = GlyphId::new(glyph_id);
    let color_glyph = color_glyphs.get(gid)?;

    let palettes = font.color_palettes();
    let palette = palettes.get(0)?;
    let colors: Vec<[u8; 4]> = palette
        .colors()
        .iter()
        .map(|c| [c.red, c.green, c.blue, c.alpha])
        .collect();

    let upem = font.head().ok()?.units_per_em() as f32;
    let scale = font_size / upem;

    let mut painter = ColorGlyphRasterizer::new(font_data, font_size, scale, &colors);
    color_glyph
        .paint(LocationRef::default(), &mut painter)
        .ok()?;

    painter.finish()
}

struct ColorGlyphRasterizer<'a> {
    font_data: &'a [u8],
    font_size: f32,
    scale: f32,
    colors: &'a [[u8; 4]],

    pixels: Vec<u8>,
    width: u32,
    height: u32,
    offset_x: f32,
    offset_y: f32,

    clip_mask: Option<ClipMask>,
    clip_stack: Vec<Option<ClipMask>>,

    layer_stack: Vec<LayerState>,

    transform_stack: Vec<Transform>,
    current_transform: Transform,

    scratch: Scratch,
}

struct ClipMask {
    data: Vec<u8>,
    placement: Placement,
}

struct LayerState {
    pixels: Vec<u8>,
    composite_mode: CompositeMode,
}

impl<'a> ColorGlyphRasterizer<'a> {
    fn new(font_data: &'a [u8], font_size: f32, scale: f32, colors: &'a [[u8; 4]]) -> Self {
        let est_size = (font_size * 1.5).ceil() as u32;
        let w = est_size.max(1);
        let h = est_size.max(1);

        Self {
            font_data,
            font_size,
            scale,
            colors,
            pixels: vec![0u8; (w * h * 4) as usize],
            width: w,
            height: h,
            offset_x: 0.0,
            offset_y: 0.0,
            clip_mask: None,
            clip_stack: Vec::new(),
            layer_stack: Vec::new(),
            transform_stack: Vec::new(),
            current_transform: Transform::default(),
            scratch: Scratch::new(),
        }
    }

    fn finish(self) -> Option<Image> {
        if self.width == 0 || self.height == 0 {
            return None;
        }

        Some(Image {
            data: self.pixels,
            width: self.width,
            height: self.height,
        })
    }

    fn rasterize_glyph_to_mask(&mut self, glyph_id: GlyphId) -> Option<(Vec<u8>, Placement)> {
        let font = FontRef::from_index(self.font_data, 0).ok()?;
        let outlines = font.outline_glyphs();
        let outline_glyph = outlines.get(glyph_id)?;

        let settings = DrawSettings::unhinted(Size::new(self.font_size), LocationRef::default());
        let mut outline = Outline::new();
        let mut writer = OutlineWriter(&mut outline);
        outline_glyph.draw(settings, &mut writer).ok()?;

        if outline.is_empty() {
            return None;
        }

        let commands = outline_to_zeno_commands(&outline);

        let mut mask_buf = Vec::new();
        let placement = Mask::with_scratch(&commands[..], &mut self.scratch)
            .format(Format::Alpha)
            .inspect(|fmt, w, h| {
                mask_buf.resize(fmt.buffer_size(w, h), 0);
            })
            .render_into(&mut mask_buf, None);

        Some((mask_buf, placement))
    }

    fn fill_solid(&mut self, r: u8, g: u8, b: u8, a: u8) {
        let clip = match &self.clip_mask {
            Some(c) => c,
            None => return,
        };

        let clip_left = clip.placement.left;
        let clip_top = clip.placement.top;
        let clip_w = clip.placement.width;
        let clip_h = clip.placement.height;

        let dst_x0 = clip_left - self.offset_x as i32;
        let dst_y0 = clip_top - self.offset_y as i32;

        for row in 0..clip_h as i32 {
            for col in 0..clip_w as i32 {
                let mask_idx = (row as u32 * clip_w + col as u32) as usize;
                let mask_alpha = match clip.data.get(mask_idx) {
                    Some(&v) => v,
                    None => continue,
                };

                if mask_alpha == 0 {
                    continue;
                }

                let px = dst_x0 + col;
                let py = dst_y0 + row;
                if px < 0 || py < 0 || px >= self.width as i32 || py >= self.height as i32 {
                    continue;
                }

                let sa = (a as u16 * mask_alpha as u16 / 255) as u8;
                let sr = (r as u16 * sa as u16 / 255) as u8;
                let sg = (g as u16 * sa as u16 / 255) as u8;
                let sb = (b as u16 * sa as u16 / 255) as u8;

                let off = ((py as u32 * self.width + px as u32) * 4) as usize;
                blend_src_over(&mut self.pixels[off..off + 4], sr, sg, sb, sa);
            }
        }
    }

    fn resolve_color(&self, palette_index: u16, alpha: f32) -> [u8; 4] {
        let base = self
            .colors
            .get(palette_index as usize)
            .copied()
            .unwrap_or([0, 0, 0, 255]);
        let a = (base[3] as f32 * alpha).round().clamp(0.0, 255.0) as u8;
        [base[0], base[1], base[2], a]
    }

    fn resize_buffer(&mut self, width: u32, height: u32, offset_x: f32, offset_y: f32) {
        self.width = width;
        self.height = height;
        self.offset_x = offset_x;
        self.offset_y = offset_y;
        self.pixels = vec![0u8; (width * height * 4) as usize];
    }
}

impl ColorPainter for ColorGlyphRasterizer<'_> {
    fn push_transform(&mut self, transform: Transform) {
        self.transform_stack.push(self.current_transform);
        self.current_transform = self.current_transform * transform;
    }

    fn pop_transform(&mut self) {
        if let Some(prev) = self.transform_stack.pop() {
            self.current_transform = prev;
        }
    }

    fn push_clip_glyph(&mut self, glyph_id: GlyphId) {
        self.clip_stack.push(self.clip_mask.take());
        self.clip_mask = self
            .rasterize_glyph_to_mask(glyph_id)
            .map(|(data, placement)| ClipMask { data, placement });
    }

    fn push_clip_box(&mut self, clip_box: BoundingBox<f32>) {
        self.clip_stack.push(self.clip_mask.take());

        let x_min = (clip_box.x_min * self.scale).floor();
        let y_min = (clip_box.y_min * self.scale).floor();
        let x_max = (clip_box.x_max * self.scale).ceil();
        let y_max = (clip_box.y_max * self.scale).ceil();

        let w = (x_max - x_min).max(0.0) as u32;
        let h = (y_max - y_min).max(0.0) as u32;

        if w > 0 && h > 0 {
            self.resize_buffer(w, h, x_min, y_min);

            self.clip_mask = Some(ClipMask {
                data: vec![255u8; (w * h) as usize],
                placement: Placement {
                    left: x_min as i32,
                    top: y_min as i32,
                    width: w,
                    height: h,
                },
            });
        }
    }

    fn pop_clip(&mut self) {
        self.clip_mask = self.clip_stack.pop().flatten();
    }

    fn fill(&mut self, brush: Brush<'_>) {
        match brush {
            Brush::Solid {
                palette_index,
                alpha,
            } => {
                let [r, g, b, a] = self.resolve_color(palette_index, alpha);
                self.fill_solid(r, g, b, a);
            }
            Brush::LinearGradient { .. }
            | Brush::RadialGradient { .. }
            | Brush::SweepGradient { .. } => {}
        }
    }

    fn push_layer(&mut self, composite_mode: CompositeMode) {
        let saved = std::mem::replace(
            &mut self.pixels,
            vec![0u8; (self.width * self.height * 4) as usize],
        );

        self.layer_stack.push(LayerState {
            pixels: saved,
            composite_mode,
        });
    }

    fn pop_layer(&mut self) {
        if let Some(layer) = self.layer_stack.pop() {
            let src = std::mem::replace(&mut self.pixels, layer.pixels);
            match layer.composite_mode {
                CompositeMode::SrcOver => {
                    for i in (0..self.pixels.len()).step_by(4) {
                        if i + 3 < src.len() {
                            blend_src_over(
                                &mut self.pixels[i..i + 4],
                                src[i],
                                src[i + 1],
                                src[i + 2],
                                src[i + 3],
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn pop_layer_with_mode(&mut self, _composite_mode: CompositeMode) {
        self.pop_layer();
    }

    fn paint_cached_color_glyph(
        &mut self,
        _glyph: GlyphId,
    ) -> Result<PaintCachedColorGlyph, PaintError> {
        Ok(PaintCachedColorGlyph::Unimplemented)
    }
}

fn outline_to_zeno_commands(outline: &Outline) -> Vec<Command> {
    let points = outline.points();
    let verbs = outline.verbs();
    let mut commands = Vec::new();
    let mut point_idx = 0usize;

    for verb in verbs {
        match verb {
            Verb::MoveTo => {
                let p = points[point_idx];
                point_idx += 1;
                commands.push(Command::MoveTo(Point::new(p.x, p.y)));
            }
            Verb::LineTo => {
                let p = points[point_idx];
                point_idx += 1;
                commands.push(Command::LineTo(Point::new(p.x, p.y)));
            }
            Verb::QuadTo => {
                let ctrl = points[point_idx];
                let p = points[point_idx + 1];
                point_idx += 2;
                commands.push(Command::QuadTo(
                    Point::new(ctrl.x, ctrl.y),
                    Point::new(p.x, p.y),
                ));
            }
            Verb::CurveTo => {
                let c1 = points[point_idx];
                let c2 = points[point_idx + 1];
                let p = points[point_idx + 2];
                point_idx += 3;
                commands.push(Command::CurveTo(
                    Point::new(c1.x, c1.y),
                    Point::new(c2.x, c2.y),
                    Point::new(p.x, p.y),
                ));
            }
            Verb::Close => {
                commands.push(Command::Close);
            }
        }
    }

    commands
}

fn blend_src_over(dst: &mut [u8], src_r: u8, src_g: u8, src_b: u8, src_a: u8) {
    if src_a == 0 {
        return;
    }

    if src_a == 255 {
        dst[0] = src_r;
        dst[1] = src_g;
        dst[2] = src_b;
        dst[3] = 255;
        return;
    }

    let inv_a = 255 - src_a as u16;
    dst[0] = (src_r as u16 + dst[0] as u16 * inv_a / 255) as u8;
    dst[1] = (src_g as u16 + dst[1] as u16 * inv_a / 255) as u8;
    dst[2] = (src_b as u16 + dst[2] as u16 * inv_a / 255) as u8;
    dst[3] = (src_a as u16 + dst[3] as u16 * inv_a / 255) as u8;
}
