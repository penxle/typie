use crate::global::GLOBALS;
use crate::layout::elements::LineElement;
use crate::render::glyph::Glyph;
use crate::render::{GlyphRenderer, Render, RenderContext, RenderPhase};
use crate::types::Point;
use macros::svg_icon_path;
use tiny_skia::{Color, Paint, PixmapMut, Rect, Stroke, Transform};

fn create_solid_paint(color: Color) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    paint
}

impl LineElement {
    fn render_line_selection(
        &self,
        pixmap: &mut PixmapMut,
        transform: Transform,
        point: Point,
        ctx: &RenderContext,
    ) {
        let color = if ctx.is_focused {
            ctx.theme.color_with_alpha("selection", 77)
        } else {
            ctx.theme.color_with_alpha("ui.surface.dark", 32)
        };
        let paint = create_solid_paint(color);
        for rect in self.compute_selection_rects(point, ctx.selections) {
            if let Some(rect) = Rect::from_xywh(rect.x, rect.y, rect.width, rect.height) {
                pixmap.fill_rect(rect, &paint, transform, None);
            }
        }
    }

    fn render_page_break(
        &self,
        pixmap: &mut PixmapMut,
        transform: Transform,
        point: Point,
        ctx: &RenderContext,
    ) {
        if !self.has_page_break {
            return;
        }

        if let Some(layout_rect) = self.page_break_indicator(point, ctx.selections) {
            let Some(rect) = Rect::from_xywh(
                layout_rect.x,
                layout_rect.y,
                layout_rect.width,
                layout_rect.height,
            ) else {
                return;
            };

            let accent_color = ctx.theme.color("ui.accent.brand.default");
            let accent_paint = create_solid_paint(accent_color);

            if let Some(line_rect) = Rect::from_xywh(
                rect.left(),
                rect.top() + rect.height() / 2.0 - 0.75,
                rect.width() - 20.0,
                1.5,
            ) {
                pixmap.fill_rect(line_rect, &accent_paint, transform, None);
            }

            let icon_size = 16.0;
            let stroke = Stroke {
                width: 1.5,
                line_cap: tiny_skia::LineCap::Round,
                line_join: tiny_skia::LineJoin::Round,
                ..Stroke::default()
            };

            let icon_x = rect.right() - icon_size / 2.0 - 2.0;
            let icon_y = rect.top() + rect.height() / 2.0;

            if let Some(path) = svg_icon_path!("lucide/file", icon_size, icon_x, icon_y) {
                pixmap.stroke_path(&path, &accent_paint, &stroke, transform, None);
            }
        }
    }

    fn render_preedit(
        &self,
        pixmap: &mut PixmapMut,
        transform: Transform,
        point: Point,
        ctx: &RenderContext,
    ) {
        let Some(preedit) = &self.preedit else {
            return;
        };

        if preedit.node_id != self.block_id {
            return;
        }

        let first = self
            .metric
            .clusters
            .iter()
            .find(|g| g.start_offset >= preedit.offset);

        let last = self.metric.clusters.iter().rev().find(|g| {
            g.end_offset <= preedit.offset + bytecount::num_chars(preedit.text.as_bytes())
        });

        let (Some(first), Some(last)) = (first, last) else {
            return;
        };

        let Some(rect) = Rect::from_xywh(
            point.x + self.metric.left + first.x,
            point.y + self.metric.top + self.metric.height,
            last.x + last.width - first.x,
            1.0,
        ) else {
            return;
        };

        let color = ctx.theme.color("ui.text.muted");
        let paint = create_solid_paint(color);
        pixmap.fill_rect(rect, &paint, transform, None);
    }

