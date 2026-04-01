use super::*;

impl Renderer {
    /// 다음 페이지에 현재 페이지 하단으로 넘치는(overflow) 콘텐츠가 있는지 검사한다.
    pub(in super::super) fn has_visible_next_page_overflow(
        next_page: &Page,
        page_height: f32,
        cull_clip: LayoutRect,
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

                let Some(node_rect) = LayoutRect::from_xywh(
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

    /// 다음 페이지 overflow의 렌더 snapshot을 수집한다.
    pub(in super::super) fn next_page_overflow_snapshot(
        next_page: &Page,
        page_height: f32,
        cull_clip: LayoutRect,
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
        cull_clip: LayoutRect,
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

    /// 다음 페이지 overflow 영역의 디버그 rect를 수집한다.
    pub(in super::super) fn collect_next_page_overflow_debug_rects(
        next_page: &Page,
        page_width: f32,
        page_height: f32,
        cull_clip: LayoutRect,
    ) -> Vec<LayoutRect> {
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
        cull_clip: LayoutRect,
        page_width: f32,
        page_height: f32,
        out: &mut Vec<LayoutRect>,
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
                    LayoutRect::from_xywh(left, top, right - left, bottom - top)
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

    /// Overflow 영역을 sink를 통해 렌더링한다 (노드 순회 부분만).
    #[allow(clippy::too_many_arguments)]
    pub(in super::super) fn render_next_page_overflow_to_sink(
        sink: &mut dyn RenderSink,
        scale_factor: f64,
        theme: &Theme,
        next_page: &Page,
        doc: &Doc,
        page_height: f32,
        hard_clip_layout_rect: LayoutRect,
        cull_clip: LayoutRect,
    ) {
        let scale = scale_factor as f32;
        let ctx = RenderParams {
            scale_factor,
            selections: &[],
            theme,
            doc,
            default_text_color: None,
            is_focused: true,
            phase: RenderPhase::Content,
            render_origin: Point::zero(),
        };

        let transform = Affine::scale_non_uniform(scale as f64, scale as f64)
            * Affine::translate((
                -hard_clip_layout_rect.x as f64,
                -hard_clip_layout_rect.y as f64,
            ));
        Self::render_node_for_next_page_overflow(
            sink,
            &next_page.root,
            Point::new(0.0, page_height),
            transform,
            &ctx,
            &RenderHints::default(),
            cull_clip,
        );
    }
}
