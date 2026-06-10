use editor_common::Rect;
use editor_macros::ffi;
use editor_model::Doc;
use editor_state::{Affinity, Position};
use serde::{Deserialize, Serialize};

use crate::page::{LayoutPage, PageRect};
use crate::paginate::*;

use super::common::*;
use super::layout_index::{LayoutEntry, LayoutIndex};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionRectKind {
    Text,
    ParagraphBreak,
    Atom,
    Block,
}

pub type SelectionRect = PageRect<SelectionRectKind>;

#[derive(Debug, Clone, PartialEq)]
struct SelectionRectSets {
    pub line_box_rects: Vec<SelectionRect>,
    pub text_rects: Vec<SelectionRect>,
}

impl SelectionRectSets {
    fn empty() -> Self {
        Self {
            line_box_rects: Vec::new(),
            text_rects: Vec::new(),
        }
    }

    fn mirrored(rects: Vec<SelectionRect>) -> Self {
        Self {
            line_box_rects: rects.clone(),
            text_rects: rects,
        }
    }

    fn push_same(&mut self, rect: SelectionRect) {
        self.line_box_rects.push(rect.clone());
        self.text_rects.push(rect);
    }

    fn sort_by_position(&mut self) {
        let mut pairs: Vec<_> = std::mem::take(&mut self.line_box_rects)
            .into_iter()
            .zip(std::mem::take(&mut self.text_rects))
            .collect();
        pairs.sort_by(|(a, _), (b, _)| {
            a.page_idx
                .cmp(&b.page_idx)
                .then_with(|| a.rect.y.total_cmp(&b.rect.y))
                .then_with(|| a.rect.x.total_cmp(&b.rect.x))
        });
        (self.line_box_rects, self.text_rects) = pairs.into_iter().unzip();
    }
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SelectionEndpoints {
    pub from: PageRect,
    pub to: PageRect,
    pub from_position: Position,
    pub to_position: Position,
}

pub fn selection_rects(
    layout_index: &LayoutIndex,
    selection: &editor_state::ResolvedSelection<'_>,
) -> Vec<SelectionRect> {
    selection_rect_sets(layout_index, selection).line_box_rects
}

pub fn selection_text_rects(
    layout_index: &LayoutIndex,
    selection: &editor_state::ResolvedSelection<'_>,
) -> Vec<SelectionRect> {
    selection_rect_sets(layout_index, selection).text_rects
}

fn selection_rect_sets(
    layout_index: &LayoutIndex,
    selection: &editor_state::ResolvedSelection<'_>,
) -> SelectionRectSets {
    if selection.is_collapsed() {
        return SelectionRectSets::empty();
    }

    if let Some(cell_rect) = selection.as_cell_rect() {
        let ids: Vec<_> = cell_rect.cells().map(|cell| cell.id()).collect();
        let rects = block_selection_rects(layout_index, &ids);
        return SelectionRectSets::mirrored(rects);
    }

    let pages = layout_index.pages();
    let hard_breaks = super::hard_break::included_in_selection(layout_index, selection);
    let paragraph_breaks = super::paragraph_break::included_in_selection(layout_index, selection);

    let from = Position::from(selection.from());
    let to = Position::from(selection.to());
    // Resolve which Line/Atom each endpoint belongs to up front so soft-wrap
    // boundary positions are disambiguated by affinity
    // rather than by the permissive per-line `line_contains_position`.
    //
    // `LayoutIndex::entry_for_position` is permissive on purpose for navigation: an attached
    // box owns its child-boundary positions, and an atom owns both of its edges.
    // For rect attribution those box matches are structural container boundaries
    // that the box-level phase machine must claim. Strip them, while keeping atom
    // ownership only on the affinity-selected edge.
    let from_owner = layout_index
        .entry_for_position(&from)
        .filter(|entry| attached(layout_index, entry, &from));
    let to_owner = layout_index
        .entry_for_position(&to)
        .filter(|entry| attached(layout_index, entry, &to));
    let mut phase = Phase::Before;
    let mut rects = SelectionRectSets::empty();

    visit_node(
        &layout_index.tree().root,
        layout_index,
        &from,
        &to,
        from_owner,
        to_owner,
        &mut phase,
        &mut rects,
        pages,
        selection.doc(),
    );

    for hard_break in hard_breaks {
        let rect = hard_break_rect(hard_break.geometry);
        rects.push_same(rect);
    }
    for paragraph_break in paragraph_breaks {
        let rect = paragraph_break_rect(paragraph_break.geometry);
        rects.push_same(rect);
    }
    rects.sort_by_position();

    rects
}

fn hard_break_rect(geometry: super::hard_break::HardBreakGeometry) -> SelectionRect {
    let rect = geometry.rect;
    PageRect::with_meta(rect.page_idx, rect.rect, SelectionRectKind::Text)
}

fn paragraph_break_rect(geometry: super::paragraph_break::ParagraphBreakGeometry) -> SelectionRect {
    let rect = geometry.rect;
    PageRect::with_meta(rect.page_idx, rect.rect, SelectionRectKind::ParagraphBreak)
}

pub(crate) fn block_selection_rects(
    layout_index: &LayoutIndex,
    ids: &[editor_model::NodeId],
) -> Vec<SelectionRect> {
    layout_index
        .box_page_rects(ids)
        .into_iter()
        .map(|rect| PageRect::with_meta(rect.page_idx, rect.rect, SelectionRectKind::Block))
        .collect()
}

pub fn selection_endpoints(
    layout_index: &LayoutIndex,
    selection: &editor_state::ResolvedSelection<'_>,
) -> Option<SelectionEndpoints> {
    if selection.is_collapsed() {
        return None;
    }
    let rects = selection_rects(layout_index, selection);
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
        from_position: Position::from(selection.from()),
        to_position: Position::from(selection.to()),
    })
}

