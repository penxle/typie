use editor_common::Rect;

use editor_state::{Affinity, Position, Selection};

use crate::page::LayoutPage;
use crate::paginate::*;

/// Exact hit: returns Selection only at precise coordinate on navigable node.
pub fn exact_hit_test(
    tree: &LayoutTree,
    page: &LayoutPage,
    x: f32,
    page_y: f32,
) -> Option<Selection> {
    let abs_y = page_y + page.y_start;
    exact_hit_node(&tree.root, x, abs_y)
}

fn exact_hit_node(node: &LayoutNode, x: f32, y: f32) -> Option<Selection> {
    match &node.content {
        LayoutContent::Box(b) => {
            if !node.rect.contains(x, y) {
                return None;
            }
            for child in &b.children {
                if let Some(sel) = exact_hit_node(child, x, y) {
                    return Some(sel);
                }
            }
            None
        }
        LayoutContent::Line(l) => {
            if y >= node.rect.y && y < node.rect.y + node.rect.height {
                Some(navigate_to_line(l, &node.rect, x))
            } else {
                None
            }
        }
        LayoutContent::Atom(a) => {
            if node.rect.contains(x, y) {
                Some(select_atom(a))
            } else {
                None
            }
        }
        LayoutContent::Spacing(_) => None,
    }
}

/// Closest hit: returns the nearest navigable by euclidean edge distance,
/// restricted to navigables owned by the given page (by `rect.y` range).
pub fn closest_hit_test(
    tree: &LayoutTree,
    page: &LayoutPage,
    x: f32,
    page_y: f32,
) -> Option<Selection> {
    let abs_y = page_y + page.y_start;
    let nav = closest_navigable(&tree.root, x, abs_y, page.y_start, page.y_end)?;
    Some(navigate_to_node(nav, x))
}

/// Extending variant: when the click fully escapes a monolithic ancestor's
/// vertical range, promote the head to that container's slot boundary so the
/// drag/shift-extend can select the container as a unit. Plain (non-extending)
/// hit testing must use [`closest_hit_test`] instead.
pub fn closest_hit_test_extending(
    tree: &LayoutTree,
    page: &LayoutPage,
    x: f32,
    page_y: f32,
) -> Option<Selection> {
    let abs_y = page_y + page.y_start;
    let nav = closest_navigable(&tree.root, x, abs_y, page.y_start, page.y_end)?;
    if let Some(promoted) = promote_outside_y(&tree.root, nav, abs_y) {
        return Some(Selection::collapsed(promoted));
    }
    Some(navigate_to_node(nav, x))
}

/// When the click sits outside the vertical range of `leaf`'s monolithic
/// ancestor boxes, snap the head up the structural ancestry to the slot
/// boundary of the outermost monolithic box the click fully escaped. Above
/// the box → its Front slot `(parent, idx)`; below → its Back slot
/// `(parent, idx + 1)`. Without this, dragging the selection past a
/// monolithic container (e.g. fold) stalls at the container's innermost text
/// position, making it impossible to select the container as a unit.
fn promote_outside_y(root: &LayoutNode, leaf: &LayoutNode, click_y: f32) -> Option<Position> {
    let mut path: Vec<(&LayoutNode, usize)> = Vec::new();
    if !build_path(root, leaf, &mut path) {
        return None;
    }
    for k in 1..path.len() {
        let ancestor = path[k].0;
        let LayoutContent::Box(ancestor_box) = &ancestor.content else {
            continue;
        };
        if !ancestor_box.style.monolithic {
            continue;
        }
        let above = click_y < ancestor.rect.y;
        let below = click_y >= ancestor.rect.y + ancestor.rect.height;
        if above || below {
            let (parent_box_node, idx) = path[k - 1];
            if let LayoutContent::Box(parent_box) = &parent_box_node.content {
                let slot = if below { idx + 1 } else { idx };
                return Some(Position::new(parent_box.node_id, slot));
            }
        }
    }
    None
}

fn build_path<'a>(
    node: &'a LayoutNode,
    target: &LayoutNode,
    path: &mut Vec<(&'a LayoutNode, usize)>,
) -> bool {
    if std::ptr::eq(node, target) {
        return true;
    }
    let LayoutContent::Box(b) = &node.content else {
        return false;
    };
    let mut content_idx = 0usize;
    for child in &b.children {
        let is_spacing = matches!(child.content, LayoutContent::Spacing(_));
        if is_spacing {
            continue;
        }
        path.push((node, content_idx));
        if build_path(child, target, path) {
            return true;
        }
        path.pop();
        content_idx += 1;
    }
    false
}

