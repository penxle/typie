use super::*;

#[test]
fn partial_render_does_not_overdraw_outside_dirty_rect() {
    let callout_id = NodeId::new();
    let page1 = callout_page_with_icon(callout_id);
    let page2 = callout_page_with_icon(callout_id);

    let doc = Doc::new();
    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_size(220.0, 160.0, 1.0);

    let width = renderer.width() as usize;
    let height = renderer.height() as usize;
    let mut buffer = vec![0u8; width * height * 4];

    assert!(renderer.render_to(&page1, 0, None, &[], None, &doc, &mut buffer));
    let first = rgba_at(&buffer, width, 120, 70);
    assert!(
        first[3] > 0,
        "샘플 픽셀이 투명하면 callout 배경이 실제로 그려졌는지 검증할 수 없음"
    );

    assert!(renderer.render_to(&page2, 0, None, &[], None, &doc, &mut buffer));
    let second = rgba_at(&buffer, width, 120, 70);

    assert_eq!(
        first, second,
        "dirty rect 밖 픽셀은 부분 렌더 후에도 변하면 안 됨"
    );
}

#[test]
fn selection_overlay_keeps_content_on_top_in_selected_region() {
    let line_id = NodeId::new();
    let cell_id = NodeId::new();
    let page = root_with_children(
        Some(vec![
            PositionedNode {
                position: Point::new(40.0, 30.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(80.0, 8.0),
                    element: Some(Element::Blockquote(BlockquoteLineElement::new(
                        Size::new(80.0, 8.0),
                        line_id,
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: Some(line_id),
                }),
            },
            PositionedNode {
                position: Point::new(36.0, 26.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(88.0, 16.0),
                    element: Some(Element::TableCell(TableCellElement::new(
                        Size::new(88.0, 16.0),
                        cell_id,
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: Some(cell_id),
                }),
            },
        ]),
        Size::new(200.0, 120.0),
    );
    let doc = Doc::new();
    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_size(200.0, 120.0, 1.0);

    let mut colors = FxHashMap::default();
    colors.insert("ui.border.default".to_string(), 0xff_ff_ff_ff);
    colors.insert("selection".to_string(), 0xff_00_00_ff);
    colors.insert("ui.surface.dark".to_string(), 0x00_00_00_ff);
    renderer.set_theme(Theme { colors });

    let width = renderer.width() as usize;
    let mut plain = vec![0u8; width * renderer.height() as usize * 4];
    let mut selected = plain.clone();

    assert!(renderer.render_to(&page, 0, None, &[], None, &doc, &mut plain));
    assert!(renderer.render_to(
        &page,
        0,
        None,
        &[SelectionDecor::Cell { node_id: cell_id }],
        None,
        &doc,
        &mut selected
    ));

    let line_plain = rgba_at(&plain, width, 60, 34);
    let line_selected = rgba_at(&selected, width, 60, 34);
    let interior_plain = rgba_at(&plain, width, 60, 40);
    let interior_selected = rgba_at(&selected, width, 60, 40);

    assert!(
        line_plain[3] > 0,
        "선택 영역 내 콘텐츠 샘플 픽셀은 실제로 그려져 있어야 함"
    );
    assert_ne!(
        interior_plain, interior_selected,
        "selection overlay가 실제로 적용되는 영역이어야 테스트가 유효함"
    );
    assert_eq!(
        line_plain, line_selected,
        "선택 영역에서 콘텐츠는 selection 위에 최종 합성되어야 함"
    );
}

#[test]
fn selection_overlay_does_not_double_blend_selected_content() {
    let line_id = NodeId::new();
    let cell_id = NodeId::new();
    let page = root_with_children(
        Some(vec![
            PositionedNode {
                position: Point::new(40.0, 30.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(80.0, 8.0),
                    element: Some(Element::Blockquote(BlockquoteLineElement::new(
                        Size::new(80.0, 8.0),
                        line_id,
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: Some(line_id),
                }),
            },
            PositionedNode {
                position: Point::new(36.0, 26.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(88.0, 16.0),
                    element: Some(Element::TableCell(TableCellElement::new(
                        Size::new(88.0, 16.0),
                        cell_id,
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: Some(cell_id),
                }),
            },
        ]),
        Size::new(200.0, 120.0),
    );
    let doc = Doc::new();
    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_size(200.0, 120.0, 1.0);

    let mut colors = FxHashMap::default();
    colors.insert("ui.border.default".to_string(), 0xff_ff_ff_ff);
    colors.insert("selection".to_string(), 0xff_00_00_ff);
    colors.insert("ui.surface.dark".to_string(), 0x00_00_00_ff);
    renderer.set_theme(Theme { colors });

    let width = renderer.width() as usize;
    let height = renderer.height() as usize;
    let mut warmup = vec![0u8; width * height * 4];
    let mut actual = vec![0u8; width * height * 4];
    let selections = [SelectionDecor::Cell { node_id: cell_id }];

    assert!(renderer.render_to(&page, 0, None, &[], None, &doc, &mut warmup));
    assert!(renderer.render_to(&page, 0, None, &selections, None, &doc, &mut actual));

    let (background_layer, content_layer) = {
        let cache = renderer
            .page_cache
            .get(&0)
            .expect("render cache should exist after render");
        (
            cache.background_pixmap.clone(),
            cache.content_pixmap.clone(),
        )
    };

    let mut expected = background_layer.data().to_vec();
    let mut expected_pixmap = PixmapMut::from_bytes(
        &mut expected,
        renderer.width() as u32,
        renderer.height() as u32,
    )
    .expect("expected frame pixmap");
    let selection_data = Renderer::collect_selection_overlay_data(
        &page,
        &selections,
        renderer.width() as f32,
        renderer.height() as f32,
    );

    Renderer::render_selection_overlay(
        &mut expected_pixmap,
        &mut renderer.glyph_renderer,
        renderer.scale_factor,
        &renderer.theme,
        renderer.is_focused,
        &page,
        &selections,
        &doc,
        &selection_data,
    );
    Renderer::composite_cached_content_layer_clipped(
        &mut expected_pixmap,
        &content_layer,
        &selection_data.clip_rects,
        renderer.scale_factor,
    );

    assert_eq!(
        actual, expected,
        "선택 영역은 background -> selection -> content 순서로 단일 합성돼야 하며, content 이중 블렌딩이 없어야 함"
    );
}

#[test]
fn selection_fast_path_avoids_double_fill_for_pixel_snapped_overlap() {
    let mut pixmap = Pixmap::new(32, 32).expect("pixmap");
    let mut frame = pixmap.as_mut();
    let color = tiny_skia::Color::from_rgba8(64, 128, 255, 77);

    // 두 rect는 layout 좌표에서는 분리돼 있지만, floor/ceil 후에는 (10, 10) 픽셀에서 겹친다.
    let rects = vec![
        CacheRect::from_xywh(10.51, 0.0, 10.0, 10.49).expect("first rect"),
        CacheRect::from_xywh(0.0, 10.51, 10.49, 10.0).expect("second rect"),
    ];

    Renderer::fill_layout_rects_src_over(&mut frame, &rects, 1.0, color);

    let premul = color.premultiply().to_color_u8();
    let overlap = rgba_at(pixmap.data(), 32, 10, 10);

    assert_eq!(
        overlap,
        [premul.red(), premul.green(), premul.blue(), premul.alpha()],
        "pixel snapping으로 rect가 겹쳐도 selection tint는 한 번만 적용돼야 함"
    );
}

#[test]
fn selection_non_text_clipped_phase_avoids_double_fill_for_pixel_snapped_overlap() {
    let cell_id = NodeId::new();
    let page = root_with_children(
        Some(vec![PositionedNode {
            position: Point::zero(),
            node: Rc::new(LayoutNode {
                size: Size::new(24.0, 24.0),
                element: Some(Element::TableCell(TableCellElement::new(
                    Size::new(24.0, 24.0),
                    cell_id,
                ))),
                children: None,
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: Some(cell_id),
            }),
        }]),
        Size::new(32.0, 32.0),
    );

    let doc = Doc::new();
    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_size(32.0, 32.0, 1.0);

    let mut colors = FxHashMap::default();
    colors.insert("selection".to_string(), 0xff_00_00_ff);
    renderer.set_theme(Theme { colors });

    let mut output = vec![0u8; 32 * 32 * 4];
    let mut pixmap = PixmapMut::from_bytes(&mut output, 32, 32).expect("pixmap");
    let selection_data = SelectionOverlayData {
        clip_rects: vec![
            CacheRect::from_xywh(10.51, 0.0, 10.0, 10.49).expect("first rect"),
            CacheRect::from_xywh(0.0, 10.51, 10.49, 10.0).expect("second rect"),
        ],
        text_paint_rects: vec![],
        has_non_text_selection: true,
    };

    Renderer::render_selection_overlay(
        &mut pixmap,
        &mut renderer.glyph_renderer,
        renderer.scale_factor,
        &renderer.theme,
        renderer.is_focused,
        &page,
        &[SelectionDecor::Cell { node_id: cell_id }],
        &doc,
        &selection_data,
    );

    let expected = renderer
        .theme
        .color_with_alpha("selection", 77)
        .premultiply()
        .to_color_u8();
    let overlap = rgba_at(&output, 32, 10, 10);
    assert_eq!(
        overlap,
        [
            expected.red(),
            expected.green(),
            expected.blue(),
            expected.alpha()
        ],
        "non-text clipped selection phase에서도 pixel 중복 페인트가 없어야 함"
    );
}

#[test]
fn selection_non_text_clipped_phase_respects_disjoint_clip_regions() {
    let cell_id = NodeId::new();
    let page = root_with_children(
        Some(vec![PositionedNode {
            position: Point::zero(),
            node: Rc::new(LayoutNode {
                size: Size::new(24.0, 24.0),
                element: Some(Element::TableCell(TableCellElement::new(
                    Size::new(24.0, 24.0),
                    cell_id,
                ))),
                children: None,
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: Some(cell_id),
            }),
        }]),
        Size::new(32.0, 32.0),
    );

    let doc = Doc::new();
    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_size(32.0, 32.0, 1.0);

    let mut colors = FxHashMap::default();
    colors.insert("selection".to_string(), 0xff_00_00_ff);
    renderer.set_theme(Theme { colors });

    let mut output = vec![0u8; 32 * 32 * 4];
    let mut pixmap = PixmapMut::from_bytes(&mut output, 32, 32).expect("pixmap");
    let selection_data = SelectionOverlayData {
        clip_rects: vec![
            CacheRect::from_xywh(0.0, 0.0, 8.0, 24.0).expect("left clip"),
            CacheRect::from_xywh(16.0, 0.0, 8.0, 24.0).expect("right clip"),
        ],
        text_paint_rects: vec![],
        has_non_text_selection: true,
    };

    Renderer::render_selection_overlay(
        &mut pixmap,
        &mut renderer.glyph_renderer,
        renderer.scale_factor,
        &renderer.theme,
        renderer.is_focused,
        &page,
        &[SelectionDecor::Cell { node_id: cell_id }],
        &doc,
        &selection_data,
    );

    let expected = renderer
        .theme
        .color_with_alpha("selection", 77)
        .premultiply()
        .to_color_u8();
    let inside = rgba_at(&output, 32, 4, 12);
    let outside = rgba_at(&output, 32, 12, 12);

    assert_eq!(
        inside,
        [
            expected.red(),
            expected.green(),
            expected.blue(),
            expected.alpha()
        ],
        "clip 내부 픽셀은 selection 색으로 칠해져야 함"
    );
    assert_eq!(
        outside,
        [0, 0, 0, 0],
        "clip 밖 픽셀은 non-text selection에서도 칠해지면 안 됨"
    );
}

#[test]
fn render_debug_marker_tracks_selection_overlay_repaint() {
    let line_id = NodeId::new();
    let cell_id = NodeId::new();
    let page = root_with_children(
        Some(vec![
            PositionedNode {
                position: Point::new(40.0, 30.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(80.0, 8.0),
                    element: Some(Element::Blockquote(BlockquoteLineElement::new(
                        Size::new(80.0, 8.0),
                        line_id,
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: Some(line_id),
                }),
            },
            PositionedNode {
                position: Point::new(36.0, 26.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(88.0, 16.0),
                    element: Some(Element::TableCell(TableCellElement::new(
                        Size::new(88.0, 16.0),
                        cell_id,
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: Some(cell_id),
                }),
            },
        ]),
        Size::new(200.0, 120.0),
    );
    let doc = Doc::new();
    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_render_debug(true);
    renderer.set_size(200.0, 120.0, 1.0);

    let mut colors = FxHashMap::default();
    colors.insert("ui.border.default".to_string(), 0xff_ff_ff_ff);
    colors.insert("selection".to_string(), 0xff_00_00_ff);
    colors.insert("ui.surface.dark".to_string(), 0x00_00_00_ff);
    renderer.set_theme(Theme { colors });

    let width = renderer.width() as usize;
    let mut buffer = vec![0u8; width * renderer.height() as usize * 4];
    assert!(renderer.render_to(&page, 0, None, &[], None, &doc, &mut buffer));
    assert!(renderer.render_to(
        &page,
        0,
        None,
        &[SelectionDecor::Cell { node_id: cell_id }],
        None,
        &doc,
        &mut buffer
    ));

    let marker = rgba_at(&buffer, width, 5, 5);
    assert_eq!(
        marker,
        [255, 179, 0, 255],
        "selection/content overlay repaint가 있으면 render debug marker는 cache-reused(초록) 대신 부분 repaint(주황)여야 함"
    );
}
