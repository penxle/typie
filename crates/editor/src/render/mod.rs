mod cache;
mod debug_overlay;
mod geometry;
pub mod glyph;
mod impls;
pub mod outline;
mod paint_diagnostics;
mod vector_codec;

pub use glyph::GlyphRenderer;
pub use outline::{ElementSink, RasterSink, VectorPage, VectorSink};
pub use vector_codec::encode_vector_page;

use crate::diagnostics::FrameDiagnostics;
use crate::layout::query::{DragImageBounds, DragImagePageBounds};
use crate::layout::{Element, Page, PositionedNode, RenderHints};
use crate::model::{Doc, LayoutMode, SelectionDecor};
use crate::runtime::DropIndicator;
use crate::types::{Point, Theme};
use cache::{PageRenderCache, PageRenderSnapshot, node_paint_bounds, same_scale_factor};
use debug_overlay::render_debug_overlay;
use geometry::{CacheRect, PixelRect, clear_layout_rect, merge_and_clamp_rects};
use paint_diagnostics::{PaintDebugFrame, PaintDiagnosticsState, collect_layout_dirty_rects};
use rustc_hash::FxHashMap;
use tiny_skia::{Color, Pixmap, PixmapMut, PixmapPaint, Rect, Transform};

const DIRTY_RECT_EPSILON: f32 = 0.5;
const DIRTY_RECT_COALESCE_EPSILON: f32 = 8.0;
const FULL_REPAINT_COVERAGE_THRESHOLD: f32 = 0.7;
const FULL_REPAINT_RECT_THRESHOLD: usize = 32;
const PAGE_EDGE_OVERFLOW_BAND: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverflowDirection {
    Upward,
    Downward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderPhase {
    Background,
    Content,
    Selection,
}

pub struct RenderContext<'a> {
    pub scale_factor: f64,
    pub selections: &'a [SelectionDecor],
    pub theme: &'a Theme,
    pub doc: &'a Doc,
    pub default_text_color: Option<Color>,
    pub is_focused: bool,
    pub phase: RenderPhase,
    pub render_origin: Point,
}

pub trait Render {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext<'_>,
    );
}

pub trait Outline {
    fn outline(&self, sink: &mut dyn ElementSink, transform: Transform, ctx: &RenderContext<'_>);
}

pub struct RenderResult {
    pub ptr: *const u8,
    pub len: usize,
    pub width: u16,
    pub height: u16,
}

#[allow(dead_code)]
pub struct RenderInfo {
    pub width: u16,
    pub height: u16,
    pub buffer_size: usize,
}

pub struct DragImageResult {
    pixmap: Pixmap,
    pub width: u16,
    pub height: u16,
    pub offset_x: f32,
    pub offset_y: f32,
    pub scale_factor: f32,
}

impl DragImageResult {
    pub fn ptr(&self) -> *const u8 {
        self.pixmap.data().as_ptr()
    }

    pub fn len(&self) -> usize {
        self.pixmap.data().len()
    }
}

pub struct Renderer {
    scale_factor: f64,
    pixmap: Pixmap,
    glyph_renderer: GlyphRenderer,
    theme: Theme,
    is_focused: bool,
    page_cache: FxHashMap<usize, PageRenderCache>,
    render_debug_enabled: bool,
    layout_debug_enabled: bool,
    paint_diagnostics: PaintDiagnosticsState,
    diagnostics: FrameDiagnostics,
}

impl Renderer {
    pub fn new(scale_factor: f64, diagnostics: FrameDiagnostics) -> Self {
        let pixmap = Pixmap::new(1, 1).unwrap();

        Self {
            scale_factor,
            pixmap,
            glyph_renderer: GlyphRenderer::new(),
            theme: Theme::default(),
            is_focused: true,
            page_cache: FxHashMap::default(),
            render_debug_enabled: false,
            layout_debug_enabled: false,
            paint_diagnostics: PaintDiagnosticsState::default(),
            diagnostics,
        }
    }

    pub fn set_size(&mut self, width: f32, height: f32, scale_factor: f64) {
        let new_width = (width as f64 * scale_factor).round() as u32;
        let new_height = (height as f64 * scale_factor).round() as u32;
        let scale_changed = !same_scale_factor(self.scale_factor, scale_factor);

        if self.pixmap.width() != new_width || self.pixmap.height() != new_height {
            if let Some(new_pixmap) = Pixmap::new(new_width.max(1), new_height.max(1)) {
                self.pixmap = new_pixmap;
            }
        }
        self.scale_factor = scale_factor;
        if scale_changed {
            self.page_cache.clear();
            self.paint_diagnostics.clear();
        }
    }

    pub fn set_theme(&mut self, theme: Theme) {
        if self.theme != theme {
            self.theme = theme;
            self.page_cache.clear();
            self.paint_diagnostics.clear();
        }
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.is_focused = focused;
    }

    pub fn set_render_debug(&mut self, enabled: bool) {
        self.render_debug_enabled = enabled;
    }

    pub fn set_layout_debug(&mut self, enabled: bool) {
        self.layout_debug_enabled = enabled;
    }

    pub fn prune_page_cache(&mut self, valid_page_count: usize) {
        self.page_cache
            .retain(|page_idx, _| *page_idx < valid_page_count);
        self.paint_diagnostics.retain_pages(valid_page_count);
    }

    pub fn width(&self) -> u16 {
        self.pixmap.width() as u16
    }

    pub fn height(&self) -> u16 {
        self.pixmap.height() as u16
    }

    pub fn render(
        &mut self,
        page: &Page,
        page_idx: usize,
        prev_page: Option<&Page>,
        next_page: Option<&Page>,
        selections: &[SelectionDecor],
        drop_indicator: Option<&DropIndicator>,
        doc: &Doc,
    ) -> RenderResult {
        let mut debug_frame = self.prepare_base_layer(page, page_idx, doc);
        if let Some(cache) = self.page_cache.get(&page_idx) {
            self.pixmap
                .data_mut()
                .copy_from_slice(cache.base_pixmap.data());
        } else {
            self.pixmap.data_mut().fill(0);
        }

        let mut pixmap = self.pixmap.as_mut();
        Self::render_overlay_layers(
            &mut pixmap,
            &mut self.glyph_renderer,
            self.scale_factor,
            &self.theme,
            self.is_focused,
            self.render_debug_enabled,
            self.layout_debug_enabled,
            page,
            page_idx,
            prev_page,
            next_page,
            selections,
            drop_indicator,
            doc,
            &mut debug_frame,
        );

        let data = self.pixmap.data();
        RenderResult {
            ptr: data.as_ptr(),
            len: data.len(),
            width: self.width(),
            height: self.height(),
        }
    }

    #[allow(dead_code)]
    pub fn render_to(
        &mut self,
        page: &Page,
        page_idx: usize,
        prev_page: Option<&Page>,
        next_page: Option<&Page>,
        selections: &[SelectionDecor],
        drop_indicator: Option<&DropIndicator>,
        doc: &Doc,
        dst: &mut [u8],
    ) -> bool {
        let expected_size = self.pixmap.width() as usize * self.pixmap.height() as usize * 4;
        if dst.len() < expected_size {
            return false;
        }

        let Some(mut pixmap) =
            PixmapMut::from_bytes(dst, self.pixmap.width(), self.pixmap.height())
        else {
            return false;
        };

        let mut debug_frame = self.prepare_base_layer(page, page_idx, doc);
        if let Some(cache) = self.page_cache.get(&page_idx) {
            pixmap.data_mut().copy_from_slice(cache.base_pixmap.data());
        } else {
            pixmap.data_mut().fill(0);
        }

        Self::render_overlay_layers(
            &mut pixmap,
            &mut self.glyph_renderer,
            self.scale_factor,
            &self.theme,
            self.is_focused,
            self.render_debug_enabled,
            self.layout_debug_enabled,
            page,
            page_idx,
            prev_page,
            next_page,
            selections,
            drop_indicator,
            doc,
            &mut debug_frame,
        );

        true
    }

