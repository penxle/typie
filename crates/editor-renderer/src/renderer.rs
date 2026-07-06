use editor_common::{Rect, Underline, UnderlineStyle};
use editor_model::{DocView, Modifier, ModifierType, Node, TableBorderStyle};
use editor_resource::{Resource, Theme};
use editor_view::style::DecorationData;
use editor_view::{
    Edges, LineMetrics, PageFragmentAtom, PageFragmentBox, PageFragmentDecoration,
    PageFragmentLine, PageFragmentNode, PageRect, PageVisitor,
};
use std::sync::{Arc, Mutex};

use crate::glyph::{
    BakedGlyphCache, Content, GlyphCache, GlyphKey, PositionedGlyph, PositionedSvgPathGlyph,
    ScaleContext, SvgPathGlyphCache,
};
use crate::icons::ICONS;
use crate::sink::RenderSink;
use crate::types::{
    Color, CornerRadii, IconData, IconElement, Image, Path, PathElement, Stroke, StrokeCap,
    StrokeJoin, Transform,
};
use crate::vector::codec::encode_vector_page;
use crate::vector::export::VectorSink;

#[cfg(test)]
thread_local! {
    pub(crate) static BAKE_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn bake_mask_to_premul_rgba(
    mask: &[u8],
    width: u32,
    height: u32,
    color: Color,
    glyph: Option<GlyphKey>,
) -> Image {
    #[cfg(test)]
    BAKE_COUNT.with(|c| c.set(c.get() + 1));

    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for &m in mask {
        data.extend_from_slice(&crate::backend::cpu::raster::premul_pixel(m, color));
    }
    Image {
        data: data.into(),
        width,
        height,
        glyph,
    }
}

fn draw_positioned_raster_glyph(
    sink: &mut dyn RenderSink,
    baked_cache: &mut BakedGlyphCache,
    pg: &PositionedGlyph,
    color: Color,
    font_generation: u64,
) {
    let key = GlyphKey {
        cache_key: pg.cache_key,
        color,
        font_generation,
    };
    let image = baked_cache.get_or_bake(key, || match pg.raster.content {
        Content::Mask => bake_mask_to_premul_rgba(
            &pg.raster.data,
            pg.raster.width,
            pg.raster.height,
            color,
            Some(key),
        ),
        Content::Color => Image {
            data: pg.raster.data.clone(),
            width: pg.raster.width,
            height: pg.raster.height,
            glyph: Some(key),
        },
    });

    sink.draw_glyph(&image, pg.blit_x, pg.blit_y);
}

fn draw_positioned_svg_path_glyph(
    sink: &mut dyn RenderSink,
    pg: &PositionedSvgPathGlyph,
    color: Color,
) {
    let t = Transform::IDENTITY.translate(pg.blit_x as f32, pg.blit_y as f32);
    sink.fill_path(&pg.path.path, color, t);
}

fn callout_token(variant: editor_model::CalloutVariant) -> &'static str {
    match variant {
        editor_model::CalloutVariant::Info => "ui.callout.info",
        editor_model::CalloutVariant::Success => "ui.callout.success",
        editor_model::CalloutVariant::Warning => "ui.callout.warning",
        editor_model::CalloutVariant::Danger => "ui.callout.danger",
    }
}

fn message_bubble_radii(
    radius: f32,
    edges: &Edges<bool>,
    is_sent: bool,
    has_tail: bool,
) -> CornerRadii {
    let top = if edges.top { radius } else { 0.0 };
    let bottom = if edges.bottom { radius } else { 0.0 };
    let mut radii = CornerRadii {
        top_left: top,
        top_right: top,
        bottom_left: bottom,
        bottom_right: bottom,
    };
    if has_tail {
        if is_sent {
            radii.bottom_right = 0.0;
        } else {
            radii.bottom_left = 0.0;
        }
    }
    radii
}

fn build_message_tail(width: f32, height: f32, is_sent: bool) -> Path {
    let s = MESSAGE_TAIL_SIZE;
    let elements = if is_sent {
        vec![
            PathElement::MoveTo {
                x: width - s * 0.8,
                y: height,
            },
            PathElement::QuadTo {
                x1: width,
                y1: height,
                x: width,
                y: height - s * 0.5,
            },
            PathElement::QuadTo {
                x1: width,
                y1: height,
                x: width + s * 0.4,
                y: height + s * 0.15,
            },
            PathElement::QuadTo {
                x1: width - s * 0.2,
                y1: height + s * 0.05,
                x: width - s * 0.8,
                y: height,
            },
            PathElement::Close,
        ]
    } else {
        vec![
            PathElement::MoveTo {
                x: s * 0.8,
                y: height,
            },
            PathElement::QuadTo {
                x1: 0.0,
                y1: height,
                x: 0.0,
                y: height - s * 0.5,
            },
            PathElement::QuadTo {
                x1: 0.0,
                y1: height,
                x: -s * 0.4,
                y: height + s * 0.15,
            },
            PathElement::QuadTo {
                x1: s * 0.2,
                y1: height + s * 0.05,
                x: s * 0.8,
                y: height,
            },
            PathElement::Close,
        ]
    };
    Path { elements }
}

const CALLOUT_BORDER_RADIUS: f32 = 8.0;
const CALLOUT_BORDER_WIDTH: f32 = 1.0;
const FOLD_BORDER_RADIUS: f32 = 8.0;
const FOLD_BORDER_WIDTH: f32 = 1.0;
const ICON_STROKE_WIDTH: f32 = 1.5;
const HR_LINE_HEIGHT: f32 = 1.0;
const HR_SHAPE_SIZE_LARGE: f32 = 10.0;
const HR_SHAPE_SIZE_SMALL: f32 = 8.0;
const HR_SHAPE_GAP: f32 = 8.0;
const MESSAGE_BORDER_RADIUS: f32 = 18.0;
const MESSAGE_TAIL_SIZE: f32 = 10.0;
const BULLET_RADIUS_RATIO: f32 = 0.125;
const TEXT_DECORATION_THICKNESS: f32 = 1.0;

fn build_partial_border(r: Rect, radii: CornerRadii, edges: &Edges<bool>) -> Path {
    let CornerRadii {
        top_left: tl,
        top_right: tr,
        bottom_right: br,
        bottom_left: bl,
    } = radii;
    let mut elements = Vec::new();

    if !edges.top && !edges.bottom {
        elements.push(PathElement::MoveTo { x: r.x, y: r.y });
        elements.push(PathElement::LineTo {
            x: r.x,
            y: r.y + r.height,
        });
        elements.push(PathElement::MoveTo {
            x: r.x + r.width,
            y: r.y,
        });
        elements.push(PathElement::LineTo {
            x: r.x + r.width,
            y: r.y + r.height,
        });
    } else if !edges.top {
        elements.push(PathElement::MoveTo { x: r.x, y: r.y });
        elements.push(PathElement::LineTo {
            x: r.x,
            y: r.y + r.height - bl,
        });
        if bl > 0.0 {
            elements.push(PathElement::QuadTo {
                x1: r.x,
                y1: r.y + r.height,
                x: r.x + bl,
                y: r.y + r.height,
            });
        }
        elements.push(PathElement::LineTo {
            x: r.x + r.width - br,
            y: r.y + r.height,
        });
        if br > 0.0 {
            elements.push(PathElement::QuadTo {
                x1: r.x + r.width,
                y1: r.y + r.height,
                x: r.x + r.width,
                y: r.y + r.height - br,
            });
        }
        elements.push(PathElement::LineTo {
            x: r.x + r.width,
            y: r.y,
        });
    } else if !edges.bottom {
        elements.push(PathElement::MoveTo {
            x: r.x,
            y: r.y + r.height,
        });
        elements.push(PathElement::LineTo {
            x: r.x,
            y: r.y + tl,
        });
        if tl > 0.0 {
            elements.push(PathElement::QuadTo {
                x1: r.x,
                y1: r.y,
                x: r.x + tl,
                y: r.y,
            });
        }
        elements.push(PathElement::LineTo {
            x: r.x + r.width - tr,
            y: r.y,
        });
        if tr > 0.0 {
            elements.push(PathElement::QuadTo {
                x1: r.x + r.width,
                y1: r.y,
                x: r.x + r.width,
                y: r.y + tr,
            });
        }
        elements.push(PathElement::LineTo {
            x: r.x + r.width,
            y: r.y + r.height,
        });
    }

    Path { elements }
}

fn circle_path(cx: f32, cy: f32, r: f32) -> Path {
    const K: f32 = 0.552_284_8;
    let kx = r * K;
    let ky = r * K;
    Path {
        elements: vec![
            PathElement::MoveTo { x: cx + r, y: cy },
            PathElement::CurveTo {
                x1: cx + r,
                y1: cy + ky,
                x2: cx + kx,
                y2: cy + r,
                x: cx,
                y: cy + r,
            },
            PathElement::CurveTo {
                x1: cx - kx,
                y1: cy + r,
                x2: cx - r,
                y2: cy + ky,
                x: cx - r,
                y: cy,
            },
            PathElement::CurveTo {
                x1: cx - r,
                y1: cy - ky,
                x2: cx - kx,
                y2: cy - r,
                x: cx,
                y: cy - r,
            },
            PathElement::CurveTo {
                x1: cx + kx,
                y1: cy - r,
                x2: cx + r,
                y2: cy - ky,
                x: cx + r,
                y: cy,
            },
            PathElement::Close,
        ],
    }
}

fn diamond_path(cx: f32, cy: f32, r: f32) -> Path {
    Path {
        elements: vec![
            PathElement::MoveTo { x: cx, y: cy - r },
            PathElement::LineTo { x: cx + r, y: cy },
            PathElement::LineTo { x: cx, y: cy + r },
            PathElement::LineTo { x: cx - r, y: cy },
            PathElement::Close,
        ],
    }
}

fn underline_rect(m: LineMetrics, run_x: f32, run_width: f32) -> Rect {
    Rect::from_xywh(
        run_x,
        m.baseline + m.descent * 0.5,
        run_width,
        TEXT_DECORATION_THICKNESS,
    )
}

fn text_background_rect(
    line_rect: Rect,
    m: LineMetrics,
    ruby_extra_top: f32,
    run_x: f32,
    run_width: f32,
) -> Rect {
    let text_height = (m.ascent + m.descent - ruby_extra_top).max(0.0);
    let text_area = (line_rect.height - ruby_extra_top).max(0.0);
    let text_top = ruby_extra_top + (text_area - text_height).max(0.0) * 0.5;
    Rect::from_xywh(run_x, text_top, run_width, text_height)
}