/// Find the closest navigable node by squared euclidean rect-edge distance.
/// Descends into the innermost containing box first, then falls back to all children.
/// Leaves (Line/Atom) are only considered if their `rect.y` lies within `[y_start, y_end)`.
fn closest_navigable(
    node: &LayoutNode,
    x: f32,
    y: f32,
    y_start: f32,
    y_end: f32,
) -> Option<&LayoutNode> {
    match &node.content {
        LayoutContent::Box(b) => {
            for child in &b.children {
                if child.rect.contains(x, y)
                    && let Some(found) = closest_navigable(child, x, y, y_start, y_end)
                {
                    return Some(found);
                }
            }
            // No containing child found; pick globally closest leaf in range.
            closest_navigable_in_range(node, x, y, y_start, y_end).map(|(_, n)| n)
        }
        LayoutContent::Line(_) | LayoutContent::Atom(_) => {
            if node.rect.y >= y_start && node.rect.y < y_end {
                Some(node)
            } else {
                None
            }
        }
        LayoutContent::Spacing(_) => None,
    }
}

/// Find the navigable descendant (Line or Atom) of `node` whose `rect.y` lies
/// within `[y_start, y_end)` and is closest to `(x, y)` by squared rect-edge
/// distance. Returns `(dist_sq, leaf)`.
fn closest_navigable_in_range(
    node: &LayoutNode,
    x: f32,
    y: f32,
    y_start: f32,
    y_end: f32,
) -> Option<(f32, &LayoutNode)> {
    match &node.content {
        LayoutContent::Box(b) => b
            .children
            .iter()
            .filter_map(|c| closest_navigable_in_range(c, x, y, y_start, y_end))
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)),
        LayoutContent::Line(_) | LayoutContent::Atom(_) => {
            if node.rect.y >= y_start && node.rect.y < y_end {
                Some((rect_distance_sq(&node.rect, x, y), node))
            } else {
                None
            }
        }
        LayoutContent::Spacing(_) => None,
    }
}

/// Squared euclidean distance from point (x, y) to the nearest edge of rect.
/// Returns 0 if point is inside rect.
pub fn rect_distance_sq(rect: &Rect, x: f32, y: f32) -> f32 {
    let dx = if x < rect.x {
        rect.x - x
    } else if x > rect.x + rect.width {
        x - (rect.x + rect.width)
    } else {
        0.0
    };
    let dy = if y < rect.y {
        rect.y - y
    } else if y > rect.y + rect.height {
        y - (rect.y + rect.height)
    } else {
        0.0
    };
    dx * dx + dy * dy
}

fn navigate_to_line(line: &LayoutLine, rect: &Rect, x: f32) -> Selection {
    Selection::collapsed(position_in_line(line, rect, x))
}

fn position_in_line(line: &LayoutLine, rect: &Rect, x: f32) -> Position {
    let local_x = x - rect.x;
    super::grapheme::position_at_x(line, local_x)
}

fn select_atom(atom: &LayoutAtom) -> Selection {
    Selection::new(
        Position {
            node_id: atom.parent_id,
            offset: atom.index,
            affinity: Affinity::Downstream,
        },
        Position {
            node_id: atom.parent_id,
            offset: atom.index + 1,
            affinity: Affinity::Upstream,
        },
    )
}