    pub fn export_page_vector(
        &mut self,
        page: &Page,
        prev_page: Option<&Page>,
        next_page: Option<&Page>,
        doc: &Doc,
        page_width: f32,
        page_height: f32,
    ) -> VectorPage {
        let mut sink = VectorSink::new();

        for phase in [RenderPhase::Background, RenderPhase::Content] {
            let ctx = RenderContext {
                scale_factor: 1.0,
                selections: &[],
                theme: &self.theme,
                doc,
                default_text_color: None,
                is_focused: self.is_focused,
                phase,
                render_origin: Point::zero(),
            };

            Self::outline_node(
                &mut sink,
                &page.root,
                Point::zero(),
                Transform::identity(),
                &ctx,
                &RenderHints::default(),
                None,
            );
        }

        if let Some(prev_page) = prev_page
            && let Some(cull_clip) = prev_page_overflow_cull_clip(page_width)
        {
            let ctx = RenderContext {
                scale_factor: 1.0,
                selections: &[],
                theme: &self.theme,
                doc,
                default_text_color: None,
                is_focused: self.is_focused,
                phase: RenderPhase::Content,
                render_origin: Point::zero(),
            };
            Self::outline_node_for_page_boundary_overflow(
                &mut sink,
                &prev_page.root,
                Point::new(0.0, -page_height),
                Transform::identity(),
                &ctx,
                &RenderHints::default(),
                cull_clip,
                OverflowDirection::Downward,
            );
        }

        if let Some(next_page) = next_page
            && let Some(cull_clip) = next_page_overflow_cull_clip(page_width, page_height)
        {
            let ctx = RenderContext {
                scale_factor: 1.0,
                selections: &[],
                theme: &self.theme,
                doc,
                default_text_color: None,
                is_focused: self.is_focused,
                phase: RenderPhase::Content,
                render_origin: Point::zero(),
            };
            Self::outline_node_for_page_boundary_overflow(
                &mut sink,
                &next_page.root,
                Point::new(0.0, page_height),
                Transform::identity(),
                &ctx,
                &RenderHints::default(),
                cull_clip,
                OverflowDirection::Upward,
            );
        }

        let (ops, text_ops) = sink.into_parts();
        VectorPage {
            width: page_width,
            height: page_height,
            ops,
            text_ops,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_overlay_layers(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        scale_factor: f64,
        theme: &Theme,
        is_focused: bool,
        render_debug_enabled: bool,
        layout_debug_enabled: bool,
        page: &Page,
        page_idx: usize,
        prev_page: Option<&Page>,
        next_page: Option<&Page>,
        selections: &[SelectionDecor],
        drop_indicator: Option<&DropIndicator>,
        doc: &Doc,
        debug_frame: &mut Option<PaintDebugFrame>,
    ) {
        if debug_frame.is_some() {
            let mut overflow_rects = Vec::new();
            if let Some(prev_page) = prev_page {
                Self::render_prev_page_overflow(
                    pixmap,
                    glyph_renderer,
                    scale_factor,
                    theme,
                    prev_page,
                    doc,
                    Some(&mut overflow_rects),
                );
            }
            if let Some(next_page) = next_page {
                Self::render_next_page_overflow(
                    pixmap,
                    glyph_renderer,
                    scale_factor,
                    theme,
                    next_page,
                    doc,
                    Some(&mut overflow_rects),
                );
            }
            if let Some(frame) = debug_frame.as_mut() {
                frame.overflow_rects = overflow_rects;
            }
        } else {
            if let Some(prev_page) = prev_page {
                Self::render_prev_page_overflow(
                    pixmap,
                    glyph_renderer,
                    scale_factor,
                    theme,
                    prev_page,
                    doc,
                    None,
                );
            }
            if let Some(next_page) = next_page {
                Self::render_next_page_overflow(
                    pixmap,
                    glyph_renderer,
                    scale_factor,
                    theme,
                    next_page,
                    doc,
                    None,
                );
            }
        }

        Self::render_selection_overlay(
            pixmap,
            glyph_renderer,
            scale_factor,
            theme,
            is_focused,
            page,
            page_idx,
            selections,
            drop_indicator,
            doc,
        );

        if let Some(frame) = debug_frame.as_ref() {
            render_debug_overlay(
                pixmap,
                scale_factor,
                frame,
                render_debug_enabled,
                layout_debug_enabled,
            );
        }
    }

    fn prepare_base_layer(
        &mut self,
        page: &Page,
        page_idx: usize,
        doc: &Doc,
    ) -> Option<PaintDebugFrame> {
        let mut debug_frame =
            (self.render_debug_enabled || self.layout_debug_enabled).then(PaintDebugFrame::default);
        let width = self.pixmap.width();
        let height = self.pixmap.height();
        let scale = self.scale_factor as f32;
        let canvas_width = width as f32 / scale;
        let canvas_height = height as f32 / scale;
        let render_snapshot = PageRenderSnapshot::from_page(page);

        let previous_cache = self.page_cache.remove(&page_idx);
        let mut resize_dirty_rects = Vec::new();
        let mut cache = match previous_cache {
            Some(entry) if entry.matches(width, height, self.scale_factor) => entry,
            Some(entry) if entry.matches_for_height_resize(width, self.scale_factor) => {
                resize_dirty_rects = entry.exposed_rects_on_resize(width, height, scale);
                entry.resize_preserving_overlap(width, height, self.scale_factor)
            }
            Some(_) | None => PageRenderCache::new(width, height, self.scale_factor),
        };

        let mut dirty_rects = if !cache.snapshot_initialized {
            CacheRect::from_canvas(canvas_width, canvas_height)
                .map(|rect| vec![rect])
                .unwrap_or_default()
        } else {
            cache.snapshot.dirty_rects(&render_snapshot)
        };
        dirty_rects.extend(resize_dirty_rects);
        dirty_rects = normalize_dirty_rects(dirty_rects, canvas_width, canvas_height);

        if !dirty_rects.is_empty() {
            let should_full_repaint =
                should_promote_full_repaint(&dirty_rects, canvas_width, canvas_height);
            let render_rects = if should_full_repaint {
                CacheRect::from_canvas(canvas_width, canvas_height)
                    .map(|rect| vec![rect])
                    .unwrap_or_default()
            } else {
                dirty_rects
            };
            if let Some(frame) = debug_frame.as_mut() {
                frame.render_rects = render_rects.clone();
                frame.full_repaint = should_full_repaint;
                frame.cache_reused = false;
            }

            if should_full_repaint {
                cache.base_pixmap.data_mut().fill(0);
                let mut cache_pixmap = cache.base_pixmap.as_mut();
                self.render_base_phases(&mut cache_pixmap, page, doc, None, Point::zero());
            } else {
                for rect in &render_rects {
                    clear_layout_rect(&mut cache.base_pixmap, *rect, scale);
                    self.render_base_phases_clipped(&mut cache.base_pixmap, page, doc, *rect);
                }
            }
        } else if let Some(frame) = debug_frame.as_mut() {
            frame.cache_reused = true;
        }

        if self.layout_debug_enabled {
            if let Some(layout_pass) = self.diagnostics.layout_pass_snapshot() {
                let revision = layout_pass.revision;
                let mut layout_rects = if self
                    .paint_diagnostics
                    .is_layout_revision_reused(page_idx, revision)
                {
                    Vec::new()
                } else {
                    collect_layout_dirty_rects(page, layout_pass.recomputed_nodes.as_ref())
                };
                layout_rects = normalize_dirty_rects(layout_rects, canvas_width, canvas_height);
                let should_full_relayout =
                    should_promote_full_repaint(&layout_rects, canvas_width, canvas_height);
                let layout_rects = if should_full_relayout {
                    CacheRect::from_canvas(canvas_width, canvas_height)
                        .map(|rect| vec![rect])
                        .unwrap_or_default()
                } else {
                    layout_rects
                };

                if let Some(frame) = debug_frame.as_mut() {
                    frame.layout_rects = layout_rects;
                    frame.full_relayout = should_full_relayout;
                    frame.layout_reused = frame.layout_rects.is_empty();
                }

                self.paint_diagnostics
                    .mark_layout_revision(page_idx, revision);
            } else if let Some(frame) = debug_frame.as_mut() {
                frame.layout_reused = false;
            }
        }

        cache.snapshot = render_snapshot;
        cache.snapshot_initialized = true;
        self.page_cache.insert(page_idx, cache);
        debug_frame
    }

    fn render_base_phases(
        &mut self,
        pixmap: &mut PixmapMut,
        page: &Page,
        doc: &Doc,
        clip: Option<CacheRect>,
        origin: Point,
    ) {
        let scale = self.scale_factor as f32;
        let transform = Transform::from_scale(scale, scale).pre_translate(-origin.x, -origin.y);

        for phase in [RenderPhase::Background, RenderPhase::Content] {
            let ctx = RenderContext {
                scale_factor: self.scale_factor,
                selections: &[],
                theme: &self.theme,
                doc,
                default_text_color: None,
                is_focused: self.is_focused,
                phase,
                render_origin: origin,
            };

            Self::render_node(
                pixmap,
                &mut self.glyph_renderer,
                &page.root,
                Point::zero(),
                transform,
                &ctx,
                &RenderHints::default(),
                clip,
            );
        }
    }

    fn render_base_phases_clipped(
        &mut self,
        base_pixmap: &mut Pixmap,
        page: &Page,
        doc: &Doc,
        clip_rect: CacheRect,
    ) {
        let scale = self.scale_factor as f32;
        let Some(pixel_rect) = PixelRect::from_layout_rect(
            clip_rect,
            scale,
            base_pixmap.width(),
            base_pixmap.height(),
        ) else {
            return;
        };

        let clipped_layout_rect = pixel_rect.to_layout_rect(scale);
        let Some(mut tile_pixmap) = Pixmap::new(pixel_rect.width, pixel_rect.height) else {
            let mut base = base_pixmap.as_mut();
            self.render_base_phases(
                &mut base,
                page,
                doc,
                Some(clipped_layout_rect),
                Point::zero(),
            );
            return;
        };

        {
            let mut tile = tile_pixmap.as_mut();
            self.render_base_phases(
                &mut tile,
                page,
                doc,
                Some(clipped_layout_rect),
                Point::new(clipped_layout_rect.x, clipped_layout_rect.y),
            );
        }

        let paint = PixmapPaint::default();
        base_pixmap.draw_pixmap(
            pixel_rect.x as i32,
            pixel_rect.y as i32,
            tile_pixmap.as_ref(),
            &paint,
            Transform::identity(),
            None,
        );
    }

    fn render_selection_overlay(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        scale_factor: f64,
        theme: &Theme,
        is_focused: bool,
        page: &Page,
        page_idx: usize,
        selections: &[SelectionDecor],
        drop_indicator: Option<&DropIndicator>,
        doc: &Doc,
    ) {
        let scale = scale_factor as f32;
        let transform = Transform::from_scale(scale, scale);
        let selection_ctx = RenderContext {
            scale_factor,
            selections,
            theme,
            doc,
            default_text_color: None,
            is_focused,
            phase: RenderPhase::Selection,
            render_origin: Point::zero(),
        };

        Self::render_node(
            pixmap,
            glyph_renderer,
            &page.root,
            Point::zero(),
            transform,
            &selection_ctx,
            &RenderHints::default(),
            None,
        );

        if let Some(indicator) = drop_indicator {
            let overlay_ctx = RenderContext {
                phase: RenderPhase::Content,
                ..selection_ctx
            };
            Self::render_drop_indicator(pixmap, indicator, page_idx, transform, &overlay_ctx);
        }
    }

    fn render_drop_indicator(
        pixmap: &mut PixmapMut,
        indicator: &DropIndicator,
        current_page_idx: usize,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        let indicator_color = ctx.theme.color("ui.accent.brand.default");
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(indicator_color);
        paint.anti_alias = true;

        match indicator {
            DropIndicator::Inline {
                page_idx,
                x,
                y,
                height,
            } => {
                if *page_idx != current_page_idx {
                    return;
                }
                if let Some(rect) = Rect::from_xywh(*x, *y, 2.0, *height) {
                    pixmap.fill_rect(rect, &paint, transform, None);
                }
            }
            DropIndicator::Block {
                page_idx,
                x,
                y,
                width,
            } => {
                if *page_idx != current_page_idx {
                    return;
                }
                if let Some(rect) = Rect::from_xywh(*x, *y - 1.0, *width, 2.0) {
                    pixmap.fill_rect(rect, &paint, transform, None);
                }
            }
        }
    }

    fn render_node(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        positioned: &PositionedNode,
        offset: Point,
        transform: Transform,
        ctx: &RenderContext<'_>,
        inherited_hints: &RenderHints,
        clip: Option<CacheRect>,
    ) {
        let scale = transform.sy;
        let pos = Point::new(
            offset.x + positioned.position.x,
            ((offset.y + positioned.position.y) * scale).round() / scale,
        );

        if let Some(clip_rect) = clip {
            if let Some(node_rect) = node_paint_bounds(positioned, pos)
                && !node_rect.intersects(clip_rect)
            {
                return;
            }
        }

        let merged_hints = positioned.node.render_hints.merge(inherited_hints);

        let child_ctx_data = RenderContext {
            default_text_color: merged_hints
                .default_text_color
                .as_ref()
                .map(|color_key| ctx.theme.color(color_key))
                .or(ctx.default_text_color),
            ..*ctx
        };
        let render_ctx = &child_ctx_data;

        if let Some(ref element) = positioned.node.element
            && let Some(render) = element.as_render()
        {
            let element_transform = transform.pre_translate(pos.x, pos.y);
            render.render(pixmap, glyph_renderer, element_transform, render_ctx);
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::render_node(
                    pixmap,
                    glyph_renderer,
                    child,
                    pos,
                    transform,
                    render_ctx,
                    &merged_hints,
                    clip,
                );
            }
        }
    }

    fn outline_node(
        sink: &mut dyn ElementSink,
        positioned: &PositionedNode,
        offset: Point,
        transform: Transform,
        ctx: &RenderContext<'_>,
        inherited_hints: &RenderHints,
        clip: Option<CacheRect>,
    ) {
        let scale = transform.sy;
        let pos = Point::new(
            offset.x + positioned.position.x,
            ((offset.y + positioned.position.y) * scale).round() / scale,
        );

        if let Some(clip_rect) = clip {
            if let Some(node_rect) = node_paint_bounds(positioned, pos)
                && !node_rect.intersects(clip_rect)
            {
                return;
            }
        }

        let merged_hints = positioned.node.render_hints.merge(inherited_hints);

        let child_ctx_data = RenderContext {
            default_text_color: merged_hints
                .default_text_color
                .as_ref()
                .map(|color_key| ctx.theme.color(color_key))
                .or(ctx.default_text_color),
            ..*ctx
        };
        let render_ctx = &child_ctx_data;

        if let Some(ref element) = positioned.node.element
            && let Some(outline) = element.as_outline()
        {
            let element_transform = transform.pre_translate(pos.x, pos.y);
            outline.outline(sink, element_transform, render_ctx);
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::outline_node(sink, child, pos, transform, render_ctx, &merged_hints, clip);
            }
        }
    }

