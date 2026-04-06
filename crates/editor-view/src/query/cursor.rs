use editor_common::Rect;
use editor_state::Position;

use crate::page::{LayoutPage, PageRect};
use crate::paginate::*;

use super::search;

pub fn cursor_rect(tree: &LayoutTree, pages: &[LayoutPage], pos: &Position) -> Option<PageRect> {
    let line_node = search::find_line_at(tree, pos)?;
    let line = match &line_node.content {
        LayoutContent::Line(l) => l,
        LayoutContent::Atom(_) => {
            let page_idx = pages
                .iter()
                .position(|p| line_node.rect.y >= p.y_start && line_node.rect.y < p.y_end)?;
            return Some(PageRect::new(
                page_idx,
                Rect::from_xywh(
                    line_node.rect.x,
                    line_node.rect.y - pages[page_idx].y_start,
                    1.0,
                    line_node.rect.height,
                ),
            ));
        }
        _ => return None,
    };

    let x = x_at_offset(line, pos);
    let page_idx = pages
        .iter()
        .position(|p| line_node.rect.y >= p.y_start && line_node.rect.y < p.y_end)?;

    Some(PageRect::new(
        page_idx,
        Rect::from_xywh(
            line_node.rect.x + x,
            line_node.rect.y - pages[page_idx].y_start,
            1.0,
            line_node.rect.height,
        ),
    ))
}

pub fn x_at_offset(line: &LayoutLine, pos: &Position) -> f32 {
    super::grapheme::x_at_offset(line, pos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph_run::{GlyphRun, GraphemeSpan};
    use crate::style::*;
    use editor_common::{Alignment, EdgeInsets, Size};
    use editor_model::NodeId;

    fn gs(n: usize) -> Vec<GraphemeSpan> {
        vec![
            GraphemeSpan {
                advance: 10.0,
                codepoints: 1
            };
            n
        ]
    }

    fn make_tree(id: NodeId) -> LayoutTree {
        LayoutTree {
            root: LayoutNode {
                rect: Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
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
                    children: vec![LayoutNode {
                        rect: Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
                        content: LayoutContent::Line(LayoutLine {
                            node_id: id,
                            baseline: 16.0,
                            glyph_runs: vec![GlyphRun::make_test_run(id, 0, "hello", 0.0, gs(5))],
                        }),
                    }],
                }),
            },
        }
    }

    #[test]
    fn cursor_rect_at_offset_0() {
        let id = NodeId::new();
        let tree = make_tree(id);
        let pages = [LayoutPage {
            y_start: 0.0,
            y_end: 800.0,
            size: Size::new(200.0, 800.0),
        }];
        let pos = Position::new(id, 0);
        let PageRect { page_idx, rect } = cursor_rect(&tree, &pages, &pos).unwrap();

        assert_eq!(page_idx, 0);
        assert_eq!(rect.x, 0.0);
        assert_eq!(rect.y, 0.0);
        assert_eq!(rect.height, 20.0);
    }

    #[test]
    fn cursor_rect_at_offset_3() {
        let id = NodeId::new();
        let tree = make_tree(id);
        let pages = [LayoutPage {
            y_start: 0.0,
            y_end: 800.0,
            size: Size::new(200.0, 800.0),
        }];
        let pos = Position::new(id, 3);
        let PageRect { rect, .. } = cursor_rect(&tree, &pages, &pos).unwrap();

        assert_eq!(rect.x, 30.0);
    }

    #[test]
    fn cursor_rect_includes_line_x_offset() {
        let id = NodeId::new();
        let tree = LayoutTree {
            root: LayoutNode {
                rect: Rect::from_xywh(20.0, 0.0, 200.0, 20.0),
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
                    children: vec![LayoutNode {
                        rect: Rect::from_xywh(20.0, 0.0, 200.0, 20.0),
                        content: LayoutContent::Line(LayoutLine {
                            node_id: id,
                            baseline: 16.0,
                            glyph_runs: vec![GlyphRun::make_test_run(id, 0, "hello", 0.0, gs(5))],
                        }),
                    }],
                }),
            },
        };
        let pages = [LayoutPage {
            y_start: 0.0,
            y_end: 800.0,
            size: Size::new(240.0, 800.0),
        }];
        let pos = Position::new(id, 2);
        let PageRect { rect, .. } = cursor_rect(&tree, &pages, &pos).unwrap();

        // x = line.rect.x(20) + run.x(0) + advances[0..2](20) = 40
        assert_eq!(rect.x, 40.0);
    }

    #[test]
    fn cursor_rect_returns_correct_page() {
        let id = NodeId::new();
        // Line at y=500, pages split at y=400
        let tree = LayoutTree {
            root: LayoutNode {
                rect: Rect::from_xywh(0.0, 0.0, 200.0, 600.0),
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
                    children: vec![LayoutNode {
                        rect: Rect::from_xywh(0.0, 500.0, 200.0, 20.0),
                        content: LayoutContent::Line(LayoutLine {
                            node_id: id,
                            baseline: 16.0,
                            glyph_runs: vec![GlyphRun::make_test_run(id, 0, "hello", 0.0, gs(5))],
                        }),
                    }],
                }),
            },
        };
        let pages = [
            LayoutPage {
                y_start: 0.0,
                y_end: 400.0,
                size: Size::new(200.0, 400.0),
            },
            LayoutPage {
                y_start: 400.0,
                y_end: 800.0,
                size: Size::new(200.0, 400.0),
            },
        ];
        let pos = Position::new(id, 0);
        let PageRect { page_idx, rect } = cursor_rect(&tree, &pages, &pos).unwrap();

        assert_eq!(page_idx, 1);
        // y should be relative to page start: 500 - 400 = 100
        assert_eq!(rect.y, 100.0);
    }
}
