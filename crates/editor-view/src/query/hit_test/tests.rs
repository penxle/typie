use super::*;

use editor_common::Rect;

use crate::page::LayoutPage;
use crate::paginate::*;

use crate::glyph_run::{GlyphRun, GraphemeSpan};
use crate::style::*;
use editor_common::EdgeInsets;
use editor_macros::doc;
use editor_model::NodeId;
use editor_state::{Affinity, Position};

fn make_line_node(id: NodeId, x: f32, y: f32, text: &str, char_w: f32) -> LayoutNode {
    let n = text.chars().count();
    LayoutNode {
        rect: Rect::from_xywh(x, y, n as f32 * char_w, 20.0),
        content: LayoutContent::Line(LayoutLine {
            node_id: id,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![GlyphRun::make_test_run(
                id,
                0,
                text,
                0.0,
                vec![
                    GraphemeSpan {
                        advance: char_w,
                        codepoints: 1
                    };
                    n
                ],
            )],
            ruby_annotations: vec![],
            empty_caret_x: 0.0,
            child_range: None,
            tab_gaps: vec![],
        }),
    }
}

fn make_box_node(
    id: NodeId,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    children: Vec<LayoutNode>,
) -> LayoutNode {
    LayoutNode {
        rect: Rect::from_xywh(x, y, w, h),
        content: LayoutContent::Box(LayoutBox {
            node_id: id,
            style: BoxStyle {
                direction: Direction::Vertical,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::ZERO,
                border_mode: BorderMode::Separate,
                alignment: Alignment::Start,
                scope: false,
                decorations: vec![],
                monolithic: false,
            },
            children,
            nav: None,
        }),
    }
}

fn make_box_node_with_style(
    id: NodeId,
    rect: Rect,
    direction: Direction,
    scope: bool,
    children: Vec<LayoutNode>,
) -> LayoutNode {
    LayoutNode {
        rect,
        content: LayoutContent::Box(LayoutBox {
            node_id: id,
            style: BoxStyle {
                direction,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::ZERO,
                border_mode: BorderMode::Separate,
                alignment: Alignment::Start,
                scope,
                decorations: vec![],
                monolithic: false,
            },
            children,
            nav: None,
        }),
    }
}

fn make_page(y_start: f32, y_end: f32) -> LayoutPage {
    LayoutPage::new(
        y_start,
        y_end,
        editor_common::Size::new(440.0, y_end - y_start),
    )
}

#[test]
fn exact_hit_on_line() {
    let id = NodeId::new();
    let tree = LayoutTree {
        root: make_box_node(
            NodeId::ROOT,
            0.0,
            0.0,
            200.0,
            20.0,
            vec![make_line_node(id, 0.0, 0.0, "hello", 10.0)],
        ),
    };
    let page = make_page(0.0, 100.0);

    let hit = HitTester::for_page(&tree, &page, 25.0, 5.0);
    let sel = hit.exact_target().unwrap().selection(hit.target_x());
    assert!(sel.is_collapsed());
    assert_eq!(sel.head.node_id, id);
}

#[test]
fn exact_hit_on_spacing_returns_none() {
    let tree = LayoutTree {
        root: make_box_node(
            NodeId::ROOT,
            0.0,
            0.0,
            200.0,
            50.0,
            vec![
                make_line_node(NodeId::new(), 0.0, 0.0, "hi", 10.0),
                LayoutNode {
                    rect: Rect::from_xywh(0.0, 20.0, 0.0, 16.0),
                    content: LayoutContent::Spacing(SpacingKind::Gap),
                },
                make_line_node(NodeId::new(), 0.0, 36.0, "lo", 10.0),
            ],
        ),
    };
    let page = make_page(0.0, 100.0);

    // Click in the spacing area (y=25)
    let hit = HitTester::for_page(&tree, &page, 5.0, 25.0);
    assert!(hit.exact_target().is_none());
}

#[test]
fn closest_hit_on_spacing_returns_nearest_line() {
    let id1 = NodeId::new();
    let id2 = NodeId::new();
    let tree = LayoutTree {
        root: make_box_node(
            NodeId::ROOT,
            0.0,
            0.0,
            200.0,
            60.0,
            vec![
                make_line_node(id1, 0.0, 0.0, "hi", 10.0),
                LayoutNode {
                    rect: Rect::from_xywh(0.0, 20.0, 0.0, 16.0),
                    content: LayoutContent::Spacing(SpacingKind::Gap),
                },
                make_line_node(id2, 0.0, 36.0, "lo", 10.0),
            ],
        ),
    };
    let page = make_page(0.0, 100.0);

    // Click in spacing (y=25) -- should find closest line
    let hit = HitTester::for_page(&tree, &page, 5.0, 25.0);
    let sel = hit.closest_target().unwrap().selection(hit.target_x());
    assert!(sel.is_collapsed());
    // Should be id1 (closer: line1 bottom at 20, dist=5; line2 top at 36, dist=11)
    assert_eq!(sel.head.node_id, id1);
}

