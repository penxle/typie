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

/// Closest hit: returns the nearest navigable by euclidean edge distance.
pub fn closest_hit_test(
    tree: &LayoutTree,
    page: &LayoutPage,
    x: f32,
    page_y: f32,
) -> Option<Selection> {
    let abs_y = page_y + page.y_start;
    let nav = closest_navigable(&tree.root, x, abs_y)?;
    Some(navigate_to_node(nav, x))
}

/// Find the closest navigable node by squared euclidean rect-edge distance.
/// Descends into the innermost containing box first, then falls back to all children.
fn closest_navigable(node: &LayoutNode, x: f32, y: f32) -> Option<&LayoutNode> {
    match &node.content {
        LayoutContent::Box(b) => {
            for child in &b.children {
                if child.rect.contains(x, y)
                    && let Some(found) = closest_navigable(child, x, y)
                {
                    return Some(found);
                }
            }
            // No containing child found; search all children by edge distance
            closest_navigable_in_children(&b.children, x, y)
        }
        LayoutContent::Line(_) | LayoutContent::Atom(_) => Some(node),
        LayoutContent::Spacing(_) => None,
    }
}

fn closest_navigable_in_children(children: &[LayoutNode], x: f32, y: f32) -> Option<&LayoutNode> {
    children
        .iter()
        .filter_map(|child| {
            find_any_navigable(child).map(|nav| {
                let dist_sq = rect_distance_sq(&nav.rect, x, y);
                (dist_sq, nav)
            })
        })
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, nav)| nav)
}

/// Find ANY navigable descendant (first Line or Atom found).
fn find_any_navigable(node: &LayoutNode) -> Option<&LayoutNode> {
    match &node.content {
        LayoutContent::Box(b) => b.children.iter().find_map(find_any_navigable),
        LayoutContent::Line(_) | LayoutContent::Atom(_) => Some(node),
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
    use editor_common::{Alignment, EdgeInsets};
    use editor_model::NodeId;

    fn make_line_node(id: NodeId, x: f32, y: f32, text: &str, char_w: f32) -> LayoutNode {
        let n = text.chars().count();
        LayoutNode {
            rect: Rect::from_xywh(x, y, n as f32 * char_w, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: id,
                baseline: 16.0,
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
