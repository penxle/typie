use super::*;
use crate::types::Affinity;

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
fn blockquote_block_selection_highlights_decoration() {
    let blockquote_id = NodeId::new();
    let page = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(40.0, 20.0),
            node: Rc::new(LayoutNode {
                size: Size::new(80.0, 40.0),
                element: Some(Element::Blockquote(BlockquoteLineElement::new(
                    Size::new(80.0, 40.0),
                    blockquote_id,
                    4.0,
                ))),
                children: None,
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: Some(blockquote_id),
            }),
        }]),
        Size::new(160.0, 100.0),
    );

    let doc = Doc::new();
    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_size(160.0, 100.0, 1.0);

    let mut colors = FxHashMap::default();
    colors.insert("ui.border.default".to_string(), 0xff_ff_ff_ff);
    colors.insert("selection".to_string(), 0xff_00_00_ff);
    renderer.set_theme(Theme { colors });

    let width = renderer.width() as usize;
    let height = renderer.height() as usize;
    let mut plain = vec![0u8; width * height * 4];
    let mut selected = plain.clone();

    assert!(renderer.render_to(&page, 0, None, &[], None, &doc, &mut plain));
    assert!(renderer.render_to(
        &page,
        0,
        None,
        &[SelectionDecor::Block {
            node_id: blockquote_id
        }],
        None,
        &doc,
        &mut selected
    ));

    let plain_pixel = rgba_at(&plain, width, 90, 30);
    let selected_pixel = rgba_at(&selected, width, 90, 30);
    assert_ne!(
        plain_pixel, selected_pixel,
        "fully selected blockquote는 내용 영역까지 선택 강조가 보여야 함"
    );
}

#[test]
fn blockquote_delete_like_selection_highlights_only_left_line_area() {
    let mut bq1 = id!();
    let mut bq2 = id!();
    let mut n1 = id!();
    let mut n2 = id!();

    let state = state! {
        doc {
            @bq1 blockquote {
                @n1 paragraph {
                    text { "asdf" }
                }
            }
            @bq2 blockquote {
                @n2 paragraph {
                    text { "asdf" }
                }
            }
            paragraph {}
        }
        selection { (n1, 2) -> (n2, 2) }
    };

    let selections = crate::state::build_selection_decorations(&state.doc, &state.selection, None);
    assert!(
        selections
            .iter()
            .any(|s| matches!(s, SelectionDecor::Block { node_id } if *node_id == bq2))
    );
    assert!(
        selections
            .iter()
            .any(|s| matches!(s, SelectionDecor::TextRange { node_id, .. } if *node_id == n2))
    );

    let page = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(40.0, 20.0),
            node: Rc::new(LayoutNode {
                size: Size::new(80.0, 24.0),
                element: Some(Element::Blockquote(BlockquoteLineElement::new(
                    Size::new(80.0, 24.0),
                    bq2,
                    4.0,
                ))),
                children: None,
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: Some(bq2),
            }),
        }]),
        Size::new(180.0, 100.0),
    );

    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_size(180.0, 100.0, 1.0);

    let mut colors = FxHashMap::default();
    colors.insert("ui.border.default".to_string(), 0xff_ff_ff_ff);
    colors.insert("selection".to_string(), 0xff_00_00_ff);
    renderer.set_theme(Theme { colors });

    let width = renderer.width() as usize;
    let mut plain = vec![0u8; width * renderer.height() as usize * 4];
    let mut selected = plain.clone();

    assert!(renderer.render_to(&page, 0, None, &[], None, &state.doc, &mut plain));
    assert!(renderer.render_to(&page, 0, None, &selections, None, &state.doc, &mut selected));

    let plain_decoration = rgba_at(&plain, width, 50, 30);
    let selected_decoration = rgba_at(&selected, width, 50, 30);
    assert_ne!(
        plain_decoration, selected_decoration,
        "삭제 대상 blockquote는 line 영역에서는 선택 강조가 보여야 함"
    );

    let plain_content = rgba_at(&plain, width, 90, 30);
    let selected_content = rgba_at(&selected, width, 90, 30);
    assert_eq!(
        plain_content, selected_content,
        "삭제 대상 blockquote는 내용 영역까지 block 선택 강조가 퍼지면 안 됨"
    );
}

