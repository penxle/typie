use super::*;
const DIRTY_RECT_EPSILON: f32 = 0.5;
const DIRTY_RECT_COALESCE_EPSILON: f32 = 8.0;
const FULL_REPAINT_COVERAGE_THRESHOLD: f32 = 0.7;
pub(super) const FULL_REPAINT_RECT_THRESHOLD: usize = 32;
pub(super) const PAGE_EDGE_OVERFLOW_BAND: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderPhase {
    Background,
    Selection,
    Content,
}

/// 렌더링 phase 순서. 모든 backend가 이 순서를 사용한다.
pub const RENDER_PHASES: [RenderPhase; 3] = [
    RenderPhase::Background,
    RenderPhase::Selection,
    RenderPhase::Content,
];

pub struct RenderParams<'a> {
    pub scale_factor: f64,
    pub selections: &'a [SelectionDecor],
    pub theme: &'a Theme,
    pub doc: &'a Doc,
    pub default_text_color: Option<Color>,
    pub is_focused: bool,
    pub phase: RenderPhase,
    pub render_origin: Point,
}

impl<'a> RenderParams<'a> {
    pub fn selection_paint(&self) -> Brush {
        selection_overlay_brush(self.theme, self.is_focused)
    }

    pub fn is_block_selected(&self, node_id: NodeId) -> bool {
        self.selections.iter().any(|selection| {
            matches!(selection, SelectionDecor::Block { node_id: id } if *id == node_id)
        })
    }

    pub fn has_descendant_text_selection(&self, node_id: NodeId) -> bool {
        self.selections.iter().any(|selection| {
            matches!(selection, SelectionDecor::TextRange { node_id: id, .. } if *id == node_id || self.doc.is_ancestor(node_id, *id))
        })
    }
}

pub trait Render {
    fn render(&self, sink: &mut dyn RenderSink, transform: Affine, ctx: &RenderParams<'_>);
}

#[allow(dead_code)]
pub struct RenderInfo {
    pub width: u16,
    pub height: u16,
    pub buffer_size: usize,
}

pub struct DragImageResult {
    pub(super) buf: PixelBuf,
    pub width: u16,
    pub height: u16,
    pub offset_x: f32,
    pub offset_y: f32,
    pub scale_factor: f32,
}

pub(crate) struct OverflowRenderCacheEntry {
    pub(super) scale_factor: f64,
    pub(super) canvas_width: u32,
    pub(super) canvas_height: u32,
    pub(super) pixel_rect: PixelRect,
    pub(super) next_root_ptr: usize,
    pub(super) next_snapshot: OverflowRenderSnapshot,
    pub(super) tile: PixelBuf,
    pub(super) debug_rects: Vec<LayoutRect>,
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
pub(crate) struct SelectionOverlayData {
    pub(super) clip_rects: Vec<LayoutRect>,
    pub(super) text_paint_rects: Vec<LayoutRect>,
    pub(super) has_non_text_selection: bool,
}

impl DragImageResult {
    pub fn ptr(&self) -> *const u8 {
        self.buf.data().as_ptr()
    }

    pub fn len(&self) -> usize {
        self.buf.data().len()
    }
}

pub struct Renderer {
    pub(super) backend: backend::RenderBackend,
    pub(super) scale_factor: f64,
    pub(super) theme: Theme,
    pub(super) is_focused: bool,
    pub(super) page_cache: FxHashMap<usize, PageCache>,
    pub(super) overflow_cache: FxHashMap<usize, OverflowRenderCacheEntry>,
    pub(super) attached_surfaces: FxHashMap<u32, super::surface::SurfaceSize>,
    pub(super) render_debug_enabled: bool,
    pub(super) layout_debug_enabled: bool,
    pub(super) diagnostics_state: DiagnosticsState,
    pub(super) diagnostics: FrameDiagnostics,
}

impl Renderer {
    #[allow(dead_code)]
    pub fn new(scale_factor: f64, diagnostics: FrameDiagnostics) -> Self {
        Self::with_backend(backend::RenderBackend::new_cpu(), scale_factor, diagnostics)
    }

