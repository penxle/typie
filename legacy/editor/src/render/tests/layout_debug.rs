use super::*;

#[test]
fn layout_debug_rects_follow_recomputed_node_ids() {
    let fold_id = NodeId::new();

    let page = root_with_children(
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

    let none = FxHashSet::default();
    assert!(
        collect_layout_dirty_rects(&page, &none).is_empty(),
        "recompute된 node id가 없으면 layout debug rect도 없어야 함"
    );

    let mut recomputed = FxHashSet::default();
    recomputed.insert(fold_id);
    let rects = collect_layout_dirty_rects(&page, &recomputed);
    assert!(
        !rects.is_empty(),
        "recompute된 node id가 있으면 layout debug rect가 표시돼야 함"
    );
}

#[test]
fn layout_debug_rects_coalesce_nested_recomputed_nodes() {
    let parent_id = NodeId::new();
    let child_id = NodeId::new();

    let child = Rc::new(LayoutNode {
        size: Size::new(60.0, 24.0),
        element: Some(Element::FoldContent(FoldContentElement::new(
            Size::new(60.0, 24.0),
            SplitEdges::default(),
            child_id,
        ))),
        children: None,
        page_break_policy: PageBreakPolicy::default(),
        render_hints: RenderHints::default(),
        scope_id: None,
    });

    let parent = Rc::new(LayoutNode {
        size: Size::new(180.0, 80.0),
        element: Some(Element::FoldContent(FoldContentElement::new(
            Size::new(180.0, 80.0),
            SplitEdges::default(),
            parent_id,
        ))),
        children: Some(vec![PositionedNode {
            position: Point::new(12.0, 16.0),
            node: Rc::clone(&child),
        }]),
        page_break_policy: PageBreakPolicy::default(),
        render_hints: RenderHints::default(),
        scope_id: None,
    });

    let page = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(8.0, 12.0),
            node: parent,
        }]),
        Size::new(300.0, 200.0),
    );

    let mut recomputed = FxHashSet::default();
    recomputed.insert(parent_id);
    recomputed.insert(child_id);

    let rects = collect_layout_dirty_rects(&page, &recomputed);
    assert_eq!(
        rects.len(),
        1,
        "중첩된 recompute는 상위 노드 rect 하나로 축약되어야 함"
    );
    assert!(
        rects[0]
            .approx_eq(LayoutRect::from_xywh(8.0, 12.0, 180.0, 80.0).expect("valid parent rect")),
        "상위 노드가 dirty rect로 선택되어야 함"
    );

    let mut child_only = FxHashSet::default();
    child_only.insert(child_id);
    let child_rects = collect_layout_dirty_rects(&page, &child_only);
    assert_eq!(
        child_rects.len(),
        1,
        "자식만 recompute되면 자식 rect를 유지해야 함"
    );
    assert!(
        child_rects[0]
            .approx_eq(LayoutRect::from_xywh(20.0, 28.0, 60.0, 24.0).expect("valid child rect")),
        "자식 단독 recompute는 자식 위치를 정확히 표시해야 함"
    );
}

#[test]
fn layout_debug_rects_track_table_cell_node() {
    let cell_id = NodeId::new();

    let page = root_with_children(
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

    let mut recomputed = FxHashSet::default();
    recomputed.insert(cell_id);
    let rects = collect_layout_dirty_rects(&page, &recomputed);
    assert!(
        !rects.is_empty(),
        "table cell node가 recompute되면 layout debug rect가 표시돼야 함"
    );

    recomputed.clear();
    recomputed.insert(NodeId::new());
    assert!(
        collect_layout_dirty_rects(&page, &recomputed).is_empty(),
        "다른 node id만 recompute되면 table cell rect는 표시되면 안 됨"
    );
}

#[test]
fn layout_debug_reuses_same_revision() {
    let fold_id = NodeId::new();
    let page = root_with_children(
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
    let doc = Doc::default();
    let diagnostics = FrameDiagnostics::new();
    let mut renderer = Renderer::new(1.0, diagnostics.clone());
    renderer.set_layout_debug(true);
    renderer.set_size(300.0, 200.0, 1.0);

    let mut pass = LayoutPassRecorder::new();
    pass.record_recomputed(fold_id);
    diagnostics.commit_layout_pass(pass);

    let frame1 = renderer
        .prepare_base_layer(&page, 0, &doc)
        .expect("debug frame should exist when layout debug is enabled");
    assert!(
        !frame1.layout_rects.is_empty(),
        "첫 revision에서는 layout rect가 표시되어야 함"
    );

    let frame2 = renderer
        .prepare_base_layer(&page, 0, &doc)
        .expect("debug frame should exist when layout debug is enabled");
    assert!(
        frame2.layout_rects.is_empty(),
        "같은 revision에서는 layout rect를 반복 표시하면 안 됨"
    );
    assert!(frame2.layout_reused, "같은 revision은 reused로 표시돼야 함");
}
