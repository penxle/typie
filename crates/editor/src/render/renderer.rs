use super::*;
const DIRTY_RECT_EPSILON: f32 = 0.5;
const DIRTY_RECT_COALESCE_EPSILON: f32 = 8.0;
const FULL_REPAINT_COVERAGE_THRESHOLD: f32 = 0.7;
pub(super) const FULL_REPAINT_RECT_THRESHOLD: usize = 32;
pub(super) const PAGE_EDGE_OVERFLOW_BAND: f32 = 16.0;

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
    pub(super) pixmap: Pixmap,
    pub width: u16,
    pub height: u16,
    pub offset_x: f32,
    pub offset_y: f32,
    pub scale_factor: f32,
}

#[derive(Default, Clone, Copy)]
pub(super) struct OverlayProfile {
    pub(super) overflow_ms: f64,
    pub(super) selection_ms: f64,
    pub(super) selection_collect_ms: f64,
    pub(super) selection_paint_ms: f64,
    pub(super) selection_fast_path: bool,
    pub(super) selection_phase_full: bool,
    pub(super) selection_clip_rect_count: usize,
    pub(super) selection_text_rect_count: usize,
    pub(super) selection_has_non_text: bool,
    pub(super) content_ms: f64,
    pub(super) content_full_ms: f64,
    pub(super) content_composite_ms: f64,
    pub(super) content_clipped_ms: f64,
    pub(super) content_full_render: bool,
    pub(super) content_cached_composite: bool,
    pub(super) content_clipped_render: bool,
    pub(super) content_clip_rect_count: usize,
    pub(super) content_clipped_pass_count: usize,
    pub(super) drop_ms: f64,
    pub(super) debug_ms: f64,
}

#[derive(Default, Clone, Copy)]
pub(super) struct BasePrepareProfile {
    pub(super) snapshot_ms: f64,
    pub(super) dirty_ms: f64,
    pub(super) background_ms: f64,
    pub(super) content_ms: f64,
    pub(super) compose_ms: f64,
    pub(super) dirty_rect_count: usize,
    pub(super) render_rect_count: usize,
}

pub(super) struct OverflowRenderCacheEntry {
    pub(super) scale_factor: f64,
    pub(super) canvas_width: u32,
    pub(super) canvas_height: u32,
    pub(super) pixel_rect: PixelRect,
    pub(super) next_root_ptr: usize,
    pub(super) next_snapshot: OverflowRenderSnapshot,
    pub(super) tile_pixmap: Pixmap,
    pub(super) debug_rects: Vec<CacheRect>,
}

#[derive(Default, Clone, PartialEq, Eq)]
pub(super) struct OverflowRenderSnapshot {
    pub(super) items: Vec<OverflowSnapshotItem>,
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct OverflowSnapshotItem {
    pub(super) signature: u64,
}

#[derive(Default)]
pub(super) struct SelectionOverlayData {
    pub(super) clip_rects: Vec<CacheRect>,
    pub(super) text_paint_rects: Vec<CacheRect>,
    pub(super) has_non_text_selection: bool,
}

#[derive(Default, Clone, Copy)]
pub(super) struct SelectionPaintStats {
    pub(super) fast_path: bool,
    pub(super) full_phase: bool,
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
    pub(super) scale_factor: f64,
    pub(super) pixmap: Pixmap,
    pub(super) scratch_pixmap: Pixmap,
    pub(super) glyph_renderer: GlyphRenderer,
    pub(super) theme: Theme,
    pub(super) is_focused: bool,
    pub(super) page_cache: FxHashMap<usize, PageRenderCache>,
    pub(super) overflow_cache: FxHashMap<usize, OverflowRenderCacheEntry>,
    pub(super) render_debug_enabled: bool,
    pub(super) layout_debug_enabled: bool,
    pub(super) paint_diagnostics: PaintDiagnosticsState,
    pub(super) diagnostics: FrameDiagnostics,
    pub(super) base_prepare_profile: BasePrepareProfile,
}

impl Renderer {
    pub fn new(scale_factor: f64, diagnostics: FrameDiagnostics) -> Self {
        let pixmap = Pixmap::new(1, 1).unwrap();
        let scratch_pixmap = Pixmap::new(1, 1).unwrap();

        Self {
            scale_factor,
            pixmap,
            scratch_pixmap,
            glyph_renderer: GlyphRenderer::new(),
            theme: Theme::default(),
            is_focused: true,
            page_cache: FxHashMap::default(),
            overflow_cache: FxHashMap::default(),
            render_debug_enabled: false,
            layout_debug_enabled: false,
            paint_diagnostics: PaintDiagnosticsState::default(),
            diagnostics,
            base_prepare_profile: BasePrepareProfile::default(),
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
            self.overflow_cache.clear();
            self.paint_diagnostics.clear();
        }
    }

