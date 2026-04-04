use editor_state::Position;

use crate::paginate::*;

/// Find the LayoutNode (Line or Atom) containing a given Position.
pub fn find_line_at<'a>(tree: &'a LayoutTree, pos: &Position) -> Option<&'a LayoutNode> {
    find_line_in(&tree.root, pos)
}

fn find_line_in<'a>(node: &'a LayoutNode, pos: &Position) -> Option<&'a LayoutNode> {
    match &node.content {
        LayoutContent::Box(b) => b.children.iter().find_map(|child| find_line_in(child, pos)),
        LayoutContent::Line(l) => {
            // Empty line: match by paragraph node_id at offset 0
            if l.glyph_runs.is_empty() {
                return if l.node_id == pos.node_id && pos.offset == 0 {
                    Some(node)
                } else {
                    None
                };
            }
            for run in &l.glyph_runs {
                if run.node_id == pos.node_id
                    && pos.offset >= run.offset
                    && pos.offset <= run.offset + run.char_advances.len()
                {
                    return Some(node);
                }
            }
            None
        }
        LayoutContent::Atom(a) => {
            if a.parent_id == pos.node_id && pos.offset >= a.index && pos.offset <= a.index + 1 {
                Some(node)
            } else {
                None
            }
        }
        LayoutContent::Spacing(_) => None,
    }
}

/// Find the first navigable (Line or Atom) node in a subtree.
pub fn find_first_navigable(node: &LayoutNode) -> Option<&LayoutNode> {
    match &node.content {
        LayoutContent::Box(b) => b.children.iter().find_map(find_first_navigable),
        LayoutContent::Line(_) | LayoutContent::Atom(_) => Some(node),
        LayoutContent::Spacing(_) => None,
    }
}

/// Find the last navigable (Line or Atom) node in a subtree.
pub fn find_last_navigable(node: &LayoutNode) -> Option<&LayoutNode> {
    match &node.content {
        LayoutContent::Box(b) => b.children.iter().rev().find_map(find_last_navigable),
        LayoutContent::Line(_) | LayoutContent::Atom(_) => Some(node),
        LayoutContent::Spacing(_) => None,
    }
}

/// Find the first navigable node whose bottom edge is below `y`.
pub fn find_navigable_below<'a>(node: &'a LayoutNode, y: f32) -> Option<&'a LayoutNode> {
    match &node.content {
        LayoutContent::Box(b) => {
            for child in &b.children {
                if let Some(nav) = find_navigable_below(child, y) {
                    return Some(nav);
                }
            }
            None
        }
        LayoutContent::Line(_) | LayoutContent::Atom(_) => {
            if node.rect.y >= y {
                Some(node)
            } else {
                None
            }
        }
        LayoutContent::Spacing(_) => None,
    }
}

/// Find the last navigable node whose top edge is above `y`.
pub fn find_navigable_above<'a>(node: &'a LayoutNode, y: f32) -> Option<&'a LayoutNode> {
    match &node.content {
        LayoutContent::Box(b) => {
            for child in b.children.iter().rev() {
                if let Some(nav) = find_navigable_above(child, y) {
                    return Some(nav);
                }
            }
            None
        }
        LayoutContent::Line(_) | LayoutContent::Atom(_) => {
            if node.rect.bottom() <= y {
                Some(node)
            } else {
                None
            }
        }
        LayoutContent::Spacing(_) => None,
    }
}

