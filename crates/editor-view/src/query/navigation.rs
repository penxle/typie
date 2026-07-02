use editor_common::{Axis, Direction, Movement};
use editor_crdt::Dot;
use editor_resource::Resource;
use editor_state::Affinity;
use editor_state::{Position, Selection};

use crate::paginate::types::{ChildAttachment, LayoutBox, LayoutContent, LayoutLine};
use crate::viewport::Viewport;

use super::cursor::x_at_offset;
use super::layout_index::{LayoutEntry, LayoutIndex};
use super::segmentation;

pub(crate) fn resolve_movement(
    layout_index: &LayoutIndex,
    pos: &Position,
    movement: &Movement,
    viewport: &Viewport,
    resource: &Resource,
    preferred_x: Option<f32>,
) -> (Option<Selection>, Option<f32>) {
    let segmenters = &resource.segmenters;
    match movement {
        Movement::Grapheme {
            direction: Direction::Forward,
        } => (move_grapheme_forward(layout_index, pos), None),
        Movement::Grapheme {
            direction: Direction::Backward,
        } => (move_grapheme_backward(layout_index, pos), None),
        Movement::Word {
            direction: Direction::Forward,
        } => (
            segmentation::move_word_forward(layout_index, pos, segmenters),
            None,
        ),
        Movement::Word {
            direction: Direction::Backward,
        } => (
            segmentation::move_word_backward(layout_index, pos, segmenters),
            None,
        ),
        Movement::Sentence {
            direction: Direction::Forward,
        } => (
            segmentation::move_sentence_forward(layout_index, pos, segmenters),
            None,
        ),
        Movement::Sentence {
            direction: Direction::Backward,
        } => (
            segmentation::move_sentence_backward(layout_index, pos, segmenters),
            None,
        ),
        Movement::Line {
            direction: Direction::Forward,
            axis: Axis::Horizontal,
        } => (move_line_horizontal_forward(layout_index, pos), None),
        Movement::Line {
            direction: Direction::Backward,
            axis: Axis::Horizontal,
        } => (move_line_horizontal_backward(layout_index, pos), None),
        Movement::Line {
            direction: Direction::Forward,
            axis: Axis::Vertical,
        } => move_line_vertical_forward(layout_index, pos, preferred_x),
        Movement::Line {
            direction: Direction::Backward,
            axis: Axis::Vertical,
        } => move_line_vertical_backward(layout_index, pos, preferred_x),
        Movement::Page {
            direction: Direction::Forward,
        } => move_page_forward(layout_index, pos, viewport, preferred_x),
        Movement::Page {
            direction: Direction::Backward,
        } => move_page_backward(layout_index, pos, viewport, preferred_x),
        Movement::Document {
            direction: Direction::Forward,
        } => (move_document_forward(layout_index), None),
        Movement::Document {
            direction: Direction::Backward,
        } => (move_document_backward(layout_index), None),
    }
}

fn move_grapheme_forward(layout_index: &LayoutIndex, pos: &Position) -> Option<Selection> {
    let entry = layout_index.entry_for_position(pos)?;

    match entry.content(layout_index)? {
        LayoutContent::Line(line) => {
            if line.glyph_runs.is_empty()
                && let Some(range) = &line.offset_range
                && pos.node == line.node
                && pos.offset >= range.start
                && pos.offset < range.end
            {
                return Some(Selection::collapsed(Position {
                    node: line.node,
                    offset: pos.offset + 1,
                    affinity: Affinity::Upstream,
                }));
            }
            for (i, run) in line.glyph_runs.iter().enumerate() {
                if pos.node != line.node {
                    continue;
                }
                if pos.offset < run.offset_range.start || pos.offset > run.offset_range.end {
                    continue;
                }
                let local = pos.offset - run.offset_range.start;
                let mut cp_acc = 0usize;
                for g in &run.graphemes {
                    let cp = g.codepoints as usize;
                    if local < cp_acc + cp {
                        return Some(Selection::collapsed(Position::new(
                            line.node,
                            run.offset_range.start + cp_acc + cp,
                        )));
                    }
                    cp_acc += cp;
                }
                if local == cp_acc
                    && let Some(next) = line.glyph_runs.get(i + 1)
                {
                    let separated_by_tab = line
                        .tab_gaps
                        .iter()
                        .any(|gap| gap.x >= run.x - 0.5 && gap.x + gap.width <= next.x + 0.5);
                    if separated_by_tab {
                        return Some(Selection::collapsed(Position::new(
                            line.node,
                            next.offset_range.start,
                        )));
                    }
                    if let Some(g) = next.graphemes.first() {
                        return Some(Selection::collapsed(Position::new(
                            line.node,
                            next.offset_range.start + g.codepoints as usize,
                        )));
                    }
                }
            }
            let next = next_navigable_entry(layout_index, entry)?;
            if let Some(LayoutContent::Line(next_line)) = next.content(layout_index)
                && let Some(first_run) = next_line.glyph_runs.first()
                && pos.node == next_line.node
                && first_run.offset_range.start == pos.offset
                && let Some(g) = first_run.graphemes.first()
            {
                return Some(Selection::collapsed(Position::new(
                    line.node,
                    pos.offset + g.codepoints as usize,
                )));
            }
            if let Some(LayoutContent::Line(next_line)) = next.content(layout_index)
                && next_line.glyph_runs.is_empty()
                && let Some(range) = &next_line.offset_range
                && pos.node == next_line.node
                && pos.offset == range.start
                && range.start < range.end
            {
                return Some(Selection::collapsed(Position {
                    node: next_line.node,
                    offset: range.start + 1,
                    affinity: Affinity::Upstream,
                }));
            }
            Some(landed_entry(layout_index, next, false, true))
        }
        LayoutContent::Box(b) => move_box_boundary(layout_index, entry, b, pos, true),
        _ => {
            let nv = unit_attachment(layout_index, entry)?;
            if let Some(next) = next_navigable_entry(layout_index, entry) {
                Some(landed_entry(layout_index, next, false, true))
            } else {
                Some(Selection::collapsed(Position::new(nv.parent, nv.index + 1)))
            }
        }
    }
}

