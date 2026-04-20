use editor_common::Rect;
use editor_model::{Doc, Node, NodeId};
use editor_resource::Resource;
use editor_view::style::{BoxStyle, DecorationData};
use editor_view::{Edges, PageRect, PageVisitor};
use std::sync::{Arc, Mutex};

use crate::glyph::{Content, GlyphCache, ScaleContext};
use crate::icons::ICONS;
use crate::sink::RenderSink;
use crate::theme::Theme;
use crate::theme_data::ThemeVariant;
use crate::types::{
    Color, CornerRadii, IconData, IconElement, Image, Path, PathElement, Stroke, StrokeCap,
    StrokeJoin, Transform,
};

fn bake_mask_to_premul_rgba(mask: &[u8], width: u32, height: u32, color: Color) -> Image {
    let color_r = color.r as u32;
    let color_g = color.g as u32;
    let color_b = color.b as u32;
    let color_a = color.a as u32;

    // legacy blit 공식과 동일: a = (m * color_a) >> 8, rgb_premul = (a * c) >> 8.
    // zero canvas 위에서 legacy 의 직접 블릿 출력과 byte-exact 일치하도록 선택했다.
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for &m in mask {
        let a = (m as u32 * color_a) >> 8;
        let pr = (a * color_r) >> 8;
        let pg = (a * color_g) >> 8;
        let pb = (a * color_b) >> 8;
        data.push(pr as u8);
        data.push(pg as u8);
        data.push(pb as u8);
        data.push(a as u8);
    }
    Image {
        data,
        width,
        height,
    }
}

fn callout_token(variant: editor_model::CalloutVariant) -> &'static str {
    match variant {
        editor_model::CalloutVariant::Info => "ui.callout.info",
        editor_model::CalloutVariant::Success => "ui.callout.success",
        editor_model::CalloutVariant::Warning => "ui.callout.warning",
        editor_model::CalloutVariant::Danger => "ui.callout.danger",
    }
}

const CALLOUT_BORDER_RADIUS: f32 = 8.0;
const CALLOUT_BORDER_WIDTH: f32 = 1.0;
const ICON_STROKE_WIDTH: f32 = 1.5;
const HR_LINE_HEIGHT: f32 = 1.0;
const HR_SHAPE_SIZE_LARGE: f32 = 10.0;
const HR_SHAPE_SIZE_SMALL: f32 = 8.0;
const HR_SHAPE_GAP: f32 = 8.0;

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
    const K: f32 = 0.5522847498;
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
    Selection,
    Composition,
}

impl MarkData {
    pub fn layer(&self) -> MarkLayer {
        match self {
            Self::Selection => MarkLayer::BelowContent,
            Self::Composition => MarkLayer::AboveContent,
        }
    }
}

pub type MarkRect = PageRect<()>;

pub struct Mark {
    pub data: MarkData,
    pub rects: Vec<MarkRect>,
}

struct MarkStyle {
    color: Color,
}

pub struct Renderer {
    pub(crate) theme: Theme,
    pub(crate) resource: Arc<Mutex<Resource>>,
    pub(crate) scale_ctx: ScaleContext,
    pub(crate) glyph_cache: GlyphCache,
}

impl Renderer {
    pub fn new(variant: ThemeVariant, resource: Arc<Mutex<Resource>>) -> Self {
        Self {
            theme: Theme::new(variant),
            resource,
            scale_ctx: ScaleContext::new(),
            glyph_cache: GlyphCache::new(),
        }
    }

    pub fn set_theme_variant(&mut self, variant: ThemeVariant) {
        self.theme.set_variant(variant);
    }

    fn resolve_mark(&self, data: &MarkData) -> MarkStyle {
        match data {
            MarkData::Selection => MarkStyle {
                color: self.theme.color_with_alpha("selection", 64),
            },
            MarkData::Composition => MarkStyle {
                color: self.theme.color("ui.text.default"),
            },
        }
    }

    fn draw_marks(
        &self,
        sink: &mut dyn RenderSink,
        marks: &[Mark],
        layer: MarkLayer,
        page_idx: usize,
        scale_factor: f32,
    ) {
        let transform = Transform::scale(scale_factor);
        for mark in marks {
            if mark.data.layer() != layer {
                continue;
            }
            let style = self.resolve_mark(&mark.data);
            for rect in &mark.rects {
                if rect.page_idx != page_idx {
                    continue;
                }
                sink.fill_rect(rect.rect, style.color, transform);
            }
        }
    }

