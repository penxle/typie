use editor_model::DocView;
use editor_state::Affinity;
use editor_state::{Position, ResolvedPosition, Selection};

use super::layout_index::{LayoutEntry, LayoutIndex, LayoutPoint};
use super::{grapheme, hard_break, paragraph_break};
use crate::paginate::types::{
    ChildAttachment, LayoutAtom, LayoutContent, LayoutLine, LayoutNode, SpacingKind,
};

pub(crate) fn hit_test(
    layout_index: &LayoutIndex,
    page_idx: usize,
    x: f32,
    y: f32,
) -> Option<Selection> {
    let point = layout_index.point(page_idx, x, y)?;
    layout_index
        .exact_entry(point, is_text_or_atom_hit_entry)
        .or_else(|| layout_index.closest_entry(point, is_text_or_atom_hit_entry))
        .and_then(|entry| text_or_atom_selection_for_entry(layout_index, entry, point.x))
}

pub(crate) fn hit_test_extending(
    layout_index: &LayoutIndex,
    view: &DocView,
    anchor: &Position,
    page_idx: usize,
    x: f32,
    y: f32,
) -> Option<Selection> {
    let point = layout_index.point(page_idx, x, y)?;
    let anchor = anchor.resolve(view)?;

    if let Some(entry) = layout_index.exact_entry(point, is_drag_exact_hit_entry) {
        if let Some(selection) =
            hard_break::drag_selection_for_entry(layout_index, view, entry, point)
        {
            return Some(selection);
        }
        if let Some(selection) =
            paragraph_break::drag_selection_for_entry(layout_index, view, &anchor, entry, point)
        {
            return Some(selection);
        }
        return match entry.content(layout_index) {
            Some(LayoutContent::Line(_) | LayoutContent::Atom(_)) => {
                text_or_atom_selection_for_entry(layout_index, entry, point.x)
            }
            Some(LayoutContent::Box(b)) if b.style.monolithic => {
                b.attachment.as_ref().map(select_unit)
            }
            Some(LayoutContent::Spacing(SpacingKind::Gap { .. })) => {
                drag_boundary_fallback(layout_index, view, &anchor, point)
            }
            Some(LayoutContent::Box(_) | LayoutContent::Spacing(SpacingKind::Fill)) | None => None,
        };
    }

    drag_boundary_fallback(layout_index, view, &anchor, point)
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
) -> Option<Selection> {
    let mut inside: Option<DragFallbackCandidate> = None;
    let mut before: Option<DragFallbackCandidate> = None;
    let mut after: Option<DragFallbackCandidate> = None;

    for entry in layout_index.entries_on_page(point.page_idx) {
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
        AtomLeaf, DocLogs, DocView, HorizontalRuleVariant, Modifier, ModifierAttrLog,
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
                },
            ),
            (Dot::new(13, 3), SeqItem::Char('A')),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
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
        let result_sel = result.unwrap();
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
    fn hit_test_extending_boundary_fallback_tiebreak() {
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

        let sel_before = sel_with_before_anchor.unwrap();
        let sel_after = sel_with_after_anchor.unwrap();

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
}
