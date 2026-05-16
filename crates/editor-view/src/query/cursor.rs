use editor_common::Rect;
use editor_macros::ffi;
use editor_state::Position;
use serde::{Deserialize, Serialize};

use crate::page::LayoutPage;
use crate::paginate::*;

use super::search;

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CursorMetrics {
    pub page_idx: usize,
    pub caret: Rect,
    pub line: Rect,
}

pub fn cursor_metrics(
    tree: &LayoutTree,
    pages: &[LayoutPage],
    pos: &Position,
    metrics_override: Option<(f32, f32)>,
) -> Option<CursorMetrics> {
    let line_node = search::find_line_at(tree, pos)?;
    let page_idx = pages
        .iter()
        .position(|p| line_node.rect.y >= p.y_start && line_node.rect.y < p.y_end)?;
    let y_start = pages[page_idx].y_start;
    let line_rect = Rect::from_xywh(
        line_node.rect.x,
        line_node.rect.y - y_start,
        line_node.rect.width,
        line_node.rect.height,
    );

    match &line_node.content {
        LayoutContent::Line(l) => {
            let x = x_at_offset(l, pos);
            let (cursor_ascent, cursor_descent) =
                metrics_override.unwrap_or((l.cursor_ascent, l.cursor_descent));
            let cursor_height = cursor_ascent + cursor_descent;
            // Anchor to baseline so mixed-font lines keep the caret aligned with the run's glyphs.
            let caret = Rect::from_xywh(
                line_node.rect.x + x,
                line_node.rect.y + l.baseline - cursor_ascent - y_start,
                1.0,
                cursor_height,
            );
            Some(CursorMetrics {
                page_idx,
                caret,
                line: line_rect,
            })
        }
        // Normalize guarantees a collapsed selection never points at an atom, so this
        // arm is unreachable for well-formed state. Returning None avoids drawing a
        // spurious caret on the atom's left edge on unexpected bypass entry.
        LayoutContent::Atom(_) => None,
        _ => None,
    }
}