pub fn selection_hit_test(
    layout_index: &LayoutIndex,
    selection: &editor_state::ResolvedSelection<'_>,
    page_idx: usize,
    x: f32,
    y: f32,
) -> bool {
    if selection.is_collapsed() {
        return false;
    }

    if selected_external_atom_hit_test(layout_index, selection, page_idx, x, y) {
        return true;
    }

    let rects: Vec<Rect> = selection_rects(layout_index, selection)
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

fn selected_external_atom_hit_test(
    layout_index: &LayoutIndex,
    selection: &editor_state::ResolvedSelection<'_>,
    page_idx: usize,
    x: f32,
    y: f32,
) -> bool {
    let Some(page) = layout_index.page(page_idx) else {
        return false;
    };
    layout_index
        .entries_on_page(page_idx)
        .into_iter()
        .any(|entry| {
            let Some(LayoutContent::Atom(atom)) = entry.content(layout_index) else {
                return false;
            };
            let doc = selection.doc();
            let Some(node_ref) = doc.node(atom.node_id) else {
                return false;
            };
            if !node_ref.spec().external || !selection.contains_subtree(&node_ref) {
                return false;
            }

            let top = entry.rect.y.max(page.y_start);
            let bottom = entry.rect.bottom().min(page.y_end);
            Rect::from_xywh(
                entry.rect.x,
                top - page.y_start,
                entry.rect.width,
                bottom - top,
            )
            .contains(x, y)
        })
}

// Whether `entry` (as returned by `LayoutIndex::entry_for_position`) should be treated as the
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
fn attached(layout_index: &LayoutIndex, entry: &LayoutEntry, pos: &Position) -> bool {
    match entry.content(layout_index) {
        Some(LayoutContent::Box(_)) => false,
        Some(LayoutContent::Atom(atom)) => {
            let leading =
                pos.offset == atom.attachment.index && pos.affinity == Affinity::Downstream;
            let trailing =
                pos.offset == atom.attachment.index + 1 && pos.affinity == Affinity::Upstream;
            leading || trailing
        }
        _ => true,
    }
}

fn strut_line_has_selectable_child_range(line: &LayoutLine) -> bool {
    line.glyph_runs.is_empty()
        && line.tab_gaps.is_empty()
        && line
            .child_range
            .as_ref()
            .is_some_and(|range| range.start < range.end)
}

fn ruby_band(line: &LayoutLine) -> f32 {
    crate::measure::text::ruby::ruby_extra_top(line.baseline, line.ascent, &line.ruby_annotations)
}

fn text_area_height(line: &LayoutLine) -> f32 {
    let height = (line.ascent + line.descent - ruby_band(line)).max(0.0);
    if height > 0.0 {
        height
    } else if !line.glyph_runs.is_empty() {
        (line.cursor_ascent + line.cursor_descent).max(0.0)
    } else {
        0.0
    }
}

fn visit_node(
    node: &LayoutNode,
    layout_index: &LayoutIndex,
    from: &Position,
    to: &Position,
    from_owner: Option<&LayoutEntry>,
    to_owner: Option<&LayoutEntry>,
    phase: &mut Phase,
    rects: &mut SelectionRectSets,
    pages: &[LayoutPage],
    doc: &Doc,
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
            doc,
        ),
        LayoutContent::Line(l) => visit_line(
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
        ),
        LayoutContent::Atom(a) => visit_atom(node, a, from, to, phase, rects, pages, doc),
        LayoutContent::Spacing(_) => {}
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
    rects: &mut SelectionRectSets,
    pages: &[LayoutPage],
) {
    let contains_from = from_owner.is_some_and(|entry| entry.is_node(layout_index, node));
    let contains_to = to_owner.is_some_and(|entry| entry.is_node(layout_index, node));

    let placeholder_width = node.rect.height * 0.15;

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
        (Phase::Inside, false, false) => {
            let x0 = line_start_x(line);
            let x1 = line_end_x(line);
            (x0, x1)
        }
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
    } else if strut_line_has_selectable_child_range(line) {
        placeholder_width
    } else {
        return;
    };

    if let Some(page_idx) = page_for_y(pages, node.rect.y) {
        let band = ruby_band(line);
        let box_height = (node.rect.height - band).max(0.0);
        let x = node.rect.x + x_start;
        let box_top = node.rect.y + band - pages[page_idx].y_start;
        let push = |rects: &mut Vec<SelectionRect>, top: f32, height: f32| {
            rects.push(PageRect::with_meta(
                page_idx,
                Rect::from_xywh(x, top, width, height),
                SelectionRectKind::Text,
            ));
        };

        push(&mut rects.line_box_rects, box_top, box_height);

        let text_height = text_area_height(line);
        let text_top = box_top + (box_height - text_height).max(0.0) * 0.5;
        push(&mut rects.text_rects, text_top, text_height);
    }
}