fn move_grapheme_backward(layout_index: &LayoutIndex, pos: &Position) -> Option<Selection> {
    let entry = layout_index.entry_for_position(pos)?;

    match entry.content(layout_index)? {
        LayoutContent::Line(line) => {
            if line.glyph_runs.is_empty()
                && let Some(range) = &line.offset_range
                && pos.node == line.node
                && pos.offset > range.start
                && pos.offset <= range.end
            {
                return Some(Selection::collapsed(Position {
                    node: line.node,
                    offset: pos.offset - 1,
                    affinity: Affinity::Downstream,
                }));
            }
            for (i, run) in line.glyph_runs.iter().enumerate() {
                if pos.node != line.node {
                    continue;
                }
                if pos.offset < run.offset_range.start || pos.offset > run.offset_range.end {
                    continue;
                }
                if pos.offset > run.offset_range.start {
                    let local = pos.offset - run.offset_range.start;
                    let mut cp_acc = 0usize;
                    let mut prev_boundary = 0usize;
                    for g in &run.graphemes {
                        let cp = g.codepoints as usize;
                        if cp_acc + cp >= local {
                            return Some(Selection::collapsed(Position::new(
                                line.node,
                                run.offset_range.start + prev_boundary,
                            )));
                        }
                        prev_boundary = cp_acc + cp;
                        cp_acc += cp;
                    }
                }
                if pos.offset == run.offset_range.start && i > 0 {
                    let prev = &line.glyph_runs[i - 1];
                    let total = prev.offset_range.len();
                    let separated_by_tab = line
                        .tab_gaps
                        .iter()
                        .any(|gap| gap.x >= prev.x - 0.5 && gap.x + gap.width <= run.x + 0.5);
                    if separated_by_tab {
                        return Some(Selection::collapsed(Position::new(
                            line.node,
                            prev.offset_range.start + total,
                        )));
                    }
                    if let Some(g) = prev.graphemes.last() {
                        return Some(Selection::collapsed(Position::new(
                            line.node,
                            prev.offset_range.start + total - g.codepoints as usize,
                        )));
                    }
                }
            }
            let prev = prev_navigable_entry(layout_index, entry)?;
            if let Some(LayoutContent::Line(prev_line)) = prev.content(layout_index)
                && let Some(last_run) = prev_line.glyph_runs.last()
                && pos.node == prev_line.node
                && last_run.offset_range.end == pos.offset
                && let Some(g) = last_run.graphemes.last()
            {
                return Some(Selection::collapsed(Position::new(
                    line.node,
                    pos.offset - g.codepoints as usize,
                )));
            }
            if let Some(LayoutContent::Line(prev_line)) = prev.content(layout_index)
                && prev_line.glyph_runs.is_empty()
                && let Some(prev_range) = &prev_line.offset_range
                && prev_range.start < prev_range.end
            {
                let at_para_boundary = pos.node == prev_line.node && pos.offset == prev_range.end;
                let at_line_start = line
                    .glyph_runs
                    .first()
                    .is_some_and(|r| pos.node == line.node && r.offset_range.start == pos.offset);
                if at_para_boundary || at_line_start {
                    return Some(Selection::collapsed(Position {
                        node: prev_line.node,
                        offset: prev_range.end - 1,
                        affinity: Affinity::Downstream,
                    }));
                }
            }
            Some(landed_entry(layout_index, prev, true, false))
        }
        LayoutContent::Box(b) => move_box_boundary(layout_index, entry, b, pos, false),
        _ => {
            let nv = unit_attachment(layout_index, entry)?;
            if let Some(prev) = prev_navigable_entry(layout_index, entry) {
                Some(landed_entry(layout_index, prev, true, false))
            } else {
                Some(Selection::collapsed(Position::new(nv.parent, nv.index)))
            }
        }
    }
}

