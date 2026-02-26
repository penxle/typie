use super::*;
impl Renderer {
    pub(in super::super) fn render_next_page_overflow(
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        scale_factor: f64,
        theme: &Theme,
        page_idx: usize,
        next_page: &Page,
        doc: &Doc,
        overflow_cache: &mut FxHashMap<usize, OverflowRenderCacheEntry>,
        debug_rects: Option<&mut Vec<CacheRect>>,
    ) {
        let scale = scale_factor as f32;
        let page_height = pixmap.height() as f32 / scale;
        let page_width = pixmap.width() as f32 / scale;
        let Some(cull_clip) = next_page_overflow_cull_clip(page_width, page_height) else {
            overflow_cache.remove(&page_idx);
            return;
        };
        let Some(pixel_rect) =
            PixelRect::from_layout_rect(cull_clip, scale, pixmap.width(), pixmap.height())
        else {
            overflow_cache.remove(&page_idx);
            return;
        };
        if !Self::has_visible_next_page_overflow(next_page, page_height, cull_clip) {
            overflow_cache.remove(&page_idx);
            return;
        }
        let next_root_ptr = Rc::as_ptr(&next_page.root.node) as usize;
        if let Some(cache_entry) = overflow_cache.get(&page_idx)
            && same_scale_factor(cache_entry.scale_factor, scale_factor)
            && cache_entry.canvas_width == pixmap.width()
            && cache_entry.canvas_height == pixmap.height()
            && cache_entry.pixel_rect == pixel_rect
            && cache_entry.next_root_ptr == next_root_ptr
        {
            let paint = PixmapPaint::default();
            pixmap.draw_pixmap(
                pixel_rect.x as i32,
                pixel_rect.y as i32,
                cache_entry.tile_pixmap.as_ref(),
                &paint,
                Transform::identity(),
                None,
            );

            if let Some(debug_rects) = debug_rects {
                debug_rects.extend(cache_entry.debug_rects.iter().copied());
            }
            return;
        }
        let next_snapshot = Self::next_page_overflow_snapshot(next_page, page_height, cull_clip);
        if let Some(cache_entry) = overflow_cache.get(&page_idx)
            && same_scale_factor(cache_entry.scale_factor, scale_factor)
            && cache_entry.canvas_width == pixmap.width()
            && cache_entry.canvas_height == pixmap.height()
            && cache_entry.pixel_rect == pixel_rect
            && cache_entry.next_snapshot == next_snapshot
        {
            let paint = PixmapPaint::default();
            pixmap.draw_pixmap(
                pixel_rect.x as i32,
                pixel_rect.y as i32,
                cache_entry.tile_pixmap.as_ref(),
                &paint,
                Transform::identity(),
                None,
            );

            if let Some(debug_rects) = debug_rects {
                debug_rects.extend(cache_entry.debug_rects.iter().copied());
            }
            return;
        }

        let hard_clip_layout_rect = pixel_rect.to_layout_rect(scale);
        let Some(mut tile_pixmap) = Pixmap::new(pixel_rect.width, pixel_rect.height) else {
            overflow_cache.remove(&page_idx);
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
        Self::render_node_for_next_page_overflow(
            &mut tile_pixmap.as_mut(),
            glyph_renderer,
            &next_page.root,
            Point::new(0.0, page_height),
            transform,
            &ctx,
            &RenderHints::default(),
            cull_clip,
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

        let cache_debug_rects = Self::collect_next_page_overflow_debug_rects(
            next_page,
            page_width,
            page_height,
            cull_clip,
        );
        if let Some(debug_rects) = debug_rects {
            debug_rects.extend(cache_debug_rects.iter().copied());
        }

        overflow_cache.insert(
            page_idx,
            OverflowRenderCacheEntry {
                scale_factor,
                canvas_width: pixmap.width(),
                canvas_height: pixmap.height(),
                pixel_rect,
                next_root_ptr,
                next_snapshot,
                tile_pixmap,
                debug_rects: cache_debug_rects,
            },
        );
    }

    pub(in super::super) fn has_visible_next_page_overflow(
        next_page: &Page,
        page_height: f32,
        cull_clip: CacheRect,
    ) -> bool {
        let local_clip = AABB::from_corners(
            [cull_clip.x, cull_clip.y - page_height],
            [cull_clip.right(), cull_clip.bottom() - page_height],
        );
        next_page
            .spatial_index()
            .locate_in_envelope_intersecting(&local_clip)
            .any(|entry| {
                let element = entry.element();
                if element.as_render().is_none() {
                    return false;
                }
                let overflow = element.paint_overflow();
                if overflow.top <= 0.0 {
                    return false;
                }

                let Some(node_rect) = CacheRect::from_xywh(
                    entry.pos.x - overflow.left,
                    page_height + entry.pos.y - overflow.top,
                    entry.size.width + overflow.left + overflow.right,
                    entry.size.height + overflow.top + overflow.bottom,
                ) else {
                    return false;
                };
                node_rect.intersects(cull_clip)
            })
    }

    pub(in super::super) fn next_page_overflow_snapshot(
        next_page: &Page,
        page_height: f32,
        cull_clip: CacheRect,
    ) -> OverflowRenderSnapshot {
        let mut items = Vec::new();
        Self::collect_next_page_overflow_snapshot_recursive(
            &next_page.root,
            Point::new(0.0, page_height),
            cull_clip,
            &mut items,
        );
        OverflowRenderSnapshot { items }
    }

    pub(in super::super) fn collect_next_page_overflow_snapshot_recursive(
        positioned: &PositionedNode,
        offset: Point,
        cull_clip: CacheRect,
        out: &mut Vec<OverflowSnapshotItem>,
    ) {
        let pos = Point::new(
            offset.x + positioned.position.x,
            offset.y + positioned.position.y,
        );

        if let Some(node_rect) = node_paint_bounds(positioned, pos)
            && !node_rect.intersects(cull_clip)
        {
            return;
        }

        if Self::should_render_next_page_overflow(positioned)
            && let Some(Element::Line(line)) = positioned.node.element.as_ref()
        {
            out.push(OverflowSnapshotItem {
                signature: Self::overflow_line_signature(positioned, line, pos),
            });
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::collect_next_page_overflow_snapshot_recursive(child, pos, cull_clip, out);
            }
        }
    }

    pub(in super::super) fn overflow_line_signature(
        positioned: &PositionedNode,
        line: &LineElement,
        pos: Point,
    ) -> u64 {
        let mut hasher = FxHasher::default();
        line.block_id.hash(&mut hasher);
        line.line_idx.hash(&mut hasher);
        line.metric.start_offset.hash(&mut hasher);
        line.metric.end_offset.hash(&mut hasher);
        line.metric.top.to_bits().hash(&mut hasher);
        line.metric.height.to_bits().hash(&mut hasher);
        line.metric.ascent.to_bits().hash(&mut hasher);
        line.metric.ascent_overflow.to_bits().hash(&mut hasher);
        line.metric.descent_overflow.to_bits().hash(&mut hasher);
        line.has_page_break.hash(&mut hasher);
        line.is_empty.hash(&mut hasher);
        line.text.hash(&mut hasher);
        match &line.preedit {
            Some(preedit) => {
                1u8.hash(&mut hasher);
                preedit.offset.hash(&mut hasher);
                preedit.text.hash(&mut hasher);
            }
            None => {
                0u8.hash(&mut hasher);
            }
        }
        line.ruby_segments.len().hash(&mut hasher);
        for segment in &line.ruby_segments {
            segment.start_offset.hash(&mut hasher);
            segment.end_offset.hash(&mut hasher);
            segment.ruby_text.hash(&mut hasher);
        }
        positioned.node.size.width.to_bits().hash(&mut hasher);
        positioned.node.size.height.to_bits().hash(&mut hasher);
        pos.x.to_bits().hash(&mut hasher);
        pos.y.to_bits().hash(&mut hasher);
        positioned.node.scope_id.hash(&mut hasher);
        positioned
            .node
            .render_hints
            .default_text_color
            .as_ref()
            .map(|value| value.as_str())
            .hash(&mut hasher);
        hasher.finish()
    }

    pub(in super::super) fn collect_next_page_overflow_debug_rects(
        next_page: &Page,
        page_width: f32,
        page_height: f32,
        cull_clip: CacheRect,
    ) -> Vec<CacheRect> {
        let mut rects = Vec::new();
        Self::collect_overflow_debug_rects_recursive(
            &next_page.root,
            Point::new(0.0, page_height),
            cull_clip,
            page_width,
            page_height,
            &mut rects,
        );
        normalize_dirty_rects(rects, page_width, page_height)
    }

    pub(in super::super) fn collect_overflow_debug_rects_recursive(
        positioned: &PositionedNode,
        offset: Point,
        cull_clip: CacheRect,
        page_width: f32,
        page_height: f32,
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

            if Self::should_render_next_page_overflow(positioned) {
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
                    out,
                );
            }
        }
    }
}
