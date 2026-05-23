use editor_common::Rect;
use editor_macros::ffi;
use editor_state::{Affinity, Position};
use serde::{Deserialize, Serialize};

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

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SelectionEndpoints {
    pub from: PageRect,
    pub to: PageRect,
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
    // Resolve which Line/Atom each endpoint belongs to up front so soft-wrap
    // boundary positions are disambiguated by affinity (via `find_line_at`)
    // rather than by the permissive per-line `line_contains_position`.
    //
    // `find_line_at` is permissive on purpose for navigation: a monolithic box
    // owns both of its bracket positions, and an atom owns both of its edges.
    // For rect attribution those permissive matches are wrong when the endpoint
    // is semantically a container boundary — the box-level phase machine (and
    // the `fully && monolithic` branch) must fire. Strip an owner whose
    // attachment to the endpoint is not the affinity-selected one.
    let from_owner = super::search::find_line_at(tree, &from).filter(|n| attached(n, &from));
    let to_owner = super::search::find_line_at(tree, &to).filter(|n| attached(n, &to));
    let mut phase = Phase::Before;
    let mut rects = Vec::new();

    visit_node(
        &tree.root, &from, &to, from_owner, to_owner, &mut phase, &mut rects, pages,
    );

    rects
}

pub fn selection_endpoints(
    tree: &LayoutTree,
    pages: &[LayoutPage],
    selection: &editor_state::ResolvedSelection<'_>,
) -> Option<SelectionEndpoints> {
    if selection.is_collapsed() {
        return None;
    }
    let rects = selection_rects(tree, pages, selection);
    let first = rects.first()?;
    let last = rects.last()?;
    Some(SelectionEndpoints {
        from: PageRect::new(
            first.page_idx,
            Rect::from_xywh(first.rect.x, first.rect.y, 0.0, first.rect.height),
        ),
        to: PageRect::new(
            last.page_idx,
            Rect::from_xywh(
                last.rect.x + last.rect.width,
                last.rect.y,
                0.0,
                last.rect.height,
            ),
        ),
    })
}

pub fn selection_hit_test(
    tree: &LayoutTree,
    pages: &[LayoutPage],
    selection: &editor_state::ResolvedSelection<'_>,
    page_idx: usize,
    x: f32,
    y: f32,
) -> bool {
    if selection.is_collapsed() {
        return false;
    }
    let rects: Vec<Rect> = selection_rects(tree, pages, selection)
        .into_iter()
        .filter(|r| r.page_idx == page_idx)
        .map(|r| r.rect)
        .collect();
    if rects.is_empty() {
        return false;
    }

    let min_x = rects.iter().map(|r| r.x).fold(f32::INFINITY, f32::min);
    let max_x = rects
        .iter()
        .map(|r| r.x + r.width)
        .fold(f32::NEG_INFINITY, f32::max);
    let last_idx = rects.len() - 1;

    for (i, rect) in rects.iter().enumerate() {
        let (x_lo, x_hi) = if last_idx == 0 {
            (rect.x, rect.x + rect.width)
        } else if i == 0 {
            (rect.x, max_x)
        } else if i == last_idx {
            (min_x, rect.x + rect.width)
        } else {
            (min_x, max_x)
        };
        if x >= x_lo && x <= x_hi && y >= rect.y && y <= rect.y + rect.height {
            return true;
        }
    }

    for pair in rects.windows(2) {
        let gap_top = pair[0].y + pair[0].height;
        let gap_bottom = pair[1].y;
        if gap_top < gap_bottom && x >= min_x && x <= max_x && y >= gap_top && y <= gap_bottom {
            return true;
        }
    }

    false
}

// Whether `node` (as returned by `find_line_at`) should be treated as the
// owner of `pos` for selection-rect attribution. The two carve-outs encode the
// same idea — the endpoint sits on a structural container boundary, not in
// the interior of a Line/Atom — but at different node kinds:
//
// - A monolithic-bracket Box position is a container boundary that the
//   `fully && monolithic` branch (and any ancestor box machine) must claim.
// - An atom edge owns the position only when affinity attaches the position to
//   the atom: leading edge with Downstream, or trailing edge with Upstream.
//   The other two cases are container boundaries — the box machine claims them
//   via its own phase transitions.
fn attached(node: &LayoutNode, pos: &Position) -> bool {
    match &node.content {
        LayoutContent::Box(b) => b.nav.is_none(),
        LayoutContent::Atom(a) => {
            let leading = pos.offset == a.index && pos.affinity == Affinity::Downstream;
            let trailing = pos.offset == a.index + 1 && pos.affinity == Affinity::Upstream;
            leading || trailing
        }
        _ => true,
    }
}