fn strikethrough_rect(m: LineMetrics, run_x: f32, run_width: f32) -> Rect {
    Rect::from_xywh(
        run_x,
        m.baseline - m.ascent * 0.3,
        run_width,
        TEXT_DECORATION_THICKNESS,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RenderLayer {
    Background = 0,
    Content = 1,
    Border = 2,
}

#[derive(Clone, Copy)]
pub struct LayerSet(u32);

impl LayerSet {
    pub fn of(layers: &[RenderLayer]) -> Self {
        Self(layers.iter().fold(0u32, |s, l| s | (1 << *l as u32)))
    }

    pub fn contains(self, layer: RenderLayer) -> bool {
        self.0 & (1 << layer as u32) != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkLayer {
    BelowContent,
    AboveContent,
}

#[derive(Debug, Clone)]
pub enum MarkData {
    Selection {
        focused: bool,
    },
    Composition,
    DropIndicator,
    TrackedBackground {
        theme_key: String,
        border_radius: f32,
        vertical_inset: f32,
    },
    TrackedUnderline {
        underline: Underline,
    },
}

impl MarkData {
    pub fn layer(&self) -> MarkLayer {
        match self {
            Self::Selection { .. } | Self::TrackedBackground { .. } => MarkLayer::BelowContent,
            Self::Composition | Self::DropIndicator | Self::TrackedUnderline { .. } => {
                MarkLayer::AboveContent
            }
        }
    }
}

pub type MarkRect = PageRect<()>;

pub struct Mark {
    pub data: MarkData,
    pub rects: Vec<MarkRect>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextRenderMode {
    Raster,
    VectorExport,
}

const SELECTION_FOCUSED_ALPHA: u8 = 77;
const SELECTION_UNFOCUSED_ALPHA: u8 = 48;

fn selection_mark_color(theme: &Theme, focused: bool) -> Color {
    theme.color_with_alpha(
        "selection",
        if focused {
            SELECTION_FOCUSED_ALPHA
        } else {
            SELECTION_UNFOCUSED_ALPHA
        },
    )
}

const UNDERLINE_DASH: f32 = 6.0;
const UNDERLINE_GAP: f32 = 4.0;
const UNDERLINE_WAVE_PERIOD: f32 = 6.0;
const UNDERLINE_WAVE_AMPLITUDE: f32 = 1.5;

fn draw_underline(
    sink: &mut dyn RenderSink,
    rect: Rect,
    underline: &Underline,
    theme: &Theme,
    transform: Transform,
) {
    let thickness = underline.thickness.max(0.0);
    if thickness == 0.0 || rect.width <= 0.0 {
        return;
    }
    let color = theme.color(&underline.color);
    let y = rect.y + rect.height - thickness;
    let bar = Rect::from_xywh(rect.x, y, rect.width, thickness);
    match underline.style {
        UnderlineStyle::Solid => sink.fill_rect(bar, color, transform),
        UnderlineStyle::Dashed => {
            let period = UNDERLINE_DASH + UNDERLINE_GAP;
            let end_x = bar.x + bar.width;
            let mut x = bar.x;
            while x < end_x {
                let w = UNDERLINE_DASH.min(end_x - x);
                sink.fill_rect(Rect::from_xywh(x, bar.y, w, thickness), color, transform);
                x += period;
            }
        }
        UnderlineStyle::Wavy => {
            let amplitude = UNDERLINE_WAVE_AMPLITUDE;
            let period = UNDERLINE_WAVE_PERIOD;
            let mid_y = rect.y + rect.height - amplitude;
            let end_x = rect.x + rect.width;
            let mut elements = Vec::new();
            elements.push(PathElement::MoveTo {
                x: rect.x,
                y: mid_y,
            });
            let mut x = rect.x;
            let mut up = true;
            while x < end_x {
                let next_x = (x + period * 0.5).min(end_x);
                let cp_x = (x + next_x) * 0.5;
                let cp_y = if up {
                    mid_y - amplitude
                } else {
                    mid_y + amplitude
                };
                elements.push(PathElement::QuadTo {
                    x1: cp_x,
                    y1: cp_y,
                    x: next_x,
                    y: mid_y,
                });
                x = next_x;
                up = !up;
            }
            let path = Path { elements };
            let stroke = Stroke::new(thickness);
            sink.stroke_path(&path, color, &stroke, transform);
        }
    }
}

pub struct Renderer {
    pub(crate) resource: Arc<Mutex<Resource>>,
    pub(crate) scale_ctx: ScaleContext,
    pub(crate) glyph_cache: GlyphCache,
    pub(crate) svg_path_glyph_cache: SvgPathGlyphCache,
    pub(crate) baked_glyph_cache: BakedGlyphCache,
}

impl Renderer {
    pub fn new(resource: Arc<Mutex<Resource>>) -> Self {
        Self {
            resource,
            scale_ctx: ScaleContext::new(),
            glyph_cache: GlyphCache::new(),
            svg_path_glyph_cache: SvgPathGlyphCache::new(),
            baked_glyph_cache: BakedGlyphCache::new(),
        }
    }

    fn resolve_mark_color(&self, data: &MarkData, theme: &Theme) -> Option<Color> {
        match data {
            MarkData::Selection { focused } => Some(selection_mark_color(theme, *focused)),
            MarkData::Composition => Some(theme.color("ui.text.default")),
            MarkData::DropIndicator => Some(theme.color("selection")),
            MarkData::TrackedBackground { theme_key, .. } => Some(theme.color(theme_key)),
            MarkData::TrackedUnderline { .. } => None,
        }
    }

    fn draw_marks(
        &self,
        sink: &mut dyn RenderSink,
        marks: &[Mark],
        layer: MarkLayer,
        page_idx: usize,
        scale_factor: f32,
        theme: &Theme,
    ) {
        let transform = Transform::scale(scale_factor);
        for mark in marks {
            if mark.data.layer() != layer {
                continue;
            }
            for rect in &mark.rects {
                if rect.page_idx != page_idx {
                    continue;
                }
                match &mark.data {
                    MarkData::TrackedUnderline { underline } => {
                        draw_underline(sink, rect.rect, underline, theme, transform);
                    }
                    MarkData::TrackedBackground {
                        border_radius,
                        vertical_inset,
                        ..
                    } => {
                        if let Some(color) = self.resolve_mark_color(&mark.data, theme) {
                            let inset = vertical_inset.max(0.0).min(rect.rect.height * 0.5);
                            let r = Rect::from_xywh(
                                rect.rect.x,
                                rect.rect.y + inset,
                                rect.rect.width,
                                (rect.rect.height - inset * 2.0).max(0.0),
                            );
                            let radius = border_radius.max(0.0);
                            if radius > 0.0 && r.height > 0.0 && r.width > 0.0 {
                                let radius = radius.min(r.height * 0.5).min(r.width * 0.5);
                                let path = Path::rrect(
                                    r,
                                    CornerRadii {
                                        top_left: radius,
                                        top_right: radius,
                                        bottom_right: radius,
                                        bottom_left: radius,
                                    },
                                );
                                sink.fill_path(&path, color, transform);
                            } else {
                                sink.fill_rect(r, color, transform);
                            }
                        }
                    }
                    _ => {
                        if let Some(color) = self.resolve_mark_color(&mark.data, theme) {
                            sink.fill_rect(rect.rect, color, transform);
                        }
                    }
                }
            }
        }
    }

    pub fn render_page(
        &mut self,
        sink: &mut dyn RenderSink,
        doc: &DocView,
        view: &editor_view::View,
        page_idx: usize,
        scale_factor: f32,
        marks: &[Mark],
    ) {
        let theme = self.resource.lock().unwrap().theme;

        view.visit_page(
            page_idx,
            &mut self.page_visitor(
                sink,
                doc,
                scale_factor,
                LayerSet::of(&[RenderLayer::Background]),
            ),
        );

        self.draw_marks(
            sink,
            marks,
            MarkLayer::BelowContent,
            page_idx,
            scale_factor,
            &theme,
        );

        view.visit_page(
            page_idx,
            &mut self.page_visitor(
                sink,
                doc,
                scale_factor,
                LayerSet::of(&[RenderLayer::Content, RenderLayer::Border]),
            ),
        );

        self.draw_marks(
            sink,
            marks,
            MarkLayer::AboveContent,
            page_idx,
            scale_factor,
            &theme,
        );
    }

    pub fn export_page_vector(
        &mut self,
        doc: &DocView,
        view: &editor_view::View,
        page_idx: usize,
        scale_factor: f32,
    ) -> Vec<u8> {
        let (width, height) = view
            .pages()
            .get(page_idx)
            .map(|p| (p.size.width, p.size.height))
            .unwrap_or((0.0, 0.0));

        let mut sink = VectorSink::new();
        view.visit_page(
            page_idx,
            &mut self.vector_page_visitor(
                &mut sink,
                doc,
                scale_factor,
                LayerSet::of(&[RenderLayer::Background]),
            ),
        );
        view.visit_page(
            page_idx,
            &mut self.vector_page_visitor(
                &mut sink,
                doc,
                scale_factor,
                LayerSet::of(&[RenderLayer::Content, RenderLayer::Border]),
            ),
        );

        let page = sink.into_page(width, height);
        encode_vector_page(&page)
    }

    fn page_visitor<'a>(
        &'a mut self,
        sink: &'a mut dyn RenderSink,
        doc: &'a DocView<'a>,
        scale_factor: f32,
        active: LayerSet,
    ) -> RenderVisitor<'a> {
        self.make_page_visitor(sink, doc, scale_factor, active, TextRenderMode::Raster)
    }

    fn vector_page_visitor<'a>(
        &'a mut self,
        sink: &'a mut dyn RenderSink,
        doc: &'a DocView<'a>,
        scale_factor: f32,
        active: LayerSet,
    ) -> RenderVisitor<'a> {
        self.make_page_visitor(
            sink,
            doc,
            scale_factor,
            active,
            TextRenderMode::VectorExport,
        )
    }

    fn make_page_visitor<'a>(
        &'a mut self,
        sink: &'a mut dyn RenderSink,
        doc: &'a DocView<'a>,
        scale_factor: f32,
        active: LayerSet,
        text_mode: TextRenderMode,
    ) -> RenderVisitor<'a> {
        let theme = self.resource.lock().unwrap().theme;
        RenderVisitor {
            renderer: self,
            sink,
            doc,
            scale_factor,
            root_transform: Transform::scale(scale_factor),
            box_stack: Vec::new(),
            active,
            text_mode,
            theme,
        }
    }
}

struct BoxFrame {
    local_rect: Rect,
    border: editor_common::EdgeInsets,
    edges: Edges<bool>,
    node: Option<Node>,
}

pub struct RenderVisitor<'a> {
    renderer: &'a mut Renderer,
    sink: &'a mut dyn RenderSink,
    doc: &'a DocView<'a>,
    scale_factor: f32,
    root_transform: Transform,
    box_stack: Vec<BoxFrame>,
    active: LayerSet,
    text_mode: TextRenderMode,
    theme: Theme,
}

impl RenderVisitor<'_> {
    fn on(&self, layer: RenderLayer) -> bool {
        self.active.contains(layer)
    }
}

impl<'a> RenderVisitor<'a> {
    fn render_icon(
        &mut self,
        icon: &'static IconData,
        color: Color,
        rect: Rect,
        base_transform: Transform,
        stroke_width: f32,
    ) {
        let s = (rect.width / icon.viewport.0).min(rect.height / icon.viewport.1);
        let dx = (rect.width - icon.viewport.0 * s) / 2.0;
        let dy = (rect.height - icon.viewport.1 * s) / 2.0;
        let icon_t = base_transform.translate(dx, dy).post_scale(s);

        for elem in icon.elements {
            match *elem {
                IconElement::Fill { path, .. } => {
                    let p = Path {
                        elements: path.to_vec(),
                    };
                    self.sink.fill_path(&p, color, icon_t);
                }
                IconElement::Stroke {
                    path,
                    stroke_cap,
                    stroke_join,
                } => {
                    let p = Path {
                        elements: path.to_vec(),
                    };
                    let stroke = Stroke {
                        width: stroke_width / s,
                        cap: stroke_cap,
                        join: stroke_join,
                    };
                    self.sink.stroke_path(&p, color, &stroke, icon_t);
                }
            }
        }
    }

