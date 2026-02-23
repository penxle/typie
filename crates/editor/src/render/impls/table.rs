use crate::layout::elements::{SplitEdges, TableBorderElement, TableCellElement};
use crate::model::{LayoutMode, TABLE_BORDER_WIDTH, TableBorderStyle};
use crate::render::outline::ElementSink;
use crate::render::{GlyphRenderer, Outline, RasterSink, Render, RenderContext, RenderPhase};
use tiny_skia::{Paint, PathBuilder, PixmapMut, Rect, Stroke, StrokeDash, Transform};

#[derive(Debug, Clone, Copy)]
struct BorderVisibility {
    top: bool,
    bottom: bool,
}

#[derive(Debug, Clone, Copy)]
struct BorderGeometry {
    left: f32,
    right: f32,
    top_line: f32,
    bottom_line: f32,
    vertical_top: f32,
    vertical_bottom: f32,
    row_start: f32,
}

#[derive(Debug, Clone, Copy)]
struct ContinuousBorderRange {
    top: f32,
    bottom: f32,
}

#[derive(Debug, Clone, Copy)]
struct BorderDrawSpec {
    left: f32,
    right: f32,
    horizontal_top: f32,
    horizontal_bottom: f32,
    vertical_top: f32,
    vertical_bottom: f32,
    row_start: f32,
    draw_top: bool,
    draw_bottom: bool,
}

fn border_visibility(layout_mode: &LayoutMode, split_edges: SplitEdges) -> BorderVisibility {
    match layout_mode {
        // In paginated layout, each page fragment should have complete top/bottom borders.
        LayoutMode::Paginated { .. } => BorderVisibility {
            top: true,
            bottom: true,
        },
        // Continuous rendering uses offset-aware clipping path and does not use this policy.
        LayoutMode::Continuous { .. } => BorderVisibility {
            top: !split_edges.top,
            bottom: true,
        },
    }
}

fn border_geometry(element: &TableBorderElement, visibility: BorderVisibility) -> BorderGeometry {
    let half = TABLE_BORDER_WIDTH / 2.0;
    let left = element.x_offset + half;
    let right = element.x_offset + element.size.width - half;
    // Keep vertical strokes centered on-half pixels even when the top border is
    // omitted, so split fragments remain seamless across canvas boundaries.
    let vertical_top = half;
    let vertical_bottom = element.size.height - half;

    BorderGeometry {
        left,
        right,
        top_line: if visibility.top { half } else { 0.0 },
        bottom_line: if visibility.bottom {
            vertical_bottom
        } else {
            element.size.height
        },
        vertical_top,
        vertical_bottom,
        row_start: if visibility.top {
            TABLE_BORDER_WIDTH
        } else {
            0.0
        },
    }
}

fn continuous_border_range(element: &TableBorderElement) -> ContinuousBorderRange {
    let half = TABLE_BORDER_WIDTH / 2.0;
    let full_height =
        TABLE_BORDER_WIDTH + element.row_heights.iter().sum::<f32>() + TABLE_BORDER_WIDTH;
    ContinuousBorderRange {
        top: half - element.offset,
        bottom: full_height - half - element.offset,
    }
}

fn border_dash(style: TableBorderStyle) -> Option<StrokeDash> {
    match style {
        TableBorderStyle::Dashed => StrokeDash::new(vec![4.0, 2.0], 0.0),
        TableBorderStyle::Dotted => StrokeDash::new(vec![1.0, 2.0], 0.0),
        _ => None,
    }
}

fn absolute_element_top(transform: Transform, render_origin_y: f32) -> Option<f32> {
    if transform.sy.abs() <= f32::EPSILON {
        return None;
    }
    Some(transform.ty / transform.sy + render_origin_y)
}

fn continuous_draw_spec(
    element: &TableBorderElement,
    transform: Transform,
    render_origin_y: f32,
) -> BorderDrawSpec {
    let half = TABLE_BORDER_WIDTH / 2.0;
    let range = continuous_border_range(element);
    let draw_top = !is_split_top_fragment(transform, render_origin_y);
    let horizontal_top = if draw_top {
        range.top
    } else {
        top_clip_for_split_fragment(transform, render_origin_y).unwrap_or(range.top)
    };

    BorderDrawSpec {
        left: element.x_offset + half,
        right: element.x_offset + element.size.width - half,
        horizontal_top,
        horizontal_bottom: range.bottom,
        vertical_top: range.top,
        vertical_bottom: range.bottom,
        row_start: TABLE_BORDER_WIDTH - element.offset,
        draw_top,
        draw_bottom: true,
    }
}

