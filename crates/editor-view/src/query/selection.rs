#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionRectKind {
    Text,
    ParagraphBreak,
    Atom,
    Block,
}

pub type SelectionRect = PageRect<SelectionRectKind>;

use editor_common::Rect;
use editor_macros::ffi;
use editor_model::DocView;
use editor_state::Affinity;
use editor_state::{Position, ResolvedSelection};
use serde::{Deserialize, Serialize};

use crate::page::{LayoutPage, PageRect};
use crate::paginate::types::{LayoutAtom, LayoutBox, LayoutContent, LayoutLine, LayoutNode};

use super::common::{Phase, line_end_x, line_start_x, page_for_y};
use super::grapheme;
use super::layout_index::{LayoutEntry, LayoutIndex};

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

pub(crate) fn selection_rects(
    layout_index: &LayoutIndex,
    selection: &ResolvedSelection<'_>,
) -> Vec<SelectionRect> {
    selection_rect_sets(layout_index, selection).line_box_rects
}

pub(crate) fn selection_text_rects(
    layout_index: &LayoutIndex,
    selection: &ResolvedSelection<'_>,
) -> Vec<SelectionRect> {
    selection_rect_sets(layout_index, selection).text_rects
}

fn selection_rect_sets(
    layout_index: &LayoutIndex,
    selection: &ResolvedSelection<'_>,
) -> SelectionRectSets {
    if selection.is_collapsed() {
        return SelectionRectSets::empty();
    }

    if let Some(cell_rect) = selection.as_cell_rect() {
        let ids: Vec<_> = cell_rect
            .cells()
            .into_iter()
            .map(|cell| cell.id())
            .collect();
        let rects = block_selection_rects(layout_index, &ids);
        return SelectionRectSets::mirrored(rects);
    }

    let pages = layout_index.pages();

    let from = selection.from().position();
    let to = selection.to().position();

    let from_entry = layout_index.entry_for_position(&from);
    let to_entry = layout_index.entry_for_position(&to);

    let break_y_bounds = match (from_entry, to_entry) {
        (Some(f), Some(t)) => Some((f.rect.y.min(t.rect.y), f.rect.bottom().max(t.rect.bottom()))),
        _ => None,
    };

    let hard_breaks =
        super::hard_break::included_in_selection(layout_index, selection, break_y_bounds);
    let paragraph_breaks =
        super::paragraph_break::included_in_selection(layout_index, selection, break_y_bounds);

    let from_owner = from_entry.filter(|entry| attached(layout_index, entry, &from));
    let to_owner = to_entry.filter(|entry| attached(layout_index, entry, &to));

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
        selection.view(),
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
    ids: &[editor_crdt::Dot],
) -> Vec<SelectionRect> {
    layout_index
        .box_page_rects(ids)
        .into_iter()
        .map(|rect| PageRect::with_meta(rect.page_idx, rect.rect, SelectionRectKind::Block))
        .collect()
}

pub(crate) fn selection_endpoints(
    layout_index: &LayoutIndex,
    selection: &ResolvedSelection<'_>,
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
        from_position: selection.from().position(),
        to_position: selection.to().position(),
    })
}

