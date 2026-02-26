use super::*;

#[test]
fn clip_intersection_uses_ruby_overhang_bounds() {
    let block_id = NodeId::new();
    let shared_layout = Rc::new(parley::Layout::default());
    let line_y = 24.0;

    let make_positioned_line = |ruby_segments: Vec<RubySegment>| PositionedNode {
        position: Point::new(20.0, line_y),
        node: Rc::new(LayoutNode {
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
        }),
    };

    let clip = CacheRect::from_xywh(0.0, line_y - 8.0, 300.0, 6.0).expect("valid clip rect");
    let with_ruby = make_positioned_line(vec![RubySegment {
        start_offset: 0,
        end_offset: 1,
        ruby_text: "루".to_string(),
    }]);
    let without_ruby = make_positioned_line(vec![]);

    let with_ruby_bounds =
        node_paint_bounds(&with_ruby, with_ruby.position).expect("line bounds should exist");
    let without_ruby_bounds =
        node_paint_bounds(&without_ruby, without_ruby.position).expect("line bounds should exist");

    assert!(
        with_ruby_bounds.intersects(clip),
        "루비 상단 overhang 영역만 clip 돼도 라인 렌더가 스킵되면 안 됨"
    );
    assert!(
        !without_ruby_bounds.intersects(clip),
        "루비가 없는 라인은 overhang clip과 교차하지 않아야 함"
    );
}

#[test]
fn next_page_ruby_clip_intersects_boundary_root() {
    let page_width = 300.0;
    let page_height = 200.0;
    let boundary_root =
        CacheRect::from_xywh(0.0, page_height, page_width, page_height).expect("valid root rect");

    let narrow_clip = CacheRect::from_xywh(
        0.0,
        page_height - PAGE_EDGE_OVERFLOW_BAND,
        page_width,
        PAGE_EDGE_OVERFLOW_BAND,
    )
    .expect("valid narrow clip");
    assert!(
        !boundary_root.intersects(narrow_clip),
        "경계에 딱 붙은 다음 페이지 루트는 좁은 clip에서는 탈락한다"
    );

    let expanded_clip =
        next_page_overflow_cull_clip(page_width, page_height).expect("expanded clip");
    assert!(
        boundary_root.intersects(expanded_clip),
        "다음 페이지 루트(y == page_height)는 오버플로우 clip과 교차해야 한다"
    );
}

#[test]
fn overflow_debug_rects_include_visible_ruby_overhang() {
    let block_id = NodeId::new();
    let shared_layout = Rc::new(parley::Layout::default());
    let page_width = 300.0;
    let page_height = 200.0;

    let make_next_page = |ruby_segments: Vec<RubySegment>| {
        root_with_children(
            Some(vec![PositionedNode {
                position: Point::new(20.0, 0.0),
                node: Rc::new(LayoutNode {
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
                }),
            }]),
            Size::new(page_width, page_height),
        )
    };

    let with_ruby = make_next_page(vec![RubySegment {
        start_offset: 0,
        end_offset: 1,
        ruby_text: "루".to_string(),
    }]);
    let without_ruby = make_next_page(vec![]);
    let clip =
        next_page_overflow_cull_clip(page_width, page_height).expect("next page overflow clip");

    let with_rects =
        Renderer::collect_next_page_overflow_debug_rects(&with_ruby, page_width, page_height, clip);
    let without_rects = Renderer::collect_next_page_overflow_debug_rects(
        &without_ruby,
        page_width,
        page_height,
        clip,
    );

    assert!(
        !with_rects.is_empty(),
        "루비가 있으면 현재 페이지 상단 오버플로우 디버그 rect가 수집돼야 함"
    );
    assert!(
        with_rects
            .iter()
            .any(|rect| rect.y < page_height && rect.bottom() <= page_height),
        "수집된 rect는 현재 페이지의 가시 영역 내에 있어야 함"
    );
    assert!(
        without_rects.is_empty(),
        "루비가 없으면 현재 페이지에 보이는 next-page overflow rect가 없어야 함"
    );
}