    fn render_glyph_runs(
        &mut self,
        glyph_runs: &[editor_view::glyph_run::GlyphRun],
        color: Color,
        base_transform: Transform,
    ) {
        for run in glyph_runs {
            if self.text_mode == TextRenderMode::VectorExport {
                let resource = Arc::clone(&self.renderer.resource);
                let fonts = resource.lock().unwrap();
                self.sink
                    .draw_glyph_run(run, color, base_transform, &fonts.font_registry);
                continue;
            }

            let resource = Arc::clone(&self.renderer.resource);
            let resource_guard = resource.lock().unwrap();
            let font_generation = resource_guard.font_registry.font_generation();
            let positioned = crate::glyph::rasterize(
                run,
                &resource_guard.font_registry,
                &mut self.renderer.scale_ctx,
                &mut self.renderer.glyph_cache,
                &mut self.renderer.svg_path_glyph_cache,
                self.scale_factor,
                base_transform,
            );
            drop(resource_guard);

            for pg in &positioned.rasters {
                draw_positioned_raster_glyph(
                    self.sink,
                    &mut self.renderer.baked_glyph_cache,
                    pg,
                    color,
                    font_generation,
                );
            }
            for pg in &positioned.svg_paths {
                draw_positioned_svg_path_glyph(self.sink, pg, color);
            }
        }
    }
    fn render_ruby_annotations(
        &mut self,
        rubies: &[editor_view::glyph_run::RubyAnnotation],
        base_transform: Transform,
    ) {
        use editor_view::glyph_run::GraphemeSpan;

        for ann in rubies {
            let color = self.theme.color(&ann.color);

            let adapter = editor_view::glyph_run::GlyphRun {
                family_id: ann.family_id,
                weight: ann.weight,
                font_size: ann.font_size,
                synthesis: ann.synthesis,
                color: ann.color.clone(),
                background_color: None,
                glyphs: ann.glyphs.clone(),
                decoration: editor_view::glyph_run::TextDecoration::default(),
                offset_range: 0..0,
                link: None,
                text: String::new(),
                x: ann.x,
                width: ann.width,
                graphemes: Vec::<GraphemeSpan>::new(),
                cursor_ascent: 0.0,
                cursor_descent: 0.0,
            };

            if self.text_mode == TextRenderMode::VectorExport {
                let resource = Arc::clone(&self.renderer.resource);
                let fonts = resource.lock().unwrap();
                self.sink
                    .draw_glyph_run(&adapter, color, base_transform, &fonts.font_registry);
                continue;
            }

            let resource = Arc::clone(&self.renderer.resource);
            let resource_guard = resource.lock().unwrap();
            let font_generation = resource_guard.font_registry.font_generation();
            let positioned = crate::glyph::rasterize(
                &adapter,
                &resource_guard.font_registry,
                &mut self.renderer.scale_ctx,
                &mut self.renderer.glyph_cache,
                &mut self.renderer.svg_path_glyph_cache,
                self.scale_factor,
                base_transform,
            );
            drop(resource_guard);

            for pg in &positioned.rasters {
                draw_positioned_raster_glyph(
                    self.sink,
                    &mut self.renderer.baked_glyph_cache,
                    pg,
                    color,
                    font_generation,
                );
            }
            for pg in &positioned.svg_paths {
                draw_positioned_svg_path_glyph(self.sink, pg, color);
            }
        }
    }
}

const TABLE_BORDER_WIDTH: f32 = 1.0;

fn draw_table_grid(
    sink: &mut dyn RenderSink,
    table_rect: Rect,
    table_box: &PageFragmentBox,
    border_style: TableBorderStyle,
    color: Color,
    t: Transform,
) {
    if border_style == TableBorderStyle::None {
        return;
    }

    let bw = TABLE_BORDER_WIDTH;
    let mut x_positions = Vec::new();
    let mut y_positions = Vec::new();
    let mut x_start = f32::INFINITY;
    let mut x_end = f32::NEG_INFINITY;
    let mut y_start = f32::INFINITY;
    let mut y_end = f32::NEG_INFINITY;

    for row_node in &table_box.children {
        let Some(row_box) = row_node.as_box() else {
            continue;
        };

        let mut last_cell_right = None;
        for cell_node in &row_box.children {
            if cell_node.as_box().is_none() {
                continue;
            }
            let cell_left = cell_node.rect.x - table_rect.x;
            let cell_right = cell_node.rect.right() - table_rect.x;
            last_cell_right = Some(cell_right);
            x_positions.push(cell_left);
            x_start = x_start.min(cell_left);
            x_end = x_end.max(cell_right);
        }

        if let Some(last_cell_right) = last_cell_right {
            let row_top = row_node.rect.y - table_rect.y;
            let row_bottom = row_node.rect.bottom() - table_rect.y;
            x_positions.push(last_cell_right - bw);
            y_start = y_start.min(row_top);
            y_end = y_end.max(row_bottom);
            if row_box.edges.top {
                y_positions.push(row_top);
            }
            if row_box.edges.bottom {
                y_positions.push(row_bottom - bw);
            }
        }
    }

    if x_positions.is_empty()
        || !x_start.is_finite()
        || !x_end.is_finite()
        || !y_start.is_finite()
        || !y_end.is_finite()
        || x_end <= x_start
        || y_end <= y_start
    {
        return;
    }

    x_positions.sort_by(|a, b| a.partial_cmp(b).unwrap());
    x_positions.dedup_by(|a, b| (*a - *b).abs() <= 0.01);
    y_positions.sort_by(|a, b| a.partial_cmp(b).unwrap());
    y_positions.dedup_by(|a, b| (*a - *b).abs() <= 0.01);

    let mut draw_segment = |rect: Rect, vertical: bool| {
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return;
        }

        match border_style {
            TableBorderStyle::Solid => sink.fill_path(&Path::rect(rect), color, t),
            TableBorderStyle::Dashed | TableBorderStyle::Dotted => {
                let (dash, gap) = match border_style {
                    TableBorderStyle::Dashed => (6.0_f32, 4.0_f32),
                    TableBorderStyle::Dotted => (2.0_f32, 2.0_f32),
                    TableBorderStyle::Solid | TableBorderStyle::None => unreachable!(),
                };
                let period = dash + gap;
                if vertical {
                    let mut y = 0.0_f32;
                    while y < rect.height {
                        let h = dash.min(rect.height - y);
                        sink.fill_path(
                            &Path::rect(Rect::from_xywh(rect.x, rect.y + y, rect.width, h)),
                            color,
                            t,
                        );
                        y += period;
                    }
                } else {
                    let mut x = 0.0_f32;
                    while x < rect.width {
                        let w = dash.min(rect.width - x);
                        sink.fill_path(
                            &Path::rect(Rect::from_xywh(rect.x + x, rect.y, w, rect.height)),
                            color,
                            t,
                        );
                        x += period;
                    }
                }
            }
            TableBorderStyle::None => {}
        }
    };

    for x in x_positions {
        draw_segment(Rect::from_xywh(x, y_start, bw, y_end - y_start), true);
    }
    for y in y_positions {
        draw_segment(Rect::from_xywh(x_start, y, x_end - x_start, bw), false);
    }
}

impl<'a> PageVisitor for RenderVisitor<'a> {
    fn box_enter(&mut self, node: &PageFragmentNode, fragment: &PageFragmentBox) {
        let node_id = fragment.node;
        let local_rect = node.rect;
        let style = &fragment.style;
        let edges = fragment.edges;
        let node = self.doc.node(node_id).map(|n| n.node());

        if self.on(RenderLayer::Background) {
            let t = self.root_transform.translate(local_rect.x, local_rect.y);
            let inner_rect = Rect::from_xywh(0.0, 0.0, local_rect.width, local_rect.height);

            match &node {
                Some(Node::Callout(callout)) => {
                    let token = callout_token(*callout.variant.get());
                    let color = self.theme.color_with_alpha(token, 8);
                    let radii = CornerRadii::from_edges(CALLOUT_BORDER_RADIUS, &edges);
                    let path = Path::rrect(inner_rect, radii);
                    self.sink.fill_path(&path, color, t);
                }
                Some(Node::Blockquote(bq))
                    if matches!(
                        *bq.variant.get(),
                        editor_model::BlockquoteVariant::MessageSent
                            | editor_model::BlockquoteVariant::MessageReceived
                    ) =>
                {
                    let is_sent = matches!(
                        *bq.variant.get(),
                        editor_model::BlockquoteVariant::MessageSent
                    );
                    // Edges<bool>.bottom = true means "box bottom is inside the page = not clipped = last fragment",
                    // the opposite of legacy SplitEdges.bottom, so no negation is needed here.
                    let has_tail = edges.bottom;
                    let token = if is_sent {
                        "ui.blockquote.message-sent"
                    } else {
                        "ui.blockquote.message-received"
                    };
                    let color = self.theme.color(token);

                    let radii =
                        message_bubble_radii(MESSAGE_BORDER_RADIUS, &edges, is_sent, has_tail);
                    let path = Path::rrect(inner_rect, radii);
                    self.sink.fill_path(&path, color, t);

                    if has_tail {
                        let tail = build_message_tail(inner_rect.width, inner_rect.height, is_sent);
                        self.sink.fill_path(&tail, color, t);
                    }
                }
                Some(Node::FoldTitle(_)) => {
                    let expanded = style
                        .decorations
                        .iter()
                        .find_map(|d| match d.data {
                            DecorationData::Bool(b) => Some(b),
                            _ => None,
                        })
                        .unwrap_or(true);
                    let inner_radius = (FOLD_BORDER_RADIUS - FOLD_BORDER_WIDTH).max(0.0);
                    let top = if edges.top { inner_radius } else { 0.0 };
                    let bottom = if !expanded && edges.bottom {
                        inner_radius
                    } else {
                        0.0
                    };
                    let radii = CornerRadii {
                        top_left: top,
                        top_right: top,
                        bottom_left: bottom,
                        bottom_right: bottom,
                    };
                    let color = self.theme.color("ui.surface.muted");
                    let path = Path::rrect(inner_rect, radii);
                    self.sink.fill_path(&path, color, t);
                }
                Some(Node::TableCell(_)) => {
                    let color_value = self.doc.node(node_id).and_then(|n| {
                        match n.block_modifier(ModifierType::BackgroundColor) {
                            Some(Modifier::BackgroundColor { value }) => Some(value.clone()),
                            _ => None,
                        }
                    });
                    if let Some(ref color_value) = color_value
                        && color_value != "none"
                    {
                        let color = self.theme.color(&format!("bg.{color_value}"));
                        let path = Path::rect(inner_rect);
                        self.sink.fill_path(&path, color, t);
                    }
                }
                _ => {}
            }
        }

        self.box_stack.push(BoxFrame {
            local_rect,
            border: style.border,
            edges,
            node,
        });
    }