fn move_line_horizontal_forward(layout_index: &LayoutIndex, pos: &Position) -> Option<Selection> {
    let entry = layout_index.entry_for_position(pos)?;
    match entry.content(layout_index)? {
        LayoutContent::Line(line) => Some(Selection::collapsed(last_position_in_line(line))),
        LayoutContent::Box(b) => move_box_boundary(layout_index, entry, b, pos, true),
        _ => None,
    }
}

fn move_line_horizontal_backward(layout_index: &LayoutIndex, pos: &Position) -> Option<Selection> {
    let entry = layout_index.entry_for_position(pos)?;
    match entry.content(layout_index)? {
        LayoutContent::Line(line) => Some(Selection::collapsed(first_position_in_line(line))),
        LayoutContent::Box(b) => move_box_boundary(layout_index, entry, b, pos, false),
        _ => None,
    }
}

fn move_line_vertical_forward(
    layout_index: &LayoutIndex,
    pos: &Position,
    preferred_x: Option<f32>,
) -> (Option<Selection>, Option<f32>) {
    let Some(entry) = layout_index.entry_for_position(pos) else {
        return (None, preferred_x);
    };
    let x = preferred_x.unwrap_or_else(|| compute_preferred_x(layout_index, entry, pos));
    let y = entry.rect.bottom();
    let target = navigable_below_at_x(layout_index, y, x);
    let sel = if let Some(t) = target {
        let s = escape_empty_line_trap(
            layout_index,
            navigate_to_entry(layout_index, t, x),
            Some(t),
            pos,
            true,
        );
        Some(skip_over_when_stuck(layout_index, s, t, pos, x, true))
    } else {
        line_end_fallback(layout_index, entry, true)
    };
    (sel, Some(x))
}

fn move_line_vertical_backward(
    layout_index: &LayoutIndex,
    pos: &Position,
    preferred_x: Option<f32>,
) -> (Option<Selection>, Option<f32>) {
    let Some(entry) = layout_index.entry_for_position(pos) else {
        return (None, preferred_x);
    };
    let x = preferred_x.unwrap_or_else(|| compute_preferred_x(layout_index, entry, pos));
    let y = entry.rect.y;
    let target = navigable_above_at_x(layout_index, y, x);
    let sel = if let Some(t) = target {
        let s = escape_empty_line_trap(
            layout_index,
            navigate_to_entry(layout_index, t, x),
            Some(t),
            pos,
            false,
        );
        Some(skip_over_when_stuck(layout_index, s, t, pos, x, false))
    } else {
        line_end_fallback(layout_index, entry, false)
    };
    (sel, Some(x))
}

fn skip_over_when_stuck(
    layout_index: &LayoutIndex,
    sel: Selection,
    target: &LayoutEntry,
    pos: &Position,
    x: f32,
    forward: bool,
) -> Selection {
    let Some(LayoutContent::Line(line)) = target.content(layout_index) else {
        return sel;
    };
    if !line.glyph_runs.is_empty() {
        return sel;
    }
    let Some(range) = &line.offset_range else {
        return sel;
    };
    if range.start != range.end {
        return sel;
    }
    if sel.head.node != pos.node || sel.head.offset != pos.offset {
        return sel;
    }
    let next = if forward {
        navigable_below_at_x(layout_index, target.rect.bottom(), x)
    } else {
        navigable_above_at_x(layout_index, target.rect.y, x)
    };
    next.map(|entry| navigate_to_entry(layout_index, entry, x))
        .unwrap_or(sel)
}

fn line_end_fallback(
    layout_index: &LayoutIndex,
    entry: &LayoutEntry,
    forward: bool,
) -> Option<Selection> {
    let Some(LayoutContent::Line(line)) = entry.content(layout_index) else {
        return None;
    };
    let pos = if forward {
        last_position_in_line(line)
    } else {
        first_position_in_line(line)
    };
    Some(Selection::collapsed(pos))
}

fn escape_empty_line_trap(
    layout_index: &LayoutIndex,
    sel: Selection,
    target: Option<&LayoutEntry>,
    pos: &Position,
    forward: bool,
) -> Selection {
    let Some(t) = target else { return sel };
    let Some(LayoutContent::Line(line)) = t.content(layout_index) else {
        return sel;
    };
    if !line.glyph_runs.is_empty() {
        return sel;
    }
    if sel.head.node != pos.node || sel.head.offset != pos.offset {
        return sel;
    }
    let Some(range) = &line.offset_range else {
        return sel;
    };
    if range.end == range.start {
        return sel;
    }
    let (offset, affinity) = if forward {
        (range.end, Affinity::Upstream)
    } else {
        (range.start, Affinity::Downstream)
    };
    Selection::collapsed(Position {
        node: line.node,
        offset,
        affinity,
    })
}

