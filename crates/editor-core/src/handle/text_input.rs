use std::collections::BTreeMap;
use std::ops::Range;
use std::sync::Arc;

use editor_commands::{self as commands, CommandError, CommandResult};
use editor_common::StrExt;
use editor_model::{DocView, Modifier, ModifierType};
use editor_state::{
    Composition, FLAT_CLOSE, FLAT_OPEN, FlatSegment, Position, ProjectedState, ResolvedPosition,
    ResolvedPositionFlatExt, Selection, apply_pending, as_gap_cursor, continuation_at, flat_chars,
    flat_segments_in_range, flat_size, is_unit_node_selection, replacement_paint,
};
use editor_transaction::Transaction;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

fn selection_from_flat_range(
    doc: &DocView,
    start: usize,
    end: usize,
) -> Result<Selection, CommandError> {
    let start_pos = ResolvedPosition::from_flat(doc, start)
        .ok_or(CommandError::Corrupted("flat start unresolvable".into()))?;
    let end_pos = ResolvedPosition::from_flat(doc, end)
        .ok_or(CommandError::Corrupted("flat end unresolvable".into()))?;
    Ok(Selection::new((&start_pos).into(), (&end_pos).into()))
}

fn seed_composition_paint(
    projected: &ProjectedState,
    start: usize,
    end: usize,
) -> BTreeMap<ModifierType, Modifier> {
    let view = projected.view();
    let (Some(from), Some(to)) = (
        ResolvedPosition::from_flat(&view, start),
        ResolvedPosition::from_flat(&view, end),
    ) else {
        return BTreeMap::new();
    };
    let from_pos: Position = (&from).into();
    let to_pos: Position = (&to).into();
    if let Some(paint) = replacement_paint(projected, from_pos, to_pos) {
        return paint.into_iter().map(|m| (m.as_type(), m)).collect();
    }
    continuation_at(projected, from_pos.node, from_pos.offset)
}

fn composition_range_valid(view: &DocView, start: usize, end: usize) -> bool {
    // AOSP parity: a zero-length composing region cannot exist in stock
    // editors (zero-length composing spans are discarded), so empty ranges
    // never persist as composition state.
    if start >= end || end > flat_size(view) {
        return false;
    }
    flat_segments_in_range(view, start..end)
        .iter()
        .all(|seg| matches!(seg, FlatSegment::Text { .. }))
}

fn is_token(c: char) -> bool {
    c == FLAT_OPEN || c == FLAT_CLOSE
}

fn balanced_structural_body_range(
    view: &DocView,
    start: usize,
    end: usize,
) -> Option<(usize, usize)> {
    let mut stack = Vec::new();
    let mut body: Option<(usize, usize)> = None;
    for (seg_start, seg) in editor_state::flat_segments_in_range_with_pos(view, start..end) {
        match seg {
            FlatSegment::Open { block } => stack.push((block, seg_start)),
            FlatSegment::Close { block } => {
                let Some(&(open_id, open_start)) = stack.last() else {
                    continue;
                };
                if open_id != block {
                    continue;
                }
                stack.pop();
                let pair = (open_start, seg_start + 1);
                body = Some(match body {
                    Some((body_start, body_end)) => (body_start.min(pair.0), body_end.max(pair.1)),
                    None => pair,
                });
            }
            _ => {}
        }
    }

    body
}

fn utf16_units_to_chars(iter: impl Iterator<Item = char>, utf16_units: usize) -> usize {
    let mut remaining = utf16_units;
    let mut count = 0;
    for c in iter {
        if remaining == 0 {
            break;
        }
        remaining = remaining.saturating_sub(c.len_utf16());
        count += 1;
    }
    count
}

/// A window of the document's flat char buffer around the edit site, addressed
/// in absolute flat coordinates. `from_editor` materializes only `[base, base +
/// chars.len())` (a handful of chars for a keystroke) instead of the whole
/// document, which is the per-keystroke cost that grows with document size.
/// `len()` still reports the full flat size so offset clamping is unchanged.
#[derive(Debug, Clone, PartialEq)]
struct FlatText {
    base: usize,
    chars: Vec<char>,
    total: usize,
}

impl FlatText {
    fn whole(chars: Vec<char>) -> Self {
        let total = chars.len();
        Self {
            base: 0,
            chars,
            total,
        }
    }

    fn len(&self) -> usize {
        self.total
    }

    fn at(&self, abs: usize) -> char {
        self.chars[abs - self.base]
    }

    fn slice(&self, range: Range<usize>) -> &[char] {
        &self.chars[range.start - self.base..range.end - self.base]
    }

    fn iter_from(&self, abs: usize) -> impl Iterator<Item = char> + '_ {
        self.chars[abs - self.base..].iter().copied()
    }

    fn iter_rev_to(&self, abs: usize) -> impl Iterator<Item = char> + '_ {
        self.chars[..abs - self.base].iter().rev().copied()
    }

    fn splice(&mut self, range: Range<usize>, replacement: impl IntoIterator<Item = char>) {
        let start = range.start - self.base;
        let end = range.end - self.base;
        let before = self.chars.len();
        self.chars.splice(start..end, replacement);
        self.total = self.total - before + self.chars.len();
    }
}

/// Flat-offset window covering everything `ops` can read or edit, starting from
/// the current selection/composition and widened by each op's reach (insert
/// length, surrounding-delete counts, cursor moves). Over-covers (sums reaches)
/// so no access falls outside; for a keystroke the window is a few chars.
fn ime_window(
    total: usize,
    sel_start: usize,
    sel_end: usize,
    comp: Option<(usize, usize)>,
    ops: &[FlatImeOp],
) -> Range<usize> {
    let mut lo = sel_start;
    let mut hi = sel_end;
    if let Some((cs, ce)) = comp {
        lo = lo.min(cs);
        hi = hi.max(ce);
    }
    let mut reach: usize = 0;
    for op in ops {
        match op {
            FlatImeOp::SetSelection { start, end } | FlatImeOp::SetComposition { start, end } => {
                lo = lo.min(*start);
                hi = hi.max(*end);
            }
            FlatImeOp::ReplaceSelection { text } | FlatImeOp::Compose { text } => {
                reach += text.chars().count();
            }
            FlatImeOp::DeleteSurrounding { before, after }
            | FlatImeOp::DeleteSurroundingUtf16 { before, after } => {
                reach += *before + *after;
            }
            FlatImeOp::MoveCursor { delta } => {
                reach += delta.unsigned_abs() as usize;
            }
            _ => {}
        }
    }
    const MARGIN: usize = 16;
    let start = lo.saturating_sub(reach + MARGIN);
    let end = hi.saturating_add(reach + MARGIN).min(total);
    start..end
}

#[derive(Debug, Clone)]
struct FlatImeState {
    text: FlatText,
    sel_start: usize,
    sel_end: usize,
    comp: Option<(usize, usize)>,
}

struct FlatImeReduction {
    state: FlatImeState,
    text_change: Option<FlatImeTextChange>,
}

#[derive(Debug, Clone, PartialEq)]
struct FlatImeTextChange {
    replace_start: usize,
    replace_end: usize,
    insert: Vec<char>,
}

fn remap_op(op: &FlatImeOp, before: &FlatImeState, after: &FlatImeState) -> FlatImeOp {
    match op {
        FlatImeOp::SetSelection { start, end } => remap_range(before, after, *start, *end)
            .map_or_else(
                || op.clone(),
                |(start, end)| FlatImeOp::SetSelection { start, end },
            ),
        FlatImeOp::SetComposition { start, end } => remap_range(before, after, *start, *end)
            .map_or_else(
                || op.clone(),
                |(start, end)| FlatImeOp::SetComposition { start, end },
            ),
        _ => op.clone(),
    }
}

fn remap_range(
    before: &FlatImeState,
    after: &FlatImeState,
    start: usize,
    end: usize,
) -> Option<(usize, usize)> {
    let before_cursor = (before.sel_start, before.sel_end);
    let after_cursor = (after.sel_start, after.sel_end);
    ((start, end) == before_cursor).then_some(after_cursor)
}

impl FlatImeTextChange {
    fn collapsed_at(pos: usize) -> Self {
        Self {
            replace_start: pos,
            replace_end: pos,
            insert: Vec::new(),
        }
    }

    fn without_reinserted_boundary_tokens(mut self, initial: &FlatText) -> Self {
        while self.replace_start < self.replace_end {
            let Some(&inserted) = self.insert.first() else {
                break;
            };
            let deleted = initial.at(self.replace_start);
            if !is_token(deleted) || deleted != inserted {
                break;
            }

            self.replace_start += 1;
            self.insert.remove(0);
        }

        while self.replace_start < self.replace_end {
            let Some(&inserted) = self.insert.last() else {
                break;
            };
            let deleted = initial.at(self.replace_end - 1);
            if !is_token(deleted) || deleted != inserted {
                break;
            }

            self.replace_end -= 1;
            self.insert.pop();
        }

        self
    }

    /// iOS Korean keyboards rewrite the composing syllable together with the
    /// committed character(s) before it (select + recommit). Drop the unchanged
    /// prefix so the replacement covers only what actually changed and the
    /// recommitted prefix keeps its own per-character paint. Prefix-only: a
    /// common-suffix trim would move the caret off the end of the insert.
    fn without_common_text_prefix(mut self, initial: &FlatText) -> Self {
        while self.replace_start < self.replace_end {
            let Some(&inserted) = self.insert.first() else {
                break;
            };
            let deleted = initial.at(self.replace_start);
            if is_token(deleted) || deleted != inserted {
                break;
            }

            self.replace_start += 1;
            self.insert.remove(0);
        }

        self
    }

    fn inserts_token(&self) -> bool {
        self.insert.iter().any(|c| is_token(*c))
    }

    fn deleted_from<'a>(&self, initial: &'a FlatText) -> &'a [char] {
        initial.slice(self.replace_start..self.replace_end)
    }
}

#[derive(Debug, Clone)]
struct FlatImeAnchoredChangeTracker {
    // Korean IMEs often update a composing syllable by deleting old composing
    // text and inserting the new one. Keep adjacent edits anchored to the
    // original range so repeated text does not make the edit origin ambiguous.
    replace_start: usize,
    replace_end: usize,
    current_start: usize,
    current_end: usize,
    insert: Vec<char>,
}

impl FlatImeAnchoredChangeTracker {
    fn new(change: FlatImeTextChange, text_after: &FlatText) -> Self {
        let current_start = change.replace_start;
        let current_end = change.replace_start + change.insert.len();
        let insert = text_after.slice(current_start..current_end).to_vec();
        Self {
            replace_start: change.replace_start,
            replace_end: change.replace_end,
            current_start,
            current_end,
            insert,
        }
    }

    fn absorb(&mut self, change: FlatImeTextChange, text_after: &FlatText) -> bool {
        if change.replace_end < self.current_start || change.replace_start > self.current_end {
            return false;
        }

        let replace_start = self.original_boundary(change.replace_start, false);
        let replace_end = self.original_boundary(change.replace_end, true);
        let current_start = self.current_start.min(change.replace_start);
        let old_current_end = self.current_end.max(change.replace_end);
        let removed_len = change.replace_end - change.replace_start;
        let inserted_len = change.insert.len();
        let Some(current_end) = old_current_end
            .checked_add(inserted_len)
            .and_then(|len| len.checked_sub(removed_len))
        else {
            return false;
        };
        if current_end > text_after.len() {
            return false;
        }

        self.replace_start = self.replace_start.min(replace_start);
        self.replace_end = self.replace_end.max(replace_end);
        self.current_start = current_start;
        self.current_end = current_end;
        self.insert = text_after
            .slice(self.current_start..self.current_end)
            .to_vec();
        true
    }