/// Find the innermost scope container (style.scope == true) that contains a given position.
pub fn find_scope_container_at<'a>(node: &'a LayoutNode, pos: &Position) -> Option<&'a LayoutNode> {
    match &node.content {
        LayoutContent::Box(b) => {
            // Try children first so the innermost scope wins
            for child in &b.children {
                if let Some(scope) = find_scope_container_at(child, pos) {
                    return Some(scope);
                }
            }
            if b.style.scope && contains_position(node, pos) {
                Some(node)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn contains_position(node: &LayoutNode, pos: &Position) -> bool {
    find_line_in(node, pos).is_some()
}

#[cfg(test)]
mod tests {
    use editor_common::{Alignment, EdgeInsets, Rect};
    use editor_model::NodeId;

    use super::*;
    use crate::glyph_run::GlyphRun;
    use crate::style::*;

    fn make_line_node(id: NodeId, y: f32) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: id,
                baseline: 16.0,
                glyph_runs: vec![GlyphRun::make_test_run(id, 0, "test", 0.0, vec![10.0; 4])],
            }),
        }
    }

    fn make_box_node(y: f32, h: f32, children: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, h),
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
                },
                children,
            }),
        }
    }

    fn make_scope_box(y: f32, h: f32, children: Vec<LayoutNode>) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, h),
            content: LayoutContent::Box(LayoutBox {
                node_id: NodeId::new(),
                style: BoxStyle {
                    direction: Direction::Vertical,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    scope: true,
                    decorations: vec![],
                },
                children,
            }),
        }
    }

    #[test]
    fn find_first_navigable_skips_spacing() {
        let id = NodeId::new();
        let root = make_box_node(
            0.0,
            40.0,
            vec![
                LayoutNode {
                    rect: Rect::from_xywh(0.0, 0.0, 0.0, 10.0),
                    content: LayoutContent::Spacing(SpacingKind::Gap),
                },
                make_line_node(id, 10.0),
            ],
        );
        let nav = find_first_navigable(&root).unwrap();
        match &nav.content {
            LayoutContent::Line(l) => assert_eq!(l.node_id, id),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn find_last_navigable_returns_bottom() {
        let id = NodeId::new();
        let root = make_box_node(
            0.0,
            40.0,
            vec![make_line_node(NodeId::new(), 0.0), make_line_node(id, 20.0)],
        );
        let nav = find_last_navigable(&root).unwrap();
        match &nav.content {
            LayoutContent::Line(l) => assert_eq!(l.node_id, id),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn find_line_at_locates_position() {
        let id = NodeId::new();
        let tree = LayoutTree {
            root: make_box_node(
                0.0,
                40.0,
                vec![make_line_node(id, 0.0), make_line_node(NodeId::new(), 20.0)],
            ),
        };
        let pos = Position::new(id, 2);
        let node = find_line_at(&tree, &pos).unwrap();
        match &node.content {
            LayoutContent::Line(l) => assert_eq!(l.node_id, id),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn find_navigable_below_finds_next() {
        let id2 = NodeId::new();
        let root = make_box_node(
            0.0,
            40.0,
            vec![
                make_line_node(NodeId::new(), 0.0),
                make_line_node(id2, 20.0),
            ],
        );
        let nav = find_navigable_below(&root, 10.0).unwrap();
        match &nav.content {
            LayoutContent::Line(l) => assert_eq!(l.node_id, id2),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn find_navigable_above_finds_prev() {
        let id1 = NodeId::new();
        let root = make_box_node(
            0.0,
            40.0,
            vec![
                make_line_node(id1, 0.0),
                make_line_node(NodeId::new(), 20.0),
            ],
        );
        let nav = find_navigable_above(&root, 20.0).unwrap();
        match &nav.content {
            LayoutContent::Line(l) => assert_eq!(l.node_id, id1),
            _ => panic!("expected Line"),
        }
    }

    #[test]
    fn find_navigable_returns_none_at_boundary() {
        let root = make_box_node(0.0, 20.0, vec![make_line_node(NodeId::new(), 0.0)]);
        assert!(find_navigable_above(&root, 0.0).is_none());
    }

    #[test]
    fn find_line_at_matches_empty_line() {
        let para_id = NodeId::new();
        let tree = LayoutTree {
            root: make_box_node(
                0.0,
                20.0,
                vec![LayoutNode {
                    rect: Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
                    content: LayoutContent::Line(LayoutLine {
                        node_id: para_id,
                        baseline: 16.0,
                        glyph_runs: vec![], // empty line
                    }),
                }],
            ),
        };
        let pos = Position::new(para_id, 0);
        let node = find_line_at(&tree, &pos);
        assert!(node.is_some());
    }

    #[test]
    fn find_scope_container_finds_innermost() {
        let id = NodeId::new();
        let tree = LayoutTree {
            root: make_scope_box(
                0.0,
                40.0,
                vec![make_scope_box(0.0, 20.0, vec![make_line_node(id, 0.0)])],
            ),
        };
        let pos = Position::new(id, 0);
        let scope = find_scope_container_at(&tree.root, &pos).unwrap();
        // Should be the inner scope (rect height 20), not the outer (40)
        assert_eq!(scope.rect.height, 20.0);
    }
}