fn move_page_forward(
    layout_index: &LayoutIndex,
    pos: &Position,
    viewport: &Viewport,
    preferred_x: Option<f32>,
) -> (Option<Selection>, Option<f32>) {
    let Some(entry) = layout_index.entry_for_position(pos) else {
        return (None, preferred_x);
    };
    let x = preferred_x.unwrap_or_else(|| compute_preferred_x(layout_index, entry, pos));
    let y = entry.rect.y + viewport.height;
    let target = navigable_below_at_x(layout_index, y, x);
    (
        target.map(|t| navigate_to_entry(layout_index, t, x)),
        Some(x),
    )
}

fn move_page_backward(
    layout_index: &LayoutIndex,
    pos: &Position,
    viewport: &Viewport,
    preferred_x: Option<f32>,
) -> (Option<Selection>, Option<f32>) {
    let Some(entry) = layout_index.entry_for_position(pos) else {
        return (None, preferred_x);
    };
    let x = preferred_x.unwrap_or_else(|| compute_preferred_x(layout_index, entry, pos));
    let y = entry.rect.bottom() - viewport.height;
    let target = navigable_above_at_x(layout_index, y, x);
    (
        target.map(|t| navigate_to_entry(layout_index, t, x)),
        Some(x),
    )
}

fn move_document_forward(layout_index: &LayoutIndex) -> Option<Selection> {
    let nav = last_navigable_entry(layout_index)?;
    Some(landed_entry(layout_index, nav, true, true))
}

fn move_document_backward(layout_index: &LayoutIndex) -> Option<Selection> {
    let nav = first_navigable_entry(layout_index)?;
    Some(landed_entry(layout_index, nav, false, false))
}

fn first_position_in_line(line: &LayoutLine) -> Position {
    let run_first = line.glyph_runs.first();
    let leading_gap = line
        .tab_gaps
        .iter()
        .filter(|g| run_first.is_none_or(|r| g.x < r.x))
        .min_by(|a, b| a.x.total_cmp(&b.x));
    if let Some(gap) = leading_gap {
        return Position::new(line.node, gap.offset_index);
    }
    if let Some(run) = run_first {
        return Position::new(line.node, run.offset_range.start);
    }
    let offset = line.offset_range.as_ref().map(|r| r.start).unwrap_or(0);
    Position {
        node: line.node,
        offset,
        affinity: Affinity::Upstream,
    }
}

fn last_position_in_line(line: &LayoutLine) -> Position {
    super::grapheme::last_position_in_line(line)
}

fn first_position_in_entry(layout_index: &LayoutIndex, entry: &LayoutEntry) -> Position {
    match entry.content(layout_index) {
        Some(LayoutContent::Line(line)) => first_position_in_line(line),
        Some(LayoutContent::Atom(atom)) => {
            Position::new(atom.attachment.parent, atom.attachment.index)
        }
        Some(LayoutContent::Box(b)) if b.style.monolithic && b.attachment.is_some() => {
            let attachment = b.attachment.as_ref().expect("checked is_some");
            Position::new(attachment.parent, attachment.index)
        }
        _ => {
            unreachable!("first_position_in_entry called on non-navigable entry")
        }
    }
}

fn last_position_in_entry(layout_index: &LayoutIndex, entry: &LayoutEntry) -> Position {
    match entry.content(layout_index) {
        Some(LayoutContent::Line(line)) => last_position_in_line(line),
        Some(LayoutContent::Atom(atom)) => {
            Position::new(atom.attachment.parent, atom.attachment.index)
        }
        Some(LayoutContent::Box(b)) if b.style.monolithic && b.attachment.is_some() => {
            let attachment = b.attachment.as_ref().expect("checked is_some");
            Position::new(attachment.parent, attachment.index)
        }
        _ => {
            unreachable!("last_position_in_entry called on non-navigable entry")
        }
    }
}

pub(crate) fn move_box_boundary(
    layout_index: &LayoutIndex,
    entry: &LayoutEntry,
    b: &LayoutBox,
    pos: &Position,
    forward: bool,
) -> Option<Selection> {
    let pivot = box_boundary_pivot(layout_index, entry, b, pos)?;
    let target = navigable_from_pivot(layout_index, pivot, forward)?;
    Some(landed_entry(layout_index, target, !forward, forward))
}

fn box_boundary_pivot(
    layout_index: &LayoutIndex,
    entry: &LayoutEntry,
    b: &LayoutBox,
    pos: &Position,
) -> Option<usize> {
    b.attachment.as_ref()?;
    let idx = layout_index.entry_index(entry)?;
    Some(match pos.affinity {
        Affinity::Downstream => idx,
        Affinity::Upstream => index_after_box_subtree(layout_index, idx, &b.node),
    })
}

fn index_after_box_subtree(layout_index: &LayoutIndex, idx: usize, node: &Dot) -> usize {
    layout_index
        .entries()
        .enumerate()
        .skip(idx + 1)
        .find(|(_, entry)| !entry.ancestors().contains(node))
        .map(|(idx, _)| idx)
        .unwrap_or_else(|| layout_index.entries().len())
}

