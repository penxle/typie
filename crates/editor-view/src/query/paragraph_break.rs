pub(crate) struct ParagraphBreakGeometry {
    pub(crate) rect: PageRect,
    pub(crate) line_right: f32,
}

use editor_common::Rect;
use editor_model::{ChildView, DocView, NodeType};
use editor_state::Affinity;
use editor_state::{
    Position, ResolvedPosition, ResolvedSelection, Selection, before_or_same, last_cursor_position,
    paragraph_break_at_end,
};

use crate::page::{LayoutPage, PageRect};
use crate::paginate::types::{LayoutContent, LayoutLine, SpacingKind};

use super::common::page_for_y;
use super::layout_index::{LayoutEntry, LayoutIndex, LayoutPoint};

pub(crate) struct SelectedParagraphBreak {
    pub(crate) selection: Selection,
    pub(crate) geometry: ParagraphBreakGeometry,
}

pub(crate) fn included_in_selection(
    layout_index: &LayoutIndex,
    selection: &ResolvedSelection<'_>,
    y_bounds: Option<(f32, f32)>,
) -> Vec<SelectedParagraphBreak> {
    let mut paragraph_breaks = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for entry in layout_index.entries() {
        if let Some((y_start, y_end)) = y_bounds
            && !entry.overlaps_y_range(y_start, y_end)
        {
            continue;
        }
        let Some(paragraph_break) =
            paragraph_break_for_entry(layout_index, selection.view(), entry)
        else {
            continue;
        };
        if !selection.contains_range(paragraph_break.selection) {
            continue;
        }
        if !seen.insert(super::common::selection_key(&paragraph_break.selection)) {
            continue;
        }
        paragraph_breaks.push(paragraph_break);
    }
    paragraph_breaks
}

fn geometry(
    layout_index: &LayoutIndex,
    paragraph_break: Selection,
    pages: &[LayoutPage],
) -> Option<ParagraphBreakGeometry> {
    let pos = paragraph_break.anchor;
    let entry = layout_index.entry_for_position(&pos)?;
    let LayoutContent::Line(line) = entry.content(layout_index)? else {
        return None;
    };
    geometry_for_line_entry(entry, line, paragraph_break, pages)
}

pub(crate) fn drag_selection_for_entry(
    layout_index: &LayoutIndex,
    view: &DocView,
    anchor: &ResolvedPosition<'_>,
    entry: &LayoutEntry,
    point: LayoutPoint,
) -> Option<Selection> {
    let paragraph_break = paragraph_break_for_entry(layout_index, view, entry)?;
    match entry.content(layout_index)? {
        LayoutContent::Line(_) => {
            let rect = paragraph_break.geometry.rect.rect;
            let page_y = point.y - point.page_y_start;
            let x_mid = rect.x + rect.width / 2.0;
            if paragraph_break.geometry.rect.page_idx == point.page_idx
                && page_y >= rect.y
                && page_y <= rect.bottom()
                && point.x >= x_mid
                && point.x <= paragraph_break.geometry.line_right
            {
                Some(paragraph_break.selection)
            } else {
                None
            }
        }
        LayoutContent::Spacing(SpacingKind::Gap { .. }) => {
            let resolved = paragraph_break.selection.resolve(view)?;
            if before_or_same(&anchor.position(), &resolved.from().position(), view) {
                Some(paragraph_break.selection)
            } else {
                Some(Selection::collapsed(paragraph_break.selection.head))
            }
        }
        LayoutContent::Box(_) | LayoutContent::Atom(_) | LayoutContent::Spacing(_) => None,
    }
}

fn paragraph_break_for_entry(
    layout_index: &LayoutIndex,
    view: &DocView,
    entry: &LayoutEntry,
) -> Option<SelectedParagraphBreak> {
    match entry.content(layout_index)? {
        LayoutContent::Line(line) => {
            let selection = paragraph_break_for_line(view, line)?;
            let geometry = geometry_for_line_entry(entry, line, selection, layout_index.pages())?;
            Some(SelectedParagraphBreak {
                selection,
                geometry,
            })
        }
        LayoutContent::Spacing(SpacingKind::Gap { position }) => {
            let selection = paragraph_break_before_gap_boundary(view, *position)?;
            let geometry = geometry(layout_index, selection, layout_index.pages())?;
            Some(SelectedParagraphBreak {
                selection,
                geometry,
            })
        }
        LayoutContent::Box(_) | LayoutContent::Atom(_) | LayoutContent::Spacing(_) => None,
    }
}

fn paragraph_break_for_line(view: &DocView, line: &LayoutLine) -> Option<Selection> {
    if !line_can_host_visual_paragraph_break(line) {
        return None;
    }
    let line_end = super::grapheme::last_position_in_line(line);
    paragraph_break_at_end(&line_end, view)
}

