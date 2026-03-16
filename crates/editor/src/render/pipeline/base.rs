use super::*;
use crate::render::blend::{
    blend_row_const_src_over_lut, blend_row_const_src_over_opaque, build_const_src_over_lut,
};
impl Renderer {
    pub(in super::super) fn prepare_base_layer(
        &mut self,
        page: &Page,
        page_idx: usize,
        doc: &Doc,
    ) -> Option<PaintDebugFrame> {
        use crate::tracing::TRACER;
        use opentelemetry::trace::{Tracer, mark_span_as_active};

        let _s = mark_span_as_active(TRACER.start("render.base"));
        let mut debug_frame =
            (self.render_debug_enabled || self.layout_debug_enabled).then(PaintDebugFrame::default);
        let width = self.pixmap.width();
        let height = self.pixmap.height();
        let scale = self.scale_factor as f32;
        let canvas_width = width as f32 / scale;
        let canvas_height = height as f32 / scale;
        let render_snapshot = {
            let _s = mark_span_as_active(TRACER.start("render.base.snapshot"));
            PageRenderSnapshot::from_page(page)
        };

        let _s_dirty = mark_span_as_active(TRACER.start("render.base.dirty_rects"));
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
        drop(_s_dirty);

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
                cache.background_pixmap.data_mut().fill(0);
                cache.content_pixmap.data_mut().fill(0);

                {
                    let _s = mark_span_as_active(TRACER.start("render.base.background"));
                    let mut background_pixmap = cache.background_pixmap.as_mut();
                    Self::render_background_phase(
                        &mut background_pixmap,
                        &mut self.glyph_renderer,
                        self.scale_factor,
                        &self.theme,
                        self.is_focused,
                        page,
                        doc,
                        None,
                        Point::zero(),
                    );
                }

                {
                    let _s = mark_span_as_active(TRACER.start("render.base.compose"));
                    cache
                        .base_pixmap
                        .data_mut()
                        .copy_from_slice(cache.background_pixmap.data());
                }

                {
                    let _s = mark_span_as_active(TRACER.start("render.base.content"));
                    let mut content_pixmap = cache.content_pixmap.as_mut();
                    Self::render_content_phase(
                        &mut content_pixmap,
                        &mut self.glyph_renderer,
                        self.scale_factor,
                        &self.theme,
                        self.is_focused,
                        page,
                        doc,
                        None,
                        Point::zero(),
                    );
                }

                {
                    let _s = mark_span_as_active(TRACER.start("render.base.compose"));
                    let mut base_pixmap = cache.base_pixmap.as_mut();
                    Self::composite_cached_content_layer_clipped(
                        &mut base_pixmap,
                        &cache.content_pixmap,
                        &render_rects,
                        self.scale_factor,
                    );
                }
            } else {
                for rect in &render_rects {
                    {
                        let _s = mark_span_as_active(TRACER.start("render.base.background"));
                        clear_layout_rect(&mut cache.background_pixmap, *rect, scale);
                        self.render_background_phase_clipped(
                            &mut cache.background_pixmap,
                            page,
                            doc,
                            *rect,
                        );
                    }

                    {
                        let _s = mark_span_as_active(TRACER.start("render.base.compose"));
                        Self::copy_cached_layer_clipped(
                            &mut cache.base_pixmap,
                            &cache.background_pixmap,
                            &[*rect],
                            self.scale_factor,
                        );
                    }

                    {
                        let _s = mark_span_as_active(TRACER.start("render.base.content"));
                        clear_layout_rect(&mut cache.content_pixmap, *rect, scale);
                        self.render_content_phase_clipped_with_scratch(
                            &mut cache.content_pixmap,
                            page,
                            doc,
                            *rect,
                        );
                    }

                    {
                        let _s = mark_span_as_active(TRACER.start("render.base.compose"));
                        let mut base_pixmap = cache.base_pixmap.as_mut();
                        Self::composite_cached_content_layer_clipped(
                            &mut base_pixmap,
                            &cache.content_pixmap,
                            &[*rect],
                            self.scale_factor,
                        );
                    }
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

    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn render_background_phase(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        scale_factor: f64,
        theme: &Theme,
        is_focused: bool,
        page: &Page,
        doc: &Doc,
        clip: Option<CacheRect>,
        origin: Point,
    ) {
        let scale = scale_factor as f32;
        let transform = Transform::from_scale(scale, scale).pre_translate(-origin.x, -origin.y);
        let ctx = RenderContext {
            scale_factor,
            selections: &[],
            theme,
            doc,
            default_text_color: None,
            is_focused,
            phase: RenderPhase::Background,
            render_origin: origin,
        };

        Self::render_node(
            pixmap,
            glyph_renderer,
            &page.root,
            Point::zero(),
            transform,
            &ctx,
            &RenderHints::default(),
            clip,
        );
    }

    pub(in super::super) fn ensure_scratch_pixmap(
        scratch_pixmap: &mut Pixmap,
        width: u32,
        height: u32,
    ) {
        if scratch_pixmap.width() < width || scratch_pixmap.height() < height {
            let new_width = scratch_pixmap.width().max(width).max(1);
            let new_height = scratch_pixmap.height().max(height).max(1);
            if let Some(new_pixmap) = Pixmap::new(new_width, new_height) {
                *scratch_pixmap = new_pixmap;
            }
        }
    }

    pub(in super::super) fn clear_scratch_region(
        scratch_pixmap: &mut Pixmap,
        width: u32,
        height: u32,
    ) {
        let stride = scratch_pixmap.width() as usize * 4;
        let row_bytes = width as usize * 4;
        let data = scratch_pixmap.data_mut();
        for row in 0..height as usize {
            let offset = row * stride;
            data[offset..offset + row_bytes].fill(0);
        }
    }

    pub(in super::super) fn blit_scratch_region(
        dst: &mut Pixmap,
        src: &Pixmap,
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

    pub(in super::super) fn composite_scratch_region_src_over(
        dst: &mut PixmapMut,
        src: &Pixmap,
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
            let src_slice = &src_data[src_offset..src_offset + row_bytes];
            let dst_slice = &mut dst_data[dst_offset..dst_offset + row_bytes];
            blend_row_src_over(src_slice, dst_slice);
        }
    }

    pub(in super::super) fn render_background_phase_clipped(
        &mut self,
        background_pixmap: &mut Pixmap,
        page: &Page,
        doc: &Doc,
        clip_rect: CacheRect,
    ) {
        let scale = self.scale_factor as f32;
        let Some(pixel_rect) = PixelRect::from_layout_rect(
            clip_rect,
            scale,
            background_pixmap.width(),
            background_pixmap.height(),
        ) else {
            return;
        };

        let clipped_layout_rect = pixel_rect.to_layout_rect(scale);
        let origin = Point::new(clipped_layout_rect.x, clipped_layout_rect.y);
        Self::ensure_scratch_pixmap(
            &mut self.scratch_pixmap,
            pixel_rect.width,
            pixel_rect.height,
        );
        Self::clear_scratch_region(
            &mut self.scratch_pixmap,
            pixel_rect.width,
            pixel_rect.height,
        );
        {
            let mut tile = self.scratch_pixmap.as_mut();
            Self::render_background_phase(
                &mut tile,
                &mut self.glyph_renderer,
                self.scale_factor,
                &self.theme,
                self.is_focused,
                page,
                doc,
                Some(clipped_layout_rect),
                origin,
            );
        }
        Self::blit_scratch_region(
            background_pixmap,
            &self.scratch_pixmap,
            pixel_rect.width,
            pixel_rect.height,
            pixel_rect.x,
            pixel_rect.y,
        );
    }

    pub(in super::super) fn composite_cached_content_layer_clipped(
        pixmap: &mut PixmapMut,
        content_layer: &Pixmap,
        clip_rects: &[CacheRect],
        scale_factor: f64,
    ) {
        if clip_rects.is_empty() {
            return;
        }

        let scale = scale_factor as f32;
        if scale <= 0.0 {
            return;
        }

        let max_width = pixmap.width().min(content_layer.width());
        let max_height = pixmap.height().min(content_layer.height());
        let src_stride = content_layer.width() as usize * 4;
        let dst_stride = pixmap.width() as usize * 4;
        let src_data = content_layer.data();
        let dst_data = pixmap.data_mut();
        let pixel_rects =
            collect_non_overlapping_pixel_rects(clip_rects, scale, max_width, max_height);

        for pixel_rect in pixel_rects {
            let row_bytes = pixel_rect.width as usize * 4;
            let x_offset = pixel_rect.x as usize * 4;
            let y_start = pixel_rect.y as usize;
            for row in 0..pixel_rect.height as usize {
                let y = y_start + row;
                let src_offset = y * src_stride + x_offset;
                let dst_offset = y * dst_stride + x_offset;
                let src_slice = &src_data[src_offset..src_offset + row_bytes];
                let dst_slice = &mut dst_data[dst_offset..dst_offset + row_bytes];
                blend_row_src_over(src_slice, dst_slice);
            }
        }
    }

    pub(in super::super) fn copy_cached_layer_to_frame_clipped(
        dst_frame: &mut PixmapMut,
        src_layer: &Pixmap,
        clip_rects: &[CacheRect],
        scale_factor: f64,
    ) {
        if clip_rects.is_empty() {
            return;
        }

        let scale = scale_factor as f32;
        if scale <= 0.0 {
            return;
        }

        let max_width = dst_frame.width().min(src_layer.width());
        let max_height = dst_frame.height().min(src_layer.height());
        let src_stride = src_layer.width() as usize * 4;
        let dst_stride = dst_frame.width() as usize * 4;
        let src_data = src_layer.data();
        let dst_data = dst_frame.data_mut();

        for rect in clip_rects {
            let Some(pixel_rect) = PixelRect::from_layout_rect(*rect, scale, max_width, max_height)
            else {
                continue;
            };

            let row_bytes = pixel_rect.width as usize * 4;
            let x_offset = pixel_rect.x as usize * 4;
            let y_start = pixel_rect.y as usize;
            for row in 0..pixel_rect.height as usize {
                let y = y_start + row;
                let src_offset = y * src_stride + x_offset;
                let dst_offset = y * dst_stride + x_offset;
                dst_data[dst_offset..dst_offset + row_bytes]
                    .copy_from_slice(&src_data[src_offset..src_offset + row_bytes]);
            }
        }
    }

    pub(in super::super) fn copy_cached_layer_clipped(
        dst_layer: &mut Pixmap,
        src_layer: &Pixmap,
        clip_rects: &[CacheRect],
        scale_factor: f64,
    ) {
        if clip_rects.is_empty() {
            return;
        }

        let scale = scale_factor as f32;
        if scale <= 0.0 {
            return;
        }

        let max_width = dst_layer.width().min(src_layer.width());
        let max_height = dst_layer.height().min(src_layer.height());
        let src_stride = src_layer.width() as usize * 4;
        let dst_stride = dst_layer.width() as usize * 4;
        let src_data = src_layer.data();
        let dst_data = dst_layer.data_mut();

        for rect in clip_rects {
            let Some(pixel_rect) = PixelRect::from_layout_rect(*rect, scale, max_width, max_height)
            else {
                continue;
            };

            let row_bytes = pixel_rect.width as usize * 4;
            let x_offset = pixel_rect.x as usize * 4;
            let y_start = pixel_rect.y as usize;
            for row in 0..pixel_rect.height as usize {
                let y = y_start + row;
                let src_offset = y * src_stride + x_offset;
                let dst_offset = y * dst_stride + x_offset;
                dst_data[dst_offset..dst_offset + row_bytes]
                    .copy_from_slice(&src_data[src_offset..src_offset + row_bytes]);
            }
        }
    }

    pub(in super::super) fn fill_layout_rects_src_over(
        pixmap: &mut PixmapMut,
        rects: &[CacheRect],
        scale_factor: f64,
        color: Color,
    ) {
        if rects.is_empty() {
            return;
        }

        let scale = scale_factor as f32;
        if scale <= 0.0 {
            return;
        }

        let premul = color.premultiply().to_color_u8();
        let src = [premul.red(), premul.green(), premul.blue(), premul.alpha()];
        let src_alpha = src[3];
        if src_alpha == 0 {
            return;
        }
        let mut lut_r = [0u8; 256];
        let mut lut_g = [0u8; 256];
        let mut lut_b = [0u8; 256];
        let mut lut_a = [0u8; 256];
        if src_alpha != 255 {
            build_const_src_over_lut(src, &mut lut_r, &mut lut_g, &mut lut_b, &mut lut_a);
        }

        let max_width = pixmap.width();
        let max_height = pixmap.height();
        let stride = pixmap.width() as usize * 4;
        let data = pixmap.data_mut();
        let pixel_rects = collect_non_overlapping_pixel_rects(rects, scale, max_width, max_height);

        for pixel_rect in pixel_rects {
            let row_bytes = pixel_rect.width as usize * 4;
            let x_offset = pixel_rect.x as usize * 4;
            let y_start = pixel_rect.y as usize;
            for row in 0..pixel_rect.height as usize {
                let y = y_start + row;
                let row_offset = y * stride + x_offset;
                let row_slice = &mut data[row_offset..row_offset + row_bytes];
                if src_alpha == 255 {
                    blend_row_const_src_over_opaque(row_slice, src);
                } else {
                    blend_row_const_src_over_lut(row_slice, &lut_r, &lut_g, &lut_b, &lut_a);
                }
            }
        }
    }
}
