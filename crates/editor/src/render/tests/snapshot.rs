use super::*;

#[test]
fn snapshot_ignores_non_renderable_nodes() {
    let page1 = root_with_children(None, Size::new(300.0, 200.0));
    let page2 = root_with_children(None, Size::new(300.0, 200.0));

    let snapshot1 = PageRenderSnapshot::from_page(&page1);
    let snapshot2 = PageRenderSnapshot::from_page(&page2);

    assert!(
        snapshot1.dirty_rects(&snapshot2).is_empty(),
        "render 없는 루트 노드 차이로 dirty rect가 생기면 안 됨"
    );
}

#[test]
fn snapshot_reuses_renderable_child_when_root_rc_changes() {
    let shared_child = marker_node(Size::new(12.0, 12.0));

    let page1 = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(16.0, 20.0),
            node: Rc::clone(&shared_child),
        }]),
        Size::new(300.0, 200.0),
    );
    let page2 = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(16.0, 20.0),
            node: Rc::clone(&shared_child),
        }]),
        Size::new(300.0, 200.0),
    );

    let snapshot1 = PageRenderSnapshot::from_page(&page1);
    let snapshot2 = PageRenderSnapshot::from_page(&page2);

    assert!(
        snapshot1.dirty_rects(&snapshot2).is_empty(),
        "페이지 루트 포인터가 바뀌어도 동일한 렌더 노드는 dirty로 잡히면 안 됨"
    );
}

#[test]
fn snapshot_reuses_wrapper_by_stable_identity() {
    let fold_id = NodeId::new();

    let page1 = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(8.0, 12.0),
            node: Rc::new(LayoutNode {
                size: Size::new(240.0, 80.0),
                element: Some(Element::FoldContent(FoldContentElement::new(
                    Size::new(240.0, 80.0),
                    SplitEdges::default(),
                    fold_id,
                ))),
                children: None,
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: None,
            }),
        }]),
        Size::new(300.0, 200.0),
    );
    let page2 = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(8.0, 12.0),
            node: Rc::new(LayoutNode {
                size: Size::new(240.0, 80.0),
                element: Some(Element::FoldContent(FoldContentElement::new(
                    Size::new(240.0, 80.0),
                    SplitEdges::default(),
                    fold_id,
                ))),
                children: None,
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: None,
            }),
        }]),
        Size::new(300.0, 200.0),
    );

    let snapshot1 = PageRenderSnapshot::from_page(&page1);
    let snapshot2 = PageRenderSnapshot::from_page(&page2);

    assert!(
        snapshot1.dirty_rects(&snapshot2).is_empty(),
        "wrapper가 매 프레임 재생성되어도 안정 키로 cache diff가 유지돼야 함"
    );
}

#[test]
fn snapshot_expands_dirty_rect_for_message_blockquote_tail() {
    let block_id = NodeId::new();
    let bubble_x = 40.0;
    let bubble_y = 30.0;
    let bubble_width = 120.0;
    let bubble_height = 40.0;

    let make_page = |variant| {
        root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(bubble_x, bubble_y),
                node: Rc::new(LayoutNode {
                    size: Size::new(bubble_width, bubble_height),
                    element: Some(Element::BlockquoteMessage(BlockquoteMessageElement::new(
                        Size::new(bubble_width, bubble_height),
                        block_id,
                        variant,
                        SplitEdges::default(),
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: None,
                }),
            }]),
            Size::new(300.0, 200.0),
        )
    };

    let sent_page = make_page(BlockquoteVariant::MessageSent);
    let received_page = make_page(BlockquoteVariant::MessageReceived);

    let snapshot_sent = PageRenderSnapshot::from_page(&sent_page);
    let snapshot_received = PageRenderSnapshot::from_page(&received_page);
    let rects = snapshot_sent.dirty_rects(&snapshot_received);

    assert!(
        rects
            .iter()
            .any(|rect| rect.right() > bubble_x + bubble_width),
        "sent tail 영역을 지우기 위해 오른쪽 dirty rect 확장이 필요함"
    );
    assert!(
        rects.iter().any(|rect| rect.x < bubble_x),
        "received tail 영역을 그리기 위해 왼쪽 dirty rect 확장이 필요함"
    );
}