#[test]
fn closest_hit_in_margin_returns_nearest() {
    let id = NodeId::new();
    let tree = LayoutTree {
        root: make_box_node(
            NodeId::ROOT,
            20.0,
            20.0,
            400.0,
            40.0,
            vec![make_line_node(id, 20.0, 20.0, "hello", 10.0)],
        ),
    };
    let page = make_page(0.0, 200.0);

    // Click in margin area (x=5, y=5) -- outside all boxes
    let hit = HitTester::for_page(&tree, &page, 5.0, 5.0);
    let sel = hit.closest_target().unwrap().selection(hit.target_x());
    assert_eq!(sel.head.node_id, id);
}

#[test]
fn block_gap_above_root_content_returns_root_start() {
    let (doc, p1, p2) = doc! {
        root {
            p1: paragraph {}
            p2: paragraph {}
        }
    };
    let tree = LayoutTree {
        root: make_box_node(
            NodeId::ROOT,
            0.0,
            20.0,
            200.0,
            80.0,
            vec![
                make_box_node(p1, 0.0, 20.0, 200.0, 20.0, vec![]),
                make_box_node(p2, 0.0, 60.0, 200.0, 20.0, vec![]),
            ],
        ),
    };
    let page = make_page(0.0, 120.0);

    let hit = HitTester::for_page(&tree, &page, 10.0, 0.0);

    assert_eq!(
        hit.block_gap_position(&doc),
        Some(Position::new(NodeId::ROOT, 0))
    );
}

#[test]
fn block_gap_below_root_content_returns_root_end() {
    let (doc, p1, p2) = doc! {
        root {
            p1: paragraph {}
            p2: paragraph {}
        }
    };
    let tree = LayoutTree {
        root: make_box_node(
            NodeId::ROOT,
            0.0,
            20.0,
            200.0,
            80.0,
            vec![
                make_box_node(p1, 0.0, 20.0, 200.0, 20.0, vec![]),
                make_box_node(p2, 0.0, 60.0, 200.0, 20.0, vec![]),
            ],
        ),
    };
    let page = make_page(0.0, 120.0);

    let hit = HitTester::for_page(&tree, &page, 10.0, 120.0);

    assert_eq!(
        hit.block_gap_position(&doc),
        Some(Position::new(NodeId::ROOT, 2))
    );
}

#[test]
fn closest_hit_below_paragraph_returns_last_line() {
    // Single paragraph (Box) wraps multiple Lines. Click below the paragraph
    // rect — but still within the page — must land on the LAST line (closest
    // by edge distance), not the FIRST.
    let line1 = NodeId::new();
    let line2 = NodeId::new();
    let line3 = NodeId::new();
    let paragraph = make_box_node(
        NodeId::new(),
        0.0,
        0.0,
        200.0,
        60.0,
        vec![
            make_line_node(line1, 0.0, 0.0, "hi", 10.0),
            make_line_node(line2, 0.0, 20.0, "lo", 10.0),
            make_line_node(line3, 0.0, 40.0, "yo", 10.0),
        ],
    );
    let tree = LayoutTree {
        root: make_box_node(NodeId::ROOT, 0.0, 0.0, 200.0, 200.0, vec![paragraph]),
    };
    let page = make_page(0.0, 200.0);

    // Click at y=100, well below paragraph (ends at y=60).
    let hit = HitTester::for_page(&tree, &page, 5.0, 100.0);
    let sel = hit.closest_target().unwrap().selection(hit.target_x());
    assert_eq!(sel.head.node_id, line3);
}

#[test]
fn rect_distance_sq_inside_is_zero() {
    let rect = Rect::from_xywh(10.0, 10.0, 100.0, 50.0);
    assert_eq!(rect_distance_sq(&rect, 50.0, 30.0), 0.0);
}

#[test]
fn rect_distance_sq_outside() {
    let rect = Rect::from_xywh(10.0, 10.0, 100.0, 50.0);
    // Point at (0, 0) -- dx=10, dy=10 -> dist_sq=200
    assert_eq!(rect_distance_sq(&rect, 0.0, 0.0), 200.0);
}

