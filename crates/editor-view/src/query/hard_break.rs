use editor_common::Rect;
use editor_model::{Doc, Node};
use editor_state::{Affinity, Position, ResolvedSelection, Selection};

use crate::page::{LayoutPage, PageRect};
use crate::paginate::{LayoutContent, LayoutLine};

use super::common::page_for_y;
use super::layout_index::{LayoutEntry, LayoutIndex, LayoutPoint};

pub(crate) struct HardBreakGeometry {
    pub(crate) rect: PageRect,
    pub(crate) line_right: f32,
}

pub(crate) struct SelectedHardBreak {
    pub(crate) selection: Selection,
    pub(crate) geometry: HardBreakGeometry,
}

pub(crate) fn included_in_selection(
    layout_index: &LayoutIndex,
    selection: &ResolvedSelection<'_>,
) -> Vec<SelectedHardBreak> {
    let mut hard_breaks = Vec::new();
    for entry in layout_index.entries() {
        let Some(hard_break) = hard_break_for_entry(layout_index, selection.doc(), entry) else {
            continue;
        };
        if !selection.contains_range(hard_break.selection) {
            continue;
        }
        if hard_breaks
            .iter()
            .any(|existing: &SelectedHardBreak| existing.selection == hard_break.selection)
        {
            continue;
        }
        hard_breaks.push(hard_break);
    }
    hard_breaks
}

pub(crate) fn drag_selection_for_entry(
    layout_index: &LayoutIndex,
    doc: &Doc,
    entry: &LayoutEntry,
    point: LayoutPoint,
) -> Option<Selection> {
    let hard_break = hard_break_for_entry(layout_index, doc, entry)?;
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
    doc: &Doc,
    entry: &LayoutEntry,
) -> Option<SelectedHardBreak> {
    let LayoutContent::Line(line) = entry.content(layout_index)? else {
        return None;
    };
    let selection = hard_break_for_line(doc, line)?;
    let geometry = geometry_for_line_entry(entry, line, selection, layout_index.pages())?;
    Some(SelectedHardBreak {
        selection,
        geometry,
    })
}

fn hard_break_for_line(doc: &Doc, line: &LayoutLine) -> Option<Selection> {
    let range = line.child_range.as_ref()?;
    let index = range.end;
    let paragraph = doc.node(line.node_id)?;
    let child = paragraph.children().nth(index)?;
    if !matches!(child.node(), Node::HardBreak(_)) {
        return None;
    }
    Some(Selection::new(
        Position {
            node_id: line.node_id,
            offset: index,
            affinity: Affinity::Downstream,
        },
        Position {
            node_id: line.node_id,
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
