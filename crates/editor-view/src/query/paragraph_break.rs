use editor_common::Rect;
use editor_model::{Doc, Node};
use editor_state::{
    Affinity, NodeRefCursorExt, Position, ResolvedPosition, ResolvedSelection, Selection,
    paragraph_break_selection_at_paragraph_end, position_before_or_same_logical_boundary,
};

use crate::page::{LayoutPage, PageRect};
use crate::paginate::{LayoutContent, SpacingKind};

use super::common::page_for_y;
use super::layout_index::{LayoutEntry, LayoutIndex, LayoutPoint};

pub(crate) struct ParagraphBreakGeometry {
    pub(crate) rect: PageRect,
    pub(crate) line_right: f32,
}

pub(crate) struct SelectedParagraphBreak {
    pub(crate) selection: Selection,
    pub(crate) geometry: ParagraphBreakGeometry,
}

pub(crate) fn included_in_selection(
    layout_index: &LayoutIndex,
    selection: &ResolvedSelection<'_>,
) -> Vec<SelectedParagraphBreak> {
    let mut paragraph_breaks = Vec::new();
    for entry in layout_index.entries() {
        let Some(paragraph_break) = paragraph_break_for_entry(layout_index, selection.doc(), entry)
        else {
            continue;
        };
        if !selection.contains_range(paragraph_break.selection) {
            continue;
        }
        if paragraph_breaks
            .iter()
            .any(|existing: &SelectedParagraphBreak| {
                existing.selection == paragraph_break.selection
            })
        {
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
    doc: &Doc,
    anchor: &ResolvedPosition<'_>,
    entry: &LayoutEntry,
    point: LayoutPoint,
) -> Option<Selection> {
    let paragraph_break = paragraph_break_for_entry(layout_index, doc, entry)?;
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
            let resolved = paragraph_break.selection.resolve(doc)?;
            if position_before_or_same_logical_boundary(
                doc,
                Position::from(anchor),
                Position::from(resolved.from()),
            ) {
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
    doc: &Doc,
    entry: &LayoutEntry,
) -> Option<SelectedParagraphBreak> {
    match entry.content(layout_index)? {
        LayoutContent::Line(line) => {
            let selection = paragraph_break_for_line(doc, line)?;
            let geometry = geometry_for_line_entry(entry, line, selection, layout_index.pages())?;
            Some(SelectedParagraphBreak {
                selection,
                geometry,
            })
        }
        LayoutContent::Spacing(SpacingKind::Gap { position }) => {
            let selection = paragraph_break_before_gap_boundary(doc, *position)?;
            let geometry = geometry(layout_index, selection, layout_index.pages())?;
            Some(SelectedParagraphBreak {
                selection,
                geometry,
            })
        }
        LayoutContent::Box(_) | LayoutContent::Atom(_) | LayoutContent::Spacing(_) => None,
    }
}

fn paragraph_break_for_line(doc: &Doc, line: &crate::paginate::LayoutLine) -> Option<Selection> {
    if !line_can_host_visual_paragraph_break(line) {
        return None;
    }
    let line_end = super::grapheme::last_position_in_line(line);
    paragraph_break_selection_at_paragraph_end(doc, line_end)
}

fn line_can_host_visual_paragraph_break(line: &crate::paginate::LayoutLine) -> bool {
    let strut_line_represents_inline_child = line.glyph_runs.is_empty()
        && line.tab_gaps.is_empty()
        && line
            .child_range
            .as_ref()
            .is_some_and(|range| range.start < range.end);
    !strut_line_represents_inline_child
}

fn paragraph_break_before_gap_boundary(doc: &Doc, position: Position) -> Option<Selection> {
    let parent = doc.node(position.node_id)?;
    let previous = position
        .offset
        .checked_sub(1)
        .and_then(|index| parent.children().nth(index))?;
    if !matches!(previous.node(), Node::Paragraph(_)) {
        return None;
    }
    paragraph_break_selection_at_paragraph_end(
        doc,
        Position {
            affinity: Affinity::Downstream,
            ..previous.last_cursor_position()?
        },
    )
}

fn geometry_for_line_entry(
    entry: &LayoutEntry,
    line: &crate::paginate::LayoutLine,
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