#[test]
fn blockquote_quote_delete_like_selection_highlights_only_quote_area() {
    let mut bq1 = id!();
    let mut bq2 = id!();
    let mut n1 = id!();
    let mut n2 = id!();

    let state = state! {
        doc {
            @bq1 blockquote(variant: BlockquoteVariant::LeftQuote,) {
                @n1 paragraph {
                    text { "asdf" }
                }
            }
            @bq2 blockquote(variant: BlockquoteVariant::LeftQuote,) {
                @n2 paragraph {
                    text { "asdf" }
                }
            }
            paragraph {}
        }
        selection { (n1, 2) -> (n2, 2) }
    };

    let selections = crate::state::build_selection_decorations(&state.doc, &state.selection, None);
    assert!(
        selections
            .iter()
            .any(|s| matches!(s, SelectionDecor::Block { node_id } if *node_id == bq2))
    );
    assert!(
        selections
            .iter()
            .any(|s| matches!(s, SelectionDecor::TextRange { node_id, .. } if *node_id == n2))
    );

    let page = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(40.0, 20.0),
            node: Rc::new(LayoutNode {
                size: Size::new(96.0, 24.0),
                element: Some(Element::BlockquoteQuote(BlockquoteQuoteElement::new(
                    Size::new(96.0, 24.0),
                    bq2,
                ))),
                children: None,
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: Some(bq2),
            }),
        }]),
        Size::new(200.0, 100.0),
    );

    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_size(200.0, 100.0, 1.0);

    let mut colors = FxHashMap::default();
    colors.insert("ui.text.muted".to_string(), 0xff_ff_ff_ff);
    colors.insert("selection".to_string(), 0xff_00_00_ff);
    renderer.set_theme(Theme { colors });

    let width = renderer.width() as usize;
    let mut plain = vec![0u8; width * renderer.height() as usize * 4];
    let mut selected = plain.clone();

    assert!(renderer.render_to(&page, 0, None, &[], None, &state.doc, &mut plain));
    assert!(renderer.render_to(&page, 0, None, &selections, None, &state.doc, &mut selected));

    let plain_quote = rgba_at(&plain, width, 62, 30);
    let selected_quote = rgba_at(&selected, width, 62, 30);
    assert_ne!(
        plain_quote, selected_quote,
        "삭제 대상 quote blockquote는 quote 영역에서 선택 강조가 보여야 함"
    );

    let plain_content = rgba_at(&plain, width, 94, 30);
    let selected_content = rgba_at(&selected, width, 94, 30);
    assert_eq!(
        plain_content, selected_content,
        "삭제 대상 quote blockquote는 내용 영역까지 block 선택 강조가 퍼지면 안 됨"
    );
}

#[test]
fn callout_block_selection_highlights_background() {
    let callout_id = NodeId::new();
    let page = callout_page_with_icon(callout_id);

    let doc = Doc::new();
    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_size(220.0, 160.0, 1.0);

    let mut colors = FxHashMap::default();
    colors.insert("ui.callout.info".to_string(), 0x33_66_99_ff);
    colors.insert("selection".to_string(), 0xff_00_00_ff);
    renderer.set_theme(Theme { colors });

    let width = renderer.width() as usize;
    let height = renderer.height() as usize;
    let mut plain = vec![0u8; width * height * 4];
    let mut selected = plain.clone();

    assert!(renderer.render_to(&page, 0, None, &[], None, &doc, &mut plain));
    assert!(renderer.render_to(
        &page,
        0,
        None,
        &[SelectionDecor::Block {
            node_id: callout_id
        }],
        None,
        &doc,
        &mut selected
    ));

    let plain_pixel = rgba_at(&plain, width, 90, 60);
    let selected_pixel = rgba_at(&selected, width, 90, 60);
    assert_ne!(
        plain_pixel, selected_pixel,
        "callout block selection 시 배경 선택 강조가 보여야 함"
    );
}