fn navigate_to_node(node: &LayoutNode, x: f32) -> Selection {
    match &node.content {
        LayoutContent::Line(l) => navigate_to_line(l, &node.rect, x),
        LayoutContent::Atom(a) => select_atom(a),
        _ => unreachable!("navigate_to_node called on non-navigable"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph_run::{GlyphRun, GraphemeSpan};
    use crate::style::*;
    use crate::view::View;
    use editor_common::EdgeInsets;
    use editor_macros::doc;
    use editor_model::NodeId;

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
                text_indent: 0.0,
                child_range: None,
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
            }),
        }
    }

    fn make_page(y_start: f32, y_end: f32) -> LayoutPage {
        LayoutPage {
            y_start,
            y_end,
            size: editor_common::Size::new(440.0, y_end - y_start),
        }
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

        let sel = exact_hit_test(&tree, &page, 25.0, 5.0).unwrap();
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
        assert!(exact_hit_test(&tree, &page, 5.0, 25.0).is_none());
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
        let sel = closest_hit_test(&tree, &page, 5.0, 25.0).unwrap();
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
        let sel = closest_hit_test(&tree, &page, 5.0, 5.0).unwrap();
        assert_eq!(sel.head.node_id, id);
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
        let sel = closest_hit_test(&tree, &page, 5.0, 100.0).unwrap();
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

        let sel = closest_hit_test(&tree, &page_0, 5.0, 1000.0).unwrap();
        assert_eq!(sel.head.node_id, id_p0);
    }

    #[test]
    fn extending_drag_above_top_promotes_to_front_slot() {
        let (doc,) = doc! {
            root {
                fold {
                    fold_title { text("title") }
                    fold_content { paragraph { text("content") } }
                }
                paragraph {}
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);

        let sel = view.hit_test_extending(0, 20.0, -100.0).unwrap();
        assert!(sel.is_collapsed());
        assert_eq!(sel.head.node_id, NodeId::ROOT);
        assert_eq!(sel.head.offset, 0, "above-top escape → Front slot (idx)");
    }

    #[test]
    fn exact_hit_in_monolithic_box_returns_leaf_while_closest_above_promotes() {
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
            }),
        };
        let tree = LayoutTree {
            root: make_box_node(NodeId::ROOT, 0.0, 0.0, 200.0, 200.0, vec![mono]),
        };
        let page = make_page(0.0, 200.0);

        let exact = exact_hit_test(&tree, &page, 25.0, 110.0).unwrap();
        assert!(exact.is_collapsed());
        assert_eq!(
            exact.head.node_id, line_id,
            "exact hit inside a monolithic box must return the text leaf"
        );

        let promoted = closest_hit_test_extending(&tree, &page, 25.0, -50.0).unwrap();
        assert!(promoted.is_collapsed());
        assert_eq!(
            promoted.head.node_id,
            NodeId::ROOT,
            "above the monolithic box, the extending closest path must promote"
        );
        assert_eq!(promoted.head.offset, 0);
    }

    #[test]
    fn extending_drag_below_bottom_promotes_to_back_slot() {
        let (doc,) = doc! {
            root {
                paragraph {}
                fold {
                    fold_title { text("title") }
                    fold_content { paragraph { text("content") } }
                }
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);

        let sel = view.hit_test_extending(0, 20.0, 99999.0).unwrap();
        assert!(sel.is_collapsed());
        assert_eq!(sel.head.node_id, NodeId::ROOT);
        assert_eq!(
            sel.head.offset, 2,
            "below-bottom escape → Back slot (idx+1)"
        );
    }

    #[test]
    fn plain_hit_test_in_gutter_does_not_promote() {
        let (doc,) = doc! {
            root {
                fold {
                    fold_title { text("title") }
                    fold_content { paragraph { text("content") } }
                }
                paragraph {}
            }
        };
        let mut view = View::new_test();
        view.layout(&doc);

        let sel = view.hit_test(0, 20.0, -100.0).unwrap();
        assert!(sel.is_collapsed());
        assert_ne!(
            sel.head.node_id,
            NodeId::ROOT,
            "plain Down must not promote; head stays on nearest text leaf"
        );
        let ft_text = doc
            .node(NodeId::ROOT)
            .unwrap()
            .children()
            .next()
            .unwrap() // fold
            .children()
            .next()
            .unwrap() // fold_title
            .children()
            .next()
            .unwrap() // text("title")
            .id();
        assert_eq!(
            sel.head.node_id, ft_text,
            "plain Down must land on the nearest text leaf (fold_title text), not promote"
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
                                text_indent: 0.0,
                                child_range: Some(0..2),
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
                                text_indent: 0.0,
                                child_range: Some(2..2),
                            }),
                        },
                    ],
                }),
            },
        };
        let page = LayoutPage {
            y_start: 0.0,
            y_end: 800.0,
            size: Size::new(200.0, 800.0),
        };
        let sel = closest_hit_test(&tree, &page, 50.0, 30.0).unwrap();
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
        let sel = exact_hit_test(&tree, &page, 5.0, 5.0);
        // exact misses (x=5 is outside line rect at x=20)
        assert!(sel.is_none());
        // closest finds the line, cursor should be at offset 0 (start)
        let sel = closest_hit_test(&tree, &page, 5.0, 5.0).unwrap();
        assert_eq!(sel.head.node_id, id);
        assert_eq!(sel.head.offset, 0);
    }
}