    fn should_render_page_boundary_overflow(
        positioned: &PositionedNode,
        direction: OverflowDirection,
    ) -> bool {
        positioned
            .node
            .element
            .as_ref()
            .is_some_and(|element| match direction {
                OverflowDirection::Upward => element.paint_overflow().top > 0.0,
                OverflowDirection::Downward => element.paint_overflow().bottom > 0.0,
            })
    }

    fn render_node_for_page_boundary_overflow(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        positioned: &PositionedNode,
        offset: Point,
        transform: Transform,
        ctx: &RenderContext<'_>,
        inherited_hints: &RenderHints,
        cull_clip: CacheRect,
        direction: OverflowDirection,
    ) {
        let scale = transform.sy;
        let pos = Point::new(
            offset.x + positioned.position.x,
            ((offset.y + positioned.position.y) * scale).round() / scale,
        );

        let node_rect = node_paint_bounds(positioned, pos);
        if let Some(node_rect) = node_rect
            && !node_rect.intersects(cull_clip)
        {
            return;
        }

        let merged_hints = positioned.node.render_hints.merge(inherited_hints);

        let child_ctx_data = RenderContext {
            default_text_color: merged_hints
                .default_text_color
                .as_ref()
                .map(|color_key| ctx.theme.color(color_key))
                .or(ctx.default_text_color),
            ..*ctx
        };
        let render_ctx = &child_ctx_data;

        if Self::should_render_page_boundary_overflow(positioned, direction)
            && let Some(element) = positioned.node.element.as_ref()
            && let Some(render) = element.as_render()
        {
            let element_transform = transform.pre_translate(pos.x, pos.y);
            render.render(pixmap, glyph_renderer, element_transform, render_ctx);
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::render_node_for_page_boundary_overflow(
                    pixmap,
                    glyph_renderer,
                    child,
                    pos,
                    transform,
                    render_ctx,
                    &merged_hints,
                    cull_clip,
                    direction,
                );
            }
        }
    }

