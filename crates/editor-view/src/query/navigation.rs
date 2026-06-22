use editor_common::{Axis, Direction, Movement};
use editor_model::NodeId;
use editor_resource::Resource;
use editor_state::{Affinity, Position, Selection};

use crate::paginate::*;
use crate::viewport::Viewport;

use super::cursor::x_at_offset;
use super::layout_index::{LayoutEntry, LayoutIndex};
use super::segmentation;

pub fn resolve_movement(
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
            // An empty hard_break line owns paragraph offsets via child_range
            // but has no glyph runs to step through; the new affinity keeps
            // the result on this line so normalize doesn't bounce to the
            // upper line.
            if line.glyph_runs.is_empty()
                && let Some(range) = &line.child_range
                && pos.node_id == line.node_id
                && pos.offset >= range.start
                && pos.offset < range.end
            {
                return Some(Selection::collapsed(Position {
                    node_id: line.node_id,
                    offset: pos.offset + 1,
                    affinity: Affinity::Upstream,
                }));
            }
            for (i, run) in line.glyph_runs.iter().enumerate() {
                if run.node_id != pos.node_id {
                    continue;
                }
                let local = pos.offset - run.offset;
                let mut cp_acc = 0usize;
                for g in &run.graphemes {
                    let cp = g.codepoints as usize;
                    if local < cp_acc + cp {
                        return Some(Selection::collapsed(Position::new(
                            run.node_id,
                            run.offset + cp_acc + cp,
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
                            next.node_id,
                            next.offset,
                        )));
                    }
                    if let Some(g) = next.graphemes.first() {
                        return Some(Selection::collapsed(Position::new(
                            next.node_id,
                            next.offset + g.codepoints as usize,
                        )));
                    }
                }
            }
            let next = next_navigable_entry(layout_index, entry)?;
            // If the next line is a soft-wrap continuation of the same text node,
            // advance directly into its first grapheme rather than landing at
            // the same offset (which would visually be the same caret).
            if let Some(LayoutContent::Line(next_line)) = next.content(layout_index)
                && let Some(first_run) = next_line.glyph_runs.first()
                && first_run.node_id == pos.node_id
                && first_run.offset == pos.offset
                && let Some(g) = first_run.graphemes.first()
            {
                return Some(Selection::collapsed(Position::new(
                    pos.node_id,
                    pos.offset + g.codepoints as usize,
                )));
            }
            // Landing at the empty line's start would re-stage the same
            // caret; normalize then locks head Upstream and the next press
            // loops on the upper line.
            if let Some(LayoutContent::Line(next_line)) = next.content(layout_index)
                && next_line.glyph_runs.is_empty()
                && let Some(range) = &next_line.child_range
                && pos.node_id == next_line.node_id
                && pos.offset == range.start
                && range.start < range.end
            {
                return Some(Selection::collapsed(Position {
                    node_id: next_line.node_id,
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
                Some(Selection::collapsed(Position::new(
                    nv.parent_id,
                    nv.index + 1,
                )))
            }
        }
    }
}

fn move_grapheme_backward(layout_index: &LayoutIndex, pos: &Position) -> Option<Selection> {
    let entry = layout_index.entry_for_position(pos)?;

    match entry.content(layout_index)? {
        LayoutContent::Line(line) => {
            // Symmetric to forward; Downstream keeps the result on this line.
            if line.glyph_runs.is_empty()
                && let Some(range) = &line.child_range
                && pos.node_id == line.node_id
                && pos.offset > range.start
                && pos.offset <= range.end
            {
                return Some(Selection::collapsed(Position {
                    node_id: line.node_id,
                    offset: pos.offset - 1,
                    affinity: Affinity::Downstream,
                }));
            }
            for (i, run) in line.glyph_runs.iter().enumerate() {
                if run.node_id != pos.node_id {
                    continue;
                }
                if pos.offset > run.offset {
                    let local = pos.offset - run.offset;
                    let mut cp_acc = 0usize;
                    let mut prev_boundary = 0usize;
                    for g in &run.graphemes {
                        let cp = g.codepoints as usize;
                        if cp_acc + cp >= local {
                            return Some(Selection::collapsed(Position::new(
                                run.node_id,
                                run.offset + prev_boundary,
                            )));
                        }
                        prev_boundary = cp_acc + cp;
                        cp_acc += cp;
                    }
                }
                if pos.offset == run.offset && i > 0 {
                    let prev = &line.glyph_runs[i - 1];
                    let total = super::grapheme::run_codepoint_count(prev);
                    let separated_by_tab = line
                        .tab_gaps
                        .iter()
                        .any(|gap| gap.x >= prev.x - 0.5 && gap.x + gap.width <= run.x + 0.5);
                    if separated_by_tab {
                        return Some(Selection::collapsed(Position::new(
                            prev.node_id,
                            prev.offset + total,
                        )));
                    }
                    if let Some(g) = prev.graphemes.last() {
                        return Some(Selection::collapsed(Position::new(
                            prev.node_id,
                            prev.offset + total - g.codepoints as usize,
                        )));
                    }
                }
            }
            let prev = prev_navigable_entry(layout_index, entry)?;
            // If the previous line is a soft-wrap continuation of the same
            // text node, retreat into its last grapheme rather than landing
            // at the same offset (which would visually be the same caret).
            if let Some(LayoutContent::Line(prev_line)) = prev.content(layout_index)
                && let Some(last_run) = prev_line.glyph_runs.last()
                && last_run.node_id == pos.node_id
                && last_run.offset + super::grapheme::run_codepoint_count(last_run) == pos.offset
                && let Some(g) = last_run.graphemes.last()
            {
                return Some(Selection::collapsed(Position::new(
                    pos.node_id,
                    pos.offset - g.codepoints as usize,
                )));
            }
            // Mirror of forward. `normalize_position` may have descended a
            // paragraph-level pos into the adjacent text node's start, so
            // accept both shapes.
            if let Some(LayoutContent::Line(prev_line)) = prev.content(layout_index)
                && prev_line.glyph_runs.is_empty()
                && let Some(prev_range) = &prev_line.child_range
                && prev_range.start < prev_range.end
            {
                let at_para_boundary =
                    pos.node_id == prev_line.node_id && pos.offset == prev_range.end;
                let at_line_start = line
                    .glyph_runs
                    .first()
                    .is_some_and(|r| r.node_id == pos.node_id && r.offset == pos.offset);
                if at_para_boundary || at_line_start {
                    return Some(Selection::collapsed(Position {
                        node_id: prev_line.node_id,
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
                Some(Selection::collapsed(Position::new(nv.parent_id, nv.index)))
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

/// A trailing empty line owns no children (zero-width `child_range`), so
/// `escape_empty_line_trap` can't push off it.
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
    let Some(range) = &line.child_range else {
        return sel;
    };
    if range.start != range.end {
        return sel;
    }
    if sel.head.node_id != pos.node_id || sel.head.offset != pos.offset {
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

/// At the document edge return the current line's end instead of `None`, so
/// the navigation handler doesn't silently swallow the keypress.
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

/// `position_at_x` on an empty line collapses to its `child_range.start`,
/// which can equal the input position when crossing an empty hard_break line
/// — making the press a visible no-op. Push to the far end of the range so
/// the next press exits in the same direction.
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
    if sel.head.node_id != pos.node_id || sel.head.offset != pos.offset {
        return sel;
    }
    let Some(range) = &line.child_range else {
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
        node_id: line.node_id,
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
        return Position::new(line.node_id, gap.child_index);
    }
    if let Some(run) = run_first {
        return Position::new(run.node_id, run.offset);
    }
    let offset = line.child_range.as_ref().map(|r| r.start).unwrap_or(0);
    Position::new(line.node_id, offset)
}

fn last_position_in_line(line: &LayoutLine) -> Position {
    super::grapheme::last_position_in_line(line)
}

fn first_position_in_entry(layout_index: &LayoutIndex, entry: &LayoutEntry) -> Position {
    match entry.content(layout_index) {
        Some(LayoutContent::Line(line)) => first_position_in_line(line),
        Some(LayoutContent::Atom(atom)) => {
            Position::new(atom.attachment.parent_id, atom.attachment.index)
        }
        Some(LayoutContent::Box(b)) if b.style.monolithic && b.attachment.is_some() => {
            let attachment = b.attachment.expect("checked is_some");
            Position::new(attachment.parent_id, attachment.index)
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
            Position::new(atom.attachment.parent_id, atom.attachment.index)
        }
        Some(LayoutContent::Box(b)) if b.style.monolithic && b.attachment.is_some() => {
            let attachment = b.attachment.expect("checked is_some");
            Position::new(attachment.parent_id, attachment.index)
        }
        _ => {
            unreachable!("last_position_in_entry called on non-navigable entry")
        }
    }
}

pub(super) fn move_box_boundary(
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
    b.attachment?;
    let idx = layout_index.entry_index(entry)?;
    Some(match pos.affinity {
        Affinity::Downstream => idx,
        Affinity::Upstream => index_after_box_subtree(layout_index, idx, b.node_id),
    })
}

fn index_after_box_subtree(layout_index: &LayoutIndex, idx: usize, node_id: NodeId) -> usize {
    layout_index
        .entries()
        .enumerate()
        .skip(idx + 1)
        .find(|(_, entry)| !entry.ancestors().contains(&node_id))
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

/// First (`at_end=false`) / last (`at_end=true`) editable caret position
/// *inside* `node_id`'s subtree — descends past the node's own attachment
/// boundary into its children. `None` when the node is not a Box (atoms
/// have no inner navigable) or has no navigable descendant.
pub(crate) fn editable_position_inside(
    layout_index: &LayoutIndex,
    node_id: NodeId,
    at_end: bool,
) -> Option<Position> {
    let nav = first_navigable_entry_inside(layout_index, node_id, at_end)?;
    Some(if at_end {
        last_position_in_entry(layout_index, nav)
    } else {
        first_position_in_entry(layout_index, nav)
    })
}

/// `pos`'s containing-Line x-coordinate, used as the preferred_x seed
/// when vertical gap entry bypasses `resolve_movement` (which is the
/// usual recorder). Returning `None` is safe: a missing seed just falls
/// back to recomputation on the next vertical move.
pub(crate) fn compute_preferred_x_at(layout_index: &LayoutIndex, pos: &Position) -> Option<f32> {
    let entry = layout_index.entry_for_position(pos)?;
    Some(compute_preferred_x(layout_index, entry, pos))
}

/// A caret Position at `x` on the first (`at_end=false`) or last
/// (`at_end=true`) navigable Line inside `node_id`. `None` when the
/// edge navigable is an Atom (no column concept) — callers must fall
/// back to their default exit (node-select or first/last position).
pub(crate) fn position_at_preferred_x_in(
    layout_index: &LayoutIndex,
    node_id: NodeId,
    at_end: bool,
    x: f32,
) -> Option<Position> {
    let nav = first_navigable_entry_inside(layout_index, node_id, at_end)?;
    match nav.content(layout_index)? {
        LayoutContent::Line(line) => Some(position_in_line(line, &nav.rect, x)),
        _ => None,
    }
}

/// `head`가 속한 Line이 `node_id`의 첫(`at_end=false`) 또는 마지막
/// (`at_end=true`) 시각적 행에 있으면 true. 세로(Line) 갭 진입에 쓰인다.
///
/// 기본은 박스 서브트리의 유일한 첫/마지막 navigable 라인과의 포인터 비교다
/// (콜아웃·폴드처럼 가장자리 자식이 여러 줄로 접히는 단락 하나일 때, 그
/// 마지막 줄에서만 벗어나야 하므로). 단, 테이블처럼 가장자리 행이 가로로
/// 나열된 여러 셀(각자 navigable을 가진 별도 자식 박스)로 이루어진 경우엔
/// 그 행 안의 어느 셀이든 가장자리로 본다 — 그래야 마지막 행이 마지막 열뿐
/// 아니라 어느 열에서도 갭으로 진입한다.
pub(crate) fn is_at_edge_line_of(
    layout_index: &LayoutIndex,
    node_id: NodeId,
    head: &Position,
    at_end: bool,
) -> bool {
    let Some(entry) = layout_index.entry_for_position(head) else {
        return false;
    };
    if !matches!(entry.content(layout_index), Some(LayoutContent::Line(_))) {
        return false;
    }
    edge_band_contains(layout_index, node_id, entry, at_end)
}

pub(super) fn next_navigable_entry<'a>(
    layout_index: &'a LayoutIndex,
    entry: &LayoutEntry,
) -> Option<&'a LayoutEntry> {
    let idx = layout_index.entry_index(entry)?;
    layout_index
        .entries()
        .skip(idx + 1)
        .find(|entry| is_navigable_entry(layout_index, entry))
}

pub(super) fn prev_navigable_entry<'a>(
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

fn first_navigable_entry_inside(
    layout_index: &LayoutIndex,
    node_id: NodeId,
    at_end: bool,
) -> Option<&LayoutEntry> {
    let mut entries = layout_index
        .entries()
        .filter(|entry| is_navigable_inside(layout_index, entry, node_id));
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
    node_id: NodeId,
    entry: &LayoutEntry,
    at_end: bool,
) -> bool {
    let entries = layout_index
        .entries()
        .filter(|entry| is_navigable_inside(layout_index, entry, node_id));
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

fn is_navigable_inside(layout_index: &LayoutIndex, entry: &LayoutEntry, node_id: NodeId) -> bool {
    is_navigable_entry(layout_index, entry) && entry.ancestors().contains(&node_id)
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
        LayoutContent::Atom(atom) => Some(atom.attachment),
        LayoutContent::Box(b) if b.style.monolithic => b.attachment,
        LayoutContent::Box(_) => None,
        LayoutContent::Line(_) | LayoutContent::Spacing(_) => None,
    }
}

fn unit_selection(nv: ChildAttachment, forward: bool) -> Selection {
    let front = Position {
        node_id: nv.parent_id,
        offset: nv.index,
        affinity: Affinity::Downstream,
    };
    let back = Position {
        node_id: nv.parent_id,
        offset: nv.index + 1,
        affinity: Affinity::Upstream,
    };
    if forward {
        Selection::new(front, back)
    } else {
        Selection::new(back, front)
    }
}

/// `at_end` and `forward` are independent concerns: `at_end` selects which end of a
/// Line to place the collapsed caret; `forward` controls the direction of the atom
/// node selection. An atom does not have a "start/end", so `at_end` is irrelevant
/// there, and a Line does not have a direction, so `forward` is irrelevant there.
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
        Some(LayoutContent::Atom(atom)) => unit_selection(atom.attachment, true),
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
    use super::*;
    use crate::glyph_run::{GlyphRun, GraphemeSpan};
    use crate::page::LayoutPage;
    use crate::style::Alignment;
    use crate::style::{BorderMode, BoxStyle, Direction as LayoutDirection};
    use crate::view::View;
    use editor_common::{Axis, Direction, EdgeInsets, Movement, Rect};
    use editor_macros::state;
    use editor_model::NodeId;
    use editor_resource::Resource;
    use editor_state::Position;

    fn gs(n: usize) -> Vec<GraphemeSpan> {
        vec![
            GraphemeSpan {
                advance: 10.0,
                codepoints: 1
            };
            n
        ]
    }

    fn make_line_node(id: NodeId, y: f32, text: &str) -> LayoutNode {
        let n = text.chars().count();
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: id,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![GlyphRun::make_test_run(id, 0, text, 0.0, gs(n))],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        }
    }

    fn make_atom_node(parent_id: NodeId, node_id: NodeId, y: f32, index: usize) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, 20.0),
            content: LayoutContent::Atom(LayoutAtom {
                node_id,
                attachment: ChildAttachment { parent_id, index },
            }),
        }
    }

    fn make_box_node(y: f32, h: f32, children: Vec<LayoutNode>) -> LayoutNode {
        make_box_node_with_id(NodeId::new(), y, h, children)
    }

    fn make_box_node_with_id(
        node_id: NodeId,
        y: f32,
        h: f32,
        children: Vec<LayoutNode>,
    ) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, h),
            content: LayoutContent::Box(LayoutBox {
                node_id,
                style: BoxStyle {
                    direction: LayoutDirection::Vertical,
                    padding: EdgeInsets::ZERO,
                    border: EdgeInsets::ZERO,
                    border_mode: BorderMode::Separate,
                    alignment: Alignment::Start,
                    decorations: vec![],
                    monolithic: false,
                },
                children,
                attachment: None,
            }),
        }
    }

    fn make_attached_box_node(
        parent_id: NodeId,
        index: usize,
        node_id: NodeId,
        y: f32,
        h: f32,
        children: Vec<LayoutNode>,
    ) -> LayoutNode {
        let mut node = make_box_node_with_id(node_id, y, h, children);
        let LayoutContent::Box(b) = &mut node.content else {
            unreachable!("make_box_node_with_id returns a box");
        };
        b.attachment = Some(ChildAttachment { parent_id, index });
        node
    }

    fn make_paragraph_line(
        paragraph_id: NodeId,
        text_id: NodeId,
        y: f32,
        text: &str,
    ) -> LayoutNode {
        let n = text.chars().count();
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 200.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: paragraph_id,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![GlyphRun::make_test_run(text_id, 0, text, 0.0, gs(n))],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        }
    }

    fn make_spacing(y: f32, h: f32) -> LayoutNode {
        LayoutNode {
            rect: Rect::from_xywh(0.0, y, 0.0, h),
            content: LayoutContent::Spacing(SpacingKind::Gap {
                position: Position::new(NodeId::ROOT, 0),
            }),
        }
    }

    /// Shared fixture:
    ///
    /// ```text
    /// Block1 (y=0..40):
    ///   Line0: "hello world"  (y=0,  h=20)
    ///   Line1: "foo bar"      (y=20, h=20)
    /// Block2 (y=40..120):
    ///   Atom                  (y=40, h=20, parent=atom_parent, index=0)
    ///   Line2: "baz"          (y=60, h=20)
    ///   Line3: "qux quux"     (y=80, h=20)
    ///   Line4: "end"          (y=100, h=20)
    /// ```
    struct Fixture {
        tree: LayoutTree,
        lines: [NodeId; 5],
        atom_parent: NodeId,
    }

    fn fixture() -> Fixture {
        let lines: [NodeId; 5] = std::array::from_fn(|_| NodeId::new());
        let atom_parent = NodeId::new();
        let atom_id = NodeId::new();

        let tree = LayoutTree {
            root: make_box_node(
                0.0,
                120.0,
                vec![
                    make_box_node(
                        0.0,
                        40.0,
                        vec![
                            make_line_node(lines[0], 0.0, "hello world"),
                            make_line_node(lines[1], 20.0, "foo bar"),
                        ],
                    ),
                    make_box_node(
                        40.0,
                        80.0,
                        vec![
                            make_atom_node(atom_parent, atom_id, 40.0, 0),
                            make_line_node(lines[2], 60.0, "baz"),
                            make_line_node(lines[3], 80.0, "qux quux"),
                            make_line_node(lines[4], 100.0, "end"),
                        ],
                    ),
                ],
            ),
        };

        Fixture {
            tree,
            lines,
            atom_parent,
        }
    }

    const VP: Viewport = Viewport {
        width: 200.0,
        height: 800.0,
        scale_factor: 1.0,
    };

    fn mov(tree: &LayoutTree, pos: Position, movement: Movement) -> Option<Selection> {
        resolve(tree, &pos, &movement, &VP, &Resource::new_test(), None).0
    }

    fn resolve(
        tree: &LayoutTree,
        pos: &Position,
        movement: &Movement,
        viewport: &Viewport,
        resource: &Resource,
        preferred_x: Option<f32>,
    ) -> (Option<Selection>, Option<f32>) {
        let pages = [LayoutPage::new(
            0.0,
            10_000.0,
            editor_common::Size::new(1_000.0, 10_000.0),
        )];
        let layout_index = LayoutIndex::new(tree.clone(), &pages);
        super::resolve_movement(
            &layout_index,
            pos,
            movement,
            viewport,
            resource,
            preferred_x,
        )
    }

    /// Tree with a single text node `t` soft-wrapped onto two visual lines:
    ///   line A (y=0..20): glyph_run(t, offset=0, "abcde")
    ///   line B (y=20..40): glyph_run(t, offset=5, "fghij")
    fn soft_wrap_tree(t: NodeId) -> LayoutTree {
        let line_a = LayoutNode {
            rect: Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: t,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![GlyphRun::make_test_run(t, 0, "abcde", 0.0, gs(5))],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        };
        let line_b = LayoutNode {
            rect: Rect::from_xywh(0.0, 20.0, 200.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: t,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![GlyphRun::make_test_run(t, 5, "fghij", 0.0, gs(5))],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        };
        LayoutTree {
            root: make_box_node(0.0, 40.0, vec![line_a, line_b]),
        }
    }

    #[test]
    fn first_position_in_line_empty_uses_child_range_start() {
        let p1 = NodeId::new();
        let line = LayoutLine {
            node_id: p1,
            baseline: 16.0,
            ascent: 14.0,
            descent: 4.0,
            cursor_ascent: 14.0,
            cursor_descent: 4.0,
            glyph_runs: vec![],
            ruby_annotations: vec![],
            empty_caret_x: 0.0,
            child_range: Some(2..2),
            tab_gaps: vec![],
            is_phantom: false,
            content_edge_x: None,
        };
        let pos = first_position_in_line(&line);
        assert_eq!(pos.node_id, p1);
        assert_eq!(pos.offset, 2);
    }

    #[test]
    fn line_horizontal_forward_in_upper_soft_wrap_renders_at_upper_end() {
        // After making position ownership affinity-aware, `last_position_in_line`
        // must produce a position that resolves back to the upper line; otherwise
        // pressing End in the upper wrapped line silently jumps to the start
        // of the lower wrapped line.
        let t = NodeId::new();
        let tree = soft_wrap_tree(t);
        let sel = mov(
            &tree,
            Position::new(t, 2),
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Horizontal,
            },
        )
        .unwrap();
        assert_eq!(sel.head.offset, 5);
        let pages = [LayoutPage::new(
            0.0,
            10_000.0,
            editor_common::Size::new(1_000.0, 10_000.0),
        )];
        let layout_index = LayoutIndex::new(tree.clone(), &pages);
        let entry = layout_index.entry_for_position(&sel.head).unwrap();
        assert_eq!(entry.rect.y, 0.0, "must resolve to upper wrapped line");
    }

    #[test]
    fn grapheme_backward_at_soft_wrap_start_retreats_into_upper_line() {
        let t = NodeId::new();
        let tree = soft_wrap_tree(t);
        // Start of lower wrapped line: (t, 5, Downstream).
        let pos = Position {
            node_id: t,
            offset: 5,
            affinity: Affinity::Downstream,
        };
        let sel = mov(
            &tree,
            pos,
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        )
        .unwrap();
        assert_eq!(sel.head.node_id, t);
        assert_eq!(sel.head.offset, 4);
    }

    #[test]
    fn grapheme_forward_at_soft_wrap_end_advances_into_lower_line() {
        let t = NodeId::new();
        let tree = soft_wrap_tree(t);
        // End of upper wrapped line: (t, 5, Upstream).
        let pos = Position {
            node_id: t,
            offset: 5,
            affinity: Affinity::Upstream,
        };
        let sel = mov(
            &tree,
            pos,
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        )
        .unwrap();
        assert_eq!(sel.head.node_id, t);
        assert_eq!(sel.head.offset, 6);
    }

    #[test]
    fn grapheme_forward() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[0], 2),
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.offset, 3);
    }

    #[test]
    fn grapheme_backward() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[0], 3),
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.offset, 2);
    }

    #[test]
    fn grapheme_forward_at_line_end_wraps() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[0], 11),
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[1]);
        assert_eq!(sel.head.offset, 0);
    }

    #[test]
    fn grapheme_backward_at_line_start_wraps() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[1], 0),
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[0]);
        assert_eq!(sel.head.offset, 11);
    }

    #[test]
    fn line_horizontal_forward() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[0], 2),
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Horizontal,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[0]);
        assert_eq!(sel.head.offset, 11);
    }

    #[test]
    fn line_horizontal_backward() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[0], 5),
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Horizontal,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[0]);
        assert_eq!(sel.head.offset, 0);
    }

    #[test]
    fn line_vertical_forward() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[0], 2),
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[1]);
    }

    #[test]
    fn line_vertical_backward() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[1], 2),
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Vertical,
            },
        )
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[0]);
    }

    #[test]
    fn line_vertical_forward_at_last_extends_to_line_end() {
        let f = fixture();
        let sel = mov(
            &f.tree,
            Position::new(f.lines[4], 0),
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        )
        .expect("falls back to end of current line at document edge");
        assert_eq!(sel.head, sel.anchor);
        // Stays on the same line, head pushed to its end position.
        assert_eq!(sel.head.node_id, f.lines[4]);
    }

    #[test]
    fn move_line_down_skips_spacing() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        let tree = LayoutTree {
            root: make_box_node(
                0.0,
                56.0,
                vec![
                    make_line_node(id1, 0.0, "hello"),
                    make_spacing(20.0, 16.0),
                    make_line_node(id2, 36.0, "world"),
                ],
            ),
        };
        let sel = mov(
            &tree,
            Position::new(id1, 2),
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
        )
        .unwrap();
        assert_eq!(sel.head.node_id, id2);
    }

    #[test]
    fn page_forward_skips_lines() {
        let f = fixture();
        let vp = Viewport {
            width: 200.0,
            height: 50.0,
            scale_factor: 1.0,
        };
        let sel = resolve(
            &f.tree,
            &Position::new(f.lines[0], 0),
            &Movement::Page {
                direction: Direction::Forward,
            },
            &vp,
            &Resource::new_test(),
            None,
        )
        .0
        .unwrap();
        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, f.lines[2]);
    }

    #[test]
    fn page_backward_skips_lines() {
        let f = fixture();
        let vp = Viewport {
            width: 200.0,
            height: 50.0,
            scale_factor: 1.0,
        };
        let (sel, _) = resolve(
            &f.tree,
            &Position::new(f.lines[4], 0),
            &Movement::Page {
                direction: Direction::Backward,
            },
            &vp,
            &Resource::new_test(),
            None,
        );
        let sel = sel.unwrap();
        assert_eq!(sel.anchor.node_id, f.atom_parent);
        assert_eq!(sel.anchor.offset, 0);
        assert_eq!(sel.head.node_id, f.atom_parent);
        assert_eq!(sel.head.offset, 1);
    }

    #[test]
    fn move_document_start_end() {
        let f = fixture();
        let sel_start = mov(
            &f.tree,
            Position::new(f.lines[4], 2),
            Movement::Document {
                direction: Direction::Backward,
            },
        )
        .unwrap();
        assert_eq!(sel_start.head, sel_start.anchor);
        assert_eq!(sel_start.head.node_id, f.lines[0]);
        assert_eq!(sel_start.head.offset, 0);

        let sel_end = mov(
            &f.tree,
            Position::new(f.lines[0], 0),
            Movement::Document {
                direction: Direction::Forward,
            },
        )
        .unwrap();
        assert_eq!(sel_end.head, sel_end.anchor);
        assert_eq!(sel_end.head.node_id, f.lines[4]);
        assert_eq!(sel_end.head.offset, 3);
    }

    #[test]
    fn grapheme_forward_at_text_node_boundary() {
        let t1 = NodeId::new();
        let t2 = NodeId::new();
        let line_node = LayoutNode {
            rect: Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: t1,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![
                    GlyphRun::make_test_run(t1, 0, "Hello", 0.0, gs(5)),
                    GlyphRun::make_test_run(t2, 0, "World", 50.0, gs(5)),
                ],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        };
        let tree = LayoutTree {
            root: make_box_node(0.0, 20.0, vec![make_box_node(0.0, 20.0, vec![line_node])]),
        };

        let sel = mov(
            &tree,
            Position::new(t1, 5),
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        )
        .unwrap();

        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, t2);
        assert_eq!(sel.head.offset, 1);
    }

    #[test]
    fn grapheme_backward_at_text_node_boundary() {
        let t1 = NodeId::new();
        let t2 = NodeId::new();
        let line_node = LayoutNode {
            rect: Rect::from_xywh(0.0, 0.0, 200.0, 20.0),
            content: LayoutContent::Line(LayoutLine {
                node_id: t1,
                baseline: 16.0,
                ascent: 14.0,
                descent: 4.0,
                cursor_ascent: 14.0,
                cursor_descent: 4.0,
                glyph_runs: vec![
                    GlyphRun::make_test_run(t1, 0, "Hello", 0.0, gs(5)),
                    GlyphRun::make_test_run(t2, 0, "World", 50.0, gs(5)),
                ],
                ruby_annotations: vec![],
                empty_caret_x: 0.0,
                child_range: None,
                tab_gaps: vec![],
                is_phantom: false,
                content_edge_x: None,
            }),
        };
        let tree = LayoutTree {
            root: make_box_node(0.0, 20.0, vec![make_box_node(0.0, 20.0, vec![line_node])]),
        };

        let sel = mov(
            &tree,
            Position::new(t2, 0),
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        )
        .unwrap();

        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, t1);
        assert_eq!(sel.head.offset, 4);
    }

    #[test]
    fn move_line_start_end() {
        let f = fixture();
        let sel_end = mov(
            &f.tree,
            Position::new(f.lines[0], 2),
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Horizontal,
            },
        )
        .unwrap();
        assert_eq!(sel_end.head.node_id, f.lines[0]);
        assert_eq!(sel_end.head.offset, 11);

        let sel_start = mov(
            &f.tree,
            Position::new(f.lines[0], 5),
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Horizontal,
            },
        )
        .unwrap();
        assert_eq!(sel_start.head.node_id, f.lines[0]);
        assert_eq!(sel_start.head.offset, 0);
    }

    #[test]
    fn preferred_x_maintained_across_short_line() {
        let ids: [NodeId; 3] = std::array::from_fn(|_| NodeId::new());
        let tree = LayoutTree {
            root: make_box_node(
                0.0,
                60.0,
                vec![make_box_node(
                    0.0,
                    60.0,
                    vec![
                        make_line_node(ids[0], 0.0, "hello world"),
                        make_line_node(ids[1], 20.0, "foo"),
                        make_line_node(ids[2], 40.0, "qux quux end"),
                    ],
                )],
            ),
        };

        let (sel1, px1) = resolve(
            &tree,
            &Position::new(ids[0], 8),
            &Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
            &VP,
            &Resource::new_test(),
            None,
        );
        let sel1 = sel1.unwrap();
        assert_eq!(sel1.head.node_id, ids[1]);
        assert_eq!(sel1.head.offset, 3);
        assert_eq!(px1, Some(80.0));

        let (sel2, px2) = resolve(
            &tree,
            &sel1.head,
            &Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
            &VP,
            &Resource::new_test(),
            px1,
        );
        let sel2 = sel2.unwrap();
        assert_eq!(sel2.head.node_id, ids[2]);
        assert_eq!(sel2.head.offset, 8);
        assert_eq!(px2, Some(80.0));
    }

    #[test]
    fn horizontal_movement_resets_preferred_x() {
        let f = fixture();
        let (_, px) = resolve(
            &f.tree,
            &Position::new(f.lines[0], 5),
            &Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
            &VP,
            &Resource::new_test(),
            None,
        );
        assert!(px.is_some());

        let (_, px2) = resolve(
            &f.tree,
            &Position::new(f.lines[1], 3),
            &Movement::Grapheme {
                direction: Direction::Forward,
            },
            &VP,
            &Resource::new_test(),
            px,
        );
        assert_eq!(px2, None);
    }

    #[test]
    fn vertical_movement_without_preferred_x_computes_fresh() {
        let f = fixture();
        let (sel, px) = resolve(
            &f.tree,
            &Position::new(f.lines[0], 3),
            &Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
            &VP,
            &Resource::new_test(),
            None,
        );
        assert_eq!(px, Some(30.0));
        assert_eq!(sel.as_ref().unwrap().head.node_id, f.lines[1]);
        assert_eq!(sel.as_ref().unwrap().head.offset, 3);
    }

    #[test]
    fn page_movement_preserves_preferred_x() {
        let f = fixture();
        let vp = Viewport {
            width: 200.0,
            height: 50.0,
            scale_factor: 1.0,
        };

        let (_, px) = resolve(
            &f.tree,
            &Position::new(f.lines[0], 5),
            &Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
            &vp,
            &Resource::new_test(),
            None,
        );
        assert_eq!(px, Some(50.0));

        let (_, px2) = resolve(
            &f.tree,
            &Position::new(f.lines[1], 5),
            &Movement::Page {
                direction: Direction::Forward,
            },
            &vp,
            &Resource::new_test(),
            px,
        );
        assert_eq!(px2, Some(50.0));
    }

    #[test]
    fn grapheme_forward_from_text_before_atom_selects_atom() {
        // Moving forward from text whose next navigable is an atom must produce a forward node selection.
        let f = fixture();
        // lines[1] "foo bar" ends at offset 7; next navigable in the layout is the atom.
        let sel = mov(
            &f.tree,
            Position::new(f.lines[1], 7),
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        )
        .unwrap();
        assert!(
            !sel.is_collapsed(),
            "must be atom node selection, got {:?}",
            sel
        );
        assert_eq!(
            sel.anchor,
            Position {
                node_id: f.atom_parent,
                offset: 0,
                affinity: Affinity::Downstream
            }
        );
        assert_eq!(
            sel.head,
            Position {
                node_id: f.atom_parent,
                offset: 1,
                affinity: Affinity::Upstream
            }
        );
    }

    #[test]
    fn grapheme_forward_from_selected_atom_passes_to_next_text() {
        // Moving forward from the head of an atom node-selection passes through to the start of the next Line.
        let f = fixture();
        // Forward head of the atom node-selection is (atom_parent, 1, Upstream).
        let pos = Position {
            node_id: f.atom_parent,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        let sel = mov(
            &f.tree,
            pos,
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        )
        .unwrap();
        assert!(
            sel.is_collapsed(),
            "passing an atom must yield a text caret, got {:?}",
            sel
        );
        assert_eq!(sel.head.node_id, f.lines[2]); // "baz"
        assert_eq!(sel.head.offset, 0);
    }

    #[test]
    fn grapheme_forward_from_attached_box_leading_boundary_uses_next_stream_entry() {
        let paragraph_id = NodeId::new();
        let text_id = NodeId::new();
        let tree = LayoutTree {
            root: make_box_node_with_id(
                NodeId::ROOT,
                0.0,
                20.0,
                vec![make_attached_box_node(
                    NodeId::ROOT,
                    0,
                    paragraph_id,
                    0.0,
                    20.0,
                    vec![make_paragraph_line(paragraph_id, text_id, 0.0, "ab")],
                )],
            ),
        };

        let sel = mov(
            &tree,
            Position {
                node_id: NodeId::ROOT,
                offset: 0,
                affinity: Affinity::Downstream,
            },
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        )
        .unwrap();

        assert_eq!(sel, Selection::collapsed(Position::new(text_id, 0)));
    }

    #[test]
    fn grapheme_backward_from_attached_box_trailing_boundary_uses_prev_stream_entry() {
        let paragraph_id = NodeId::new();
        let text_id = NodeId::new();
        let tree = LayoutTree {
            root: make_box_node_with_id(
                NodeId::ROOT,
                0.0,
                20.0,
                vec![make_attached_box_node(
                    NodeId::ROOT,
                    0,
                    paragraph_id,
                    0.0,
                    20.0,
                    vec![make_paragraph_line(paragraph_id, text_id, 0.0, "ab")],
                )],
            ),
        };

        let sel = mov(
            &tree,
            Position {
                node_id: NodeId::ROOT,
                offset: 1,
                affinity: Affinity::Upstream,
            },
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        )
        .unwrap();

        assert_eq!(
            sel,
            Selection::collapsed(Position {
                node_id: text_id,
                offset: 2,
                affinity: Affinity::Upstream,
            })
        );
    }

    #[test]
    fn grapheme_movement_skips_empty_attached_box_by_stream_order() {
        let before_id = NodeId::new();
        let empty_box_id = NodeId::new();
        let after_id = NodeId::new();
        let tree = LayoutTree {
            root: make_box_node_with_id(
                NodeId::ROOT,
                0.0,
                60.0,
                vec![
                    make_line_node(before_id, 0.0, "aa"),
                    make_attached_box_node(NodeId::ROOT, 1, empty_box_id, 20.0, 20.0, vec![]),
                    make_line_node(after_id, 40.0, "bb"),
                ],
            ),
        };

        let forward = mov(
            &tree,
            Position {
                node_id: NodeId::ROOT,
                offset: 1,
                affinity: Affinity::Downstream,
            },
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        )
        .unwrap();
        assert_eq!(forward, Selection::collapsed(Position::new(after_id, 0)));

        let backward = mov(
            &tree,
            Position {
                node_id: NodeId::ROOT,
                offset: 2,
                affinity: Affinity::Upstream,
            },
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        )
        .unwrap();
        assert_eq!(
            backward,
            Selection::collapsed(Position {
                node_id: before_id,
                offset: 2,
                affinity: Affinity::Upstream,
            })
        );
    }

    #[test]
    fn grapheme_backward_from_text_after_atom_selects_atom_backward() {
        let f = fixture();
        // Moving backward from the start of "baz" whose previous navigable is an atom must produce a backward node selection.
        let sel = mov(
            &f.tree,
            Position::new(f.lines[2], 0),
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        )
        .unwrap();
        assert!(
            !sel.is_collapsed(),
            "must be atom node selection, got {:?}",
            sel
        );
        assert_eq!(
            sel.anchor,
            Position {
                node_id: f.atom_parent,
                offset: 1,
                affinity: Affinity::Upstream
            }
        );
        assert_eq!(
            sel.head,
            Position {
                node_id: f.atom_parent,
                offset: 0,
                affinity: Affinity::Downstream
            }
        );
    }

    #[test]
    fn grapheme_backward_from_selected_atom_passes_to_prev_text() {
        let f = fixture();
        // Backward head of the atom node-selection is (atom_parent, 0, Downstream).
        let pos = Position {
            node_id: f.atom_parent,
            offset: 0,
            affinity: Affinity::Downstream,
        };
        let sel = mov(
            &f.tree,
            pos,
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        )
        .unwrap();
        assert!(
            sel.is_collapsed(),
            "passing an atom backward must yield a text caret, got {:?}",
            sel
        );
        assert_eq!(sel.head.node_id, f.lines[1]); // "foo bar"
        assert_eq!(sel.head.offset, 7);
    }

    #[test]
    fn document_backward_to_leading_atom_selects_atom() {
        // When the document's first navigable is an atom, document-backward must node-select it.
        let atom_parent = NodeId::new();
        let atom_id = NodeId::new();
        let line_id = NodeId::new();
        let tree = LayoutTree {
            root: make_box_node(
                0.0,
                40.0,
                vec![
                    make_atom_node(atom_parent, atom_id, 0.0, 0),
                    make_line_node(line_id, 20.0, "tail"),
                ],
            ),
        };
        let sel = mov(
            &tree,
            Position::new(line_id, 2),
            Movement::Document {
                direction: Direction::Backward,
            },
        )
        .unwrap();
        // document-backward lands on the first entry with forward=false, so the
        // backward form is: anchor=(parent, idx+1, Upstream), head=(parent, idx, Downstream).
        assert!(!sel.is_collapsed(), "got {:?}", sel);
        assert_eq!(
            sel.anchor,
            Position {
                node_id: atom_parent,
                offset: 1,
                affinity: Affinity::Upstream
            }
        );
        assert_eq!(
            sel.head,
            Position {
                node_id: atom_parent,
                offset: 0,
                affinity: Affinity::Downstream
            }
        );
    }

    #[test]
    fn line_vertical_forward_from_start_two_paragraphs() {
        let (state, t1, t2) = state! {
            doc {
                root [paragraph_indent(1), block_gap(1)] {
                    paragraph { t1: text("hello") }
                    paragraph { t2: text("world") }
                }
            }
            selection: (t1, 0)
        };

        let mut view = View::new_test();
        view.layout(&state.doc);

        let sel = view
            .resolve_movement(
                &Position::new(t1, 0),
                &Movement::Line {
                    direction: Direction::Forward,
                    axis: Axis::Vertical,
                },
                &Resource::new_test(),
            )
            .unwrap();

        assert_eq!(sel.head.node_id, t2);
        assert_eq!(sel.head.offset, 0);
    }

    #[test]
    fn line_end_offset_is_node_relative_in_multi_text_paragraph() {
        let (state, t1) = state! {
            doc {
                root {
                    paragraph {
                        text("Hello, ")
                        t1: text("World!") [bold]
                    }
                }
            }
            selection: (t1, 0)
        };

        let mut view = View::new_test();
        view.layout(&state.doc);

        let sel = view
            .resolve_movement(
                &Position::new(t1, 0),
                &Movement::Line {
                    direction: Direction::Forward,
                    axis: Axis::Horizontal,
                },
                &Resource::new_test(),
            )
            .unwrap();

        assert_eq!(sel.head.node_id, t1);
        assert_eq!(sel.head.offset, 6);
    }

    #[test]
    fn grapheme_forward_across_text_nodes_offset_is_node_relative() {
        let (state, t1, t2) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello, ")
                        t2: text("World!") [bold]
                    }
                }
            }
            selection: (t1, 7)
        };

        let mut view = View::new_test();
        view.layout(&state.doc);

        let sel = view
            .resolve_movement(
                &Position::new(t1, 7),
                &Movement::Grapheme {
                    direction: Direction::Forward,
                },
                &Resource::new_test(),
            )
            .unwrap();

        assert_eq!(sel.head.node_id, t2);
        assert_eq!(sel.head.offset, 1);
    }

    #[test]
    fn grapheme_backward_across_text_nodes_offset_is_node_relative() {
        let (state, t1, t2) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("Hello, ")
                        t2: text("World!") [bold]
                    }
                }
            }
            selection: (t2, 0)
        };

        let mut view = View::new_test();
        view.layout(&state.doc);

        let sel = view
            .resolve_movement(
                &Position::new(t2, 0),
                &Movement::Grapheme {
                    direction: Direction::Backward,
                },
                &Resource::new_test(),
            )
            .unwrap();

        assert_eq!(sel.head.node_id, t1);
        assert_eq!(sel.head.offset, 6);
    }

    #[test]
    fn hit_test_offset_is_node_relative_in_multi_text_paragraph() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph {
                        text("Hello, ")
                        t1: text("World!") [bold]
                    }
                }
            }
            selection: (t1, 6)
        };

        let mut view = View::new_test();
        view.layout(&state.doc);

        let sel = view.hit_test(0, 9999.0, 5.0).unwrap();
        assert_eq!(sel.head.offset, 6);
    }

    #[test]
    fn grapheme_forward_in_empty_table_cell_moves_to_next_cell_in_same_row() {
        let (state, p1, p2) = state! {
            doc {
                root {
                    table {
                        table_row {
                            table_cell { p1: paragraph {} }
                            table_cell { p2: paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                        }
                        table_row {
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                        }
                        table_row {
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };

        let mut view = View::new_test();
        view.layout(&state.doc);

        let sel = view
            .resolve_movement(
                &Position::new(p1, 0),
                &Movement::Grapheme {
                    direction: Direction::Forward,
                },
                &Resource::new_test(),
            )
            .unwrap();

        assert_eq!(sel.head, sel.anchor);
        assert_eq!(
            sel.head.node_id, p2,
            "must move to next cell in the same row"
        );
        assert_eq!(sel.head.offset, 0);
    }

    #[test]
    fn grapheme_backward_in_empty_table_cell_moves_to_prev_cell_in_same_row() {
        let (state, p1, p2) = state! {
            doc {
                root {
                    table {
                        table_row {
                            table_cell { p1: paragraph {} }
                            table_cell { p2: paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                        }
                        table_row {
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p2, 0)
        };

        let mut view = View::new_test();
        view.layout(&state.doc);

        let sel = view
            .resolve_movement(
                &Position::new(p2, 0),
                &Movement::Grapheme {
                    direction: Direction::Backward,
                },
                &Resource::new_test(),
            )
            .unwrap();

        assert_eq!(sel.head, sel.anchor);
        assert_eq!(
            sel.head.node_id, p1,
            "must move to previous cell in the same row"
        );
        assert_eq!(sel.head.offset, 0);
    }

    #[test]
    fn grapheme_forward_at_end_of_nonempty_table_cell_moves_to_next_cell() {
        let (state, t1, p2) = state! {
            doc {
                root {
                    table {
                        table_row {
                            table_cell { paragraph { t1: text("A") } }
                            table_cell { p2: paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                        }
                        table_row {
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 1)
        };

        let mut view = View::new_test();
        view.layout(&state.doc);

        let sel = view
            .resolve_movement(
                &Position::new(t1, 1),
                &Movement::Grapheme {
                    direction: Direction::Forward,
                },
                &Resource::new_test(),
            )
            .unwrap();

        assert_eq!(sel.head, sel.anchor);
        assert_eq!(
            sel.head.node_id, p2,
            "must move to next cell in the same row"
        );
        assert_eq!(sel.head.offset, 0);
    }

    #[test]
    fn word_forward_at_cell_end_moves_to_next_cell() {
        let (state, t1, p2) = state! {
            doc {
                root {
                    table {
                        table_row {
                            table_cell { paragraph { t1: text("hi") } }
                            table_cell { p2: paragraph {} }
                        }
                        table_row {
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t1, 2)
        };

        let mut view = View::new_test();
        view.layout(&state.doc);

        let sel = view
            .resolve_movement(
                &Position::new(t1, 2),
                &Movement::Word {
                    direction: Direction::Forward,
                },
                &Resource::new_test(),
            )
            .unwrap();

        assert_eq!(sel.head, sel.anchor);
        assert_eq!(sel.head.node_id, p2);
        assert_eq!(sel.head.offset, 0);
    }

    #[test]
    fn word_forward_outside_table_still_crosses_paragraphs() {
        let (state, t1, t2) = state! {
            doc {
                root {
                    paragraph { t1: text("hi") }
                    paragraph { t2: text("there") }
                }
            }
            selection: (t1, 2)
        };

        let mut view = View::new_test();
        view.layout(&state.doc);

        let sel = view
            .resolve_movement(
                &Position::new(t1, 2),
                &Movement::Word {
                    direction: Direction::Forward,
                },
                &Resource::new_test(),
            )
            .unwrap();

        assert_eq!(
            sel.head.node_id, t2,
            "word movement outside tables must still cross paragraph boundaries"
        );
    }

    #[test]
    fn line_vertical_forward_in_table_stays_in_same_column() {
        let (state, p1, below) = state! {
            doc {
                root {
                    table {
                        table_row {
                            table_cell { paragraph {} }
                            table_cell { p1: paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                        }
                        table_row {
                            table_cell { paragraph {} }
                            table_cell { below: paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                        }
                        table_row {
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };

        let mut view = View::new_test();
        view.layout(&state.doc);

        let sel = view
            .resolve_movement(
                &Position::new(p1, 0),
                &Movement::Line {
                    direction: Direction::Forward,
                    axis: Axis::Vertical,
                },
                &Resource::new_test(),
            )
            .unwrap();

        assert_eq!(
            sel.head.node_id, below,
            "ArrowDown must move to the cell directly below in the same column"
        );
    }

    #[test]
    fn line_vertical_backward_in_table_stays_in_same_column() {
        let (state, above, p1) = state! {
            doc {
                root {
                    table {
                        table_row {
                            table_cell { paragraph {} }
                            table_cell { above: paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                        }
                        table_row {
                            table_cell { paragraph {} }
                            table_cell { p1: paragraph {} }
                            table_cell { paragraph {} }
                            table_cell { paragraph {} }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };

        let mut view = View::new_test();
        view.layout(&state.doc);

        let sel = view
            .resolve_movement(
                &Position::new(p1, 0),
                &Movement::Line {
                    direction: Direction::Backward,
                    axis: Axis::Vertical,
                },
                &Resource::new_test(),
            )
            .unwrap();

        assert_eq!(
            sel.head.node_id, above,
            "ArrowUp must move to the cell directly above in the same column"
        );
    }

    #[test]
    fn line_vertical_forward_from_paragraph_above_fold_enters_fold_title() {
        let (state, p1, t1) = state! {
            doc {
                root {
                    p1: paragraph {}
                    fold {
                        fold_title { t1: text("13") }
                        fold_content { paragraph { text("123") } }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 0)
        };

        let mut view = View::new_test();
        view.layout(&state.doc);

        let sel = view
            .resolve_movement(
                &Position::new(p1, 0),
                &Movement::Line {
                    direction: Direction::Forward,
                    axis: Axis::Vertical,
                },
                &Resource::new_test(),
            )
            .unwrap();

        assert_eq!(
            sel.head.node_id, t1,
            "ArrowDown from the paragraph above a fold must enter the fold title, not skip the fold"
        );
        assert_eq!(sel.head.offset, 0);
    }

    #[test]
    fn line_vertical_backward_from_paragraph_below_fold_enters_fold_content() {
        let (state, c1, p1) = state! {
            doc {
                root {
                    paragraph {}
                    fold {
                        fold_title { text("13") }
                        fold_content { paragraph { c1: text("123") } }
                    }
                    p1: paragraph {}
                }
            }
            selection: (p1, 0)
        };

        let mut view = View::new_test();
        view.layout(&state.doc);

        let sel = view
            .resolve_movement(
                &Position::new(p1, 0),
                &Movement::Line {
                    direction: Direction::Backward,
                    axis: Axis::Vertical,
                },
                &Resource::new_test(),
            )
            .unwrap();

        assert_eq!(
            sel.head.node_id, c1,
            "ArrowUp from the paragraph below a fold must enter the fold content, not skip the fold"
        );
    }

    #[test]
    fn fold_node_selection_arrow_right_steps_out_to_next_paragraph() {
        let (st, r, after) = state! {
            doc { r: root {
                fold { fold_title { text("123123") } fold_content { paragraph { text("123") } } }
                paragraph { after: text("1231231232131") }
            } }
            selection: (after, 0)
        };
        let mut view = View::new_test();
        view.layout(&st.doc);
        let head = Position {
            node_id: r,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        let sel = view
            .resolve_movement(
                &head,
                &Movement::Grapheme {
                    direction: Direction::Forward,
                },
                &Resource::new_test(),
            )
            .expect("arrow-right must move from a fold node-selection");
        assert!(sel.is_collapsed());
        assert_eq!(sel.head.node_id, after);
        assert_eq!(sel.head.offset, 0);
    }

    #[test]
    fn fold_trailing_boundary_arrow_left_enters_fold_content() {
        let (st, r, c) = state! {
            doc { r: root {
                paragraph { text("prev") }
                fold { fold_title { text("t") } fold_content { paragraph { c: text("c") } } }
                paragraph { text("after") }
            } }
            selection: (c, 0)
        };
        let mut view = View::new_test();
        view.layout(&st.doc);
        let head = Position {
            node_id: r,
            offset: 2,
            affinity: Affinity::Upstream,
        };
        let sel = view
            .resolve_movement(
                &head,
                &Movement::Grapheme {
                    direction: Direction::Backward,
                },
                &Resource::new_test(),
            )
            .expect("arrow-left must move from the fold trailing boundary");
        assert!(sel.is_collapsed());
        assert_eq!(sel.head.node_id, c);
        assert_eq!(sel.head.offset, 1);
    }

    #[test]
    fn fold_node_selection_arrow_down_resolves() {
        let (st, r, ..) = state! {
            doc { r: root {
                fold { fold_title { text("123123") } fold_content { paragraph { text("123") } } }
                paragraph { t: text("1231231232131") }
            } }
            selection: (t, 0)
        };
        let mut view = View::new_test();
        view.layout(&st.doc);
        let head = Position {
            node_id: r,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        let sel = view.resolve_movement(
            &head,
            &Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Vertical,
            },
            &Resource::new_test(),
        );
        assert!(
            sel.is_some(),
            "arrow-down from a fold node-selection must resolve"
        );
    }

    #[test]
    fn word_forward_from_fold_node_selection_steps_out() {
        let (st, r, after) = state! {
            doc { r: root {
                fold { fold_title { text("t") } fold_content { paragraph { text("c") } } }
                paragraph { after: text("hello world") }
            } }
            selection: (after, 0)
        };
        let mut view = View::new_test();
        view.layout(&st.doc);
        let head = Position {
            node_id: r,
            offset: 1,
            affinity: Affinity::Upstream,
        };
        let sel = view
            .resolve_movement(
                &head,
                &Movement::Word {
                    direction: Direction::Forward,
                },
                &Resource::new_test(),
            )
            .expect("word-forward from a fold node-selection must step out");
        assert_eq!(sel.head.node_id, after);
    }

    #[test]
    fn caret_inside_fold_content_still_resolves_after_parity_change() {
        let (st, c1, ..) = state! {
            doc { root {
                fold { fold_title { text("t") } fold_content { paragraph { c1: text("abc") } } }
                paragraph { text("after") }
            } }
            selection: (c1, 1)
        };
        let mut view = View::new_test();
        view.layout(&st.doc);
        let sel = view
            .resolve_movement(
                &Position::new(c1, 1),
                &Movement::Grapheme {
                    direction: Direction::Forward,
                },
                &Resource::new_test(),
            )
            .expect("interior fold caret must still navigate");
        assert_eq!(sel.head.node_id, c1);
        assert_eq!(sel.head.offset, 2);
    }

    #[test]
    fn editable_position_inside_fold_returns_inner_first() {
        // state!/doc! returns ids in label declaration order (here: r, then t).
        let (state, _, t) = state! {
            doc {
                r: root {
                    fold { fold_title { t: text("title") } fold_content { paragraph { text("body") } } }
                    paragraph { text("after") }
                }
            }
            selection: (t, 0)
        };
        let mut view = View::new_test();
        view.layout(&state.doc);
        let fold_id = state
            .doc
            .node(t)
            .unwrap()
            .ancestors()
            .find(|n| matches!(n.node(), editor_model::Node::Fold(_)))
            .unwrap()
            .id();
        let pos = view
            .editable_position_inside(fold_id, false)
            .expect("fold has inner navigable");
        assert_eq!(pos.node_id, t);
        assert_eq!(pos.offset, 0);
    }

    #[test]
    fn editable_position_inside_atom_is_none() {
        let (state, r) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut view = View::new_test();
        view.layout(&state.doc);
        let image_id = state.doc.node(r).unwrap().children().next().unwrap().id();
        assert!(view.editable_position_inside(image_id, false).is_none());
    }

    // TR-199: 테이블 마지막 행의 어느 셀에 있는 커서든 테이블의 아래쪽
    // 가장자리 행에 있으므로 세로 갭 진입에서 가장자리로 봐야 한다 — 라인이
    // 테이블 서브트리의 유일한 마지막 navigable인 마지막 열만이 아니라.
    #[test]
    fn is_at_edge_line_of_table_last_row_any_column() {
        let (state, table, t_first, t_last) = state! {
            doc {
                root {
                    table: table {
                        table_row {
                            table_cell { paragraph { text("a") } }
                            table_cell { paragraph { text("b") } }
                        }
                        table_row {
                            table_cell { paragraph { t_first: text("c") } }
                            table_cell { paragraph { t_last: text("d") } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t_first, 0)
        };
        let mut view = View::new_test();
        view.layout(&state.doc);

        let first_col = Position::new(t_first, 1);
        let last_col = Position::new(t_last, 1);

        assert!(
            view.is_at_edge_line_of(table, &last_col, true),
            "last column of last row is on the bottom edge"
        );
        assert!(
            view.is_at_edge_line_of(table, &first_col, true),
            "first column of last row must also be on the bottom edge"
        );
    }

    // 반대 경우: 첫 행의 커서는 아래쪽 가장자리로 보면 안 되고(그러면 첫 행에서
    // 아래로 새어 나간다) 위쪽 가장자리로 봐야 한다(첫 행에서 위로 벗어남).
    #[test]
    fn is_at_edge_line_of_table_first_row_is_top_edge_not_bottom() {
        let (state, table, t_top, _t_bottom) = state! {
            doc {
                root {
                    table: table {
                        table_row {
                            table_cell { paragraph { t_top: text("a") } }
                            table_cell { paragraph { text("b") } }
                        }
                        table_row {
                            table_cell { paragraph { text("c") } }
                            table_cell { paragraph { t_bottom: text("d") } }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (t_top, 0)
        };
        let mut view = View::new_test();
        view.layout(&state.doc);

        let top = Position::new(t_top, 1);
        assert!(
            !view.is_at_edge_line_of(table, &top, true),
            "first row is not the table's bottom edge"
        );
        assert!(
            view.is_at_edge_line_of(table, &top, false),
            "first row IS the table's top edge"
        );
    }

    #[test]
    fn grapheme_forward_across_tab_stops_after_tab() {
        use crate::view::View;
        use editor_macros::doc;
        let (doc, _p1, t1, t2) =
            doc! { root { p1: paragraph { t1: text("1") tab {} t2: text("2") } } };
        let mut view = View::new_test();
        view.layout(&doc);
        let tree = view.layout_tree_for_test().unwrap();
        let sel = mov(
            tree,
            Position::new(t1, 1),
            Movement::Grapheme {
                direction: Direction::Forward,
            },
        )
        .expect("move");
        assert_eq!(sel.head.node_id, t2);
        assert_eq!(
            sel.head.offset, 0,
            "right-arrow across a tab must land before '2' (after the tab), not skip it"
        );
    }

    #[test]
    fn grapheme_backward_across_tab_stops_before_tab() {
        use crate::view::View;
        use editor_macros::doc;
        let (doc, _p1, t1, t2) =
            doc! { root { p1: paragraph { t1: text("1") tab {} t2: text("2") } } };
        let mut view = View::new_test();
        view.layout(&doc);
        let tree = view.layout_tree_for_test().unwrap();
        let sel = mov(
            tree,
            Position::new(t2, 0),
            Movement::Grapheme {
                direction: Direction::Backward,
            },
        )
        .expect("move");
        assert_eq!(sel.head.node_id, t1);
        assert_eq!(
            sel.head.offset, 1,
            "left-arrow across a tab must land after '1' (before the tab), not skip it"
        );
    }

    #[test]
    fn word_forward_stops_at_tab_boundary() {
        use crate::view::View;
        use editor_macros::doc;
        let (doc, _p1, t1, _t2) =
            doc! { root { p1: paragraph { t1: text("foo") tab {} t2: text("bar") } } };
        let mut view = View::new_test();
        view.layout(&doc);
        let tree = view.layout_tree_for_test().unwrap();
        let sel = mov(
            tree,
            Position::new(t1, 0),
            Movement::Word {
                direction: Direction::Forward,
            },
        )
        .expect("move");
        assert_eq!(
            (sel.head.node_id, sel.head.offset),
            (t1, 3),
            "word-forward must stop at the tab (end of 'foo'), not skip across to end of 'bar'"
        );
    }

    #[test]
    fn word_backward_stops_at_tab_boundary() {
        use crate::view::View;
        use editor_macros::doc;
        let (doc, _p1, _t1, t2) =
            doc! { root { p1: paragraph { t1: text("foo") tab {} t2: text("bar") } } };
        let mut view = View::new_test();
        view.layout(&doc);
        let tree = view.layout_tree_for_test().unwrap();
        let sel = mov(
            tree,
            Position::new(t2, 3),
            Movement::Word {
                direction: Direction::Backward,
            },
        )
        .expect("move");
        assert_eq!(
            (sel.head.node_id, sel.head.offset),
            (t2, 0),
            "word-backward from end of 'bar' must stop at start of 'bar', not skip across the tab"
        );
    }

    #[test]
    fn home_with_leading_tab_goes_before_tab() {
        use crate::view::View;
        use editor_macros::doc;
        let (doc, p1, t) = doc! { root { p1: paragraph { tab {} t: text("x") } } };
        let mut view = View::new_test();
        view.layout(&doc);
        let tree = view.layout_tree_for_test().unwrap();
        let sel = mov(
            tree,
            Position::new(t, 0),
            Movement::Line {
                direction: Direction::Backward,
                axis: Axis::Horizontal,
            },
        )
        .expect("move");
        assert_eq!(
            (sel.head.node_id, sel.head.offset),
            (p1, 0),
            "Home must go to the line start (before the leading tab)"
        );
    }

    #[test]
    fn end_with_trailing_tab_goes_after_tab() {
        use crate::view::View;
        use editor_macros::doc;
        let (doc, p1, t) = doc! { root { p1: paragraph { t: text("x") tab {} } } };
        let mut view = View::new_test();
        view.layout(&doc);
        let tree = view.layout_tree_for_test().unwrap();
        let sel = mov(
            tree,
            Position::new(t, 0),
            Movement::Line {
                direction: Direction::Forward,
                axis: Axis::Horizontal,
            },
        )
        .expect("move");
        assert_eq!(
            (sel.head.node_id, sel.head.offset),
            (p1, 2),
            "End must go to the line end (after the trailing tab)"
        );
    }
}