fn navigable_from_pivot(
    layout_index: &LayoutIndex,
    pivot: usize,
    forward: bool,
) -> Option<&LayoutEntry> {
    if forward {
        layout_index
            .entries()
            .skip(pivot)
            .find(|entry| is_navigable_entry(layout_index, entry))
    } else {
        layout_index
            .entries()
            .take(pivot)
            .rev()
            .find(|entry| is_navigable_entry(layout_index, entry))
    }
}

pub(crate) fn editable_position_inside(
    layout_index: &LayoutIndex,
    node: &Dot,
    at_end: bool,
) -> Option<Position> {
    let nav = first_navigable_entry_inside(layout_index, node, at_end)?;
    Some(if at_end {
        last_position_in_entry(layout_index, nav)
    } else {
        first_position_in_entry(layout_index, nav)
    })
}

pub(crate) fn compute_preferred_x_at(layout_index: &LayoutIndex, pos: &Position) -> Option<f32> {
    let entry = layout_index.entry_for_position(pos)?;
    Some(compute_preferred_x(layout_index, entry, pos))
}

pub(crate) fn position_at_preferred_x_in(
    layout_index: &LayoutIndex,
    node: &Dot,
    at_end: bool,
    x: f32,
) -> Option<Position> {
    let nav = first_navigable_entry_inside(layout_index, node, at_end)?;
    match nav.content(layout_index)? {
        LayoutContent::Line(line) => Some(position_in_line(line, &nav.rect, x)),
        _ => None,
    }
}

pub(crate) fn is_at_edge_line_of(
    layout_index: &LayoutIndex,
    node: &Dot,
    head: &Position,
    at_end: bool,
) -> bool {
    let Some(entry) = layout_index.entry_for_position(head) else {
        return false;
    };
    if !matches!(entry.content(layout_index), Some(LayoutContent::Line(_))) {
        return false;
    }
    edge_band_contains(layout_index, node, entry, at_end)
}

pub(crate) fn next_navigable_entry<'a>(
    layout_index: &'a LayoutIndex,
    entry: &LayoutEntry,
) -> Option<&'a LayoutEntry> {
    let idx = layout_index.entry_index(entry)?;
    layout_index
        .entries()
        .skip(idx + 1)
        .find(|entry| is_navigable_entry(layout_index, entry))
}

pub(crate) fn prev_navigable_entry<'a>(
    layout_index: &'a LayoutIndex,
    entry: &LayoutEntry,
) -> Option<&'a LayoutEntry> {
    let idx = layout_index.entry_index(entry)?;
    layout_index
        .entries()
        .take(idx)
        .rev()
        .find(|entry| is_navigable_entry(layout_index, entry))
}

fn first_navigable_entry(layout_index: &LayoutIndex) -> Option<&LayoutEntry> {
    layout_index
        .entries()
        .find(|entry| is_navigable_entry(layout_index, entry))
}

fn last_navigable_entry(layout_index: &LayoutIndex) -> Option<&LayoutEntry> {
    layout_index
        .entries()
        .rev()
        .find(|entry| is_navigable_entry(layout_index, entry))
}

fn first_navigable_entry_inside<'a>(
    layout_index: &'a LayoutIndex,
    node: &Dot,
    at_end: bool,
) -> Option<&'a LayoutEntry> {
    let mut entries = layout_index
        .entries()
        .filter(|entry| is_navigable_inside(layout_index, entry, node));
    if at_end {
        entries.next_back()
    } else {
        entries.next()
    }
}

pub(crate) fn navigable_below_at_x(
    layout_index: &LayoutIndex,
    y_threshold: f32,
    x: f32,
) -> Option<&LayoutEntry> {
    let candidates: Vec<&LayoutEntry> = layout_index
        .entries()
        .filter(|entry| is_navigable_entry(layout_index, entry) && entry.rect.y >= y_threshold)
        .collect();
    let top = candidates.iter().copied().min_by(|a, b| {
        a.rect
            .y
            .total_cmp(&b.rect.y)
            .then(a.rect.x.total_cmp(&b.rect.x))
    })?;
    let band_end = top.rect.bottom();
    candidates
        .into_iter()
        .filter(|entry| entry.rect.y < band_end)
        .min_by(|a, b| compare_navigation_band_entry(a, b, x, true))
}

pub(crate) fn navigable_above_at_x(
    layout_index: &LayoutIndex,
    y_threshold: f32,
    x: f32,
) -> Option<&LayoutEntry> {
    let candidates: Vec<&LayoutEntry> = layout_index
        .entries()
        .filter(|entry| {
            is_navigable_entry(layout_index, entry) && entry.rect.bottom() <= y_threshold
        })
        .collect();
    let bottom = candidates.iter().copied().min_by(|a, b| {
        b.rect
            .bottom()
            .total_cmp(&a.rect.bottom())
            .then(a.rect.x.total_cmp(&b.rect.x))
    })?;
    let band_start = bottom.rect.y;
    candidates
        .into_iter()
        .filter(|entry| entry.rect.bottom() > band_start)
        .min_by(|a, b| compare_navigation_band_entry(a, b, x, false))
}