#[test]
fn list_item_block_selection_highlights_marker_without_text_selection() {
    let list_item_id = NodeId::new();
    let line_id = NodeId::new();
    let page = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(20.0, 20.0),
            node: Rc::new(LayoutNode {
                size: Size::new(100.0, 40.0),
                element: None,
                children: Some(vec![
                    PositionedNode {
                        position: Point::new(0.0, 0.0),
                        node: marker_node_for(list_item_id, Size::new(28.0, 16.0)),
                    },
                    PositionedNode {
                        position: Point::new(28.0, 0.0),
                        node: line_node(line_id, "item", Size::new(72.0, 16.0)),
                    },
                ]),
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: None,
            }),
        }]),
        Size::new(120.0, 80.0),
    );

    let doc = Doc::new();
    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_size(120.0, 80.0, 1.0);

    let mut colors = FxHashMap::default();
    colors.insert("ui.text.default".to_string(), 0xff_ff_ff_ff);
    colors.insert("selection".to_string(), 0xff_00_00_ff);
    renderer.set_theme(Theme { colors });

    let width = renderer.width() as usize;
    let height = renderer.height() as usize;
    let mut plain = vec![0u8; width * height * 4];
    let mut selected = plain.clone();

    assert!(renderer.render_to(&page, 0, None, &[], None, &doc, &mut plain));
    assert!(renderer.render_to(
        &page,
        0,
        None,
        &[SelectionDecor::Block {
            node_id: list_item_id
        }],
        None,
        &doc,
        &mut selected
    ));

    let plain_bg_pixel = rgba_at(&plain, width, 28, 30);
    let selected_bg_pixel = rgba_at(&selected, width, 28, 30);
    assert_ne!(
        plain_bg_pixel, selected_bg_pixel,
        "list item block decoration이면 marker 영역 선택 강조가 보여야 함"
    );

    let plain_marker_pixel = rgba_at(&plain, width, 32, 24);
    let selected_marker_pixel = rgba_at(&selected, width, 32, 24);
    assert!(
        plain_marker_pixel == selected_marker_pixel,
        "marker 자체 픽셀은 별도 선택 강조가 없어야 함"
    );
}

#[test]
fn list_item_text_selection_without_front_boundary_does_not_highlight_marker() {
    let mut list_item_id = id!();
    let mut p = id!();
    let state = state! {
        doc {
            bullet_list {
                @list_item_id list_item {
                    @p paragraph {
                        text { "item" }
                    }
                }
            }
        }
        selection { (p, 1) -> (p, 3) }
    };

    let selections = crate::state::build_selection_decorations(&state.doc, &state.selection, None);
    assert!(
        selections.iter().any(|s| {
            matches!(
                s,
                SelectionDecor::TextRange {
                    node_id,
                    start_offset,
                    end_offset
                } if *node_id == p && *start_offset == 1 && *end_offset == 3
            )
        }),
        "partial text selection should remain on list paragraph"
    );

    let page = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(20.0, 20.0),
            node: Rc::new(LayoutNode {
                size: Size::new(100.0, 40.0),
                element: None,
                children: Some(vec![
                    PositionedNode {
                        position: Point::new(0.0, 0.0),
                        node: marker_node_for(list_item_id, Size::new(28.0, 16.0)),
                    },
                    PositionedNode {
                        position: Point::new(28.0, 0.0),
                        node: line_node(p, "item", Size::new(72.0, 16.0)),
                    },
                ]),
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: None,
            }),
        }]),
        Size::new(120.0, 80.0),
    );

    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_size(120.0, 80.0, 1.0);
    let mut colors = FxHashMap::default();
    colors.insert("ui.text.default".to_string(), 0xff_ff_ff_ff);
    colors.insert("selection".to_string(), 0xff_00_00_ff);
    renderer.set_theme(Theme { colors });

    let width = renderer.width() as usize;
    let mut plain = vec![0u8; width * renderer.height() as usize * 4];
    let mut selected = plain.clone();

    assert!(renderer.render_to(&page, 0, None, &[], None, &state.doc, &mut plain));
    assert!(renderer.render_to(&page, 0, None, &selections, None, &state.doc, &mut selected));

    let plain_marker_pixel = rgba_at(&plain, width, 24, 30);
    let selected_marker_pixel = rgba_at(&selected, width, 24, 30);
    assert_eq!(
        plain_marker_pixel, selected_marker_pixel,
        "list item front boundary가 포함되지 않으면 marker 선택 강조가 보이면 안 됨"
    );
}