fn line_can_host_visual_paragraph_break(line: &LayoutLine) -> bool {
    let strut_line_represents_inline_child = line.glyph_runs.is_empty()
        && line.tab_gaps.is_empty()
        && line
            .offset_range
            .as_ref()
            .is_some_and(|range| range.start < range.end);
    !strut_line_represents_inline_child
}

fn paragraph_break_before_gap_boundary(view: &DocView, position: Position) -> Option<Selection> {
    let parent = view.node(position.node)?;
    let previous = position
        .offset
        .checked_sub(1)
        .and_then(|i| parent.child_at(i))?;
    let ChildView::Block(prev) = previous else {
        return None;
    };
    if prev.node_type() != NodeType::Paragraph {
        return None;
    }
    paragraph_break_at_end(
        &Position {
            affinity: Affinity::Downstream,
            ..last_cursor_position(&prev)?
        },
        view,
    )
}

fn geometry_for_line_entry(
    entry: &LayoutEntry,
    line: &LayoutLine,
    paragraph_break: Selection,
    pages: &[LayoutPage],
) -> Option<ParagraphBreakGeometry> {
    let pos = paragraph_break.anchor;
    let page_idx = page_for_y(pages, entry.rect.y)?;
    let x = entry.rect.x + super::grapheme::x_at_offset(line, &pos);
    let width = entry.rect.height * 0.15;
    Some(ParagraphBreakGeometry {
        rect: PageRect::new(
            page_idx,
            Rect::from_xywh(
                x,
                entry.rect.y - pages[page_idx].y_start,
                width,
                entry.rect.height,
            ),
        ),
        line_right: entry.rect.right(),
    })
}