fn is_split_top_fragment(transform: Transform, render_origin_y: f32) -> bool {
    absolute_element_top(transform, render_origin_y).is_some_and(|top| top < -0.001)
}

fn top_clip_for_split_fragment(transform: Transform, render_origin_y: f32) -> Option<f32> {
    let element_top = absolute_element_top(transform, render_origin_y)?;
    Some(-element_top + TABLE_BORDER_WIDTH / 2.0)
}

fn paginated_draw_spec(element: &TableBorderElement, layout_mode: &LayoutMode) -> BorderDrawSpec {
    let visibility = border_visibility(layout_mode, element.split_edges);
    let geometry = border_geometry(element, visibility);

    BorderDrawSpec {
        left: geometry.left,
        right: geometry.right,
        horizontal_top: geometry.top_line,
        horizontal_bottom: geometry.bottom_line,
        vertical_top: geometry.vertical_top,
        vertical_bottom: geometry.vertical_bottom,
        row_start: geometry.row_start,
        draw_top: visibility.top,
        draw_bottom: visibility.bottom,
    }
}

fn draw_outer_lines(pb: &mut PathBuilder, spec: BorderDrawSpec) {
    if spec.draw_top {
        pb.move_to(spec.left, spec.horizontal_top);
        pb.line_to(spec.right, spec.horizontal_top);
    }

    if spec.draw_bottom {
        pb.move_to(spec.left, spec.horizontal_bottom);
        pb.line_to(spec.right, spec.horizontal_bottom);
    }

    pb.move_to(spec.left, spec.vertical_top);
    pb.line_to(spec.left, spec.vertical_bottom);
    pb.move_to(spec.right, spec.vertical_top);
    pb.line_to(spec.right, spec.vertical_bottom);
}

fn draw_row_lines(pb: &mut PathBuilder, spec: BorderDrawSpec, row_heights: &[f32]) {
    const EDGE_EPSILON: f32 = 0.001;
    let half = TABLE_BORDER_WIDTH / 2.0;
    let clip_top = if spec.draw_top {
        spec.vertical_top
    } else {
        spec.horizontal_top - half
    };
    let mut y = spec.row_start - half;
    for (idx, row_height) in row_heights.iter().enumerate() {
        y += *row_height;
        let visible_from_top = if spec.draw_top {
            y + half > clip_top + EDGE_EPSILON
        } else {
            y > clip_top + EDGE_EPSILON
        };
        if idx < row_heights.len() - 1
            && visible_from_top
            && y < spec.vertical_bottom - EDGE_EPSILON
        {
            pb.move_to(spec.left, y);
            pb.line_to(spec.right, y);
        }
    }
}

fn draw_column_lines(
    pb: &mut PathBuilder,
    x_offset: f32,
    col_widths: &[f32],
    vertical_top: f32,
    vertical_bottom: f32,
) {
    let mut x = TABLE_BORDER_WIDTH + x_offset;
    for (idx, col_width) in col_widths.iter().enumerate() {
        x += *col_width;
        if idx < col_widths.len() - 1 {
            x += TABLE_BORDER_WIDTH;
            pb.move_to(x - TABLE_BORDER_WIDTH / 2.0, vertical_top);
            pb.line_to(x - TABLE_BORDER_WIDTH / 2.0, vertical_bottom);
        }
    }
}

impl Render for TableBorderElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        let mut sink = RasterSink::new(pixmap, glyph_renderer);
        self.paint_to(&mut sink, transform, ctx);
    }
}

impl Outline for TableBorderElement {
    fn outline(&self, sink: &mut dyn ElementSink, transform: Transform, ctx: &RenderContext<'_>) {
        self.paint_to(sink, transform, ctx);
    }
}