#[test]
fn list_item_partial_block_selection_highlights_marker_not_container() {
    let mut p1 = id!();
    let mut list_item_id = id!();
    let mut p2 = id!();

    let state = state! {
        doc {
            bullet_list {
                list_item {
                    @p1 paragraph {
                        text { "prev" }
                    }
                }
                @list_item_id list_item {
                    @p2 paragraph {
                        text { "item" }
                    }
                }
            }
            paragraph {}
        }
        selection { (p1, 2) -> (p2, 2) }
    };

    let page = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(20.0, 20.0),
            node: Rc::new(LayoutNode {
                size: Size::new(100.0, 40.0),
                element: None,
                children: Some(vec![
                    PositionedNode {
                        position: Point::new(0.0, 0.0),
                        node: marker_node_for(list_item_id, Size::new(28.0, 16.0)),
                    },
                    PositionedNode {
                        position: Point::new(28.0, 0.0),
                        node: line_node(p2, "item", Size::new(72.0, 16.0)),
                    },
                ]),
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: None,
            }),
        }]),
        Size::new(120.0, 80.0),
    );

    let selections = crate::state::build_selection_decorations(&state.doc, &state.selection, None);
    assert!(
        selections
            .iter()
            .any(|s| matches!(s, SelectionDecor::Block { node_id } if *node_id == list_item_id)),
        "list item front boundary가 포함되면 block selection이 있어야 함"
    );
    assert!(
        selections
            .iter()
            .any(|s| matches!(s, SelectionDecor::TextRange { node_id, .. } if *node_id == p2)),
        "list item descendant text selection should exist"
    );

    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_size(120.0, 80.0, 1.0);
    let mut colors = FxHashMap::default();
    colors.insert("ui.text.default".to_string(), 0xff_ff_ff_ff);
    colors.insert("selection".to_string(), 0xff_00_00_ff);
    renderer.set_theme(Theme { colors });

    let width = renderer.width() as usize;
    let mut plain = vec![0u8; width * renderer.height() as usize * 4];
    let mut selected = plain.clone();

    assert!(renderer.render_to(&page, 0, None, &[], None, &state.doc, &mut plain));
    assert!(renderer.render_to(&page, 0, None, &selections, None, &state.doc, &mut selected));

    let plain_bg_pixel = rgba_at(&plain, width, 60, 30);
    let selected_bg_pixel = rgba_at(&selected, width, 60, 30);
    assert_eq!(
        plain_bg_pixel, selected_bg_pixel,
        "partial list-item block selection에서는 container 배경 선택 강조가 없어야 함"
    );

    let plain_marker_pixel = rgba_at(&plain, width, 24, 30);
    let selected_marker_pixel = rgba_at(&selected, width, 24, 30);
    assert_ne!(
        plain_marker_pixel, selected_marker_pixel,
        "partial list-item block selection에서는 marker 선택 강조가 보여야 함"
    );

    let plain_gap_pixel = rgba_at(&plain, width, 44, 30);
    let selected_gap_pixel = rgba_at(&selected, width, 44, 30);
    assert_ne!(
        plain_gap_pixel, selected_gap_pixel,
        "marker 선택 강조와 paragraph 선택 강조 사이 gap 영역도 선택 강조되어야 함"
    );

    let plain_lower_marker_pixel = rgba_at(&plain, width, 24, 55);
    let selected_lower_marker_pixel = rgba_at(&selected, width, 24, 55);
    assert_eq!(
        plain_lower_marker_pixel, selected_lower_marker_pixel,
        "marker 선택 강조는 첫 줄 marker 영역에만 그려지고 아래로 길어지면 안 됨"
    );
}

