use super::*;

pub fn selection_overlay_color(theme: &Theme, is_focused: bool) -> Color {
    if is_focused {
        theme.color_with_alpha("selection", 77)
    } else {
        theme.color_with_alpha("selection", 48)
    }
}

pub fn selection_overlay_brush(theme: &Theme, is_focused: bool) -> Brush {
    Brush::Solid(selection_overlay_color(theme, is_focused))
}

impl Renderer {
    pub(in super::super) fn collect_selection_clip_rects(
        positioned: &PositionedNode,
        offset: Point,
        selections: &[SelectionDecor],
        bounds_origin: Point,
        scale: f32,
        out: &mut Vec<LayoutRect>,
        selection_data: Option<&mut SelectionOverlayData>,
    ) {
        if selections.is_empty() {
            return;
        }

        let selections_by_node = Self::index_selections_by_node(selections);
        let mut selection_data = selection_data;
        Self::visit_selection_clip_rects(
            positioned,
            offset,
            &selections_by_node,
            bounds_origin,
            scale,
            out,
            &mut selection_data,
        );
    }

    fn index_selections_by_node(
        selections: &[SelectionDecor],
    ) -> FxHashMap<NodeId, Vec<SelectionDecor>> {
        let mut selections_by_node: FxHashMap<NodeId, Vec<SelectionDecor>> = FxHashMap::default();
        for selection in selections {
            selections_by_node
                .entry(selection.node_id())
                .or_default()
                .push(selection.clone());
        }
        selections_by_node
    }

    #[allow(clippy::too_many_arguments)]
    fn visit_selection_clip_rects(
        positioned: &PositionedNode,
        offset: Point,
        selections_by_node: &FxHashMap<NodeId, Vec<SelectionDecor>>,
        bounds_origin: Point,
        scale: f32,
        out: &mut Vec<LayoutRect>,
        selection_data: &mut Option<&mut SelectionOverlayData>,
    ) {
        let pos = Point::new(
            offset.x + positioned.position.x,
            offset.y + positioned.position.y,
        );

        if let Some(ref element) = positioned.node.element {
            match element {
                Element::Line(line) => {
                    if let Some(node_selections) = selections_by_node.get(&line.block_id) {
                        if let Some(data) = selection_data.as_deref_mut()
                            && line.page_break_indicator(pos, node_selections).is_some()
                        {
                            data.has_non_text_selection = true;
                        }

                        for rect in line.compute_selection_rects(pos, node_selections) {
                            if let Some(data) = selection_data.as_deref_mut()
                                && let Some(layout_rect) =
                                    LayoutRect::from_xywh(rect.x, rect.y, rect.width, rect.height)
                            {
                                data.text_paint_rects.push(layout_rect);
                            }
                            Self::push_selection_clip_rect(rect, bounds_origin, scale, out);
                        }
                    }
                }
                _ => {
                    if let Some(block_id) = element.block_id()
                        && selections_by_node.contains_key(&block_id)
                    {
                        if let Some(data) = selection_data.as_deref_mut() {
                            data.has_non_text_selection = true;
                        }
                        let node_size = &positioned.node.size;
                        if let Some(translated) = LayoutRect::from_xywh(
                            (pos.x - bounds_origin.x) * scale,
                            (pos.y - bounds_origin.y) * scale,
                            node_size.width * scale,
                            node_size.height * scale,
                        ) {
                            out.push(translated);
                        }
                    }
                }
            }
        }

        if let Some(children) = &positioned.node.children {
            for child in children {
                Self::visit_selection_clip_rects(
                    child,
                    pos,
                    selections_by_node,
                    bounds_origin,
                    scale,
                    out,
                    selection_data,
                );
            }
        }
    }

    fn push_selection_clip_rect(
        rect: crate::types::Rect,
        bounds_origin: Point,
        scale: f32,
        out: &mut Vec<LayoutRect>,
    ) {
        if let Some(translated) = LayoutRect::from_xywh(
            (rect.x - bounds_origin.x) * scale,
            (rect.y - bounds_origin.y) * scale,
            rect.width * scale,
            rect.height * scale,
        ) {
            out.push(translated);
        }
    }
}