impl TableBorderElement {
    fn paint_to(&self, sink: &mut dyn ElementSink, transform: Transform, ctx: &RenderContext<'_>) {
        match ctx.phase {
            RenderPhase::Background => {
                let mut paint = Paint::default();
                paint.set_color(ctx.theme.color("ui.surface.default"));
                if let Some(rect) =
                    Rect::from_xywh(self.x_offset, 0.0, self.size.width, self.size.height)
                {
                    sink.fill_rect(rect, &paint, transform);
                }
            }
            RenderPhase::Content => {
                if matches!(self.border_style, TableBorderStyle::None) {
                    return;
                }

                let color = ctx.theme.color("ui.border.default");
                let mut paint = Paint::default();
                paint.set_color(color);
                paint.anti_alias = true;

                let stroke = Stroke {
                    width: TABLE_BORDER_WIDTH,
                    dash: border_dash(self.border_style),
                    ..Default::default()
                };

                let mut pb = PathBuilder::new();

                let layout_mode = ctx.doc.settings().layout_mode;
                let spec = match layout_mode {
                    LayoutMode::Continuous { .. } => {
                        continuous_draw_spec(self, transform, ctx.render_origin.y)
                    }
                    LayoutMode::Paginated { .. } => paginated_draw_spec(self, &layout_mode),
                };

                draw_outer_lines(&mut pb, spec);
                draw_row_lines(&mut pb, spec, &self.row_heights);
                draw_column_lines(
                    &mut pb,
                    self.x_offset,
                    &self.col_widths,
                    spec.vertical_top,
                    spec.vertical_bottom,
                );

                if let Some(path) = pb.finish() {
                    sink.stroke_path(&path, &paint, &stroke, transform);
                }
            }
            _ => {}
        }
    }
}

impl Render for TableCellElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext,
    ) {
        let mut sink = RasterSink::new(pixmap, glyph_renderer);
        self.paint_to(&mut sink, transform, ctx);
    }
}

impl Outline for TableCellElement {
    fn outline(&self, sink: &mut dyn ElementSink, transform: Transform, ctx: &RenderContext<'_>) {
        self.paint_to(sink, transform, ctx);
    }
}