    pub fn with_backend(
        backend: backend::RenderBackend,
        scale_factor: f64,
        diagnostics: FrameDiagnostics,
    ) -> Self {
        Self {
            backend,
            scale_factor,
            theme: Theme::default(),
            is_focused: true,
            page_cache: FxHashMap::default(),
            overflow_cache: FxHashMap::default(),
            attached_surfaces: FxHashMap::default(),
            render_debug_enabled: false,
            layout_debug_enabled: false,
            diagnostics_state: DiagnosticsState::default(),
            diagnostics,
        }
    }

    #[allow(dead_code)]
    pub fn is_gpu(&self) -> bool {
        matches!(self.backend, backend::RenderBackend::Gpu { .. })
    }

    #[allow(dead_code)]
    pub fn width(&self) -> u32 {
        match &self.backend {
            backend::RenderBackend::Cpu { buf, .. } => buf.width(),
            backend::RenderBackend::Gpu { post_buf, .. } => post_buf.width(),
        }
    }

    #[allow(dead_code)]
    pub fn height(&self) -> u32 {
        match &self.backend {
            backend::RenderBackend::Cpu { buf, .. } => buf.height(),
            backend::RenderBackend::Gpu { post_buf, .. } => post_buf.height(),
        }
    }

    pub fn set_size(&mut self, width: f32, height: f32, scale_factor: f64) {
        let new_width = (width as f64 * scale_factor).round() as u32;
        let new_height = (height as f64 * scale_factor).round() as u32;
        let scale_changed = !same_scale_factor(self.scale_factor, scale_factor);

        match &mut self.backend {
            backend::RenderBackend::Cpu { buf, .. } => {
                if buf.width() != new_width || buf.height() != new_height {
                    if let Some(new_buf) = PixelBuf::new(new_width.max(1), new_height.max(1)) {
                        *buf = new_buf;
                    }
                }
            }
            backend::RenderBackend::Gpu { post_buf, .. } => {
                if post_buf.width() != new_width || post_buf.height() != new_height {
                    if let Some(new_buf) = PixelBuf::new(new_width.max(1), new_height.max(1)) {
                        *post_buf = new_buf;
                    }
                }
            }
        }
        self.scale_factor = scale_factor;
        if scale_changed {
            self.page_cache.clear();
            self.overflow_cache.clear();
            self.diagnostics_state.clear();
        }
    }

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) -> bool {
        let scale_changed = !same_scale_factor(self.scale_factor, scale_factor);
        self.scale_factor = scale_factor;
        if scale_changed {
            self.page_cache.clear();
            self.overflow_cache.clear();
            self.diagnostics_state.clear();
        }
        scale_changed
    }

