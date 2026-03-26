use super::*;
impl Renderer {
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
        let mut drag_buf = PixelBuf::new(pixel_width, pixel_height)?;

        for pb in &visible_bounds {
            let page = pages.get(pb.page_idx)?;
            let page_y = page_y_offsets[pb.page_idx];

            let ctx = RenderParams {
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
                &mut drag_buf,
            )?;
        }

        let drag_page_y = page_y_offsets.get(drag_page_idx).copied().unwrap_or(0.0);

        Some(DragImageResult {
            buf: drag_buf,
            width: pixel_width as u16,
            height: pixel_height as u16,
            offset_x: min_x,
            offset_y: min_y - drag_page_y,
            scale_factor: scale,
        })
    }

    pub(super) fn compute_page_y_offsets(pages: &[Page], doc: &Doc) -> Vec<f32> {
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

    pub(super) fn compute_global_bounds(
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
    pub(super) fn render_page_part_inner(
        page: &Page,
        pb: &DragImagePageBounds,
        selections: &[SelectionDecor],
        page_y: f32,
        min_x: f32,
        min_y: f32,
        scale: f32,
        pixel_width: u32,
        pixel_height: u32,
        ctx: &RenderParams<'_>,
        drag_buf: &mut PixelBuf,
    ) -> Option<()> {
        let dest_x = pb.bounds.x - min_x;
        let dest_y = (page_y + pb.bounds.y) - min_y;

        let part_pixel_w = ((pb.bounds.width * scale).ceil() as u32).max(1);
        let part_pixel_h = ((pb.bounds.height * scale).ceil() as u32).max(1);

        let mut temp_buf = PixelBuf::new(part_pixel_w, part_pixel_h)?;
        let transform = Affine::scale_non_uniform(scale as f64, scale as f64)
            * Affine::translate((-pb.bounds.x as f64, -pb.bounds.y as f64));

        let w = temp_buf.width() as u16;
        let h = temp_buf.height() as u16;
        let mut sink = CpuSink::new(w, h);
        Self::render_node(
            &mut sink,
            &page.root,
            Point::zero(),
            transform,
            ctx,
            &RenderHints::default(),
            None,
        );
        sink.flush_to(temp_buf.data_mut(), w, h);

        let mut clip_rects = Vec::new();
        Self::collect_selection_clip_rects(
            &page.root,
            Point::zero(),
            selections,
            Point::new(pb.bounds.x, pb.bounds.y),
            scale,
            &mut clip_rects,
            None,
        );

        if clip_rects.is_empty() {
            for cr in &pb.clip_rects {
                if let Some(rect) = LayoutRect::from_xywh(
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
            &temp_buf,
            drag_buf,
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
    pub(super) fn copy_clipped_pixels(
        src: &PixelBuf,
        dest: &mut PixelBuf,
        clip_rects: &[LayoutRect],
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
            let x_start = rect.x.floor() as i32;
            let y_start = rect.y.floor() as i32;
            let x_end = (rect.x + rect.width).ceil() as i32;
            let y_end = (rect.y + rect.height).ceil() as i32;

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
}