    pub fn render_page(
        &mut self,
        sink: &mut dyn RenderSink,
        doc: &Doc,
        view: &editor_view::View,
        page_idx: usize,
        scale_factor: f32,
        marks: &[Mark],
    ) {
        view.visit_page(
            page_idx,
            &mut self.page_visitor(
                sink,
                doc,
                scale_factor,
                LayerSet::of(&[RenderLayer::Background]),
            ),
        );

        self.draw_marks(sink, marks, MarkLayer::BelowContent, page_idx, scale_factor);

        view.visit_page(
            page_idx,
            &mut self.page_visitor(
                sink,
                doc,
                scale_factor,
                LayerSet::of(&[RenderLayer::Content, RenderLayer::Border]),
            ),
        );

        self.draw_marks(sink, marks, MarkLayer::AboveContent, page_idx, scale_factor);
    }

    pub fn page_visitor<'a>(
        &'a mut self,
        sink: &'a mut dyn RenderSink,
        doc: &'a Doc,
        scale_factor: f32,
        active: LayerSet,
    ) -> RenderVisitor<'a> {
        RenderVisitor {
            renderer: self,
            sink,
            doc,
            scale_factor,
            root_transform: Transform::scale(scale_factor),
            box_stack: Vec::new(),
            active,
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
    doc: &'a Doc,
    scale_factor: f32,
    root_transform: Transform,
    box_stack: Vec<BoxFrame>,
    active: LayerSet,
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
            let resource = Arc::clone(&self.renderer.resource);
            let resource_guard = resource.lock().unwrap();
            let positioned = crate::glyph::rasterize(
                run,
                &resource_guard.font_registry,
                &mut self.renderer.scale_ctx,
                &mut self.renderer.glyph_cache,
                self.scale_factor,
                base_transform,
            );
            drop(resource_guard);

            for pg in &positioned {
                let image = match pg.raster.content {
                    Content::Mask => bake_mask_to_premul_rgba(
                        &pg.raster.data,
                        pg.raster.width,
                        pg.raster.height,
                        color,
                    ),
                    Content::Color => Image {
                        data: pg.raster.data.clone(),
                        width: pg.raster.width,
                        height: pg.raster.height,
                    },
                };

                let t = Transform::IDENTITY.translate(pg.blit_x as f32, pg.blit_y as f32);
                let rect = Rect::from_xywh(0.0, 0.0, image.width as f32, image.height as f32);
                self.sink.draw_image(&image, rect, t);
            }
        }
    }
}