#[test]
fn snapshot_ignores_selection_only_table_cell_element() {
    let cell_id = NodeId::new();

    let page1 = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(20.0, 24.0),
            node: Rc::new(LayoutNode {
                size: Size::new(120.0, 48.0),
                element: Some(Element::TableCell(TableCellElement::new(
                    Size::new(120.0, 48.0),
                    cell_id,
                ))),
                children: None,
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: Some(cell_id),
            }),
        }]),
        Size::new(300.0, 200.0),
    );
    let page2 = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(20.0, 24.0),
            node: Rc::new(LayoutNode {
                size: Size::new(120.0, 48.0),
                element: Some(Element::TableCell(TableCellElement::new(
                    Size::new(120.0, 48.0),
                    cell_id,
                ))),
                children: None,
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: Some(cell_id),
            }),
        }]),
        Size::new(300.0, 200.0),
    );

    let snapshot1 = PageRenderSnapshot::from_page(&page1);
    let snapshot2 = PageRenderSnapshot::from_page(&page2);

    assert!(
        snapshot1.dirty_rects(&snapshot2).is_empty(),
        "selection-only 요소(TableCell)는 base layer dirty 판단에서 제외돼야 함"
    );
}

#[test]
fn snapshot_reuses_line_in_scoped_node_when_layout_is_unchanged() {
    let block_id = NodeId::new();
    let scope_id = NodeId::new();
    let shared_layout = Rc::new(parley::Layout::default());

    let make_line_node = || {
        Rc::new(LayoutNode {
            size: Size::new(180.0, 20.0),
            element: Some(Element::Line(LineElement::build(
                block_id,
                Size::new(180.0, 20.0),
                0,
                Rc::clone(&shared_layout),
                LineMetric {
                    top: 0.0,
                    left: 0.0,
                    height: 20.0,
                    leading: 0.0,
                    baseline: 14.0,
                    ascent: 14.0,
                    content_width: 120.0,
                    start_offset: 0,
                    end_offset: 5,
                    clusters: vec![],
                    break_reason: parley::layout::BreakReason::None,
                    grapheme_offsets: vec![0, 5],
                    ascent_overflow: 0.0,
                    descent_overflow: 0.0,
                },
                None,
                false,
                Rc::from("hello"),
                vec![],
                vec![],
                false,
            ))),
            children: None,
            page_break_policy: PageBreakPolicy::default(),
            render_hints: RenderHints {
                default_text_color: Some("ui.text.default".to_string()),
            },
            scope_id: Some(scope_id),
        })
    };

    let page1 = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(20.0, 24.0),
            node: make_line_node(),
        }]),
        Size::new(300.0, 200.0),
    );
    let page2 = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(20.0, 24.0),
            node: make_line_node(),
        }]),
        Size::new(300.0, 200.0),
    );

    let snapshot1 = PageRenderSnapshot::from_page(&page1);
    let snapshot2 = PageRenderSnapshot::from_page(&page2);

    assert!(
        snapshot1.dirty_rects(&snapshot2).is_empty(),
        "scope/힌트 보정으로 라인 노드 Rc가 바뀌어도 동일 라인은 dirty로 잡히면 안 됨"
    );
}