    pub fn set_theme(&mut self, theme: Theme) {
        if self.theme != theme {
            self.theme = theme;
            self.page_cache.clear();
            self.overflow_cache.clear();
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
        self.overflow_cache
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
        next_page: Option<&Page>,
        selections: &[SelectionDecor],
        drop_indicator: Option<&DropIndicator>,
        doc: &Doc,
    ) -> RenderResult {
        let total_started_at = profile_now();

        let base_prepare_started_at = profile_now();
        let mut debug_frame = self.prepare_base_layer(page, page_idx, doc);
        let base_prepare_ms = profile_elapsed_ms(base_prepare_started_at);
        let base_prepare_profile = self.base_prepare_profile;

        let base_copy_started_at = profile_now();
        let mut background_layer = None;
        let mut content_layer = None;
        if let Some(cache) = self.page_cache.get(&page_idx) {
            self.pixmap
                .data_mut()
                .copy_from_slice(cache.base_pixmap.data());
            if !selections.is_empty() {
                background_layer = Some(&cache.background_pixmap);
                content_layer = Some(&cache.content_pixmap);
            }
        } else {
            self.pixmap.data_mut().fill(0);
        }
        let base_copy_ms = profile_elapsed_ms(base_copy_started_at);

        let overlay_started_at = profile_now();
        let mut pixmap = self.pixmap.as_mut();
        let overlay_profile = Self::render_overlay_layers(
            &mut pixmap,
            &mut self.glyph_renderer,
            self.scale_factor,
            &self.theme,
            self.is_focused,
            self.render_debug_enabled,
            self.layout_debug_enabled,
            self.render_debug_enabled,
            background_layer,
            content_layer,
            &mut self.overflow_cache,
            page,
            page_idx,
            next_page,
            selections,
            drop_indicator,
            doc,
            &mut debug_frame,
        );
        let overlay_ms = profile_elapsed_ms(overlay_started_at);
        let total_ms = profile_elapsed_ms(total_started_at);

        if self.render_debug_enabled {
            self.log_render_profile(
                page_idx,
                selections.len(),
                drop_indicator.is_some(),
                next_page.is_some(),
                debug_frame.as_ref(),
                base_prepare_ms,
                base_prepare_profile,
                base_copy_ms,
                overlay_ms,
                total_ms,
                overlay_profile,
            );
        }

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
        let mut background_layer = None;
        let mut content_layer = None;
        if let Some(cache) = self.page_cache.get(&page_idx) {
            pixmap.data_mut().copy_from_slice(cache.base_pixmap.data());
            if !selections.is_empty() {
                background_layer = Some(&cache.background_pixmap);
                content_layer = Some(&cache.content_pixmap);
            }
        } else {
            pixmap.data_mut().fill(0);
        }

        let _overlay_profile = Self::render_overlay_layers(
            &mut pixmap,
            &mut self.glyph_renderer,
            self.scale_factor,
            &self.theme,
            self.is_focused,
            self.render_debug_enabled,
            self.layout_debug_enabled,
            self.render_debug_enabled,
            background_layer,
            content_layer,
            &mut self.overflow_cache,
            page,
            page_idx,
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
            Self::outline_node_for_next_page_overflow(
                &mut sink,
                &next_page.root,
                Point::new(0.0, page_height),
                Transform::identity(),
                &ctx,
                &RenderHints::default(),
                cull_clip,
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
    fn log_render_profile(
        &self,
        page_idx: usize,
        selection_count: usize,
        has_drop_indicator: bool,
        has_next_page: bool,
        frame: Option<&PaintDebugFrame>,
        base_prepare_ms: f64,
        base_prepare_profile: BasePrepareProfile,
        base_copy_ms: f64,
        overlay_ms: f64,
        total_ms: f64,
        overlay_profile: OverlayProfile,
    ) {
        let render_rect_count = frame.map(|f| f.render_rects.len()).unwrap_or(0);
        let overflow_rect_count = frame.map(|f| f.overflow_rects.len()).unwrap_or(0);
        let full_repaint = frame.map(|f| f.full_repaint).unwrap_or(false);
        let cache_reused = frame.map(|f| f.cache_reused).unwrap_or(false);
        let dirty_area: f32 = frame
            .map(|f| f.render_rects.iter().map(|rect| rect.area()).sum())
            .unwrap_or(0.0);

        let scale = self.scale_factor as f32;
        let canvas_width = if scale > 0.0 {
            self.pixmap.width() as f32 / scale
        } else {
            0.0
        };
        let canvas_height = if scale > 0.0 {
            self.pixmap.height() as f32 / scale
        } else {
            0.0
        };
        let canvas_area = canvas_width * canvas_height;
        let dirty_ratio = if canvas_area > 0.0 {
            dirty_area / canvas_area
        } else {
            0.0
        };
        let selection_mode = if overlay_profile.selection_fast_path {
            "fast"
        } else if overlay_profile.selection_phase_full {
            "phase_full"
        } else if selection_count > 0 && overlay_profile.selection_ms > 0.0 {
            "phase_partial"
        } else {
            "none"
        };
        let content_mode = if overlay_profile.content_full_render {
            "full"
        } else if overlay_profile.content_cached_composite {
            "composite"
        } else if overlay_profile.content_clipped_render {
            "clipped"
        } else {
            "none"
        };

        log!(
            "[render-prof] page={} total={:.2}ms base_prepare={:.2}ms base_snapshot={:.2}ms base_dirty={:.2}ms base_background={:.2}ms base_content={:.2}ms base_compose={:.2}ms base_dirty_rects={} base_render_rects={} base_copy={:.2}ms overlay={:.2}ms overlay_overflow={:.2}ms overlay_selection={:.2}ms overlay_selection_collect={:.2}ms overlay_selection_paint={:.2}ms overlay_selection_mode={} overlay_selection_clip_rects={} overlay_selection_text_rects={} overlay_selection_non_text={} overlay_content={:.2}ms overlay_content_mode={} overlay_content_full={:.2}ms overlay_content_composite={:.2}ms overlay_content_clipped={:.2}ms overlay_content_clip_rects={} overlay_content_clipped_passes={} overlay_drop={:.2}ms overlay_debug={:.2}ms rects={} overflow_rects={} dirty={:.1}% full={} cache_reused={} selections={} drop={} next_page={}",
            page_idx,
            total_ms,
            base_prepare_ms,
            base_prepare_profile.snapshot_ms,
            base_prepare_profile.dirty_ms,
            base_prepare_profile.background_ms,
            base_prepare_profile.content_ms,
            base_prepare_profile.compose_ms,
            base_prepare_profile.dirty_rect_count,
            base_prepare_profile.render_rect_count,
            base_copy_ms,
            overlay_ms,
            overlay_profile.overflow_ms,
            overlay_profile.selection_ms,
            overlay_profile.selection_collect_ms,
            overlay_profile.selection_paint_ms,
            selection_mode,
            overlay_profile.selection_clip_rect_count,
            overlay_profile.selection_text_rect_count,
            overlay_profile.selection_has_non_text,
            overlay_profile.content_ms,
            content_mode,
            overlay_profile.content_full_ms,
            overlay_profile.content_composite_ms,
            overlay_profile.content_clipped_ms,
            overlay_profile.content_clip_rect_count,
            overlay_profile.content_clipped_pass_count,
            overlay_profile.drop_ms,
            overlay_profile.debug_ms,
            render_rect_count,
            overflow_rect_count,
            dirty_ratio * 100.0,
            full_repaint,
            cache_reused,
            selection_count,
            has_drop_indicator,
            has_next_page,
        );
    }
}

pub(super) fn normalize_dirty_rects(
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
pub(super) fn should_promote_full_repaint(
    rects: &[CacheRect],
    canvas_width: f32,
    canvas_height: f32,
) -> bool {
    let full_area = canvas_width * canvas_height;
    let dirty_area: f32 = rects.iter().map(|rect| rect.area()).sum();
    rects.len() > FULL_REPAINT_RECT_THRESHOLD
        || (full_area > 0.0 && dirty_area / full_area >= FULL_REPAINT_COVERAGE_THRESHOLD)
}

pub(super) fn next_page_overflow_cull_clip(page_width: f32, page_height: f32) -> Option<CacheRect> {
    CacheRect::from_xywh(
        0.0,
        (page_height - PAGE_EDGE_OVERFLOW_BAND).max(0.0),
        page_width,
        PAGE_EDGE_OVERFLOW_BAND * 2.0,
    )
}