    fn render_background_segments(
        &self,
        pixmap: &mut PixmapMut,
        transform: Transform,
        line_metrics: &parley::layout::LineMetrics,
        ctx: &RenderContext<'_>,
    ) {
        if self.background_segments.is_empty() {
            return;
        }

        let line_height = line_metrics.ascent + line_metrics.descent;

        for segment in &self.background_segments {
            let color = ctx.theme.color(&format!("bg.{}", segment.color_key));

            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;
            let mut found_cluster = false;

            for cluster in &self.metric.clusters {
                if cluster.start_offset < segment.end_offset
                    && cluster.end_offset > segment.start_offset
                {
                    found_cluster = true;
                    let cluster_x = self.metric.left + cluster.x;
                    min_x = min_x.min(cluster_x);
                    max_x = max_x.max(cluster_x + cluster.width);
                }
            }

            if !found_cluster {
                continue;
            }

            if let Some(rect) = Rect::from_xywh(min_x, self.metric.top, max_x - min_x, line_height)
            {
                let paint = create_solid_paint(color);
                pixmap.fill_rect(rect, &paint, transform, None);
            }
        }
    }

    fn render_ruby_annotations(
        &self,
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        line_metrics: &parley::layout::LineMetrics,
        ctx: &RenderContext<'_>,
    ) {
        if self.ruby_segments.is_empty() {
            return;
        }

        let scale = ctx.scale_factor as f32;
        let run_y = self.metric.top + line_metrics.ascent;

        GLOBALS.with(|globals| {
            use parley::style::*;

            let globals = globals.borrow();
            let mut lcx = globals.parley_layout_context.borrow_mut();
            let mut fcx = globals.parley_font_context.borrow_mut();

            for ruby_seg in &self.ruby_segments {
                let mut min_x = f32::MAX;
                let mut max_x = f32::MIN;
                let mut found_cluster = false;

                for cluster in &self.metric.clusters {
                    if cluster.start_offset < ruby_seg.end_offset
                        && cluster.end_offset > ruby_seg.start_offset
                    {
                        found_cluster = true;
                        let cluster_x = self.metric.left + cluster.x;
                        min_x = min_x.min(cluster_x);
                        max_x = max_x.max(cluster_x + cluster.width);
                    }
                }

                if !found_cluster {
                    continue;
                }

                let base_width = max_x - min_x;
                let base_x = min_x;

                const RUBY_FONT_SIZE: f32 = 12.0;

                let mut ruby_builder =
                    lcx.ranged_builder(&mut fcx, &ruby_seg.ruby_text, 1.0, false);

                ruby_builder.push_default(StyleProperty::FontStack(FontStack::Single(
                    FontFamily::Named(ctx.doc.default_attrs().font_family().into()),
                )));
                ruby_builder.push_default(StyleProperty::FontSize(RUBY_FONT_SIZE));
                ruby_builder.push_default(StyleProperty::FontWeight(FontWeight::new(400.0)));

                let mut ruby_layout = ruby_builder.build(&ruby_seg.ruby_text);
                ruby_layout.break_all_lines(None);

                if let Some(ruby_line) = ruby_layout.lines().next() {
                    let ruby_metrics = ruby_line.metrics();
                    let ruby_width = ruby_metrics.advance;

                    let ruby_x_offset = (base_x + (base_width - ruby_width) / 2.0)
                        .clamp(0.0, (self.size.width - ruby_width).max(0.0));

                    let line_baseline = run_y + line_metrics.ascent;
                    let ruby_height = ruby_metrics.ascent + ruby_metrics.descent;
                    let ruby_y_offset = line_baseline - line_metrics.ascent - ruby_height;

                    let color = ctx.theme.color("text.black");
                    let ruby_paint = create_solid_paint(color);

                    for item in ruby_line.items() {
                        if let parley::PositionedLayoutItem::GlyphRun(glyph_run) = item {
                            let run = glyph_run.run();
                            let run_x = glyph_run.offset();

                            let mut x_advance = 0.0;
                            let glyphs: Vec<_> = glyph_run
                                .glyphs()
                                .map(|g| {
                                    let glyph_x = x_advance + g.x;
                                    x_advance += g.advance;
                                    Glyph {
                                        id: g.id,
                                        x: ruby_x_offset + run_x + glyph_x,
                                        y: ruby_y_offset + g.y,
                                    }
                                })
                                .collect();

                            glyph_renderer.draw_glyphs(
                                pixmap,
                                &run.font(),
                                RUBY_FONT_SIZE * scale,
                                &ruby_paint,
                                transform,
                                None,
                                &glyphs,
                            );
                        }
                    }
                }
            }
        });
    }
}