fn edge_band_contains(
    layout_index: &LayoutIndex,
    node: &Dot,
    entry: &LayoutEntry,
    at_end: bool,
) -> bool {
    let entries = layout_index
        .entries()
        .filter(|entry| is_navigable_inside(layout_index, entry, node));
    let Some(edge) = (if at_end {
        entries.max_by(|a, b| {
            a.rect
                .bottom()
                .total_cmp(&b.rect.bottom())
                .then(a.rect.x.total_cmp(&b.rect.x))
        })
    } else {
        entries.min_by(|a, b| {
            a.rect
                .y
                .total_cmp(&b.rect.y)
                .then(a.rect.x.total_cmp(&b.rect.x))
        })
    }) else {
        return false;
    };
    if at_end {
        entry.rect.bottom() > edge.rect.y
    } else {
        entry.rect.y < edge.rect.bottom()
    }
}

fn is_navigable_entry(layout_index: &LayoutIndex, entry: &LayoutEntry) -> bool {
    matches!(
        entry.content(layout_index),
        Some(LayoutContent::Line(_) | LayoutContent::Atom(_))
    )
}

fn is_navigable_inside(layout_index: &LayoutIndex, entry: &LayoutEntry, node: &Dot) -> bool {
    is_navigable_entry(layout_index, entry) && entry.ancestors().contains(node)
}

fn compare_navigation_band_entry(
    a: &LayoutEntry,
    b: &LayoutEntry,
    x: f32,
    forward: bool,
) -> std::cmp::Ordering {
    let key = |entry: &LayoutEntry| {
        let group = if contains_x(&entry.rect, x) { 0u8 } else { 1u8 };
        let y = if forward {
            entry.rect.y
        } else {
            -entry.rect.bottom()
        };
        (group, axis_distance(entry.rect.x, entry.rect.right(), x), y)
    };
    let (a_group, a_dx, a_y) = key(a);
    let (b_group, b_dx, b_y) = key(b);
    a_group
        .cmp(&b_group)
        .then(a_dx.total_cmp(&b_dx))
        .then(a_y.total_cmp(&b_y))
}

