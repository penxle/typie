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
        let mut debug_frame = self.prepare_base_layer(page, page_idx, doc);

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

        let mut pixmap = self.pixmap.as_mut();
        Self::render_overlay_layers(
            &mut pixmap,
            &mut self.glyph_renderer,
            self.scale_factor,
            &self.theme,
            self.is_focused,
            self.render_debug_enabled,
            self.layout_debug_enabled,
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

        Self::render_overlay_layers(
            &mut pixmap,
            &mut self.glyph_renderer,
            self.scale_factor,
            &self.theme,
            self.is_focused,
            self.render_debug_enabled,
            self.layout_debug_enabled,
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
