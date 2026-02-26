use crate::layout::cursor::{Cursor, NavigationContext};
use crate::layout::elements::TableBorderElement;
use crate::layout::{Page, PositionedNode};
use crate::model::{
    Doc, LayoutMode, Node, NodeId, TABLE_BORDER_WIDTH, TableAlign, TableBorderStyle,
    TableWidthModel,
};
use crate::runtime::Runtime;
use crate::runtime::cmd::TableOverlay;
use crate::state::Selection;
use crate::state::table_helpers::compute_table_selection;
use crate::types::{Point, Rect};
use std::collections::{HashMap, hash_map::Entry};
use std::rc::Rc;

#[derive(Debug)]
struct ContinuousTableOverlaySegment {
    page_idx: usize,
    table_id: String,
    bounds: Rect,
    page_offset: f32,
    border_style: String,
    align: String,
    proportion: f32,
    content_width: f32,
    min_proportion_width: f32,
    max_proportion_width: f32,
    col_widths: Vec<f32>,
    col_widths_as_px: Vec<f32>,
    row_heights: Vec<f32>,
    start_row_index: usize,
    total_rows: usize,
    is_focused: bool,
    show_cell_selector: bool,
}

#[derive(Debug)]
struct TableOverlayPayload {
    table_id: String,
    bounds: Rect,
    border_style: String,
    align: String,
    proportion: f32,
    content_width: f32,
    min_proportion_width: f32,
    max_proportion_width: f32,
    col_widths: Vec<f32>,
    col_widths_as_px: Vec<f32>,
    row_heights: Vec<f32>,
    start_row_index: usize,
    total_rows: usize,
}

#[derive(Debug)]
struct ContinuousTableOverlayAccum {
    table_id: String,
    border_style: String,
    align: String,
    proportion: f32,
    content_width: f32,
    min_proportion_width: f32,
    max_proportion_width: f32,
    first_page_idx: usize,
    is_focused: bool,
    show_cell_selector: bool,
    min_x: f32,
    max_right: f32,
    global_top: f32,
    global_bottom: f32,
    col_widths: Vec<f32>,
    col_widths_as_px: Vec<f32>,
    row_heights: Vec<Option<f32>>,
}

impl ContinuousTableOverlayAccum {
    fn from_segment(segment: ContinuousTableOverlaySegment) -> Self {
        let global_top = segment.page_offset + segment.bounds.y;
        let global_bottom = global_top + segment.bounds.height;

        let mut this = Self {
            table_id: segment.table_id,
            border_style: segment.border_style,
            align: segment.align,
            proportion: segment.proportion,
            content_width: segment.content_width,
            min_proportion_width: segment.min_proportion_width,
            max_proportion_width: segment.max_proportion_width,
            first_page_idx: segment.page_idx,
            is_focused: segment.is_focused,
            show_cell_selector: segment.show_cell_selector,
            min_x: segment.bounds.x,
            max_right: segment.bounds.x + segment.bounds.width,
            global_top,
            global_bottom,
            col_widths: segment.col_widths,
            col_widths_as_px: segment.col_widths_as_px,
            row_heights: vec![None; segment.total_rows],
        };
        this.apply_segment_rows(segment.start_row_index, &segment.row_heights);
        this
    }

    fn absorb(&mut self, segment: ContinuousTableOverlaySegment) {
        if segment.total_rows > self.row_heights.len() {
            self.row_heights.resize(segment.total_rows, None);
        }

        self.apply_segment_rows(segment.start_row_index, &segment.row_heights);

        if segment.col_widths_as_px.len() > self.col_widths_as_px.len() {
            self.col_widths_as_px = segment.col_widths_as_px;
        }
        if segment.col_widths.len() > self.col_widths.len() {
            self.col_widths = segment.col_widths;
        }

        let global_top = segment.page_offset + segment.bounds.y;
        let global_bottom = global_top + segment.bounds.height;

        self.first_page_idx = self.first_page_idx.min(segment.page_idx);
        self.content_width = self.content_width.max(segment.content_width);
        self.min_proportion_width = self.min_proportion_width.max(segment.min_proportion_width);
        self.max_proportion_width = self.max_proportion_width.max(segment.max_proportion_width);
        if segment.is_focused {
            self.is_focused = true;
        }
        if segment.show_cell_selector {
            self.show_cell_selector = true;
        }
        self.min_x = self.min_x.min(segment.bounds.x);
        self.max_right = self.max_right.max(segment.bounds.x + segment.bounds.width);
        self.global_top = self.global_top.min(global_top);
        self.global_bottom = self.global_bottom.max(global_bottom);
    }

