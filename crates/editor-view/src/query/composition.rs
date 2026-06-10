use editor_common::Rect;
use editor_state::Position;

use crate::page::{LayoutPage, PageRect};
use crate::paginate::*;

use super::common::*;
use super::layout_index::{LayoutEntry, LayoutIndex};

pub type CompositionRect = PageRect;

pub fn composition_rects(
    layout_index: &LayoutIndex,
    from: &Position,
    to: &Position,
) -> Vec<CompositionRect> {
    if from == to {
        return vec![];
    }

    let from_owner = layout_index.entry_for_position(from);
    let to_owner = layout_index.entry_for_position(to);
    let mut phase = Phase::Before;
    let mut rects = Vec::new();

    visit_node(
        &layout_index.tree().root,
        layout_index,
        from,
        to,
        from_owner,
        to_owner,
        &mut phase,
        &mut rects,
        layout_index.pages(),
    );

    rects
}

fn visit_node(
    node: &LayoutNode,
    layout_index: &LayoutIndex,
    from: &Position,
    to: &Position,
    from_owner: Option<&LayoutEntry>,
    to_owner: Option<&LayoutEntry>,
    phase: &mut Phase,
    rects: &mut Vec<CompositionRect>,
    pages: &[LayoutPage],
) {
    match &node.content {
        LayoutContent::Box(b) => visit_box(
            node,
            b,
            layout_index,
            from,
            to,
            from_owner,
            to_owner,
            phase,
            rects,
            pages,
        ),
        LayoutContent::Line(l) => {
            visit_line(
                node,
                l,
                layout_index,
                from,
                to,
                from_owner,
                to_owner,
                phase,
                rects,
                pages,
            );
        }
        LayoutContent::Atom(_) | LayoutContent::Spacing(_) => {}
    }
}

fn visit_line(
    node: &LayoutNode,
    line: &LayoutLine,
    layout_index: &LayoutIndex,
    from: &Position,
    to: &Position,
    from_owner: Option<&LayoutEntry>,
    to_owner: Option<&LayoutEntry>,
    phase: &mut Phase,
    rects: &mut Vec<CompositionRect>,
    pages: &[LayoutPage],
) {
    let contains_from = from_owner.is_some_and(|entry| entry.is_node(layout_index, node));
    let contains_to = to_owner.is_some_and(|entry| entry.is_node(layout_index, node));

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

        rects.push(PageRect::new(
            page_idx,
            Rect::from_xywh(node.rect.x + x_start, underline_y, width, 1.0),
        ));
    }
}

fn visit_box(
    _node: &LayoutNode,
    bx: &LayoutBox,
    layout_index: &LayoutIndex,
    from: &Position,
    to: &Position,
    from_owner: Option<&LayoutEntry>,
    to_owner: Option<&LayoutEntry>,
    phase: &mut Phase,
    rects: &mut Vec<CompositionRect>,
    pages: &[LayoutPage],
) {
    for child in &bx.children {
        if *phase == Phase::After {
            break;
        }
        visit_node(
            child,
            layout_index,
            from,
            to,
            from_owner,
            to_owner,
            phase,
            rects,
            pages,
        );
    }
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
    fn composition_starting_at_lower_soft_wrap_line_emits_rect() {
        // Same soft-wrap setup as selection.rs's regression test: text wraps
        // such that offset 6 is the lower visual line's leading boundary.
        let (doc, t) = doc! {
            root (layout_mode: editor_model::LayoutMode::Continuous { max_width: 40 }) {
                paragraph { t: text("abcdefgh") }
            }
        };
        let view = layout(&doc);
        let rects = view.composition_rects(&Position::new(t, 6), &Position::new(t, 7));

        assert_eq!(rects.len(), 1);
        assert!(rects[0].rect.width > 0.0);
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