fn contains_x(rect: &editor_common::Rect, x: f32) -> bool {
    x >= rect.x && x <= rect.right()
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

fn unit_attachment(layout_index: &LayoutIndex, entry: &LayoutEntry) -> Option<ChildAttachment> {
    match entry.content(layout_index)? {
        LayoutContent::Atom(atom) => Some(atom.attachment.clone()),
        LayoutContent::Box(b) if b.style.monolithic => b.attachment.clone(),
        LayoutContent::Box(_) => None,
        LayoutContent::Line(_) | LayoutContent::Spacing(_) => None,
    }
}

fn unit_selection(nv: ChildAttachment, forward: bool) -> Selection {
    let front = Position {
        node: nv.parent,
        offset: nv.index,
        affinity: Affinity::Downstream,
    };
    let back = Position {
        node: nv.parent,
        offset: nv.index + 1,
        affinity: Affinity::Upstream,
    };
    if forward {
        Selection::new(front, back)
    } else {
        Selection::new(back, front)
    }
}

pub(crate) fn landed_entry(
    layout_index: &LayoutIndex,
    entry: &LayoutEntry,
    at_end: bool,
    forward: bool,
) -> Selection {
    if let Some(nv) = unit_attachment(layout_index, entry) {
        return unit_selection(nv, forward);
    }
    let pos = if at_end {
        last_position_in_entry(layout_index, entry)
    } else {
        first_position_in_entry(layout_index, entry)
    };
    Selection::collapsed(pos)
}

fn navigate_to_entry(
    layout_index: &LayoutIndex,
    entry: &LayoutEntry,
    preferred_x: f32,
) -> Selection {
    match entry.content(layout_index) {
        Some(LayoutContent::Line(line)) => {
            Selection::collapsed(position_in_line(line, &entry.rect, preferred_x))
        }
        Some(LayoutContent::Atom(atom)) => unit_selection(atom.attachment.clone(), true),
        _ => {
            unreachable!("navigate_to_entry called on non-navigable")
        }
    }
}

fn position_in_line(line: &LayoutLine, rect: &editor_common::Rect, x: f32) -> Position {
    let local_x = x - rect.x;
    super::grapheme::position_at_x(line, local_x)
}

fn compute_preferred_x(layout_index: &LayoutIndex, entry: &LayoutEntry, pos: &Position) -> f32 {
    match entry.content(layout_index) {
        Some(LayoutContent::Line(line)) => entry.rect.x + x_at_offset(line, pos),
        _ => entry.rect.x,
    }
}

#[cfg(test)]
mod tests {
    use editor_common::Size;
    use editor_common::{Axis, Direction, EdgeInsets, Movement};
    use editor_crdt::{Dot, InputEvent, ListOp, build_oplog};
    use editor_model::{
        DocLogs, DocView, ModifierAttrLog, NodeAttrLog, NodeMarkerLog, NodeStyleLog, NodeType,
        SeqItem, SpanLog, StyleLog, project_document,
    };
    use editor_resource::Resource;
    use editor_state::Position;

    use crate::glyph_run::GlyphRun;
    use crate::glyph_run::{GraphemeSpan, Synthesis, TextDecoration};
    use crate::measure::context::MeasureContext;
    use crate::measure::nodes::dispatch::measure_node;
    use crate::measure::types::MeasuredTree;
    use crate::page::LayoutPage;
    use crate::paginate::paginator::Paginator;
    use crate::paginate::types::{LayoutBox, LayoutContent, LayoutLine, LayoutNode, LayoutTree};
    use crate::style::BoxStyle;
    use crate::viewport::Viewport;

    use super::*;

    fn viewport() -> Viewport {
        Viewport::new(800.0, 600.0, 1.0)
    }

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

    fn build_index(doc: &DocLogs, width: f32) -> (Dot, LayoutIndex) {
        let pd = project_document(doc).unwrap();
        let view = DocView::new(&pd);
        let root_node = view.root().unwrap();
        let root_id = root_node.id();
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
        (root_id, index)
    }

    fn para_doc(text: &str, width: f32) -> (Dot, Dot, LayoutIndex) {
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
        let (root_id, index) = build_index(&doc, width);
        (root_id, para_id, index)
    }

    fn two_line_doc(text: &str, width: f32) -> (Dot, LayoutIndex) {
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
        let (_, index) = build_index(&doc, width);
        (para_id, index)
    }

    fn gs(advance: f32, codepoints: u8) -> GraphemeSpan {
        GraphemeSpan {
            advance,
            codepoints,
        }
    }

    fn vrun(
        offset_range: std::ops::Range<usize>,
        x: f32,
        text: &str,
        graphemes: Vec<GraphemeSpan>,
    ) -> GlyphRun {
        let width = graphemes.iter().map(|g| g.advance).sum();
        GlyphRun {
            family_id: 0,
            weight: 400,
            font_size: 16.0,
            synthesis: Synthesis::default(),
            color: String::new(),
            background_color: None,
            glyphs: vec![],
            decoration: TextDecoration::default(),
            offset_range,
            link: None,
            text: text.to_string(),
            x,
            width,
            graphemes,
            cursor_ascent: 0.0,
            cursor_descent: 0.0,
        }
    }

    fn vline(
        node: Dot,
        offset_range: Option<std::ops::Range<usize>>,
        runs: Vec<GlyphRun>,
    ) -> LayoutLine {
        LayoutLine {
            measured: std::sync::Arc::new(crate::measure::text::measure::MeasuredLine {
                height: 0.0,
                node,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: runs,
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                offset_range,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        }
    }

    fn make_single_line_index(para_id: Dot, line: LayoutLine, line_height: f32) -> LayoutIndex {
        let root_id = Dot::ROOT;
        let line_rect = editor_common::Rect::from_xywh(0.0, 0.0, 200.0, line_height);
        let para_rect = editor_common::Rect::from_xywh(0.0, 0.0, 200.0, line_height);
        let root_rect = editor_common::Rect::from_xywh(0.0, 0.0, 200.0, line_height);

        let tree = LayoutTree {
            root: LayoutNode {
                rect: root_rect,
                content: LayoutContent::Box(LayoutBox {
                    node: root_id,
                    style: BoxStyle::default(),
                    children: vec![LayoutNode {
                        rect: para_rect,
                        content: LayoutContent::Box(LayoutBox {
                            node: para_id,
                            style: BoxStyle::default(),
                            children: vec![LayoutNode {
                                rect: line_rect,
                                content: LayoutContent::Line(line),
                            }],
                            attachment: None,
                        }),
                    }],
                    attachment: None,
                }),
            },
        };

        let page = LayoutPage::new(
            0.0,
            line_height + 1.0,
            Size {
                width: 200.0,
                height: line_height + 1.0,
            },
        );
        LayoutIndex::new(tree, &[page])
    }

    #[test]
    fn grapheme_forward_backward() {
        let (_root_id, para_id, index) = para_doc("Hello", 400.0);
        let vp = viewport();
        let res = Resource::new_test();

        let pos = Position::new(para_id, 2);
        let (sel, _) = resolve_movement(
            &index,
            &pos,
            &Movement::Grapheme {
                direction: Direction::Forward,
            },
            &vp,
            &res,
            None,
        );
        let sel = sel.expect("forward from 2 must yield a selection");
        assert_eq!(sel.head.node, para_id);
        assert_eq!(sel.head.offset, 3);

        let pos3 = Position::new(para_id, 3);
        let (sel_back, _) = resolve_movement(
            &index,
            &pos3,
            &Movement::Grapheme {
                direction: Direction::Backward,
            },
            &vp,
            &res,
            None,
        );
        let sel_back = sel_back.expect("backward from 3 must yield a selection");
        assert_eq!(sel_back.head.node, para_id);
        assert_eq!(sel_back.head.offset, 2);
    }

    #[test]
    fn line_horizontal() {
        let (_root_id, para_id, index) = para_doc("Hello", 400.0);
        let vp = viewport();
        let res = Resource::new_test();

        let pos = Position::new(para_id, 2);

        let (end_sel, _) = resolve_movement(
            &index,
            &pos,
            &Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Horizontal,
            },
            &vp,
            &res,
            None,
        );
        let end_sel = end_sel.expect("line-end must resolve");
        assert_eq!(end_sel.head.node, para_id);
        assert_eq!(
            end_sel.head.offset, 5,
            "line-end must be at offset 5 (after 'Hello')"
        );

        let (start_sel, _) = resolve_movement(
            &index,
            &pos,
            &Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Horizontal,
            },
            &vp,
            &res,
            None,
        );
        let start_sel = start_sel.expect("line-start must resolve");
        assert_eq!(start_sel.head.node, para_id);
        assert_eq!(start_sel.head.offset, 0, "line-start must be at offset 0");
    }

    #[test]
    fn line_vertical() {
        let text: String = "a".repeat(20);
        let (para_id, index) = two_line_doc(&text, 60.0);
        let vp = viewport();
        let res = Resource::new_test();

        let pos = Position::new(para_id, 0);
        let (down_sel, pref_x) = resolve_movement(
            &index,
            &pos,
            &Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
            &vp,
            &res,
            None,
        );
        let down_sel = down_sel.expect("vertical-down must resolve to line below");
        assert_eq!(
            down_sel.head.node, para_id,
            "vertical move must stay in same para node"
        );
        assert!(
            down_sel.head.offset > 0,
            "offset must advance to line below (got {})",
            down_sel.head.offset
        );
        assert!(pref_x.is_some(), "vertical move must return preferred x");

        let (up_sel, _) = resolve_movement(
            &index,
            &down_sel.head,
            &Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Vertical,
            },
            &vp,
            &res,
            pref_x,
        );
        let up_sel = up_sel.expect("vertical-up must resolve");
        assert_eq!(up_sel.head.node, para_id);
        assert_eq!(
            up_sel.head.offset, 0,
            "vertical-up with preserved preferred-x must return to offset 0"
        );
    }

    #[test]
    fn multi_run_word_boundary() {
        let para_id = Dot::new(1, 1);

        let run0 = vrun(0..6, 0.0, "hello ", vec![gs(10.0, 1); 6]);
        let run1 = vrun(6..11, 60.0, "world", vec![gs(10.0, 1); 5]);
        let line = vline(para_id, Some(0..11), vec![run0, run1]);

        let index = make_single_line_index(para_id, line, 20.0);
        let res = Resource::new_test();

        // From offset 5 (end of "hello" in run0), word-forward skips the space
        // in run0 and lands at offset 11 (end of "world" in run1). This proves
        // the word-boundary logic crosses the run boundary using offset_range only.
        let pos_end_of_hello = Position::new(para_id, 5);
        let sel =
            super::segmentation::move_word_forward(&index, &pos_end_of_hello, &res.segmenters);
        let sel = sel.expect("word-forward from end-of-hello must resolve");
        assert_eq!(sel.head.node, para_id);
        assert_eq!(
            sel.head.offset, 11,
            "word-forward from offset 5 must cross run boundary and land at offset 11 (end of 'world')"
        );

        // From offset 11 (end of run1), word-backward must land in run1's territory.
        let pos_end = Position::new(para_id, 11);
        let sel_back = super::segmentation::move_word_backward(&index, &pos_end, &res.segmenters);
        let sel_back = sel_back.expect("word-backward from end must resolve");
        assert_eq!(sel_back.head.node, para_id);
        assert_eq!(
            sel_back.head.offset, 6,
            "word-backward from offset 11 must land at offset 6 (start of 'world' in run1)"
        );
    }

    #[test]
    fn editable_and_edge() {
        let (_root_id, para_id, index) = para_doc("Hello", 400.0);

        let first = editable_position_inside(&index, &para_id, false)
            .expect("editable_position_inside(at_end=false) must return Some");
        assert_eq!(first.node, para_id);
        assert_eq!(first.offset, 0, "first editable position must be offset 0");

        let last = editable_position_inside(&index, &para_id, true)
            .expect("editable_position_inside(at_end=true) must return Some");
        assert_eq!(last.node, para_id);
        assert_eq!(
            last.offset, 5,
            "last editable position must be offset 5 (end of 'Hello')"
        );

        assert!(
            is_at_edge_line_of(&index, &para_id, &first, false),
            "first position must be on edge line (at_end=false)"
        );
        assert!(
            is_at_edge_line_of(&index, &para_id, &last, true),
            "last position must be on edge line (at_end=true)"
        );

        let mid = Position::new(para_id, 2);
        assert!(
            is_at_edge_line_of(&index, &para_id, &mid, false),
            "mid position is still on the only line, so it is on the start-edge line"
        );
    }
}
