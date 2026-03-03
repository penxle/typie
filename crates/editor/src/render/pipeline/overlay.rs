use super::*;
impl Renderer {
    pub(in super::super) fn render_overlay_layers(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        scale_factor: f64,
        theme: &Theme,
        is_focused: bool,
        render_debug_enabled: bool,
        layout_debug_enabled: bool,
        profiling_enabled: bool,
        background_layer: Option<&Pixmap>,
        content_layer: Option<&Pixmap>,
        overflow_cache: &mut FxHashMap<usize, OverflowRenderCacheEntry>,
        page: &Page,
        page_idx: usize,
        next_page: Option<&Page>,
        selections: &[SelectionDecor],
        drop_indicator: Option<&DropIndicator>,
        doc: &Doc,
        debug_frame: &mut Option<PaintDebugFrame>,
    ) -> OverlayProfile {
        let mut overlay_profile = OverlayProfile::default();
        let scale = scale_factor as f32;
        let canvas_width = if scale > 0.0 {
            pixmap.width() as f32 / scale
        } else {
            0.0
        };
        let canvas_height = if scale > 0.0 {
            pixmap.height() as f32 / scale
        } else {
            0.0
        };

        let selection_collect_started_at = profiling_enabled.then(profile_now);
        let selection_data =
            Self::collect_selection_overlay_data(page, selections, canvas_width, canvas_height);
        overlay_profile.selection_clip_rect_count = selection_data.clip_rects.len();
        overlay_profile.selection_text_rect_count = selection_data.text_paint_rects.len();
        overlay_profile.selection_has_non_text = selection_data.has_non_text_selection;
        if let Some(started_at) = selection_collect_started_at {
            overlay_profile.selection_collect_ms = profile_elapsed_ms(started_at);
        }

        if !selection_data.clip_rects.is_empty()
            && let Some(background_layer) = background_layer
        {
            // Base pixmap에는 content가 이미 포함됨. selection 대상 영역만 background로 되돌린 뒤
            // selection/content를 다시 합성해야 content가 이중 블렌딩되지 않는다.
            Self::copy_cached_layer_to_frame_clipped(
                pixmap,
                background_layer,
                &selection_data.clip_rects,
                scale_factor,
            );
        }

        let selection_paint_started_at = profiling_enabled.then(profile_now);
        let selection_paint_stats = Self::render_selection_overlay(
            pixmap,
            glyph_renderer,
            scale_factor,
            theme,
            is_focused,
            page,
            selections,
            doc,
            &selection_data,
        );
        overlay_profile.selection_fast_path = selection_paint_stats.fast_path;
        overlay_profile.selection_phase_full = selection_paint_stats.full_phase;
        if let Some(started_at) = selection_paint_started_at {
            overlay_profile.selection_paint_ms = profile_elapsed_ms(started_at);
        }
        overlay_profile.selection_ms =
            overlay_profile.selection_collect_ms + overlay_profile.selection_paint_ms;

        let content_started_at = profiling_enabled.then(profile_now);
        if !selections.is_empty() && !selection_data.clip_rects.is_empty() {
            if should_promote_full_repaint(&selection_data.clip_rects, canvas_width, canvas_height)
            {
                let started_at = profiling_enabled.then(profile_now);
                Self::render_content_phase(
                    pixmap,
                    glyph_renderer,
                    scale_factor,
                    theme,
                    is_focused,
                    page,
                    doc,
                    None,
                    Point::zero(),
                );
                overlay_profile.content_full_render = true;
                if let Some(started_at) = started_at {
                    overlay_profile.content_full_ms = profile_elapsed_ms(started_at);
                }
            } else if let Some(content_layer) = content_layer {
                let started_at = profiling_enabled.then(profile_now);
                Self::composite_cached_content_layer_clipped(
                    pixmap,
                    content_layer,
                    &selection_data.clip_rects,
                    scale_factor,
                );
                overlay_profile.content_cached_composite = true;
                overlay_profile.content_clip_rect_count = selection_data.clip_rects.len();
                if let Some(started_at) = started_at {
                    overlay_profile.content_composite_ms = profile_elapsed_ms(started_at);
                }
            } else {
                overlay_profile.content_clipped_render = true;
                let clip_pixel_rects = collect_non_overlapping_pixel_rects(
                    &selection_data.clip_rects,
                    scale,
                    pixmap.width(),
                    pixmap.height(),
                );
                if !clip_pixel_rects.is_empty() {
                    overlay_profile.content_clip_rect_count = selection_data.clip_rects.len();
                    for pixel_rect in clip_pixel_rects {
                        let started_at = profiling_enabled.then(profile_now);
                        Self::render_content_phase_clipped(
                            pixmap,
                            glyph_renderer,
                            scale_factor,
                            theme,
                            is_focused,
                            page,
                            doc,
                            pixel_rect.to_layout_rect(scale),
                        );
                        overlay_profile.content_clipped_pass_count += 1;
                        if let Some(started_at) = started_at {
                            overlay_profile.content_clipped_ms += profile_elapsed_ms(started_at);
                        }
                    }
                }
            }
        }
        if let Some(started_at) = content_started_at {
            overlay_profile.content_ms = profile_elapsed_ms(started_at);
        }

        let mut overflow_rects = Vec::new();
        if let Some(next_page) = next_page {
            let overflow_started_at = profiling_enabled.then(profile_now);
            let overflow_debug = if debug_frame.is_some() {
                Some(&mut overflow_rects)
            } else {
                None
            };
            Self::render_next_page_overflow(
                pixmap,
                glyph_renderer,
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
            if let Some(started_at) = overflow_started_at {
                overlay_profile.overflow_ms = profile_elapsed_ms(started_at);
            }
        } else {
            overflow_cache.remove(&page_idx);
        }

        let mut drop_rect = None;
        if let Some(indicator) = drop_indicator {
            let drop_started_at = profiling_enabled.then(profile_now);
            let transform = Transform::from_scale(scale, scale);
            let overlay_ctx = RenderContext {
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
                Self::drop_indicator_layout_rect(indicator, page_idx, canvas_width, canvas_height);
            Self::render_drop_indicator(pixmap, indicator, page_idx, transform, &overlay_ctx);
            if let Some(started_at) = drop_started_at {
                overlay_profile.drop_ms = profile_elapsed_ms(started_at);
            }
        }

        if let Some(frame) = debug_frame.as_mut() {
            let mut overlay_render_rects = selection_data.clip_rects;
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
            let debug_started_at = profiling_enabled.then(profile_now);
            render_debug_overlay(
                pixmap,
                scale_factor,
                frame,
                render_debug_enabled,
                layout_debug_enabled,
            );
            if let Some(started_at) = debug_started_at {
                overlay_profile.debug_ms = profile_elapsed_ms(started_at);
            }
        }

        overlay_profile
    }

    pub(in super::super) fn render_selection_overlay(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        scale_factor: f64,
        theme: &Theme,
        is_focused: bool,
        page: &Page,
        selections: &[SelectionDecor],
        doc: &Doc,
        selection_data: &SelectionOverlayData,
    ) -> SelectionPaintStats {
        if selections.is_empty() || selection_data.clip_rects.is_empty() {
            return SelectionPaintStats::default();
        }

        let scale = scale_factor as f32;
        if scale <= 0.0 {
            return SelectionPaintStats::default();
        }

        let canvas_width = pixmap.width() as f32 / scale;
        let canvas_height = pixmap.height() as f32 / scale;
        if !selection_data.has_non_text_selection && !selection_data.text_paint_rects.is_empty() {
            let color = if is_focused {
                theme.color_with_alpha("selection", 77)
            } else {
                theme.color_with_alpha("selection", 48)
            };
            Self::fill_layout_rects_src_over(
                pixmap,
                &selection_data.text_paint_rects,
                scale_factor,
                color,
            );
            return SelectionPaintStats {
                fast_path: true,
                full_phase: false,
            };
        }

        if should_promote_full_repaint(&selection_data.clip_rects, canvas_width, canvas_height) {
            Self::render_selection_phase(
                pixmap,
                glyph_renderer,
                scale_factor,
                theme,
                is_focused,
                page,
                selections,
                doc,
                None,
                Point::zero(),
            );
            return SelectionPaintStats {
                fast_path: false,
                full_phase: true,
            };
        }

        let clip_pixel_rects = collect_non_overlapping_pixel_rects(
            &selection_data.clip_rects,
            scale,
            pixmap.width(),
            pixmap.height(),
        );
        for pixel_rect in clip_pixel_rects {
            Self::render_selection_phase_clipped(
                pixmap,
                glyph_renderer,
                scale_factor,
                theme,
                is_focused,
                page,
                selections,
                doc,
                pixel_rect.to_layout_rect(scale),
            );
        }

        SelectionPaintStats {
            fast_path: false,
            full_phase: false,
        }
    }

    pub(in super::super) fn collect_selection_overlay_data(
        page: &Page,
        selections: &[SelectionDecor],
        canvas_width: f32,
        canvas_height: f32,
    ) -> SelectionOverlayData {
        if selections.is_empty() {
            return SelectionOverlayData::default();
        }

        let mut selection_data = SelectionOverlayData::default();
        let mut raw_clip_rects = Vec::new();
        Self::collect_selection_clip_rects(
            &page.root,
            Point::zero(),
            selections,
            Point::zero(),
            1.0,
            &mut raw_clip_rects,
            Some(&mut selection_data),
        );

        selection_data.clip_rects = normalize_dirty_rects(
            raw_clip_rects
                .iter()
                .filter_map(|rect| {
                    CacheRect::from_xywh(rect.x(), rect.y(), rect.width(), rect.height())
                })
                .collect(),
            canvas_width,
            canvas_height,
        );
        selection_data
    }

    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn render_selection_phase(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        scale_factor: f64,
        theme: &Theme,
        is_focused: bool,
        page: &Page,
        selections: &[SelectionDecor],
        doc: &Doc,
        clip: Option<CacheRect>,
        origin: Point,
    ) {
        let scale = scale_factor as f32;
        let transform = Transform::from_scale(scale, scale).pre_translate(-origin.x, -origin.y);
        let ctx = RenderContext {
            scale_factor,
            selections,
            theme,
            doc,
            default_text_color: None,
            is_focused,
            phase: RenderPhase::Selection,
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

    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn render_selection_phase_clipped(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        scale_factor: f64,
        theme: &Theme,
        is_focused: bool,
        page: &Page,
        selections: &[SelectionDecor],
        doc: &Doc,
        clip_rect: CacheRect,
    ) {
        let scale = scale_factor as f32;
        let Some(pixel_rect) =
            PixelRect::from_layout_rect(clip_rect, scale, pixmap.width(), pixmap.height())
        else {
            return;
        };
        let clipped_layout_rect = pixel_rect.to_layout_rect(scale);
        let origin = Point::new(clipped_layout_rect.x, clipped_layout_rect.y);
        let Some(mut tile_pixmap) = Pixmap::new(pixel_rect.width, pixel_rect.height) else {
            Self::render_selection_phase(
                pixmap,
                glyph_renderer,
                scale_factor,
                theme,
                is_focused,
                page,
                selections,
                doc,
                Some(clipped_layout_rect),
                origin,
            );
            return;
        };

        {
            let mut tile = tile_pixmap.as_mut();
            Self::render_selection_phase(
                &mut tile,
                glyph_renderer,
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

        let paint = PixmapPaint::default();
        pixmap.draw_pixmap(
            pixel_rect.x as i32,
            pixel_rect.y as i32,
            tile_pixmap.as_ref(),
            &paint,
            Transform::identity(),
            None,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn render_content_phase(
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
            phase: RenderPhase::Content,
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

    pub(in super::super) fn render_content_phase_clipped_with_scratch(
        &mut self,
        pixmap: &mut Pixmap,
        page: &Page,
        doc: &Doc,
        clip_rect: CacheRect,
    ) {
        let scale = self.scale_factor as f32;
        let Some(pixel_rect) =
            PixelRect::from_layout_rect(clip_rect, scale, pixmap.width(), pixmap.height())
        else {
            return;
        };
        let clipped_layout_rect = pixel_rect.to_layout_rect(scale);
        let origin = Point::new(clipped_layout_rect.x, clipped_layout_rect.y);

        self.ensure_scratch_pixmap(pixel_rect.width, pixel_rect.height);
        self.clear_scratch_region(pixel_rect.width, pixel_rect.height);
        {
            let mut tile = self.scratch_pixmap.as_mut();
            Self::render_content_phase(
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
            pixmap,
            &self.scratch_pixmap,
            pixel_rect.width,
            pixel_rect.height,
            pixel_rect.x,
            pixel_rect.y,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn render_content_phase_clipped(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        scale_factor: f64,
        theme: &Theme,
        is_focused: bool,
        page: &Page,
        doc: &Doc,
        clip_rect: CacheRect,
    ) {
        let scale = scale_factor as f32;
        let Some(pixel_rect) =
            PixelRect::from_layout_rect(clip_rect, scale, pixmap.width(), pixmap.height())
        else {
            return;
        };
        let clipped_layout_rect = pixel_rect.to_layout_rect(scale);
        let origin = Point::new(clipped_layout_rect.x, clipped_layout_rect.y);
        let Some(mut tile_pixmap) = Pixmap::new(pixel_rect.width, pixel_rect.height) else {
            Self::render_content_phase(
                pixmap,
                glyph_renderer,
                scale_factor,
                theme,
                is_focused,
                page,
                doc,
                Some(clipped_layout_rect),
                origin,
            );
            return;
        };

        {
            let mut tile = tile_pixmap.as_mut();
            Self::render_content_phase(
                &mut tile,
                glyph_renderer,
                scale_factor,
                theme,
                is_focused,
                page,
                doc,
                Some(clipped_layout_rect),
                origin,
            );
        }

        let paint = PixmapPaint::default();
        pixmap.draw_pixmap(
            pixel_rect.x as i32,
            pixel_rect.y as i32,
            tile_pixmap.as_ref(),
            &paint,
            Transform::identity(),
            None,
        );
    }

    pub(in super::super) fn render_drop_indicator(
        pixmap: &mut PixmapMut,
        indicator: &DropIndicator,
        current_page_idx: usize,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        let indicator_color = ctx.theme.color("ui.accent.info.default");
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

    pub(in super::super) fn drop_indicator_layout_rect(
        indicator: &DropIndicator,
        current_page_idx: usize,
        canvas_width: f32,
        canvas_height: f32,
    ) -> Option<CacheRect> {
        let rect = match indicator {
            DropIndicator::Inline {
                page_idx,
                x,
                y,
                height,
            } if *page_idx == current_page_idx => CacheRect::from_xywh(*x, *y, 2.0, *height),
            DropIndicator::Block {
                page_idx,
                x,
                y,
                width,
            } if *page_idx == current_page_idx => CacheRect::from_xywh(*x, *y - 1.0, *width, 2.0),
            _ => None,
        };
        rect.and_then(|rect| rect.clamp(canvas_width, canvas_height))
    }
}