    fn original_boundary(&self, current_pos: usize, end_bias: bool) -> usize {
        if current_pos < self.current_start {
            current_pos
        } else if current_pos > self.current_end {
            self.replace_end + (current_pos - self.current_end)
        } else if end_bias {
            self.replace_end
        } else {
            self.replace_start
        }
    }

    fn into_text_change(self) -> FlatImeTextChange {
        FlatImeTextChange {
            replace_start: self.replace_start,
            replace_end: self.replace_end,
            insert: self.insert,
        }
    }
}

impl FlatImeState {
    fn from_editor(editor: &Editor, ops: &[FlatImeOp]) -> Option<Self> {
        let state = editor.state();
        let doc = state.view();
        let total = flat_size(&doc);

        let selection = state.selection?;
        let anchor = selection.anchor.resolve(&doc)?.to_flat();
        let head = selection.head.resolve(&doc)?.to_flat();
        let sel_start = anchor.min(head);
        let sel_end = anchor.max(head);

        let comp = state
            .composition
            .filter(|c| composition_range_valid(&doc, c.start, c.end))
            .map(|c| (c.start, c.end));

        let window = ime_window(total, sel_start, sel_end, comp, ops);
        let text = FlatText {
            base: window.start,
            chars: flat_chars(&doc, window),
            total,
        };

        Some(FlatImeState {
            text,
            sel_start,
            sel_end,
            comp,
        })
    }

    fn from_editor_whole(editor: &Editor) -> Option<Self> {
        let state = editor.state();
        let doc = state.view();
        let flat_size = flat_size(&doc);
        let selection = state.selection?;
        let anchor = selection.anchor.resolve(&doc)?.to_flat();
        let head = selection.head.resolve(&doc)?.to_flat();
        let comp = state
            .composition
            .filter(|c| composition_range_valid(&doc, c.start, c.end))
            .map(|c| (c.start, c.end));
        Some(FlatImeState {
            text: FlatText::whole(flat_chars(&doc, 0..flat_size)),
            sel_start: anchor.min(head),
            sel_end: anchor.max(head),
            comp,
        })
    }

    fn apply(&mut self, op: &FlatImeOp) -> Option<FlatImeTextChange> {
        match op {
            FlatImeOp::SetSelection { start, end } => {
                self.sel_start = (*start).min(self.text.len());
                self.sel_end = (*end).min(self.text.len());
                None
            }
            FlatImeOp::ReplaceSelection { text } => {
                let chars: Vec<char> = text.chars().collect();
                let start = self.sel_start.min(self.text.len());
                let end = self.sel_end.min(self.text.len());
                let new_pos = start + chars.len();
                self.text.splice(start..end, chars.iter().copied());
                self.sel_start = new_pos;
                self.sel_end = new_pos;
                self.comp = None;
                Some(FlatImeTextChange {
                    replace_start: start,
                    replace_end: end,
                    insert: chars,
                })
            }
            FlatImeOp::Compose { text } => {
                let chars: Vec<char> = text.chars().collect();
                let (start, end) = self.comp.unwrap_or((self.sel_start, self.sel_end));
                let start = start.min(self.text.len());
                let end = end.min(self.text.len());
                let new_end = start + chars.len();
                self.text.splice(start..end, chars.iter().copied());
                self.sel_start = new_end;
                self.sel_end = new_end;
                self.comp = Some((start, new_end));
                Some(FlatImeTextChange {
                    replace_start: start,
                    replace_end: end,
                    insert: chars,
                })
            }
            FlatImeOp::DeleteSurrounding { before, after } => {
                let (base_before, base_after) = self.surrounding_delete_bases();
                let del_start = base_before.saturating_sub(*before);
                let del_end = base_after.saturating_add(*after).min(self.text.len());
                self.apply_surrounding_deletes(del_start, base_before, base_after, del_end)
            }
            FlatImeOp::DeleteSurroundingUtf16 { before, after } => {
                let (base_before, base_after) = self.surrounding_delete_bases();
                let del_start =
                    base_before - utf16_units_to_chars(self.text.iter_rev_to(base_before), *before);
                let del_end =
                    base_after + utf16_units_to_chars(self.text.iter_from(base_after), *after);
                self.apply_surrounding_deletes(del_start, base_before, base_after, del_end)
            }
            FlatImeOp::SetComposition { start, end } => {
                let (start, end) = (*start.min(end), *start.max(end));
                self.comp = Some((start, end));
                None
            }
            FlatImeOp::ClearComposition | FlatImeOp::CommitAsIs => {
                self.comp = None;
                None
            }
            FlatImeOp::MoveCursor { delta } => {
                let pos = if *delta >= 0 {
                    self.sel_end.saturating_add(*delta as usize)
                } else {
                    self.sel_start.saturating_sub(delta.unsigned_abs() as usize)
                }
                .min(self.text.len());
                self.sel_start = pos;
                self.sel_end = pos;
                None
            }
        }
    }

    // deleteSurrounding must not touch the composing text: lengths count from
    // its edges (AOSP BaseInputConnection contract).
    fn surrounding_delete_bases(&self) -> (usize, usize) {
        let len = self.text.len();
        let cursor = self.sel_start.min(len);
        match self.comp {
            Some((comp_start, comp_end)) => {
                let base_before = cursor.min(comp_start);
                let base_after = self.sel_end.max(comp_end).min(len).max(base_before);
                (base_before, base_after)
            }
            None => (cursor, cursor),
        }
    }

    fn apply_surrounding_deletes(
        &mut self,
        del_start: usize,
        base_before: usize,
        base_after: usize,
        del_end: usize,
    ) -> Option<FlatImeTextChange> {
        let deletes_before = del_start < base_before;
        let deletes_after = base_after < del_end;
        // Deleting on both sides of a non-empty composing region is two
        // disjoint edits — unrepresentable as one flat replacement (same
        // policy as disjoint batches), so the op is ignored.
        if base_before < base_after && deletes_before && deletes_after {
            return None;
        }

        let remap_selection = self.comp.is_some();
        let (start, end) = if base_before < base_after {
            if deletes_before {
                (del_start, base_before)
            } else {
                (base_after, del_end)
            }
        } else {
            (del_start, del_end)
        };

        if start < end {
            self.text.splice(start..end, std::iter::empty());
            self.remap_after_delete(start, end, remap_selection);
        }
        if !remap_selection {
            self.sel_start = del_start;
            self.sel_end = del_start;
        }
        (start < end).then_some(FlatImeTextChange {
            replace_start: start,
            replace_end: end,
            insert: Vec::new(),
        })
    }

    fn remap_after_delete(&mut self, del_start: usize, del_end: usize, remap_selection: bool) {
        let removed = del_end - del_start;
        let map = |pos: usize| {
            if pos >= del_end {
                pos - removed
            } else {
                pos.min(del_start)
            }
        };
        if let Some((start, end)) = self.comp {
            self.comp = Some((map(start), map(end)));
        }
        if remap_selection {
            self.sel_start = map(self.sel_start);
            self.sel_end = map(self.sel_end);
        }
    }

    #[cfg(test)]
    fn reduce(mut self, ops: &[FlatImeOp]) -> Self {
        for op in ops {
            let _ = self.apply(op);
        }
        self
    }

    fn reduce_flat_ime_ops(mut self, ops: &[FlatImeOp]) -> FlatImeReduction {
        let initial_text = self.text.clone();
        let initial_sel_start = self.sel_start;
        let mut anchored_change: Option<FlatImeAnchoredChangeTracker> = None;
        let mut can_track_anchored_change = true;

        for op in ops {
            if let Some(change) = self.apply(op) {
                if !can_track_anchored_change {
                    continue;
                }

                match &mut anchored_change {
                    Some(tracker) => {
                        can_track_anchored_change = tracker.absorb(change, &self.text);
                    }
                    None => {
                        anchored_change =
                            Some(FlatImeAnchoredChangeTracker::new(change, &self.text));
                    }
                }
            }
        }

        let text_change = if can_track_anchored_change {
            anchored_change.map(FlatImeAnchoredChangeTracker::into_text_change)
        } else {
            None
        };
        let text_change = match text_change {
            Some(change) => {
                let change = change.without_reinserted_boundary_tokens(&initial_text);
                Some(if self.comp.is_none() {
                    change.without_common_text_prefix(&initial_text)
                } else {
                    change
                })
            }
            None if initial_text == self.text => {
                Some(FlatImeTextChange::collapsed_at(initial_sel_start))
            }
            // A batch with disjoint text edits cannot be represented as one safe
            // flat replacement. Ignore it instead of widening the edited range.
            None => None,
        };

        FlatImeReduction {
            state: self,
            text_change,
        }
    }
}

fn count_opens(chars: &[char]) -> usize {
    chars.iter().filter(|&&c| c == FLAT_OPEN).count()
}

fn count_closes(chars: &[char]) -> usize {
    chars.iter().filter(|&&c| c == FLAT_CLOSE).count()
}

fn structural_backward(tr: &mut Transaction) -> CommandResult {
    commands::first!(
        tr,
        commands::join_paragraph_backward(),
        commands::sink_paragraph_backward(),
        commands::lift_first_paragraph(),
    )
}

fn structural_forward(tr: &mut Transaction) -> CommandResult {
    commands::first!(
        tr,
        commands::join_paragraph_forward(),
        commands::lift_last_paragraph(),
        commands::lift_paragraph_forward(),
    )
}

fn set_selection_at_flat(tr: &mut Transaction, flat: usize) -> CommandResult {
    let doc = tr.view();
    let Some(pos) = ResolvedPosition::from_flat(&doc, flat) else {
        return Ok(false);
    };
    commands::set_selection(tr, Selection::collapsed((&pos).into()))
}

struct FlatDelta {
    start_tokens: usize,
    replace_start: usize,
    replace_end: usize,
    end_tokens: usize,
    ins_text: String,
    // Result-buffer text start for remapping composition after token-only replacement.
    composition_text_start: Option<usize>,
}

fn analyze_delta(
    doc: &DocView,
    chars: &FlatText,
    del_start: usize,
    del_end: usize,
    ins: &[char],
    cursor: usize,
) -> FlatDelta {
    let del = chars.slice(del_start..del_end);
    let ins_text: String = ins.iter().collect();
    let text_body = del
        .iter()
        .position(|c| !is_token(*c))
        .zip(del.iter().rposition(|c| !is_token(*c)))
        .map(|(first, last)| (del_start + first, del_start + last + 1));
    let structural_body = balanced_structural_body_range(doc, del_start, del_end);
    let replace_body = match (text_body, structural_body) {
        (Some((text_start, text_end)), Some((structural_start, structural_end))) => Some((
            text_start.min(structural_start),
            text_end.max(structural_end),
        )),
        (Some(body), None) | (None, Some(body)) => Some(body),
        (None, None) => None,
    };

    let (replace_start, replace_end) = if let Some((start, end)) = replace_body {
        (start, end)
    } else {
        let cursor = cursor.clamp(del_start, del_end);
        (cursor, cursor)
    };

    let left_tokens = chars.slice(del_start..replace_start);
    let right_tokens = chars.slice(replace_end..del_end);
    let backward_count = count_opens(left_tokens).max(count_closes(left_tokens));
    let forward_count = count_opens(right_tokens).max(count_closes(right_tokens));

    FlatDelta {
        start_tokens: backward_count,
        replace_start,
        replace_end,
        end_tokens: forward_count,
        ins_text,
        composition_text_start: chars
            .slice(replace_start..replace_end)
            .iter()
            .any(|c| is_token(*c))
            .then_some(replace_start),
    }
}