    fn outline_node_for_page_boundary_overflow(
        sink: &mut dyn ElementSink,
        positioned: &PositionedNode,
        offset: Point,
        transform: Transform,
        ctx: &RenderContext<'_>,
        inherited_hints: &RenderHints,
        cull_clip: CacheRect,
        direction: OverflowDirection,
    ) {
        let scale = transform.sy;
        let pos = Point::new(
            offset.x + positioned.position.x,
            ((offset.y + positioned.position.y) * scale).round() / scale,
        );

        let node_rect = node_paint_bounds(positioned, pos);
        if let Some(node_rect) = node_rect
            && !node_rect.intersects(cull_clip)
        {
            return;
        }

        let merged_hints = positioned.node.render_hints.merge(inherited_hints);

        let child_ctx_data = RenderContext {
            default_text_color: merged_hints
                .default_text_color
                .as_ref()
                .map(|color_key| ctx.theme.color(color_key))
                .or(ctx.default_text_color),
            ..*ctx
        };
        let render_ctx = &child_ctx_data;

        if Self::should_render_page_boundary_overflow(positioned, direction)
            && let Some(element) = positioned.node.element.as_ref()
            && let Some(outline) = element.as_outline()
        {
            let element_transform = transform.pre_translate(pos.x, pos.y);
            outline.outline(sink, element_transform, render_ctx);
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::outline_node_for_page_boundary_overflow(
                    sink,
                    child,
                    pos,
                    transform,
                    render_ctx,
                    &merged_hints,
                    cull_clip,
                    direction,
                );
            }
        }
    }

    fn render_next_page_overflow(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        scale_factor: f64,
        theme: &Theme,
        next_page: &Page,
        doc: &Doc,
        debug_rects: Option<&mut Vec<CacheRect>>,
    ) {
        let scale = scale_factor as f32;
        let page_height = pixmap.height() as f32 / scale;
        let page_width = pixmap.width() as f32 / scale;
        let Some(cull_clip) = next_page_overflow_cull_clip(page_width, page_height) else {
            return;
        };
        Self::render_page_boundary_overflow_with_clip(
            pixmap,
            glyph_renderer,
            scale_factor,
            theme,
            next_page,
            doc,
            Point::new(0.0, page_height),
            cull_clip,
            OverflowDirection::Upward,
            debug_rects,
        );
    }

    fn render_prev_page_overflow(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        scale_factor: f64,
        theme: &Theme,
        prev_page: &Page,
        doc: &Doc,
        debug_rects: Option<&mut Vec<CacheRect>>,
    ) {
        let scale = scale_factor as f32;
        let page_width = pixmap.width() as f32 / scale;
        let Some(cull_clip) = prev_page_overflow_cull_clip(page_width) else {
            return;
        };
        let page_height = pixmap.height() as f32 / scale;
        Self::render_page_boundary_overflow_with_clip(
            pixmap,
            glyph_renderer,
            scale_factor,
            theme,
            prev_page,
            doc,
            Point::new(0.0, -page_height),
            cull_clip,
            OverflowDirection::Downward,
            debug_rects,
        );
    }

    fn render_page_boundary_overflow_with_clip(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        scale_factor: f64,
        theme: &Theme,
        page: &Page,
        doc: &Doc,
        page_offset: Point,
        cull_clip: CacheRect,
        direction: OverflowDirection,
        debug_rects: Option<&mut Vec<CacheRect>>,
    ) {
        let scale = scale_factor as f32;
        let page_width = pixmap.width() as f32 / scale;
        let page_height = pixmap.height() as f32 / scale;
        let Some(pixel_rect) =
            PixelRect::from_layout_rect(cull_clip, scale, pixmap.width(), pixmap.height())
        else {
            return;
        };
        let hard_clip_layout_rect = pixel_rect.to_layout_rect(scale);
        let Some(mut tile_pixmap) = Pixmap::new(pixel_rect.width, pixel_rect.height) else {
            return;
        };
        let ctx = RenderContext {
            scale_factor,
            selections: &[],
            theme,
            doc,
            default_text_color: None,
            is_focused: true,
            phase: RenderPhase::Content,
            render_origin: Point::zero(),
        };

        let transform = Transform::from_scale(scale, scale)
            .pre_translate(-hard_clip_layout_rect.x, -hard_clip_layout_rect.y);
        Self::render_node_for_page_boundary_overflow(
            &mut tile_pixmap.as_mut(),
            glyph_renderer,
            &page.root,
            page_offset,
            transform,
            &ctx,
            &RenderHints::default(),
            cull_clip,
            direction,
        );
        let paint = PixmapPaint::default();
        pixmap.draw_pixmap(
            pixel_rect.x as i32,
            pixel_rect.y as i32,
            tile_pixmap.as_ref(),
            &paint,
            Transform::identity(),
            None,
        );

        if let Some(debug_rects) = debug_rects {
            debug_rects.extend(Self::collect_page_boundary_overflow_debug_rects(
                page,
                page_offset,
                page_width,
                page_height,
                cull_clip,
                direction,
            ));
        }
    }

    fn collect_page_boundary_overflow_debug_rects(
        page: &Page,
        page_offset: Point,
        page_width: f32,
        page_height: f32,
        cull_clip: CacheRect,
        direction: OverflowDirection,
    ) -> Vec<CacheRect> {
        let mut rects = Vec::new();
        Self::collect_overflow_debug_rects_recursive(
            &page.root,
            page_offset,
            cull_clip,
            page_width,
            page_height,
            direction,
            &mut rects,
        );
        normalize_dirty_rects(rects, page_width, page_height)
    }

    fn collect_overflow_debug_rects_recursive(
        positioned: &PositionedNode,
        offset: Point,
        cull_clip: CacheRect,
        page_width: f32,
        page_height: f32,
        direction: OverflowDirection,
        out: &mut Vec<CacheRect>,
    ) {
        let pos = Point::new(
            offset.x + positioned.position.x,
            offset.y + positioned.position.y,
        );

        if let Some(node_rect) = node_paint_bounds(positioned, pos) {
            if !node_rect.intersects(cull_clip) {
                return;
            }

            if Self::should_render_page_boundary_overflow(positioned, direction) {
                let left = node_rect.x.max(cull_clip.x);
                let top = node_rect.y.max(cull_clip.y);
                let right = node_rect.right().min(cull_clip.right());
                let bottom = node_rect.bottom().min(cull_clip.bottom());

                if let Some(intersection) =
                    CacheRect::from_xywh(left, top, right - left, bottom - top)
                    && let Some(visible_rect) = intersection.clamp(page_width, page_height)
                {
                    out.push(visible_rect);
                }
            }
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::collect_overflow_debug_rects_recursive(
                    child,
                    pos,
                    cull_clip,
                    page_width,
                    page_height,
                    direction,
                    out,
                );
            }
        }
    }

    pub fn render_drag_image(
        &mut self,
        pages: &[Page],
        bounds: &DragImageBounds,
        selections: &[SelectionDecor],
        doc: &Doc,
        visible_pages: &[usize],
        drag_page_idx: usize,
    ) -> Option<DragImageResult> {
        let visible_bounds: Vec<_> = bounds
            .pages
            .iter()
            .filter(|pb| visible_pages.contains(&pb.page_idx))
            .collect();

        if visible_bounds.is_empty() {
            return None;
        }

        let scale = self.scale_factor as f32;
        let page_y_offsets = Self::compute_page_y_offsets(pages, doc);
        let (min_x, min_y, total_width, total_height) =
            Self::compute_global_bounds(&visible_bounds, &page_y_offsets);

        let pixel_width = ((total_width * scale).ceil() as u32).max(1);
        let pixel_height = ((total_height * scale).ceil() as u32).max(1);
        let mut drag_pixmap = Pixmap::new(pixel_width, pixel_height)?;

        for pb in &visible_bounds {
            let page = pages.get(pb.page_idx)?;
            let page_y = page_y_offsets[pb.page_idx];

            let ctx = RenderContext {
                scale_factor: self.scale_factor,
                selections: &[],
                theme: &self.theme,
                doc,
                default_text_color: None,
                is_focused: true,
                phase: RenderPhase::Content,
                render_origin: Point::new(pb.bounds.x, pb.bounds.y),
            };

            Self::render_page_part_inner(
                &mut self.glyph_renderer,
                page,
                pb,
                selections,
                page_y,
                min_x,
                min_y,
                scale,
                pixel_width,
                pixel_height,
                &ctx,
                &mut drag_pixmap,
            )?;
        }

        let drag_page_y = page_y_offsets.get(drag_page_idx).copied().unwrap_or(0.0);

        Some(DragImageResult {
            pixmap: drag_pixmap,
            width: pixel_width as u16,
            height: pixel_height as u16,
            offset_x: min_x,
            offset_y: min_y - drag_page_y,
            scale_factor: scale,
        })
    }

    fn compute_page_y_offsets(pages: &[Page], doc: &Doc) -> Vec<f32> {
        let settings = doc.settings();
        let gap = 24.0;
        let mut offsets = Vec::with_capacity(pages.len());
        let mut current_y = 0.0f32;

        for page in pages {
            offsets.push(current_y);
            let h = match settings.layout_mode {
                LayoutMode::Paginated { page_height, .. } => page_height,
                LayoutMode::Continuous { .. } => page.root.node.size.height,
            };
            current_y += h + gap;
        }
        offsets
    }

    fn compute_global_bounds(
        visible_bounds: &[&DragImagePageBounds],
        page_y_offsets: &[f32],
    ) -> (f32, f32, f32, f32) {
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for pb in visible_bounds {
            let page_y = page_y_offsets.get(pb.page_idx).copied().unwrap_or(0.0);
            let global_x = pb.bounds.x;
            let global_y = page_y + pb.bounds.y;

            min_x = min_x.min(global_x);
            min_y = min_y.min(global_y);
            max_x = max_x.max(global_x + pb.bounds.width);
            max_y = max_y.max(global_y + pb.bounds.height);
        }

        (min_x, min_y, max_x - min_x, max_y - min_y)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_page_part_inner(
        glyph_renderer: &mut GlyphRenderer,
        page: &Page,
        pb: &DragImagePageBounds,
        selections: &[SelectionDecor],
        page_y: f32,
        min_x: f32,
        min_y: f32,
        scale: f32,
        pixel_width: u32,
        pixel_height: u32,
        ctx: &RenderContext<'_>,
        drag_pixmap: &mut Pixmap,
    ) -> Option<()> {
        let dest_x = pb.bounds.x - min_x;
        let dest_y = (page_y + pb.bounds.y) - min_y;

        let part_pixel_w = ((pb.bounds.width * scale).ceil() as u32).max(1);
        let part_pixel_h = ((pb.bounds.height * scale).ceil() as u32).max(1);

        let mut temp_pixmap = Pixmap::new(part_pixel_w, part_pixel_h)?;
        let transform =
            Transform::from_scale(scale, scale).pre_translate(-pb.bounds.x, -pb.bounds.y);

        Self::render_node(
            &mut temp_pixmap.as_mut(),
            glyph_renderer,
            &page.root,
            Point::zero(),
            transform,
            ctx,
            &RenderHints::default(),
            None,
        );

        let mut clip_rects = Vec::new();
        Self::collect_clip_rects(
            &page.root,
            Point::zero(),
            selections,
            Point::new(pb.bounds.x, pb.bounds.y),
            scale,
            &mut clip_rects,
        );

        if clip_rects.is_empty() {
            for cr in &pb.clip_rects {
                if let Some(rect) = Rect::from_xywh(
                    (cr.x - pb.bounds.x) * scale,
                    (cr.y - pb.bounds.y) * scale,
                    cr.width * scale,
                    cr.height * scale,
                ) {
                    clip_rects.push(rect);
                }
            }
        }

        Self::copy_clipped_pixels(
            &temp_pixmap,
            drag_pixmap,
            &clip_rects,
            (dest_x * scale).round() as i32,
            (dest_y * scale).round() as i32,
            part_pixel_w,
            part_pixel_h,
            pixel_width,
            pixel_height,
        );

        Some(())
    }

    #[allow(clippy::too_many_arguments)]
    fn copy_clipped_pixels(
        src: &Pixmap,
        dest: &mut Pixmap,
        clip_rects: &[Rect],
        dest_base_x: i32,
        dest_base_y: i32,
        src_width: u32,
        src_height: u32,
        dest_width: u32,
        dest_height: u32,
    ) {
        let src_data = src.data();
        let dest_data = dest.data_mut();

        for rect in clip_rects {
            let x_start = rect.x().floor() as i32;
            let y_start = rect.y().floor() as i32;
            let x_end = rect.right().ceil() as i32;
            let y_end = rect.bottom().ceil() as i32;

            for y in y_start..y_end {
                for x in x_start..x_end {
                    if x >= 0 && y >= 0 && (x as u32) < src_width && (y as u32) < src_height {
                        let src_idx = (y as u32 * src_width + x as u32) as usize * 4;
                        let dest_px = dest_base_x + x;
                        let dest_py = dest_base_y + y;

                        if dest_px >= 0
                            && dest_py >= 0
                            && (dest_px as u32) < dest_width
                            && (dest_py as u32) < dest_height
                        {
                            let dest_idx =
                                (dest_py as u32 * dest_width + dest_px as u32) as usize * 4;
                            if src_idx + 3 < src_data.len() && dest_idx + 3 < dest_data.len() {
                                dest_data[dest_idx..dest_idx + 4]
                                    .copy_from_slice(&src_data[src_idx..src_idx + 4]);
                            }
                        }
                    }
                }
            }
        }
    }

    fn collect_clip_rects(
        positioned: &PositionedNode,
        offset: Point,
        selections: &[SelectionDecor],
        bounds_origin: Point,
        scale: f32,
        out: &mut Vec<Rect>,
    ) {
        let pos = Point::new(
            offset.x + positioned.position.x,
            offset.y + positioned.position.y,
        );

        if let Some(ref element) = positioned.node.element {
            match element {
                Element::Line(line) => {
                    let line_rects = line.compute_selection_rects(pos, selections);
                    for rect in line_rects {
                        if let Some(translated) = Rect::from_xywh(
                            (rect.x - bounds_origin.x) * scale,
                            (rect.y - bounds_origin.y) * scale,
                            rect.width * scale,
                            rect.height * scale,
                        ) {
                            out.push(translated);
                        }
                    }
                }
                _ => {
                    if let Some(block_id) = element.block_id() {
                        if selections.iter().any(|s| s.node_id() == block_id) {
                            let node_size = &positioned.node.size;
                            if let Some(translated) = Rect::from_xywh(
                                (pos.x - bounds_origin.x) * scale,
                                (pos.y - bounds_origin.y) * scale,
                                node_size.width * scale,
                                node_size.height * scale,
                            ) {
                                out.push(translated);
                            }
                        }
                    }
                }
            }
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::collect_clip_rects(child, pos, selections, bounds_origin, scale, out);
            }
        }
    }
}