impl Render for LineElement {
    fn render(
        &self,
        pixmap: &mut PixmapMut,
        glyph_renderer: &mut GlyphRenderer,
        transform: Transform,
        ctx: &RenderContext<'_>,
    ) {
        let Some(line) = self.layout.lines().nth(self.line_idx) else {
            return;
        };

        let line_metrics = line.metrics();
        let point = Point::zero();

        match ctx.phase {
            RenderPhase::Background => {
                self.render_background_segments(pixmap, transform, &line_metrics, ctx);
            }
            RenderPhase::Selection => {
                self.render_line_selection(pixmap, transform, point, ctx);
                self.render_page_break(pixmap, transform, point, ctx);
            }
            RenderPhase::Content => {
                let scale = ctx.scale_factor as f32;
                let run_y = self.metric.top + line_metrics.ascent;

                for item in line.items() {
                    match item {
                        parley::PositionedLayoutItem::InlineBox(_) => {}
                        parley::PositionedLayoutItem::GlyphRun(glyph_run) => {
                            let run = glyph_run.run();
                            let style = glyph_run.style();

                            let default_text_brush =
                                format!("text.{}", ctx.doc.default_attrs().text_color());
                            let color =
                                if style.brush.is_empty() || style.brush == default_text_brush {
                                    ctx.default_text_color
                                        .unwrap_or_else(|| ctx.theme.color(&default_text_brush))
                                } else {
                                    ctx.theme.color(&style.brush)
                                };
                            let text_paint = create_solid_paint(color);

                            let run_x = glyph_run.offset();

                            let synthesis = run.synthesis();
                            let skew_transform = if synthesis.skew() != Some(0.0) {
                                synthesis.skew().map(|skew| {
                                    Transform::from_row(
                                        1.0,
                                        0.0,
                                        (skew as f64).to_radians().tan() as f32,
                                        1.0,
                                        0.0,
                                        0.0,
                                    )
                                })
                            } else {
                                None
                            };

                            let mut x_advance = 0.0;
                            let glyphs: Vec<_> = glyph_run
                                .glyphs()
                                .map(|g| {
                                    let glyph_x = x_advance + g.x;
                                    x_advance += g.advance;
                                    Glyph {
                                        id: g.id,
                                        x: run_x + glyph_x,
                                        y: run_y + g.y,
                                    }
                                })
                                .collect();

                            glyph_renderer.draw_glyphs(
                                pixmap,
                                &run.font(),
                                run.font_size() * scale,
                                &text_paint,
                                transform,
                                skew_transform,
                                &glyphs,
                            );

                            let run_width = glyph_run.advance();

                            if let Some(underline_style) = &style.underline {
                                let metrics = line_metrics;
                                let default_offset = metrics.descent * 0.5;
                                let offset = underline_style.offset.unwrap_or(default_offset);
                                let size = underline_style.size.unwrap_or(1.0);

                                if let Some(rect) =
                                    Rect::from_xywh(run_x, run_y + offset, run_width, size)
                                {
                                    pixmap.fill_rect(rect, &text_paint, transform, None);
                                }
                            }

                            if let Some(strikethrough_style) = &style.strikethrough {
                                let metrics = line_metrics;
                                let default_offset = -metrics.ascent * 0.3;
                                let offset = strikethrough_style.offset.unwrap_or(default_offset);
                                let size = strikethrough_style.size.unwrap_or(1.0);

                                if let Some(rect) =
                                    Rect::from_xywh(run_x, run_y + offset, run_width, size)
                                {
                                    pixmap.fill_rect(rect, &text_paint, transform, None);
                                }
                            }
                        }
                    }
                }

                self.render_preedit(pixmap, transform, point, ctx);
                self.render_ruby_annotations(pixmap, glyph_renderer, transform, &line_metrics, ctx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::elements::LineMetric;
    use crate::layout::{Element, Layout, LayoutCache, LayoutContext, LayoutNode};
    use crate::model::{Decorations, NodeId, SelectionDecor};
    use crate::runtime::{State, ViewStates};
    use crate::state::{build_selection_decorations, collect_blocks_in_range};
    use crate::types::{Affinity, BoxConstraints, Rect, Size};
    use std::cell::RefCell;

    fn selections_from_state(state: &State) -> Vec<SelectionDecor> {
        build_selection_decorations(&state.doc, &state.selection, None)
    }

    fn decorations_from_state(_state: &State) -> Decorations {
        Decorations {
            preedit: None,
            pending_styles: Default::default(),
        }
    }

    fn layout_for_paragraph(state: &State, para_id: NodeId) -> LayoutNode {
        let decorations = decorations_from_state(state);
        let paragraph = state.doc.node(para_id).unwrap();
        let settings = state.doc.settings();
        let default_attrs = state.doc.default_attrs();
        let cache = RefCell::new(LayoutCache::new());
        let view_states = ViewStates::default();
        let ctx = LayoutContext::new(
            &paragraph,
            &settings,
            &default_attrs,
            &decorations,
            1.0,
            &view_states,
            &cache,
        );
        let constraints = BoxConstraints::new(0.0, 400.0, 0.0, f32::INFINITY);
        paragraph.node().layout(&ctx, constraints)
    }

    fn selection_rects(line: &LineElement, selections: &[SelectionDecor]) -> Vec<Rect> {
        line.compute_selection_rects(Point::zero(), selections)
    }

    #[test]
    fn test_selection_rect_is_drawn_for_hard_break() {
        let mut p = id!();
        let hello_len = "Hello".chars().count();
        let hard_break_end = hello_len + 1;

        let state = state! {
            doc {
                @p paragraph {
                    text { "Hello" }
                    hard_break {}
                    text { "World" }
                }
            }
            selection { (p, hello_len) -> (p, hard_break_end) }
        };

        let layout = layout_for_paragraph(&state, p);
        let selections = selections_from_state(&state);

        let line_with_selection = layout
            .children
            .as_ref()
            .and_then(|children| {
                children
                    .iter()
                    .find_map(|child| match child.node.element.as_ref()? {
                        Element::Line(line) => {
                            let metric = &line.metric;
                            if metric.start_offset <= hard_break_end
                                && metric.end_offset >= hello_len
                            {
                                Some(line)
                            } else {
                                None
                            }
                        }
                        _ => None,
                    })
            })
            .expect("line containing hard break selection");

        let rects = selection_rects(line_with_selection, &selections);
        assert!(
            !rects.is_empty(),
            "hard break selection은 최소한 하나의 사각형을 그려야 함"
        );
        assert!(
            rects.iter().any(|r| r.width > 0.0),
            "hard break 선택 영역의 너비는 0보다 커야 함"
        );
    }

    #[test]
    fn test_hard_break_selection_does_not_draw_on_next_line() {
        let mut p = id!();
        let hello_len = "Hello".chars().count();
        let hard_break_end = hello_len + 1;

        let state = state! {
            doc {
                @p paragraph {
                    text { "Hello" }
                    hard_break {}
                    text { "World" }
                }
            }
            selection { (p, hello_len) -> (p, hard_break_end) }
        };

        let layout = layout_for_paragraph(&state, p);
        let selections = selections_from_state(&state);

        let mut line_rects = Vec::new();
        if let Some(children) = layout.children {
            for child in children {
                if let Some(Element::Line(line)) = &child.node.element {
                    line_rects.push(selection_rects(&line, &selections));
                }
            }
        }

        assert_eq!(line_rects.len(), 2, "두 줄이 생성되어야 함");
        assert!(
            !line_rects[0].is_empty(),
            "hard break 선택은 첫 번째 줄에서만 그려져야 함"
        );
        assert!(
            line_rects[1].is_empty(),
            "hard break 선택은 다음 줄에서 그려지면 안 됨"
        );
    }

    #[test]
    fn test_select_all_draws_selection_across_lines() {
        let mut p = id!();
        let text = "Hello\nWorld";
        let selection_end = text.chars().count();

        let state = state! {
            doc {
                @p paragraph {
                    text { "Hello" }
                    hard_break {}
                    text { "World" }
                }
            }
            selection { (p, 0) -> (p, selection_end) }
        };

        let layout = layout_for_paragraph(&state, p);
        let selections = selections_from_state(&state);

        let mut drawn = 0;
        if let Some(children) = layout.children {
            for child in children {
                if let Some(Element::Line(line)) = &child.node.element {
                    if !selection_rects(&line, &selections).is_empty() {
                        drawn += 1;
                    }
                }
            }
        }

        assert_eq!(drawn, 2, "전체 선택 시 두 줄 모두에 선택이 그려져야 함");
    }

    #[test]
    fn test_empty_paragraph_selection_draws_min_rect_for_first_block_only() {
        let mut p1 = id!();
        let mut p2 = id!();
        let state = state! {
            doc {
                @p1 paragraph { }
                @p2 paragraph { }
            }
            selection { (p1, 0) -> (p2, 0) }
        };

        let layout = layout_for_paragraph(&state, p1);
        let selections = selections_from_state(&state);

        let line = layout
            .children
            .as_ref()
            .and_then(|children| {
                children
                    .iter()
                    .find_map(|child| match child.node.element.as_ref()? {
                        Element::Line(line) => Some(line),
                        _ => None,
                    })
            })
            .expect("첫 번째 빈 문단은 라인을 하나 생성해야 함");

        let rects = selection_rects(line, &selections);
        assert!(
            rects.iter().any(|r| r.width >= 4.0),
            "빈 문단 선택은 최소 너비 4의 사각형을 첫 문단에서만 그려야 함"
        );
    }

    #[test]
    fn test_blank_line_selection_only_shows_marker_on_second_line() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    hard_break {}
                    hard_break {}
                }
            }
            selection { (p, 1) -> (p, 2) }
        };

        let layout = layout_for_paragraph(&state, p);
        let selections = selections_from_state(&state);

        let selection_instances = layout
            .children
            .as_ref()
            .expect("라인이 존재해야 함")
            .iter()
            .filter_map(|child| match child.node.element.as_ref()? {
                Element::Line(line) => Some((line.line_idx, selection_rects(line, &selections))),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(
            selection_instances
                .iter()
                .any(|(idx, rects)| *idx == 0 && rects.is_empty()),
            "첫 번째 빈 줄에는 선택 표시가 없어야 함"
        );

        let second = selection_instances
            .iter()
            .find(|(idx, _)| *idx == 1)
            .expect("두 번째 줄이 존재해야 함");

        assert!(
            second.1.len() == 1 && (second.1[0].width - 4.0).abs() < f32::EPSILON,
            "두 번째 빈 줄에는 최소 너비의 마커만 하나 그려져야 함"
        );
    }

    #[test]
    fn test_select_all_shows_hard_break_marker() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text { "Hello" }
                    hard_break {}
                    text { "World" }
                }
            }
            selection { (p, 0) -> (p, 11) }
        };

        let layout = layout_for_paragraph(&state, p);
        let selections = selections_from_state(&state);

        let first_line_rects = layout
            .children
            .as_ref()
            .and_then(|children| {
                children
                    .iter()
                    .find_map(|child| match child.node.element.as_ref()? {
                        Element::Line(line) if line.line_idx == 0 => {
                            Some(selection_rects(line, &selections))
                        }
                        _ => None,
                    })
            })
            .expect("첫 번째 줄 선택이 렌더링되어야 함");

        assert!(
            first_line_rects.iter().any(|r| (r.width - 4.0).abs() < 0.1),
            "전체 선택 시 hard break 마커(작은 사각형)가 포함되어야 함"
        );
    }

    #[test]
    fn test_consecutive_hard_breaks_render_single_marker_per_empty_line() {
        let mut p = id!();
        let state = state! {
            doc {
                @p paragraph {
                    text { "ㅁ" }
                    hard_break {}
                    hard_break {}
                }
            }
            selection { (p, 0) -> (p, 3) }
        };

        let layout = layout_for_paragraph(&state, p);
        let selections = selections_from_state(&state);
        let blank_line = layout
            .children
            .as_ref()
            .and_then(|children| {
                children
                    .iter()
                    .find_map(|child| match child.node.element.as_ref()? {
                        Element::Line(line) if line.line_idx == 1 => Some(line),
                        _ => None,
                    })
            })
            .expect("두 번째 줄이 존재해야 함");

        let rects = selection_rects(blank_line, &selections);
        assert_eq!(rects.len(), 1, "빈 줄에는 단일 마커만 그려져야 함");
        assert!(
            (rects[0].width - 4.0).abs() < 0.001,
            "빈 줄 마커 너비는 최소값과 동일해야 함"
        );
    }

    #[test]
    fn test_page_break_indicator_in_empty_paragraph() {
        let metric = LineMetric {
            top: 0.0,
            left: 0.0,
            height: 20.0,
            leading: 0.0,
            baseline: 14.0,
            ascent: 14.0,
            content_width: 100.0,
            start_offset: 0,
            end_offset: 3, // \u{200B} length
            clusters: vec![],
            break_reason: parley::layout::BreakReason::None,
            grapheme_offsets: vec![],
        };

        let line = LineElement::build(
            id!(),
            Size::new(100.0, 20.0),
            0,
            std::rc::Rc::new(parley::Layout::default()),
            metric,
            None,
            true, // is_empty
            std::rc::Rc::from("\u{200B}"),
            vec![],
            vec![],
            true, // has_page_break
        );

        let selection = SelectionDecor::Text {
            node_id: line.block_id,
            start_offset: 0,
            end_offset: 1,
        };

        let selections = [selection];
        let rects = line.compute_selection_rects(Point::zero(), &selections);
        let page_break_rect = line.page_break_indicator(Point::zero(), &selections);

        assert_eq!(
            rects.len(),
            1,
            "Should render empty paragraph rect when page break is selected"
        );
        assert!(
            page_break_rect.is_some(),
            "Page break indicator should be present"
        );
    }

    #[test]
    fn test_list_selection_decoration() {
        let mut n1 = id!();
        let mut n2 = id!();
        let mut list_p1 = id!();
        let mut list_p2 = id!();

        let state = state! {
            doc {
                @n1 paragraph {
                    text { "1" }
                }
                bullet_list {
                    list_item {
                        @list_p1 paragraph {
                            text { "2" }
                        }
                    }
                    list_item {
                        @list_p2 paragraph {
                            text { "3" }
                        }
                    }
                }
                @n2 paragraph {
                    text { "4" }
                }
                paragraph {}
            }
            selection { (n2, 1, Affinity::Upstream) -> (n1, 0) }
        };

        let (from, to) = state.selection.as_sorted(&state.doc).unwrap();
        let block_ids = collect_blocks_in_range(&state.doc, from, to).unwrap();
        let selections =
            build_selection_decorations(&state.doc, &state.selection, Some(&block_ids));

        println!("Selections: {:#?}", selections);
        println!("list_p1: {:?}", list_p1);
        println!("list_p2: {:?}", list_p2);

        let list_p1_decor = selections.iter().find(|s| s.node_id() == list_p1).unwrap();
        let list_p2_decor = selections.iter().find(|s| s.node_id() == list_p2).unwrap();

        assert_eq!(
            list_p1_decor.start_offset(),
            0,
            "list_p1 start offset mismatch"
        );
        assert_eq!(list_p1_decor.end_offset(), 1, "list_p1 end offset mismatch");

        assert_eq!(
            list_p2_decor.start_offset(),
            0,
            "list_p2 start offset mismatch"
        );
        assert_eq!(list_p2_decor.end_offset(), 1, "list_p2 end offset mismatch");
    }
}