#[cfg(test)]
mod tests {
    use editor_common::EdgeInsets;
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        DocLogs, DocView, Modifier, ModifierAttrLog, ModifierAttrOp, NodeAttrLog, NodeMarkerLog,
        NodeStyleLog, NodeType, ProjectedDoc, SeqItem, SpanLog, StyleLog, project_document,
    };
    use editor_state::Affinity;
    use editor_state::{Position, Selection};

    use crate::measure::context::MeasureContext;
    use crate::measure::nodes::dispatch::measure_node;
    use crate::measure::types::MeasuredTree;
    use crate::paginate::paginator::Paginator;
    use crate::paginate::types::LayoutContent;
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

    fn two_para_doc() -> DocLogs {
        let root = Dot::ROOT;
        let p1 = Dot::new(1, 1);
        let p2 = Dot::new(1, 4);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 2), SeqItem::Char('A')),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(1, 5), SeqItem::Char('B')),
        ];
        logs(&items)
    }

    fn single_para_doc() -> DocLogs {
        let root = Dot::ROOT;
        let p1 = Dot::new(2, 1);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (Dot::new(2, 2), SeqItem::Char('A')),
        ];
        logs(&items)
    }

    fn two_para_gap_doc() -> DocLogs {
        let root = Dot::ROOT;
        let p1 = Dot::new(3, 1);
        let p2 = Dot::new(3, 2);
        let items = vec![
            (
                p1,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
            (
                p2,
                SeqItem::Block {
                    node_type: NodeType::Paragraph,
                    parents: vec![root],
                },
            ),
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
        doc
    }

    fn first_line_for_para<'a>(
        index: &'a LayoutIndex,
        para_id: &Dot,
    ) -> Option<(&'a LayoutEntry, &'a LayoutLine)> {
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
    fn paragraph_break_for_line_detects_and_rejects() {
        let doc = two_para_doc();
        let (pd, index) = build_index(&doc, 400.0);
        let view = DocView::new(&pd);

        let root = view.root().unwrap();
        let mut blocks = root.child_blocks();
        let para_a_id = blocks.next().expect("first para must exist").id();

        let (_, line) = first_line_for_para(&index, &para_a_id).expect("must find line for para A");

        let sel = paragraph_break_for_line(&view, line)
            .expect("must detect paragraph break at end of para A");
        assert_eq!(sel.anchor.node, para_a_id);

        let doc2 = single_para_doc();
        let (pd2, index2) = build_index(&doc2, 400.0);
        let view2 = DocView::new(&pd2);

        let root2 = view2.root().unwrap();
        let para_id2 = root2
            .child_blocks()
            .next()
            .expect("single para must exist")
            .id();

        let (_, line2) =
            first_line_for_para(&index2, &para_id2).expect("must find line for single para");

        assert!(paragraph_break_for_line(&view2, line2).is_none());
    }

    #[test]
    fn included_in_selection_covered() {
        let doc = two_para_doc();
        let (pd, index) = build_index(&doc, 400.0);
        let view = DocView::new(&pd);

        let root = view.root().unwrap();
        let mut blocks = root.child_blocks();
        let para_a_id = blocks.next().expect("first para must exist").id();
        let para_b_id = blocks.next().expect("second para must exist").id();

        let covering = Selection::new(
            Position {
                node: para_a_id,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node: para_b_id,
                offset: 1,
                affinity: Affinity::Upstream,
            },
        );
        let rsel = covering
            .resolve(&view)
            .expect("must resolve covering selection");
        let results = included_in_selection(&index, &rsel, None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].selection.anchor.node, para_a_id);

        let not_covering = Selection::new(
            Position {
                node: para_a_id,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node: para_a_id,
                offset: 0,
                affinity: Affinity::Downstream,
            },
        );
        let rsel2 = not_covering
            .resolve(&view)
            .expect("must resolve collapsed selection");
        let results2 = included_in_selection(&index, &rsel2, None);
        assert!(results2.is_empty());
    }

    #[test]
    fn gap_boundary() {
        let doc = two_para_gap_doc();
        let (pd, index) = build_index(&doc, 400.0);
        let view = DocView::new(&pd);

        let gap_entry = index.entries().find(|e| {
            matches!(
                e.content(&index),
                Some(LayoutContent::Spacing(SpacingKind::Gap { .. }))
            )
        });

        if let Some(gap_entry) = gap_entry {
            let LayoutContent::Spacing(SpacingKind::Gap { position }) =
                gap_entry.content(&index).unwrap()
            else {
                panic!("expected Gap");
            };
            let gap_pos = *position;

            let result = paragraph_break_before_gap_boundary(&view, gap_pos);
            assert!(result.is_some(), "gap after paragraph must produce a break");

            let root = view.root().unwrap();
            let root_id = root.id();
            let none_result = paragraph_break_before_gap_boundary(
                &view,
                Position {
                    node: root_id,
                    offset: 0,
                    affinity: Affinity::Downstream,
                },
            );
            assert!(none_result.is_none(), "offset 0 must produce None");
        }
    }

    #[test]
    fn geometry_relational() {
        let doc = two_para_doc();
        let (pd, index) = build_index(&doc, 400.0);
        let view = DocView::new(&pd);

        let root = view.root().unwrap();
        let para_a_id = root
            .child_blocks()
            .next()
            .expect("first para must exist")
            .id();

        let (entry, line) =
            first_line_for_para(&index, &para_a_id).expect("must find line for para A");

        let sel = paragraph_break_for_line(&view, line).expect("must detect paragraph break");

        let geom = geometry_for_line_entry(entry, line, sel, index.pages())
            .expect("must produce geometry");

        assert!(
            geom.rect.rect.width > 0.0,
            "paragraph-break glyph must have positive width"
        );
        assert_eq!(geom.line_right, entry.rect.right());

        let expected_x = entry.rect.x + super::super::grapheme::x_at_offset(line, &sel.anchor);
        assert_eq!(geom.rect.rect.x, expected_x);
    }

    #[test]
    fn drag_gap_branch() {
        let doc = two_para_gap_doc();
        let (pd, index) = build_index(&doc, 400.0);
        let view = DocView::new(&pd);

        let gap_entry = index.entries().find(|e| {
            matches!(
                e.content(&index),
                Some(LayoutContent::Spacing(SpacingKind::Gap { .. }))
            )
        });

        if let Some(gap_entry) = gap_entry {
            let LayoutContent::Spacing(SpacingKind::Gap { position }) =
                gap_entry.content(&index).unwrap()
            else {
                panic!("expected Gap");
            };
            let gap_pos = *position;

            let pb = paragraph_break_before_gap_boundary(&view, gap_pos);
            if pb.is_none() {
                return;
            }
            let pb_sel = pb.unwrap();

            let resolved_pb = pb_sel.resolve(&view).expect("pb selection must resolve");
            let from_pos = resolved_pb.from().position();

            let anchor_rp = from_pos.resolve(&view).expect("anchor must resolve");

            let point = LayoutPoint {
                page_idx: 0,
                x: 0.0,
                y: gap_entry.rect.y + gap_entry.rect.height / 2.0,
                page_y_start: 0.0,
            };

            let result = drag_selection_for_entry(&index, &view, &anchor_rp, gap_entry, point);
            assert!(
                result.is_some(),
                "drag_selection_for_entry must return Some for gap entry"
            );

            let root = view.root().unwrap();
            let last_para_id = root
                .child_blocks()
                .last()
                .expect("last para must exist")
                .id();
            let last_para_children = view.node(last_para_id).unwrap().children().count();
            let after_anchor = Position {
                node: last_para_id,
                offset: last_para_children,
                affinity: Affinity::Upstream,
            };
            let after_anchor_rp = after_anchor
                .resolve(&view)
                .expect("after-anchor must resolve");

            let result2 =
                drag_selection_for_entry(&index, &view, &after_anchor_rp, gap_entry, point);
            assert!(
                result2.is_some(),
                "drag_selection_for_entry after gap must return Some"
            );
        }
    }
}