    fn box_exit(&mut self, node: &PageFragmentNode, fragment: &PageFragmentBox) {
        let Some(frame) = self.box_stack.pop() else {
            return;
        };

        if !self.on(RenderLayer::Border) {
            return;
        }

        let t = self
            .root_transform
            .translate(frame.local_rect.x, frame.local_rect.y);

        if let Some(Node::Fold(_)) = &frame.node {
            let border_color = self.theme.color("ui.border.default");
            let stroke = Stroke::new(FOLD_BORDER_WIDTH);
            let mb = FOLD_BORDER_WIDTH / 2.0;
            let inner_radius = (FOLD_BORDER_RADIUS - mb).max(0.0);
            let radii = CornerRadii::from_edges(inner_radius, &frame.edges);

            let stroke_rect = Rect::from_xywh(
                mb,
                mb,
                frame.local_rect.width - FOLD_BORDER_WIDTH,
                frame.local_rect.height - FOLD_BORDER_WIDTH,
            );

            if frame.edges.top && frame.edges.bottom {
                let path = Path::rrect(stroke_rect, radii);
                self.sink.stroke_path(&path, border_color, &stroke, t);
            } else {
                let path = build_partial_border(stroke_rect, radii, &frame.edges);
                self.sink.stroke_path(&path, border_color, &stroke, t);
            }

            return;
        }

        if let Some(Node::Callout(callout)) = &frame.node {
            let token = callout_token(*callout.variant.get());
            let border_color = self.theme.color(token);
            let stroke = Stroke::new(CALLOUT_BORDER_WIDTH);
            let mb = CALLOUT_BORDER_WIDTH / 2.0;
            let inner_radius = (CALLOUT_BORDER_RADIUS - mb).max(0.0);
            let radii = CornerRadii::from_edges(inner_radius, &frame.edges);

            let stroke_rect = Rect::from_xywh(
                mb,
                mb,
                frame.local_rect.width - CALLOUT_BORDER_WIDTH,
                frame.local_rect.height - CALLOUT_BORDER_WIDTH,
            );

            if frame.edges.top && frame.edges.bottom {
                let path = Path::rrect(stroke_rect, radii);
                self.sink.stroke_path(&path, border_color, &stroke, t);
            } else {
                let path = build_partial_border(stroke_rect, radii, &frame.edges);
                self.sink.stroke_path(&path, border_color, &stroke, t);
            }

            return;
        }

        if let Some(Node::Table(table)) = &frame.node {
            let border_style = *table.border_style.get();
            if border_style != TableBorderStyle::None {
                let border_color = self.theme.color("ui.border.default");
                draw_table_grid(
                    self.sink,
                    node.rect,
                    fragment,
                    border_style,
                    border_color,
                    t,
                );
            }
            return;
        }

        if matches!(&frame.node, Some(Node::TableRow(_) | Node::TableCell(_))) {
            return;
        }

        let b = &frame.border;
        let border_color = match &frame.node {
            Some(Node::Blockquote(_) | Node::FoldTitle(_)) => self.theme.color("ui.border.default"),
            _ => self.theme.color("ui.border"),
        };

        if frame.edges.left && b.left > 0.0 {
            self.sink.fill_path(
                &Path::rect(Rect::from_xywh(0.0, 0.0, b.left, frame.local_rect.height)),
                border_color,
                t,
            );
        }
        if frame.edges.right && b.right > 0.0 {
            self.sink.fill_path(
                &Path::rect(Rect::from_xywh(
                    frame.local_rect.width - b.right,
                    0.0,
                    b.right,
                    frame.local_rect.height,
                )),
                border_color,
                t,
            );
        }
        if frame.edges.top && b.top > 0.0 {
            self.sink.fill_path(
                &Path::rect(Rect::from_xywh(0.0, 0.0, frame.local_rect.width, b.top)),
                border_color,
                t,
            );
        }
        if frame.edges.bottom && b.bottom > 0.0 {
            self.sink.fill_path(
                &Path::rect(Rect::from_xywh(
                    0.0,
                    frame.local_rect.height - b.bottom,
                    frame.local_rect.width,
                    b.bottom,
                )),
                border_color,
                t,
            );
        }
    }

    fn line(&mut self, node: &PageFragmentNode, fragment: &PageFragmentLine) {
        let local_rect = node.rect;
        let metrics = LineMetrics {
            baseline: fragment.baseline,
            ascent: fragment.ascent,
            descent: fragment.descent,
        };
        let glyph_runs = &fragment.glyph_runs;
        let ruby_annotations = &fragment.ruby_annotations;
        let t = self.root_transform.translate(local_rect.x, local_rect.y);

        if self.on(RenderLayer::Background) {
            let ruby_extra_top =
                editor_view::ruby_extra_top(metrics.baseline, metrics.ascent, ruby_annotations);
            for run in glyph_runs {
                if let Some(ref bg_token) = run.background_color {
                    let bg_color = self.theme.color(bg_token);
                    let run_rect =
                        text_background_rect(local_rect, metrics, ruby_extra_top, run.x, run.width);
                    self.sink.fill_rect(run_rect, bg_color, t);
                }
            }
        }

        if !self.on(RenderLayer::Content) {
            return;
        }

        for run in glyph_runs {
            let color = self.theme.color(&run.color);
            self.render_glyph_runs(std::slice::from_ref(run), color, t);
        }

        for run in glyph_runs {
            if !run.decoration.underline && !run.decoration.strikethrough {
                continue;
            }
            let color = self.theme.color(&run.color);
            if run.decoration.underline {
                self.sink
                    .fill_rect(underline_rect(metrics, run.x, run.width), color, t);
            }
            if run.decoration.strikethrough {
                self.sink
                    .fill_rect(strikethrough_rect(metrics, run.x, run.width), color, t);
            }
        }

        self.render_ruby_annotations(ruby_annotations, t);
    }

    fn atom(&mut self, node: &PageFragmentNode, fragment: &PageFragmentAtom) {
        if !self.on(RenderLayer::Content) {
            return;
        }

        let node_id = fragment.node;
        let local_rect = node.rect;
        let t = self.root_transform.translate(local_rect.x, local_rect.y);
        let inner_rect = Rect::from_xywh(0.0, 0.0, local_rect.width, local_rect.height);

        let node = self.doc.leaf(node_id).and_then(|l| l.node());

        match node {
            Some(Node::HorizontalRule(hr)) => {
                let color = self.theme.color("ui.text.default");
                let w = inner_rect.width;
                let h = inner_rect.height;
                let cx = w / 2.0;
                let cy = h / 2.0;

                match *hr.variant.get() {
                    editor_model::HorizontalRuleVariant::Line => {
                        let y = (h - HR_LINE_HEIGHT) / 2.0;
                        self.sink
                            .fill_rect(Rect::from_xywh(0.0, y, w, HR_LINE_HEIGHT), color, t);
                    }
                    editor_model::HorizontalRuleVariant::DashedLine => {
                        let y = cy - HR_LINE_HEIGHT / 2.0;
                        let segment_width: f32 = 16.0;
                        let dash_width: f32 = segment_width * 0.5;
                        let mut x = 0.0_f32;
                        while x < w {
                            let dw = dash_width.min(w - x);
                            self.sink.fill_rect(
                                Rect::from_xywh(x, y, dw, HR_LINE_HEIGHT),
                                color,
                                t,
                            );
                            x += segment_width;
                        }
                    }
                    editor_model::HorizontalRuleVariant::Circle => {
                        let path = circle_path(cx, cy, HR_SHAPE_SIZE_LARGE / 2.0);
                        self.sink.fill_path(&path, color, t);
                    }
                    editor_model::HorizontalRuleVariant::Diamond => {
                        let path = diamond_path(cx, cy, HR_SHAPE_SIZE_LARGE / 2.0);
                        let stroke = Stroke::new(1.0);
                        self.sink.stroke_path(&path, color, &stroke, t);
                    }
                    editor_model::HorizontalRuleVariant::ThreeCircles => {
                        let r = HR_SHAPE_SIZE_SMALL / 2.0;
                        let gap = HR_SHAPE_GAP + HR_SHAPE_SIZE_SMALL;
                        for offset in [-gap, 0.0, gap] {
                            let path = circle_path(cx + offset, cy, r);
                            self.sink.fill_path(&path, color, t);
                        }
                    }
                    editor_model::HorizontalRuleVariant::ThreeDiamonds => {
                        let r = HR_SHAPE_SIZE_SMALL / 2.0;
                        let gap = HR_SHAPE_GAP + HR_SHAPE_SIZE_SMALL;
                        let stroke = Stroke::new(1.0);
                        for offset in [-gap, 0.0, gap] {
                            let path = diamond_path(cx + offset, cy, r);
                            self.sink.stroke_path(&path, color, &stroke, t);
                        }
                    }
                    editor_model::HorizontalRuleVariant::CircleLine => {
                        let shape_half = (HR_SHAPE_SIZE_LARGE / 2.0) + 10.0;
                        let line_y = cy - HR_LINE_HEIGHT / 2.0;
                        let container_half = w / 4.0;
                        let line_width = container_half - shape_half;
                        self.sink.fill_rect(
                            Rect::from_xywh(
                                cx - container_half,
                                line_y,
                                line_width,
                                HR_LINE_HEIGHT,
                            ),
                            color,
                            t,
                        );
                        self.sink.fill_rect(
                            Rect::from_xywh(cx + shape_half, line_y, line_width, HR_LINE_HEIGHT),
                            color,
                            t,
                        );
                        let path = circle_path(cx, cy, HR_SHAPE_SIZE_LARGE / 2.0);
                        self.sink.fill_path(&path, color, t);
                    }
                    editor_model::HorizontalRuleVariant::DiamondLine => {
                        let shape_half = (HR_SHAPE_SIZE_LARGE / 2.0) + 10.0;
                        let line_y = cy - HR_LINE_HEIGHT / 2.0;
                        let container_half = w / 4.0;
                        let line_width = container_half - shape_half;
                        self.sink.fill_rect(
                            Rect::from_xywh(
                                cx - container_half,
                                line_y,
                                line_width,
                                HR_LINE_HEIGHT,
                            ),
                            color,
                            t,
                        );
                        self.sink.fill_rect(
                            Rect::from_xywh(cx + shape_half, line_y, line_width, HR_LINE_HEIGHT),
                            color,
                            t,
                        );
                        let path = diamond_path(cx, cy, HR_SHAPE_SIZE_LARGE / 2.0);
                        let stroke = Stroke::new(1.0);
                        self.sink.stroke_path(&path, color, &stroke, t);
                    }
                    editor_model::HorizontalRuleVariant::Zigzag => {
                        const POINTS: usize = 8;
                        const SEGMENT_WIDTH: f32 = 8.0;
                        const AMPLITUDE: f32 = 4.0;
                        let total_width = (POINTS - 1) as f32 * SEGMENT_WIDTH;
                        let start_x = cx - total_width / 2.0;
                        let mut elements = Vec::new();
                        for i in 0..POINTS {
                            let px = start_x + i as f32 * SEGMENT_WIDTH;
                            let py = if i % 2 == 0 {
                                cy + AMPLITUDE
                            } else {
                                cy - AMPLITUDE
                            };
                            if i == 0 {
                                elements.push(PathElement::MoveTo { x: px, y: py });
                            } else {
                                elements.push(PathElement::LineTo { x: px, y: py });
                            }
                        }
                        let path = Path { elements };
                        let stroke = Stroke {
                            width: 1.0,
                            cap: StrokeCap::Round,
                            join: StrokeJoin::Round,
                        };
                        self.sink.stroke_path(&path, color, &stroke, t);
                    }
                }
            }
            Some(Node::Image(_) | Node::File(_) | Node::Embed(_) | Node::Archived(_)) => {}
            _ => {}
        }
    }

