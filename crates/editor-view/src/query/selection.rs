use editor_common::{EdgeInsets, Rect};
use editor_state::Position;

use crate::page::{LayoutPage, PageRect};
use crate::paginate::*;

use super::common::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionRectKind {
    Text,
    Atom,
    Block,
}

pub type SelectionRect = PageRect<SelectionRectKind>;

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
    // Resolve which Line/Atom each endpoint belongs to up front so soft-wrap
    // boundary positions are disambiguated by affinity (via `find_line_at`)
    // rather than by the permissive per-line `line_contains_position`.
    let from_owner = super::search::find_line_at(tree, &from);
    let to_owner = super::search::find_line_at(tree, &to);
    let mut phase = Phase::Before;
    let mut rects = Vec::new();

    visit_node(
        &tree.root, &from, &to, from_owner, to_owner, &mut phase, &mut rects, pages,
    );

    rects
}

fn has_visual_boundary(style: &crate::style::BoxStyle) -> bool {
    style.padding != EdgeInsets::ZERO || style.border != EdgeInsets::ZERO
}

fn visit_node(
    node: &LayoutNode,
    from: &Position,
    to: &Position,
    from_owner: Option<&LayoutNode>,
    to_owner: Option<&LayoutNode>,
    phase: &mut Phase,
    rects: &mut Vec<SelectionRect>,
    pages: &[LayoutPage],
) {
    match &node.content {
        LayoutContent::Box(b) => {
            visit_box(node, b, from, to, from_owner, to_owner, phase, rects, pages)
        }
        LayoutContent::Line(l) => {
            visit_line(node, l, from, to, from_owner, to_owner, phase, rects, pages)
        }
        LayoutContent::Atom(a) => visit_atom(node, a, from, to, phase, rects, pages),
        LayoutContent::Spacing(_) => {}
    }
}

fn visit_line(
    node: &LayoutNode,
    line: &LayoutLine,
    from: &Position,
    to: &Position,
    from_owner: Option<&LayoutNode>,
    to_owner: Option<&LayoutNode>,
    phase: &mut Phase,
    rects: &mut Vec<SelectionRect>,
    pages: &[LayoutPage],
) {
    let contains_from = from_owner.map(|n| std::ptr::eq(n, node)).unwrap_or(false);
    let contains_to = to_owner.map(|n| std::ptr::eq(n, node)).unwrap_or(false);

    let (x_start, x_end) = match (*phase, contains_from, contains_to) {
        (Phase::Before, true, true) => {
            let x0 = super::grapheme::x_at_offset(line, from);
            let x1 = super::grapheme::x_at_offset(line, to);
            *phase = Phase::After;
            (x0, x1)
        }
        (Phase::Before, true, false) => {
            let x0 = super::grapheme::x_at_offset(line, from);
            let x1 = line_end_x(line);
            *phase = Phase::Inside;
            (x0, x1)
        }
        (Phase::Inside, false, false) => (line_start_x(line), line_end_x(line)),
        (Phase::Inside, false, true) => {
            let x0 = line_start_x(line);
            let x1 = super::grapheme::x_at_offset(line, to);
            *phase = Phase::After;
            (x0, x1)
        }
        _ => return,
    };

    let width = if x_end > x_start {
        x_end - x_start
    } else if line.glyph_runs.is_empty() {
        // empty line — show a small placeholder like a virtual space
        node.rect.height * 0.3
    } else {
        return;
    };

    if let Some(page_idx) = page_for_y(pages, node.rect.y) {
        rects.push(PageRect::with_meta(
            page_idx,
            Rect::from_xywh(
                node.rect.x + x_start,
                node.rect.y - pages[page_idx].y_start,
                width,
                node.rect.height,
            ),
            SelectionRectKind::Text,
        ));
    }
}

fn visit_atom(
    node: &LayoutNode,
    atom: &LayoutAtom,
    from: &Position,
    to: &Position,
    phase: &mut Phase,
    rects: &mut Vec<SelectionRect>,
    pages: &[LayoutPage],
) {
    let is_from = from.node_id == atom.parent_id && from.offset == atom.index;
    let is_to = to.node_id == atom.parent_id && to.offset == atom.index + 1;

    if *phase == Phase::Before && is_from {
        *phase = Phase::Inside;
    }

    if *phase != Phase::Inside {
        return;
    }

    if let Some(page_idx) = page_for_y(pages, node.rect.y) {
        rects.push(PageRect::with_meta(
            page_idx,
            Rect::from_xywh(
                node.rect.x,
                node.rect.y - pages[page_idx].y_start,
                node.rect.width,
                node.rect.height,
            ),
            SelectionRectKind::Atom,
        ));
    }

    if is_to {
        *phase = Phase::After;
    }
}