#[test]
fn snapshot_expands_dirty_rect_upward_for_ruby_line() {
    let block_id = NodeId::new();
    let shared_layout = Rc::new(parley::Layout::default());
    let line_y = 24.0;

    let make_line_node = |ruby_segments: Vec<RubySegment>| {
        Rc::new(LayoutNode {
            size: Size::new(180.0, 20.0),
            element: Some(Element::Line(LineElement::build(
                block_id,
                Size::new(180.0, 20.0),
                0,
                Rc::clone(&shared_layout),
                LineMetric {
                    top: 0.0,
                    left: 0.0,
                    height: 20.0,
                    leading: 0.0,
                    baseline: 14.0,
                    ascent: 14.0,
                    content_width: 120.0,
                    start_offset: 0,
                    end_offset: 2,
                    clusters: vec![],
                    break_reason: parley::layout::BreakReason::None,
                    grapheme_offsets: vec![0, 2],
                    ascent_overflow: 0.0,
                    descent_overflow: 0.0,
                },
                None,
                false,
                Rc::from("ab"),
                ruby_segments,
                vec![],
                false,
            ))),
            children: None,
            page_break_policy: PageBreakPolicy::default(),
            render_hints: RenderHints::default(),
            scope_id: None,
        })
    };

    let page_without_ruby = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(20.0, line_y),
            node: make_line_node(vec![]),
        }]),
        Size::new(300.0, 200.0),
    );
    let page_with_ruby = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(20.0, line_y),
            node: make_line_node(vec![RubySegment {
                start_offset: 0,
                end_offset: 1,
                ruby_text: "루".to_string(),
            }]),
        }]),
        Size::new(300.0, 200.0),
    );

    let snapshot1 = PageRenderSnapshot::from_page(&page_without_ruby);
    let snapshot2 = PageRenderSnapshot::from_page(&page_with_ruby);
    let rects = snapshot1.dirty_rects(&snapshot2);

    assert!(
        !rects.is_empty(),
        "루비 추가 시 라인 snapshot dirty rect가 비어 있으면 안 됨"
    );
    assert!(
        rects.iter().any(|rect| rect.y < line_y),
        "루비 상단 영역이 partial repaint 대상에 포함돼야 함"
    );
}

#[test]
fn snapshot_distinguishes_ruby_and_background_segments() {
    let block_id = NodeId::new();
    let shared_layout = Rc::new(parley::Layout::default());

    let make_line_node = |ruby_segments: Vec<RubySegment>,
                          background_segments: Vec<BackgroundSegment>| {
        Rc::new(LayoutNode {
            size: Size::new(180.0, 20.0),
            element: Some(Element::Line(LineElement::build(
                block_id,
                Size::new(180.0, 20.0),
                0,
                Rc::clone(&shared_layout),
                LineMetric {
                    top: 0.0,
                    left: 0.0,
                    height: 20.0,
                    leading: 0.0,
                    baseline: 14.0,
                    ascent: 14.0,
                    content_width: 120.0,
                    start_offset: 0,
                    end_offset: 5,
                    clusters: vec![],
                    break_reason: parley::layout::BreakReason::None,
                    grapheme_offsets: vec![0, 5],
                    ascent_overflow: 0.0,
                    descent_overflow: 0.0,
                },
                None,
                false,
                Rc::from("abcde"),
                ruby_segments,
                background_segments,
                false,
            ))),
            children: None,
            page_break_policy: PageBreakPolicy::default(),
            render_hints: RenderHints::default(),
            scope_id: None,
        })
    };

    let page_with_ruby = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(20.0, 24.0),
            node: make_line_node(
                vec![RubySegment {
                    start_offset: 0,
                    end_offset: 5,
                    ruby_text: "abc".to_string(),
                }],
                vec![],
            ),
        }]),
        Size::new(300.0, 200.0),
    );
    let page_with_background = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(20.0, 24.0),
            node: make_line_node(
                vec![],
                vec![BackgroundSegment {
                    start_offset: 0,
                    end_offset: 5,
                    color_key: "abc".to_string(),
                }],
            ),
        }]),
        Size::new(300.0, 200.0),
    );

    let snapshot_ruby = PageRenderSnapshot::from_page(&page_with_ruby);
    let snapshot_background = PageRenderSnapshot::from_page(&page_with_background);

    assert!(
        !snapshot_ruby.dirty_rects(&snapshot_background).is_empty(),
        "ruby/background segment 구성 차이는 signature에 반영되어 dirty rect가 발생해야 함"
    );
}