/// True when `child_range` owns more slots than the number of distinct text
/// children — the surplus is non-text content (in practice, a trailing
/// `hard_break`).
fn line_has_trailing_non_text(line: &LayoutLine) -> bool {
    let Some(range) = &line.child_range else {
        return false;
    };
    if line.glyph_runs.is_empty() {
        return false;
    }
    let mut text_child_count = 1usize;
    let mut prev = line.glyph_runs[0].node_id;
    for run in line.glyph_runs.iter().skip(1) {
        if run.node_id != prev {
            text_child_count += 1;
            prev = run.node_id;
        }
    }
    range.end.saturating_sub(range.start) > text_child_count
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

    let hb_placeholder = node.rect.height * 0.15;
    // Without an explicit extension, a trailing hard_break enveloped by the
    // selection reads as zero width because `x_at_offset` pins to the last
    // run's right edge.
    let trailing_hb = line_has_trailing_non_text(line);
    let at_line_trailing = |pos: &Position| -> bool {
        line.child_range
            .as_ref()
            .is_some_and(|r| pos.node_id == line.node_id && pos.offset == r.end && r.end > r.start)
    };

    let (x_start, x_end) = match (*phase, contains_from, contains_to) {
        (Phase::Before, true, true) => {
            let x0 = super::grapheme::x_at_offset(line, from);
            let mut x1 = super::grapheme::x_at_offset(line, to);
            if trailing_hb && at_line_trailing(to) {
                x1 += hb_placeholder;
            }
            *phase = Phase::After;
            (x0, x1)
        }
        (Phase::Before, true, false) => {
            let x0 = super::grapheme::x_at_offset(line, from);
            let mut x1 = line_end_x(line);
            if trailing_hb {
                x1 += hb_placeholder;
            }
            *phase = Phase::Inside;
            (x0, x1)
        }
        (Phase::Inside, false, false) => {
            let x0 = line_start_x(line);
            let mut x1 = line_end_x(line);
            if trailing_hb {
                x1 += hb_placeholder;
            }
            (x0, x1)
        }
        (Phase::Inside, false, true) => {
            let x0 = line_start_x(line);
            let mut x1 = super::grapheme::x_at_offset(line, to);
            if trailing_hb && at_line_trailing(to) {
                x1 += hb_placeholder;
            }
            *phase = Phase::After;
            (x0, x1)
        }
        _ => return,
    };

    // Both endpoints at paragraph-level offsets in the same line collapse
    // onto the same x via `x_at_offset` — show a placeholder so the
    // hard_break still reads as selected.
    let spans_hard_break =
        from.node_id == line.node_id && to.node_id == line.node_id && from.offset != to.offset;
    let width = if x_end > x_start {
        x_end - x_start
    } else if line.glyph_runs.is_empty() || spans_hard_break {
        hb_placeholder
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
    // Box-level phase transitions fire only for endpoints anchored at a
    // structural container boundary — positions like `(bullet_list, 0)` that
    // no line owns. When a descendant line owns the endpoint, visit_line is
    // responsible for the transition; firing here too would double-count and
    // break collapsed selections that sit on the same physical boundary
    // expressed at different container levels (e.g. end of text `t` vs.
    // offset 0 of the following sibling container).
    let from_at_box_level = from.node_id == bx.node_id && from_owner.is_none();
    let to_at_box_level = to.node_id == bx.node_id && to_owner.is_none();

    // `entry_phase` is captured before any of this box's own boundary
    // transitions fire. A container is "fully selected" only when phase was
    // Inside on entry and remains Inside on exit — that is, the selection
    // envelopes the box from an ancestor's perspective rather than anchoring
    // inside it.
    let entry_phase = *phase;
    let rects_before = rects.len();
    let mut has_content_child = false;
    let mut content_idx = 0usize;

    // Position 0 (box entry). Handles `from.offset == 0` and `to.offset == 0`.
    if from_at_box_level && *phase == Phase::Before && from.offset == 0 {
        *phase = Phase::Inside;
    }
    if to_at_box_level && *phase == Phase::Inside && to.offset == 0 {
        *phase = Phase::After;
    }

    for child in &bx.children {
        let is_spacing = matches!(child.content, LayoutContent::Spacing(_));

        visit_node(child, from, to, from_owner, to_owner, phase, rects, pages);

        if !is_spacing {
            has_content_child = true;
            content_idx += 1;
            // Position `content_idx` — the gap between this child and the next.
            if from_at_box_level && *phase == Phase::Before && content_idx == from.offset {
                *phase = Phase::Inside;
            }
            if to_at_box_level && *phase == Phase::Inside && content_idx == to.offset {
                *phase = Phase::After;
            }
        }
    }

    let fully = has_content_child && entry_phase == Phase::Inside && *phase == Phase::Inside;

    if fully && bx.style.monolithic {
        rects.truncate(rects_before);
        let node_top = node.rect.y;
        let node_bottom = node_top + node.rect.height;
        for (page_idx, page) in pages.iter().enumerate() {
            if node_bottom <= page.y_start || node_top >= page.y_end {
                continue;
            }
            let top = node_top.max(page.y_start);
            let bottom = node_bottom.min(page.y_end);
            rects.push(PageRect::with_meta(
                page_idx,
                Rect::from_xywh(
                    node.rect.x,
                    top - page.y_start,
                    node.rect.width,
                    bottom - top,
                ),
                SelectionRectKind::Block,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::{doc, state};
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
    fn fully_selected_box_spanning_pages_emits_rect_per_page() {
        // Fully select a fold whose FoldContent spans two pages. FoldContent
        // has padding > 0, so visit_box's "fully selected" branch fires for it.
        // The selection rect must appear on every page the box overlaps; if it
        // is anchored to a single page only, the renderer's per-page mark
        // filtering hides the selection on the other pages.
        let (doc,) = doc! {
            root (
                layout_mode: editor_model::LayoutMode::Paginated {
                    page_width: 400,
                    page_height: 120,
                    page_margin_top: 10,
                    page_margin_bottom: 10,
                    page_margin_left: 10,
                    page_margin_right: 10,
                }
            ) {
                fold {
                    fold_title { text("title") }
                    fold_content {
                        paragraph { text("a") }
                        paragraph { text("b") }
                        paragraph { text("c") }
                        paragraph { text("d") }
                        paragraph { text("e") }
                        paragraph { text("f") }
                        paragraph { text("g") }
                        paragraph { text("h") }
                    }
                }
            }
        };
        let view = layout(&doc);
        assert!(
            view.pages().len() >= 2,
            "expected >= 2 pages, got {}",
            view.pages().len()
        );

        let sel = Selection::new(
            Position::new(NodeId::ROOT, 0),
            Position::new(NodeId::ROOT, 1),
        );
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        let pages: std::collections::HashSet<_> = rects.iter().map(|r| r.page_idx).collect();
        assert!(
            pages.len() >= 2,
            "fully-selected box spanning pages must emit rects on every overlapped page; pages={:?}, rects={:?}",
            pages,
            rects,
        );
    }

    #[test]
    fn nested_bullet_lists_with_external_endpoint_emit_text_rects() {
        // Selection runs from inside the outer list_item's paragraph down through
        // nested list_items and ends at the start of a sibling paragraph outside
        // the bullet_list. The middle and innermost list_items are enveloped by
        // the selection (entry_phase=Inside on entry, phase=Inside on exit). Their
        // padding.left is structural — it reserves the bullet marker slot, not a
        // visual envelope. Selection rendering must NOT collapse them into block
        // rects covering the marker slot. Each line emits its own text rect.
        let (doc, t1, p1) = doc! {
            root {
                bullet_list {
                    list_item {
                        paragraph {
                            t1: text("a")
                        }
                        bullet_list {
                            list_item {
                                paragraph {
                                    text("b")
                                }
                                bullet_list {
                                    list_item {
                                        paragraph {
                                            text("c")
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                p1: paragraph {}
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(Position::new(t1, 0), Position::new(p1, 0));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(
            rects.len(),
            4,
            "expected text rects for a/b/c plus an empty-line placeholder for p1, got {:?}",
            rects,
        );
        assert!(
            rects.iter().all(|r| r.meta == SelectionRectKind::Text),
            "list_item's structural padding must not promote rects to Block kind: {:?}",
            rects,
        );
    }

    #[test]
    fn selection_ending_at_offset_zero_of_container_does_not_leak_to_following_siblings() {
        // `to` lands at offset 0 of a container box (`bl1`) — i.e. immediately
        // before its first child. The selection's `from` sits at the end of `t1`,
        // which is the same physical boundary at a different level. Nothing
        // should be drawn: neither inside `bl1` nor in the bottom paragraph
        // that follows the outer bullet_list.
        let (state, ..) = state! {
            doc {
                root {
                    bullet_list {
                        list_item {
                            paragraph {
                                t1: text("a")
                            }
                            bl1: bullet_list {
                                list_item {
                                    paragraph {
                                        text("b")
                                    }
                                }
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 1, <) -> (bl1, 0, >)
        };
        let view = layout(&state.doc);
        let resolved = state.selection.unwrap().resolve(&state.doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(
            rects.len(),
            0,
            "selection collapsed at the boundary just outside t1 / just inside bl1 must emit no rects, got {:?}",
            rects,
        );
    }

    #[test]
    fn fully_enveloped_fold_content_emits_text_not_block() {
        // (fold,1)->(fold,2) envelopes the fold_content subtree from the
        // fold's perspective (fold_title excluded, so Fold itself is not
        // fully selected). FoldContent is not monolithic (only Fold is), so a
        // fully-enveloped fold_content must emit text rects, not one Block rect.
        let (doc,) = doc! {
            root {
                fold {
                    fold_title { text("t") }
                    fold_content { paragraph { text("body") } }
                }
            }
        };
        let f = doc
            .node(NodeId::ROOT)
            .unwrap()
            .children()
            .next()
            .unwrap()
            .id();
        let view = layout(&doc);
        let sel = Selection::new(Position::new(f, 1), Position::new(f, 2));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);
        assert!(!rects.is_empty());
        assert!(
            rects.iter().all(|r| r.meta == SelectionRectKind::Text),
            "fold_content is not monolithic; must emit text rects, got {:?}",
            rects
        );
    }

    #[test]
    fn fully_enveloped_callout_emits_block_rect() {
        let (doc,) = doc! {
            root {
                callout(variant: CalloutVariant::Danger) {
                    paragraph { text("hi") }
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
    fn fully_enveloped_table_emits_block_rect() {
        let (doc,) = doc! {
            root {
                table {
                    table_row {
                        table_cell { paragraph { text("a") } }
                    }
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
        assert!(
            rects.iter().any(|r| r.meta == SelectionRectKind::Block),
            "fully-enveloped monolithic Table must emit a Block rect, got {:?}",
            rects
        );
    }

    #[test]
    fn empty_paragraph_after_atom_node_selected_emits_placeholder_rect() {
        // Node-selecting the empty paragraph that follows an image atom.
        // (r1, 1, >) means Downstream-attached-to-child[1] (the paragraph),
        // (r1, 2, <) means Upstream-attached-to-child[1] (the paragraph);
        // both endpoints bracket the empty paragraph from the root level.
        // The empty-line placeholder rect inside that paragraph must render.
        let (state, ..) = state! {
            doc {
                r1: root {
                    image
                    paragraph {}
                }
            }
            selection: (r1, 1, >) -> (r1, 2, <)
        };
        let view = layout(&state.doc);
        let resolved = state.selection.unwrap().resolve(&state.doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 1, "got {:?}", rects);
        assert_eq!(rects[0].meta, SelectionRectKind::Text);
        assert!(rects[0].rect.width > 0.0);
        assert!(rects[0].rect.height > 0.0);
    }

    #[test]
    fn paragraph_after_atom_node_selected_emits_text_rect() {
        // Non-empty counterpart of the previous test. The same affinity-bracket
        // pattern around child[1] must also paint the line for a populated
        // paragraph — proving the fix is not limited to the empty-line branch.
        let (state, ..) = state! {
            doc {
                r1: root {
                    image
                    paragraph { text("hello") }
                }
            }
            selection: (r1, 1, >) -> (r1, 2, <)
        };
        let view = layout(&state.doc);
        let resolved = state.selection.unwrap().resolve(&state.doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 1, "got {:?}", rects);
        assert_eq!(rects[0].meta, SelectionRectKind::Text);
        assert!(rects[0].rect.width > 0.0);
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

    #[test]
    fn selection_endpoints_collapsed_returns_none() {
        let (doc, t) = doc! { root { paragraph { t: text("hello") } } };
        let view = layout(&doc);

        let sel = Selection::collapsed(Position::new(t, 2));
        let resolved = sel.resolve(&doc).unwrap();
        let layout_tree = view.layout_tree_for_test().unwrap();
        let pages = view.pages();
        assert!(selection_endpoints(layout_tree, pages, &resolved).is_none());
    }

    #[test]
    fn selection_hit_test_collapsed_is_false() {
        let (doc, t) = doc! { root { paragraph { t: text("hello") } } };
        let view = layout(&doc);
        let resolved = Selection::collapsed(Position::new(t, 2))
            .resolve(&doc)
            .unwrap();
        let tree = view.layout_tree_for_test().unwrap();
        let pages = view.pages();
        assert!(!selection_hit_test(tree, pages, &resolved, 0, 5.0, 5.0));
    }

    #[test]
    fn selection_hit_test_single_line_inside_rect_is_true() {
        let (doc, t) = doc! { root { paragraph { t: text("hello world") } } };
        let view = layout(&doc);
        let resolved = Selection::new(Position::new(t, 0), Position::new(t, 5))
            .resolve(&doc)
            .unwrap();
        let tree = view.layout_tree_for_test().unwrap();
        let pages = view.pages();
        let rect = view.selection_rects(&resolved)[0].rect;

        assert!(selection_hit_test(
            tree,
            pages,
            &resolved,
            0,
            rect.x + rect.width * 0.5,
            rect.y + rect.height * 0.5,
        ));
    }

    #[test]
    fn selection_hit_test_single_line_outside_rect_is_false() {
        let (doc, t) = doc! { root { paragraph { t: text("hello world") } } };
        let view = layout(&doc);
        let resolved = Selection::new(Position::new(t, 0), Position::new(t, 5))
            .resolve(&doc)
            .unwrap();
        let tree = view.layout_tree_for_test().unwrap();
        let pages = view.pages();
        let rect = view.selection_rects(&resolved)[0].rect;

        assert!(!selection_hit_test(
            tree,
            pages,
            &resolved,
            0,
            rect.x + rect.width + 5.0,
            rect.y + rect.height * 0.5,
        ));
    }

    #[test]
    fn selection_hit_test_multi_line_extends_first_rect_to_max_x() {
        let (doc, t1, t2) = doc! {
            root {
                paragraph { t1: text("hi") }
                paragraph { t2: text("a much longer line") }
            }
        };
        let view = layout(&doc);
        let resolved = Selection::new(Position::new(t1, 0), Position::new(t2, 18))
            .resolve(&doc)
            .unwrap();
        let tree = view.layout_tree_for_test().unwrap();
        let pages = view.pages();
        let rects = view.selection_rects(&resolved);
        let first = rects[0].rect;
        let last = rects[1].rect;
        let max_x = last.x + last.width;

        assert!(max_x > first.x + first.width, "second line must be wider");

        let probe_x = first.x + first.width + (max_x - (first.x + first.width)) * 0.5;
        let probe_y = first.y + first.height * 0.5;
        assert!(selection_hit_test(
            tree, pages, &resolved, 0, probe_x, probe_y
        ));
    }

    #[test]
    fn selection_hit_test_multi_line_extends_last_rect_to_min_x() {
        // callout's measured padding.left = CALLOUT_PADDING_X + CALLOUT_ICON_WIDTH
        // + CALLOUT_ICON_CONTENT_GAP = 40, so the inner paragraph's line starts
        // further right than the outer paragraph. The partial range (t1..t2) keeps
        // the callout from collapsing into a Block rect, so a Text rect is emitted
        // for the inner line — that's the last rect whose left edge envelope
        // extension we want to exercise.
        let (doc, t1, t2) = doc! {
            root {
                paragraph { t1: text("hi") }
                callout(variant: CalloutVariant::Danger) {
                    paragraph { t2: text("inside callout") }
                }
            }
        };
        let view = layout(&doc);
        let resolved = Selection::new(Position::new(t1, 0), Position::new(t2, 5))
            .resolve(&doc)
            .unwrap();
        let tree = view.layout_tree_for_test().unwrap();
        let pages = view.pages();
        let rects = view.selection_rects(&resolved);
        let first = rects[0].rect;
        let last = rects.last().unwrap().rect;
        let min_x = first.x.min(last.x);

        assert!(
            last.x > min_x,
            "last line must start further right than envelope min_x",
        );

        let probe_x = min_x + (last.x - min_x) * 0.5;
        let probe_y = last.y + last.height * 0.5;
        assert!(selection_hit_test(
            tree, pages, &resolved, 0, probe_x, probe_y
        ));
    }

    #[test]
    fn selection_hit_test_vertical_gap_inside_envelope_is_true() {
        let (doc, t1, t2) = doc! {
            root {
                blockquote(variant: BlockquoteVariant::LeftLine) {
                    paragraph { t1: text("first") }
                }
                blockquote(variant: BlockquoteVariant::LeftLine) {
                    paragraph { t2: text("second") }
                }
            }
        };
        let view = layout(&doc);
        let resolved = Selection::new(Position::new(t1, 0), Position::new(t2, 6))
            .resolve(&doc)
            .unwrap();
        let tree = view.layout_tree_for_test().unwrap();
        let pages = view.pages();
        let rects: Vec<_> = view
            .selection_rects(&resolved)
            .into_iter()
            .filter(|r| r.page_idx == 0)
            .collect();
        assert!(rects.len() >= 2, "expected rects spanning two blocks");
        let a = rects[0].rect;
        let b = rects[1].rect;
        let gap_top = a.y + a.height;
        let gap_bottom = b.y;
        assert!(gap_bottom > gap_top, "expected vertical gap between blocks");

        let probe_y = (gap_top + gap_bottom) * 0.5;
        let probe_x = a.x.min(b.x) + 1.0;
        assert!(selection_hit_test(
            tree, pages, &resolved, 0, probe_x, probe_y
        ));
    }

    #[test]
    fn selection_hit_test_outside_envelope_band_is_false() {
        let (doc, t1, t2) = doc! {
            root {
                paragraph { t1: text("hi") }
                paragraph { t2: text("a much longer line") }
            }
        };
        let view = layout(&doc);
        let resolved = Selection::new(Position::new(t1, 0), Position::new(t2, 18))
            .resolve(&doc)
            .unwrap();
        let tree = view.layout_tree_for_test().unwrap();
        let pages = view.pages();
        let rects = view.selection_rects(&resolved);
        let last = rects[1].rect;

        let probe_x = last.x + last.width + 50.0;
        let probe_y = last.y + last.height * 0.5;
        assert!(!selection_hit_test(
            tree, pages, &resolved, 0, probe_x, probe_y
        ));
    }

    #[test]
    fn selection_hit_test_wrong_page_is_false() {
        let (doc, t) = doc! { root { paragraph { t: text("hello world") } } };
        let view = layout(&doc);
        let resolved = Selection::new(Position::new(t, 0), Position::new(t, 5))
            .resolve(&doc)
            .unwrap();
        let tree = view.layout_tree_for_test().unwrap();
        let pages = view.pages();
        let rect = view.selection_rects(&resolved)[0].rect;
        let probe_x = rect.x + rect.width * 0.5;
        let probe_y = rect.y + rect.height * 0.5;

        assert!(selection_hit_test(
            tree, pages, &resolved, 0, probe_x, probe_y
        ));
        assert!(!selection_hit_test(
            tree, pages, &resolved, 1, probe_x, probe_y
        ));
    }
}