fn normalize_dirty_rects(
    rects: Vec<CacheRect>,
    canvas_width: f32,
    canvas_height: f32,
) -> Vec<CacheRect> {
    let mut merged = merge_and_clamp_rects(rects, canvas_width, canvas_height, DIRTY_RECT_EPSILON);
    if merged.len() > FULL_REPAINT_RECT_THRESHOLD {
        merged = merge_and_clamp_rects(
            merged,
            canvas_width,
            canvas_height,
            DIRTY_RECT_COALESCE_EPSILON,
        );
    }
    merged
}

fn should_promote_full_repaint(rects: &[CacheRect], canvas_width: f32, canvas_height: f32) -> bool {
    let full_area = canvas_width * canvas_height;
    let dirty_area: f32 = rects.iter().map(|rect| rect.area()).sum();
    rects.len() > FULL_REPAINT_RECT_THRESHOLD
        || (full_area > 0.0 && dirty_area / full_area >= FULL_REPAINT_COVERAGE_THRESHOLD)
}

fn next_page_overflow_cull_clip(page_width: f32, page_height: f32) -> Option<CacheRect> {
    CacheRect::from_xywh(
        0.0,
        page_height - PAGE_EDGE_OVERFLOW_BAND,
        page_width,
        PAGE_EDGE_OVERFLOW_BAND * 2.0,
    )
}