#[test]
fn overflow_overlay_reuses_cached_tile_when_next_page_is_unchanged() {
    let block_id = NodeId::new();
    let shared_layout = Rc::new(parley::Layout::default());
    let page_width = 300.0;
    let page_height = 200.0;
    let current_page = root_with_children(None, Size::new(page_width, page_height));
    let next_page = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(20.0, 0.0),
            node: Rc::new(LayoutNode {
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
                    vec![RubySegment {
                        start_offset: 0,
                        end_offset: 1,
                        ruby_text: "루".to_string(),
                    }],
                    vec![],
                    false,
                ))),
                children: None,
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: None,
            }),
        }]),
        Size::new(page_width, page_height),
    );

    let doc = Doc::new();
    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_size(page_width, page_height, 1.0);

    let width = renderer.width() as usize;
    let height = renderer.height() as usize;
    let mut buffer = vec![0u8; width * height * 4];

    assert!(renderer.render_to(
        &current_page,
        0,
        Some(&next_page),
        &[],
        None,
        &doc,
        &mut buffer
    ));
    assert!(
        renderer.overflow_cache.contains_key(&0),
        "첫 렌더 후 overflow 캐시가 생성되어야 함"
    );

    let poison = [17, 200, 91, 255];
    {
        let entry = renderer
            .overflow_cache
            .get_mut(&0)
            .expect("overflow cache entry should exist");
        for pixel in entry.tile_pixmap.data_mut().chunks_exact_mut(4) {
            pixel.copy_from_slice(&poison);
        }
    }

    buffer.fill(0);
    assert!(renderer.render_to(
        &current_page,
        0,
        Some(&next_page),
        &[],
        None,
        &doc,
        &mut buffer
    ));

    let sample = rgba_at(&buffer, width, 10, height.saturating_sub(2));
    assert_eq!(
        sample, poison,
        "같은 next_page 프레임에서는 overflow 타일을 재래스터하지 않고 캐시를 재사용해야 함"
    );
}

#[test]
fn overflow_line_signature_changes_when_preedit_presence_changes() {
    let block_id = NodeId::new();
    let shared_layout = Rc::new(parley::Layout::default());

    let make_positioned_line = |preedit: Option<crate::model::PreeditDecor>| PositionedNode {
        position: Point::new(20.0, 0.0),
        node: Rc::new(LayoutNode {
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
                preedit,
                false,
                Rc::from("ab"),
                vec![RubySegment {
                    start_offset: 0,
                    end_offset: 1,
                    ruby_text: "루".to_string(),
                }],
                vec![],
                false,
            ))),
            children: None,
            page_break_policy: PageBreakPolicy::default(),
            render_hints: RenderHints::default(),
            scope_id: None,
        }),
    };

    let without_preedit = make_positioned_line(None);
    let with_preedit = make_positioned_line(Some(crate::model::PreeditDecor {
        node_id: block_id,
        offset: 1,
        text: "x".to_string(),
    }));

    let line_without = match without_preedit.node.element.as_ref() {
        Some(Element::Line(line)) => line,
        _ => panic!("expected line element"),
    };
    let line_with = match with_preedit.node.element.as_ref() {
        Some(Element::Line(line)) => line,
        _ => panic!("expected line element"),
    };

    let sig_without =
        Renderer::overflow_line_signature(&without_preedit, line_without, without_preedit.position);
    let sig_with =
        Renderer::overflow_line_signature(&with_preedit, line_with, with_preedit.position);

    assert_ne!(
        sig_without, sig_with,
        "preedit None/Some 전환은 overflow line signature를 바꿔 overflow 캐시 재사용을 막아야 함"
    );
}

#[test]
fn overflow_line_signature_changes_when_ruby_segment_count_changes() {
    let block_id = NodeId::new();
    let shared_layout = Rc::new(parley::Layout::default());

    let make_positioned_line = |ruby_segments: Vec<RubySegment>| PositionedNode {
        position: Point::new(20.0, 0.0),
        node: Rc::new(LayoutNode {
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
                vec![],
                false,
            ))),
            children: None,
            page_break_policy: PageBreakPolicy::default(),
            render_hints: RenderHints::default(),
            scope_id: None,
        }),
    };

    let one_segment = make_positioned_line(vec![RubySegment {
        start_offset: 0,
        end_offset: 5,
        ruby_text: "abc".to_string(),
    }]);
    let two_segments = make_positioned_line(vec![
        RubySegment {
            start_offset: 0,
            end_offset: 2,
            ruby_text: "a".to_string(),
        },
        RubySegment {
            start_offset: 2,
            end_offset: 5,
            ruby_text: "bc".to_string(),
        },
    ]);

    let line_one = match one_segment.node.element.as_ref() {
        Some(Element::Line(line)) => line,
        _ => panic!("expected line element"),
    };
    let line_two = match two_segments.node.element.as_ref() {
        Some(Element::Line(line)) => line,
        _ => panic!("expected line element"),
    };

    let sig_one = Renderer::overflow_line_signature(&one_segment, line_one, one_segment.position);
    let sig_two = Renderer::overflow_line_signature(&two_segments, line_two, two_segments.position);

    assert_ne!(
        sig_one, sig_two,
        "ruby segment 개수/경계 변경은 overflow line signature를 바꿔 cache 재사용을 막아야 함"
    );
}
