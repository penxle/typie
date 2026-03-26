use super::*;

impl Renderer {
    /// Selection overlay의 clip rect와 metadata를 수집한다.
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

        selection_data.clip_rects =
            normalize_dirty_rects(raw_clip_rects, canvas_width, canvas_height);
        selection_data
    }

    /// Drop indicator의 layout rect를 계산한다 (backend 무관).
    pub(in super::super) fn drop_indicator_layout_rect(
        indicator: &DropIndicator,
        current_page_idx: usize,
        canvas_width: f32,
        canvas_height: f32,
    ) -> Option<LayoutRect> {
        let rect = match indicator {
            DropIndicator::Inline {
                page_idx,
                x,
                y,
                height,
            } if *page_idx == current_page_idx => LayoutRect::from_xywh(*x, *y, 2.0, *height),
            DropIndicator::Block {
                page_idx,
                x,
                y,
                width,
            } if *page_idx == current_page_idx => LayoutRect::from_xywh(*x, *y - 1.0, *width, 2.0),
            _ => None,
        };
        rect.and_then(|rect| rect.clamp(canvas_width, canvas_height))
    }
}