#[test]
fn list_item_reverse_start_boundary_delete_like_highlights_marker_not_container() {
    let mut n1 = id!();
    let mut list_item_id = id!();
    let mut n2 = id!();

    let state = state! {
        doc {
            @n1 paragraph {}
            bullet_list {
                @list_item_id list_item {
                    @n2 paragraph {
                        text { "a" }
                    }
                    bullet_list {
                        list_item {
                            paragraph {
                                text { "b" }
                            }
                        }
                    }
                }
            }
            paragraph {}
        }
        selection { (n2, 0) -> (n1, 0) }
    };

    let page = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(20.0, 20.0),
            node: Rc::new(LayoutNode {
                size: Size::new(100.0, 40.0),
                element: None,
                children: Some(vec![
                    PositionedNode {
                        position: Point::new(0.0, 0.0),
                        node: marker_node_for(list_item_id, Size::new(28.0, 16.0)),
                    },
                    PositionedNode {
                        position: Point::new(28.0, 0.0),
                        node: line_node(n2, "a", Size::new(72.0, 16.0)),
                    },
                ]),
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: None,
            }),
        }]),
        Size::new(120.0, 80.0),
    );

    let selections = crate::state::build_selection_decorations(&state.doc, &state.selection, None);
    assert!(
        selections
            .iter()
            .any(|s| matches!(s, SelectionDecor::Block { node_id } if *node_id == list_item_id)),
        "reverse start-boundary delete-like selection에서 list item front boundary가 포함되면 block selection이 있어야 함"
    );
    assert!(
        selections
            .iter()
            .all(|s| !matches!(s, SelectionDecor::TextRange { node_id, .. } if *node_id == n2)),
        "reverse start-boundary selection에서는 descendant text anchor가 없어야 함"
    );

    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_size(120.0, 80.0, 1.0);
    let mut colors = FxHashMap::default();
    colors.insert("ui.text.default".to_string(), 0xff_ff_ff_ff);
    colors.insert("selection".to_string(), 0xff_00_00_ff);
    renderer.set_theme(Theme { colors });

    let width = renderer.width() as usize;
    let mut plain = vec![0u8; width * renderer.height() as usize * 4];
    let mut selected = plain.clone();

    assert!(renderer.render_to(&page, 0, None, &[], None, &state.doc, &mut plain));
    assert!(renderer.render_to(&page, 0, None, &selections, None, &state.doc, &mut selected));

    let plain_bg_pixel = rgba_at(&plain, width, 60, 30);
    let selected_bg_pixel = rgba_at(&selected, width, 60, 30);
    assert_eq!(
        plain_bg_pixel, selected_bg_pixel,
        "reverse start-boundary delete-like list item selection에서는 container 배경 선택 강조가 없어야 함"
    );

    let plain_marker_pixel = rgba_at(&plain, width, 24, 30);
    let selected_marker_pixel = rgba_at(&selected, width, 24, 30);
    assert_ne!(
        plain_marker_pixel, selected_marker_pixel,
        "reverse start-boundary delete-like list item selection에서는 marker 선택 강조가 보여야 함"
    );
}