#[test]
fn closest_hit_stays_within_page() {
    // Two pages, each 1123 tall.
    // Page 0 has a short line near the top (y=0..20).
    // Page 1 has a line right at its top (y=1123..1143).
    // A click at page_y=1000 (near bottom of page 0) is abs_y=1000:
    //   - distance to page 0 line bottom (y=20): 980
    //   - distance to page 1 line top (y=1123): 123
    // Without the fix, closest_hit_test returns page 1's line.
    // With the fix, it must return page 0's line (only candidate in page).
    let id_p0 = NodeId::new();
    let id_p1 = NodeId::new();
    let tree = LayoutTree {
        root: make_box_node(
            NodeId::ROOT,
            0.0,
            0.0,
            200.0,
            2246.0,
            vec![
                make_line_node(id_p0, 0.0, 0.0, "hi", 10.0),
                make_line_node(id_p1, 0.0, 1123.0, "lo", 10.0),
            ],
        ),
    };
    let page_0 = make_page(0.0, 1123.0);

    let hit = HitTester::for_page(&tree, &page_0, 5.0, 1000.0);
    let sel = hit.closest_target().unwrap().selection(hit.target_x());
    assert_eq!(sel.head.node_id, id_p0);
}

#[test]
fn exact_hit_in_monolithic_box_returns_leaf() {
    let line_id = NodeId::new();
    let line = make_line_node(line_id, 0.0, 100.0, "hello", 10.0);
    let mono = LayoutNode {
        rect: Rect::from_xywh(0.0, 100.0, 200.0, 20.0),
        content: LayoutContent::Box(LayoutBox {
            node_id: NodeId::new(),
            style: BoxStyle {
                direction: Direction::Vertical,
                padding: EdgeInsets::ZERO,
                border: EdgeInsets::ZERO,
                border_mode: BorderMode::Separate,
                alignment: Alignment::Start,
                scope: false,
                decorations: vec![],
                monolithic: true,
            },
            children: vec![line],
            nav: None,
        }),
    };
    let tree = LayoutTree {
        root: make_box_node(NodeId::ROOT, 0.0, 0.0, 200.0, 200.0, vec![mono]),
    };
    let page = make_page(0.0, 200.0);

    let hit = HitTester::for_page(&tree, &page, 25.0, 110.0);
    let exact = hit.exact_target().unwrap().selection(hit.target_x());
    assert!(exact.is_collapsed());
    assert_eq!(
        exact.head.node_id, line_id,
        "exact hit inside a monolithic box must return the text leaf"
    );
}

use editor_common::Size;

fn gs(n: usize) -> Vec<GraphemeSpan> {
    vec![
        GraphemeSpan {
            advance: 10.0,
            codepoints: 1
        };
        n
    ]
}

#[test]
fn hit_test_in_empty_trailing_line_returns_paragraph_offset() {
    let p1 = NodeId::new();
    let t1 = NodeId::new();
    let tree = LayoutTree {
        root: LayoutNode {
            rect: Rect::from_xywh(0.0, 0.0, 200.0, 40.0),
            content: LayoutContent::Box(LayoutBox {
                node_id: NodeId::new(),
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    scope: false,
                    decorations: vec![],
                    monolithic: false,
                },
                children: vec![
                    LayoutNode {
                        rect: Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
                        content: LayoutContent::Line(LayoutLine {
                            node_id: p1,
                            baseline: 16.0,
                            ascent: 14.0,
                            descent: 4.0,
                            cursor_ascent: 14.0,
                            cursor_descent: 4.0,
                            glyph_runs: vec![GlyphRun::make_test_run(t1, 0, "a", 0.0, gs(1))],
                            ruby_annotations: vec![],
                            empty_caret_x: 0.0,
                            child_range: Some(0..2),
                            tab_gaps: vec![],
                        }),
                    },
                    LayoutNode {
                        rect: Rect::from_xywh(0.0, 20.0, 200.0, 20.0),
                        content: LayoutContent::Line(LayoutLine {
                            node_id: p1,
                            baseline: 16.0,
                            ascent: 14.0,
                            descent: 4.0,
                            cursor_ascent: 14.0,
                            cursor_descent: 4.0,
                            glyph_runs: vec![],
                            ruby_annotations: vec![],
                            empty_caret_x: 0.0,
                            child_range: Some(2..2),
                            tab_gaps: vec![],
                        }),
                    },
                ],
                nav: None,
            }),
        },
    };
    let page = LayoutPage::new(0.0, 800.0, Size::new(200.0, 800.0));
    let hit = HitTester::for_page(&tree, &page, 50.0, 30.0);
    let sel = hit.closest_target().unwrap().selection(hit.target_x());
    assert_eq!(sel.head.node_id, p1);
    assert_eq!(sel.head.offset, 2);
}

