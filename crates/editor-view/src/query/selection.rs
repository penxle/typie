use editor_common::{EdgeInsets, Rect};
use editor_state::Position;

use crate::page::LayoutPage;
use crate::paginate::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionRectKind {
    Text,
    Atom,
    Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectionRect {
    pub page_idx: usize,
    pub rect: Rect,
    pub kind: SelectionRectKind,
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Before,
    Inside,
    After,
}

pub fn selection_rects(
    tree: &LayoutTree,
    pages: &[LayoutPage],
    selection: &editor_state::ResolvedSelection<'_>,
) -> Vec<SelectionRect> {
    if selection.is_collapsed() {
        return vec![];
    }

    let from = Position::from(selection.from());
    let to = Position::from(selection.to());
    let mut phase = Phase::Before;
    let mut rects = Vec::new();

    visit_node(&tree.root, &from, &to, &mut phase, &mut rects, pages);

    rects
}

fn page_for_y(pages: &[LayoutPage], y: f32) -> Option<usize> {
    pages.iter().position(|p| y >= p.y_start && y < p.y_end)
}

fn has_visual_boundary(style: &crate::style::BoxStyle) -> bool {
    style.padding != EdgeInsets::ZERO || style.border != EdgeInsets::ZERO
}

fn visit_node(
    node: &LayoutNode,
    from: &Position,
    to: &Position,
    phase: &mut Phase,
    rects: &mut Vec<SelectionRect>,
    pages: &[LayoutPage],
) -> bool {
    match &node.content {
        LayoutContent::Box(b) => visit_box(node, b, from, to, phase, rects, pages),
        LayoutContent::Line(l) => visit_line(node, l, from, to, phase, rects, pages),
        LayoutContent::Atom(a) => visit_atom(node, a, from, to, phase, rects, pages),
        LayoutContent::Spacing(_) => *phase == Phase::Inside,
    }
}

fn visit_line(
    node: &LayoutNode,
    line: &LayoutLine,
    from: &Position,
    to: &Position,
    phase: &mut Phase,
    rects: &mut Vec<SelectionRect>,
    pages: &[LayoutPage],
) -> bool {
    let contains_from = line_contains_position(line, from);
    let contains_to = line_contains_position(line, to);

    let (x_start, x_end, fully_selected) = match (*phase, contains_from, contains_to) {
        (Phase::Before, true, true) => {
            let x0 = super::grapheme::x_at_offset(line, from);
            let x1 = super::grapheme::x_at_offset(line, to);
            let ls = line_start_x(line);
            let le = line_end_x(line);
            let fully = (x0 - ls).abs() < f32::EPSILON && (x1 - le).abs() < f32::EPSILON;
            *phase = Phase::After;
            (x0, x1, fully)
        }
        (Phase::Before, true, false) => {
            let x0 = super::grapheme::x_at_offset(line, from);
            let x1 = line_end_x(line);
            *phase = Phase::Inside;
            (x0, x1, false)
        }
        (Phase::Inside, false, false) => {
            let x0 = line_start_x(line);
            let x1 = line_end_x(line);
            (x0, x1, true)
        }
        (Phase::Inside, false, true) => {
            let x0 = line_start_x(line);
            let x1 = super::grapheme::x_at_offset(line, to);
            *phase = Phase::After;
            (x0, x1, false)
        }
        _ => return false,
    };

    if let Some(page_idx) = page_for_y(pages, node.rect.y) {
        rects.push(SelectionRect {
            page_idx,
            rect: Rect::from_xywh(
                node.rect.x + x_start,
                node.rect.y - pages[page_idx].y_start,
                x_end - x_start,
                node.rect.height,
            ),
            kind: SelectionRectKind::Text,
        });
    }

    fully_selected
}

fn visit_atom(
    node: &LayoutNode,
    atom: &LayoutAtom,
    from: &Position,
    to: &Position,
    phase: &mut Phase,
    rects: &mut Vec<SelectionRect>,
    pages: &[LayoutPage],
) -> bool {
    let is_from = from.node_id == atom.parent_id && from.offset == atom.index;
    let is_to = to.node_id == atom.parent_id && to.offset == atom.index + 1;

    if *phase == Phase::Before && is_from {
        *phase = Phase::Inside;
    }

    if *phase != Phase::Inside {
        return false;
    }

    if let Some(page_idx) = page_for_y(pages, node.rect.y) {
        rects.push(SelectionRect {
            page_idx,
            rect: Rect::from_xywh(
                node.rect.x,
                node.rect.y - pages[page_idx].y_start,
                node.rect.width,
                node.rect.height,
            ),
            kind: SelectionRectKind::Atom,
        });
    }

    if is_to {
        *phase = Phase::After;
    }

    true
}

fn visit_box(
    node: &LayoutNode,
    bx: &LayoutBox,
    from: &Position,
    to: &Position,
    phase: &mut Phase,
    rects: &mut Vec<SelectionRect>,
    pages: &[LayoutPage],
) -> bool {
    let from_in_box = from.node_id == bx.node_id;
    let to_in_box = to.node_id == bx.node_id;

    let rects_before = rects.len();
    let mut all_fully_selected = true;
    let mut has_content_child = false;

    for (i, child) in bx.children.iter().enumerate() {
        if from_in_box && *phase == Phase::Before && i == from.offset {
            *phase = Phase::Inside;
        }

        let child_fully = visit_node(child, from, to, phase, rects, pages);

        let is_spacing = matches!(child.content, LayoutContent::Spacing(_));
        if !is_spacing {
            has_content_child = true;
            if !child_fully {
                all_fully_selected = false;
            }
        }

        if to_in_box && *phase == Phase::Inside && i + 1 == to.offset {
            *phase = Phase::After;
        }
    }

    let fully = has_content_child && all_fully_selected && *phase == Phase::After;

    if fully && has_visual_boundary(&bx.style) {
        rects.truncate(rects_before);
        if let Some(page_idx) = page_for_y(pages, node.rect.y) {
            rects.push(SelectionRect {
                page_idx,
                rect: Rect::from_xywh(
                    node.rect.x,
                    node.rect.y - pages[page_idx].y_start,
                    node.rect.width,
                    node.rect.height,
                ),
                kind: SelectionRectKind::Block,
            });
        }
    }

    fully
}

fn line_contains_position(line: &LayoutLine, pos: &Position) -> bool {
    if line.glyph_runs.is_empty() {
        return line.node_id == pos.node_id && pos.offset == 0;
    }
    for run in &line.glyph_runs {
        if run.node_id == pos.node_id
            && pos.offset >= run.offset
            && pos.offset <= run.offset + super::grapheme::run_codepoint_count(run)
        {
            return true;
        }
    }
    false
}

fn line_start_x(line: &LayoutLine) -> f32 {
    line.glyph_runs
        .first()
        .map(|r| r.x)
        .unwrap_or(line.text_indent)
}

fn line_end_x(line: &LayoutLine) -> f32 {
    line.glyph_runs
        .last()
        .map(|r| r.x + r.width)
        .unwrap_or(line.text_indent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph_run::{GlyphRun, GraphemeSpan};
    use crate::page::LayoutPage;
    use crate::style::*;
    use editor_common::{Alignment, EdgeInsets, Rect, Size};
    use editor_macros::doc;
    use editor_model::NodeId;
    use editor_state::{Position, Selection};

    fn gs(n: usize) -> Vec<GraphemeSpan> {
        vec![
            GraphemeSpan {
                advance: 10.0,
                codepoints: 1
            };
            n
        ]
    }

    fn make_page() -> Vec<LayoutPage> {
        vec![LayoutPage {
            y_start: 0.0,
            y_end: 800.0,
            size: Size::new(200.0, 800.0),
        }]
    }

    fn plain_style() -> BoxStyle {
        BoxStyle {
            direction: Direction::Vertical,
            padding: EdgeInsets::ZERO,
            border: EdgeInsets::ZERO,
            border_mode: BorderMode::Separate,
            alignment: Alignment::Start,
            scope: false,
            decorations: vec![],
        }
    }

    fn styled_box_style() -> BoxStyle {
        BoxStyle {
            padding: EdgeInsets::all(8.0),
            ..plain_style()
        }
    }

    fn make_line(id: NodeId, y: f32, text: &str) -> LayoutNode {
        let n = text.len();
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, n as f32 * 10.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: id,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                glyph_runs: vec![GlyphRun::make_test_run(id, 0, text, 0.0, gs(n))],
                text_indent: 0.0,
            }),
        }
    }

    fn make_box(
        id: NodeId,
        y: f32,
        h: f32,
        style: BoxStyle,
        children: Vec<LayoutNode>,
    ) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, h),
            content: LayoutContent::Box(LayoutBox {
                node_id: id,
                style,
                children,
            }),
        }
    }

    #[test]
    fn collapsed_selection_returns_empty() {
        let (doc, t) = doc! { root { paragraph { t: text("hello") } } };
        let tree = LayoutTree {
            root: make_box(
                NodeId::ROOT,
                0.0,
                20.0,
                plain_style(),
                vec![make_box(
                    NodeId::new(),
                    0.0,
                    20.0,
                    plain_style(),
                    vec![make_line(t, 0.0, "hello")],
                )],
            ),
        };
        let pages = make_page();
        let sel = Selection::collapsed(Position::new(t, 2));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = selection_rects(&tree, &pages, &resolved);
        assert!(rects.is_empty());
    }

    #[test]
    fn single_line_partial_selection() {
        let (doc, t) = doc! { root { paragraph { t: text("hello") } } };
        let tree = LayoutTree {
            root: make_box(
                NodeId::ROOT,
                0.0,
                20.0,
                plain_style(),
                vec![make_box(
                    NodeId::new(),
                    0.0,
                    20.0,
                    plain_style(),
                    vec![make_line(t, 0.0, "hello")],
                )],
            ),
        };
        let pages = make_page();
        let sel = Selection::new(Position::new(t, 1), Position::new(t, 4));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = selection_rects(&tree, &pages, &resolved);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].kind, SelectionRectKind::Text);
        assert_eq!(rects[0].rect.x, 10.0);
        assert_eq!(rects[0].rect.width, 30.0);
        assert_eq!(rects[0].rect.height, 20.0);
    }

    #[test]
    fn multi_line_selection() {
        let (doc, t1, t2) = doc! {
            root {
                paragraph { t1: text("hello") }
                paragraph { t2: text("world") }
            }
        };
        let tree = LayoutTree {
            root: make_box(
                NodeId::ROOT,
                0.0,
                40.0,
                plain_style(),
                vec![
                    make_box(
                        NodeId::new(),
                        0.0,
                        20.0,
                        plain_style(),
                        vec![make_line(t1, 0.0, "hello")],
                    ),
                    make_box(
                        NodeId::new(),
                        20.0,
                        20.0,
                        plain_style(),
                        vec![make_line(t2, 20.0, "world")],
                    ),
                ],
            ),
        };
        let pages = make_page();
        let sel = Selection::new(Position::new(t1, 2), Position::new(t2, 3));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = selection_rects(&tree, &pages, &resolved);
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].kind, SelectionRectKind::Text);
        assert_eq!(rects[0].rect.x, 20.0);
        assert_eq!(rects[0].rect.width, 30.0);
        assert_eq!(rects[1].kind, SelectionRectKind::Text);
        assert_eq!(rects[1].rect.x, 0.0);
        assert_eq!(rects[1].rect.width, 30.0);
    }

    #[test]
    fn atom_selection() {
        let (doc, _t, img) = doc! {
            root {
                paragraph { _t: text("a") }
                img: image {}
            }
        };
        let tree = LayoutTree {
            root: make_box(
                NodeId::ROOT,
                0.0,
                120.0,
                plain_style(),
                vec![
                    make_box(
                        NodeId::new(),
                        0.0,
                        20.0,
                        plain_style(),
                        vec![make_line(_t, 0.0, "a")],
                    ),
                    LayoutNode {
                        rect: Rect::from_xywh(0.0, 20.0, 100.0, 100.0),
                        content: LayoutContent::Atom(LayoutAtom {
                            node_id: img,
                            parent_id: NodeId::ROOT,
                            index: 1,
                        }),
                    },
                ],
            ),
        };
        let pages = make_page();
        let sel = Selection::new(
            Position::new(NodeId::ROOT, 1),
            Position::new(NodeId::ROOT, 2),
        );
        let resolved = sel.resolve(&doc).unwrap();
        let rects = selection_rects(&tree, &pages, &resolved);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].kind, SelectionRectKind::Atom);
        assert_eq!(rects[0].rect.x, 0.0);
        assert_eq!(rects[0].rect.y, 20.0);
        assert_eq!(rects[0].rect.width, 100.0);
        assert_eq!(rects[0].rect.height, 100.0);
    }

    #[test]
    fn block_fully_selected_emits_block_rect() {
        let (doc, t) = doc! {
            root { blockquote { paragraph { t: text("quoted") } } }
        };
        let tree = LayoutTree {
            root: make_box(
                NodeId::ROOT,
                0.0,
                36.0,
                plain_style(),
                vec![make_box(
                    NodeId::new(),
                    0.0,
                    36.0,
                    styled_box_style(),
                    vec![make_line(t, 8.0, "quoted")],
                )],
            ),
        };
        let pages = make_page();
        let sel = Selection::new(Position::new(t, 0), Position::new(t, 6));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = selection_rects(&tree, &pages, &resolved);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].kind, SelectionRectKind::Block);
        assert_eq!(rects[0].rect.height, 36.0);
    }

    #[test]
    fn block_partially_selected_emits_text_rects() {
        let (doc, t) = doc! {
            root { blockquote { paragraph { t: text("quoted") } } }
        };
        let tree = LayoutTree {
            root: make_box(
                NodeId::ROOT,
                0.0,
                36.0,
                plain_style(),
                vec![make_box(
                    NodeId::new(),
                    0.0,
                    36.0,
                    styled_box_style(),
                    vec![make_line(t, 8.0, "quoted")],
                )],
            ),
        };
        let pages = make_page();
        let sel = Selection::new(Position::new(t, 1), Position::new(t, 4));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = selection_rects(&tree, &pages, &resolved);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].kind, SelectionRectKind::Text);
    }
}
