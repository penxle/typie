use crate::diagnostics::FrameDiagnostics;
use crate::layout::Page;
use crate::model::{Doc, SelectionDecor};
use crate::render::backend::RenderBackend;
use crate::render::backend::cpu::pixel_buf::{PixelBuf, PixelBufMut};
use crate::render::backend::cpu::sink::CpuSink;
use crate::render::cache::{PageCache, PageSnapshot};
use crate::render::debug_overlay::render_debug_overlay;
use crate::render::diagnostics::{DebugFrame, DiagnosticsState, collect_layout_dirty_rects};
use crate::render::geometry::{LayoutRect, PixelRect, clear_layout_rect};
use crate::render::renderer::{
    OverflowRenderCacheEntry, normalize_dirty_rects, should_promote_full_repaint,
};
use crate::render::{RenderParams, RenderPhase, Renderer};
use crate::runtime::DropIndicator;
use crate::types::{Point, Theme};
use kurbo::Affine;
use rustc_hash::FxHashMap;

#[allow(clippy::too_many_arguments)]
pub fn prepare_base_layer(
    page: &Page,
    page_idx: usize,
    doc: &Doc,
    backend: &mut RenderBackend,
    scale_factor: f64,
    theme: &Theme,
    is_focused: bool,
    page_cache: &mut FxHashMap<usize, PageCache>,
    render_debug_enabled: bool,
    layout_debug_enabled: bool,
    diagnostics_state: &mut DiagnosticsState,
    diagnostics: &FrameDiagnostics,
) -> Option<DebugFrame> {
    use crate::tracing::TRACER;
    use opentelemetry::trace::{Tracer, mark_span_as_active};

    let _s = mark_span_as_active(TRACER.start("render.base"));

    let mut debug_frame = (render_debug_enabled || layout_debug_enabled).then(DebugFrame::default);
    let RenderBackend::Cpu { buf, .. } = backend else {
        unreachable!("prepare_base_layer requires CPU backend");
    };

    let width = buf.width();
    let height = buf.height();
    let scale = scale_factor as f32;
    let canvas_width = width as f32 / scale;
    let canvas_height = height as f32 / scale;

    let render_snapshot = {
        let _s = mark_span_as_active(TRACER.start("render.base.snapshot"));
        PageSnapshot::from_page(page)
    };

    let _s_dirty = mark_span_as_active(TRACER.start("render.base.dirty_rects"));

    let previous_cache = page_cache.remove(&page_idx);
    let mut resize_dirty_rects = Vec::new();
    let mut cache = match previous_cache {
        Some(entry) if entry.matches(width, height, scale_factor) => entry,
        Some(entry) if entry.matches_for_height_resize(width, scale_factor) => {
            resize_dirty_rects = entry.exposed_rects_on_resize(width, height, scale);
            entry.resize_preserving_overlap(width, height, scale_factor)
        }
        Some(_) | None => PageCache::new(width, height, scale_factor),
    };

    let (mut dirty_rects, mut should_full_repaint) = PageSnapshot::compute_dirty_rects(
        Some((&cache.snapshot, cache.snapshot_initialized)),
        &render_snapshot,
        canvas_width,
        canvas_height,
    );

    if !resize_dirty_rects.is_empty() && !should_full_repaint {
        dirty_rects.extend(resize_dirty_rects);
        dirty_rects = normalize_dirty_rects(dirty_rects, canvas_width, canvas_height);
        should_full_repaint =
            should_promote_full_repaint(&dirty_rects, canvas_width, canvas_height);
    }

    drop(_s_dirty);

    if !dirty_rects.is_empty() {
        let render_rects = if should_full_repaint {
            LayoutRect::from_canvas(canvas_width, canvas_height)
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
            cache.background.data_mut().fill(0);
            cache.content.data_mut().fill(0);

            {
                let _s = mark_span_as_active(TRACER.start("render.base.background"));
                let w = cache.background.width();
                let h = cache.background.height();
                let mut background_buf = PixelBufMut::from_slice(cache.background.data_mut(), w, h);
                render_background_phase(
                    &mut background_buf,
                    scale_factor,
                    theme,
                    is_focused,
                    page,
                    doc,
                    None,
                    Point::zero(),
                );
            }

            {
                let _s = mark_span_as_active(TRACER.start("render.base.content"));
                let w = cache.content.width();
                let h = cache.content.height();
                let mut content_buf = PixelBufMut::from_slice(cache.content.data_mut(), w, h);
                render_content_phase(
                    &mut content_buf,
                    scale_factor,
                    theme,
                    is_focused,
                    page,
                    doc,
                    None,
                    Point::zero(),
                );
            }
        } else {
            for rect in &render_rects {
                {
                    let _s = mark_span_as_active(TRACER.start("render.base.background"));
                    clear_layout_rect(&mut cache.background, *rect, scale);
                    render_background_phase_clipped(
                        &mut cache.background,
                        backend,
                        scale_factor,
                        theme,
                        is_focused,
                        page,
                        doc,
                        *rect,
                    );
                }

                {
                    let _s = mark_span_as_active(TRACER.start("render.base.content"));
                    clear_layout_rect(&mut cache.content, *rect, scale);
                    render_content_phase_clipped_with_scratch(
                        &mut cache.content,
                        backend,
                        scale_factor,
                        theme,
                        is_focused,
                        page,
                        doc,
                        *rect,
                    );
                }
            }
        }
    } else if let Some(frame) = debug_frame.as_mut() {
        frame.cache_reused = true;
    }

    if layout_debug_enabled {
        if let Some(layout_pass) = diagnostics.layout_pass_snapshot() {
            let revision = layout_pass.revision;
            let mut layout_rects =
                if diagnostics_state.is_layout_revision_reused(page_idx, revision) {
                    Vec::new()
                } else {
                    collect_layout_dirty_rects(page, layout_pass.recomputed_nodes.as_ref())
                };
            layout_rects = normalize_dirty_rects(layout_rects, canvas_width, canvas_height);
            let should_full_relayout =
                should_promote_full_repaint(&layout_rects, canvas_width, canvas_height);
            let layout_rects = if should_full_relayout {
                LayoutRect::from_canvas(canvas_width, canvas_height)
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

            diagnostics_state.mark_layout_revision(page_idx, revision);
        } else if let Some(frame) = debug_frame.as_mut() {
            frame.layout_reused = false;
        }
    }

    cache.snapshot = render_snapshot;
    cache.snapshot_initialized = true;
    page_cache.insert(page_idx, cache);
    debug_frame
}

// ── CPU phase rendering (CpuSink wrapper) ────────────────────────────

/// CPU 배경 phase: CpuSink를 생성하여 pipeline의 backend-agnostic 함수를 호출한 뒤 flush.
#[allow(clippy::too_many_arguments)]
pub fn render_background_phase(
    buf: &mut PixelBufMut,
    scale_factor: f64,
    theme: &Theme,
    is_focused: bool,
    page: &Page,
    doc: &Doc,
    clip: Option<LayoutRect>,
    origin: Point,
) {
    let w = buf.width() as u16;
    let h = buf.height() as u16;
    let mut sink = CpuSink::new(w, h);
    Renderer::render_background_phase_to_sink(
        &mut sink,
        scale_factor,
        theme,
        is_focused,
        page,
        doc,
        clip,
        origin,
    );
    sink.flush_to(buf.data_mut(), w, h);
}

/// CPU selection phase: CpuSink를 생성하여 pipeline의 backend-agnostic 함수를 호출한 뒤 flush.
#[allow(clippy::too_many_arguments)]
pub fn render_selection_phase(
    buf: &mut PixelBufMut,
    scale_factor: f64,
    theme: &Theme,
    is_focused: bool,
    page: &Page,
    selections: &[SelectionDecor],
    doc: &Doc,
    clip: Option<LayoutRect>,
    origin: Point,
) {
    let w = buf.width() as u16;
    let h = buf.height() as u16;
    let mut sink = CpuSink::new(w, h);
    Renderer::render_selection_phase_to_sink(
        &mut sink,
        scale_factor,
        theme,
        is_focused,
        page,
        selections,
        doc,
        clip,
        origin,
    );
    sink.flush_to(buf.data_mut(), w, h);
}

/// CPU content phase: CpuSink를 생성하여 pipeline의 backend-agnostic 함수를 호출한 뒤 flush.
#[allow(clippy::too_many_arguments)]
pub fn render_content_phase(
    buf: &mut PixelBufMut,
    scale_factor: f64,
    theme: &Theme,
    is_focused: bool,
    page: &Page,
    doc: &Doc,
    clip: Option<LayoutRect>,
    origin: Point,
) {
    let w = buf.width() as u16;
    let h = buf.height() as u16;
    let mut sink = CpuSink::new(w, h);
    Renderer::render_content_phase_to_sink(
        &mut sink,
        scale_factor,
        theme,
        is_focused,
        page,
        doc,
        clip,
        origin,
    );
    sink.flush_to(buf.data_mut(), w, h);
}

// ── Scratch buf helpers ───────────────────────────────────────────

pub fn ensure_scratch_buf(scratch_buf: &mut PixelBuf, width: u32, height: u32) {
    if scratch_buf.width() < width || scratch_buf.height() < height {
        let new_width = scratch_buf.width().max(width).max(1);
        let new_height = scratch_buf.height().max(height).max(1);
        if let Some(new_buf) = PixelBuf::new(new_width, new_height) {
            *scratch_buf = new_buf;
        }
    }
}

pub fn clear_scratch_region(scratch_buf: &mut PixelBuf, width: u32, height: u32) {
    let stride = scratch_buf.width() as usize * 4;
    let row_bytes = width as usize * 4;
    let data = scratch_buf.data_mut();
    for row in 0..height as usize {
        let offset = row * stride;
        data[offset..offset + row_bytes].fill(0);
    }
}

pub fn blit_scratch_region(
    dst: &mut PixelBuf,
    src: &PixelBuf,
    src_width: u32,
    src_height: u32,
    dst_x: u32,
    dst_y: u32,
) {
    if src_width == 0 || src_height == 0 {
        return;
    }

    let copy_width = src_width.min(dst.width().saturating_sub(dst_x));
    let copy_height = src_height.min(dst.height().saturating_sub(dst_y));
    if copy_width == 0 || copy_height == 0 {
        return;
    }

    let src_stride = src.width() as usize * 4;
    let dst_stride = dst.width() as usize * 4;
    let row_bytes = copy_width as usize * 4;
    let src_data = src.data();
    let dst_data = dst.data_mut();
    for row in 0..copy_height as usize {
        let src_offset = row * src_stride;
        let dst_offset = (dst_y as usize + row) * dst_stride + dst_x as usize * 4;
        dst_data[dst_offset..dst_offset + row_bytes]
            .copy_from_slice(&src_data[src_offset..src_offset + row_bytes]);
    }
}

// ── Clipped phase rendering ──────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn render_background_phase_clipped(
    background_buf: &mut PixelBuf,
    backend: &mut RenderBackend,
    scale_factor: f64,
    theme: &Theme,
    is_focused: bool,
    page: &Page,
    doc: &Doc,
    clip_rect: LayoutRect,
) {
    let scale = scale_factor as f32;
    let Some(pixel_rect) = PixelRect::from_layout_rect(
        clip_rect,
        scale,
        background_buf.width(),
        background_buf.height(),
    ) else {
        return;
    };

    let clipped_layout_rect = pixel_rect.to_layout_rect(scale);
    let origin = Point::new(clipped_layout_rect.x, clipped_layout_rect.y);
    let RenderBackend::Cpu { scratch_buf, .. } = backend else {
        unreachable!("render_background_phase_clipped requires CPU backend");
    };
    ensure_scratch_buf(scratch_buf, pixel_rect.width, pixel_rect.height);
    clear_scratch_region(scratch_buf, pixel_rect.width, pixel_rect.height);
    {
        let w = scratch_buf.width();
        let h = scratch_buf.height();
        let mut tile = PixelBufMut::from_slice(scratch_buf.data_mut(), w, h);
        render_background_phase(
            &mut tile,
            scale_factor,
            theme,
            is_focused,
            page,
            doc,
            Some(clipped_layout_rect),
            origin,
        );
    }
    blit_scratch_region(
        background_buf,
        scratch_buf,
        pixel_rect.width,
        pixel_rect.height,
        pixel_rect.x,
        pixel_rect.y,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn render_selection_phase_clipped(
    buf: &mut PixelBufMut,
    scratch_buf: &mut PixelBuf,
    scale_factor: f64,
    theme: &Theme,
    is_focused: bool,
    page: &Page,
    selections: &[SelectionDecor],
    doc: &Doc,
    clip_rect: LayoutRect,
) {
    let scale = scale_factor as f32;
    let Some(pixel_rect) = PixelRect::from_layout_rect(clip_rect, scale, buf.width(), buf.height())
    else {
        return;
    };
    let clipped_layout_rect = pixel_rect.to_layout_rect(scale);
    let origin = Point::new(clipped_layout_rect.x, clipped_layout_rect.y);
    ensure_scratch_buf(scratch_buf, pixel_rect.width, pixel_rect.height);
    clear_scratch_region(scratch_buf, pixel_rect.width, pixel_rect.height);

    {
        let w = scratch_buf.width();
        let h = scratch_buf.height();
        let mut tile = PixelBufMut::from_slice(scratch_buf.data_mut(), w, h);
        render_selection_phase(
            &mut tile,
            scale_factor,
            theme,
            is_focused,
            page,
            selections,
            doc,
            Some(clipped_layout_rect),
            origin,
        );
    }

    super::compose::composite_scratch_region_src_over(
        buf,
        scratch_buf,
        pixel_rect.width,
        pixel_rect.height,
        pixel_rect.x,
        pixel_rect.y,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn render_content_phase_clipped_with_scratch(
    buf: &mut PixelBuf,
    backend: &mut RenderBackend,
    scale_factor: f64,
    theme: &Theme,
    is_focused: bool,
    page: &Page,
    doc: &Doc,
    clip_rect: LayoutRect,
) {
    let scale = scale_factor as f32;
    let Some(pixel_rect) = PixelRect::from_layout_rect(clip_rect, scale, buf.width(), buf.height())
    else {
        return;
    };
    let clipped_layout_rect = pixel_rect.to_layout_rect(scale);
    let origin = Point::new(clipped_layout_rect.x, clipped_layout_rect.y);

    let RenderBackend::Cpu { scratch_buf, .. } = backend else {
        unreachable!("render_content_phase_clipped_with_scratch requires CPU backend");
    };
    ensure_scratch_buf(scratch_buf, pixel_rect.width, pixel_rect.height);
    clear_scratch_region(scratch_buf, pixel_rect.width, pixel_rect.height);
    {
        let w = scratch_buf.width();
        let h = scratch_buf.height();
        let mut tile = PixelBufMut::from_slice(scratch_buf.data_mut(), w, h);
        render_content_phase(
            &mut tile,
            scale_factor,
            theme,
            is_focused,
            page,
            doc,
            Some(clipped_layout_rect),
            origin,
        );
    }

    blit_scratch_region(
        buf,
        scratch_buf,
        pixel_rect.width,
        pixel_rect.height,
        pixel_rect.x,
        pixel_rect.y,
    );
}

// ── Post layers ──────────────────────────────────────────────────────

/// Selection 이후의 후처리 레이어를 렌더링한다.
/// overflow, drop indicator, debug overlay를 처리한다.
#[allow(clippy::too_many_arguments)]
pub fn render_post_layers(
    buf: &mut PixelBufMut,
    scale_factor: f64,
    theme: &Theme,
    is_focused: bool,
    render_debug_enabled: bool,
    layout_debug_enabled: bool,
    overflow_cache: &mut FxHashMap<usize, OverflowRenderCacheEntry>,
    page: &Page,
    page_idx: usize,
    next_page: Option<&Page>,
    selections: &[SelectionDecor],
    drop_indicator: Option<&DropIndicator>,
    doc: &Doc,
    debug_frame: &mut Option<DebugFrame>,
) {
    use crate::tracing::TRACER;
    use opentelemetry::trace::{Tracer, mark_span_as_active};

    let _s = mark_span_as_active(TRACER.start("render.post"));
    let scale = scale_factor as f32;
    let canvas_width = if scale > 0.0 {
        buf.width() as f32 / scale
    } else {
        0.0
    };
    let canvas_height = if scale > 0.0 {
        buf.height() as f32 / scale
    } else {
        0.0
    };

    let mut overflow_rects = Vec::new();
    if let Some(next_page) = next_page {
        let _s_overflow = mark_span_as_active(TRACER.start("render.post.overflow"));
        let overflow_debug = if debug_frame.is_some() {
            Some(&mut overflow_rects)
        } else {
            None
        };
        super::overflow::render_next_page_overflow(
            buf,
            scale_factor,
            theme,
            page_idx,
            next_page,
            doc,
            overflow_cache,
            overflow_debug,
        );
        if let Some(frame) = debug_frame.as_mut() {
            frame.overflow_rects = overflow_rects.clone();
        }
        drop(_s_overflow);
    } else {
        overflow_cache.remove(&page_idx);
    }

    let mut drop_rect = None;
    if let Some(indicator) = drop_indicator {
        let _s_drop = mark_span_as_active(TRACER.start("render.post.drop_indicator"));
        let transform = Affine::scale_non_uniform(scale as f64, scale as f64);
        let params = RenderParams {
            scale_factor,
            selections,
            theme,
            doc,
            default_text_color: None,
            is_focused,
            phase: RenderPhase::Content,
            render_origin: Point::zero(),
        };
        drop_rect =
            Renderer::drop_indicator_layout_rect(indicator, page_idx, canvas_width, canvas_height);
        render_drop_indicator(buf, indicator, page_idx, transform, &params);
        drop(_s_drop);
    }

    // selection clip rects를 수집하여 debug frame에 반영
    let selection_clip_rects = if debug_frame.is_some() && !selections.is_empty() {
        let data =
            Renderer::collect_selection_overlay_data(page, selections, canvas_width, canvas_height);
        data.clip_rects
    } else {
        Vec::new()
    };

    if let Some(frame) = debug_frame.as_mut() {
        let mut overlay_render_rects = selection_clip_rects;
        overlay_render_rects.extend(overflow_rects);
        if let Some(rect) = drop_rect {
            overlay_render_rects.push(rect);
        }
        if !overlay_render_rects.is_empty() {
            frame.render_rects.extend(overlay_render_rects);
            frame.render_rects =
                normalize_dirty_rects(frame.render_rects.clone(), canvas_width, canvas_height);
            frame.full_repaint =
                should_promote_full_repaint(&frame.render_rects, canvas_width, canvas_height);
            frame.cache_reused = false;
        }
    }

    if let Some(frame) = debug_frame.as_ref() {
        let _s = mark_span_as_active(TRACER.start("render.post.debug"));
        render_debug_overlay(
            buf,
            scale_factor,
            frame,
            render_debug_enabled,
            layout_debug_enabled,
        );
    }
}

/// Drop indicator를 pixel buffer에 렌더링한다.
pub fn render_drop_indicator(
    buf: &mut PixelBufMut,
    indicator: &DropIndicator,
    current_page_idx: usize,
    transform: Affine,
    ctx: &RenderParams,
) {
    let indicator_color = ctx.theme.color("ui.accent.info.default");
    let premul = indicator_color.premultiply().to_rgba8();
    let color_pm = [premul.r, premul.g, premul.b, premul.a];

    let coeffs = transform.as_coeffs();
    let sx = coeffs[0] as f32;
    let sy = coeffs[3] as f32;
    let tx = coeffs[4] as f32;
    let ty = coeffs[5] as f32;

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
            let px = (*x * sx + tx).round() as i32;
            let py = (*y * sy + ty).round() as i32;
            let pw = (2.0 * sx).ceil() as u32;
            let ph = (*height * sy).ceil() as u32;
            super::compose::fill_pixel_rect_src_over(buf, px, py, pw, ph, color_pm);
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
            let px = (*x * sx + tx).round() as i32;
            let py = ((*y - 1.0) * sy + ty).round() as i32;
            let pw = (*width * sx).ceil() as u32;
            let ph = (2.0 * sy).ceil() as u32;
            super::compose::fill_pixel_rect_src_over(buf, px, py, pw, ph, color_pm);
        }
    }
}