fn visit_atom(
    node: &LayoutNode,
    atom: &LayoutAtom,
    from: &Position,
    to: &Position,
    phase: &mut Phase,
    rects: &mut SelectionRectSets,
    pages: &[LayoutPage],
    doc: &Doc,
) {
    let is_from = from.node_id == atom.attachment.parent_id && from.offset == atom.attachment.index;
    let is_to = to.node_id == atom.attachment.parent_id && to.offset == atom.attachment.index + 1;

    if *phase == Phase::Before && is_from {
        *phase = Phase::Inside;
    }

    if *phase != Phase::Inside {
        return;
    }

    // External nodes render their own selection affordance; the selection
    // layer must not paint a rect over them.
    let is_external = doc.node(atom.node_id).is_some_and(|n| n.spec().external);
    if !is_external && let Some(page_idx) = page_for_y(pages, node.rect.y) {
        let rect = PageRect::with_meta(
            page_idx,
            Rect::from_xywh(
                node.rect.x,
                node.rect.y - pages[page_idx].y_start,
                node.rect.width,
                node.rect.height,
            ),
            SelectionRectKind::Atom,
        );
        rects.push_same(rect);
    }

    if is_to {
        *phase = Phase::After;
    }
}

fn visit_box(
    node: &LayoutNode,
    bx: &LayoutBox,
    layout_index: &LayoutIndex,
    from: &Position,
    to: &Position,
    from_owner: Option<&LayoutEntry>,
    to_owner: Option<&LayoutEntry>,
    phase: &mut Phase,
    rects: &mut SelectionRectSets,
    pages: &[LayoutPage],
    doc: &Doc,
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
    let line_box_rects_before = rects.line_box_rects.len();
    let text_rects_before = rects.text_rects.len();
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
            doc,
        );

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
        rects.line_box_rects.truncate(line_box_rects_before);
        rects.text_rects.truncate(text_rects_before);
        let node_top = node.rect.y;
        let node_bottom = node_top + node.rect.height;
        for (page_idx, page) in pages.iter().enumerate() {
            if node_bottom <= page.y_start || node_top >= page.y_end {
                continue;
            }
            let top = node_top.max(page.y_start);
            let bottom = node_bottom.min(page.y_end);
            let rect = PageRect::with_meta(
                page_idx,
                Rect::from_xywh(
                    node.rect.x,
                    top - page.y_start,
                    node.rect.width,
                    bottom - top,
                ),
                SelectionRectKind::Block,
            );
            rects.push_same(rect);
        }
    }
}

#[cfg(test)]
mod tests {
    use editor_macros::{doc, state};
    use editor_model::NodeId;
    use editor_state::{Affinity, Position, Selection};

    use super::*;
    use crate::view::View;

    fn layout(doc: &editor_model::Doc) -> View {
        let mut view = View::new_test();
        view.layout(doc);
        view
    }

