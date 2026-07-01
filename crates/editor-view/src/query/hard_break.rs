pub(crate) struct HardBreakGeometry {
    pub(crate) rect: PageRect,
    pub(crate) line_right: f32,
}

use editor_common::Rect;
use editor_model::{AtomLeaf, ChildView, DocView};
use editor_state::Affinity;
use editor_state::{Position, ResolvedSelection, Selection};

use crate::page::{LayoutPage, PageRect};
use crate::paginate::types::{LayoutContent, LayoutLine};

use super::common::page_for_y;
use super::layout_index::{LayoutEntry, LayoutIndex, LayoutPoint};

pub(crate) struct SelectedHardBreak {
    pub(crate) selection: Selection,
    pub(crate) geometry: HardBreakGeometry,
}

pub(crate) fn included_in_selection(
    layout_index: &LayoutIndex,
    selection: &ResolvedSelection<'_>,
    y_bounds: Option<(f32, f32)>,
) -> Vec<SelectedHardBreak> {
    let mut hard_breaks = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for entry in layout_index.entries() {
        if let Some((y_start, y_end)) = y_bounds
            && !entry.overlaps_y_range(y_start, y_end)
        {
            continue;
        }
        let Some(hard_break) = hard_break_for_entry(layout_index, selection.view(), entry) else {
            continue;
        };
        if !selection.contains_range(hard_break.selection) {
            continue;
        }
        if !seen.insert(super::common::selection_key(&hard_break.selection)) {
            continue;
        }
        hard_breaks.push(hard_break);
    }
    hard_breaks
}

pub(crate) fn drag_selection_for_entry(
    layout_index: &LayoutIndex,
    view: &DocView,
    entry: &LayoutEntry,
    point: LayoutPoint,
) -> Option<Selection> {
    let hard_break = hard_break_for_entry(layout_index, view, entry)?;
    let rect = hard_break.geometry.rect.rect;
    let page_y = point.y - point.page_y_start;
    let x_mid = rect.x + rect.width / 2.0;
    if hard_break.geometry.rect.page_idx == point.page_idx
        && page_y >= rect.y
        && page_y <= rect.bottom()
        && point.x >= x_mid
        && point.x <= hard_break.geometry.line_right
    {
        Some(hard_break.selection)
    } else {
        None
    }
}

fn hard_break_for_entry(
    layout_index: &LayoutIndex,
    view: &DocView,
    entry: &LayoutEntry,
) -> Option<SelectedHardBreak> {
    let LayoutContent::Line(line) = entry.content(layout_index)? else {
        return None;
    };
    let selection = hard_break_for_line(view, line)?;
    let geometry = geometry_for_line_entry(entry, line, selection, layout_index.pages())?;
    Some(SelectedHardBreak {
        selection,
        geometry,
    })
}

fn hard_break_for_line(view: &DocView, line: &LayoutLine) -> Option<Selection> {
    let range = line.offset_range.as_ref()?;
    let index = range.end;
    let paragraph = view.node(line.node)?;
    let child = paragraph.children().nth(index)?;
    if !matches!(child, ChildView::Leaf(ref l) if matches!(l.as_atom(), Some(AtomLeaf::HardBreak)))
    {
        return None;
    }
    Some(Selection::new(
        Position {
            node: line.node,
            offset: index,
            affinity: Affinity::Downstream,
        },
        Position {
            node: line.node,
            offset: index + 1,
            affinity: Affinity::Upstream,
        },
    ))
}