impl<'a> PageVisitor for RenderVisitor<'a> {
    fn box_enter(
        &mut self,
        node_id: NodeId,
        local_rect: Rect,
        style: &BoxStyle,
        edges: Edges<bool>,
    ) {
        let node = self.doc.node(node_id).map(|n| n.node().clone());

        if self.on(RenderLayer::Background) {
            let t = self.root_transform.translate(local_rect.x, local_rect.y);
            let inner_rect = Rect::from_xywh(0.0, 0.0, local_rect.width, local_rect.height);

            match &node {
                Some(Node::Callout(callout)) => {
                    let token = callout_token(callout.variant);
                    let color = self.renderer.theme.color_with_alpha(token, 8);
                    let radii = CornerRadii::from_edges(CALLOUT_BORDER_RADIUS, &edges);
                    let path = Path::rrect(inner_rect, radii);
                    self.sink.fill_path(&path, color, t);
                }
                Some(Node::Fold(_)) => {
                    let color = self.renderer.theme.color("ui.surface.muted");
                    self.sink.fill_rect(inner_rect, color, t);
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

    fn box_exit(&mut self) {
        let Some(frame) = self.box_stack.pop() else {
            return;
        };

        if !self.on(RenderLayer::Border) {
            return;
        }

        let t = self
            .root_transform
            .translate(frame.local_rect.x, frame.local_rect.y);

        if let Some(Node::Callout(callout)) = &frame.node {
            let token = callout_token(callout.variant);
            let border_color = self.renderer.theme.color(token);
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

        let b = &frame.border;

        let border_color = match &frame.node {
            Some(Node::Blockquote(_)) => self.renderer.theme.color("ui.border.default"),
            Some(Node::Fold(_)) => self.renderer.theme.color("ui.border.default"),
            Some(Node::Table(_)) => self.renderer.theme.color("ui.border.default"),
            _ => self.renderer.theme.color("ui.border"),
        };

        if frame.edges.left && b.left > 0.0 {
            let path = Path::rect(Rect::from_xywh(0.0, 0.0, b.left, frame.local_rect.height));
            self.sink.fill_path(&path, border_color, t);
        }

        if frame.edges.right && b.right > 0.0 {
            let path = Path::rect(Rect::from_xywh(
                frame.local_rect.width - b.right,
                0.0,
                b.right,
                frame.local_rect.height,
            ));
            self.sink.fill_path(&path, border_color, t);
        }

        if frame.edges.top && b.top > 0.0 {
            let path = Path::rect(Rect::from_xywh(0.0, 0.0, frame.local_rect.width, b.top));
            self.sink.fill_path(&path, border_color, t);
        }

        if frame.edges.bottom && b.bottom > 0.0 {
            let path = Path::rect(Rect::from_xywh(
                0.0,
                frame.local_rect.height - b.bottom,
                frame.local_rect.width,
                b.bottom,
            ));
            self.sink.fill_path(&path, border_color, t);
        }
    }

    fn line(
        &mut self,
        _node_id: NodeId,
        local_rect: Rect,
        _baseline: f32,
        glyph_runs: &[editor_view::glyph_run::GlyphRun],
    ) {
        if !self.on(RenderLayer::Content) {
            return;
        }

        let t = self.root_transform.translate(local_rect.x, local_rect.y);

        for run in glyph_runs {
            if let Some(ref bg_token) = run.background_color {
                let bg_color = self.renderer.theme.color(bg_token);
                let run_rect = Rect::from_xywh(run.x, 0.0, run.width, local_rect.height);
                self.sink.fill_rect(run_rect, bg_color, t);
            }
        }

        for run in glyph_runs {
            let color = self.renderer.theme.color(&run.color);
            self.render_glyph_runs(std::slice::from_ref(run), color, t);
        }
    }

    fn atom(&mut self, node_id: NodeId, local_rect: Rect) {
        if !self.on(RenderLayer::Content) {
            return;
        }

        let t = self.root_transform.translate(local_rect.x, local_rect.y);
        let inner_rect = Rect::from_xywh(0.0, 0.0, local_rect.width, local_rect.height);

        let node = self.doc.node(node_id);

        match node.map(|n| n.node()) {
            Some(Node::HorizontalRule(hr)) => {
                let color = self.renderer.theme.color("ui.text.default");
                let w = inner_rect.width;
                let h = inner_rect.height;
                let cx = w / 2.0;
                let cy = h / 2.0;

                match hr.variant {
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

    fn decoration(&mut self, local_rect: Rect, data: &DecorationData) {
        if !self.on(RenderLayer::Content) {
            return;
        }

        let t = self.root_transform.translate(local_rect.x, local_rect.y);
        let inner_rect = Rect::from_xywh(0.0, 0.0, local_rect.width, local_rect.height);

        let parent_node = self.box_stack.last().and_then(|f| f.node.as_ref());

        match (parent_node, data) {
            (Some(Node::Callout(callout)), _) => {
                let icon_name = match callout.variant {
                    editor_model::CalloutVariant::Info => "lucide/info",
                    editor_model::CalloutVariant::Success => "lucide/circle-check",
                    editor_model::CalloutVariant::Warning => "lucide/circle-alert",
                    editor_model::CalloutVariant::Danger => "lucide/triangle-alert",
                };
                let color = self.renderer.theme.color(callout_token(callout.variant));
                if let Some(icon) = ICONS.resolve(icon_name) {
                    self.render_icon(icon, color, inner_rect, t, ICON_STROKE_WIDTH);
                }
            }

            (Some(Node::Blockquote(bq)), _)
                if bq.variant == editor_model::BlockquoteVariant::LeftQuote =>
            {
                let color = self.renderer.theme.color("ui.text.muted");
                if let Some(icon) = ICONS.resolve("typie/blockquote-quote") {
                    self.render_icon(icon, color, inner_rect, t, ICON_STROKE_WIDTH);
                }
            }

            (Some(Node::Blockquote(bq)), _)
                if bq.variant == editor_model::BlockquoteVariant::LeftLine =>
            {
                let color = self.renderer.theme.color("ui.border.default");
                self.sink.fill_rect(inner_rect, color, t);
            }

            (Some(Node::FoldTitle(_)), _) => {
                let expanded = matches!(data, DecorationData::Bool(true));
                let icon_name = if expanded {
                    "lucide/chevron-up"
                } else {
                    "lucide/chevron-down"
                };
                let color = self.renderer.theme.color("ui.text.muted");
                if let Some(icon) = ICONS.resolve(icon_name) {
                    self.render_icon(icon, color, inner_rect, t, ICON_STROKE_WIDTH);
                }
            }

            (Some(Node::ListItem(_)), _) => match data {
                DecorationData::Glyphs(glyph_runs) => {
                    let color = self.renderer.theme.color("ui.text.default");
                    let total_width: f32 = glyph_runs.iter().map(|r| r.width).sum();
                    let x_offset = inner_rect.width - total_width;
                    let offset_t = t.translate(x_offset, 0.0);
                    self.render_glyph_runs(glyph_runs, color, offset_t);
                }
                _ => {
                    let color = self.renderer.theme.color("ui.text");
                    self.sink.fill_rect(inner_rect, color, t);
                }
            },

            (Some(Node::BulletList(_)), _) => {
                let color = self.renderer.theme.color("ui.text");
                self.sink.fill_rect(inner_rect, color, t);
            }

            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(MarkData::Selection.layer(), MarkLayer::BelowContent);
    }

    #[test]
    fn mark_data_layer_above_content() {
        assert_eq!(MarkData::Composition.layer(), MarkLayer::AboveContent);
    }
}
