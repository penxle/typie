use editor_common::Rect;
use editor_model::{Doc, Node, NodeId};
use editor_resource::Resource;
use editor_view::style::{BoxStyle, DecorationData};
use editor_view::{CompositionRect, Edges, PageRect, PageVisitor, SelectionRect};
use std::sync::{Arc, Mutex};

use crate::glyph::{GlyphCache, ScaleContext};
use crate::icons::ICONS;
use crate::sink::RenderSink;
use crate::theme::Theme;
use crate::theme_data::ThemeVariant;
use crate::types::{Color, CornerRadii, Path, Stroke, Transform};

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

fn build_partial_border(r: Rect, radii: CornerRadii, edges: &Edges<bool>) -> Path {
    use crate::types::PathElement;

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

    pub fn page_visitor<'a>(
        &'a mut self,
        sink: &'a mut dyn RenderSink,
        doc: &'a Doc,
        scale_factor: f32,
    ) -> RenderVisitor<'a> {
        RenderVisitor {
            renderer: self,
            sink,
            doc,
            scale_factor,
            root_transform: Transform::scale(scale_factor),
            box_stack: Vec::new(),
        }
    }

    pub fn draw_selection(
        &self,
        sink: &mut dyn RenderSink,
        rects: &[SelectionRect],
        page_idx: usize,
        scale_factor: f32,
    ) {
        let color = self.theme.color_with_alpha("selection", 64);
        self.draw_page_rects(sink, rects, page_idx, scale_factor, color);
    }

    pub fn draw_composition(
        &self,
        sink: &mut dyn RenderSink,
        rects: &[CompositionRect],
        page_idx: usize,
        scale_factor: f32,
    ) {
        let color = self.theme.color("ui.text.default");
        self.draw_page_rects(sink, rects, page_idx, scale_factor, color);
    }

    fn draw_page_rects<T>(
        &self,
        sink: &mut dyn RenderSink,
        rects: &[PageRect<T>],
        page_idx: usize,
        scale_factor: f32,
        color: Color,
    ) {
        let transform = Transform::scale(scale_factor);
        for rect in rects {
            if rect.page_idx != page_idx {
                continue;
            }
            sink.fill_rect(rect.rect, color, transform);
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
}

impl<'a> PageVisitor for RenderVisitor<'a> {
    fn box_enter(
        &mut self,
        node_id: NodeId,
        local_rect: Rect,
        style: &BoxStyle,
        edges: Edges<bool>,
    ) {
        let t = self.root_transform.translate(local_rect.x, local_rect.y);
        let inner_rect = Rect::from_xywh(0.0, 0.0, local_rect.width, local_rect.height);

        let node = self.doc.node(node_id).map(|n| n.node().clone());

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
        let t = self.root_transform.translate(local_rect.x, local_rect.y);
        let inv_scale = 1.0 / self.scale_factor;

        for run in glyph_runs {
            if let Some(ref bg_token) = run.background_color {
                let bg_color = self.renderer.theme.color(bg_token);
                let run_rect = Rect::from_xywh(run.x, 0.0, run.width, local_rect.height);
                self.sink.fill_rect(run_rect, bg_color, t);
            }

            let color = self.renderer.theme.color(&run.color);

            let resource = Arc::clone(&self.renderer.resource);
            let resource_guard = resource.lock().unwrap();
            let positioned = crate::glyph::rasterize(
                run,
                &resource_guard.font_registry,
                &mut self.renderer.scale_ctx,
                &mut self.renderer.glyph_cache,
                self.scale_factor,
            );
            drop(resource_guard);

            for pg in &positioned {
                let gt = t.translate(pg.x, pg.y).post_scale(inv_scale);
                match &pg.raster {
                    crate::glyph::RasterizedGlyph::Path(path) => {
                        self.sink.fill_path(path, color, gt);
                    }
                    crate::glyph::RasterizedGlyph::Bitmap(image) => {
                        let rect =
                            Rect::from_xywh(0.0, 0.0, image.width as f32, image.height as f32);
                        self.sink.draw_image(image, rect, gt);
                    }
                }
            }
        }
    }

    fn atom(&mut self, node_id: NodeId, local_rect: Rect) {
        let t = self.root_transform.translate(local_rect.x, local_rect.y);
        let inner_rect = Rect::from_xywh(0.0, 0.0, local_rect.width, local_rect.height);

        let node = self.doc.node(node_id);

        match node.map(|n| n.node()) {
            Some(Node::HorizontalRule(hr)) => {
                let color = self.renderer.theme.color("ui.border");
                let icon = match hr.variant {
                    editor_model::HorizontalRuleVariant::Line => "hr/line",
                    editor_model::HorizontalRuleVariant::DashedLine => "hr/dashed-line",
                    editor_model::HorizontalRuleVariant::Circle => "hr/circle",
                    editor_model::HorizontalRuleVariant::Diamond => "hr/diamond",
                    editor_model::HorizontalRuleVariant::ThreeCircles => "hr/three-circles",
                    editor_model::HorizontalRuleVariant::ThreeDiamonds => "hr/three-diamonds",
                    editor_model::HorizontalRuleVariant::Zigzag => "hr/zigzag",
                    editor_model::HorizontalRuleVariant::CircleLine => "hr/circle-line",
                    editor_model::HorizontalRuleVariant::DiamondLine => "hr/diamond-line",
                };
                let path = ICONS.resolve(icon, inner_rect);
                self.sink.fill_path(&path, color, t);
            }
            Some(Node::Image(_) | Node::File(_) | Node::Embed(_) | Node::Archived(_)) => {}
            _ => {}
        }
    }

    fn decoration(&mut self, local_rect: Rect, data: &DecorationData) {
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
                let path = ICONS.resolve(icon_name, inner_rect);
                self.sink.fill_path(&path, color, t);
            }

            (Some(Node::Blockquote(bq)), _)
                if bq.variant == editor_model::BlockquoteVariant::LeftQuote =>
            {
                let color = self.renderer.theme.color("ui.text.muted");
                let path = ICONS.resolve("typie/blockquote-quote", inner_rect);
                self.sink.fill_path(&path, color, t);
            }

            (Some(Node::Blockquote(bq)), _)
                if bq.variant == editor_model::BlockquoteVariant::LeftLine =>
            {
                let color = self.renderer.theme.color("ui.border.default");
                self.sink.fill_rect(inner_rect, color, t);
            }

            (Some(Node::Fold(_)), _) => {
                let expanded = matches!(data, DecorationData::Bool(true));
                let icon_name = if expanded {
                    "lucide/chevron-up"
                } else {
                    "lucide/chevron-down"
                };
                let color = self.renderer.theme.color("ui.text.muted");
                let path = ICONS.resolve(icon_name, inner_rect);
                self.sink.fill_path(&path, color, t);
            }

            (Some(Node::ListItem(_)), _) => match data {
                DecorationData::Text(_label) => {
                    let color = self.renderer.theme.color("ui.text.muted");
                    let path = ICONS.resolve("list/ordered", inner_rect);
                    self.sink.fill_path(&path, color, t);
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