    fn into_overlay(self, page_offsets: &[f32]) -> TableOverlay {
        let anchor_page_idx = self
            .first_page_idx
            .min(page_offsets.len().saturating_sub(1));
        let anchor_offset = page_offsets[anchor_page_idx];

        let fallback_row_height = self.row_heights.iter().find_map(|h| *h).unwrap_or(0.0);
        let row_heights = self
            .row_heights
            .into_iter()
            .map(|h| h.unwrap_or(fallback_row_height))
            .collect::<Vec<_>>();
        let row_positions = table_row_positions(&row_heights);
        let col_positions = table_col_positions(&self.col_widths_as_px);
        let total_rows = row_heights.len();
        let is_focused = self.is_focused;
        let show_cell_selector = self.show_cell_selector;

        TableOverlay {
            page_idx: anchor_page_idx,
            table_id: self.table_id,
            bounds: Rect {
                x: self.min_x,
                y: self.global_top - anchor_offset,
                width: (self.max_right - self.min_x).max(0.0),
                height: (self.global_bottom - self.global_top).max(0.0),
            },
            border_style: self.border_style,
            align: self.align,
            proportion: self.proportion,
            content_width: self.content_width,
            min_proportion_width: self.min_proportion_width,
            col_widths: self.col_widths,
            col_widths_as_px: self.col_widths_as_px,
            max_proportion_width: self.max_proportion_width,
            col_positions,
            row_heights,
            row_positions,
            start_row_index: 0,
            total_rows,
            is_focused,
            show_cell_selector,
        }
    }

    fn apply_segment_rows(&mut self, start_row_index: usize, row_heights: &[f32]) {
        for (idx, height) in row_heights.iter().copied().enumerate() {
            let row_idx = start_row_index + idx;
            if row_idx < self.row_heights.len() && self.row_heights[row_idx].is_none() {
                self.row_heights[row_idx] = Some(height);
            }
        }
    }
}

#[derive(Default)]
struct ContinuousOverlayBuilder {
    order: Vec<String>,
    overlays: HashMap<String, ContinuousTableOverlayAccum>,
}

impl ContinuousOverlayBuilder {
    fn push(&mut self, segment: ContinuousTableOverlaySegment) {
        let key = segment.table_id.clone();
        match self.overlays.entry(key.clone()) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().absorb(segment);
            }
            Entry::Vacant(entry) => {
                self.order.push(key);
                entry.insert(ContinuousTableOverlayAccum::from_segment(segment));
            }
        }
    }

    fn finish(mut self, page_offsets: &[f32]) -> Vec<TableOverlay> {
        if page_offsets.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(self.order.len());
        for table_id in self.order {
            if let Some(acc) = self.overlays.remove(&table_id) {
                result.push(acc.into_overlay(page_offsets));
            }
        }
        result
    }
}

impl Runtime {
    pub fn build_table_overlays(&self) -> Vec<TableOverlay> {
        let focused_page_idx =
            focused_cursor_page(&self.state.selection, &self.state.doc, self.pages());

        match self.doc().settings().layout_mode {
            LayoutMode::Paginated { .. } => collect_paginated_table_overlays(
                self.pages(),
                &self.state.selection,
                &self.state.doc,
                focused_page_idx,
            ),
            LayoutMode::Continuous { .. } => collect_continuous_table_overlays(
                self.pages(),
                &self.state.selection,
                &self.state.doc,
                focused_page_idx,
            ),
        }
    }
}

fn first_row_col_ratios(doc: &Doc, table_id: NodeId, col_count: usize) -> Option<Vec<f32>> {
    if let Some(table_node) = doc.node(table_id) {
        if let Some(first_row) = table_node.children().next() {
            let widths: Vec<Option<f32>> = first_row
                .children()
                .map(|cell| match cell.node() {
                    Some(Node::TableCell(cell_node)) => cell_node.col_width,
                    _ => None,
                })
                .collect();

            if widths.len() >= col_count
                && widths.iter().take(col_count).all(|width| width.is_some())
            {
                let ratios = widths
                    .into_iter()
                    .take(col_count)
                    .map(|width| width.unwrap_or(0.0))
                    .collect::<Vec<_>>();
                return TableWidthModel::validate_ratio_widths(&ratios, col_count);
            }
        }
    }

    None
}