impl TableCellElement {
    fn paint_to(&self, sink: &mut dyn ElementSink, transform: Transform, ctx: &RenderContext<'_>) {
        let is_selected = ctx
            .selections
            .iter()
            .any(|s| s.is_cell() && s.node_id() == self.node_id);
        match ctx.phase {
            RenderPhase::Background => {}
            RenderPhase::Selection => {
                if is_selected {
                    let color = if ctx.is_focused {
                        ctx.theme.color_with_alpha("selection", 77)
                    } else {
                        ctx.theme.color_with_alpha("selection", 48)
                    };
                    let mut paint = Paint::default();
                    paint.set_color(color);

                    if let Some(rect) = Rect::from_xywh(0.0, 0.0, self.size.width, self.size.height)
                    {
                        sink.fill_rect(rect, &paint, transform);
                    }
                }
            }
            RenderPhase::Content => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn border_visibility_is_continuous_aware() {
        let mode = LayoutMode::Continuous { max_width: 600.0 };
        let top = border_visibility(
            &mode,
            SplitEdges {
                top: true,
                bottom: false,
            },
        );
        assert!(!top.top);
        assert!(top.bottom);

        let bottom = border_visibility(
            &mode,
            SplitEdges {
                top: false,
                bottom: true,
            },
        );
        assert!(bottom.top);
        assert!(bottom.bottom);
    }

    #[test]
    fn border_visibility_in_paginated_always_draws_outer_edges() {
        let mode = LayoutMode::Paginated {
            page_width: 800.0,
            page_height: 1000.0,
            page_margin_top: 80.0,
            page_margin_bottom: 80.0,
            page_margin_left: 80.0,
            page_margin_right: 80.0,
        };

        let vis = border_visibility(
            &mode,
            SplitEdges {
                top: true,
                bottom: true,
            },
        );
        assert!(vis.top);
        assert!(vis.bottom);
    }

    #[test]
    fn border_geometry_keeps_vertical_strokes_on_pixel_centers_for_split_top() {
        let visibility = BorderVisibility {
            top: false,
            bottom: true,
        };
        let geometry = border_geometry(
            &TableBorderElement {
                size: crate::types::Size::new(200.0, 120.0),
                node_id: crate::model::NodeId::new(),
                border_style: TableBorderStyle::Solid,
                align: crate::model::TableAlign::Left,
                rows: 3,
                cols: 2,
                row_heights: vec![40.0, 40.0, 40.0],
                col_widths: vec![99.0, 99.0],
                split_edges: SplitEdges {
                    top: true,
                    bottom: false,
                },
                offset: 0.0,
                x_offset: 0.0,
                start_row_index: 0,
                total_rows: 3,
            },
            visibility,
        );

        assert_eq!(geometry.vertical_top, TABLE_BORDER_WIDTH / 2.0);
        assert_eq!(geometry.vertical_bottom, 120.0 - TABLE_BORDER_WIDTH / 2.0);
    }

    #[test]
    fn continuous_border_range_applies_offset() {
        let range = continuous_border_range(&TableBorderElement {
            size: crate::types::Size::new(200.0, 120.0),
            node_id: crate::model::NodeId::new(),
            border_style: TableBorderStyle::Solid,
            align: crate::model::TableAlign::Left,
            rows: 3,
            cols: 2,
            row_heights: vec![40.0, 40.0, 40.0],
            col_widths: vec![99.0, 99.0],
            split_edges: SplitEdges::default(),
            offset: 40.0,
            x_offset: 0.0,
            start_row_index: 0,
            total_rows: 3,
        });

        assert_eq!(range.top, TABLE_BORDER_WIDTH / 2.0 - 40.0);
        assert_eq!(range.bottom, 121.5 - 40.0);
    }

    #[test]
    fn continuous_draw_spec_encodes_row_clip_without_cutting_verticals() {
        let spec = continuous_draw_spec(
            &TableBorderElement {
                size: crate::types::Size::new(200.0, 120.0),
                node_id: crate::model::NodeId::new(),
                border_style: TableBorderStyle::Solid,
                align: crate::model::TableAlign::Left,
                rows: 3,
                cols: 2,
                row_heights: vec![40.0, 40.0, 40.0],
                col_widths: vec![99.0, 99.0],
                split_edges: SplitEdges::default(),
                offset: 0.0,
                x_offset: 0.0,
                start_row_index: 0,
                total_rows: 3,
            },
            Transform::from_scale(2.0, 2.0).pre_translate(0.0, -501.0),
            0.0,
        );

        assert!(!spec.draw_top);
        assert!((spec.horizontal_top - 501.5).abs() < 0.01);
        assert_eq!(spec.vertical_top, TABLE_BORDER_WIDTH / 2.0);
    }

    #[test]
    fn top_clip_for_split_fragment_respects_transform_translation() {
        let transform = Transform::from_scale(2.0, 2.0).pre_translate(0.0, -501.0);
        let top_clip = top_clip_for_split_fragment(transform, 0.0).unwrap();

        assert!((top_clip - 501.5).abs() < 0.01);
    }

    #[test]
    fn tile_origin_does_not_trigger_false_split_top() {
        let spec = continuous_draw_spec(
            &TableBorderElement {
                size: crate::types::Size::new(200.0, 120.0),
                node_id: crate::model::NodeId::new(),
                border_style: TableBorderStyle::Solid,
                align: crate::model::TableAlign::Left,
                rows: 3,
                cols: 2,
                row_heights: vec![40.0, 40.0, 40.0],
                col_widths: vec![99.0, 99.0],
                split_edges: SplitEdges::default(),
                offset: 0.0,
                x_offset: 0.0,
                start_row_index: 0,
                total_rows: 3,
            },
            Transform::from_scale(2.0, 2.0).pre_translate(0.0, -120.0),
            120.0,
        );

        assert!(spec.draw_top);
        assert!((spec.horizontal_top - TABLE_BORDER_WIDTH / 2.0).abs() < 0.01);
    }

    #[test]
    fn split_top_fragment_skips_boundary_row_line_to_avoid_double_stroke() {
        let spec = continuous_draw_spec(
            &TableBorderElement {
                size: crate::types::Size::new(200.0, 902.0),
                node_id: crate::model::NodeId::new(),
                border_style: TableBorderStyle::Solid,
                align: crate::model::TableAlign::Left,
                rows: 2,
                cols: 2,
                row_heights: vec![500.0, 400.0],
                col_widths: vec![99.0, 99.0],
                split_edges: SplitEdges::default(),
                offset: 0.0,
                x_offset: 0.0,
                start_row_index: 0,
                total_rows: 2,
            },
            Transform::from_scale(1.0, 1.0).pre_translate(0.0, -501.0),
            0.0,
        );

        let mut pb = PathBuilder::new();
        draw_row_lines(&mut pb, spec, &[500.0, 400.0]);
        assert!(
            pb.finish().is_none(),
            "split top fragment should not draw a row line centered on the page seam"
        );
    }
}