pub(crate) fn selection_hit_test(
    layout_index: &LayoutIndex,
    selection: &ResolvedSelection<'_>,
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
    selection: &ResolvedSelection<'_>,
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
            let view = selection.view();
            let Some(node_ref) = view.node(atom.node) else {
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
            .offset_range
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
    doc: &DocView,
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
            let x0 = grapheme::x_at_offset(line, from);
            let x1 = grapheme::x_at_offset(line, to);
            *phase = Phase::After;
            (x0, x1)
        }
        (Phase::Before, true, false) => {
            let x0 = grapheme::x_at_offset(line, from);
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
            let x1 = grapheme::x_at_offset(line, to);
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
    doc: &DocView,
) {
    let is_from = from.node == atom.attachment.parent && from.offset == atom.attachment.index;
    let is_to = to.node == atom.attachment.parent && to.offset == atom.attachment.index + 1;

    if *phase == Phase::Before && is_from {
        *phase = Phase::Inside;
    }

    if *phase != Phase::Inside {
        return;
    }

    let is_external = doc.node(atom.node).is_some_and(|n| n.spec().external);
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

#[allow(clippy::too_many_arguments)]
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
    doc: &DocView,
) {
    let from_at_box_level = from.node == bx.node && from_owner.is_none();
    let to_at_box_level = to.node == bx.node && to_owner.is_none();

    let entry_phase = *phase;
    let line_box_rects_before = rects.line_box_rects.len();
    let text_rects_before = rects.text_rects.len();
    let mut has_content_child = false;
    let mut content_idx = 0usize;

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
    use editor_common::EdgeInsets;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AtomLeaf, DocLogs, DocView, HorizontalRuleVariant, ModifierAttrLog, NodeAttrLog,
        NodeMarkerLog, NodeStyleLog, NodeType, ProjectedDoc, SeqItem, SpanLog, StyleLog,
        project_document,
    };
    use editor_state::Affinity;
    use editor_state::{Position, Selection};

    use crate::measure::context::MeasureContext;
    use crate::measure::nodes::dispatch::measure_node;
    use crate::measure::types::MeasuredTree;
    use crate::paginate::paginator::Paginator;
    use crate::query::layout_index::LayoutIndex;
    use editor_resource::Resource;

    use super::*;

    fn logs(items: &[(Dot, SeqItem)]) -> DocLogs {
        let mut ev = Vec::new();
        let mut prev: Option<Dot> = None;
        for (i, (id, item)) in items.iter().enumerate() {
            ev.push(InputEvent {
                id: *id,
                parents: prev.into_iter().collect(),
                op: ListOp::Ins {
                    pos: i,
                    item: item.clone(),
                },
            });
            prev = Some(*id);
        }
        DocLogs {
            seq: build_oplog(&ev),
            spans: SpanLog::new(),
            block_modifiers: ModifierAttrLog::new(),
            node_attrs: NodeAttrLog::new(),
            node_styles: NodeStyleLog::new(),
            node_markers: NodeMarkerLog::new(),
            styles: StyleLog::new(),
        }
    }

    fn build_index(doc: &DocLogs, width: f32) -> (ProjectedDoc, LayoutIndex) {
        let pd = project_document(doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let mut res = Resource::new_test();
        let measured = measure_node(
            &mut crate::measure::Measurer::new(),
            &root_node,
            width,
            &MeasureContext::default(),
            &mut res,
        );
        let layout = Paginator::continuous(width, 100_000.0, EdgeInsets::all(0.0))
            .paginate(MeasuredTree { root: measured });
        let index = LayoutIndex::new(layout.tree, &layout.pages);
        (pd, index)
    }

    fn simple_para_doc(text: &str) -> (DocLogs, Dot, Dot) {
        let root = Dot::ROOT;
        let para = Dot::new(1, 1);
        let mut items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
            },
        )];
        for (i, ch) in text.chars().enumerate() {
            items.push((Dot::new(1, 2 + i as u64), SeqItem::Char(ch)));
        }
        (logs(&items), root, para)
    }

    fn first_line_for_para<'a>(
        index: &'a LayoutIndex,
        para_id: &Dot,
    ) -> Option<(&'a crate::query::layout_index::LayoutEntry, &'a LayoutLine)> {
        for entry in index.entries() {
            if let Some(node) = entry.node(index)
                && let LayoutContent::Line(line) = &node.content
                && &line.node == para_id
            {
                return Some((entry, line));
            }
        }
        None
    }

    #[test]
    fn collapsed_empty() {
        let (doc, _root, para) = simple_para_doc("Hello");
        let (pd, index) = build_index(&doc, 400.0);
        let view = DocView::new(&pd);
        let para_id = para;

        let sel = Selection::collapsed(Position {
            node: para_id,
            offset: 2,
            affinity: Affinity::Downstream,
        });
        let rsel = sel.resolve(&view).expect("must resolve");

        assert!(selection_rects(&index, &rsel).is_empty());
        assert!(selection_endpoints(&index, &rsel).is_none());
        assert!(!selection_hit_test(&index, &rsel, 0, 0.0, 0.0));
    }

    #[test]
    fn single_line_span() {
        let (doc, _root, para) = simple_para_doc("Hello");
        let (pd, index) = build_index(&doc, 400.0);
        let view = DocView::new(&pd);
        let para_id = para;

        let from_pos = Position {
            node: para_id,
            offset: 1,
            affinity: Affinity::Downstream,
        };
        let to_pos = Position {
            node: para_id,
            offset: 4,
            affinity: Affinity::Upstream,
        };
        let sel = Selection::new(from_pos, to_pos);
        let rsel = sel.resolve(&view).expect("must resolve");

        let rects = selection_rects(&index, &rsel);
        assert_eq!(
            rects.len(),
            1,
            "single-line span must yield one rect, got {:?}",
            rects
        );
        assert_eq!(rects[0].meta, SelectionRectKind::Text);
        assert!(rects[0].rect.width > 0.0);
        assert!(rects[0].rect.height > 0.0);

        let (entry, line) = first_line_for_para(&index, &para_id).expect("must find line");
        let expected_x = entry.rect.x + super::grapheme::x_at_offset(line, &from_pos);
        let expected_right = entry.rect.x + super::grapheme::x_at_offset(line, &to_pos);
        assert!(
            (rects[0].rect.x - expected_x).abs() < 0.1,
            "rect.x must equal entry.rect.x + x_at_offset(from): got {}, expected {}",
            rects[0].rect.x,
            expected_x
        );
        assert!(
            (rects[0].rect.x + rects[0].rect.width - expected_right).abs() < 0.1,
            "rect right edge must equal entry.rect.x + x_at_offset(to)"
        );
    }

    #[test]
    fn multi_line_span() {
        let text = "abcdefghijklmnopqrstuvwxyzabcdef";
        let (doc, _root, para) = simple_para_doc(text);
        let (pd, index) = build_index(&doc, 40.0);
        let view = DocView::new(&pd);
        let para_id = para;

        let from_pos = Position {
            node: para_id,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let to_pos = Position {
            node: para_id,
            offset: text.len(),
            affinity: Affinity::Upstream,
        };
        let sel = Selection::new(from_pos, to_pos);
        let rsel = sel.resolve(&view).expect("must resolve");

        let rects = selection_rects(&index, &rsel);
        let text_rects: Vec<_> = rects
            .iter()
            .filter(|r| r.meta == SelectionRectKind::Text)
            .collect();
        assert!(
            text_rects.len() >= 2,
            "wrapped paragraph must yield ≥2 text rects (phase machine), got {:?}",
            rects
        );
    }

    #[test]
    fn atom_in_selection() {
        let root = Dot::ROOT;
        let hr = Dot::new(2, 1);
        let para = Dot::new(2, 2);
        let items = vec![
            (
                hr,
                SeqItem::BlockAtom {
                    leaf: AtomLeaf::HorizontalRule {
                        variant: HorizontalRuleVariant::default(),
                    },
                    parents: vec![root],
                },
            ),
            (
                para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(2, 3), SeqItem::Char('x')),
        ];
        let doc = logs(&items);
        let (pd, index) = build_index(&doc, 400.0);
        let view = DocView::new(&pd);
        let root_id = root;
        let para_id = para;

        let covering = Selection::new(
            Position {
                node: root_id,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node: root_id,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        );
        let rsel = covering.resolve(&view).expect("must resolve covering");
        let rects = selection_rects(&index, &rsel);
        assert!(
            rects.iter().any(|r| r.meta == SelectionRectKind::Atom),
            "selection covering HR atom must yield Atom rect, got {:?}",
            rects
        );

        let not_covering = Selection::new(
            Position {
                node: para_id,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node: para_id,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        );
        let rsel2 = not_covering
            .resolve(&view)
            .expect("must resolve non-covering");
        let rects2 = selection_rects(&index, &rsel2);
        assert!(
            !rects2.iter().any(|r| r.meta == SelectionRectKind::Atom),
            "selection not covering HR must not yield Atom rect, got {:?}",
            rects2
        );
    }

    #[test]
    fn cell_rect_block() {
        let root = Dot::ROOT;
        let table = Dot::new(3, 1);
        let row0 = Dot::new(3, 2);
        let cell00 = Dot::new(3, 3);
        let cell01 = Dot::new(3, 4);
        let mut counter = 10u64;
        let mut next = || {
            let d = Dot::new(3, counter);
            counter += 1;
            d
        };
        let items = vec![
            (
                table,
                SeqItem::Block {
                    node_type: NodeType::Table,
                    parents: vec![root],
                },
            ),
            (
                row0,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                },
            ),
            (
                cell00,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row0],
                },
            ),
            (
                next(),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row0, cell00],
                },
            ),
            (
                cell01,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row0],
                },
            ),
            (
                next(),
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row0, cell01],
                },
            ),
        ];
        let doc = logs(&items);
        let (pd, index) = build_index(&doc, 400.0);
        let view = DocView::new(&pd);
        let row0_id = row0;
        let cell00_id = cell00;
        let cell01_id = cell01;

        let cell_rect_sel = Selection::new(
            Position {
                node: row0_id,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node: row0_id,
                offset: 2,
                affinity: Affinity::Downstream,
            },
        );
        let rsel = cell_rect_sel
            .resolve(&view)
            .expect("must resolve cell rect selection");

        assert!(
            rsel.as_cell_rect().is_some(),
            "selection must be recognised as a cell rect"
        );

        let rects = selection_rects(&index, &rsel);
        assert!(
            rects.iter().any(|r| r.meta == SelectionRectKind::Block),
            "cell rect selection must yield Block rects, got {:?}",
            rects
        );

        let cell_ids: Vec<_> = rsel
            .as_cell_rect()
            .unwrap()
            .cells()
            .into_iter()
            .map(|c| c.id())
            .collect();
        assert!(cell_ids.contains(&cell00_id), "cell00 must be in cell ids");
        assert!(cell_ids.contains(&cell01_id), "cell01 must be in cell ids");
    }

    #[test]
    fn endpoints() {
        let (doc, _root, para) = simple_para_doc("Hello");
        let (pd, index) = build_index(&doc, 400.0);
        let view = DocView::new(&pd);
        let para_id = para;

        let from_pos = Position {
            node: para_id,
            offset: 1,
            affinity: Affinity::Downstream,
        };
        let to_pos = Position {
            node: para_id,
            offset: 4,
            affinity: Affinity::Upstream,
        };
        let sel = Selection::new(from_pos, to_pos);
        let rsel = sel.resolve(&view).expect("must resolve");

        let eps = selection_endpoints(&index, &rsel)
            .expect("non-collapsed selection must have endpoints");

        assert_eq!(
            eps.from_position.node, para_id,
            "from_position.node must equal the from Dot"
        );
        assert_eq!(
            eps.to_position.node, para_id,
            "to_position.node must equal the to Dot"
        );
        assert_eq!(eps.from.rect.width, 0.0, "from endpoint must be zero-width");
        assert_eq!(eps.to.rect.width, 0.0, "to endpoint must be zero-width");

        let rects = selection_rects(&index, &rsel);
        let first = rects.first().unwrap();
        let last = rects.last().unwrap();
        assert!(
            (eps.from.rect.x - first.rect.x).abs() < 0.1,
            "from.rect.x must be at the left edge of the first rect"
        );
        assert!(
            (eps.to.rect.x - (last.rect.x + last.rect.width)).abs() < 0.1,
            "to.rect.x must be at the right edge of the last rect"
        );
    }

    #[test]
    fn hit_test() {
        let (doc, _root, para) = simple_para_doc("Hello world");
        let (pd, index) = build_index(&doc, 400.0);
        let view = DocView::new(&pd);
        let para_id = para;

        let from_pos = Position {
            node: para_id,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let to_pos = Position {
            node: para_id,
            offset: 5,
            affinity: Affinity::Upstream,
        };
        let sel = Selection::new(from_pos, to_pos);
        let rsel = sel.resolve(&view).expect("must resolve");

        let rect = selection_rects(&index, &rsel)[0].rect;

        assert!(
            selection_hit_test(
                &index,
                &rsel,
                0,
                rect.x + rect.width * 0.5,
                rect.y + rect.height * 0.5
            ),
            "point inside selection rect must hit"
        );

        assert!(
            !selection_hit_test(
                &index,
                &rsel,
                0,
                rect.x + rect.width + 50.0,
                rect.y + rect.height * 0.5
            ),
            "point clearly outside selection rect must not hit"
        );

        assert!(
            !selection_hit_test(
                &index,
                &rsel,
                1,
                rect.x + rect.width * 0.5,
                rect.y + rect.height * 0.5
            ),
            "point inside rect on wrong page must not hit"
        );
    }
}