fn table_col_ratios(doc: &Doc, table_id: NodeId, col_count: usize) -> Vec<f32> {
    if col_count == 0 {
        return Vec::new();
    }
    first_row_col_ratios(doc, table_id, col_count)
        .unwrap_or_else(|| vec![1.0 / col_count as f32; col_count])
}

fn table_proportion(doc: &Doc, table_id: NodeId) -> f32 {
    doc.node(table_id)
        .and_then(|node| match node.node() {
            Some(Node::Table(table_node)) => Some(table_node.proportion),
            _ => None,
        })
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 1.0))
        .unwrap_or(1.0)
}

fn table_min_proportion_width(col_count: usize, max_width: f32) -> f32 {
    if col_count == 0 {
        return 0.0;
    }

    let width_model = TableWidthModel::new(col_count, max_width.max(0.0));
    width_model.actual_table_width_for_proportion(0.0)
}

fn table_max_proportion_width(col_count: usize, max_width: f32) -> f32 {
    if col_count == 0 {
        return 0.0;
    }

    // Upper bound for proportion-resize should be the layout content width itself.
    max_width.max(0.0)
}

fn collect_paginated_table_overlays(
    pages: &[Page],
    selection: &Selection,
    doc: &Rc<Doc>,
    focused_page_idx: Option<usize>,
) -> Vec<TableOverlay> {
    let mut overlays = Vec::new();
    let cell_selector_table_id = selected_table_overlay_table_id(selection, doc.as_ref());

    for (page_idx, page) in pages.iter().enumerate() {
        visit_table_borders(
            &page.root,
            Point::zero(),
            &mut |abs_pos, table_border, table_max_width| {
                let is_focused = is_table_focused_on_page(
                    selection,
                    doc,
                    focused_page_idx,
                    page_idx,
                    table_border.node_id,
                );
                let show_cell_selector = cell_selector_table_id == Some(table_border.node_id);
                let col_widths =
                    table_col_ratios(doc.as_ref(), table_border.node_id, table_border.cols);
                let proportion = table_proportion(doc.as_ref(), table_border.node_id);
                let min_proportion_width =
                    table_min_proportion_width(table_border.cols, table_max_width);
                let max_proportion_width =
                    table_max_proportion_width(table_border.cols, table_max_width);
                overlays.push(to_paginated_overlay(
                    page_idx,
                    abs_pos,
                    table_border,
                    proportion,
                    is_focused,
                    show_cell_selector,
                    table_max_width,
                    min_proportion_width,
                    max_proportion_width,
                    col_widths,
                ));
            },
        );
    }

    overlays
}

fn collect_continuous_table_overlays(
    pages: &[Page],
    selection: &Selection,
    doc: &Rc<Doc>,
    focused_page_idx: Option<usize>,
) -> Vec<TableOverlay> {
    if pages.is_empty() {
        return Vec::new();
    }

    let page_offsets = compute_page_offsets(pages);
    let mut builder = ContinuousOverlayBuilder::default();
    let cell_selector_table_id = selected_table_overlay_table_id(selection, doc.as_ref());

    for (page_idx, page) in pages.iter().enumerate() {
        let page_offset = page_offsets[page_idx];
        visit_table_borders(
            &page.root,
            Point::zero(),
            &mut |abs_pos, table_border, table_max_width| {
                let is_focused = is_table_focused_on_page(
                    selection,
                    doc,
                    focused_page_idx,
                    page_idx,
                    table_border.node_id,
                );
                let show_cell_selector = cell_selector_table_id == Some(table_border.node_id);
                let col_widths =
                    table_col_ratios(doc.as_ref(), table_border.node_id, table_border.cols);
                let proportion = table_proportion(doc.as_ref(), table_border.node_id);
                let min_proportion_width =
                    table_min_proportion_width(table_border.cols, table_max_width);
                let max_proportion_width =
                    table_max_proportion_width(table_border.cols, table_max_width);
                builder.push(to_continuous_segment(
                    page_idx,
                    page_offset,
                    abs_pos,
                    table_border,
                    proportion,
                    is_focused,
                    show_cell_selector,
                    table_max_width,
                    min_proportion_width,
                    max_proportion_width,
                    col_widths,
                ));
            },
        );
    }

    builder.finish(&page_offsets)
}

