use editor_model::DocView;
use editor_state::Affinity;
use editor_state::{Position, ResolvedPosition, Selection};

use super::layout_index::{LayoutEntry, LayoutIndex, LayoutPoint};
use super::{grapheme, hard_break, paragraph_break};
use crate::paginate::types::{
    ChildAttachment, LayoutAtom, LayoutContent, LayoutLine, LayoutNode, SpacingKind,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtendingHit {
    pub selection: Selection,
    pub source: ExtendingHitSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtendingHitSource {
    Exact,
    Fallback,
}

impl ExtendingHit {
    fn exact(selection: Selection) -> Self {
        Self {
            selection,
            source: ExtendingHitSource::Exact,
        }
    }

    fn fallback(selection: Selection) -> Self {
        Self {
            selection,
            source: ExtendingHitSource::Fallback,
        }
    }
}

pub(crate) fn hit_test(
    layout_index: &LayoutIndex,
    page_idx: usize,
    x: f32,
    y: f32,
) -> Option<Selection> {
    let point = layout_index.point(page_idx, x, y)?;
    let scope = layout_index.container_scope(point);
    let in_scope = |entry: &LayoutEntry| {
        scope.is_none_or(|scope| layout_index.entry_is_in_scope(entry, scope))
    };

    layout_index
        .exact_entry(point, |entry, node| {
            in_scope(entry) && is_text_or_atom_hit_entry(entry, node)
        })
        .or_else(|| {
            layout_index.closest_entry(point, |entry, node| {
                in_scope(entry) && is_text_or_atom_hit_entry(entry, node)
            })
        })
        .and_then(|entry| text_or_atom_selection_for_entry(layout_index, entry, point.x))
}

pub(crate) fn hit_test_extending(
    layout_index: &LayoutIndex,
    view: &DocView,
    anchor: &Position,
    page_idx: usize,
    x: f32,
    y: f32,
) -> Option<ExtendingHit> {
    let point = layout_index.point(page_idx, x, y)?;
    let anchor = anchor.resolve(view)?;
    let scope = layout_index.container_scope(point);

    drag_exact_selection_at(layout_index, view, &anchor, point, scope)
        .map(ExtendingHit::exact)
        .or_else(|| {
            drag_boundary_fallback(layout_index, view, &anchor, point, scope)
                .map(ExtendingHit::fallback)
        })
}

fn drag_exact_selection_at(
    layout_index: &LayoutIndex,
    view: &DocView,
    anchor: &ResolvedPosition,
    point: LayoutPoint,
    scope: Option<&LayoutEntry>,
) -> Option<Selection> {
    let is_scoped_drag_exact_hit_entry = |entry: &LayoutEntry, node: &LayoutNode| {
        is_drag_exact_hit_entry(entry, node)
            && scope.is_none_or(|scope| layout_index.entry_is_in_scope(entry, scope))
    };

    let entry = layout_index.exact_entry(point, is_scoped_drag_exact_hit_entry)?;
    drag_exact_selection_for_entry(layout_index, view, anchor, entry, point)
}

fn drag_exact_selection_for_entry(
    layout_index: &LayoutIndex,
    view: &DocView,
    anchor: &ResolvedPosition,
    entry: &LayoutEntry,
    point: LayoutPoint,
) -> Option<Selection> {
    hard_break::drag_selection_for_entry(layout_index, view, entry, point)
        .or_else(|| {
            paragraph_break::drag_selection_for_entry(layout_index, view, anchor, entry, point)
        })
        .or_else(|| match entry.content(layout_index)? {
            LayoutContent::Line(_) | LayoutContent::Atom(_) => {
                text_or_atom_selection_for_entry(layout_index, entry, point.x)
            }
            LayoutContent::Box(b) if b.style.monolithic => b.attachment.as_ref().map(select_unit),
            LayoutContent::Box(_) | LayoutContent::Spacing(_) => None,
        })
}

fn is_text_or_atom_hit_entry(_entry: &LayoutEntry, node: &LayoutNode) -> bool {
    matches!(
        node.content,
        LayoutContent::Line(_) | LayoutContent::Atom(_)
    )
}

fn is_drag_exact_hit_entry(_entry: &LayoutEntry, node: &LayoutNode) -> bool {
    match &node.content {
        LayoutContent::Line(_)
        | LayoutContent::Atom(_)
        | LayoutContent::Spacing(SpacingKind::Gap { .. }) => true,
        LayoutContent::Box(b) => b.style.monolithic && b.attachment.is_some(),
        LayoutContent::Spacing(SpacingKind::Fill) => false,
    }
}

fn text_or_atom_selection_for_entry(
    layout_index: &LayoutIndex,
    entry: &LayoutEntry,
    x: f32,
) -> Option<Selection> {
    match entry.content(layout_index)? {
        LayoutContent::Line(line) => {
            Some(Selection::collapsed(position_in_line(line, &entry.rect, x)))
        }
        LayoutContent::Atom(atom) => Some(select_atom(atom)),
        LayoutContent::Box(_) | LayoutContent::Spacing(_) => None,
    }
}

fn drag_boundary_fallback(
    layout_index: &LayoutIndex,
    view: &DocView,
    anchor: &ResolvedPosition,
    point: LayoutPoint,
    scope: Option<&LayoutEntry>,
) -> Option<Selection> {
    let mut inside: Option<DragFallbackCandidate> = None;
    let mut before: Option<DragFallbackCandidate> = None;
    let mut after: Option<DragFallbackCandidate> = None;

    for entry in layout_index.entries_on_page(point.page_idx) {
        if scope.is_some_and(|scope| !layout_index.entry_is_in_scope(entry, scope)) {
            continue;
        }
        let Some(candidate) = drag_fallback_candidate(layout_index, entry, point) else {
            continue;
        };
        let slot = if point.y >= entry.rect.y && point.y < entry.rect.bottom() {
            &mut inside
        } else if entry.rect.bottom() <= point.y {
            &mut before
        } else if entry.rect.y >= point.y {
            &mut after
        } else {
            continue;
        };
        if candidate.is_better_than(slot.as_ref()) {
            *slot = Some(candidate);
        }
    }

    if let Some(candidate) = inside {
        return Some(candidate.selection);
    }

    let prefer_before = after
        .as_ref()
        .and_then(|candidate| candidate.start.resolve(view))
        .is_none_or(|after_start| anchor < &after_start);
    let candidate = if prefer_before {
        before.or(after)
    } else {
        after.or(before)
    };
    candidate.map(|candidate| candidate.selection)
}

struct DragFallbackCandidate {
    distance: (f32, f32),
    start: Position,
    selection: Selection,
}

impl DragFallbackCandidate {
    fn new(entry: &LayoutEntry, point: LayoutPoint, start: Position, selection: Selection) -> Self {
        Self {
            distance: distance_key(&entry.rect, point.x, point.y),
            start,
            selection,
        }
    }

    fn is_better_than(&self, other: Option<&Self>) -> bool {
        other.is_none_or(|best| compare_distance_key(self.distance, best.distance).is_lt())
    }
}

fn drag_fallback_candidate(
    layout_index: &LayoutIndex,
    entry: &LayoutEntry,
    point: LayoutPoint,
) -> Option<DragFallbackCandidate> {
    match entry.content(layout_index)? {
        LayoutContent::Line(line) => {
            let start = position_in_line(line, &entry.rect, entry.rect.x);
            let end = grapheme::last_position_in_line(line);
            let pos = if point.y < entry.rect.y {
                start
            } else if point.y >= entry.rect.bottom() {
                end
            } else {
                position_in_line(line, &entry.rect, point.x)
            };
            Some(DragFallbackCandidate::new(
                entry,
                point,
                start,
                Selection::collapsed(pos),
            ))
        }
        LayoutContent::Atom(atom) => {
            let hit = select_atom(atom);
            Some(DragFallbackCandidate::new(entry, point, hit.anchor, hit))
        }
        LayoutContent::Box(b) if b.style.monolithic && b.attachment.is_some() => {
            if point.y >= entry.rect.y && point.y < entry.rect.bottom() {
                return None;
            }
            let hit = select_unit(b.attachment.as_ref()?);
            Some(DragFallbackCandidate::new(entry, point, hit.anchor, hit))
        }
        LayoutContent::Box(_) | LayoutContent::Spacing(_) => None,
    }
}

fn position_in_line(line: &LayoutLine, rect: &editor_common::Rect, x: f32) -> Position {
    grapheme::position_at_x(line, x - rect.x)
}

fn select_atom(atom: &LayoutAtom) -> Selection {
    Selection::new(
        Position {
            node: atom.attachment.parent,
            offset: atom.attachment.index,
            affinity: Affinity::Downstream,
        },
        Position {
            node: atom.attachment.parent,
            offset: atom.attachment.index + 1,
            affinity: Affinity::Upstream,
        },
    )
}

fn select_unit(attachment: &ChildAttachment) -> Selection {
    Selection::new(
        Position {
            node: attachment.parent,
            offset: attachment.index,
            affinity: Affinity::Downstream,
        },
        Position {
            node: attachment.parent,
            offset: attachment.index + 1,
            affinity: Affinity::Upstream,
        },
    )
}

// Points that resolve to the caret through the exact-entry path only: the
// closest-entry margin catchment is intentionally excluded, so containment in
// these rects implies `hit_test == caret` but not the converse.
pub(crate) fn cursor_hit_rects(
    layout_index: &LayoutIndex,
    view: &DocView,
    selection: &editor_state::Selection,
) -> Vec<crate::page::PageRect> {
    if !selection.is_collapsed() {
        return Vec::new();
    }
    let Some(caret) = selection.head.resolve(view) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for entry in layout_index.entries_for_node(&selection.head.node) {
        let Some(LayoutContent::Line(line)) = entry.content(layout_index) else {
            continue;
        };
        let Some(page_rect) = layout_index.page_rect(entry.rect) else {
            continue;
        };

        let base = entry.rect.x;
        let mut xs: Vec<f32> = vec![entry.rect.x, entry.rect.right(), base + line.empty_caret_x];
        for gap in &line.tab_gaps {
            xs.push(base + gap.x);
            xs.push(base + gap.x + gap.width / 2.0);
            xs.push(base + gap.x + gap.width);
        }
        for run in &line.glyph_runs {
            xs.push(base + run.x);
            xs.push(base + run.x + run.width);
            let mut acc = run.x;
            for g in &run.graphemes {
                xs.push(base + acc + g.advance / 2.0);
                acc += g.advance;
                xs.push(base + acc);
            }
        }
        xs.retain(|x| x.is_finite());
        for x in &mut xs {
            *x = x.clamp(entry.rect.x, entry.rect.right());
        }
        xs.sort_by(f32::total_cmp);
        xs.dedup();

        let mut intervals: Vec<(f32, f32)> = Vec::new();
        for pair in xs.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if b <= a {
                continue;
            }
            let sample = a + (b - a) / 2.0;
            let position = position_in_line(line, &entry.rect, sample);
            if position.resolve(view).is_some_and(|hit| hit == caret) {
                match intervals.last_mut() {
                    Some(last) if last.1 == a => last.1 = b,
                    _ => intervals.push((a, b)),
                }
            }
        }
        if intervals.is_empty() {
            continue;
        }

        let entry_area = entry.rect.width * entry.rect.height;
        let occluders: Vec<editor_common::Rect> = layout_index
            .entries_on_page(page_rect.page_idx)
            .into_iter()
            .filter(|other| !std::ptr::eq(*other, entry))
            .filter(|other| {
                other.rect.y < entry.rect.bottom() && other.rect.bottom() > entry.rect.y
            })
            .filter(|other| other.rect.width * other.rect.height <= entry_area)
            .filter(|other| {
                other
                    .node(layout_index)
                    .is_some_and(|node| is_text_or_atom_hit_entry(other, node))
            })
            .map(|other| other.rect)
            .collect();

        let band_top = entry.rect.y;
        let band_bottom = entry.rect.bottom();
        let mut y_cuts = vec![band_top, band_bottom];
        for occluder in &occluders {
            y_cuts.push(occluder.y.clamp(band_top, band_bottom));
            y_cuts.push(occluder.bottom().clamp(band_top, band_bottom));
        }
        y_cuts.sort_by(f32::total_cmp);
        y_cuts.dedup();

        for band in y_cuts.windows(2) {
            let (y0, y1) = (band[0], band[1]);
            if y1 <= y0 {
                continue;
            }
            let cuts: Vec<(f32, f32)> = occluders
                .iter()
                .filter(|occluder| occluder.y < y1 && occluder.bottom() > y0)
                .map(|occluder| (occluder.x, occluder.right()))
                .collect();
            for (lo, hi) in subtract_intervals(intervals.clone(), &cuts) {
                if hi > lo {
                    out.push(crate::page::PageRect::new(
                        page_rect.page_idx,
                        editor_common::Rect::from_xywh(
                            lo,
                            page_rect.rect.y + (y0 - entry.rect.y),
                            hi - lo,
                            y1 - y0,
                        ),
                    ));
                }
            }
        }
    }
    out
}

fn subtract_intervals(intervals: Vec<(f32, f32)>, cuts: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut result = intervals;
    for &(cut_lo, cut_hi) in cuts {
        if cut_hi <= cut_lo {
            continue;
        }
        let mut next = Vec::with_capacity(result.len() + 1);
        for (lo, hi) in result {
            if cut_hi <= lo || cut_lo >= hi {
                next.push((lo, hi));
                continue;
            }
            if cut_lo > lo {
                next.push((lo, cut_lo));
            }
            if cut_hi < hi {
                next.push((cut_hi, hi));
            }
        }
        result = next;
    }
    result
}

fn compare_distance_key(a: (f32, f32), b: (f32, f32)) -> std::cmp::Ordering {
    match a.0.total_cmp(&b.0) {
        std::cmp::Ordering::Equal => a.1.total_cmp(&b.1),
        ordering => ordering,
    }
}

fn distance_key(rect: &editor_common::Rect, x: f32, y: f32) -> (f32, f32) {
    (
        axis_distance(rect.y, rect.bottom(), y),
        axis_distance(rect.x, rect.right(), x),
    )
}

fn axis_distance(start: f32, end: f32, value: f32) -> f32 {
    if value < start {
        start - value
    } else if value > end {
        value - end
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use editor_common::EdgeInsets;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        AliasLog, AtomLeaf, DocLogs, DocView, HorizontalRuleVariant, Modifier, ModifierAttrLog,
        ModifierAttrOp, NodeAttrLog, NodeType, ProjectedDoc, SeqItem, SpanLog, project_document,
    };
    use editor_state::Affinity;
    use editor_state::Position;

    use crate::measure::context::MeasureContext;
    use crate::measure::nodes::dispatch::measure_node;
    use crate::measure::types::MeasuredTree;
    use crate::paginate::paginator::Paginator;
    use crate::paginate::types::{LayoutContent, SpacingKind};
    use crate::query::layout_index::LayoutIndex;
    use editor_resource::Resource;

    use super::super::grapheme;
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
            node_carries: ModifierAttrLog::new(),
            aliases: AliasLog::new(),
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

    fn para_doc(text: &str, width: f32) -> (ProjectedDoc, Dot, LayoutIndex) {
        let root = Dot::ROOT;
        let para = Dot::new(10, 1);
        let mut items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
            },
        )];
        for (i, ch) in text.chars().enumerate() {
            items.push((Dot::new(10, 2 + i as u64), SeqItem::Char(ch)));
        }
        let doc = logs(&items);
        let para_id = para;
        let (pd, index) = build_index(&doc, width);
        (pd, para_id, index)
    }

    fn para_items_doc(children: Vec<SeqItem>, width: f32) -> (ProjectedDoc, Dot, LayoutIndex) {
        let root = Dot::ROOT;
        let para = Dot::new(14, 1);
        let mut items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
            },
        )];
        for (i, child) in children.into_iter().enumerate() {
            items.push((Dot::new(14, 2 + i as u64), child));
        }
        let doc = logs(&items);
        let para_id = para;
        let (pd, index) = build_index(&doc, width);
        (pd, para_id, index)
    }

    fn hr_doc(width: f32) -> (ProjectedDoc, Dot, LayoutIndex) {
        let root = Dot::ROOT;
        let hr = Dot::new(11, 1);
        let p = Dot::new(11, 2);
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
                p,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(11, 3), SeqItem::Char('x')),
        ];
        let doc = logs(&items);
        let root_id = root;
        let (pd, index) = build_index(&doc, width);
        (pd, root_id, index)
    }

    fn para_with_hard_break(text: &str, width: f32) -> (ProjectedDoc, Dot, LayoutIndex) {
        let root = Dot::ROOT;
        let para = Dot::new(12, 1);
        let mut items = vec![(
            para,
            SeqItem::Block {
                node_type: NodeType::Paragraph,
                parents: vec![root],
                attrs: vec![],
            },
        )];
        for (i, ch) in text.chars().enumerate() {
            items.push((Dot::new(12, 2 + i as u64), SeqItem::Char(ch)));
        }
        let hb_idx = 2 + text.len() as u64;
        items.push((Dot::new(12, hb_idx), SeqItem::Atom(AtomLeaf::HardBreak)));
        let doc = logs(&items);
        let para_id = para;
        let (pd, index) = build_index(&doc, width);
        (pd, para_id, index)
    }

    fn table_with_short_cell_and_wrapped_neighbor(
        width: f32,
    ) -> (ProjectedDoc, Dot, Dot, Dot, LayoutIndex) {
        let root = Dot::ROOT;
        let table = Dot::new(15, 1);
        let row = Dot::new(15, 2);
        let left_cell = Dot::new(15, 3);
        let left_para = Dot::new(15, 4);
        let right_cell = Dot::new(15, 20);
        let right_para = Dot::new(15, 21);

        let mut items = vec![
            (
                table,
                SeqItem::Block {
                    node_type: NodeType::Table,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (
                row,
                SeqItem::Block {
                    node_type: NodeType::TableRow,
                    parents: vec![root, table],
                    attrs: vec![],
                },
            ),
            (
                left_cell,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row],
                    attrs: vec![],
                },
            ),
            (
                left_para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row, left_cell],
                    attrs: vec![],
                },
            ),
            (Dot::new(15, 5), SeqItem::Char('x')),
            (
                right_cell,
                SeqItem::Block {
                    node_type: NodeType::TableCell,
                    parents: vec![root, table, row],
                    attrs: vec![],
                },
            ),
            (
                right_para,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root, table, row, right_cell],
                    attrs: vec![],
                },
            ),
        ];
        for (i, ch) in "right cell text wraps onto a second visual line"
            .chars()
            .enumerate()
        {
            items.push((Dot::new(15, 22 + i as u64), SeqItem::Char(ch)));
        }

        let doc = logs(&items);
        let (pd, index) = build_index(&doc, width);
        (pd, left_cell, left_para, right_para, index)
    }

    fn two_para_gap_doc(width: f32) -> (ProjectedDoc, Dot, Dot, LayoutIndex) {
        let root = Dot::ROOT;
        let p1 = Dot::new(13, 1);
        let p2 = Dot::new(13, 2);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(13, 3), SeqItem::Char('A')),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                    attrs: vec![],
                },
            ),
            (Dot::new(13, 4), SeqItem::Char('B')),
        ];
        let mut doc = logs(&items);
        doc.block_modifiers = ModifierAttrLog::new()
            .apply(
                Dot::ROOT,
                ModifierAttrOp::SetModifier {
                    target: root,
                    modifier: Modifier::BlockGap { value: 100 },
                },
            )
            .unwrap();
        let para1_id = p1;
        let para2_id = p2;
        let (pd, index) = build_index(&doc, width);
        (pd, para1_id, para2_id, index)
    }

    fn first_line_for_para<'a>(
        index: &'a LayoutIndex,
        para_id: &Dot,
    ) -> Option<(
        &'a crate::query::layout_index::LayoutEntry,
        &'a crate::paginate::types::LayoutLine,
    )> {
        for entry in index.entries() {
            if let Some(node) = entry.node(index)
                && let LayoutContent::Line(line) = &node.content
                && &line.node == para_id
                && line.offset_range.is_some()
            {
                return Some((entry, line));
            }
        }
        None
    }

    fn line_entries_for_para<'a>(
        index: &'a LayoutIndex,
        para_id: &Dot,
    ) -> Vec<(
        &'a crate::query::layout_index::LayoutEntry,
        &'a crate::paginate::types::LayoutLine,
    )> {
        index
            .entries()
            .filter_map(|entry| {
                let node = entry.node(index)?;
                let LayoutContent::Line(line) = &node.content else {
                    return None;
                };
                (line.node == *para_id && line.offset_range.is_some()).then_some((entry, line))
            })
            .collect()
    }

    #[test]
    fn hit_test_text_line_caret() {
        let (_pd, para_id, index) = para_doc("Hello", 400.0);

        let (entry, line) = first_line_for_para(&index, &para_id).expect("must find line");

        let local_x = entry.rect.width / 2.0;
        let abs_x = entry.rect.x + local_x;
        let page_y = entry.rect.y - index.pages()[0].y_start + entry.rect.height / 2.0;

        let result = hit_test(&index, 0, abs_x, page_y);
        assert!(result.is_some(), "hit_test must return Some for text line");

        let sel = result.unwrap();
        assert_eq!(
            sel.anchor, sel.head,
            "click must return a collapsed selection"
        );
        assert_eq!(
            sel.anchor.node, para_id,
            "anchor node must be the para elem"
        );

        let expected_pos = grapheme::position_at_x(line, local_x);
        assert_eq!(
            sel.anchor.offset, expected_pos.offset,
            "anchor offset must match position_at_x for local_x"
        );
    }

    #[test]
    fn hit_test_after_tab_only_line_lands_after_last_tab() {
        let (_pd, para_id, index) = para_items_doc(
            vec![SeqItem::Atom(AtomLeaf::Tab), SeqItem::Atom(AtomLeaf::Tab)],
            400.0,
        );
        let (entry, line) = first_line_for_para(&index, &para_id).expect("must find tab line");
        let last_gap = line.tab_gaps.last().expect("tab line must expose tab gap");
        let local_x = last_gap.x + last_gap.width + 10.0;
        let abs_x = entry.rect.x + local_x;
        let page_y = entry.rect.y - index.pages()[0].y_start + entry.rect.height / 2.0;

        let sel = hit_test(&index, 0, abs_x, page_y).expect("click must hit tab line");

        assert_eq!(sel.anchor, sel.head);
        assert_eq!(sel.anchor.node, para_id);
        assert_eq!(sel.anchor.offset, 2);
        assert_eq!(sel.anchor.affinity, Affinity::Upstream);
    }

    #[test]
    fn hit_test_inside_short_table_cell_stays_in_that_cell_when_neighbor_wraps() {
        let (_pd, left_cell, left_para, right_para, index) =
            table_with_short_cell_and_wrapped_neighbor(180.0);

        let left_cell_rect = index
            .box_rect(&left_cell)
            .expect("left cell must have a box");
        let right_lines = line_entries_for_para(&index, &right_para);
        assert!(
            right_lines.len() >= 2,
            "right cell text must wrap to at least two lines"
        );
        let (second_right_line, _) = right_lines[1];

        let x = left_cell_rect.x + left_cell_rect.width / 2.0;
        let page_y = second_right_line.rect.y - index.pages()[0].y_start
            + second_right_line.rect.height / 2.0;

        let sel = hit_test(&index, 0, x, page_y).expect("click inside table cell must hit");

        assert_eq!(sel.anchor, sel.head);
        assert_eq!(
            sel.anchor.node, left_para,
            "click inside the left cell must resolve within that cell"
        );
    }

    #[test]
    fn hit_test_side_gutter_routes_to_nearest_cell_scope_in_row() {
        let (_pd, left_cell, left_para, right_para, index) =
            table_with_short_cell_and_wrapped_neighbor(180.0);

        let left_cell_rect = index
            .box_rect(&left_cell)
            .expect("left cell must have a box");
        let right_lines = line_entries_for_para(&index, &right_para);
        assert!(
            right_lines.len() >= 2,
            "right cell text must wrap to at least two lines"
        );
        let (second_right_line, _) = right_lines[1];

        let x = left_cell_rect.x - 0.25;
        let page_y = second_right_line.rect.y - index.pages()[0].y_start
            + second_right_line.rect.height / 2.0;

        let sel = hit_test(&index, 0, x, page_y).expect("gutter click in row must hit");

        assert_eq!(sel.anchor, sel.head);
        assert_eq!(
            sel.anchor.node, left_para,
            "row side gutter must route to the nearest cell scope, not the y-nearest neighboring cell line"
        );
    }

    #[test]
    fn hit_test_atom_unit() {
        let (_pd, root_id, index) = hr_doc(400.0);

        let atom_entry = index.entries().find(|e| {
            matches!(
                e.content(&index),
                Some(LayoutContent::Atom(a)) if a.attachment.parent == root_id
            )
        });
        assert!(atom_entry.is_some(), "must find HR atom entry");
        let atom_entry = atom_entry.unwrap();

        let mid_x = atom_entry.rect.x + atom_entry.rect.width / 2.0;
        let page_y = atom_entry.rect.y - index.pages()[0].y_start + atom_entry.rect.height / 2.0;

        let result = hit_test(&index, 0, mid_x, page_y);
        assert!(result.is_some(), "click on HR atom must return Some");
        let sel = result.unwrap();

        assert_eq!(
            sel.anchor.node, root_id,
            "anchor node must be HR attachment parent"
        );
        assert_eq!(
            sel.anchor.offset, 0,
            "anchor offset must be HR child slot index"
        );
        assert_eq!(sel.head.offset, 1, "head offset must be idx+1");
        assert_ne!(sel.anchor, sel.head, "atom selection must not be collapsed");
    }

    #[test]
    fn hit_test_extending_hard_break_routes() {
        let (pd, para_id, index) = para_with_hard_break("Hi", 400.0);
        let view = DocView::new(&pd);

        let (entry, line) = first_line_for_para(&index, &para_id).expect("must find line");
        let hb_index = line.offset_range.as_ref().unwrap().end;

        let anchor_pos = Position {
            node: para_id,
            offset: hb_index,
            affinity: Affinity::Downstream,
        };

        let hb_glyph_x = entry.rect.x + grapheme::x_at_offset(line, &anchor_pos);
        let hb_glyph_width = entry.rect.height * 0.15;
        let x_mid = hb_glyph_x + hb_glyph_width / 2.0;
        let click_x = x_mid + 1.0;

        let page_y_start = index.pages()[0].y_start;
        let page_y = entry.rect.y - page_y_start + entry.rect.height / 2.0;

        let drag_anchor = Position {
            node: para_id,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let result = hit_test_extending(&index, &view, &drag_anchor, 0, click_x, page_y);

        assert!(
            result.is_some(),
            "hit_test_extending must return Some over hard-break glyph"
        );
        let result = result.unwrap();
        assert_eq!(result.source, ExtendingHitSource::Exact);
        let result_sel = result.selection;
        assert_eq!(
            result_sel.anchor.node, para_id,
            "selection must be within the para"
        );
        assert_eq!(
            result_sel.anchor.offset, hb_index,
            "anchor must be at hard-break start offset"
        );
        assert_eq!(
            result_sel.head.offset,
            hb_index + 1,
            "head must be at hard-break end offset"
        );
    }

    #[test]
    fn hit_test_extending_cell_padding_is_fallback() {
        let (pd, left_cell, left_para, _right_para, index) =
            table_with_short_cell_and_wrapped_neighbor(180.0);
        let view = DocView::new(&pd);
        let cell_rect = index
            .box_rect(&left_cell)
            .expect("left cell must have a box");
        let (line_entry, _) =
            first_line_for_para(&index, &left_para).expect("left para has a line");
        let anchor = Position {
            node: left_para,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let x = cell_rect.x + cell_rect.width / 2.0;
        let page_y = (cell_rect.y + line_entry.rect.y) / 2.0 - index.pages()[0].y_start;

        let hit = hit_test_extending(&index, &view, &anchor, 0, x, page_y)
            .expect("cell padding must still produce an extending hit");

        assert_eq!(hit.source, ExtendingHitSource::Fallback);
        assert_eq!(
            hit.selection.anchor, hit.selection.head,
            "padding fallback should produce a collapsed hit"
        );
        assert_eq!(
            hit.selection.anchor.node, left_para,
            "padding fallback should stay in the scoped cell"
        );
    }

    #[test]
    fn hit_test_extending_gap_tiebreak_is_exact() {
        let (pd, para1_id, para2_id, index) = two_para_gap_doc(400.0);
        let view = DocView::new(&pd);

        let gap_entry = index.entries().find(|e| {
            matches!(
                e.content(&index),
                Some(LayoutContent::Spacing(SpacingKind::Gap { .. }))
            )
        });

        let gap_entry = gap_entry.expect("two_para_gap_doc must produce a Gap spacing entry");

        let gap_mid_y_page =
            gap_entry.rect.y - index.pages()[0].y_start + gap_entry.rect.height / 2.0;
        let x = 0.0;

        let anchor_before = Position {
            node: para1_id,
            offset: 0,
            affinity: Affinity::Downstream,
        };

        let para2_children = view.node(para2_id).unwrap().children().count();
        let anchor_after = Position {
            node: para2_id,
            offset: para2_children,
            affinity: Affinity::Upstream,
        };

        let sel_with_before_anchor =
            hit_test_extending(&index, &view, &anchor_before, 0, x, gap_mid_y_page);
        let sel_with_after_anchor =
            hit_test_extending(&index, &view, &anchor_after, 0, x, gap_mid_y_page);

        assert!(
            sel_with_before_anchor.is_some(),
            "before-anchor drag must return Some"
        );
        assert!(
            sel_with_after_anchor.is_some(),
            "after-anchor drag must return Some"
        );

        let hit_before = sel_with_before_anchor.unwrap();
        let hit_after = sel_with_after_anchor.unwrap();
        assert_eq!(hit_before.source, ExtendingHitSource::Exact);
        assert_eq!(hit_after.source, ExtendingHitSource::Exact);

        let sel_before = hit_before.selection;
        let sel_after = hit_after.selection;

        assert_ne!(
            (
                sel_before.anchor.node,
                sel_before.anchor.offset,
                sel_before.head.offset
            ),
            (
                sel_after.anchor.node,
                sel_after.anchor.offset,
                sel_after.head.offset
            ),
            "tie-break must produce different selections for before-anchor vs after-anchor"
        );
    }

    fn oracle_cursor_hit(
        index: &LayoutIndex,
        view: &DocView,
        selection: &editor_state::Selection,
        page_idx: usize,
        x: f32,
        y: f32,
    ) -> bool {
        if !selection.is_collapsed() {
            return false;
        }
        let Some(hit) = hit_test(index, page_idx, x, y) else {
            return false;
        };
        if !hit.is_collapsed() {
            return false;
        }
        let (Some(current), Some(hit_head)) =
            (selection.head.resolve(view), hit.head.resolve(view))
        else {
            return false;
        };
        current == hit_head
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig {
            cases: 24,
            ..Default::default()
        })]
        #[test]
        fn cursor_hit_rects_agree_with_hit_test(
            width in 150.0f32..500.0,
            texts in proptest::collection::vec(
                proptest::collection::vec(
                    proptest::sample::select(vec!['a', 'w', '한', '글', ' ', '\t', 'i']),
                    0..24,
                ),
                1..4,
            ),
            caret_seed in 0usize..1000,
        ) {
            let root = Dot::ROOT;
            let mut items = Vec::new();
            let mut paras: Vec<(Dot, usize)> = Vec::new();
            for (i, text) in texts.iter().enumerate() {
                let para = Dot::new(10 + i as u64, 1);
                items.push((
                    para,
                    SeqItem::Block {
                        node_type: NodeType::Paragraph,
                        parents: vec![root],
                        attrs: vec![],
                    },
                ));
                for (j, ch) in text.iter().enumerate() {
                    items.push((Dot::new(10 + i as u64, 2 + j as u64), SeqItem::Char(*ch)));
                }
                paras.push((para, text.len()));
            }
            let doc = logs(&items);
            let pd = editor_model::project_document(&doc).unwrap();
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

            let (para, len) = paras[caret_seed % paras.len()];
            let offset = if len == 0 { 0 } else { caret_seed % (len + 1) };
            let selection = editor_state::Selection::collapsed(Position::new(para, offset));
            let rects = cursor_hit_rects(&index, &view, &selection);

            let mut bbox: Option<editor_common::Rect> = None;
            for entry in index.entries() {
                let rect = entry.rect;
                bbox = Some(match bbox {
                    None => rect,
                    Some(prev) => {
                        let x0 = prev.x.min(rect.x);
                        let y0 = prev.y.min(rect.y);
                        editor_common::Rect::from_xywh(
                            x0,
                            y0,
                            prev.right().max(rect.right()) - x0,
                            prev.bottom().max(rect.bottom()) - y0,
                        )
                    }
                });
            }
            let Some(bbox) = bbox else { return Ok(()) };

            let steps = 20;
            for iy in 0..=steps {
                for ix in 0..=steps {
                    let x = bbox.x - 15.0 + (bbox.width + 30.0) * (ix as f32 + 0.37) / (steps as f32 + 1.0);
                    let y = bbox.y - 15.0 + (bbox.height + 30.0) * (iy as f32 + 0.41) / (steps as f32 + 1.0);
                    let in_rects = rects
                        .iter()
                        .any(|pr| pr.page_idx == 0 && pr.rect.contains(x, y));
                    let hit = oracle_cursor_hit(&index, &view, &selection, 0, x, y);
                    proptest::prop_assert!(
                        !in_rects || hit,
                        "unsound at ({x}, {y}): in cursor_hit_rects but hit_test disagrees"
                    );
                    let line_band_count = index
                        .entries()
                        .filter(|entry| {
                            matches!(
                                entry.content(&index),
                                Some(LayoutContent::Line(_))
                            ) && y >= entry.rect.y
                                && y < entry.rect.bottom()
                                && x >= entry.rect.x
                                && x <= entry.rect.right()
                        })
                        .count();
                    proptest::prop_assert!(
                        !(hit && line_band_count == 1) || in_rects,
                        "incomplete at ({x}, {y}): hit_test says caret inside a sole line band but rects miss it"
                    );
                }
            }
        }
    }
}