#[test]
fn dense_line_shift_rects_are_coalesced_before_full_promotion() {
    let mut rects = Vec::new();
    for i in 0..40 {
        rects.push(
            CacheRect::from_xywh(20.0, 32.0 + i as f32 * 14.0, 260.0, 12.0).expect("valid rect"),
        );
    }

    let normalized = normalize_dirty_rects(rects, 300.0, 900.0);
    assert!(
        normalized.len() < FULL_REPAINT_RECT_THRESHOLD,
        "dense line shift dirty rects should be coalesced before full repaint threshold"
    );
    assert!(
        !should_promote_full_repaint(&normalized, 300.0, 900.0),
        "line shift region that covers about half the page should stay partial repaint"
    );
}

#[test]
fn height_only_resize_reuses_snapshot_and_repaints_exposed_strip() {
    let callout_id = NodeId::new();
    let make_page = |height: f32| {
        root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(20.0, 24.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(260.0, 72.0),
                    element: Some(Element::CalloutBackground(CalloutBackgroundElement::new(
                        Size::new(260.0, 72.0),
                        CalloutVariant::Info,
                        callout_id,
                        SplitEdges::default(),
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: None,
                }),
            }]),
            Size::new(300.0, height),
        )
    };

    let page1 = make_page(200.0);
    let page2 = make_page(260.0);

    let doc = Doc::new();
    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_render_debug(true);

    renderer.set_size(300.0, 200.0, 1.0);
    let _ = renderer
        .prepare_base_layer(&page1, 0, &doc)
        .expect("debug frame should exist");

    renderer.set_size(300.0, 260.0, 1.0);
    let frame = renderer
        .prepare_base_layer(&page2, 0, &doc)
        .expect("debug frame should exist");

    assert!(
        !frame.full_repaint,
        "height-only resize should not force full repaint when snapshot can be reused"
    );
    assert!(
        !frame.render_rects.is_empty(),
        "newly exposed strip should be marked dirty and repainted"
    );
    assert!(
        frame.render_rects.iter().any(|rect| rect.y >= 199.0),
        "dirty rect should include the exposed bottom strip after height growth"
    );
}

#[test]
fn snapshot_marks_table_border_dirty_when_columns_change_without_bounds_change() {
    let table_id = NodeId::new();

    let make_page = |cols: usize, col_widths: Vec<f32>| {
        root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(20.0, 24.0),
                node: Rc::new(LayoutNode {
                    size: Size::new(300.0, 120.0),
                    element: Some(Element::TableBorder(TableBorderElement::new(
                        Size::new(300.0, 120.0),
                        table_id,
                        TableBorderStyle::Solid,
                        TableAlign::Left,
                        2,
                        cols,
                        vec![59.0, 59.0],
                        col_widths,
                        SplitEdges::default(),
                        0.0,
                        0.0,
                        0,
                        2,
                    ))),
                    children: None,
                    page_break_policy: PageBreakPolicy::default(),
                    render_hints: RenderHints::default(),
                    scope_id: None,
                }),
            }]),
            Size::new(360.0, 200.0),
        )
    };

    let page1 = make_page(3, vec![98.0, 98.0, 98.0]);
    let page2 = make_page(2, vec![148.0, 148.0]);

    let snapshot1 = PageRenderSnapshot::from_page(&page1);
    let snapshot2 = PageRenderSnapshot::from_page(&page2);

    assert!(
        !snapshot1.dirty_rects(&snapshot2).is_empty(),
        "테이블 열/폭이 바뀌면 bounds가 같아도 dirty로 잡혀야 함"
    );
}