pub fn handle_flat_ime_ops(editor: &mut Editor, ops: Vec<FlatImeOp>) -> Result<(), EditorError> {
    let mut delete_paint = if editor.is_probing() {
        editor.ime_delete_paint.clone()
    } else {
        editor.ime_delete_paint.take()
    };
    let commit_as_is = ops.iter().any(|op| matches!(op, FlatImeOp::CommitAsIs));

    let committed_insert = (|| -> Result<bool, EditorError> {
        let initial = match FlatImeState::from_editor(editor, &ops) {
            Some(s) => s,
            None => return Ok(false),
        };

        let reduced = initial.clone().reduce_flat_ime_ops(&ops);

        if reduced.text_change.is_none() {
            return Ok(false);
        }

        let gap_view = editor.state.view();
        let started_at_gap = editor
            .state
            .selection
            .as_ref()
            .and_then(|s| s.resolve(&gap_view))
            .and_then(|rs| as_gap_cursor(&rs))
            .is_some();
        let (initial, reduced) = if initial.text != reduced.state.text && started_at_gap {
            delete_paint = None;
            let before_materialize = initial.clone();
            editor.transact(|tr| {
                tr.keep_pending_modifiers();
                tr.set_composition(None)?;
                commands::materialize_gap_paragraph(tr)?;
                Ok(())
            })?;
            let initial = match FlatImeState::from_editor_whole(editor) {
                Some(s) => s,
                None => return Ok(false),
            };
            let replay_ops: Vec<_> = ops
                .iter()
                .map(|op| remap_op(op, &before_materialize, &initial))
                .collect();
            let reduced = initial.clone().reduce_flat_ime_ops(&replay_ops);
            (initial, reduced)
        } else {
            (initial, reduced)
        };

        let result = reduced.state;
        let Some(text_change) = reduced.text_change else {
            return Ok(false);
        };
        let del = text_change.deleted_from(&initial.text);

        let del_opens = count_opens(del);
        let del_closes = count_closes(del);
        let ins_opens = count_opens(&text_change.insert);
        let ins_closes = count_closes(&text_change.insert);

        let tokens_increased = ins_opens > del_opens || ins_closes > del_closes;
        if tokens_increased {
            return Ok(false);
        }

        if text_change.inserts_token() {
            return Ok(false);
        }

        let delta = analyze_delta(
            &editor.state().view(),
            &initial.text,
            text_change.replace_start,
            text_change.replace_end,
            &text_change.insert,
            initial.sel_start,
        );
        let should_insert_after_unit_selection =
            !delta.ins_text.is_empty()
                && text_change.replace_start == initial.sel_start
                && text_change.replace_end == initial.sel_end
                && editor.state().selection.as_ref().is_some_and(|selection| {
                    is_unit_node_selection(selection, &editor.state().view())
                });
        let has_text_delta = !del.is_empty() || !text_change.insert.is_empty();
        let has_structural_seams = delta.start_tokens > 0 || delta.end_tokens > 0;

        let sidecar_before = editor.composition_paint.clone();

        let next_delete_paint = if !del.is_empty()
            && text_change.insert.is_empty()
            && !del.iter().any(|c| is_token(*c))
            && result.comp.is_none()
            && editor.state().composition.is_none()
            && sidecar_before.is_none()
        {
            selection_from_flat_range(
                &editor.state().view(),
                text_change.replace_start,
                text_change.replace_end,
            )
            .ok()
            .and_then(|sel| {
                editor_state::replacement_paint(&editor.state().projected, sel.anchor, sel.head)
            })
            .map(|paint| (text_change.replace_start, paint))
        } else {
            None
        };

        if has_text_delta || result.comp.is_some() || editor.state().composition.is_some() {
            editor.transact(|tr| {
                let mut composition_text_start = delta.composition_text_start;

                if has_text_delta {
                    if should_insert_after_unit_selection {
                        commands::chain!(
                            tr,
                            commands::insert_paragraph_after_unit_selection(),
                            commands::insert_text(&delta.ins_text),
                        )?;
                        composition_text_start = Some(text_change.replace_start);
                    } else if has_structural_seams {
                        let full_replace = text_change.replace_start == initial.sel_start
                            && text_change.replace_end == initial.sel_end;
                        if full_replace {
                            let sel = selection_from_flat_range(
                                &tr.view(),
                                text_change.replace_start,
                                text_change.replace_end,
                            )?;
                            commands::replace_range_with_text(
                                tr,
                                sel,
                                &delta.ins_text,
                                sidecar_before.clone(),
                            )?;
                        } else {
                            let deletes_body = delta.replace_start != delta.replace_end;
                            let paint = if !delta.ins_text.is_empty() {
                                let sel = selection_from_flat_range(
                                    &tr.view(),
                                    delta.replace_start,
                                    delta.replace_end,
                                )
                                .ok();
                                sel.and_then(|sel| {
                                    editor_state::replacement_paint(
                                        &tr.state().projected,
                                        sel.anchor,
                                        sel.head,
                                    )
                                })
                            } else {
                                None
                            };

                            if deletes_body {
                                let sel = selection_from_flat_range(
                                    &tr.view(),
                                    delta.replace_start,
                                    delta.replace_end,
                                )?;
                                commands::replace_range_with_text(tr, sel, "", None)?;
                            }

                            if delta.end_tokens > 0 {
                                if !deletes_body {
                                    set_selection_at_flat(tr, delta.replace_end)?;
                                }
                                for _ in 0..delta.end_tokens {
                                    structural_forward(tr)?;
                                }
                            }

                            if delta.start_tokens > 0 {
                                let selection_ready = if deletes_body {
                                    true
                                } else {
                                    set_selection_at_flat(tr, delta.replace_start)?
                                };

                                if selection_ready {
                                    let previous_selection = tr.selection();
                                    for _ in 0..delta.start_tokens {
                                        if !structural_backward(tr)? {
                                            tr.set_selection(previous_selection)?;
                                            break;
                                        }
                                    }
                                }
                            }

                            if !delta.ins_text.is_empty()
                                && let Some(head) = tr.selection().map(|s| s.head)
                            {
                                commands::replace_range_with_text(
                                    tr,
                                    Selection::collapsed(head),
                                    &delta.ins_text,
                                    sidecar_before.clone().or(paint),
                                )?;
                            }
                        }
                        composition_text_start = Some(text_change.replace_start);
                    } else {
                        let sel = selection_from_flat_range(
                            &tr.view(),
                            delta.replace_start,
                            delta.replace_end,
                        )?;
                        let paint = sidecar_before.clone().or_else(|| {
                            delete_paint
                                .as_ref()
                                .filter(|(flat, _)| {
                                    text_change.replace_start == text_change.replace_end
                                        && text_change.replace_start == *flat
                                })
                                .map(|(_, paint)| paint.clone())
                        });
                        commands::replace_range_with_text(tr, sel, &delta.ins_text, paint)?;
                    }
                }

                let composition = match (result.comp, composition_text_start) {
                    // A structural replay rebuilds flat coordinates, so a composing
                    // region the IME placed outside the replayed insert has no
                    // mapping; drop it rather than fail the whole edit.
                    (Some((start, end)), Some(text_start)) => (|| {
                        // Text before the replayed range keeps its flat
                        // coordinates, so a composing region there needs no
                        // remap.
                        if start <= end && end < text_start {
                            return Some(Composition { start, end });
                        }
                        let inserted_len = delta.ins_text.char_count();
                        let relative_start = start.checked_sub(text_start)?;
                        let relative_end = end.checked_sub(text_start)?;
                        if relative_start > relative_end || relative_end > inserted_len {
                            return None;
                        }

                        let doc = tr.view();
                        let inserted_start = tr
                            .selection()
                            .and_then(|selection| selection.head.resolve(&doc))
                            .and_then(|head| head.to_flat().checked_sub(inserted_len))?;

                        Some(Composition {
                            start: inserted_start + relative_start,
                            end: inserted_start + relative_end,
                        })
                    })(),
                    (Some((start, end)), None) => Some(Composition { start, end }),
                    (None, _) => None,
                };
                let composition = composition.filter(|composition| {
                    composition_range_valid(&tr.view(), composition.start, composition.end)
                });

                if composition.is_some() || tr.composition().is_some() {
                    tr.set_composition(composition)?;
                }

                if sidecar_before.is_none()
                    && let Some(comp) = composition
                {
                    let mut paint =
                        seed_composition_paint(&tr.state().projected, comp.start, comp.end);
                    apply_pending(&mut paint, tr.pending_modifiers());
                    tr.clear_pending_format()?;
                    let paint: Vec<Modifier> = paint.into_values().collect();
                    tr.update_meta(|m| m.composition_paint = Some(paint));
                }

                // Without structural replay the reduced coordinates match the
                // document 1:1, so the reduced selection is authoritative
                // (e.g. a delete before the composing text keeps the caret at
                // the composition, not at the deletion site). If the document
                // size disagrees with the reduced buffer, the edit was dropped
                // (e.g. a rejected replacement), so the reduced selection is
                // meaningless — keep the caret where it is.
                if has_text_delta && composition_text_start.is_none() {
                    let doc = tr.view();
                    if flat_size(&doc) == result.text.len() {
                        let current = tr.selection().and_then(|s| {
                            let anchor = s.anchor.resolve(&doc)?.to_flat();
                            let head = s.head.resolve(&doc)?.to_flat();
                            Some((anchor.min(head), anchor.max(head)))
                        });
                        if current != Some((result.sel_start, result.sel_end))
                            && let Ok(sel) =
                                selection_from_flat_range(&doc, result.sel_start, result.sel_end)
                        {
                            commands::set_selection(tr, sel)?;
                        }
                    }
                }

                Ok(())
            })?;
        }

        if !editor.is_probing() {
            editor.ime_delete_paint = next_delete_paint;
        }

        Ok(!text_change.insert.is_empty() && result.comp.is_none())
    })()?;

    if commit_as_is || committed_insert {
        let resource = Arc::clone(&editor.resource);
        let resource = resource.lock().unwrap();
        editor.transact(|tr| {
            commands::optional!(commands::try_text_replacement(&resource))(tr)?;
            Ok(())
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use editor_macros::state;
    use editor_state::assert_state_eq;

    use super::*;
    use crate::test_utils::assert_probe_predicts_apply;

    fn leading_gap_state() -> editor_state::State {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        state
    }

    fn between_monolithic_gap_state() -> editor_state::State {
        let (state, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 1)
        };
        state
    }

    #[test]
    fn probe_composition_commit_empty() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        assert_probe_predicts_apply(
            state,
            Message::TextInput {
                ops: vec![FlatImeOp::CommitAsIs],
            },
        );
    }

    #[test]
    fn probe_composition_clear_no_composition() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        assert_probe_predicts_apply(
            state,
            Message::TextInput {
                ops: vec![FlatImeOp::ClearComposition],
            },
        );
    }

    #[test]
    fn composition_range_valid_rejects_cross_block() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("abc") }
                    paragraph { text("def") }
                }
            }
            selection: (p1, 0)
        };
        let view = state.view();
        // O(p1)=0, abc=1..4, C(p1)=4, O(p2)=5, def=6..9, C(p2)=9
        assert!(composition_range_valid(&view, 1, 4));
        assert!(!composition_range_valid(&view, 1, 5)); // crosses C(p1)
        assert!(composition_range_valid(&view, 6, 9));
    }

    #[test]
    fn composition_range_valid_rejects_out_of_range() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 0)
        };
        let view = state.view();
        // O=0, ab=1..3, C=3; flat_size=4
        assert!(composition_range_valid(&view, 1, 3));
        assert!(!composition_range_valid(&view, 1, 4));
        assert!(!composition_range_valid(&view, 2, 1)); // start > end
    }

    #[test]
    fn composition_range_valid_rejects_empty_range() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let view = state.view();
        // O=0, hello=1..6, C=6; empty ranges never persist as composition
        assert!(!composition_range_valid(&view, 1, 1));
        assert!(!composition_range_valid(&view, 4, 4));
        assert!(!composition_range_valid(&view, 6, 6));
    }

    #[test]
    fn composition_range_valid_rejects_atom() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    image
                    paragraph { text("b") }
                }
            }
            selection: (p1, 0)
        };
        let view = state.view();
        // O(p1)=0, a=1, C(p1)=2, img=3, O(p2)=4, b=5, C(p2)=6
        assert!(composition_range_valid(&view, 1, 2)); // "a" only
        assert!(!composition_range_valid(&view, 2, 4)); // crosses image atom
        assert!(!composition_range_valid(&view, 3, 4)); // starts at image atom
        assert!(composition_range_valid(&view, 5, 6)); // "b" only
    }

    #[test]
    fn composition_range_valid_rejects_open_token() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let view = state.view();
        // O=0, text=1..6, C=6
        assert!(!composition_range_valid(&view, 0, 2)); // includes Open
        assert!(!composition_range_valid(&view, 5, 7)); // includes Close
        assert!(composition_range_valid(&view, 1, 6)); // text only
    }

    #[test]
    fn composition_range_valid_rejects_nested_tokens() {
        let (state, ..) = state! {
            doc { root { blockquote { p1: paragraph { text("hi") } } } }
            selection: (p1, 0)
        };
        let view = state.view();
        // O(bq)=0, O(p)=1, h=2, i=3, C(p)=4, C(bq)=5
        assert!(composition_range_valid(&view, 2, 4)); // "hi"
        assert!(!composition_range_valid(&view, 1, 4)); // includes Open(p)
        assert!(!composition_range_valid(&view, 2, 5)); // includes Close(p)
    }

    #[test]
    fn set_region_stores_valid_range() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 2, end: 5 }],
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 5 })
        );
    }

    #[test]
    fn ime_commit_into_synthetic_trailing_paragraph_after_unit() {
        use editor_model::NodeType;
        use editor_state::{Position, Selection};

        // A Korean/IME commit into the synthetic trailing paragraph after a unit
        // block must materialize it too — the flat-IME path funnels through the
        // same insert_text command as direct typing, so it must not crash.
        let (state, _root) = state! {
            doc { r: root { horizontal_rule } }
            selection: (r, 0)
        };
        let synth_p = {
            let view = state.view();
            let root = view.root().unwrap();
            root.child_blocks()
                .find(|b| b.node_type() == NodeType::Paragraph)
                .map(|b| b.id())
                .expect("synthetic trailing paragraph")
        };
        assert!(synth_p.is_synthetic());

        let mut editor = Editor::new_test(state);
        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: Selection::collapsed(Position::new(synth_p, 0)),
            },
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "가".into() }],
        });

        let (expected, ..) = state! {
            doc { root {
                horizontal_rule
                p1: paragraph { text("가") }
            } }
            selection: (p1, 1, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn ime_composition_into_synthetic_empty_fold_content_paragraph() {
        use editor_model::NodeType;
        use editor_state::{Position, Selection};

        let (state,) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    fold
                    paragraph {}
                }
            }
            selection: none
        };
        let synth_p = {
            let view = state.view();
            let fold = view
                .root()
                .unwrap()
                .child_blocks()
                .find(|b| b.node_type() == NodeType::Fold)
                .expect("fold");
            let content = fold
                .child_blocks()
                .find(|b| b.node_type() == NodeType::FoldContent)
                .expect("synthetic fold content");
            assert!(content.id().is_synthetic());
            let paragraph = content
                .child_blocks()
                .find(|b| b.node_type() == NodeType::Paragraph)
                .expect("synthetic fold content paragraph");
            paragraph.id()
        };
        assert!(synth_p.is_synthetic());

        let mut editor = Editor::new_test(state);
        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: Selection::collapsed(Position::new(synth_p, 0)),
            },
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });

        let (expected, ..) = state! {
            doc {
                root [text_color("black".to_string()), background_color("none".to_string())] {
                    fold {
                        fold_title {}
                        fold_content {
                            p1: paragraph { text("ㅎ") }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (p1, 1, <)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 5, end: 6 })
        );
    }

    #[test]
    fn set_region_rejects_cross_block() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("abc") }
                    p2: paragraph { text("def") }
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 1, end: 6 }],
        });
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn set_region_replaces_prior_composition() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 1, end: 6 }],
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 6 })
        );
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 7, end: 12 }],
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 7, end: 12 })
        );
    }

    #[test]
    fn set_region_invalid_clears_prior_composition() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("abc") }
                    paragraph { text("def") }
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 1, end: 4 }],
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 4 })
        );
        // Now apply invalid cross-block range → should clear prior composition
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 1, end: 6 }],
        });
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn set_region_rejects_atom_via_apply() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("a") }
                    image
                    paragraph { text("b") }
                }
            }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        // O(p1)=0, a=1, C(p1)=2, img=3 — range 3..4 covers the image atom
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 3, end: 4 }],
        });
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn set_region_empty_range_does_not_persist_composition() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 0, end: 0 }],
        });
        assert_eq!(editor.state().composition, None);
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn empty_set_region_then_compose_inserts_at_cursor() {
        // Samsung HoneyBoard post-backspace recompose sends
        // setComposingRegion(0,0) and then setComposingText(word) as separate
        // dispatches while the cursor sits far from position 0; the compose
        // must target the cursor, not position 0.
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 0, end: 0 }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "한".into() }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose {
                text: "한글".into(),
            }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello한글") } } }
            selection: (p1, 7)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 6, end: 8 })
        );
    }

    #[test]
    fn batched_empty_set_region_anchors_compose() {
        // Web IME adapter contract: [set_composition(n,n), compose] in one
        // batch anchors the compose at n even when n differs from the cursor.
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetComposition { start: 3, end: 3 },
                FlatImeOp::Compose { text: "X".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("heXllo") } } }
            selection: (p1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 3, end: 4 })
        );
    }

    #[test]
    fn set_region_reversed_range_normalizes_order() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetComposition { start: 5, end: 2 },
                FlatImeOp::Compose { text: "X".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hXo") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 3 })
        );
    }

    #[test]
    fn rejected_newline_compose_keeps_caret() {
        // replace_range_with_text rejects newline-bearing replacements; the
        // dropped edit must not move the caret to reduced coordinates.
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose {
                text: "a\nb".into(),
            }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn commit_as_is_clears_composition() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 1, end: 4 }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::CommitAsIs],
        });
        assert_eq!(editor.state().composition, None);
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn clear_composition_keeps_composing_text() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 2, end: 5 }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ClearComposition],
        });
        assert_eq!(editor.state().composition, None);
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn clear_composition_without_composition_is_noop() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ClearComposition],
        });
        assert_eq!(editor.state().composition, None);
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn update_no_composition_inserts_at_cursor() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "X".into() }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("heXllo") } } }
            selection: (p1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 3, end: 4 })
        );
    }

    #[test]
    fn update_no_composition_replace_length_deletes_before() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::DeleteSurrounding {
                    before: 2,
                    after: 0,
                },
                FlatImeOp::Compose { text: "XY".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hXYlo") } } }
            selection: (p1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 4 })
        );
    }

    #[test]
    fn delete_surrounding_backward_deletes_text_after_tab() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::DeleteSurrounding {
                before: 1,
                after: 0,
            }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("a") tab } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_surrounding_forward_deletes_text_after_tab() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::DeleteSurrounding {
                before: 0,
                after: 1,
            }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("a") tab } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn replace_selection_empty_deletes_text_after_tab() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("a") tab text("b") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 3, end: 4 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("a") tab } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn update_with_composition_replaces_region() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 4)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 2, end: 5 }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "XYZ".into() }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hXYZo") } } }
            selection: (p1, 4)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 5 })
        );
    }

    #[test]
    fn update_stale_composition_falls_back_to_cursor() {
        let (mut state, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 0)
        };
        // Manually inject a stale composition (range exceeds doc size of 2).
        state.composition = Some(Composition { start: 10, end: 20 });
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "X".into() }],
        });
        // resolve_target should detect stale composition, clear it,
        // and insert "X" at the cursor (flat offset 1).
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("Xhi") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
    }

    #[test]
    fn commit_with_composition_replaces_and_clears() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 4)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 2, end: 5 }],
        });
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::Compose { text: "Y".into() },
                FlatImeOp::CommitAsIs,
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hYo") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn batched_autocomplete_commit_replaces_composition() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "안".into() }],
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::Compose {
                    text: "안녕하세요".into(),
                },
                FlatImeOp::CommitAsIs,
                FlatImeOp::CommitAsIs,
                FlatImeOp::Compose { text: " ".into() },
                FlatImeOp::CommitAsIs,
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("안녕하세요 ") } } }
            selection: (p1, 6)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn commit_no_composition_inserts_at_cursor() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::ReplaceSelection { text: "!".into() },
                FlatImeOp::CommitAsIs,
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hi!") } } }
            selection: (p1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn update_with_cjk_unicode_text() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        // Type "한" (Korean "Han"): single Unicode scalar, 3 UTF-8 bytes, 1 flat offset unit.
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "한".into() }],
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
        // Replace with "안녕": 2 scalars, 2 flat offset units.
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose {
                text: "안녕".into(),
            }],
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 3 })
        );
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("안녕") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn update_preserves_existing_composition_modifiers() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("하") } } }
            selection: (p1, 1)
            pending_modifiers: [bold]
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "하".into() }],
        });

        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("하")
                        text("하") [bold]
                    }
                }
            }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 3 })
        );
    }

    #[test]
    fn update_preserves_font_weight_from_actual_bold_toggle() {
        let mut resource = Resource::new_test();
        resource
            .font_registry
            .set_fonts(vec![editor_resource::FontFamily {
                name: "Pretendard".into(),
                source: editor_resource::FontFamilySource::Default,
                weights: vec![
                    editor_resource::FontWeight {
                        value: 400,
                        hash: "pretendard_400".into(),
                    },
                    editor_resource::FontWeight {
                        value: 700,
                        hash: "pretendard_700".into(),
                    },
                ],
            }]);
        let resource = Arc::new(Mutex::new(resource));
        let (state, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph { text("하") }
                }
            }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test_with_resource(state, resource);

        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: editor_model::ModifierType::Bold,
            },
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "하".into() }],
        });

        let (expected, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    p1: paragraph {
                        text("하")
                        text("하") [font_weight(700)]
                    }
                }
            }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 3 })
        );
    }

    #[test]
    fn update_preserves_regular_composition_after_bold_text() {
        let (mut state, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("A") [bold]
                        text("ㅎ")
                    }
                }
            }
            selection: (p1, 2)
        };
        state.composition = Some(Composition { start: 2, end: 3 });
        let mut editor = Editor::new_test(state);

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "하".into() }],
        });

        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("A") [bold]
                        text("하")
                    }
                }
            }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 3 })
        );
    }

    #[test]
    fn flat_ime_delete_then_compose_preserves_deleted_modifiers() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("하")
                        text("ㅎ") [italic]
                    }
                }
            }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::DeleteSurrounding {
                    before: 1,
                    after: 0,
                },
                FlatImeOp::Compose { text: "하".into() },
            ],
        });

        let (expected, ..) = state! {
            doc {
                root {
                    p1: paragraph {
                        text("하")
                        text("하") [italic]
                    }
                }
            }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 3 })
        );
    }

    #[test]
    fn commit_empty_text_deletes_composition_region() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 4)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 2, end: 5 }],
        });
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::Compose { text: "".into() },
                FlatImeOp::CommitAsIs,
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ho") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn commit_with_cjk_unicode_text() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::ReplaceSelection {
                    text: "안녕".into(),
                },
                FlatImeOp::CommitAsIs,
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hi안녕") } } }
            selection: (p1, 4)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn commit_stale_composition_falls_back_to_cursor() {
        let (mut state, ..) = state! {
            doc { root { p1: paragraph { text("hi") } } }
            selection: (p1, 1)
        };
        // Inject stale composition (range exceeds doc size of 2).
        state.composition = Some(Composition { start: 10, end: 20 });
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::ReplaceSelection { text: "X".into() },
                FlatImeOp::CommitAsIs,
            ],
        });
        // resolve_target detects stale composition, clears it, inserts "X" at cursor (flat 2).
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hXi") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn retroactive_composition_across_formatting_boundary() {
        // "안"[bold] + "녕" (two text nodes in same paragraph, different modifiers)
        let (state, p1) = state! {
            doc { root { p1: paragraph {
                text("안") [bold]
                text("녕")
            }}}
            selection: (p1, 2)  // cursor after "녕"
        };
        let mut editor = Editor::new_test(state);

        // IME: SetComposingRegion(1, 3) covers "안녕" (cross-node)
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 1, end: 3 }],
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 3 })
        );

        // IME: Update("안녕하", None) — replace composing region with new text
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose {
                text: "안녕하".into(),
            }],
        });

        // After: composition should be { start: 1, end: 4 }
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 4 })
        );

        let view = editor.state().view();
        assert_eq!(editor_state::flat_text(&view, 1..4), "안녕하");

        let para = view.node(p1).unwrap();
        assert!(
            para.inline().iter().all(|item| item
                .effective
                .values()
                .any(|m| matches!(m, editor_model::Modifier::Bold))),
            "the replacement inherits the first-charlike own paint (bold) across the whole composition"
        );
    }

    #[test]
    fn composition_start_seeds_bold_neighbor_and_consumes_italic_pending() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("가") [bold] } } }
            selection: (p1, 1)
            pending_modifiers: [italic]
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        assert!(
            editor.state().pending_modifiers.is_empty(),
            "pending consumed at composition start"
        );

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "하".into() }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::CommitAsIs],
        });

        let (expected, ..) = state! {
            doc { root { p1: paragraph {
                text("가") [bold]
                text("하") [bold, italic]
            } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn empty_set_region_keeps_pending_for_first_composed_char() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("하") } } }
            selection: (p1, 1)
            pending_modifiers: [italic]
        };
        let mut editor = Editor::new_test(state);

        let flat = {
            let view = editor.state().view();
            editor
                .state()
                .selection
                .unwrap()
                .head
                .resolve(&view)
                .unwrap()
                .to_flat()
        };
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition {
                start: flat,
                end: flat,
            }],
        });
        assert!(
            !editor.state().pending_modifiers.is_empty(),
            "pending survives the empty SetRegion and is consumed at first compose"
        );

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "가".into() }],
        });

        let (expected, ..) = state! {
            doc { root { p1: paragraph {
                text("하")
                text("가") [italic]
            } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn delete_then_compose_seed_overlays_and_consumes_pending() {
        let (state, ..) = state! {
            doc { root { p1: paragraph {
                text("하")
                text("ㅎ") [italic]
            } } }
            selection: (p1, 2)
            pending_modifiers: [bold]
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::DeleteSurrounding {
                    before: 1,
                    after: 0,
                },
                FlatImeOp::Compose { text: "하".into() },
            ],
        });
        assert!(
            editor.state().pending_modifiers.is_empty(),
            "pending consumed at composition start"
        );

        let (expected, ..) = state! {
            doc { root { p1: paragraph {
                text("하")
                text("하") [italic, bold]
            } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    fn remote_change(
        base: &editor_state::State,
        ops: Vec<editor_model::EditOp>,
    ) -> editor_crdt::Changeset<editor_model::EditOp> {
        use hashbrown::HashSet;
        let mut pa = base.projected.as_ref().clone();
        let baseline: HashSet<editor_crdt::Dot> = pa.graph().current_heads().copied().collect();
        pa.apply_batch(ops).unwrap();
        pa.commit();
        pa.graph()
            .local_changesets_since(&baseline)
            .unwrap()
            .remove(0)
    }

    #[test]
    fn remote_concurrent_edit_preserves_confirmed_composition_paint() {
        use editor_crdt::ListOp;
        use editor_model::{EditOp, SeqItem};

        let (replica_a, _p1) = state! {
            doc { root { p1: paragraph { text("가") [bold] } } }
            selection: (p1, 1)
        };
        let css_a = replica_a.graph().changesets_as_vec();
        let replica_b = editor_state::State::from_changesets(css_a, replica_a.selection).unwrap();
        let mut editor = Editor::new_test(replica_b);

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        let seeded = editor.composition_paint.clone();
        assert!(
            seeded
                .as_ref()
                .is_some_and(|p| p.iter().any(|m| matches!(m, editor_model::Modifier::Bold))),
            "composition start seeds the bold neighbor paint"
        );

        let cs = remote_change(
            &replica_a,
            vec![EditOp::Seq(ListOp::Ins {
                pos: 1,
                item: SeqItem::Char('Z'),
            })],
        );
        editor.receive_remote_changeset(cs);
        let _ = editor.tick().unwrap();

        assert_eq!(
            editor.composition_paint, seeded,
            "the confirmed composition paint survives a concurrent remote edit"
        );
    }

    #[test]
    fn undo_dissolves_active_composition() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("a") } } }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(state);

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        assert!(editor.state().composition.is_some(), "composition active");
        assert!(editor.composition_paint.is_some(), "sidecar seeded");

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });

        assert!(
            editor.state().composition.is_none(),
            "a successful undo dissolves the active composition rather than restoring it"
        );
        assert!(
            editor.composition_paint.is_none(),
            "the sidecar is dropped alongside the dissolved composition"
        );
        let view = editor.state().view();
        let full = editor_state::flat_text(&view, 0..editor_state::flat_size(&view));
        assert!(!full.contains('ㅎ'), "the composed char is undone");
    }

    #[test]
    fn noop_undo_without_history_keeps_pending() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("a") } } }
            selection: (p1, 1)
            pending_modifiers: [bold]
        };
        let mut editor = Editor::new_test(state);
        assert!(
            !editor.undo_history.can_undo(),
            "no history: undo is a no-op"
        );

        editor.apply(Message::History {
            op: HistoryOp::Undo,
        });

        assert_eq!(
            editor.state().pending_modifiers.as_slice(),
            &[editor_state::PendingModifier::Set {
                modifier: editor_model::Modifier::Bold
            }],
            "a no-op undo moves no cursor and so keeps the pending format"
        );
    }

    fn flat_ime_state(text: &str, sel: usize) -> FlatImeState {
        FlatImeState {
            text: FlatText::whole(text.chars().collect()),
            sel_start: sel,
            sel_end: sel,
            comp: None,
        }
    }

    fn flat_ime_state_sel(text: &str, sel_start: usize, sel_end: usize) -> FlatImeState {
        FlatImeState {
            text: FlatText::whole(text.chars().collect()),
            sel_start,
            sel_end,
            comp: None,
        }
    }

    fn flat_ime_text(s: &FlatImeState) -> String {
        s.text.chars.iter().collect()
    }

    #[test]
    fn replace_selection_inserts_text() {
        let s = flat_ime_state("hello", 5);
        let s = s.reduce(&[FlatImeOp::ReplaceSelection { text: "!".into() }]);
        assert_eq!(flat_ime_text(&s), "hello!");
        assert_eq!(s.sel_start, 6);
    }

    #[test]
    fn replace_selection_replaces_range() {
        let s = flat_ime_state_sel("hello", 1, 4);
        let s = s.reduce(&[FlatImeOp::ReplaceSelection { text: "X".into() }]);
        assert_eq!(flat_ime_text(&s), "hXo");
        assert_eq!(s.sel_start, 2);
    }

    #[test]
    fn delete_surrounding_backward() {
        let s = flat_ime_state("hello", 3);
        let s = s.reduce(&[FlatImeOp::DeleteSurrounding {
            before: 2,
            after: 0,
        }]);
        assert_eq!(flat_ime_text(&s), "hlo");
        assert_eq!(s.sel_start, 1);
    }

    #[test]
    fn compose_without_existing_composition() {
        let s = flat_ime_state("hello", 3);
        let s = s.reduce(&[FlatImeOp::Compose { text: "X".into() }]);
        assert_eq!(flat_ime_text(&s), "helXlo");
        assert_eq!(s.comp, Some((3, 4)));
    }

    #[test]
    fn compose_replaces_existing_composition() {
        let mut s = flat_ime_state("hello", 3);
        s.comp = Some((1, 4));
        let s = s.reduce(&[FlatImeOp::Compose { text: "XY".into() }]);
        assert_eq!(flat_ime_text(&s), "hXYo");
        assert_eq!(s.comp, Some((1, 3)));
    }

    #[test]
    fn korean_ime_recomposition_batch() {
        let o = FLAT_OPEN;
        let c = FLAT_CLOSE;
        let initial = format!("{o}!ㅇ{c}");
        let s = FlatImeState {
            text: FlatText::whole(initial.chars().collect()),
            sel_start: 3,
            sel_end: 3,
            comp: None,
        };
        let s = s.reduce(&[
            FlatImeOp::SetSelection { start: 0, end: 3 },
            FlatImeOp::ReplaceSelection { text: "".into() },
            FlatImeOp::ReplaceSelection {
                text: format!("{o}"),
            },
            FlatImeOp::ReplaceSelection { text: "아".into() },
        ]);
        assert_eq!(flat_ime_text(&s), format!("{o}아{c}"));
    }

    #[test]
    fn empty_paragraph_backspace_batch() {
        let o = FLAT_OPEN;
        let c = FLAT_CLOSE;
        let initial = format!("{o}{c}");
        let s = FlatImeState {
            text: FlatText::whole(initial.chars().collect()),
            sel_start: 1,
            sel_end: 1,
            comp: None,
        };
        let s = s.reduce(&[
            FlatImeOp::SetSelection { start: 0, end: 1 },
            FlatImeOp::ReplaceSelection { text: "".into() },
        ]);
        assert_eq!(flat_ime_text(&s), format!("{c}"));
    }

    use editor_resource::Resource;
    use std::sync::{Arc, Mutex};

    fn editor_with_resource(s: editor_state::State) -> Editor {
        let resource = Arc::new(Mutex::new(Resource::new_test()));
        Editor::new_test_with_resource(s, resource)
    }

    fn apply_flat_ime_ops(s: editor_state::State, ops: Vec<FlatImeOp>) -> Editor {
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput { ops });
        editor
    }

    #[test]
    fn flat_ime_text_replacement() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 5)
        };
        let editor = apply_flat_ime_ops(s, vec![FlatImeOp::ReplaceSelection { text: "!".into() }]);
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hello!") } } }
            selection: (p1, 6)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_repeated_text_insertion_uses_cursor_position() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("aaaa") } } }
            selection: (p1, 0)
        };
        let editor = apply_flat_ime_ops(s, vec![FlatImeOp::ReplaceSelection { text: "a".into() }]);
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("aaaaa") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_repeated_text_middle_insertion_uses_cursor_position() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("aaaa") } } }
            selection: (p1, 2)
        };
        let editor = apply_flat_ime_ops(s, vec![FlatImeOp::ReplaceSelection { text: "a".into() }]);
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("aaaaa") } } }
            selection: (p1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_all_with_same_text_places_cursor_after_inserted_text() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("a") } } }
            selection: (p1, 1)
        };
        let editor = apply_flat_ime_ops(
            s,
            vec![
                FlatImeOp::SetSelection { start: 0, end: 3 },
                FlatImeOp::ReplaceSelection { text: "a".into() },
            ],
        );
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("a") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_nested_full_selection_removes_structure() {
        let (s, ..) = state! {
            doc { root { blockquote { p1: paragraph { text("a") } } } }
            selection: (p1, 1)
        };
        let editor = apply_flat_ime_ops(
            s,
            vec![
                FlatImeOp::SetSelection { start: 0, end: 5 },
                FlatImeOp::ReplaceSelection { text: "a".into() },
            ],
        );
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("a") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_disjoint_text_edits_are_ignored() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("abcdef") } } }
            selection: (p1, 0)
        };
        let editor = apply_flat_ime_ops(
            s,
            vec![
                FlatImeOp::SetSelection { start: 2, end: 3 },
                FlatImeOp::ReplaceSelection { text: "B".into() },
                FlatImeOp::SetSelection { start: 5, end: 6 },
                FlatImeOp::ReplaceSelection { text: "E".into() },
            ],
        );
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("abcdef") } } }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_disjoint_text_edits_at_gap_cursor_do_not_materialize() {
        let (s, ..) = state! {
            doc { r: root { image paragraph { text("abcdef") } } }
            selection: (r, 0, <)
        };
        let mut editor = editor_with_resource(s);
        let view = editor.state().view();
        let flat_text = editor_state::flat_text(&view, 0..flat_size(&view));
        drop(view);
        let text_start = flat_text
            .find("abcdef")
            .map(|idx| flat_text[..idx].chars().count())
            .unwrap();

        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection {
                    start: text_start + 1,
                    end: text_start + 2,
                },
                FlatImeOp::ReplaceSelection { text: "B".into() },
                FlatImeOp::SetSelection {
                    start: text_start + 4,
                    end: text_start + 5,
                },
                FlatImeOp::ReplaceSelection { text: "E".into() },
            ],
        });

        let (expected, ..) = state! {
            doc { r: root { image paragraph { text("abcdef") } } }
            selection: (r, 0, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_repeated_composition_middle_insertion_uses_cursor_position() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("ㅁㅁㅁㅁ") } } }
            selection: (p1, 2)
        };
        let editor = apply_flat_ime_ops(s, vec![FlatImeOp::Compose { text: "ㅁ".into() }]);
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ㅁㅁㅁㅁㅁ") } } }
            selection: (p1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 3, end: 4 })
        );
    }

    #[test]
    fn flat_ime_repeated_composition_recomposition_uses_replaced_range() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("ㅁㅁㅁㅁ") } } }
            selection: (p1, 2)
        };
        let editor = apply_flat_ime_ops(
            s,
            vec![
                FlatImeOp::DeleteSurrounding {
                    before: 1,
                    after: 0,
                },
                FlatImeOp::Compose {
                    text: "ㅁㅁ".into(),
                },
            ],
        );
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ㅁㅁㅁㅁㅁ") } } }
            selection: (p1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 4 })
        );
    }

    #[test]
    fn flat_ime_repeated_composition_recomposition_commit_keeps_cursor_position() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("ㅁㅁㅁㅁ") } } }
            selection: (p1, 2)
        };
        let editor = apply_flat_ime_ops(
            s,
            vec![
                FlatImeOp::DeleteSurrounding {
                    before: 1,
                    after: 0,
                },
                FlatImeOp::Compose {
                    text: "ㅁㅁ".into(),
                },
                FlatImeOp::ClearComposition,
            ],
        );
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ㅁㅁㅁㅁㅁ") } } }
            selection: (p1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn flat_ime_replace_selection_replaces_empty_paragraph_selection() {
        let (s, ..) = state! {
            doc { r: root { paragraph {} } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "a".into() }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("a") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn flat_ime_replace_selection_preserves_unit_selection_inserts_after() {
        let (s, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "b".into() }],
        });
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                horizontal_rule
                p1: paragraph { text("b") }
                paragraph { text("c") }
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn flat_ime_replace_mixed_seam_and_table_body_removes_body() {
        let (s, ..) = state! {
            doc { root {
                p1: paragraph { text("aa") }
                table(proportion: 21) {
                    table_row {
                        table_cell(col_width: Some(40)) {
                            paragraph {}
                        }
                        table_cell(col_width: Some(515)) {
                            paragraph {}
                        }
                    }
                }
                p2: paragraph { text("cc") }
            } }
            selection: (p2, 0, <) -> (p1, 2, >)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "d".into() }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("aadcc") } } }
            selection: (p1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn flat_ime_delete_mixed_seam_and_table_body_removes_body() {
        let (s, ..) = state! {
            doc { root {
                p1: paragraph { text("aa") }
                table(proportion: 21) {
                    table_row {
                        table_cell(col_width: Some(40)) {
                            paragraph {}
                        }
                        table_cell(col_width: Some(515)) {
                            paragraph {}
                        }
                    }
                }
                p2: paragraph { text("cc") }
            } }
            selection: (p2, 0, <) -> (p1, 2, >)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "".into() }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("aacc") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn flat_ime_compose_mixed_seam_and_table_body_sets_composition() {
        let (s, ..) = state! {
            doc { root {
                p1: paragraph { text("aa") }
                table(proportion: 21) {
                    table_row {
                        table_cell(col_width: Some(40)) {
                            paragraph {}
                        }
                        table_cell(col_width: Some(515)) {
                            paragraph {}
                        }
                    }
                }
                p2: paragraph { text("cc") }
            } }
            selection: (p2, 0, <) -> (p1, 2, >)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("aaㅎcc") } } }
            selection: (p1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 3, end: 4 })
        );
    }

    #[test]
    fn flat_ime_compose_nested_seam_and_table_body_sets_composition() {
        let (s, ..) = state! {
            doc { root {
                callout {
                    p1: paragraph { text("aa") }
                }
                table(proportion: 21) {
                    table_row {
                        table_cell(col_width: Some(40)) {
                            paragraph {}
                        }
                        table_cell(col_width: Some(515)) {
                            paragraph {}
                        }
                    }
                }
                p2: paragraph { text("cc") }
            } }
            selection: (p2, 0, <) -> (p1, 2, >)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        let (expected, ..) = state! {
            doc { root {
                callout {
                    p1: paragraph { text("aaㅎcc") }
                }
                paragraph {}
            } }
            selection: (p1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        // O(callout)=0, O(p1)=1, a=2, a=3, ㅎ=4
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 4, end: 5 })
        );
    }

    #[test]
    fn flat_ime_compose_preserves_unit_selection_inserts_after() {
        let (s, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                horizontal_rule
                p1: paragraph { text("ㅎ") }
                paragraph { text("c") }
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 5, end: 6 })
        );
    }

    #[test]
    fn flat_ime_compose_replaces_empty_paragraph_selection() {
        let (s, ..) = state! {
            doc { r: root { paragraph {} } }
            selection: (r, 0, >) -> (r, 1, <)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ㅎ") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
    }

    #[test]
    fn flat_ime_compose_replaces_nested_empty_paragraph_selection() {
        let (s, ..) = state! {
            doc { root {
                bq: blockquote { paragraph {} }
                paragraph { text("after") }
            } }
            selection: (bq, 0, >) -> (bq, 1, <)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "나".into() }],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("나") } }
                paragraph { text("after") }
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 3 })
        );
    }

    #[test]
    fn flat_ime_composition_replaces_full_structural_selection_with_text() {
        let (s, ..) = state! {
            doc {
                r1: root {
                    paragraph { text("a") }
                    table(proportion: 21) {
                        table_row {
                            table_cell(col_width: Some(40)) {
                                paragraph {}
                            }
                            table_cell(col_width: Some(515)) {
                                paragraph {}
                            }
                        }
                        table_row {
                            table_cell(col_width: Some(40)) {
                                paragraph {}
                            }
                            table_cell(col_width: Some(515)) {
                                paragraph {}
                            }
                        }
                    }
                    paragraph {}
                }
            }
            selection: (r1, 0, >) -> (r1, 3, <)
        };
        let mut editor = editor_with_resource(s);
        let end = flat_size(&editor.state().view());

        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetComposition { start: 0, end },
                FlatImeOp::Compose { text: "ㅁ".into() },
            ],
        });

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ㅁ") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
    }

    #[test]
    fn flat_ime_korean_recomposition_preserves_structure() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("!ㅇ") } } }
            selection: (p1, 2)
        };
        let mut editor = editor_with_resource(s);
        let o = "\u{2028}";
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 0, end: 3 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection { text: o.into() },
                FlatImeOp::ReplaceSelection {
                    text: "!아".into()
                },
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("!아") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_no_change_is_noop() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetSelection { start: 4, end: 4 }],
        });
        let view = editor.state().view();
        assert_eq!(editor_state::flat_text(&view, 1..6), "hello");
    }

    #[test]
    fn flat_ime_pua_reinsert_filtered() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 2)
        };
        let mut editor = editor_with_resource(s);
        let o = "\u{2028}";
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 0, end: 3 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection { text: o.into() },
                FlatImeOp::ReplaceSelection { text: "ab".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ab") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_delete_surrounding() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::DeleteSurrounding {
                before: 2,
                after: 0,
            }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("hlo") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_compose_sets_composition() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 3)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "X".into() }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("helXlo") } } }
            selection: (p1, 4)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 4, end: 5 })
        );
    }

    #[test]
    fn flat_ime_bulk_backward_delete_at_boundary_does_structural() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("paragraph1") } p2: paragraph { text("") } } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::DeleteSurrounding {
                before: 1,
                after: 0,
            }],
        });
        let view = editor.state().view();
        let flat = editor_state::flat_text(&view, 0..flat_size(&view));
        assert!(
            !flat.contains("\u{2028}\u{2029}\u{2029}"),
            "empty paragraph should have been removed"
        );
    }

    #[test]
    fn flat_ime_join_paragraph_backward_cursor_at_end() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("A") } p2: paragraph {} } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 3, end: 4 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("A") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn structural_backward_merge_drops_front_trailing_page_break() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { text("AB") page_break }
                    cur: paragraph { text("CD") }
                }
            }
            selection: (cur, 0)
        };
        let mut tr = Transaction::new(&state);
        assert!(structural_backward(&mut tr).unwrap());
        let (actual, ..) = tr.commit();
        let (expected, ..) = state! {
            doc {
                root {
                    m: paragraph {
                        text("AB")
                        text("CD")
                    }
                }
            }
            selection: (m, 2, <)
        };
        assert_state_eq!(actual, expected);
    }

    #[test]
    fn structural_forward_merge_drops_front_trailing_page_break() {
        let (state, ..) = state! {
            doc {
                root {
                    p1: paragraph { text("AB") page_break }
                    paragraph { text("CD") }
                }
            }
            selection: (p1, 3)
        };
        let mut tr = Transaction::new(&state);
        assert!(structural_forward(&mut tr).unwrap());
        let (actual, ..) = tr.commit();
        let (expected, ..) = state! {
            doc {
                root {
                    m: paragraph {
                        text("AB")
                        text("CD")
                    }
                }
            }
            selection: (m, 2)
        };
        assert_state_eq!(actual, expected);
    }

    #[test]
    fn flat_ime_empty_paragraph_backspace_removes_paragraph() {
        let (s, ..) = state! {
            doc { root { p1: paragraph { text("hello") } p2: paragraph {} } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        let original_size = flat_size(&editor.state().view());
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 7, end: 8 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let new_size = flat_size(&editor.state().view());
        assert!(
            new_size < original_size,
            "empty paragraph should be removed: new_size={new_size} original={original_size}"
        );
    }

    #[test]
    fn flat_ime_input_context_has_tokens() {
        let (s, ..) = state! {
            doc { root { blockquote { p1: paragraph { text("") } } paragraph {} } }
            selection: (p1, 0)
        };
        let editor = Editor::new_test(s);
        let ctx = editor.ime(100, 100).unwrap().unwrap();
        assert!(
            !ctx.text.is_empty(),
            "empty blockquote paragraph should have tokens in buffer"
        );
        assert!(
            ctx.text.contains(FLAT_OPEN),
            "buffer should contain Open tokens"
        );
    }

    #[test]
    fn flat_ime_bulk_delete_single_open_token() {
        let (s, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("hello") } }
                p2: paragraph { text("world") }
            } }
            selection: (p2, 1)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 9, end: 10 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph { text("hello") }
                    p2: paragraph { text("world") }
                }
                paragraph {}
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_bulk_delete_close_open_pair() {
        let (s, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("hello") } }
                p2: paragraph { text("world") }
            } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 8, end: 10 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph { text("hello") }
                    p2: paragraph { text("world") }
                }
                paragraph {}
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_bulk_delete_two_boundaries() {
        let (s, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("hello") } }
                p2: paragraph { text("world") }
            } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 7, end: 10 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("helloworld") } }
                paragraph {}
            } }
            selection: (p1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_bulk_delete_single_open_token_through_first_text() {
        let (s, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("hello") } }
                p2: paragraph { text("world") }
            } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 9, end: 11 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph { text("hello") }
                    p2: paragraph { text("orld") }
                }
                paragraph {}
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_bulk_delete_close_open_pair_through_first_text() {
        let (s, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("hello") } }
                p2: paragraph { text("world") }
            } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 8, end: 11 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph { text("hello") }
                    p2: paragraph { text("orld") }
                }
                paragraph {}
            } }
            selection: (p2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_bulk_delete_two_boundaries_through_first_text() {
        let (s, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("hello") } }
                p2: paragraph { text("world") }
            } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 7, end: 11 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("helloorld") } }
                paragraph {}
            } }
            selection: (p1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_single_open_token_through_first_text() {
        let (s, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("hello") } }
                p2: paragraph { text("world") }
            } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 9, end: 11 },
                FlatImeOp::ReplaceSelection { text: "a".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph { text("hello") }
                    p2: paragraph { text("aorld") }
                }
                paragraph {}
            } }
            selection: (p2, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_close_open_pair_through_first_text() {
        let (s, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("hello") } }
                p2: paragraph { text("world") }
            } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 8, end: 11 },
                FlatImeOp::ReplaceSelection { text: "a".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph { text("hello") }
                    p2: paragraph { text("aorld") }
                }
                paragraph {}
            } }
            selection: (p2, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_two_boundaries_through_first_text() {
        let (s, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("hello") } }
                p2: paragraph { text("world") }
            } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 7, end: 11 },
                FlatImeOp::ReplaceSelection { text: "a".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("helloaorld") } }
                paragraph {}
            } }
            selection: (p1, 6)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_single_open_token_inserts_at_boundary() {
        let (s, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("hello") } }
                p2: paragraph { text("world") }
            } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 9, end: 10 },
                FlatImeOp::ReplaceSelection { text: "a".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph { text("hello") }
                    p2: paragraph { text("aworld") }
                }
                paragraph {}
            } }
            selection: (p2, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_close_open_pair_inserts_at_boundary() {
        let (s, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("hello") } }
                p2: paragraph { text("world") }
            } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 8, end: 10 },
                FlatImeOp::ReplaceSelection { text: "a".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    p1: paragraph { text("hello") }
                    p2: paragraph { text("aworld") }
                }
                paragraph {}
            } }
            selection: (p2, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_two_boundaries_inserts_at_join() {
        let (s, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("hello") } }
                p2: paragraph { text("world") }
            } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 7, end: 10 },
                FlatImeOp::ReplaceSelection { text: "a".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("helloaworld") } }
                paragraph {}
            } }
            selection: (p1, 6)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn composition_commit_preserves_unit_selection_inserts_after() {
        let (s, ..) = state! {
            doc { r: root {
                paragraph { text("a") }
                horizontal_rule
                paragraph { text("c") }
            } }
            selection: (r, 1, >) -> (r, 2, <)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::Compose { text: "b".into() },
                FlatImeOp::CommitAsIs,
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                paragraph { text("a") }
                horizontal_rule
                p1: paragraph { text("b") }
                paragraph { text("c") }
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_bulk_delete_text_across_structure() {
        let (s, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("hello") } }
                p2: paragraph { text("world") }
            } }
            selection: (p2, 3)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 3, end: 13 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote { p1: paragraph { text("hld") } }
                paragraph {}
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn update_at_leading_gap_materializes_and_inserts() {
        let mut editor = editor_with_resource(leading_gap_state());
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ㅎ") } image paragraph { text("b") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
    }

    #[test]
    fn update_at_between_monolithic_gap_materializes_and_inserts() {
        let mut editor = editor_with_resource(between_monolithic_gap_state());
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        let (expected, ..) = state! {
            doc { root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                p1: paragraph { text("ㅎ") }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn replace_selection_at_leading_gap_materializes_and_inserts() {
        let mut editor = editor_with_resource(leading_gap_state());
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 0, end: 0 },
                FlatImeOp::ReplaceSelection { text: "a".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("a") } image paragraph { text("b") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn replace_selection_at_between_monolithic_gap_materializes_and_inserts() {
        let mut editor = editor_with_resource(between_monolithic_gap_state());
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 10, end: 10 },
                FlatImeOp::ReplaceSelection { text: "X".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                p1: paragraph { text("X") }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn compose_batch_at_leading_gap_materializes_and_composes() {
        let mut editor = editor_with_resource(leading_gap_state());
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetComposition { start: 0, end: 0 },
                FlatImeOp::Compose { text: "ㅎ".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ㅎ") } image paragraph { text("b") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
    }

    #[test]
    fn compose_batch_at_between_monolithic_gap_materializes_and_composes() {
        let mut editor = editor_with_resource(between_monolithic_gap_state());
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetComposition { start: 10, end: 10 },
                FlatImeOp::Compose { text: "ㅎ".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                p1: paragraph { text("ㅎ") }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert!(editor.state().composition.is_some());
    }

    #[test]
    fn compose_commit_batch_at_leading_gap_materializes_and_commits() {
        let mut editor = editor_with_resource(leading_gap_state());
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetComposition { start: 0, end: 0 },
                FlatImeOp::Compose { text: "안".into() },
                FlatImeOp::CommitAsIs,
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("안") } image paragraph { text("b") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn update_with_empty_text_at_leading_gap_preserves_gap() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ClearComposition],
        });
        let (expected, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn update_after_empty_set_region_at_leading_gap_materializes_and_inserts() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 0, end: 0 }],
        });
        assert_eq!(
            editor.state().composition,
            None,
            "precondition: empty SetRegion never persists composition"
        );
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ㅎ") } image paragraph { text("b") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
    }

    #[test]
    fn commit_at_leading_gap_materializes_and_inserts_clears_composition() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::Compose { text: "안".into() },
                FlatImeOp::CommitAsIs,
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("안") } image paragraph { text("b") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn commit_at_between_monolithic_gap_materializes_and_inserts() {
        let (state, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 1)
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::Compose { text: "X".into() },
                FlatImeOp::CommitAsIs,
            ],
        });
        let (expected, ..) = state! {
            doc { root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                p1: paragraph { text("X") }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn commit_with_empty_text_at_leading_gap_preserves_gap() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::CommitAsIs],
        });
        let (expected, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn commit_after_empty_set_region_at_leading_gap_materializes_and_inserts() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 0, end: 0 }],
        });
        assert_eq!(
            editor.state().composition,
            None,
            "precondition: empty SetRegion never persists composition"
        );
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::Compose { text: "안".into() },
                FlatImeOp::CommitAsIs,
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("안") } image paragraph { text("b") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn flat_ime_compose_at_leading_gap_materializes_and_composes() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ㅎ") } image paragraph { text("b") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
    }

    #[test]
    fn flat_ime_compose_at_between_monolithic_gap_materializes_and_composes() {
        let (state, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 1)
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        let (expected, ..) = state! {
            doc { root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                p1: paragraph { text("ㅎ") }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_no_text_delta_at_leading_gap_preserves_gap() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetSelection { start: 0, end: 0 }],
        });
        let (expected, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn flat_ime_set_composition_only_at_leading_gap_preserves_gap() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 0, end: 0 }],
        });
        let (expected, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        assert_state_eq!(editor.state(), &expected);
        // Pin the "state-only Flat op survives the gap gate" property: the op
        // must not materialize the gap, and the empty region must not persist
        // as composition (AOSP parity).
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn flat_ime_compose_after_empty_set_region_at_leading_gap_materializes_and_composes() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 0, end: 0 }],
        });
        assert_eq!(
            editor.state().composition,
            None,
            "precondition: empty SetRegion never persists composition"
        );
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ㅎ") } image paragraph { text("b") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
    }

    #[test]
    fn clear_after_empty_set_region_at_leading_gap_preserves_unit() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 0, end: 0 }],
        });
        assert_eq!(
            editor.state().composition,
            None,
            "precondition: empty SetRegion never persists composition"
        );
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ClearComposition],
        });
        let (expected, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn set_composition_at_leading_gap_does_not_materialize() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 0, end: 0 }],
        });
        let (expected, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn clear_composition_at_leading_gap_is_noop() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ClearComposition],
        });
        let (expected, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn commit_as_is_at_leading_gap_is_noop() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::CommitAsIs],
        });
        let (expected, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn update_at_leading_gap_preserves_pending_modifiers() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
            pending_modifiers: [bold]
        };
        let mut editor = editor_with_resource(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "X".into() }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("X") [bold] } image paragraph { text("b") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    fn paint_at(editor: &Editor, p: editor_crdt::Dot, slot: usize) -> Vec<Modifier> {
        editor_state::replacement_paint(
            &editor.state().projected,
            Position::new(p, slot),
            Position::new(p, slot + 1),
        )
        .unwrap_or_default()
    }

    #[test]
    fn ime_single_message_delete_then_insert_preserves_deleted_paint() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("가") } } }
            selection: (p1, 1)
            pending_modifiers: [bold]
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "ㅇ".into() }],
        });
        assert_eq!(paint_at(&editor, p1, 1), vec![Modifier::Bold], "ㅇ bold");

        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::DeleteSurrounding {
                    before: 1,
                    after: 0,
                },
                FlatImeOp::ReplaceSelection { text: "아".into() },
            ],
        });
        assert_eq!(paint_at(&editor, p1, 1), vec![Modifier::Bold], "아 bold");
    }

    #[test]
    fn ime_split_messages_delete_then_insert_preserves_deleted_paint() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("가") } } }
            selection: (p1, 1)
            pending_modifiers: [bold]
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "ㅇ".into() }],
        });
        assert_eq!(paint_at(&editor, p1, 1), vec![Modifier::Bold], "ㅇ bold");

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::DeleteSurrounding {
                before: 1,
                after: 0,
            }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "아".into() }],
        });
        assert_eq!(paint_at(&editor, p1, 1), vec![Modifier::Bold], "아 bold");
    }

    #[test]
    fn ime_single_message_utf16_delete_then_insert_preserves_deleted_paint() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("가") } } }
            selection: (p1, 1)
            pending_modifiers: [bold]
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "ㅇ".into() }],
        });
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::DeleteSurroundingUtf16 {
                    before: 1,
                    after: 0,
                },
                FlatImeOp::ReplaceSelection { text: "아".into() },
            ],
        });
        assert_eq!(paint_at(&editor, p1, 1), vec![Modifier::Bold], "아 bold");
    }

    #[test]
    fn ime_single_message_recompose_sole_char_preserves_deleted_paint() {
        let (state, p1) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
            pending_modifiers: [bold]
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "ㅇ".into() }],
        });
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::DeleteSurroundingUtf16 {
                    before: 1,
                    after: 0,
                },
                FlatImeOp::ReplaceSelection { text: "아".into() },
            ],
        });
        assert_eq!(paint_at(&editor, p1, 0), vec![Modifier::Bold], "아 bold");
    }

    #[test]
    fn ime_split_delete_paint_invalidated_by_intervening_message() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("가") } } }
            selection: (p1, 1)
            pending_modifiers: [bold]
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "ㅇ".into() }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::DeleteSurroundingUtf16 {
                before: 1,
                after: 0,
            }],
        });
        editor.apply(Message::Navigation {
            op: NavigationOp::Move {
                movement: Movement::Grapheme {
                    direction: Direction::Backward,
                },
                extend: false,
            },
        });
        editor.apply(Message::Navigation {
            op: NavigationOp::Move {
                movement: Movement::Grapheme {
                    direction: Direction::Forward,
                },
                extend: false,
            },
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "아".into() }],
        });
        assert_eq!(paint_at(&editor, p1, 1), vec![], "아 unstyled");
    }

    #[test]
    fn ime_rewrite_with_common_prefix_keeps_composing_char_paint() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("안녕") } } }
            selection: (p1, 2)
            pending_modifiers: [bold]
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "ㅎ".into() }],
        });
        assert_eq!(paint_at(&editor, p1, 2), vec![Modifier::Bold], "ㅎ bold");

        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 2, end: 4 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection { text: "녕".into() },
                FlatImeOp::ReplaceSelection { text: "하".into() },
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("안녕") text("하") [bold] } } }
            selection: (p1, 3, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn ime_ios_korean_full_log_replay_keeps_pending_paint() {
        let (state, p1) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        let msgs: Vec<Vec<FlatImeOp>> = vec![
            vec![FlatImeOp::ReplaceSelection { text: "ㅇ".into() }],
            vec![
                FlatImeOp::SetSelection { start: 0, end: 2 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection {
                    text: "\u{2028}".into(),
                },
                FlatImeOp::ReplaceSelection { text: "아".into() },
            ],
            vec![
                FlatImeOp::SetSelection { start: 0, end: 2 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection {
                    text: "\u{2028}".into(),
                },
                FlatImeOp::ReplaceSelection { text: "안".into() },
            ],
            vec![FlatImeOp::ReplaceSelection { text: "ㄴ".into() }],
            vec![
                FlatImeOp::SetSelection { start: 1, end: 3 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection { text: "안".into() },
                FlatImeOp::ReplaceSelection { text: "녀".into() },
            ],
            vec![
                FlatImeOp::SetSelection { start: 1, end: 3 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection { text: "안".into() },
                FlatImeOp::ReplaceSelection { text: "녕".into() },
            ],
        ];
        for ops in msgs {
            editor.apply(Message::TextInput { ops });
        }

        editor.apply(Message::Modifier {
            op: ModifierOp::Toggle {
                modifier_type: ModifierType::Bold,
            },
        });

        let msgs: Vec<Vec<FlatImeOp>> = vec![
            vec![FlatImeOp::ReplaceSelection { text: "ㅎ".into() }],
            vec![
                FlatImeOp::SetSelection { start: 2, end: 4 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection { text: "녕".into() },
                FlatImeOp::ReplaceSelection { text: "하".into() },
            ],
            vec![
                FlatImeOp::SetSelection { start: 2, end: 4 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection { text: "녕".into() },
                FlatImeOp::ReplaceSelection { text: "핫".into() },
            ],
            vec![
                FlatImeOp::SetSelection { start: 2, end: 4 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection { text: "녕".into() },
                FlatImeOp::ReplaceSelection {
                    text: "하세".into(),
                },
            ],
            vec![
                FlatImeOp::SetSelection { start: 3, end: 5 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection { text: "하".into() },
                FlatImeOp::ReplaceSelection { text: "셍".into() },
            ],
            vec![
                FlatImeOp::SetSelection { start: 3, end: 5 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection { text: "하".into() },
                FlatImeOp::ReplaceSelection {
                    text: "세요".into(),
                },
            ],
        ];
        for ops in msgs {
            editor.apply(Message::TextInput { ops });
        }

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("안녕") text("하세요") [bold] } } }
            selection: (p1, 5, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn ime_rewrite_deleting_composing_char_then_retype_keeps_paint() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("안녕") } } }
            selection: (p1, 2)
            pending_modifiers: [bold]
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "ㅎ".into() }],
        });
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 2, end: 4 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection { text: "녕".into() },
            ],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "ㅁ".into() }],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("안녕") text("ㅁ") [bold] } } }
            selection: (p1, 3, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn ime_rewrite_with_common_prefix_extending_syllable_keeps_paint() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("안녕") text("핫") [bold] } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 2, end: 4 },
                FlatImeOp::ReplaceSelection { text: "".into() },
                FlatImeOp::ReplaceSelection { text: "녕".into() },
                FlatImeOp::ReplaceSelection {
                    text: "하세".into(),
                },
            ],
        });
        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("안녕") text("하세") [bold] } } }
            selection: (p1, 4, <)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn ime_split_messages_recompose_sole_char_preserves_deleted_paint() {
        let (state, p1) = state! {
            doc { root { p1: paragraph {} } }
            selection: (p1, 0)
            pending_modifiers: [bold]
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "ㅇ".into() }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::DeleteSurroundingUtf16 {
                before: 1,
                after: 0,
            }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "아".into() }],
        });
        assert_eq!(paint_at(&editor, p1, 0), vec![Modifier::Bold], "아 bold");
    }

    #[test]
    fn flat_ime_backspace_across_open_token_with_empty_composition() {
        let (state, ..) = state! {
            doc { root {
                paragraph { text("가나") }
                p2: paragraph { text("다") }
            } }
            selection: (p2, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "라".into() }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "".into() }],
        });
        assert_eq!(editor.state().composition, None);
        editor.enqueue(Message::TextInput {
            ops: vec![FlatImeOp::DeleteSurroundingUtf16 {
                before: 1,
                after: 0,
            }],
        });
        editor
            .tick()
            .expect("backspace across open token must not fail");

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("가나다") } } }
            selection: (p1, 2, <)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn flat_ime_empty_compose_then_delete_across_token_single_batch() {
        let (state, ..) = state! {
            doc { root {
                paragraph { text("가나") }
                p2: paragraph { text("다") }
            } }
            selection: (p2, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "라".into() }],
        });
        editor.enqueue(Message::TextInput {
            ops: vec![
                FlatImeOp::Compose { text: "".into() },
                FlatImeOp::DeleteSurroundingUtf16 {
                    before: 1,
                    after: 0,
                },
            ],
        });
        editor
            .tick()
            .expect("batched empty compose + delete across token must not fail");

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("가나다") } } }
            selection: (p1, 2, <)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn flat_ime_delete_surrounding_excludes_composing_text() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("가") } } }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "나".into() }],
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 3 })
        );
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::DeleteSurroundingUtf16 {
                before: 1,
                after: 0,
            }],
        });

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("나") } } }
            selection: (p1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
    }

    #[test]
    fn flat_ime_delete_surrounding_after_composing_text() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("가다") } } }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "나".into() }],
        });
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 3 })
        );
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::DeleteSurroundingUtf16 {
                before: 0,
                after: 1,
            }],
        });

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("가나") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 3 })
        );
    }

    #[test]
    fn flat_ime_delete_surrounding_both_sides_of_composing_text_is_ignored() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("가다") } } }
            selection: (p1, 1)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "나".into() }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::DeleteSurroundingUtf16 {
                before: 1,
                after: 1,
            }],
        });

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("가나다") } } }
            selection: (p1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 3 })
        );
    }

    #[test]
    fn flat_ime_text_edit_with_set_selection_moves_cursor() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("abc") } } }
            selection: (p1, 3)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::ReplaceSelection { text: "d".into() },
                FlatImeOp::SetSelection { start: 1, end: 1 },
            ],
        });

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("abcd") } } }
            selection: (p1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_set_composition_before_token_edit_is_preserved() {
        let (state, ..) = state! {
            doc { root {
                paragraph { text("가나") }
                p2: paragraph { text("다") }
            } }
            selection: (p2, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.enqueue(Message::TextInput {
            ops: vec![
                FlatImeOp::DeleteSurroundingUtf16 {
                    before: 1,
                    after: 0,
                },
                FlatImeOp::SetComposition { start: 1, end: 2 },
            ],
        });
        editor
            .tick()
            .expect("composing region before the edit must not fail");

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("가나다") } } }
            selection: (p1, 2, <)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
    }

    #[test]
    fn flat_ime_set_composition_beyond_insert_in_token_edit_is_dropped() {
        let (state, ..) = state! {
            doc { root {
                paragraph { text("가나") }
                p2: paragraph { text("다라") }
            } }
            selection: (p2, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.enqueue(Message::TextInput {
            ops: vec![
                FlatImeOp::DeleteSurroundingUtf16 {
                    before: 1,
                    after: 0,
                },
                FlatImeOp::SetComposition { start: 5, end: 6 },
            ],
        });
        editor
            .tick()
            .expect("unmappable composing region must not fail");

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("가나다라") } } }
            selection: (p1, 2, <)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }
}