#[test]
fn list_item_reverse_upstream_end_delete_like_highlights_marker_not_container() {
    let mut n1 = id!();
    let mut list_item_id = id!();
    let mut n2 = id!();

    let state = state! {
        doc {
            @n1 paragraph {}
            bullet_list {
                @list_item_id list_item {
                    @n2 paragraph {
                        text { "1" }
                    }
                    bullet_list {
                        list_item {
                            paragraph {
                                text { "2" }
                            }
                        }
                    }
                }
            }
            paragraph {}
        }
        selection { (n2, 1, Affinity::Upstream) -> (n1, 0) }
    };

    let page = root_with_children(
        Some(vec![PositionedNode {
            position: Point::new(20.0, 20.0),
            node: Rc::new(LayoutNode {
                size: Size::new(100.0, 40.0),
                element: None,
                children: Some(vec![
                    PositionedNode {
                        position: Point::new(0.0, 0.0),
                        node: marker_node_for(list_item_id, Size::new(28.0, 16.0)),
                    },
                    PositionedNode {
                        position: Point::new(28.0, 0.0),
                        node: line_node(n2, "1", Size::new(72.0, 16.0)),
                    },
                ]),
                page_break_policy: PageBreakPolicy::default(),
                render_hints: RenderHints::default(),
                scope_id: None,
            }),
        }]),
        Size::new(120.0, 80.0),
    );

    let selections = crate::state::build_selection_decorations(&state.doc, &state.selection, None);
    assert!(
        selections
            .iter()
            .any(|s| matches!(s, SelectionDecor::Block { node_id } if *node_id == list_item_id)),
        "reverse upstream-end delete-like selection에서 list item front boundary가 포함되면 block selection이 있어야 함"
    );
    assert!(
        selections.iter().any(|s| {
            matches!(
                s,
                SelectionDecor::TextRange {
                    node_id,
                    start_offset,
                    end_offset
                } if *node_id == n2 && *start_offset == 0 && *end_offset == 1
            )
        }),
        "reverse upstream-end delete-like selection should keep first paragraph text range"
    );

    let mut renderer = Renderer::new(1.0, FrameDiagnostics::new());
    renderer.set_size(120.0, 80.0, 1.0);
    let mut colors = FxHashMap::default();
    colors.insert("ui.text.default".to_string(), 0xff_ff_ff_ff);
    colors.insert("selection".to_string(), 0xff_00_00_ff);
    renderer.set_theme(Theme { colors });

    let width = renderer.width() as usize;
    let mut plain = vec![0u8; width * renderer.height() as usize * 4];
    let mut selected = plain.clone();

    assert!(renderer.render_to(&page, 0, None, &[], None, &state.doc, &mut plain));
    assert!(renderer.render_to(&page, 0, None, &selections, None, &state.doc, &mut selected));

    let plain_bg_pixel = rgba_at(&plain, width, 60, 30);
    let selected_bg_pixel = rgba_at(&selected, width, 60, 30);
    assert_eq!(
        plain_bg_pixel, selected_bg_pixel,
        "reverse upstream-end delete-like list item selection에서는 container 배경 선택 강조가 없어야 함"
    );

    let plain_marker_pixel = rgba_at(&plain, width, 24, 30);
    let selected_marker_pixel = rgba_at(&selected, width, 24, 30);
    assert_ne!(
        plain_marker_pixel, selected_marker_pixel,
        "reverse upstream-end delete-like list item selection에서는 marker 선택 강조가 보여야 함"
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
                        80.0,
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
        &[SelectionDecor::Block { node_id: cell_id }],
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
                        80.0,
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
    let selections = [SelectionDecor::Block { node_id: cell_id }];

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
        "pixel snapping으로 rect가 겹쳐도 선택 강조는 한 번만 적용돼야 함"
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
        &[SelectionDecor::Block { node_id: cell_id }],
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
        &[SelectionDecor::Block { node_id: cell_id }],
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
                        80.0,
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
        &[SelectionDecor::Block { node_id: cell_id }],
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