fn visit_box(
    node: &LayoutNode,
    bx: &LayoutBox,
    from: &Position,
    to: &Position,
    from_owner: Option<&LayoutNode>,
    to_owner: Option<&LayoutNode>,
    phase: &mut Phase,
    rects: &mut Vec<SelectionRect>,
    pages: &[LayoutPage],
) {
    let from_in_box = from.node_id == bx.node_id;
    let to_in_box = to.node_id == bx.node_id;

    // A container is "fully selected" only when the selection spans across it
    // from an ancestor's perspective — i.e. phase is already Inside on entry
    // and remains Inside after visiting all children. If phase transitions
    // Before→Inside or Inside→After inside this box, then an anchor lives in
    // a descendant and the box itself is only partially covered.
    let entry_phase = *phase;
    let rects_before = rects.len();
    let mut has_content_child = false;
    let mut content_idx = 0usize;

    for child in &bx.children {
        let is_spacing = matches!(child.content, LayoutContent::Spacing(_));

        if !is_spacing && from_in_box && *phase == Phase::Before && content_idx == from.offset {
            *phase = Phase::Inside;
        }

        visit_node(child, from, to, from_owner, to_owner, phase, rects, pages);

        if !is_spacing {
            has_content_child = true;
            content_idx += 1;
            if to_in_box && *phase == Phase::Inside && content_idx == to.offset {
                *phase = Phase::After;
            }
        }
    }

    let fully = has_content_child && entry_phase == Phase::Inside && *phase == Phase::Inside;

    if fully && has_visual_boundary(&bx.style) {
        rects.truncate(rects_before);
        if let Some(page_idx) = page_for_y(pages, node.rect.y) {
            rects.push(PageRect::with_meta(
                page_idx,
                Rect::from_xywh(
                    node.rect.x,
                    node.rect.y - pages[page_idx].y_start,
                    node.rect.width,
                    node.rect.height,
                ),
                SelectionRectKind::Block,
            ));
        }
    }
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
        assert_eq!(rects[0].meta, SelectionRectKind::Text);
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
        assert_eq!(rects[0].meta, SelectionRectKind::Text);
        assert_eq!(rects[1].meta, SelectionRectKind::Text);
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
        assert_eq!(rects[0].meta, SelectionRectKind::Atom);
    }

    #[test]
    fn block_fully_selected_emits_block_rect() {
        let (doc,) = doc! {
            root {
                blockquote(variant: BlockquoteVariant::LeftLine) {
                    paragraph { text("quoted") }
                }
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(
            Position::new(NodeId::ROOT, 0),
            Position::new(NodeId::ROOT, 1),
        );
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].meta, SelectionRectKind::Block);
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
        assert_eq!(rects[0].meta, SelectionRectKind::Text);
    }

    #[test]
    fn fold_title_selection() {
        let (doc, t1) = doc! {
            root {
                paragraph {}
                fold {
                    fold_title {
                        t1: text("hello")
                    }
                    fold_content {
                        paragraph {
                            text("body")
                        }
                    }
                }
                paragraph {}
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(Position::new(t1, 0), Position::new(t1, 5));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].meta, SelectionRectKind::Text);
        assert!(rects[0].rect.width > 0.0);
        assert!(rects[0].rect.height > 0.0);
    }

    #[test]
    fn fold_content_selection() {
        let (doc, t1) = doc! {
            root {
                paragraph {}
                fold {
                    fold_title {
                        text("title")
                    }
                    fold_content {
                        paragraph {
                            t1: text("body!")
                        }
                    }
                }
                paragraph {}
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(Position::new(t1, 5), Position::new(t1, 0));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].meta, SelectionRectKind::Text);
        assert!(rects[0].rect.width > 0.0);
        assert!(rects[0].rect.height > 0.0);
    }

    #[test]
    fn selection_starting_at_lower_soft_wrap_line_emits_rect() {
        // Force soft-wrap: max_width=80 with default margin 20 gives content
        // width 40 — exactly 4 ASCII test chars per line. "abcdefgh" wraps as
        //   line A (offset 0..4): "abcd"
        //   line B (offset 4..8): "efgh"
        // Selecting the lower line's leading character (offset 4 → 5) must
        // produce one rect over the lower line.
        let (doc, t) = doc! {
            root (layout_mode: editor_model::LayoutMode::Continuous { max_width: 80 }) {
                paragraph { t: text("abcdefgh") }
            }
        };
        let view = layout(&doc);

        // Layout dumped via PageVisitor confirms the wrap boundary is at
        // codepoint offset 6: the last visual line owns offsets 6..8 ("gh").
        // Selecting from (t, 6) — the first character of that lower line —
        // exercises the bug: both upper and lower lines claim offset 6 and
        // visit_line consumes it on the upper line with zero width.
        let sel = Selection::new(Position::new(t, 6), Position::new(t, 7));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(
            rects.len(),
            1,
            "expected one rect on the lower wrapped line"
        );
        assert!(rects[0].rect.width > 0.0);
        // The lower visual line is well below the upper ones (y ~ 71).
        assert!(
            rects[0].rect.y > 50.0,
            "rect must be on the lower line, got y={}",
            rects[0].rect.y
        );
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
        assert_eq!(rects[0].meta, SelectionRectKind::Text);
        assert!(rects[0].rect.width > 0.0);
        assert_eq!(rects[1].meta, SelectionRectKind::Text);
        assert!(rects[1].rect.width > 0.0);
    }
}
