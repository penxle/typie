use editor_common::Rect;
use editor_state::Position;

use crate::page::LayoutPage;
use crate::paginate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct CompositionRect {
    pub page_idx: usize,
    pub rect: Rect,
}

pub fn composition_rects(
    tree: &LayoutTree,
    pages: &[LayoutPage],
    from: &Position,
    to: &Position,
) -> Vec<CompositionRect> {
    if from == to {
        return vec![];
    }

    let mut phase = Phase::Before;
    let mut rects = Vec::new();

    visit_node(&tree.root, from, to, &mut phase, &mut rects, pages);

    rects
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Before,
    Inside,
    After,
}

fn page_for_y(pages: &[LayoutPage], y: f32) -> Option<usize> {
    pages.iter().position(|p| y >= p.y_start && y < p.y_end)
}

fn visit_node(
    node: &LayoutNode,
    from: &Position,
    to: &Position,
    phase: &mut Phase,
    rects: &mut Vec<CompositionRect>,
    pages: &[LayoutPage],
) {
    match &node.content {
        LayoutContent::Box(b) => visit_box(node, b, from, to, phase, rects, pages),
        LayoutContent::Line(l) => {
            visit_line(node, l, from, to, phase, rects, pages);
        }
        LayoutContent::Atom(_) | LayoutContent::Spacing(_) => {}
    }
}

fn visit_line(
    node: &LayoutNode,
    line: &LayoutLine,
    from: &Position,
    to: &Position,
    phase: &mut Phase,
    rects: &mut Vec<CompositionRect>,
    pages: &[LayoutPage],
) {
    let contains_from = line_contains_position(line, from);
    let contains_to = line_contains_position(line, to);

    let (x_start, x_end) = match (*phase, contains_from, contains_to) {
        (Phase::Before, true, true) => {
            *phase = Phase::After;
            (
                super::grapheme::x_at_offset(line, from),
                super::grapheme::x_at_offset(line, to),
            )
        }
        (Phase::Before, true, false) => {
            *phase = Phase::Inside;
            (super::grapheme::x_at_offset(line, from), line_end_x(line))
        }
        (Phase::Inside, false, false) => (line_start_x(line), line_end_x(line)),
        (Phase::Inside, false, true) => {
            *phase = Phase::After;
            (line_start_x(line), super::grapheme::x_at_offset(line, to))
        }
        _ => return,
    };

    let width = x_end - x_start;
    if width <= 0.0 {
        return;
    }

    if let Some(page_idx) = page_for_y(pages, node.rect.y) {
        let underline_y =
            node.rect.y - pages[page_idx].y_start + line.baseline + line.descent * 0.5;

        rects.push(CompositionRect {
            page_idx,
            rect: Rect::from_xywh(node.rect.x + x_start, underline_y, width, 1.0),
        });
    }
}

fn visit_box(
    _node: &LayoutNode,
    bx: &LayoutBox,
    from: &Position,
    to: &Position,
    phase: &mut Phase,
    rects: &mut Vec<CompositionRect>,
    pages: &[LayoutPage],
) {
    for child in &bx.children {
        if *phase == Phase::After {
            break;
        }
        visit_node(child, from, to, phase, rects, pages);
    }
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
    use editor_macros::doc;

    use super::*;
    use crate::view::View;

    fn layout(doc: &editor_model::Doc) -> View {
        let mut view = View::new_test();
        view.layout(doc);
        view
    }

    #[test]
    fn same_position_returns_empty() {
        let (doc, t) = doc! { root { paragraph { t: text("hello") } } };
        let view = layout(&doc);
        let rects = view.composition_rects(&Position::new(t, 2), &Position::new(t, 2));
        assert!(rects.is_empty());
    }

    #[test]
    fn single_line_composition() {
        let (doc, t) = doc! { root { paragraph { t: text("hello") } } };
        let view = layout(&doc);
        let rects = view.composition_rects(&Position::new(t, 1), &Position::new(t, 4));

        assert_eq!(rects.len(), 1);
        assert!(rects[0].rect.width > 0.0);
        assert_eq!(rects[0].rect.height, 1.0);
    }

    #[test]
    fn multi_paragraph_composition() {
        let (doc, t1, t2) = doc! {
            root {
                paragraph { t1: text("hello") }
                paragraph { t2: text("world") }
            }
        };
        let view = layout(&doc);
        let rects = view.composition_rects(&Position::new(t1, 2), &Position::new(t2, 3));

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].rect.height, 1.0);
        assert_eq!(rects[1].rect.height, 1.0);
        assert!(rects[0].rect.y < rects[1].rect.y);
    }

    #[test]
    fn underline_y_below_baseline() {
        let (doc, t) = doc! { root { paragraph { t: text("hello") } } };
        let view = layout(&doc);

        let comp_rects = view.composition_rects(&Position::new(t, 0), &Position::new(t, 5));
        let sel = editor_state::Selection::new(Position::new(t, 0), Position::new(t, 5));
        let resolved = sel.resolve(&doc).unwrap();
        let sel_rects = view.selection_rects(&resolved);

        assert_eq!(comp_rects.len(), 1);
        assert_eq!(sel_rects.len(), 1);
        assert!(comp_rects[0].rect.y > sel_rects[0].rect.y);
    }
}
