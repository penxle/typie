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

    let width = if x_end > x_start {
        x_end - x_start
    } else if line.glyph_runs.is_empty() {
        // empty line — show a small placeholder like a virtual space
        node.rect.height * 0.3
    } else {
        return fully_selected;
    };

    if let Some(page_idx) = page_for_y(pages, node.rect.y) {
        rects.push(SelectionRect {
            page_idx,
            rect: Rect::from_xywh(
                node.rect.x + x_start,
                node.rect.y - pages[page_idx].y_start,
                width,
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
    let mut content_idx = 0usize;

    for child in &bx.children {
        let is_spacing = matches!(child.content, LayoutContent::Spacing(_));

        if !is_spacing {
            if from_in_box && *phase == Phase::Before && content_idx == from.offset {
                *phase = Phase::Inside;
            }
        }

        let child_fully = visit_node(child, from, to, phase, rects, pages);

        if !is_spacing {
            has_content_child = true;
            if !child_fully {
                all_fully_selected = false;
            }
            content_idx += 1;
            if to_in_box && *phase == Phase::Inside && content_idx == to.offset {
                *phase = Phase::After;
            }
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
    use editor_macros::doc;
    use editor_model::NodeId;
    use editor_state::{Position, Selection};

    use super::*;
    use crate::view::View;

    fn layout(doc: &editor_model::Doc) -> View {
        let mut view = View::new_test();
        view.layout(doc);
        view
    }

    #[test]
    fn collapsed_selection_returns_empty() {
        let (doc, t) = doc! { root { paragraph { t: text("hello") } } };
        let view = layout(&doc);

        let sel = Selection::collapsed(Position::new(t, 2));
        let resolved = sel.resolve(&doc).unwrap();
        assert!(view.selection_rects(&resolved).is_empty());
    }

    #[test]
    fn single_line_partial_selection() {
        let (doc, t) = doc! { root { paragraph { t: text("hello") } } };
        let view = layout(&doc);

        let sel = Selection::new(Position::new(t, 1), Position::new(t, 4));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].kind, SelectionRectKind::Text);
        assert!(rects[0].rect.width > 0.0);
        assert!(rects[0].rect.height > 0.0);
    }

    #[test]
    fn multi_line_selection() {
        let (doc, t1, t2) = doc! {
            root {
                paragraph { t1: text("hello") }
                paragraph { t2: text("world") }
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(Position::new(t1, 2), Position::new(t2, 3));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].kind, SelectionRectKind::Text);
        assert_eq!(rects[1].kind, SelectionRectKind::Text);
        assert!(rects[0].rect.y < rects[1].rect.y);
    }

    #[test]
    fn atom_selection() {
        let (doc,) = doc! {
            root {
                paragraph { text("a") }
                horizontal_rule {}
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(
            Position::new(NodeId::ROOT, 1),
            Position::new(NodeId::ROOT, 2),
        );
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].kind, SelectionRectKind::Atom);
    }

    #[test]
    fn block_fully_selected_emits_block_rect() {
        let (doc, t) = doc! {
            root {
                blockquote(variant: BlockquoteVariant::LeftLine) {
                    paragraph { t: text("quoted") }
                }
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(Position::new(t, 0), Position::new(t, 6));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].kind, SelectionRectKind::Block);
    }

    #[test]
    fn block_partially_selected_emits_text_rects() {
        let (doc, t) = doc! {
            root {
                blockquote(variant: BlockquoteVariant::LeftLine) {
                    paragraph { t: text("quoted") }
                }
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(Position::new(t, 1), Position::new(t, 4));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].kind, SelectionRectKind::Text);
    }

    #[test]
    fn cross_block_selection() {
        let (doc, t1, p1) = doc! {
            root {
                callout(variant: CalloutVariant::Danger) {
                    paragraph { text("A") }
                    paragraph { text("Hello, World!") }
                    paragraph { t1: text("end") }
                }
                p1: paragraph {}
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(Position::new(t1, 0), Position::new(p1, 0));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].kind, SelectionRectKind::Text);
        assert!(rects[0].rect.width > 0.0);
        assert_eq!(rects[1].kind, SelectionRectKind::Text);
        assert!(rects[1].rect.width > 0.0);
    }
}