#[test]
fn click_left_of_line_goes_to_start() {
    let id = NodeId::new();
    // Line at x=20 (margin), text "hello" with 10px per char
    let tree = LayoutTree {
        root: make_box_node(
            NodeId::ROOT,
            20.0,
            0.0,
            200.0,
            20.0,
            vec![make_line_node(id, 20.0, 0.0, "hello", 10.0)],
        ),
    };
    let page = make_page(0.0, 100.0);

    // Click at x=5 (left margin, before line x=20)
    let hit = HitTester::for_page(&tree, &page, 5.0, 5.0);
    let sel = hit.exact_target();
    // exact misses (x=5 is outside line rect at x=20)
    assert!(sel.is_none());
    // closest finds the line, cursor should be at offset 0 (start)
    let hit = HitTester::for_page(&tree, &page, 5.0, 5.0);
    let sel = hit.closest_target().unwrap().selection(hit.target_x());
    assert_eq!(sel.head.node_id, id);
    assert_eq!(sel.head.offset, 0);
}

#[test]
fn exact_hit_on_atom_node_selects_atom() {
    let parent = NodeId::new();
    let atom_id = NodeId::new();
    let atom = LayoutNode {
        rect: Rect::from_xywh(10.0, 0.0, 100.0, 40.0),
        content: LayoutContent::Atom(LayoutAtom {
            node_id: atom_id,
            parent_id: parent,
            index: 0,
        }),
    };
    let tree = LayoutTree {
        root: make_box_node(NodeId::ROOT, 0.0, 0.0, 200.0, 40.0, vec![atom]),
    };
    let page = make_page(0.0, 100.0);
    // page-local (50,20) → abs (50,20) ∈ atom rect (10..110, 0..40).
    let hit = HitTester::for_page(&tree, &page, 50.0, 20.0);
    let sel = hit.exact_target().unwrap().selection(hit.target_x());
    assert!(
        !sel.is_collapsed(),
        "click on atom must node-select, got {:?}",
        sel
    );
    assert_eq!(
        sel.anchor,
        Position {
            node_id: parent,
            offset: 0,
            affinity: Affinity::Downstream
        }
    );
    assert_eq!(
        sel.head,
        Position {
            node_id: parent,
            offset: 1,
            affinity: Affinity::Upstream
        }
    );
}

#[test]
fn closest_hit_in_table_row_side_margin_stays_in_nearest_cell_scope() {
    let (_doc, table, row, left_cell, right_cell, left_p, right_p, below_p) = doc! {
        root {
            table: table {
                row: table_row {
                    left_cell: table_cell { left_p: paragraph {} }
                    right_cell: table_cell { right_p: paragraph {} }
                }
            }
            below_p: paragraph {}
        }
    };
    let tree = LayoutTree {
        root: make_box_node(
            NodeId::ROOT,
            0.0,
            0.0,
            300.0,
            80.0,
            vec![
                make_box_node(
                    table,
                    0.0,
                    0.0,
                    200.0,
                    40.0,
                    vec![make_box_node_with_style(
                        row,
                        Rect::from_xywh(0.0, 0.0, 200.0, 40.0),
                        Direction::Horizontal,
                        false,
                        vec![
                            make_box_node_with_style(
                                left_cell,
                                Rect::from_xywh(0.0, 0.0, 100.0, 40.0),
                                Direction::Vertical,
                                true,
                                vec![make_line_node(left_p, 10.0, 10.0, "", 10.0)],
                            ),
                            make_box_node_with_style(
                                right_cell,
                                Rect::from_xywh(100.0, 0.0, 100.0, 40.0),
                                Direction::Vertical,
                                true,
                                vec![make_line_node(right_p, 110.0, 10.0, "", 10.0)],
                            ),
                        ],
                    )],
                ),
                make_line_node(below_p, 0.0, 45.0, "below", 60.0),
            ],
        ),
    };
    let page = make_page(0.0, 100.0);

    let hit = HitTester::for_page(&tree, &page, 290.0, 20.0);
    let sel = hit
        .closest_target()
        .map(|target| target.selection(hit.target_x()))
        .expect("same-row side margin should resolve into a table cell scope");

    assert_eq!(sel.head.node_id, right_p);
}