    fn first_line<'a>(node: &'a LayoutNode) -> Option<(&'a LayoutNode, &'a LayoutLine)> {
        match &node.content {
            LayoutContent::Line(line) => Some((node, line)),
            LayoutContent::Box(bx) => bx.children.iter().find_map(first_line),
            LayoutContent::Atom(_) | LayoutContent::Spacing(_) => None,
        }
    }

    fn line_y_for_child_range(
        view: &View,
        node_id: NodeId,
        child_range: std::ops::Range<usize>,
    ) -> Option<f32> {
        fn find(
            node: &LayoutNode,
            node_id: NodeId,
            child_range: &std::ops::Range<usize>,
        ) -> Option<f32> {
            match &node.content {
                LayoutContent::Line(line)
                    if line.node_id == node_id
                        && line.child_range.as_ref() == Some(child_range) =>
                {
                    Some(node.rect.y)
                }
                LayoutContent::Box(bx) => bx
                    .children
                    .iter()
                    .find_map(|child| find(child, node_id, child_range)),
                LayoutContent::Line(_) | LayoutContent::Atom(_) | LayoutContent::Spacing(_) => None,
            }
        }

        find(&view.layout_tree_for_test()?.root, node_id, &child_range)
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
    fn paragraph_break_selection_has_paragraph_break_rect() {
        let (doc, _p1, t1, _p2, _t2) = doc! {
            root {
                p1: paragraph { t1: text("a") }
                p2: paragraph { t2: text("b") }
            }
        };
        let view = layout(&doc);

        let sel =
            editor_state::paragraph_break_selection_at_paragraph_end(&doc, Position::new(t1, 1))
                .expect("P -> P has paragraph break");
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].meta, SelectionRectKind::ParagraphBreak);
        assert!(rects[0].rect.width > 0.0);
        assert!(rects[0].rect.height > 0.0);
    }

    #[test]
    fn removable_empty_paragraph_break_selection_has_paragraph_break_rect() {
        let (doc, root, p1) = doc! {
            root: root {
                p1: paragraph {}
                callout { paragraph { text("callout") } }
                paragraph { text("tail") }
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(
            Position::new(p1, 0),
            Position {
                node_id: root,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        );
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].meta, SelectionRectKind::ParagraphBreak);
        assert!(rects[0].rect.width > 0.0);
        assert!(rects[0].rect.height > 0.0);
    }

    #[test]
    fn selection_containing_paragraph_break_shows_paragraph_break_rect() {
        let (doc, _p1, t1, _p2, t2) = doc! {
            root {
                p1: paragraph { t1: text("a") }
                p2: paragraph { t2: text("bc") }
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(
            Position {
                node_id: t1,
                offset: 1,
                affinity: Affinity::Downstream,
            },
            Position::new(t2, 1),
        );
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert!(
            rects.iter().any(|r| r.meta == SelectionRectKind::Text),
            "range must still show selected text, got {rects:?}"
        );
        assert!(
            rects
                .iter()
                .any(|r| r.meta == SelectionRectKind::ParagraphBreak),
            "range containing PB must show PB affordance, got {rects:?}"
        );
    }

    #[test]
    fn selection_to_next_paragraph_start_with_upstream_affinity_shows_paragraph_break_rect() {
        let (state, ..) = state! {
            doc {
                root [block_gap(200)] {
                    paragraph {
                        t1: text("aa")
                    }
                    paragraph {
                        t2: text("bb")
                    }
                }
            }
            selection: (t1, 1, >) -> (t2, 0, <)
        };
        let view = layout(&state.doc);
        let resolved = state.selection.unwrap().resolve(&state.doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert!(
            rects
                .iter()
                .any(|r| r.meta == SelectionRectKind::ParagraphBreak),
            "range ending at next paragraph start must show PB affordance, got {rects:?}"
        );
    }

    #[test]
    fn reversed_selection_from_next_paragraph_text_shows_paragraph_break_rect() {
        let (state, ..) = state! {
            doc {
                root [block_gap(200)] {
                    paragraph {
                        t1: text("aa")
                    }
                    paragraph {
                        t2: text("bb")
                    }
                }
            }
            selection: (t2, 1) -> (t1, 2)
        };
        let view = layout(&state.doc);
        let resolved = state.selection.unwrap().resolve(&state.doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert!(
            rects
                .iter()
                .any(|r| r.meta == SelectionRectKind::ParagraphBreak),
            "reversed range containing PB must show PB affordance, got {rects:?}"
        );
    }

    #[test]
    fn selection_from_text_middle_to_empty_paragraph_shows_text_and_paragraph_break_only() {
        let (doc, _p1, t1, p2) = doc! {
            root {
                p1: paragraph { t1: text("abc") }
                p2: paragraph {}
                paragraph { text("tail") }
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(
            Position::new(t1, 1),
            Position {
                node_id: p2,
                offset: 0,
                affinity: Affinity::Downstream,
            },
        );
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);
        let kinds: Vec<_> = rects.iter().map(|r| r.meta).collect();

        assert_eq!(
            kinds,
            vec![SelectionRectKind::Text, SelectionRectKind::ParagraphBreak],
            "empty paragraph placeholder rect must not be emitted, got {rects:?}"
        );
    }

    #[test]
    fn selection_from_empty_paragraph_to_text_shows_paragraph_break_and_text_only() {
        let (doc, p1, _p2, t2) = doc! {
            root {
                p1: paragraph {}
                p2: paragraph { t2: text("abc") }
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(Position::new(p1, 0), Position::new(t2, 1));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);
        let kinds: Vec<_> = rects.iter().map(|r| r.meta).collect();

        assert_eq!(
            kinds,
            vec![SelectionRectKind::ParagraphBreak, SelectionRectKind::Text],
            "empty paragraph placeholder rect must not be emitted, got {rects:?}"
        );
    }

    #[test]
    fn selection_through_empty_paragraph_shows_paragraph_breaks_without_placeholder() {
        let (doc, _p1, t1, _p2, _p3, t3) = doc! {
            root {
                p1: paragraph { t1: text("abc") }
                p2: paragraph {}
                p3: paragraph { t3: text("tail") }
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(Position::new(t1, 1), Position::new(t3, 1));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);
        let kinds: Vec<_> = rects.iter().map(|r| r.meta).collect();

        assert_eq!(
            kinds,
            vec![
                SelectionRectKind::Text,
                SelectionRectKind::ParagraphBreak,
                SelectionRectKind::ParagraphBreak,
                SelectionRectKind::Text,
            ],
            "empty paragraph placeholder rect must not be emitted, got {rects:?}"
        );
    }

    #[test]
    fn paragraph_boundary_selection_over_two_hard_breaks_shows_each_break_and_pb() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        hard_break
                        hard_break
                    }
                    p2: paragraph {}
                }
            }
            selection: (p1, 0, >) -> (p2, 0, <)
        };
        let view = layout(&state.doc);
        let resolved = state.selection.unwrap().resolve(&state.doc).unwrap();
        let rects = view.selection_rects(&resolved);
        let kinds: Vec<_> = rects.iter().map(|r| r.meta).collect();

        assert_eq!(
            kinds,
            vec![
                SelectionRectKind::Text,
                SelectionRectKind::Text,
                SelectionRectKind::ParagraphBreak,
            ],
            "two hard_break rects and the paragraph break must be visible, got {rects:?}"
        );
    }

    #[test]
    fn trailing_hard_break_after_soft_wrap_line_is_visible() {
        let (state, p1, _t1, ..) = state! {
            doc {
                root (layout_mode: editor_model::LayoutMode::Continuous { max_width: 40 }) {
                    p1: paragraph {
                        t1: text("abcdefgh")
                        hard_break
                        text("z")
                    }
                }
            }
            selection: (t1, 8, >) -> (p1, 2, <)
        };
        let view = layout(&state.doc);
        let trailing_line_y = line_y_for_child_range(&view, p1, 1..1)
            .expect("soft-wrapped trailing hard_break line exists");
        let resolved = state.selection.unwrap().resolve(&state.doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 1, "hard_break must be visible, got {rects:?}");
        assert_eq!(rects[0].meta, SelectionRectKind::Text);
        assert!(rects[0].rect.width > 0.0, "got {rects:?}");
        assert!(
            (rects[0].rect.y - trailing_line_y).abs() < 0.01,
            "hard_break rect must be on trailing wrapped line y={trailing_line_y}, got {rects:?}",
        );
    }

    #[test]
    fn paragraph_break_after_trailing_hard_break_uses_trailing_empty_line() {
        let (state, p1, _p2) = state! {
            doc {
                root [block_gap(200)] {
                    p1: paragraph {
                        hard_break
                        hard_break
                    }
                    p2: paragraph {}
                }
            }
            selection: (p1, 2, >) -> (p2, 0, <)
        };
        let view = layout(&state.doc);
        let trailing_line_y = line_y_for_child_range(&view, p1, 2..2)
            .expect("trailing empty line after hard_breaks exists");
        let resolved = state.selection.unwrap().resolve(&state.doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 1, "got {rects:?}");
        assert_eq!(rects[0].meta, SelectionRectKind::ParagraphBreak);
        assert!(
            (rects[0].rect.y - trailing_line_y).abs() < 0.01,
            "PB must be drawn on trailing empty line y={trailing_line_y}, got {rects:?}",
        );
    }

    #[test]
    fn root_selection_over_atom_and_hard_break_paragraph_shows_hard_break_rects() {
        let (state, ..) = state! {
            doc {
                r1: root {
                    image
                    paragraph {
                        hard_break
                        hard_break
                    }
                }
            }
            selection: (r1, 0, >) -> (r1, 2, <)
        };
        let view = layout(&state.doc);
        let resolved = state.selection.unwrap().resolve(&state.doc).unwrap();
        let rects = view.selection_rects(&resolved);
        let text_rect_count = rects
            .iter()
            .filter(|rect| rect.meta == SelectionRectKind::Text)
            .count();

        assert_eq!(
            text_rect_count, 2,
            "both hard_breaks in the selected paragraph must be visible, got {rects:?}"
        );
    }

    #[test]
    fn selection_text_rect_uses_centered_text_area_height() {
        let (doc, t) = doc! {
            root {
                paragraph [line_height(300)] { t: text("hello") }
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(Position::new(t, 1), Position::new(t, 4));
        let resolved = sel.resolve(&doc).unwrap();
        let line_box_rect = view.selection_rects(&resolved)[0].rect;
        let text_rect = view.selection_text_rects(&resolved)[0].rect;

        let tree = view.layout_tree_for_test().unwrap();
        let page_start = view.pages()[0].y_start;
        let (line_node, line) = first_line(&tree.root).expect("line must exist");
        let expected_height = text_area_height(line);
        let expected_y =
            line_node.rect.y + (line_node.rect.height - expected_height) * 0.5 - page_start;

        assert!(line_box_rect.height > text_rect.height);
        assert!((text_rect.y - expected_y).abs() < 0.01);
        assert!((text_rect.height - expected_height).abs() < 0.01);
        assert!((line_box_rect.x - text_rect.x).abs() < 0.01);
        assert!((line_box_rect.width - text_rect.width).abs() < 0.01);
    }

    #[test]
    fn selection_highlight_excludes_ruby_band() {
        // The rendered selection highlight uses `selection_rects` (line box).
        // Ruby inflates the line box upward; that band must be excluded so the
        // highlight never covers the ruby drawn above the text. (TR-222)
        let (plain_doc, p) = doc! { root { paragraph { p: text("ABCD") } } };
        let (ruby_doc, r) = doc! {
            root { paragraph { r: text("ABCD") [ruby(text: "xy".to_string())] } }
        };
        let plain = layout(&plain_doc);
        let ruby = layout(&ruby_doc);

        let plain_box = {
            let sel = Selection::new(Position::new(p, 0), Position::new(p, 4));
            plain.selection_rects(&sel.resolve(&plain_doc).unwrap())[0].rect
        };
        let resolved = Selection::new(Position::new(r, 0), Position::new(r, 4))
            .resolve(&ruby_doc)
            .unwrap();
        let ruby_box = ruby.selection_rects(&resolved)[0].rect;

        let ruby_tree = ruby.layout_tree_for_test().unwrap();
        let (ruby_node, ruby_line) = first_line(&ruby_tree.root).expect("line must exist");
        let band = ruby_band(ruby_line);

        // Ruby actually inflated this line (else the test is vacuous).
        assert!(band > 0.0, "ruby must reserve space above the text");
        // The rendered highlight excludes the ruby band: same height as the
        // plain (ruby-free, v1-equivalent) line, not the inflated line box.
        assert!(
            (ruby_box.height - plain_box.height).abs() < 0.5,
            "ruby must not enlarge the selection highlight: plain={}, ruby={}",
            plain_box.height,
            ruby_box.height,
        );
        assert!(
            (ruby_box.height - (ruby_node.rect.height - band)).abs() < 0.01,
            "highlight height must be line box minus the ruby band",
        );
        // ...and the highlight starts below the reserved ruby band, not at the
        // inflated line-box top.
        let page_start = ruby.pages()[0].y_start;
        assert!(ruby_box.y >= ruby_node.rect.y + band - page_start - 0.01);
    }

    #[test]
    fn selection_text_rect_matches_v1_metric_height_without_ruby() {
        // Without ruby the text rect height equals the v1 formula `ascent +
        // descent` (v1 `metric.height`).
        let (doc, t) = doc! { root { paragraph { t: text("ABCD") } } };
        let view = layout(&doc);
        let resolved = Selection::new(Position::new(t, 0), Position::new(t, 4))
            .resolve(&doc)
            .unwrap();
        let text_rect = view.selection_text_rects(&resolved)[0].rect;

        let tree = view.layout_tree_for_test().unwrap();
        let (_, line) = first_line(&tree.root).expect("line must exist");
        let v1_height = line.ascent + line.descent;
        assert!(
            (text_rect.height - v1_height).abs() < 0.01,
            "text rect height must equal v1 metric.height: rect={}, ascent+descent={}",
            text_rect.height,
            v1_height,
        );
    }

    #[test]
    fn selection_rects_match_v1_height_formulas_on_mixed_font_line() {
        let (doc, small, big) = doc! {
            root {
                paragraph [line_height(200)] {
                    small: text("a")
                    big: text("A") [font_size(4800)]
                }
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(Position::new(small, 0), Position::new(big, 1));
        let resolved = sel.resolve(&doc).unwrap();
        let line_box_rect = view.selection_rects(&resolved)[0].rect;
        let text_rect = view.selection_text_rects(&resolved)[0].rect;

        let tree = view.layout_tree_for_test().unwrap();
        let page_start = view.pages()[0].y_start;
        let (line_node, line) = first_line(&tree.root).expect("line must exist");

        let expected_text_height = text_area_height(line);
        let expected_text_y =
            line_node.rect.y + (line_node.rect.height - expected_text_height) * 0.5 - page_start;
        let expected_line_box_height = line_node.rect.height;
        let cursor_height = line.cursor_ascent + line.cursor_descent;

        assert!(
            text_rect.height > cursor_height,
            "mixed-font line must follow v1-like line ascent/descent, not base cursor strut: text={}, cursor={}",
            text_rect.height,
            cursor_height,
        );
        assert!((text_rect.y - expected_text_y).abs() < 0.01);
        assert!((text_rect.height - expected_text_height).abs() < 0.01);
        assert!((line_box_rect.height - expected_line_box_height).abs() < 0.01);
        assert!(line_box_rect.height > text_rect.height);
    }

    #[test]
    fn selection_text_rects_preserve_line_box_rect_order_and_horizontal_span() {
        let (doc, t1, t2) = doc! {
            root {
                paragraph [line_height(300)] { t1: text("hello") }
                paragraph [line_height(300)] { t2: text("world") }
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(Position::new(t1, 1), Position::new(t2, 4));
        let resolved = sel.resolve(&doc).unwrap();
        let line_box_rects = view.selection_rects(&resolved);
        let text_rects = view.selection_text_rects(&resolved);

        assert_eq!(line_box_rects.len(), text_rects.len());
        for (line_box, text) in line_box_rects.iter().zip(text_rects.iter()) {
            assert_eq!(line_box.page_idx, text.page_idx);
            assert_eq!(line_box.meta, text.meta);
            assert!((line_box.rect.x - text.rect.x).abs() < 0.01);
            assert!((line_box.rect.width - text.rect.width).abs() < 0.01);
            if line_box.meta == SelectionRectKind::Text {
                assert!(line_box.rect.height > text.rect.height);
            } else {
                assert!((line_box.rect.height - text.rect.height).abs() < 0.01);
            }
        }
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

        assert_eq!(rects.len(), 3);
        assert_eq!(rects[0].meta, SelectionRectKind::Text);
        assert_eq!(rects[1].meta, SelectionRectKind::ParagraphBreak);
        assert_eq!(rects[2].meta, SelectionRectKind::Text);
        assert!(rects[0].rect.y < rects[2].rect.y);
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
    fn external_atom_selection_emits_no_rect() {
        let (doc,) = doc! {
            root {
                image
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
            rects.is_empty(),
            "external atom must not emit a selection rect, got {:?}",
            rects
        );
    }

    #[test]
    fn selection_hit_test_includes_selected_external_atom_bounds() {
        let (doc, img) = doc! {
            root {
                img: image
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(
            Position::new(NodeId::ROOT, 0),
            Position::new(NodeId::ROOT, 1),
        );
        let resolved = sel.resolve(&doc).unwrap();
        let rect = view
            .external_elements(&doc, Some(&sel))
            .into_iter()
            .find(|element| element.node_id == img)
            .expect("image external element")
            .bounds;

        assert!(
            view.selection_hit_test(
                &resolved,
                0,
                rect.x + rect.width * 0.5,
                rect.y + rect.height * 0.5,
            ),
            "selected external atoms must be a hit target for native DnD admission"
        );
    }

    #[test]
    fn selection_skips_external_atom_between_text() {
        let (doc, t1, t2) = doc! {
            root {
                paragraph { t1: text("hi") }
                image
                paragraph { t2: text("bye") }
            }
        };
        let view = layout(&doc);

        let sel = Selection::new(Position::new(t1, 0), Position::new(t2, 3));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert_eq!(rects.len(), 2, "expected text rects only, got {:?}", rects);
        assert!(rects.iter().all(|r| r.meta == SelectionRectKind::Text));
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
        // Force soft-wrap: max_width=40 (content width) — exactly 4 ASCII test
        // chars per line. "abcdefgh" wraps as
        //   line A (offset 0..4): "abcd"
        //   line B (offset 4..8): "efgh"
        // Selecting the lower line's leading character (offset 4 → 5) must
        // produce one rect over the lower line.
        let (doc, t) = doc! {
            root (layout_mode: editor_model::LayoutMode::Continuous { max_width: 40 }) {
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
            3,
            "expected text rects for a/b/c without an empty paragraph placeholder for p1, got {:?}",
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
    fn empty_paragraph_after_atom_node_selected_emits_no_rect() {
        // Node-selecting the empty paragraph that follows an image atom.
        // (r1, 1, >) means Downstream-attached-to-child[1] (the paragraph),
        // (r1, 2, <) means Upstream-attached-to-child[1] (the paragraph);
        // both endpoints bracket the empty paragraph from the root level.
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

        assert!(rects.is_empty(), "got {:?}", rects);
    }

    #[test]
    fn paragraph_after_atom_node_selected_emits_text_rect() {
        // Non-empty counterpart of the previous test. The same affinity-bracket
        // pattern around child[1] must also paint the line for a populated
        // paragraph.
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

        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].meta, SelectionRectKind::Text);
        assert!(rects[0].rect.width > 0.0);
    }

    #[test]
    fn selection_endpoints_collapsed_returns_none() {
        let (doc, t) = doc! { root { paragraph { t: text("hello") } } };
        let view = layout(&doc);

        let sel = Selection::collapsed(Position::new(t, 2));
        let resolved = sel.resolve(&doc).unwrap();
        assert!(view.selection_endpoints(&resolved).is_none());
    }

    #[test]
    fn selection_hit_test_collapsed_is_false() {
        let (doc, t) = doc! { root { paragraph { t: text("hello") } } };
        let view = layout(&doc);
        let resolved = Selection::collapsed(Position::new(t, 2))
            .resolve(&doc)
            .unwrap();
        assert!(!view.selection_hit_test(&resolved, 0, 5.0, 5.0));
    }

    #[test]
    fn selection_hit_test_single_line_inside_rect_is_true() {
        let (doc, t) = doc! { root { paragraph { t: text("hello world") } } };
        let view = layout(&doc);
        let resolved = Selection::new(Position::new(t, 0), Position::new(t, 5))
            .resolve(&doc)
            .unwrap();
        let rect = view.selection_rects(&resolved)[0].rect;

        assert!(view.selection_hit_test(
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
        let rect = view.selection_rects(&resolved)[0].rect;

        assert!(!view.selection_hit_test(
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
        let rects = view.selection_rects(&resolved);
        let first = rects[0].rect;
        let last = rects[1].rect;
        let max_x = last.x + last.width;

        assert!(max_x > first.x + first.width, "second line must be wider");

        let probe_x = first.x + first.width + (max_x - (first.x + first.width)) * 0.5;
        let probe_y = first.y + first.height * 0.5;
        assert!(view.selection_hit_test(&resolved, 0, probe_x, probe_y));
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
        assert!(view.selection_hit_test(&resolved, 0, probe_x, probe_y));
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
        assert!(view.selection_hit_test(&resolved, 0, probe_x, probe_y));
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
        let rects = view.selection_rects(&resolved);
        let last = rects
            .iter()
            .rev()
            .find(|rect| rect.meta == SelectionRectKind::Text)
            .unwrap()
            .rect;

        let probe_x = last.x + last.width + 50.0;
        let probe_y = last.y + last.height * 0.5;
        assert!(!view.selection_hit_test(&resolved, 0, probe_x, probe_y));
    }

    #[test]
    fn selection_hit_test_wrong_page_is_false() {
        let (doc, t) = doc! { root { paragraph { t: text("hello world") } } };
        let view = layout(&doc);
        let resolved = Selection::new(Position::new(t, 0), Position::new(t, 5))
            .resolve(&doc)
            .unwrap();
        let rect = view.selection_rects(&resolved)[0].rect;
        let probe_x = rect.x + rect.width * 0.5;
        let probe_y = rect.y + rect.height * 0.5;

        assert!(view.selection_hit_test(&resolved, 0, probe_x, probe_y));
        assert!(!view.selection_hit_test(&resolved, 1, probe_x, probe_y));
    }

    #[test]
    fn selection_covering_leading_tab_emits_rect_over_gap() {
        // Paragraph: tab(child 0) text("xx")(child 1). Select the whole
        // paragraph content; the emitted rect must span the tab gap.
        let (doc, p1) = doc! { root { p1: paragraph { tab {} text("xx") } } };
        let view = layout(&doc);

        let tree = view.layout_tree_for_test().unwrap();
        let (line_node, line) = first_line(&tree.root).expect("line must exist");
        let gap = line.tab_gaps.first().cloned().expect("tab gap exists");
        let line_x = line_node.rect.x;

        let sel = Selection::new(Position::new(p1, 0), Position::new(p1, 2));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert!(!rects.is_empty(), "expected a selection rect");
        let r = rects[0].rect;
        let gap_lo = line_x + gap.x;
        let gap_hi = line_x + gap.x + gap.width;
        // The selection rect must overlap the gap's [x, x+width] span.
        assert!(
            r.x <= gap_lo + 0.5 && r.x + r.width >= gap_hi - 0.5,
            "rect [{}, {}] must cover gap [{}, {}]",
            r.x,
            r.x + r.width,
            gap_lo,
            gap_hi,
        );
    }

    #[test]
    fn selection_covering_only_tab_emits_rect_over_gap() {
        // Tab-only paragraph: selecting the tab must still produce a visible
        // rect spanning the gap (no glyph runs to fall back on).
        let (doc, p1) = doc! { root { p1: paragraph { tab {} } } };
        let view = layout(&doc);

        let tree = view.layout_tree_for_test().unwrap();
        let (line_node, line) = first_line(&tree.root).expect("line must exist");
        let gap = line.tab_gaps.first().cloned().expect("tab gap exists");
        let line_x = line_node.rect.x;

        let sel = Selection::new(Position::new(p1, 0), Position::new(p1, 1));
        let resolved = sel.resolve(&doc).unwrap();
        let rects = view.selection_rects(&resolved);

        assert!(!rects.is_empty(), "expected a selection rect for the tab");
        let r = rects[0].rect;
        let gap_lo = line_x + gap.x;
        let gap_hi = line_x + gap.x + gap.width;
        assert!(r.width > 0.0, "tab selection rect must have width");
        assert!(
            r.x <= gap_lo + 0.5 && r.x + r.width >= gap_hi - 0.5,
            "rect [{}, {}] must cover gap [{}, {}]",
            r.x,
            r.x + r.width,
            gap_lo,
            gap_hi,
        );
    }
}