pub fn x_at_offset(line: &LayoutLine, pos: &Position) -> f32 {
    super::grapheme::x_at_offset(line, pos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph_run::{GlyphRun, GraphemeSpan};
    use crate::style::*;
    use editor_common::{EdgeInsets, Size};
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
                        monolithic: false,
                    },
                    children: vec![LayoutNode {
                        rect: Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
                        content: LayoutContent::Line(LayoutLine {
                            node_id: id,
                            baseline: 16.0,
                            ascent: 14.0,
                            descent: 4.0,
                            cursor_ascent: 14.0,
                            cursor_descent: 4.0,
                            glyph_runs: vec![GlyphRun::make_test_run(id, 0, "hello", 0.0, gs(5))],
                            text_indent: 0.0,
                            child_range: None,
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
        let CursorMetrics {
            page_idx, caret, ..
        } = cursor_metrics(&tree, &pages, &pos, None).unwrap();

        // Caret anchored to baseline: caret.y = baseline(16) - cursor_ascent(14) = 2.
        assert_eq!(page_idx, 0);
        assert_eq!(caret.x, 0.0);
        assert_eq!(caret.y, 2.0);
        assert_eq!(caret.height, 18.0);
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
        let CursorMetrics { caret, .. } = cursor_metrics(&tree, &pages, &pos, None).unwrap();

        assert_eq!(caret.x, 30.0);
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
                        monolithic: false,
                    },
                    children: vec![LayoutNode {
                        rect: Rect::from_xywh(20.0, 0.0, 200.0, 20.0),
                        content: LayoutContent::Line(LayoutLine {
                            node_id: id,
                            baseline: 16.0,
                            ascent: 14.0,
                            descent: 4.0,
                            cursor_ascent: 14.0,
                            cursor_descent: 4.0,
                            glyph_runs: vec![GlyphRun::make_test_run(id, 0, "hello", 0.0, gs(5))],
                            text_indent: 0.0,
                            child_range: None,
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
        let CursorMetrics { caret, .. } = cursor_metrics(&tree, &pages, &pos, None).unwrap();

        // x = line.rect.x(20) + run.x(0) + advances[0..2](20) = 40
        assert_eq!(caret.x, 40.0);
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
                        monolithic: false,
                    },
                    children: vec![LayoutNode {
                        rect: Rect::from_xywh(0.0, 500.0, 200.0, 20.0),
                        content: LayoutContent::Line(LayoutLine {
                            node_id: id,
                            baseline: 16.0,
                            ascent: 14.0,
                            descent: 4.0,
                            cursor_ascent: 14.0,
                            cursor_descent: 4.0,
                            glyph_runs: vec![GlyphRun::make_test_run(id, 0, "hello", 0.0, gs(5))],
                            text_indent: 0.0,
                            child_range: None,
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
        let CursorMetrics {
            page_idx, caret, ..
        } = cursor_metrics(&tree, &pages, &pos, None).unwrap();

        assert_eq!(page_idx, 1);
        // Page-local baseline-anchored y: line.y(500) + baseline(16) - cursor_ascent(14) - page_start(400) = 102.
        assert_eq!(caret.y, 102.0);
    }

    #[test]
    fn cursor_rect_empty_line_with_text_indent() {
        let id = NodeId::new();
        let tree = LayoutTree {
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
                        monolithic: false,
                    },
                    children: vec![LayoutNode {
                        rect: Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
                        content: LayoutContent::Line(LayoutLine {
                            node_id: id,
                            baseline: 16.0,
                            ascent: 14.0,
                            descent: 4.0,
                            cursor_ascent: 14.0,
                            cursor_descent: 4.0,
                            glyph_runs: vec![],
                            text_indent: 32.0,
                            child_range: Some(0..0),
                        }),
                    }],
                }),
            },
        };
        let pages = [LayoutPage {
            y_start: 0.0,
            y_end: 800.0,
            size: Size::new(200.0, 800.0),
        }];
        let pos = Position::new(id, 0);
        let CursorMetrics { caret, .. } = cursor_metrics(&tree, &pages, &pos, None).unwrap();

        assert_eq!(caret.x, 32.0);
    }

    #[test]
    fn cursor_metrics_line_covers_full_line_box() {
        // line-height=30, baseline=22, cursor_ascent=14 → caret.y = 22 - 14 = 8.
        let id = NodeId::new();
        let tree = LayoutTree {
            root: LayoutNode {
                rect: Rect::from_xywh(0.0, 0.0, 200.0, 30.0),
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
                    children: vec![LayoutNode {
                        rect: Rect::from_xywh(0.0, 0.0, 200.0, 30.0),
                        content: LayoutContent::Line(LayoutLine {
                            node_id: id,
                            baseline: 22.0,
                            ascent: 14.0,
                            descent: 4.0,
                            cursor_ascent: 14.0,
                            cursor_descent: 4.0,
                            glyph_runs: vec![GlyphRun::make_test_run(id, 0, "hello", 0.0, gs(5))],
                            text_indent: 0.0,
                            child_range: None,
                        }),
                    }],
                }),
            },
        };
        let pages = [LayoutPage {
            y_start: 0.0,
            y_end: 800.0,
            size: Size::new(200.0, 800.0),
        }];
        let pos = Position::new(id, 0);
        let CursorMetrics {
            page_idx,
            caret,
            line,
        } = cursor_metrics(&tree, &pages, &pos, None).unwrap();

        assert_eq!(page_idx, 0);
        // caret: baseline(22) - cursor_ascent(14) = 8 (top), height = 18.
        assert_eq!(caret.y, 8.0);
        assert_eq!(caret.height, 18.0);
        // line: line_node.rect 전체
        assert_eq!(line.x, 0.0);
        assert_eq!(line.y, 0.0);
        assert_eq!(line.width, 200.0);
        assert_eq!(line.height, 30.0);
        // line이 caret을 상하로 감쌈
        assert!(line.y < caret.y);
        assert!(line.y + line.height > caret.y + caret.height);
    }

    #[test]
    fn cursor_metrics_atom_returns_none() {
        // Invariant: normalize expands collapsed-on-atom selections to node selections,
        // so the Atom branch is unreachable for well-formed state. cursor_metrics must
        // return None here rather than draw a wrong caret.
        let para_id = NodeId::new();
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
                    children: vec![LayoutNode {
                        rect: Rect::from_xywh(10.0, 5.0, 150.0, 40.0),
                        content: LayoutContent::Atom(LayoutAtom {
                            node_id: NodeId::new(),
                            parent_id: para_id,
                            index: 0,
                        }),
                    }],
                }),
            },
        };
        let pages = [LayoutPage {
            y_start: 0.0,
            y_end: 800.0,
            size: Size::new(200.0, 800.0),
        }];
        let pos = Position::new(para_id, 0);
        assert!(cursor_metrics(&tree, &pages, &pos, None).is_none());
    }

    #[test]
    fn cursor_metrics_on_trailing_empty_line_after_hard_break() {
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
        let pages = [LayoutPage {
            y_start: 0.0,
            y_end: 800.0,
            size: Size::new(200.0, 800.0),
        }];
        let pos = editor_state::Position {
            node_id: p1,
            offset: 2,
            affinity: editor_state::Affinity::Downstream,
        };
        let CursorMetrics { caret, line, .. } = cursor_metrics(&tree, &pages, &pos, None).unwrap();
        assert!(caret.y >= 20.0);
        assert_eq!(line.y, 20.0);
        assert_eq!(caret.x, 0.0);
    }

    #[test]
    fn cursor_metrics_page_relative_line_on_second_page() {
        let id = NodeId::new();
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
                        monolithic: false,
                    },
                    children: vec![LayoutNode {
                        rect: Rect::from_xywh(0.0, 500.0, 200.0, 20.0),
                        content: LayoutContent::Line(LayoutLine {
                            node_id: id,
                            baseline: 16.0,
                            ascent: 14.0,
                            descent: 4.0,
                            cursor_ascent: 14.0,
                            cursor_descent: 4.0,
                            glyph_runs: vec![GlyphRun::make_test_run(id, 0, "hello", 0.0, gs(5))],
                            text_indent: 0.0,
                            child_range: None,
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
        let CursorMetrics { page_idx, line, .. } =
            cursor_metrics(&tree, &pages, &pos, None).unwrap();

        assert_eq!(page_idx, 1);
        // line.y = 500 - 400 = 100 (페이지 로컬)
        assert_eq!(line.y, 100.0);
        assert_eq!(line.height, 20.0);
    }
}