    fn decoration(&mut self, decoration: &PageFragmentDecoration) {
        if !self.on(RenderLayer::Content) {
            return;
        }

        let local_rect = decoration.rect;
        let data = &decoration.data;
        let t = self.root_transform.translate(local_rect.x, local_rect.y);
        let inner_rect = Rect::from_xywh(0.0, 0.0, local_rect.width, local_rect.height);

        let parent_node = self.box_stack.last().and_then(|f| f.node.as_ref());

        match (parent_node, data) {
            (Some(Node::Callout(callout)), _) => {
                let icon_name = match *callout.variant.get() {
                    editor_model::CalloutVariant::Info => "lucide/info",
                    editor_model::CalloutVariant::Success => "lucide/circle-check",
                    editor_model::CalloutVariant::Warning => "lucide/circle-alert",
                    editor_model::CalloutVariant::Danger => "lucide/triangle-alert",
                };
                let color = self.theme.color(callout_token(*callout.variant.get()));
                if let Some(icon) = ICONS.resolve(icon_name) {
                    self.render_icon(icon, color, inner_rect, t, ICON_STROKE_WIDTH);
                }
            }

            (Some(Node::Blockquote(bq)), _)
                if *bq.variant.get() == editor_model::BlockquoteVariant::LeftQuote =>
            {
                let color = self.theme.color("ui.text.muted");
                if let Some(icon) = ICONS.resolve("typie/blockquote-quote") {
                    self.render_icon(icon, color, inner_rect, t, ICON_STROKE_WIDTH);
                }
            }

            (Some(Node::Blockquote(bq)), _)
                if *bq.variant.get() == editor_model::BlockquoteVariant::LeftLine =>
            {
                let color = self.theme.color("ui.border.default");
                self.sink.fill_rect(inner_rect, color, t);
            }

            (Some(Node::FoldTitle(_)), _) => {
                let expanded = matches!(data, DecorationData::Bool(true));
                let icon_name = if expanded {
                    "lucide/chevron-up"
                } else {
                    "lucide/chevron-down"
                };
                let color = self.theme.color("text.gray");
                if let Some(icon) = ICONS.resolve(icon_name) {
                    self.render_icon(icon, color, inner_rect, t, ICON_STROKE_WIDTH);
                }
            }

            (Some(Node::ListItem(_)), DecorationData::Glyphs(glyph_runs)) => {
                let color = self.theme.color("ui.text.default");
                let total_width: f32 = glyph_runs.iter().map(|r| r.width).sum();
                let x_offset = inner_rect.width - total_width;
                let offset_t = t.translate(x_offset, 0.0);
                self.render_glyph_runs(glyph_runs, color, offset_t);
            }

            (Some(Node::ListItem(_)), DecorationData::Bullet) => {
                // Bullets center on the line box; ordered markers align to baseline (set in measure).
                let color = self.theme.color("ui.text.default");
                let r = inner_rect.height * BULLET_RADIUS_RATIO;
                let cx = inner_rect.width - r;
                let cy = inner_rect.height / 2.0;
                let path = circle_path(cx, cy, r);
                self.sink.fill_path(&path, color, t);
            }

            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::export::VectorSink;
    use crate::vector::types::{VectorOp, VectorPage};
    use editor_common::EdgeInsets;
    use editor_crdt::Dot;
    use editor_resource::ThemeVariant;
    use editor_state::State;
    use editor_view::PageFragmentContent;
    use editor_view::glyph_run::RubyAnnotation;
    use editor_view::style::{Alignment, BorderMode, BoxStyle, Direction};

    #[derive(Default)]
    struct PathFillCounter {
        count: usize,
    }

    impl RenderSink for PathFillCounter {
        fn pixel_size(&self) -> (u32, u32) {
            (1000, 1000)
        }

        fn fill_rect(&mut self, _r: Rect, _c: Color, _t: Transform) {}

        fn fill_path(&mut self, _p: &Path, _c: Color, _t: Transform) {
            self.count += 1;
        }

        fn stroke_path(&mut self, _p: &Path, _c: Color, _s: &Stroke, _t: Transform) {}

        fn draw_glyph_run(
            &mut self,
            _r: &editor_view::glyph_run::GlyphRun,
            _c: Color,
            _t: Transform,
            _f: &editor_resource::FontRegistry,
        ) {
        }

        fn draw_image(&mut self, _i: &Image, _r: Rect, _t: Transform) {}
    }

    #[derive(Default)]
    struct PathRecorder {
        paths: Vec<Path>,
    }

    impl RenderSink for PathRecorder {
        fn pixel_size(&self) -> (u32, u32) {
            (1000, 1000)
        }

        fn fill_rect(&mut self, _r: Rect, _c: Color, _t: Transform) {}

        fn fill_path(&mut self, p: &Path, _c: Color, _t: Transform) {
            self.paths.push(p.clone());
        }

        fn stroke_path(&mut self, _p: &Path, _c: Color, _s: &Stroke, _t: Transform) {}

        fn draw_glyph_run(
            &mut self,
            _r: &editor_view::glyph_run::GlyphRun,
            _c: Color,
            _t: Transform,
            _f: &editor_resource::FontRegistry,
        ) {
        }

        fn draw_image(&mut self, _i: &Image, _r: Rect, _t: Transform) {}
    }

    fn black() -> Color {
        Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    fn all_edges() -> Edges<bool> {
        Edges {
            top: true,
            bottom: true,
            left: true,
            right: true,
        }
    }

    fn table_box_style(direction: Direction) -> BoxStyle {
        BoxStyle {
            direction,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets::all(TABLE_BORDER_WIDTH),
            border_mode: BorderMode::Collapse,
            alignment: Alignment::Start,
            decorations: vec![],
            monolithic: false,
        }
    }

    fn table_fragment(row_fragments: &[(Rect, Edges<bool>, Vec<Rect>)]) -> PageFragmentBox {
        PageFragmentBox {
            node: Dot::new(90, 1),
            style: table_box_style(Direction::Vertical),
            edges: all_edges(),
            decorations: vec![],
            children: row_fragments
                .iter()
                .map(|(row_rect, row_edges, cells)| PageFragmentNode {
                    rect: *row_rect,
                    content: PageFragmentContent::Box(PageFragmentBox {
                        node: Dot::new(91, 1),
                        style: table_box_style(Direction::Horizontal),
                        edges: *row_edges,
                        decorations: vec![],
                        children: cells
                            .iter()
                            .map(|cell_rect| PageFragmentNode {
                                rect: *cell_rect,
                                content: PageFragmentContent::Box(PageFragmentBox {
                                    node: Dot::new(92, 1),
                                    style: table_box_style(Direction::Vertical),
                                    edges: all_edges(),
                                    decorations: vec![],
                                    children: vec![],
                                    attachment: None,
                                }),
                            })
                            .collect(),
                        attachment: None,
                    }),
                })
                .collect(),
            attachment: None,
        }
    }

    fn two_by_two_table_fragment() -> PageFragmentBox {
        let edges = all_edges();
        table_fragment(&[
            (
                Rect::from_xywh(0.0, 0.0, 203.0, 32.0),
                edges,
                vec![
                    Rect::from_xywh(0.0, 0.0, 102.0, 32.0),
                    Rect::from_xywh(101.0, 0.0, 102.0, 32.0),
                ],
            ),
            (
                Rect::from_xywh(0.0, 31.0, 203.0, 32.0),
                edges,
                vec![
                    Rect::from_xywh(0.0, 31.0, 102.0, 32.0),
                    Rect::from_xywh(101.0, 31.0, 102.0, 32.0),
                ],
            ),
        ])
    }

    fn clipped_two_cell_table_fragment() -> PageFragmentBox {
        let clipped_edges = Edges {
            top: false,
            bottom: false,
            left: true,
            right: true,
        };
        table_fragment(&[(
            Rect::from_xywh(0.0, 0.0, 203.0, 20.0),
            clipped_edges,
            vec![
                Rect::from_xywh(0.0, 0.0, 102.0, 20.0),
                Rect::from_xywh(101.0, 0.0, 102.0, 20.0),
            ],
        )])
    }

    fn path_start(path: &Path) -> Option<(f32, f32)> {
        match path.elements.first() {
            Some(PathElement::MoveTo { x, y }) => Some((*x, *y)),
            _ => None,
        }
    }

    fn fragment_box(
        node: Dot,
        rect: Rect,
        style: BoxStyle,
        edges: Edges<bool>,
    ) -> PageFragmentNode {
        PageFragmentNode {
            rect,
            content: PageFragmentContent::Box(PageFragmentBox {
                node,
                style,
                edges,
                decorations: vec![],
                children: vec![],
                attachment: None,
            }),
        }
    }

    fn fragment_line(
        rect: Rect,
        metrics: LineMetrics,
        glyph_runs: Vec<editor_view::glyph_run::GlyphRun>,
        ruby_annotations: Vec<RubyAnnotation>,
    ) -> PageFragmentNode {
        PageFragmentNode {
            rect,
            content: PageFragmentContent::Line(PageFragmentLine {
                node: Dot::ROOT,
                baseline: metrics.baseline,
                ascent: metrics.ascent,
                descent: metrics.descent,
                cursor_ascent: metrics.ascent,
                cursor_descent: metrics.descent,
                glyph_runs,
                ruby_annotations,
                empty_caret_x: 0.0,
                offset_range: None,
                tab_gaps: vec![],
            }),
        }
    }

    fn line_fragment(node: &PageFragmentNode) -> &PageFragmentLine {
        match &node.content {
            PageFragmentContent::Line(fragment) => fragment,
            _ => unreachable!("expected line fragment"),
        }
    }

    #[test]
    fn circle_path_has_four_curves_and_close() {
        let path = circle_path(10.0, 10.0, 5.0);
        let curve_count = path
            .elements
            .iter()
            .filter(|e| matches!(e, PathElement::CurveTo { .. }))
            .count();
        assert_eq!(curve_count, 4);
        assert!(matches!(
            path.elements.first(),
            Some(PathElement::MoveTo { .. })
        ));
        assert!(matches!(path.elements.last(), Some(PathElement::Close)));
    }

    #[test]
    fn diamond_path_has_four_lines_and_close() {
        let path = diamond_path(10.0, 10.0, 5.0);
        let line_count = path
            .elements
            .iter()
            .filter(|e| matches!(e, PathElement::LineTo { .. }))
            .count();
        assert_eq!(line_count, 3); // 3 LineTo + 1 MoveTo + Close
        assert!(matches!(
            path.elements.first(),
            Some(PathElement::MoveTo { .. })
        ));
        assert!(matches!(path.elements.last(), Some(PathElement::Close)));
    }

    #[test]
    fn layer_set_contains_single() {
        let set = LayerSet::of(&[RenderLayer::Background]);
        assert!(set.contains(RenderLayer::Background));
        assert!(!set.contains(RenderLayer::Content));
        assert!(!set.contains(RenderLayer::Border));
    }

    #[test]
    fn layer_set_contains_multiple() {
        let set = LayerSet::of(&[RenderLayer::Content, RenderLayer::Border]);
        assert!(!set.contains(RenderLayer::Background));
        assert!(set.contains(RenderLayer::Content));
        assert!(set.contains(RenderLayer::Border));
    }

    #[test]
    fn layer_set_empty() {
        let set = LayerSet::of(&[]);
        assert!(!set.contains(RenderLayer::Background));
        assert!(!set.contains(RenderLayer::Content));
        assert!(!set.contains(RenderLayer::Border));
    }

    #[test]
    fn mark_data_layer_below_content() {
        assert_eq!(
            MarkData::Selection { focused: true }.layer(),
            MarkLayer::BelowContent
        );
    }

    #[test]
    fn mark_data_layer_above_content() {
        assert_eq!(MarkData::Composition.layer(), MarkLayer::AboveContent);
        assert_eq!(MarkData::DropIndicator.layer(), MarkLayer::AboveContent);
    }

    #[test]
    fn selection_mark_alpha_depends_on_focus() {
        let theme = Theme::new(ThemeVariant::LightWhite);

        assert_eq!(selection_mark_color(&theme, true).a, 77);
        assert_eq!(selection_mark_color(&theme, false).a, 48);
    }

    #[test]
    fn drop_indicator_mark_uses_selection_color() {
        let renderer = Renderer::new(Arc::new(Mutex::new(Resource::new_test())));
        let theme = Theme::new(ThemeVariant::LightWhite);

        assert_eq!(
            renderer
                .resolve_mark_color(&MarkData::DropIndicator, &theme)
                .unwrap(),
            theme.color("selection")
        );
    }

    #[test]
    fn message_tail_sent_path_has_moveto_three_quads_and_close() {
        let path = build_message_tail(100.0, 50.0, true);
        let move_count = path
            .elements
            .iter()
            .filter(|e| matches!(e, PathElement::MoveTo { .. }))
            .count();
        let quad_count = path
            .elements
            .iter()
            .filter(|e| matches!(e, PathElement::QuadTo { .. }))
            .count();
        let close_count = path
            .elements
            .iter()
            .filter(|e| matches!(e, PathElement::Close))
            .count();
        assert_eq!(move_count, 1);
        assert_eq!(quad_count, 3);
        assert_eq!(close_count, 1);

        // The sent tail overflows to the right past the box width.
        let max_x = path
            .elements
            .iter()
            .filter_map(|e| match e {
                PathElement::MoveTo { x, .. } | PathElement::LineTo { x, .. } => Some(x),
                PathElement::QuadTo { x, .. } | PathElement::CurveTo { x, .. } => Some(x),
                PathElement::Close => None,
            })
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            max_x > 100.0,
            "sent tail should overflow right past width, max_x={}",
            max_x
        );
    }

    #[test]
    fn message_tail_received_path_overflows_to_the_left() {
        let path = build_message_tail(100.0, 50.0, false);
        let min_x = path
            .elements
            .iter()
            .filter_map(|e| match e {
                PathElement::MoveTo { x, .. } | PathElement::LineTo { x, .. } => Some(x),
                PathElement::QuadTo { x, .. } | PathElement::CurveTo { x, .. } => Some(x),
                PathElement::Close => None,
            })
            .copied()
            .fold(f32::INFINITY, f32::min);
        assert!(
            min_x < 0.0,
            "received tail should overflow left past 0, min_x={}",
            min_x
        );
    }

    #[test]
    fn message_bubble_radii_full_box_sent_no_split() {
        let edges = Edges {
            top: true,
            bottom: true,
            left: true,
            right: true,
        };
        let radii = message_bubble_radii(18.0, &edges, true, true);
        assert!((radii.top_left - 18.0).abs() < 0.01);
        assert!((radii.top_right - 18.0).abs() < 0.01);
        assert!((radii.bottom_left - 18.0).abs() < 0.01);
        // The sent tail makes bottom-right a sharp corner.
        assert!(radii.bottom_right.abs() < 0.01);
    }

    #[test]
    fn message_bubble_radii_received_kills_bottom_left_when_tail_present() {
        let edges = Edges {
            top: true,
            bottom: true,
            left: true,
            right: true,
        };
        let radii = message_bubble_radii(18.0, &edges, false, true);
        assert!((radii.bottom_right - 18.0).abs() < 0.01);
        assert!(radii.bottom_left.abs() < 0.01);
    }

    #[test]
    fn message_bubble_radii_top_split_squares_top_corners() {
        let edges = Edges {
            top: false,
            bottom: true,
            left: true,
            right: true,
        };
        let radii = message_bubble_radii(18.0, &edges, true, true);
        assert!(radii.top_left.abs() < 0.01);
        assert!(radii.top_right.abs() < 0.01);
        // bottom_left remains rounded; bottom_right is sharp due to the tail.
        assert!((radii.bottom_left - 18.0).abs() < 0.01);
        assert!(radii.bottom_right.abs() < 0.01);
    }

    #[test]
    fn message_bubble_radii_bottom_split_no_tail_squares_bottom_corners() {
        let edges = Edges {
            top: true,
            bottom: false,
            left: true,
            right: true,
        };
        // The call site passes has_tail = false when the bottom edge is split.
        let radii = message_bubble_radii(18.0, &edges, true, false);
        assert!((radii.top_left - 18.0).abs() < 0.01);
        assert!((radii.top_right - 18.0).abs() < 0.01);
        assert!(radii.bottom_left.abs() < 0.01);
        assert!(radii.bottom_right.abs() < 0.01);
    }

    #[test]
    fn underline_rect_sits_below_baseline_in_descent_band() {
        let m = LineMetrics {
            baseline: 80.0,
            ascent: 70.0,
            descent: 10.0,
        };
        let r = underline_rect(m, 5.0, 50.0);
        assert!((r.x - 5.0).abs() < 0.01);
        assert!((r.y - 85.0).abs() < 0.01); // 80 + 10 * 0.5
        assert!((r.width - 50.0).abs() < 0.01);
        assert!((r.height - 1.0).abs() < 0.01);
    }

    #[test]
    fn strikethrough_rect_sits_above_baseline_within_ascent() {
        let m = LineMetrics {
            baseline: 80.0,
            ascent: 70.0,
            descent: 10.0,
        };
        let r = strikethrough_rect(m, 5.0, 50.0);
        assert!((r.x - 5.0).abs() < 0.01);
        assert!((r.y - 59.0).abs() < 0.01); // 80 - 70 * 0.3
        assert!((r.width - 50.0).abs() < 0.01);
        assert!((r.height - 1.0).abs() < 0.01);
    }

    #[test]
    fn text_background_rect_matches_v1_formula_and_differs_from_selection_height() {
        let line_rect = Rect::from_xywh(0.0, 0.0, 120.0, 100.0);
        let m = LineMetrics {
            baseline: 65.0,
            ascent: 45.0,
            descent: 15.0,
        };

        // No ruby (ruby_extra_top = 0): identical to the legacy centred formula.
        let r = text_background_rect(line_rect, m, 0.0, 12.0, 34.0);
        let v1_text_height = m.ascent + m.descent;
        let selection_height = line_rect.height;
        let line_top = (selection_height - v1_text_height) * 0.5;

        assert!((r.x - 12.0).abs() < 0.01);
        assert!((r.y - line_top).abs() < 0.01);
        assert!((r.width - 34.0).abs() < 0.01);
        assert!((r.height - v1_text_height).abs() < 0.01);
        assert!((selection_height - 100.0).abs() < 0.01);
        assert!(r.height < selection_height);
    }

    #[test]
    fn text_background_rect_excludes_ruby_band() {
        // Ruby inflates `ascent` and the line box top by `ruby_extra_top`. The
        // fill must hug the base text only: height drops by the ruby band and
        // the rect starts below it, never reaching into the ruby. (TR-222)
        let line_rect = Rect::from_xywh(0.0, 0.0, 120.0, 100.0);
        let m = LineMetrics {
            baseline: 80.0,
            ascent: 60.0, // base ascent 40 + ruby band 20
            descent: 15.0,
        };
        let ruby_extra_top = 20.0;

        let with_ruby = text_background_rect(line_rect, m, ruby_extra_top, 12.0, 34.0);
        let without = text_background_rect(line_rect, m, 0.0, 12.0, 34.0);

        // Height excludes the ruby band: (60-20)+15 = 55, vs 75 without.
        assert!((with_ruby.height - 55.0).abs() < 0.01);
        assert!(with_ruby.height < without.height);
        // The rect starts below the ruby band reserved at the top.
        assert!(with_ruby.y >= ruby_extra_top - 0.01);
    }

    /// Render the Background layer for a one-line doc and return the single
    /// background-fill rect (page-local).
    fn background_fill_rect(state: &State) -> Rect {
        #[derive(Default)]
        struct RecordingSink {
            rects: Vec<Rect>,
        }
        impl RenderSink for RecordingSink {
            fn pixel_size(&self) -> (u32, u32) {
                (1000, 1000)
            }
            fn fill_rect(&mut self, r: Rect, _c: Color, _t: Transform) {
                self.rects.push(r);
            }
            fn fill_path(&mut self, _p: &Path, _c: Color, _t: Transform) {}
            fn stroke_path(&mut self, _p: &Path, _c: Color, _s: &Stroke, _t: Transform) {}
            fn draw_glyph_run(
                &mut self,
                _r: &editor_view::glyph_run::GlyphRun,
                _c: Color,
                _t: Transform,
                _f: &editor_resource::FontRegistry,
            ) {
            }
            fn draw_image(&mut self, _i: &Image, _r: Rect, _t: Transform) {}
        }

        let resource = Arc::new(Mutex::new(Resource::new_test()));
        let mut view = editor_view::View::new_test();
        view.layout(state);
        let doc = state.view();
        let mut renderer = Renderer::new(resource);
        let mut sink = RecordingSink::default();
        view.visit_page(
            0,
            &mut renderer.page_visitor(
                &mut sink,
                &doc,
                1.0,
                LayerSet::of(&[RenderLayer::Background]),
            ),
        );
        assert_eq!(sink.rects.len(), 1, "expected exactly one background fill");
        sink.rects[0]
    }

    #[test]
    fn text_background_height_matches_v1_and_excludes_ruby() {
        use editor_macros::state;

        // V1 fills the background with `metric.height` (= ascent + descent),
        // excluding ruby which it parks in paint-overflow above the line box.
        // The new engine must match: the same text with and without ruby yields
        // the same background height. (TR-222)
        let (plain, _) = state! { doc { root { p1: paragraph { text("ABCD") [background_color("yellow".to_string())] } } } selection: none };
        let (ruby, _) = state! {
            doc {
                root {
                    p2: paragraph {
                        text("ABCD") [background_color("yellow".to_string()), ruby(text: "かな".to_string())]
                    }
                }
            }
            selection: none
        };

        let plain_rect = background_fill_rect(&plain);
        let ruby_rect = background_fill_rect(&ruby);

        // Ruby must not change the background height — it hugs the base text only.
        assert!(
            (ruby_rect.height - plain_rect.height).abs() < 0.5,
            "ruby must not change background height: plain={}, ruby={}",
            plain_rect.height,
            ruby_rect.height,
        );
    }

    #[test]
    fn line_background_color_fill_uses_text_area_not_line_box_height() {
        use editor_view::glyph_run::{GlyphRun, Synthesis, TextDecoration};

        #[derive(Default)]
        struct RecordingSink {
            rects: Vec<Rect>,
        }
        impl RenderSink for RecordingSink {
            fn pixel_size(&self) -> (u32, u32) {
                (1000, 1000)
            }
            fn fill_rect(&mut self, r: Rect, _c: Color, _t: Transform) {
                self.rects.push(r);
            }
            fn fill_path(&mut self, _p: &Path, _c: Color, _t: Transform) {}
            fn stroke_path(&mut self, _p: &Path, _c: Color, _s: &Stroke, _t: Transform) {}
            fn draw_glyph_run(
                &mut self,
                _r: &editor_view::glyph_run::GlyphRun,
                _c: Color,
                _t: Transform,
                _f: &editor_resource::FontRegistry,
            ) {
            }
            fn draw_image(&mut self, _i: &Image, _r: Rect, _t: Transform) {}
        }

        let resource = Arc::new(Mutex::new(Resource::new_test()));
        let state = State::empty();
        let doc = state.view();
        let mut renderer = Renderer::new(resource);
        let mut sink = RecordingSink::default();
        let mut visitor = renderer.page_visitor(
            &mut sink,
            &doc,
            1.0,
            LayerSet::of(&[RenderLayer::Background]),
        );

        let run = GlyphRun {
            family_id: 0,
            weight: 400,
            font_size: 16.0,
            synthesis: Synthesis::default(),
            color: "text.black".to_string(),
            background_color: Some("bg.yellow".to_string()),
            glyphs: vec![],
            decoration: TextDecoration::default(),
            offset_range: 0..0,
            link: None,
            text: "abc".to_string(),
            x: 8.0,
            width: 24.0,
            graphemes: vec![],
            cursor_ascent: 0.0,
            cursor_descent: 0.0,
        };

        let line_node = fragment_line(
            Rect::from_xywh(0.0, 0.0, 120.0, 100.0),
            LineMetrics {
                baseline: 65.0,
                ascent: 45.0,
                descent: 15.0,
            },
            vec![run],
            vec![],
        );
        visitor.line(&line_node, line_fragment(&line_node));

        assert_eq!(sink.rects.len(), 1);
        let rect = sink.rects[0];
        assert!((rect.x - 8.0).abs() < 0.01);
        assert!((rect.y - 20.0).abs() < 0.01);
        assert!((rect.width - 24.0).abs() < 0.01);
        assert!((rect.height - 60.0).abs() < 0.01);
    }

    fn export_page_to_vector_page(state: &State) -> VectorPage {
        let resource = Arc::new(Mutex::new(Resource::new_test()));
        let mut view = editor_view::View::new_test();
        view.layout(state);
        let doc = state.view();

        let (width, height) = view
            .pages()
            .first()
            .map(|p| (p.size.width, p.size.height))
            .expect("test document must layout to at least one page");

        let mut renderer = Renderer::new(resource);
        let mut sink = VectorSink::new();

        view.visit_page(
            0,
            &mut renderer.vector_page_visitor(
                &mut sink,
                &doc,
                1.0,
                LayerSet::of(&[RenderLayer::Background]),
            ),
        );
        view.visit_page(
            0,
            &mut renderer.vector_page_visitor(
                &mut sink,
                &doc,
                1.0,
                LayerSet::of(&[RenderLayer::Content, RenderLayer::Border]),
            ),
        );

        sink.into_page(width, height)
    }

    #[test]
    fn table_border_page_is_vectorized() {
        // 테이블 보더가 페이지 export 결과에서 벡터 path op로 나타나는지 확인한다.
        use editor_macros::state;

        let (state,) = state! {
            doc {
                root {
                    table {
                        table_row {
                            table_cell { paragraph }
                            table_cell { paragraph }
                        }
                    }
                }
            }
            selection: none
        };

        let page = export_page_to_vector_page(&state);

        assert!(page.width > 0.0);
        assert!(page.height > 0.0);
        assert!(
            page.ops
                .iter()
                .any(|op| matches!(op, VectorOp::FillPath { .. } | VectorOp::StrokePath { .. })),
            "table border page must contain vector path ops"
        );
    }

    #[test]
    fn horizontal_rule_pattern_page_is_vectorized() {
        // horizontal rule 패턴이 페이지 export 결과에서 벡터 path op로 나타나는지 확인한다.
        use editor_macros::state;

        let (state,) = state! {
            doc {
                root {
                    paragraph { text("a") }
                    horizontal_rule
                }
            }
            selection: none
        };

        let page = export_page_to_vector_page(&state);

        assert!(page.width > 0.0);
        assert!(page.height > 0.0);
        assert!(
            page.ops
                .iter()
                .any(|op| matches!(op, VectorOp::FillPath { .. } | VectorOp::StrokePath { .. })),
            "horizontal rule page must contain vector path ops"
        );
    }

    #[test]
    fn line_with_ruby_annotation_renders_glyph_through_raster_path() {
        use editor_view::glyph_run::{Glyph, RubyAnnotation, Synthesis};

        // Pretendard-Regular 'A' (U+0041) is glyph id 3, which has an outline.
        const TEST_FONT: &[u8] = include_bytes!("../../../assets/Pretendard-Regular.ttf");

        #[derive(Default)]
        struct CountingSink {
            path_fills: usize,
            image_draws: usize,
        }
        impl RenderSink for CountingSink {
            fn pixel_size(&self) -> (u32, u32) {
                (1000, 1000)
            }
            fn fill_rect(&mut self, _r: Rect, _c: Color, _t: Transform) {}
            fn fill_path(&mut self, _p: &Path, _c: Color, _t: Transform) {
                self.path_fills += 1;
            }
            fn stroke_path(&mut self, _p: &Path, _c: Color, _s: &Stroke, _t: Transform) {}
            fn draw_glyph_run(
                &mut self,
                _r: &editor_view::glyph_run::GlyphRun,
                _c: Color,
                _t: Transform,
                _f: &editor_resource::FontRegistry,
            ) {
            }
            fn draw_image(&mut self, _i: &Image, _r: Rect, _t: Transform) {
                self.image_draws += 1;
            }
        }

        let mut resource = Resource::new_test();
        let compressed = editor_resource::compress_zstd(TEST_FONT);
        resource.add_font_base("test", 400, &compressed).unwrap();
        let family_id = resource
            .font_registry
            .intern_id("test")
            .expect("test font must be registered");

        let state = State::empty();
        let doc = state.view();
        let mut renderer = Renderer::new(Arc::new(Mutex::new(resource)));

        let mut sink = CountingSink::default();
        let mut v =
            renderer.page_visitor(&mut sink, &doc, 1.0, LayerSet::of(&[RenderLayer::Content]));

        let ruby = RubyAnnotation {
            family_id,
            weight: 400,
            font_size: 8.0,
            synthesis: Synthesis::default(),
            color: "text.black".to_string(),
            ascent: 6.0,
            descent: 2.0,
            glyphs: vec![Glyph {
                id: 3,
                x: 0.0,
                y: 0.0,
            }], // 'A'
            x: 0.0,
            baseline_y: -8.0,
            width: 10.0,
        };

        let line_node = fragment_line(
            Rect::from_xywh(0.0, 0.0, 100.0, 20.0),
            LineMetrics {
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
            },
            vec![],
            vec![ruby],
        );
        v.line(&line_node, line_fragment(&line_node));

        // SVG 테이블이 없는 Pretendard outline glyph 는 raster 경로(draw_glyph → draw_image)로
        // 렌더된다. fill_path 는 SVG 테이블 glyph 전용.
        assert!(
            sink.image_draws > 0,
            "ruby annotation glyph must be rendered through the raster glyph path"
        );
        assert_eq!(sink.path_fills, 0);
    }

    #[test]
    fn baked_glyph_mask_is_cached_across_identical_renders() {
        use editor_view::glyph_run::{Glyph, GlyphRun, Synthesis, TextDecoration};

        const TEST_FONT: &[u8] = include_bytes!("../../../assets/Pretendard-Regular.ttf");

        struct NoopSink;
        impl RenderSink for NoopSink {
            fn pixel_size(&self) -> (u32, u32) {
                (1000, 1000)
            }
            fn fill_rect(&mut self, _r: Rect, _c: Color, _t: Transform) {}
            fn fill_path(&mut self, _p: &Path, _c: Color, _t: Transform) {}
            fn stroke_path(&mut self, _p: &Path, _c: Color, _s: &Stroke, _t: Transform) {}
            fn draw_image(&mut self, _i: &Image, _r: Rect, _t: Transform) {}
        }

        let mut resource = Resource::new_test();
        let compressed = editor_resource::compress_zstd(TEST_FONT);
        resource.add_font_base("test", 400, &compressed).unwrap();
        let family_id = resource
            .font_registry
            .intern_id("test")
            .expect("test font must be registered");

        let state = State::empty();
        let doc = state.view();
        let mut renderer = Renderer::new(Arc::new(Mutex::new(resource)));

        let make_line = || {
            let run = GlyphRun {
                family_id,
                weight: 400,
                font_size: 16.0,
                synthesis: Synthesis::default(),
                color: "text.black".to_string(),
                background_color: None,
                glyphs: vec![
                    Glyph {
                        id: 3,
                        x: 0.0,
                        y: 0.0,
                    },
                    Glyph {
                        id: 3,
                        x: 12.0,
                        y: 0.0,
                    },
                ],
                decoration: TextDecoration::default(),
                offset_range: 0..0,
                link: None,
                text: "AA".to_string(),
                x: 0.0,
                width: 24.0,
                graphemes: vec![],
                cursor_ascent: 0.0,
                cursor_descent: 0.0,
            };
            fragment_line(
                Rect::from_xywh(0.0, 0.0, 100.0, 20.0),
                LineMetrics {
                    baseline: 16.0,
                    ascent: 14.0,
                    descent: 4.0,
                },
                vec![run],
                vec![],
            )
        };

        let render_once = |renderer: &mut Renderer| {
            let mut sink = NoopSink;
            let line = make_line();
            let mut v =
                renderer.page_visitor(&mut sink, &doc, 1.0, LayerSet::of(&[RenderLayer::Content]));
            v.line(&line, line_fragment(&line));
        };

        BAKE_COUNT.with(|c| c.set(0));
        render_once(&mut renderer);
        let first = BAKE_COUNT.with(|c| c.get());
        assert!(
            first > 0,
            "first render of real-font text must bake at least one glyph mask"
        );

        render_once(&mut renderer);
        let second = BAKE_COUNT.with(|c| c.get()) - first;
        assert_eq!(
            second, 0,
            "second render of identical text must hit the baked-glyph cache (zero re-bakes)"
        );
    }

    #[test]
    fn svg_path_glyphs_are_filled_instead_of_drawn_as_images() {
        // SVG path 캐시 glyph 는 이미지 draw 경로가 아니라 fill_path 로 렌더되어야 한다.
        #[derive(Default)]
        struct CountingSink {
            path_fills: usize,
            image_draws: usize,
        }

        impl RenderSink for CountingSink {
            fn pixel_size(&self) -> (u32, u32) {
                (100, 100)
            }
            fn fill_rect(&mut self, _r: Rect, _c: Color, _t: Transform) {}
            fn fill_path(&mut self, _p: &Path, _c: Color, _t: Transform) {
                self.path_fills += 1;
            }
            fn stroke_path(&mut self, _p: &Path, _c: Color, _s: &Stroke, _t: Transform) {}
            fn draw_glyph_run(
                &mut self,
                _r: &editor_view::glyph_run::GlyphRun,
                _c: Color,
                _t: Transform,
                _f: &editor_resource::FontRegistry,
            ) {
            }
            fn draw_image(&mut self, _i: &Image, _r: Rect, _t: Transform) {
                self.image_draws += 1;
            }
        }

        let glyph = PositionedSvgPathGlyph {
            path: crate::glyph::SvgPathGlyph {
                path: Path {
                    elements: vec![
                        PathElement::MoveTo { x: 0.0, y: 0.0 },
                        PathElement::LineTo { x: 4.0, y: 0.0 },
                        PathElement::LineTo { x: 4.0, y: 4.0 },
                        PathElement::Close,
                    ],
                },
                placement_left: 0,
                placement_top: 0,
            },
            blit_x: 12,
            blit_y: 24,
        };

        let mut sink = CountingSink::default();
        draw_positioned_svg_path_glyph(
            &mut sink,
            &glyph,
            Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            },
        );

        assert_eq!(sink.path_fills, 1);
        assert_eq!(sink.image_draws, 0);
    }

    #[test]
    fn fold_title_background_drawn_on_every_visible_page_piece() {
        use editor_common::EdgeInsets;
        use editor_macros::state;
        use editor_view::style::{Alignment, BorderMode, Decoration, Direction};

        #[derive(Default)]
        struct RecordingSink {
            fills: Vec<Color>,
        }
        impl RenderSink for RecordingSink {
            fn pixel_size(&self) -> (u32, u32) {
                (1000, 1000)
            }
            fn fill_rect(&mut self, _r: Rect, c: Color, _t: Transform) {
                self.fills.push(c);
            }
            fn fill_path(&mut self, _p: &Path, c: Color, _t: Transform) {
                self.fills.push(c);
            }
            fn stroke_path(&mut self, _p: &Path, _c: Color, _s: &Stroke, _t: Transform) {}
            fn draw_glyph_run(
                &mut self,
                _r: &editor_view::glyph_run::GlyphRun,
                _c: Color,
                _t: Transform,
                _f: &editor_resource::FontRegistry,
            ) {
            }
            fn draw_image(&mut self, _i: &Image, _r: Rect, _t: Transform) {}
        }

        let (state, ft) = state! {
            doc {
                root {
                    fold {
                        ft: fold_title { text("Title") }
                        fold_content { paragraph { text("c") } }
                    }
                }
            }
            selection: none
        };
        let doc = state.view();

        let mut renderer = Renderer::new(Arc::new(Mutex::new(Resource::new_test())));
        let muted = renderer
            .resource
            .lock()
            .unwrap()
            .theme
            .color("ui.surface.muted");

        let icon_rect = Rect::from_xywh(12.0, 8.0, 20.0, 20.0);
        let style = BoxStyle {
            direction: Direction::Vertical,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets::ZERO,
            border_mode: BorderMode::Separate,
            alignment: Alignment::Start,
            decorations: vec![Decoration {
                id: 0,
                rect: icon_rect,
                data: DecorationData::Bool(true),
            }],
            monolithic: false,
        };

        let mut sink = RecordingSink::default();
        {
            let mut v = renderer.page_visitor(
                &mut sink,
                &doc,
                1.0,
                LayerSet::of(&[RenderLayer::Background]),
            );

            let first_node = fragment_box(
                ft,
                Rect::from_xywh(0.0, 500.0, 300.0, 80.0),
                style.clone(),
                Edges {
                    top: true,
                    bottom: false,
                    left: true,
                    right: true,
                },
            );
            v.box_enter(&first_node, first_node.as_box().unwrap());
            v.decoration(&PageFragmentDecoration {
                rect: icon_rect,
                data: DecorationData::Bool(true),
            });
            v.box_exit(&first_node, first_node.as_box().unwrap());

            let second_node = fragment_box(
                ft,
                Rect::from_xywh(0.0, -40.0, 300.0, 80.0),
                style.clone(),
                Edges {
                    top: false,
                    bottom: true,
                    left: true,
                    right: true,
                },
            );
            v.box_enter(&second_node, second_node.as_box().unwrap());
            v.box_exit(&second_node, second_node.as_box().unwrap());
        }

        let muted_fills = sink.fills.iter().filter(|c| **c == muted).count();
        assert_eq!(
            muted_fills, 2,
            "fold title background must be painted on every page fragment, got {muted_fills}",
        );
    }

    #[test]
    fn table_solid_grid_draws_correct_line_count() {
        let table = two_by_two_table_fragment();
        let mut sink = PathFillCounter::default();
        draw_table_grid(
            &mut sink,
            Rect::from_xywh(0.0, 0.0, 203.0, 63.0),
            &table,
            TableBorderStyle::Solid,
            black(),
            Transform::IDENTITY,
        );

        assert_eq!(
            sink.count, 6,
            "solid 2×2 table should produce 6 fill_path calls, got {}",
            sink.count
        );
    }

    #[test]
    fn table_none_border_draws_nothing() {
        let edges = all_edges();
        let table = table_fragment(&[(
            Rect::from_xywh(0.0, 0.0, 102.0, 32.0),
            edges,
            vec![Rect::from_xywh(0.0, 0.0, 102.0, 32.0)],
        )]);
        let mut sink = PathFillCounter::default();
        draw_table_grid(
            &mut sink,
            Rect::from_xywh(0.0, 0.0, 102.0, 32.0),
            &table,
            TableBorderStyle::None,
            black(),
            Transform::IDENTITY,
        );

        assert_eq!(sink.count, 0, "TableBorderStyle::None must draw nothing");
    }

    #[test]
    fn table_row_and_cell_draw_no_border() {
        use editor_macros::state;

        #[derive(Default)]
        struct FillCounter {
            count: usize,
        }
        impl RenderSink for FillCounter {
            fn pixel_size(&self) -> (u32, u32) {
                (1000, 1000)
            }
            fn fill_rect(&mut self, _r: Rect, _c: Color, _t: Transform) {}
            fn fill_path(&mut self, _p: &Path, _c: Color, _t: Transform) {
                self.count += 1;
            }
            fn stroke_path(&mut self, _p: &Path, _c: Color, _s: &Stroke, _t: Transform) {}
            fn draw_glyph_run(
                &mut self,
                _r: &editor_view::glyph_run::GlyphRun,
                _c: Color,
                _t: Transform,
                _f: &editor_resource::FontRegistry,
            ) {
            }
            fn draw_image(&mut self, _i: &Image, _r: Rect, _t: Transform) {}
        }

        let (state, t1) = state! {
            doc {
                root {
                    t1: table {
                        table_row {
                            table_cell { paragraph }
                        }
                    }
                }
            }
            selection: none
        };
        let doc = state.view();

        let row_id = doc.node(t1).unwrap().child_blocks().next().unwrap().id();
        let cell_id = doc
            .node(row_id)
            .unwrap()
            .child_blocks()
            .next()
            .unwrap()
            .id();

        let mut renderer = Renderer::new(Arc::new(Mutex::new(Resource::new_test())));
        let mut sink = FillCounter::default();
        {
            let mut v =
                renderer.page_visitor(&mut sink, &doc, 1.0, LayerSet::of(&[RenderLayer::Border]));
            let row_node = fragment_box(
                row_id,
                Rect::from_xywh(0.0, 0.0, 102.0, 32.0),
                table_box_style(Direction::Horizontal),
                Edges {
                    top: true,
                    bottom: true,
                    left: true,
                    right: true,
                },
            );
            v.box_enter(&row_node, row_node.as_box().unwrap());
            v.box_exit(&row_node, row_node.as_box().unwrap());
            let cell_node = fragment_box(
                cell_id,
                Rect::from_xywh(0.0, 0.0, 100.0, 30.0),
                table_box_style(Direction::Vertical),
                Edges {
                    top: true,
                    bottom: true,
                    left: true,
                    right: true,
                },
            );
            v.box_enter(&cell_node, cell_node.as_box().unwrap());
            v.box_exit(&cell_node, cell_node.as_box().unwrap());
        }
        assert_eq!(
            sink.count, 0,
            "TableRow and TableCell box_exit must draw nothing"
        );
    }

    #[test]
    fn table_clipped_row_draws_no_page_edge_horizontal_border() {
        let table = clipped_two_cell_table_fragment();
        let mut sink = PathFillCounter::default();
        draw_table_grid(
            &mut sink,
            Rect::from_xywh(0.0, 0.0, 203.0, 20.0),
            &table,
            TableBorderStyle::Solid,
            black(),
            Transform::IDENTITY,
        );

        assert_eq!(
            sink.count, 3,
            "clipped row should draw only vertical grid lines, got {} fills",
            sink.count
        );
    }

    #[test]
    fn table_grid_uses_table_local_coordinates_for_offset_fragment() {
        let edges = all_edges();
        let table = table_fragment(&[(
            Rect::from_xywh(40.0, 60.0, 203.0, 32.0),
            edges,
            vec![
                Rect::from_xywh(40.0, 60.0, 102.0, 32.0),
                Rect::from_xywh(141.0, 60.0, 102.0, 32.0),
            ],
        )]);
        let mut sink = PathRecorder::default();

        draw_table_grid(
            &mut sink,
            Rect::from_xywh(40.0, 60.0, 203.0, 32.0),
            &table,
            TableBorderStyle::Solid,
            black(),
            Transform::IDENTITY,
        );

        assert_eq!(path_start(&sink.paths[0]), Some((0.0, 0.0)));
        assert_eq!(path_start(&sink.paths[1]), Some((101.0, 0.0)));
        assert_eq!(path_start(&sink.paths[2]), Some((202.0, 0.0)));
    }

    #[test]
    fn table_dashed_produces_more_draws_than_solid() {
        let solid_table = two_by_two_table_fragment();
        let dashed_table = two_by_two_table_fragment();
        let mut solid_sink = PathFillCounter::default();
        let mut dashed_sink = PathFillCounter::default();

        draw_table_grid(
            &mut solid_sink,
            Rect::from_xywh(0.0, 0.0, 203.0, 63.0),
            &solid_table,
            TableBorderStyle::Solid,
            black(),
            Transform::IDENTITY,
        );
        draw_table_grid(
            &mut dashed_sink,
            Rect::from_xywh(0.0, 0.0, 203.0, 63.0),
            &dashed_table,
            TableBorderStyle::Dashed,
            black(),
            Transform::IDENTITY,
        );

        assert!(
            dashed_sink.count > solid_sink.count,
            "dashed ({}) should produce more fill_path calls than solid ({})",
            dashed_sink.count,
            solid_sink.count
        );
    }
}