fn visit_table_borders(
    positioned: &PositionedNode,
    offset: Point,
    visitor: &mut impl FnMut(Point, &TableBorderElement, f32),
) {
    let abs_pos = Point::new(
        offset.x + positioned.position.x,
        offset.y + positioned.position.y,
    );

    if let Some(crate::layout::Element::TableBorder(table_border)) =
        positioned.node.element.as_ref()
    {
        visitor(abs_pos, table_border, positioned.node.size.width);
    }

    if let Some(children) = &positioned.node.children {
        for child in children {
            visit_table_borders(child, abs_pos, visitor);
        }
    }
}

fn to_paginated_overlay(
    page_idx: usize,
    abs_pos: Point,
    table_border: &TableBorderElement,
    proportion: f32,
    is_focused: bool,
    show_cell_selector: bool,
    content_width: f32,
    min_proportion_width: f32,
    max_proportion_width: f32,
    col_widths: Vec<f32>,
) -> TableOverlay {
    let payload = table_overlay_payload(
        abs_pos,
        table_border,
        proportion,
        content_width,
        min_proportion_width,
        max_proportion_width,
        col_widths,
    );
    let col_positions = table_col_positions(&payload.col_widths_as_px);
    let row_positions = table_row_positions(&payload.row_heights);

    TableOverlay {
        page_idx,
        table_id: payload.table_id,
        bounds: payload.bounds,
        border_style: payload.border_style,
        align: payload.align,
        proportion: payload.proportion,
        content_width: payload.content_width,
        min_proportion_width: payload.min_proportion_width,
        max_proportion_width: payload.max_proportion_width,
        col_widths: payload.col_widths,
        col_widths_as_px: payload.col_widths_as_px,
        col_positions,
        row_heights: payload.row_heights,
        row_positions,
        start_row_index: payload.start_row_index,
        total_rows: payload.total_rows,
        is_focused,
        show_cell_selector,
    }
}

fn to_continuous_segment(
    page_idx: usize,
    page_offset: f32,
    abs_pos: Point,
    table_border: &TableBorderElement,
    proportion: f32,
    is_focused: bool,
    show_cell_selector: bool,
    content_width: f32,
    min_proportion_width: f32,
    max_proportion_width: f32,
    col_widths: Vec<f32>,
) -> ContinuousTableOverlaySegment {
    let payload = table_overlay_payload(
        abs_pos,
        table_border,
        proportion,
        content_width,
        min_proportion_width,
        max_proportion_width,
        col_widths,
    );

    ContinuousTableOverlaySegment {
        page_idx,
        table_id: payload.table_id,
        bounds: payload.bounds,
        page_offset,
        border_style: payload.border_style,
        align: payload.align,
        proportion: payload.proportion,
        content_width: payload.content_width,
        min_proportion_width: payload.min_proportion_width,
        max_proportion_width: payload.max_proportion_width,
        col_widths: payload.col_widths,
        col_widths_as_px: payload.col_widths_as_px,
        row_heights: payload.row_heights,
        start_row_index: payload.start_row_index,
        total_rows: payload.total_rows,
        is_focused,
        show_cell_selector,
    }
}

fn table_bounds(abs_pos: Point, table_border: &TableBorderElement) -> Rect {
    Rect {
        x: abs_pos.x + table_border.x_offset,
        y: abs_pos.y,
        width: table_border.size.width,
        height: table_border.size.height,
    }
}

fn table_overlay_payload(
    abs_pos: Point,
    table_border: &TableBorderElement,
    proportion: f32,
    content_width: f32,
    min_proportion_width: f32,
    max_proportion_width: f32,
    col_widths: Vec<f32>,
) -> TableOverlayPayload {
    TableOverlayPayload {
        table_id: table_border.node_id.to_string(),
        bounds: table_bounds(abs_pos, table_border),
        border_style: table_border_style_str(table_border.border_style).to_string(),
        align: table_align_str(table_border.align).to_string(),
        proportion,
        content_width,
        min_proportion_width,
        max_proportion_width,
        col_widths,
        col_widths_as_px: table_border.col_widths.clone(),
        row_heights: table_border.row_heights.clone(),
        start_row_index: table_border.start_row_index,
        total_rows: table_border.total_rows,
    }
}

fn table_col_positions(col_widths: &[f32]) -> Vec<f32> {
    let mut positions = Vec::with_capacity(col_widths.len());
    let mut x = TABLE_BORDER_WIDTH;
    for &col_width in col_widths {
        x += col_width;
        positions.push(x);
        x += TABLE_BORDER_WIDTH;
    }
    positions
}