fn geometry_for_line_entry(
    entry: &LayoutEntry,
    line: &LayoutLine,
    hard_break: Selection,
    pages: &[LayoutPage],
) -> Option<HardBreakGeometry> {
    let page_idx = page_for_y(pages, entry.rect.y)?;
    let x = entry.rect.x + super::grapheme::x_at_offset(line, &hard_break.anchor);
    let width = entry.rect.height * 0.15;
    Some(HardBreakGeometry {
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
        AtomLeaf, DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeMarkerLog, NodeStyleLog,
        NodeType, ProjectedDoc, SeqItem, SpanLog, StyleLog, project_document,
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

    fn build_index_and_project(doc: &DocLogs, width: f32) -> (ProjectedDoc, LayoutIndex) {
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

    fn para_with_hard_break(text: &str, width: f32) -> (ProjectedDoc, Dot, LayoutIndex) {
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
        let hb_idx = 2 + text.len() as u64;
        items.push((Dot::new(1, hb_idx), SeqItem::Atom(AtomLeaf::HardBreak)));
        let doc = logs(&items);
        let para_id = para;
        let (pd, index) = build_index_and_project(&doc, width);
        (pd, para_id, index)
    }

    fn para_without_hard_break(text: &str, width: f32) -> (ProjectedDoc, Dot, LayoutIndex) {
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
        let doc = logs(&items);
        let para_id = para;
        let (pd, index) = build_index_and_project(&doc, width);
        (pd, para_id, index)
    }

    fn first_line_for_para<'a>(
        index: &'a LayoutIndex,
        para_id: &Dot,
    ) -> Option<(&'a crate::query::layout_index::LayoutEntry, &'a LayoutLine)> {
        for entry in index.entries() {
            if let Some(node) = entry.node(index)
                && let LayoutContent::Line(line) = &node.content
                && &line.node == para_id
                && let Some(_range) = &line.offset_range
            {
                return Some((entry, line));
            }
        }
        None
    }

    #[test]
    fn hard_break_for_line_detects_and_rejects() {
        let (pd, para_id, index) = para_with_hard_break("Hi", 400.0);
        let view = DocView::new(&pd);

        let (entry, line) = first_line_for_para(&index, &para_id)
            .expect("must find line with offset_range for para with hard break");

        let sel = hard_break_for_line(&view, line).expect("must detect trailing hard break");
        assert_eq!(sel.anchor.node, para_id);
        assert_eq!(sel.head.node, para_id);
        let expected_index = line.offset_range.as_ref().unwrap().end;
        assert_eq!(sel.anchor.offset, expected_index);
        assert_eq!(sel.head.offset, expected_index + 1);
        assert_eq!(sel.anchor.affinity, Affinity::Downstream);
        assert_eq!(sel.head.affinity, Affinity::Upstream);

        let (pd2, para_id2, index2) = para_without_hard_break("Hi", 400.0);
        let view2 = DocView::new(&pd2);
        let (_, line2) = first_line_for_para(&index2, &para_id2)
            .expect("must find line for para without hard break");
        assert!(hard_break_for_line(&view2, line2).is_none());

        let _ = entry;
    }

    #[test]
    fn included_in_selection_covered_and_dedup() {
        let (pd, para_id, index) = para_with_hard_break("Hi", 400.0);
        let view = DocView::new(&pd);

        let (_, line) = first_line_for_para(&index, &para_id).expect("must find line");
        let hb_index = line.offset_range.as_ref().unwrap().end;

        let covering = Selection::new(
            Position {
                node: para_id,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node: para_id,
                offset: hb_index + 1,
                affinity: Affinity::Upstream,
            },
        );
        let rsel = covering
            .resolve(&view)
            .expect("must resolve covering selection");
        let results = included_in_selection(&index, &rsel, None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].selection.anchor.node, para_id);

        let not_covering = Selection::new(
            Position {
                node: para_id,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Position {
                node: para_id,
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
    fn geometry_relational() {
        let (pd, para_id, index) = para_with_hard_break("Hi", 400.0);
        let _view = DocView::new(&pd);

        let (entry, line) = first_line_for_para(&index, &para_id).expect("must find line");
        let hb_index = line.offset_range.as_ref().unwrap().end;

        let sel = Selection::new(
            Position {
                node: para_id,
                offset: hb_index,
                affinity: Affinity::Downstream,
            },
            Position {
                node: para_id,
                offset: hb_index + 1,
                affinity: Affinity::Upstream,
            },
        );

        let geom = geometry_for_line_entry(entry, line, sel, index.pages())
            .expect("must produce geometry");

        assert!(
            geom.rect.rect.width > 0.0,
            "hard-break glyph must have positive width"
        );
        assert_eq!(geom.line_right, entry.rect.right());

        let expected_x = entry.rect.x + super::super::grapheme::x_at_offset(line, &sel.anchor);
        assert_eq!(geom.rect.rect.x, expected_x);
    }

    #[test]
    fn drag_selection_hit() {
        let (pd, para_id, index) = para_with_hard_break("Hi", 400.0);
        let view = DocView::new(&pd);

        let (entry, line) = first_line_for_para(&index, &para_id).expect("must find line");
        let hb_index = line.offset_range.as_ref().unwrap().end;

        let sel = Selection::new(
            Position {
                node: para_id,
                offset: hb_index,
                affinity: Affinity::Downstream,
            },
            Position {
                node: para_id,
                offset: hb_index + 1,
                affinity: Affinity::Upstream,
            },
        );
        let geom = geometry_for_line_entry(entry, line, sel, index.pages())
            .expect("must produce geometry");

        let rect = &geom.rect.rect;
        let mid_x = rect.x + rect.width / 2.0 + 1.0;
        let page_y_start = index.pages()[geom.rect.page_idx].y_start;
        let point_inside = LayoutPoint {
            page_idx: geom.rect.page_idx,
            x: mid_x,
            y: page_y_start + rect.y + rect.height / 2.0,
            page_y_start,
        };
        let hit = drag_selection_for_entry(&index, &view, entry, point_inside);
        assert!(hit.is_some(), "point inside glyph rect must hit");

        let point_outside = LayoutPoint {
            page_idx: geom.rect.page_idx,
            x: rect.x - 10.0,
            y: page_y_start + rect.y + rect.height / 2.0,
            page_y_start,
        };
        let miss = drag_selection_for_entry(&index, &view, entry, point_outside);
        assert!(miss.is_none(), "point outside glyph rect must miss");
    }
}