fn prev_page_overflow_cull_clip(page_width: f32) -> Option<CacheRect> {
    CacheRect::from_xywh(
        0.0,
        -PAGE_EDGE_OVERFLOW_BAND,
        page_width,
        PAGE_EDGE_OVERFLOW_BAND * 2.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::LayoutPassRecorder;
    use crate::layout::elements::{
        CalloutBackgroundElement, CalloutIconElement, FoldContentElement, LineElement, LineMetric,
        ListMarkerElement, ListMarkerType, RubySegment, SplitEdges, TableBorderElement,
        TableCellElement,
    };
    use crate::layout::{LayoutNode, PageBreakPolicy};
    use crate::model::{CalloutVariant, NodeId, TableAlign, TableBorderStyle};
    use crate::types::Size;
    use rustc_hash::FxHashSet;
    use std::rc::Rc;

    fn root_with_children(children: Option<Vec<PositionedNode>>, size: Size) -> Page {
        Page::from_root(PositionedNode {
            position: Point::zero(),
            node: Rc::new(LayoutNode {
                size,
                element: None,
                children,
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: None,
            }),
        })
    }

    fn marker_node(size: Size) -> Rc<LayoutNode> {
        Rc::new(LayoutNode {
            size,
            element: Some(Element::ListMarker(ListMarkerElement::new(
                ListMarkerType::Bullet,
                8.0,
                6.0,
                size.width,
            ))),
            children: None,
            page_break_policy: PageBreakPolicy::default(),
            render_hints: RenderHints::default(),
            scope_id: None,
        })
    }

    fn callout_page_with_icon(callout_id: NodeId) -> Page {
        let icon_node = Rc::new(LayoutNode {
            size: Size::new(20.0, 20.0),
            element: Some(Element::CalloutIcon(CalloutIconElement::new(
                Size::new(20.0, 20.0),
                CalloutVariant::Info,
                callout_id,
            ))),
            children: None,
            page_break_policy: PageBreakPolicy::default(),
            render_hints: RenderHints::default(),
            scope_id: None,
        });

        let callout_node = Rc::new(LayoutNode {
            size: Size::new(140.0, 80.0),
            element: Some(Element::CalloutBackground(CalloutBackgroundElement::new(
                Size::new(140.0, 80.0),
                CalloutVariant::Info,
                callout_id,
                SplitEdges::default(),
            ))),
            children: Some(vec![PositionedNode {
                position: Point::new(12.0, 12.0),
                node: icon_node,
            }]),
            page_break_policy: PageBreakPolicy::default(),
            render_hints: RenderHints::default(),
            scope_id: None,
        });

        root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(20.0, 20.0),
                node: callout_node,
            }]),
            Size::new(220.0, 160.0),
        )
    }

    fn rgba_at(buf: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
        let idx = (y * width + x) * 4;
        [buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]]
    }

    fn collect_next_page_overflow_debug_rects(
        next_page: &Page,
        page_width: f32,
        page_height: f32,
        cull_clip: CacheRect,
    ) -> Vec<CacheRect> {
        Renderer::collect_page_boundary_overflow_debug_rects(
            next_page,
            Point::new(0.0, page_height),
            page_width,
            page_height,
            cull_clip,
            OverflowDirection::Upward,
        )
    }

    fn collect_prev_page_overflow_debug_rects(
        prev_page: &Page,
        page_width: f32,
        page_height: f32,
        cull_clip: CacheRect,
    ) -> Vec<CacheRect> {
        Renderer::collect_page_boundary_overflow_debug_rects(
            prev_page,
            Point::new(0.0, -page_height),
            page_width,
            page_height,
            cull_clip,
            OverflowDirection::Downward,
        )
    }

    #[test]
    fn snapshot_ignores_non_renderable_nodes() {
        let page1 = root_with_children(None, Size::new(300.0, 200.0));
        let page2 = root_with_children(None, Size::new(300.0, 200.0));

        let snapshot1 = PageRenderSnapshot::from_page(&page1);
        let snapshot2 = PageRenderSnapshot::from_page(&page2);

        assert!(
            snapshot1.dirty_rects(&snapshot2).is_empty(),
            "render 없는 루트 노드 차이로 dirty rect가 생기면 안 됨"
        );
    }

    #[test]
    fn snapshot_reuses_renderable_child_when_root_rc_changes() {
        let shared_child = marker_node(Size::new(12.0, 12.0));

        let page1 = root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(16.0, 20.0),
                node: Rc::clone(&shared_child),
            }]),
            Size::new(300.0, 200.0),
        );
        let page2 = root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(16.0, 20.0),
                node: Rc::clone(&shared_child),
            }]),
            Size::new(300.0, 200.0),
        );

        let snapshot1 = PageRenderSnapshot::from_page(&page1);
        let snapshot2 = PageRenderSnapshot::from_page(&page2);

        assert!(
            snapshot1.dirty_rects(&snapshot2).is_empty(),
            "페이지 루트 포인터가 바뀌어도 동일한 렌더 노드는 dirty로 잡히면 안 됨"
        );
    }

    #[test]
    fn snapshot_reuses_wrapper_by_stable_identity() {
        let fold_id = NodeId::new();

        let page1 = root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(8.0, 12.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(240.0, 80.0),
                    element: Some(Element::FoldContent(FoldContentElement::new(
                        Size::new(240.0, 80.0),
                        SplitEdges::default(),
                        fold_id,
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: None,
                }),
            }]),
            Size::new(300.0, 200.0),
        );
        let page2 = root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(8.0, 12.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(240.0, 80.0),
                    element: Some(Element::FoldContent(FoldContentElement::new(
                        Size::new(240.0, 80.0),
                        SplitEdges::default(),
                        fold_id,
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: None,
                }),
            }]),
            Size::new(300.0, 200.0),
        );

        let snapshot1 = PageRenderSnapshot::from_page(&page1);
        let snapshot2 = PageRenderSnapshot::from_page(&page2);

        assert!(
            snapshot1.dirty_rects(&snapshot2).is_empty(),
            "wrapper가 매 프레임 재생성되어도 안정 키로 cache diff가 유지돼야 함"
        );
    }

    #[test]
    fn layout_debug_rects_follow_recomputed_node_ids() {
        let fold_id = NodeId::new();

        let page = root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(8.0, 12.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(240.0, 80.0),
                    element: Some(Element::FoldContent(FoldContentElement::new(
                        Size::new(240.0, 80.0),
                        SplitEdges::default(),
                        fold_id,
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: None,
                }),
            }]),
            Size::new(300.0, 200.0),
        );

        let none = FxHashSet::default();
        assert!(
            collect_layout_dirty_rects(&page, &none).is_empty(),
            "recompute된 node id가 없으면 layout debug rect도 없어야 함"
        );

        let mut recomputed = FxHashSet::default();
        recomputed.insert(fold_id);
        let rects = collect_layout_dirty_rects(&page, &recomputed);
        assert!(
            !rects.is_empty(),
            "recompute된 node id가 있으면 layout debug rect가 표시돼야 함"
        );
    }

    #[test]
    fn layout_debug_rects_coalesce_nested_recomputed_nodes() {
        let parent_id = NodeId::new();
        let child_id = NodeId::new();

        let child = Rc::new(LayoutNode {
            size: Size::new(60.0, 24.0),
            element: Some(Element::FoldContent(FoldContentElement::new(
                Size::new(60.0, 24.0),
                SplitEdges::default(),
                child_id,
            ))),
            children: None,
            page_break_policy: PageBreakPolicy::default(),
            render_hints: RenderHints::default(),
            scope_id: None,
        });

        let parent = Rc::new(LayoutNode {
            size: Size::new(180.0, 80.0),
            element: Some(Element::FoldContent(FoldContentElement::new(
                Size::new(180.0, 80.0),
                SplitEdges::default(),
                parent_id,
            ))),
            children: Some(vec![PositionedNode {
                position: Point::new(12.0, 16.0),
                node: Rc::clone(&child),
            }]),
            page_break_policy: PageBreakPolicy::default(),
            render_hints: RenderHints::default(),
            scope_id: None,
        });

        let page = root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(8.0, 12.0),
                node: parent,
            }]),
            Size::new(300.0, 200.0),
        );

        let mut recomputed = FxHashSet::default();
        recomputed.insert(parent_id);
        recomputed.insert(child_id);

        let rects = collect_layout_dirty_rects(&page, &recomputed);
        assert_eq!(
            rects.len(),
            1,
            "중첩된 recompute는 상위 노드 rect 하나로 축약되어야 함"
        );
        assert!(
            rects[0].approx_eq(
                CacheRect::from_xywh(8.0, 12.0, 180.0, 80.0).expect("valid parent rect")
            ),
            "상위 노드가 dirty rect로 선택되어야 함"
        );

        let mut child_only = FxHashSet::default();
        child_only.insert(child_id);
        let child_rects = collect_layout_dirty_rects(&page, &child_only);
        assert_eq!(
            child_rects.len(),
            1,
            "자식만 recompute되면 자식 rect를 유지해야 함"
        );
        assert!(
            child_rects[0]
                .approx_eq(CacheRect::from_xywh(20.0, 28.0, 60.0, 24.0).expect("valid child rect")),
            "자식 단독 recompute는 자식 위치를 정확히 표시해야 함"
        );
    }

    #[test]
    fn snapshot_ignores_selection_only_table_cell_element() {
        let cell_id = NodeId::new();

        let page1 = root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(20.0, 24.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(120.0, 48.0),
                    element: Some(Element::TableCell(TableCellElement::new(
                        Size::new(120.0, 48.0),
                        cell_id,
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: Some(cell_id),
                }),
            }]),
            Size::new(300.0, 200.0),
        );
        let page2 = root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(20.0, 24.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(120.0, 48.0),
                    element: Some(Element::TableCell(TableCellElement::new(
                        Size::new(120.0, 48.0),
                        cell_id,
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: Some(cell_id),
                }),
            }]),
            Size::new(300.0, 200.0),
        );

        let snapshot1 = PageRenderSnapshot::from_page(&page1);
        let snapshot2 = PageRenderSnapshot::from_page(&page2);

        assert!(
            snapshot1.dirty_rects(&snapshot2).is_empty(),
            "selection-only 요소(TableCell)는 base layer dirty 판단에서 제외돼야 함"
        );
    }

    #[test]
    fn layout_debug_rects_track_table_cell_node() {
        let cell_id = NodeId::new();

        let page = root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(20.0, 24.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(120.0, 48.0),
                    element: Some(Element::TableCell(TableCellElement::new(
                        Size::new(120.0, 48.0),
                        cell_id,
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: Some(cell_id),
                }),
            }]),
            Size::new(300.0, 200.0),
        );

        let mut recomputed = FxHashSet::default();
        recomputed.insert(cell_id);
        let rects = collect_layout_dirty_rects(&page, &recomputed);
        assert!(
            !rects.is_empty(),
            "table cell node가 recompute되면 layout debug rect가 표시돼야 함"
        );

        recomputed.clear();
        recomputed.insert(NodeId::new());
        assert!(
            collect_layout_dirty_rects(&page, &recomputed).is_empty(),
            "다른 node id만 recompute되면 table cell rect는 표시되면 안 됨"
        );
    }

    #[test]
    fn layout_debug_reuses_same_revision() {
        let fold_id = NodeId::new();
        let page = root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(8.0, 12.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(240.0, 80.0),
                    element: Some(Element::FoldContent(FoldContentElement::new(
                        Size::new(240.0, 80.0),
                        SplitEdges::default(),
                        fold_id,
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: None,
                }),
            }]),
            Size::new(300.0, 200.0),
        );
        let doc = Doc::new();
        let diagnostics = FrameDiagnostics::new();
        let mut renderer = Renderer::new(1.0, diagnostics.clone());
        renderer.set_layout_debug(true);
        renderer.set_size(300.0, 200.0, 1.0);

        let mut pass = LayoutPassRecorder::new();
        pass.record_recomputed(fold_id);
        diagnostics.commit_layout_pass(pass);

        let frame1 = renderer
            .prepare_base_layer(&page, 0, &doc)
            .expect("debug frame should exist when layout debug is enabled");
        assert!(
            !frame1.layout_rects.is_empty(),
            "첫 revision에서는 layout rect가 표시되어야 함"
        );

        let frame2 = renderer
            .prepare_base_layer(&page, 0, &doc)
            .expect("debug frame should exist when layout debug is enabled");
        assert!(
            frame2.layout_rects.is_empty(),
            "같은 revision에서는 layout rect를 반복 표시하면 안 됨"
        );
        assert!(frame2.layout_reused, "같은 revision은 reused로 표시돼야 함");
    }

    #[test]
    fn snapshot_reuses_line_in_scoped_node_when_layout_is_unchanged() {
        let block_id = NodeId::new();
        let scope_id = NodeId::new();
        let shared_layout = Rc::new(parley::Layout::default());

        let make_line_node = || {
            Rc::new(LayoutNode {
                size: Size::new(180.0, 20.0),
                element: Some(Element::Line(LineElement::build(
                    block_id,
                    Size::new(180.0, 20.0),
                    0,
                    Rc::clone(&shared_layout),
                    LineMetric {
                        top: 0.0,
                        left: 0.0,
                        height: 20.0,
                        leading: 0.0,
                        baseline: 14.0,
                        ascent: 14.0,
                        content_width: 120.0,
                        start_offset: 0,
                        end_offset: 5,
                        clusters: vec![],
                        break_reason: parley::layout::BreakReason::None,
                        grapheme_offsets: vec![0, 5],
                        ascent_overflow: 0.0,
                        descent_overflow: 0.0,
                    },
                    None,
                    false,
                    Rc::from("hello"),
                    vec![],
                    vec![],
                    false,
                ))),
                children: None,
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints {
                    default_text_color: Some("ui.text.default".to_string()),
                },
                scope_id: Some(scope_id),
            })
        };

        let page1 = root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(20.0, 24.0),
                node: make_line_node(),
            }]),
            Size::new(300.0, 200.0),
        );
        let page2 = root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(20.0, 24.0),
                node: make_line_node(),
            }]),
            Size::new(300.0, 200.0),
        );

        let snapshot1 = PageRenderSnapshot::from_page(&page1);
        let snapshot2 = PageRenderSnapshot::from_page(&page2);

        assert!(
            snapshot1.dirty_rects(&snapshot2).is_empty(),
            "scope/힌트 보정으로 라인 노드 Rc가 바뀌어도 동일 라인은 dirty로 잡히면 안 됨"
        );
    }

    #[test]
    fn snapshot_expands_dirty_rect_upward_for_ruby_line() {
        let block_id = NodeId::new();
        let shared_layout = Rc::new(parley::Layout::default());
        let line_y = 24.0;

        let make_line_node = |ruby_segments: Vec<RubySegment>| {
            Rc::new(LayoutNode {
                size: Size::new(180.0, 20.0),
                element: Some(Element::Line(LineElement::build(
                    block_id,
                    Size::new(180.0, 20.0),
                    0,
                    Rc::clone(&shared_layout),
                    LineMetric {
                        top: 0.0,
                        left: 0.0,
                        height: 20.0,
                        leading: 0.0,
                        baseline: 14.0,
                        ascent: 14.0,
                        content_width: 120.0,
                        start_offset: 0,
                        end_offset: 2,
                        clusters: vec![],
                        break_reason: parley::layout::BreakReason::None,
                        grapheme_offsets: vec![0, 2],
                        ascent_overflow: 0.0,
                        descent_overflow: 0.0,
                    },
                    None,
                    false,
                    Rc::from("ab"),
                    ruby_segments,
                    vec![],
                    false,
                ))),
                children: None,
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: None,
            })
        };

        let page_without_ruby = root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(20.0, line_y),
                node: make_line_node(vec![]),
            }]),
            Size::new(300.0, 200.0),
        );
        let page_with_ruby = root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(20.0, line_y),
                node: make_line_node(vec![RubySegment {
                    start_offset: 0,
                    end_offset: 1,
                    ruby_text: "루".to_string(),
                }]),
            }]),
            Size::new(300.0, 200.0),
        );

        let snapshot1 = PageRenderSnapshot::from_page(&page_without_ruby);
        let snapshot2 = PageRenderSnapshot::from_page(&page_with_ruby);
        let rects = snapshot1.dirty_rects(&snapshot2);

        assert!(
            !rects.is_empty(),
            "루비 추가 시 라인 snapshot dirty rect가 비어 있으면 안 됨"
        );
        assert!(
            rects.iter().any(|rect| rect.y < line_y),
            "루비 상단 영역이 partial repaint 대상에 포함돼야 함"
        );
    }

    #[test]
    fn clip_intersection_uses_ruby_overhang_bounds() {
        let block_id = NodeId::new();
        let shared_layout = Rc::new(parley::Layout::default());
        let line_y = 24.0;

        let make_positioned_line = |ruby_segments: Vec<RubySegment>| PositionedNode {
            position: Point::new(20.0, line_y),
            node: Rc::new(LayoutNode {
                size: Size::new(180.0, 20.0),
                element: Some(Element::Line(LineElement::build(
                    block_id,
                    Size::new(180.0, 20.0),
                    0,
                    Rc::clone(&shared_layout),
                    LineMetric {
                        top: 0.0,
                        left: 0.0,
                        height: 20.0,
                        leading: 0.0,
                        baseline: 14.0,
                        ascent: 14.0,
                        content_width: 120.0,
                        start_offset: 0,
                        end_offset: 2,
                        clusters: vec![],
                        break_reason: parley::layout::BreakReason::None,
                        grapheme_offsets: vec![0, 2],
                        ascent_overflow: 0.0,
                        descent_overflow: 0.0,
                    },
                    None,
                    false,
                    Rc::from("ab"),
                    ruby_segments,
                    vec![],
                    false,
                ))),
                children: None,
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: None,
            }),
        };

        let clip = CacheRect::from_xywh(0.0, line_y - 8.0, 300.0, 6.0).expect("valid clip rect");
        let with_ruby = make_positioned_line(vec![RubySegment {
            start_offset: 0,
            end_offset: 1,
            ruby_text: "루".to_string(),
        }]);
        let without_ruby = make_positioned_line(vec![]);

        let with_ruby_bounds =
            node_paint_bounds(&with_ruby, with_ruby.position).expect("line bounds should exist");
        let without_ruby_bounds = node_paint_bounds(&without_ruby, without_ruby.position)
            .expect("line bounds should exist");

        assert!(
            with_ruby_bounds.intersects(clip),
            "루비 상단 overhang 영역만 clip 돼도 라인 렌더가 스킵되면 안 됨"
        );
        assert!(
            !without_ruby_bounds.intersects(clip),
            "루비가 없는 라인은 overhang clip과 교차하지 않아야 함"
        );
    }

    #[test]
    fn next_page_ruby_clip_intersects_boundary_root() {
        let page_width = 300.0;
        let page_height = 200.0;
        let boundary_root = CacheRect::from_xywh(0.0, page_height, page_width, page_height)
            .expect("valid root rect");

        let narrow_clip = CacheRect::from_xywh(
            0.0,
            page_height - PAGE_EDGE_OVERFLOW_BAND,
            page_width,
            PAGE_EDGE_OVERFLOW_BAND,
        )
        .expect("valid narrow clip");
        assert!(
            !boundary_root.intersects(narrow_clip),
            "경계에 딱 붙은 다음 페이지 루트는 좁은 clip에서는 탈락한다"
        );

        let expanded_clip =
            next_page_overflow_cull_clip(page_width, page_height).expect("expanded clip");
        assert!(
            boundary_root.intersects(expanded_clip),
            "다음 페이지 루트(y == page_height)는 오버플로우 clip과 교차해야 한다"
        );
    }

    #[test]
    fn overflow_debug_rects_include_visible_ruby_overhang() {
        let block_id = NodeId::new();
        let shared_layout = Rc::new(parley::Layout::default());
        let page_width = 300.0;
        let page_height = 200.0;

        let make_next_page = |ruby_segments: Vec<RubySegment>| {
            root_with_children(
                Some(vec![PositionedNode {
                    position: Point::new(20.0, 0.0),
                    node: Rc::new(LayoutNode {
                        size: Size::new(180.0, 20.0),
                        element: Some(Element::Line(LineElement::build(
                            block_id,
                            Size::new(180.0, 20.0),
                            0,
                            Rc::clone(&shared_layout),
                            LineMetric {
                                top: 0.0,
                                left: 0.0,
                                height: 20.0,
                                leading: 0.0,
                                baseline: 14.0,
                                ascent: 14.0,
                                content_width: 120.0,
                                start_offset: 0,
                                end_offset: 2,
                                clusters: vec![],
                                break_reason: parley::layout::BreakReason::None,
                                grapheme_offsets: vec![0, 2],
                                ascent_overflow: 0.0,
                                descent_overflow: 0.0,
                            },
                            None,
                            false,
                            Rc::from("ab"),
                            ruby_segments,
                            vec![],
                            false,
                        ))),
                        children: None,
                        page_break_policy: PageBreakPolicy::default(),
                        render_hints: RenderHints::default(),
                        scope_id: None,
                    }),
                }]),
                Size::new(page_width, page_height),
            )
        };

        let with_ruby = make_next_page(vec![RubySegment {
            start_offset: 0,
            end_offset: 1,
            ruby_text: "루".to_string(),
        }]);
        let without_ruby = make_next_page(vec![]);
        let clip =
            next_page_overflow_cull_clip(page_width, page_height).expect("next page overflow clip");

        let with_rects =
            collect_next_page_overflow_debug_rects(&with_ruby, page_width, page_height, clip);
        let without_rects =
            collect_next_page_overflow_debug_rects(&without_ruby, page_width, page_height, clip);

        assert!(
            !with_rects.is_empty(),
            "루비가 있으면 현재 페이지 상단 오버플로우 디버그 rect가 수집돼야 함"
        );
        assert!(
            with_rects
                .iter()
                .any(|rect| rect.y < page_height && rect.bottom() <= page_height),
            "수집된 rect는 현재 페이지의 가시 영역 내에 있어야 함"
        );
        assert!(
            without_rects.is_empty(),
            "루비가 없으면 현재 페이지에 보이는 next-page overflow rect가 없어야 함"
        );
    }

    #[test]
    fn overflow_debug_rects_include_visible_non_ruby_top_overflow() {
        let block_id = NodeId::new();
        let shared_layout = Rc::new(parley::Layout::default());
        let page_width = 300.0;
        let page_height = 200.0;

        let make_next_page = |ascent_overflow: f32| {
            root_with_children(
                Some(vec![PositionedNode {
                    position: Point::new(20.0, 0.0),
                    node: Rc::new(LayoutNode {
                        size: Size::new(180.0, 20.0),
                        element: Some(Element::Line(LineElement::build(
                            block_id,
                            Size::new(180.0, 20.0),
                            0,
                            Rc::clone(&shared_layout),
                            LineMetric {
                                top: 0.0,
                                left: 0.0,
                                height: 20.0,
                                leading: 0.0,
                                baseline: 14.0,
                                ascent: 14.0,
                                content_width: 120.0,
                                start_offset: 0,
                                end_offset: 2,
                                clusters: vec![],
                                break_reason: parley::layout::BreakReason::None,
                                grapheme_offsets: vec![0, 2],
                                ascent_overflow,
                                descent_overflow: 0.0,
                            },
                            None,
                            false,
                            Rc::from("ab"),
                            vec![],
                            vec![],
                            false,
                        ))),
                        children: None,
                        page_break_policy: PageBreakPolicy::default(),
                        render_hints: RenderHints::default(),
                        scope_id: None,
                    }),
                }]),
                Size::new(page_width, page_height),
            )
        };

        let with_top_overflow = make_next_page(6.0);
        let without_top_overflow = make_next_page(0.0);
        let clip =
            next_page_overflow_cull_clip(page_width, page_height).expect("next page overflow clip");

        let with_rects = collect_next_page_overflow_debug_rects(
            &with_top_overflow,
            page_width,
            page_height,
            clip,
        );
        let without_rects = collect_next_page_overflow_debug_rects(
            &without_top_overflow,
            page_width,
            page_height,
            clip,
        );

        assert!(
            !with_rects.is_empty(),
            "루비가 없어도 top paint_overflow가 있으면 next-page overflow rect가 수집돼야 함"
        );
        assert!(
            with_rects.iter().any(|rect| rect.y < page_height),
            "수집된 rect는 현재 페이지 가시 영역으로 클램프되어야 함"
        );
        assert!(
            without_rects.is_empty(),
            "top paint_overflow가 없으면 next-page overflow rect가 없어야 함"
        );
    }

    #[test]
    fn prev_page_overflow_debug_rects_include_visible_bottom_overflow() {
        let block_id = NodeId::new();
        let shared_layout = Rc::new(parley::Layout::default());
        let page_width = 300.0;
        let page_height = 200.0;

        let make_prev_page = |descent_overflow: f32| {
            root_with_children(
                Some(vec![PositionedNode {
                    position: Point::new(20.0, page_height - 20.0),
                    node: Rc::new(LayoutNode {
                        size: Size::new(180.0, 20.0),
                        element: Some(Element::Line(LineElement::build(
                            block_id,
                            Size::new(180.0, 20.0),
                            0,
                            Rc::clone(&shared_layout),
                            LineMetric {
                                top: 0.0,
                                left: 0.0,
                                height: 20.0,
                                leading: 0.0,
                                baseline: 14.0,
                                ascent: 14.0,
                                content_width: 120.0,
                                start_offset: 0,
                                end_offset: 2,
                                clusters: vec![],
                                break_reason: parley::layout::BreakReason::None,
                                grapheme_offsets: vec![0, 2],
                                ascent_overflow: 0.0,
                                descent_overflow,
                            },
                            None,
                            false,
                            Rc::from("ab"),
                            vec![],
                            vec![],
                            false,
                        ))),
                        children: None,
                        page_break_policy: PageBreakPolicy::default(),
                        render_hints: RenderHints::default(),
                        scope_id: None,
                    }),
                }]),
                Size::new(page_width, page_height),
            )
        };

        let with_bottom_overflow = make_prev_page(8.0);
        let without_bottom_overflow = make_prev_page(0.0);
        let clip = prev_page_overflow_cull_clip(page_width).expect("prev page overflow clip");

        let with_rects = collect_prev_page_overflow_debug_rects(
            &with_bottom_overflow,
            page_width,
            page_height,
            clip,
        );
        let without_rects = collect_prev_page_overflow_debug_rects(
            &without_bottom_overflow,
            page_width,
            page_height,
            clip,
        );

        assert!(
            !with_rects.is_empty(),
            "bottom paint_overflow가 있으면 이전 페이지 overflow rect가 수집돼야 함"
        );
        assert!(
            with_rects
                .iter()
                .any(|rect| rect.y <= PAGE_EDGE_OVERFLOW_BAND && rect.bottom() > 0.0),
            "수집된 rect는 현재 페이지 상단 가시 영역에 위치해야 함"
        );
        assert!(
            without_rects.is_empty(),
            "bottom paint_overflow가 없으면 이전 페이지 overflow rect가 없어야 함"
        );
    }

    #[test]
    fn partial_render_does_not_overdraw_outside_dirty_rect() {
        let callout_id = NodeId::new();
        let page1 = callout_page_with_icon(callout_id);
        let page2 = callout_page_with_icon(callout_id);

        let doc = Doc::new();
        let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
        renderer.set_size(220.0, 160.0, 1.0);

        let width = renderer.width() as usize;
        let height = renderer.height() as usize;
        let mut buffer = vec![0u8; width * height * 4];

        assert!(renderer.render_to(&page1, 0, None, None, &[], None, &doc, &mut buffer));
        let first = rgba_at(&buffer, width, 120, 70);
        assert!(
            first[3] > 0,
            "샘플 픽셀이 투명하면 callout 배경이 실제로 그려졌는지 검증할 수 없음"
        );

        assert!(renderer.render_to(&page2, 0, None, None, &[], None, &doc, &mut buffer));
        let second = rgba_at(&buffer, width, 120, 70);

        assert_eq!(
            first, second,
            "dirty rect 밖 픽셀은 부분 렌더 후에도 변하면 안 됨"
        );
    }

    #[test]
    fn dense_line_shift_rects_are_coalesced_before_full_promotion() {
        let mut rects = Vec::new();
        for i in 0..40 {
            rects.push(
                CacheRect::from_xywh(20.0, 32.0 + i as f32 * 14.0, 260.0, 12.0)
                    .expect("valid rect"),
            );
        }

        let normalized = normalize_dirty_rects(rects, 300.0, 900.0);
        assert!(
            normalized.len() < FULL_REPAINT_RECT_THRESHOLD,
            "dense line shift dirty rects should be coalesced before full repaint threshold"
        );
        assert!(
            !should_promote_full_repaint(&normalized, 300.0, 900.0),
            "line shift region that covers about half the page should stay partial repaint"
        );
    }

    #[test]
    fn height_only_resize_reuses_snapshot_and_repaints_exposed_strip() {
        let callout_id = NodeId::new();
        let make_page = |height: f32| {
            root_with_children(
                Some(vec![PositionedNode {
                    position: Point::new(20.0, 24.0),
                    node: Rc::new(LayoutNode {
                        size: Size::new(260.0, 72.0),
                        element: Some(Element::CalloutBackground(CalloutBackgroundElement::new(
                            Size::new(260.0, 72.0),
                            CalloutVariant::Info,
                            callout_id,
                            SplitEdges::default(),
                        ))),
                        children: None,
                        page_break_policy: PageBreakPolicy::default(),
                        render_hints: RenderHints::default(),
                        scope_id: None,
                    }),
                }]),
                Size::new(300.0, height),
            )
        };

        let page1 = make_page(200.0);
        let page2 = make_page(260.0);

        let doc = Doc::new();
        let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
        renderer.set_render_debug(true);

        renderer.set_size(300.0, 200.0, 1.0);
        let _ = renderer
            .prepare_base_layer(&page1, 0, &doc)
            .expect("debug frame should exist");

        renderer.set_size(300.0, 260.0, 1.0);
        let frame = renderer
            .prepare_base_layer(&page2, 0, &doc)
            .expect("debug frame should exist");

        assert!(
            !frame.full_repaint,
            "height-only resize should not force full repaint when snapshot can be reused"
        );
        assert!(
            !frame.render_rects.is_empty(),
            "newly exposed strip should be marked dirty and repainted"
        );
        assert!(
            frame.render_rects.iter().any(|rect| rect.y >= 199.0),
            "dirty rect should include the exposed bottom strip after height growth"
        );
    }

    #[test]
    fn snapshot_marks_table_border_dirty_when_columns_change_without_bounds_change() {
        let table_id = NodeId::new();

        let make_page = |cols: usize, col_widths: Vec<f32>| {
            root_with_children(
                Some(vec![PositionedNode {
                    position: Point::new(20.0, 24.0),
                    node: Rc::new(LayoutNode {
                        size: Size::new(300.0, 120.0),
                        element: Some(Element::TableBorder(TableBorderElement::new(
                            Size::new(300.0, 120.0),
                            table_id,
                            TableBorderStyle::Solid,
                            TableAlign::Left,
                            2,
                            cols,
                            vec![59.0, 59.0],
                            col_widths,
                            SplitEdges::default(),
                            0.0,
                            0.0,
                            0,
                            2,
                        ))),
                        children: None,
                        page_break_policy: PageBreakPolicy::default(),
                        render_hints: RenderHints::default(),
                        scope_id: None,
                    }),
                }]),
                Size::new(360.0, 200.0),
            )
        };

        let page1 = make_page(3, vec![98.0, 98.0, 98.0]);
        let page2 = make_page(2, vec![148.0, 148.0]);

        let snapshot1 = PageRenderSnapshot::from_page(&page1);
        let snapshot2 = PageRenderSnapshot::from_page(&page2);

        assert!(
            !snapshot1.dirty_rects(&snapshot2).is_empty(),
            "테이블 열/폭이 바뀌면 bounds가 같아도 dirty로 잡혀야 함"
        );
    }

    #[test]
    fn prune_page_cache_removes_entries_outside_page_count() {
        let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());

        renderer
            .page_cache
            .insert(0, PageRenderCache::new(64, 64, 1.0));
        renderer
            .page_cache
            .insert(1, PageRenderCache::new(64, 64, 1.0));
        renderer
            .page_cache
            .insert(3, PageRenderCache::new(64, 64, 1.0));

        renderer.prune_page_cache(2);

        assert_eq!(renderer.page_cache.len(), 2);
        assert!(renderer.page_cache.contains_key(&0));
        assert!(renderer.page_cache.contains_key(&1));
        assert!(!renderer.page_cache.contains_key(&3));
    }
}