fn table_row_positions(row_heights: &[f32]) -> Vec<f32> {
    let mut positions = Vec::with_capacity(row_heights.len());
    let mut y = 0.0;
    for &row_height in row_heights {
        y += row_height;
        positions.push(y);
    }
    positions
}

fn compute_page_offsets(pages: &[Page]) -> Vec<f32> {
    let mut offsets = Vec::with_capacity(pages.len());
    let mut acc = 0.0;
    for page in pages {
        offsets.push(acc);
        acc += page.root.node.size.height;
    }
    offsets
}

fn table_border_style_str(style: TableBorderStyle) -> &'static str {
    match style {
        TableBorderStyle::Solid => "solid",
        TableBorderStyle::Dashed => "dashed",
        TableBorderStyle::Dotted => "dotted",
        TableBorderStyle::None => "none",
    }
}

fn table_align_str(align: TableAlign) -> &'static str {
    match align {
        TableAlign::Left => "left",
        TableAlign::Center => "center",
        TableAlign::Right => "right",
    }
}

fn is_table_focused_on_page(
    selection: &Selection,
    doc: &Rc<Doc>,
    focused_page_idx: Option<usize>,
    page_idx: usize,
    table_id: NodeId,
) -> bool {
    is_cursor_in_table(selection.head.node_id, table_id, doc) && focused_page_idx == Some(page_idx)
}

fn focused_cursor_page(selection: &Selection, doc: &Rc<Doc>, pages: &[Page]) -> Option<usize> {
    let ctx = NavigationContext::new(doc);
    Cursor::bounds(&ctx, pages, selection.head).map(|(page_idx, _)| page_idx)
}

fn selected_table_overlay_table_id(selection: &Selection, doc: &Doc) -> Option<NodeId> {
    compute_table_selection(doc, selection).map(|(table_id, _)| table_id)
}

fn is_cursor_in_table(cursor_node_id: NodeId, table_id: NodeId, doc: &Rc<Doc>) -> bool {
    let Some(cursor_node) = doc.node(cursor_node_id) else {
        return false;
    };

    for ancestor in cursor_node.ancestors() {
        if ancestor.node_id() == table_id {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(
        page_idx: usize,
        page_offset: f32,
        table_id: &str,
        y: f32,
        height: f32,
        start_row_index: usize,
        row_heights: Vec<f32>,
        total_rows: usize,
        is_focused: bool,
        show_cell_selector: bool,
    ) -> ContinuousTableOverlaySegment {
        ContinuousTableOverlaySegment {
            page_idx,
            table_id: table_id.to_string(),
            bounds: Rect {
                x: 10.0,
                y,
                width: 200.0,
                height,
            },
            page_offset,
            border_style: "solid".to_string(),
            align: "left".to_string(),
            proportion: 1.0,
            content_width: 200.0,
            min_proportion_width: 100.0,
            max_proportion_width: 220.0,
            col_widths: vec![0.5, 0.5],
            col_widths_as_px: vec![80.0, 80.0],
            row_heights,
            start_row_index,
            total_rows,
            is_focused,
            show_cell_selector,
        }
    }

    #[test]
    fn builds_single_overlay_for_split_table_in_continuous_mode() {
        let mut builder = ContinuousOverlayBuilder::default();

        builder.push(segment(
            0,
            0.0,
            "table-1",
            900.0,
            124.0,
            0,
            vec![40.0, 40.0, 40.0],
            5,
            false,
            false,
        ));
        builder.push(segment(
            1,
            1024.0,
            "table-1",
            0.0,
            84.0,
            3,
            vec![40.0, 40.0],
            5,
            true,
            true,
        ));

        let merged = builder.finish(&[0.0, 1024.0]);

        assert_eq!(merged.len(), 1);
        let m = &merged[0];
        assert_eq!(m.table_id, "table-1");
        assert_eq!(m.page_idx, 0);
        assert!(m.is_focused);
        assert!(m.show_cell_selector);
        assert_eq!(m.start_row_index, 0);
        assert_eq!(m.total_rows, 5);
        assert_eq!(m.row_heights, vec![40.0, 40.0, 40.0, 40.0, 40.0]);
        assert_eq!(m.row_positions, vec![40.0, 80.0, 120.0, 160.0, 200.0]);
        assert!((m.bounds.y - 900.0).abs() < 0.01);
        assert!((m.bounds.height - 208.0).abs() < 0.01);
    }
}