    pub fn set_theme(&mut self, theme: Theme) {
        if self.theme != theme {
            self.theme = theme;
            self.page_cache.clear();
            self.overflow_cache.clear();
            self.diagnostics_state.clear();
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
        self.diagnostics_state.retain_pages(valid_page_count);
    }

    /// Base layer 준비를 backend::cpu::layers에 위임한다.
    #[allow(dead_code)]
    pub(super) fn prepare_base_layer(
        &mut self,
        page: &Page,
        page_idx: usize,
        doc: &Doc,
    ) -> Option<DebugFrame> {
        backend::cpu::prepare_base_layer(
            page,
            page_idx,
            doc,
            &mut self.backend,
            self.scale_factor,
            &self.theme,
            self.is_focused,
            &mut self.page_cache,
            self.render_debug_enabled,
            self.layout_debug_enabled,
            &mut self.diagnostics_state,
            &self.diagnostics,
        )
    }

    // ── Mount / Unmount API ───────────────────────────────────────────

    /// 페이지에 렌더링 surface를 할당하고 크기를 반환한다.
    /// page_width, page_height는 레이아웃 좌표(DPI 미적용).
    pub fn attach_surface(
        &mut self,
        page_index: u32,
        page_width: f32,
        page_height: f32,
    ) -> super::surface::SurfaceSize {
        let width = (page_width as f64 * self.scale_factor).round().max(1.0) as u32;
        let height = (page_height as f64 * self.scale_factor).round().max(1.0) as u32;
        self.attached_surfaces
            .insert(page_index, super::surface::SurfaceSize { width, height });
        super::surface::SurfaceSize { width, height }
    }

    pub fn detach_surface(&mut self, page_index: u32) {
        self.attached_surfaces.remove(&page_index);
        self.page_cache.remove(&(page_index as usize));
        self.overflow_cache.remove(&(page_index as usize));
    }

    /// 할당된 surface의 크기를 갱신한다.
    /// 크기가 변경되면 Some(새 크기)를 반환하고 캐시를 무효화한다.
    /// 변경 없으면 None.
    pub fn resize_surface(
        &mut self,
        page_index: u32,
        page_width: f32,
        page_height: f32,
    ) -> Option<super::surface::SurfaceSize> {
        let width = (page_width as f64 * self.scale_factor).round().max(1.0) as u32;
        let height = (page_height as f64 * self.scale_factor).round().max(1.0) as u32;
        let surface = self.attached_surfaces.get_mut(&page_index)?;
        if surface.width == width && surface.height == height {
            return None;
        }
        surface.width = width;
        surface.height = height;
        self.page_cache.remove(&(page_index as usize));
        self.overflow_cache.remove(&(page_index as usize));
        Some(super::surface::SurfaceSize { width, height })
    }

    // ── Size / Query ────────────────────────────────────────────────────

    /// 페이지의 전체 Vello scene을 빌드한다.
    /// WASM GPU 경로에서 surface에 직접 렌더링할 때 사용한다.
    pub fn build_surface_scene(
        &mut self,
        page: &Page,
        selections: &[SelectionDecor],
        doc: &Doc,
    ) -> vello::Scene {
        let scale = self.scale_factor as f32;
        let transform = Affine::scale_non_uniform(scale as f64, scale as f64);
        let mut sink = backend::gpu::GpuSink::new();

        for phase in RENDER_PHASES {
            let ctx = RenderParams {
                scale_factor: self.scale_factor,
                selections,
                theme: &self.theme,
                doc,
                default_text_color: None,
                is_focused: self.is_focused,
                phase,
                render_origin: Point::zero(),
            };
            Self::render_node(
                &mut sink,
                &page.root,
                Point::zero(),
                transform,
                &ctx,
                &RenderHints::default(),
                None,
            );
        }

        sink.into_scene()
    }

    pub fn render_into(
        &mut self,
        page: &Page,
        page_idx: usize,
        next_page: Option<&Page>,
        selections: &[SelectionDecor],
        drop_indicator: Option<&DropIndicator>,
        doc: &Doc,
        dst: &mut [u8],
    ) -> bool {
        let backend::RenderBackend::Cpu { ref buf, .. } = self.backend else {
            return false; // render_into는 CPU 전용
        };
        let w = buf.width();
        let h = buf.height();
        let expected_size = w as usize * h as usize * 4;
        if dst.len() < expected_size {
            return false;
        }

        let Some(mut buf) = PixelBuf::from_bytes(dst, w, h) else {
            return false;
        };

        let mut debug_frame = backend::cpu::prepare_base_layer(
            page,
            page_idx,
            doc,
            &mut self.backend,
            self.scale_factor,
            &self.theme,
            self.is_focused,
            &mut self.page_cache,
            self.render_debug_enabled,
            self.layout_debug_enabled,
            &mut self.diagnostics_state,
            &self.diagnostics,
        );
        let backend::RenderBackend::Cpu {
            ref mut scratch_buf,
            ..
        } = self.backend
        else {
            unreachable!("render_into requires CPU backend");
        };

        if let Some(cache) = self.page_cache.get(&page_idx) {
            // Phase 1: Background (cached)
            buf.data_mut().copy_from_slice(cache.background.data());

            // Phase 2: Selection (real-time)
            if !selections.is_empty() {
                let pw = buf.width();
                let ph = buf.height();
                let canvas_width = pw as f32 / self.scale_factor as f32;
                let canvas_height = ph as f32 / self.scale_factor as f32;
                let selection_data = Self::collect_selection_overlay_data(
                    page,
                    selections,
                    canvas_width,
                    canvas_height,
                );
                backend::cpu::render_selection_overlay(
                    &mut buf,
                    scratch_buf,
                    self.scale_factor,
                    &self.theme,
                    self.is_focused,
                    page,
                    selections,
                    doc,
                    &selection_data,
                );
            }

            // Phase 3: Content (cached, src-over)
            {
                let pw = buf.width();
                let ph = buf.height();
                backend::cpu::composite_cached_content_layer_clipped(
                    &mut buf,
                    &cache.content,
                    &LayoutRect::from_canvas(
                        pw as f32 / self.scale_factor as f32,
                        ph as f32 / self.scale_factor as f32,
                    )
                    .map(|r| vec![r])
                    .unwrap_or_default(),
                    self.scale_factor,
                );
            }
        } else {
            buf.data_mut().fill(0);
        }

        // Post-processing
        backend::cpu::render_post_layers(
            &mut buf,
            self.scale_factor,
            &self.theme,
            self.is_focused,
            self.render_debug_enabled,
            self.layout_debug_enabled,
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

    pub fn export_page(
        &mut self,
        page: &Page,
        next_page: Option<&Page>,
        doc: &Doc,
        page_width: f32,
        page_height: f32,
    ) -> ExportPage {
        let mut sink = ExportSink::new();

        for phase in [RenderPhase::Background, RenderPhase::Content] {
            let ctx = RenderParams {
                scale_factor: 1.0,
                selections: &[],
                theme: &self.theme,
                doc,
                default_text_color: None,
                is_focused: self.is_focused,
                phase,
                render_origin: Point::zero(),
            };

            Self::render_node(
                &mut sink,
                &page.root,
                Point::zero(),
                Affine::IDENTITY,
                &ctx,
                &RenderHints::default(),
                None,
            );
        }

        if let Some(next_page) = next_page
            && let Some(cull_clip) = next_page_overflow_cull_clip(page_width, page_height)
        {
            let ctx = RenderParams {
                scale_factor: 1.0,
                selections: &[],
                theme: &self.theme,
                doc,
                default_text_color: None,
                is_focused: self.is_focused,
                phase: RenderPhase::Content,
                render_origin: Point::zero(),
            };
            Self::render_node_for_next_page_overflow(
                &mut sink,
                &next_page.root,
                Point::new(0.0, page_height),
                Affine::IDENTITY,
                &ctx,
                &RenderHints::default(),
                cull_clip,
            );
        }

        let (ops, text_ops) = sink.into_parts();
        ExportPage {
            width: page_width,
            height: page_height,
            ops,
            text_ops,
        }
    }
}

pub fn normalize_dirty_rects(
    rects: Vec<LayoutRect>,
    canvas_width: f32,
    canvas_height: f32,
) -> Vec<LayoutRect> {
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
pub fn should_promote_full_repaint(
    rects: &[LayoutRect],
    canvas_width: f32,
    canvas_height: f32,
) -> bool {
    let full_area = canvas_width * canvas_height;
    let dirty_area: f32 = rects.iter().map(|rect| rect.area()).sum();
    rects.len() > FULL_REPAINT_RECT_THRESHOLD
        || (full_area > 0.0 && dirty_area / full_area >= FULL_REPAINT_COVERAGE_THRESHOLD)
}

pub(super) fn next_page_overflow_cull_clip(
    page_width: f32,
    page_height: f32,
) -> Option<LayoutRect> {
    LayoutRect::from_xywh(
        0.0,
        (page_height - PAGE_EDGE_OVERFLOW_BAND).max(0.0),
        page_width,
        PAGE_EDGE_OVERFLOW_BAND * 2.0,
    )
}
