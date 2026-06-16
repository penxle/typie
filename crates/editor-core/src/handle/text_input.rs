use std::ops::Range;
use std::sync::Arc;

use editor_commands::{self as commands, CommandError, CommandResult};
use editor_common::StrExt;
use editor_model::{Doc, Modifier};
use editor_state::{
    Composition, DocFlatExt, FLAT_CLOSE, FLAT_OPEN, FlatSegment, PendingModifier, ResolvedPosition,
    ResolvedPositionFlatExt, Selection, resolve_effective_modifiers_at,
};
use editor_transaction::Transaction;

use crate::editor::Editor;
use crate::error::EditorError;
use crate::message::*;

fn replace_text_range(tr: &mut Transaction, start: usize, end: usize, text: &str) -> CommandResult {
    let doc = tr.doc();
    let replacement_modifiers = uniform_text_modifiers_in_range(&doc, start, end);
    let resolve_selection_from_flat = || -> Result<Selection, CommandError> {
        let start_pos = ResolvedPosition::from_flat(&doc, start)
            .ok_or(CommandError::Corrupted("flat start unresolvable".into()))?;
        let end_pos = ResolvedPosition::from_flat(&doc, end)
            .ok_or(CommandError::Corrupted("flat end unresolvable".into()))?;
        Ok(Selection::new((&start_pos).into(), (&end_pos).into()))
    };
    let current_selection_flat_range = |selection: Selection| -> Option<(usize, usize)> {
        let anchor = selection.anchor.resolve(&doc)?.to_flat();
        let head = selection.head.resolve(&doc)?.to_flat();
        Some((anchor.min(head), anchor.max(head)))
    };
    let selection = tr
        .selection()
        .filter(|selection| current_selection_flat_range(*selection) == Some((start, end)))
        .map_or_else(|| resolve_selection_from_flat(), Ok)?;

    commands::chain!(
        tr,
        commands::set_selection(selection),
        commands::optional!(commands::ensure_paragraph()),
        commands::optional!(commands::delete_selection()),
        |tr| apply_replacement_modifiers(tr, text, replacement_modifiers.as_deref()),
        commands::when!(!text.is_empty(), commands::insert_text(text)),
    )
}

fn apply_replacement_modifiers(
    tr: &mut Transaction,
    text: &str,
    target_modifiers: Option<&[Modifier]>,
) -> CommandResult {
    if text.is_empty() {
        return Ok(true);
    }
    let Some(target_modifiers) = target_modifiers else {
        return Ok(true);
    };

    let Some(selection) = tr.selection() else {
        return Ok(true);
    };
    let base_modifiers = resolve_effective_modifiers_at(tr.state(), &selection.head);
    let pending = PendingModifier::diff(&base_modifiers, target_modifiers);
    tr.set_pending_modifiers(pending)?;
    Ok(true)
}

fn uniform_text_modifiers_in_range(doc: &Doc, start: usize, end: usize) -> Option<Vec<Modifier>> {
    if start >= end {
        return None;
    }

    let mut range_modifiers: Option<Vec<Modifier>> = None;
    for (_seg_start, seg) in doc.flat_segments_in_range(start..end) {
        let FlatSegment::Text { node_id, .. } = seg else {
            return None;
        };
        let node = doc.node(node_id)?;
        let mut modifiers: Vec<_> = node.modifiers().cloned().collect();
        modifiers.sort_by_key(|m| m.as_type());
        match &range_modifiers {
            Some(existing) if existing != &modifiers => return None,
            Some(_) => {}
            None => range_modifiers = Some(modifiers),
        }
    }

    range_modifiers
}

fn composition_range_valid(doc: &Doc, start: usize, end: usize) -> bool {
    if start > end || end > doc.flat_size() {
        return false;
    }
    for (seg_start, seg) in doc.flat_segments_in_range(start..end) {
        if seg_start < start {
            continue;
        }
        if matches!(
            seg,
            FlatSegment::Break { .. }
                | FlatSegment::Atom { .. }
                | FlatSegment::Open { .. }
                | FlatSegment::Close { .. }
        ) {
            return false;
        }
    }
    true
}

fn is_token(c: char) -> bool {
    c == FLAT_OPEN || c == FLAT_CLOSE
}

fn balanced_structural_body_range(doc: &Doc, start: usize, end: usize) -> Option<(usize, usize)> {
    let mut stack = Vec::new();
    let mut body: Option<(usize, usize)> = None;
    for (seg_start, seg) in doc.flat_segments_in_range(start..end) {
        match seg {
            FlatSegment::Open { node_id } => stack.push((node_id, seg_start)),
            FlatSegment::Close { node_id } => {
                let Some(&(open_id, open_start)) = stack.last() else {
                    continue;
                };
                if open_id != node_id {
                    continue;
                }
                stack.pop();
                let pair = (open_start, seg_start + seg.size());
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
        let doc = &state.doc;
        let total = doc.flat_size();

        let selection = state.selection?;
        let anchor = selection.anchor.resolve(doc)?.to_flat();
        let head = selection.head.resolve(doc)?.to_flat();
        let sel_start = anchor.min(head);
        let sel_end = anchor.max(head);

        let comp = state
            .composition
            .filter(|c| composition_range_valid(doc, c.start, c.end))
            .map(|c| (c.start, c.end));

        let window = ime_window(total, sel_start, sel_end, comp, ops);
        let text = FlatText {
            base: window.start,
            chars: doc.flat_chars(window),
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
        let doc = &state.doc;
        let flat_size = doc.flat_size();
        let selection = state.selection?;
        let anchor = selection.anchor.resolve(doc)?.to_flat();
        let head = selection.head.resolve(doc)?.to_flat();
        let comp = state
            .composition
            .filter(|c| composition_range_valid(doc, c.start, c.end))
            .map(|c| (c.start, c.end));
        Some(FlatImeState {
            text: FlatText::whole(doc.flat_chars(0..flat_size)),
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
                let cursor = self.sel_start.min(self.text.len());
                let del_start = cursor.saturating_sub(*before);
                let del_end = (cursor + after).min(self.text.len());
                let change = (del_start < del_end).then_some(FlatImeTextChange {
                    replace_start: del_start,
                    replace_end: del_end,
                    insert: Vec::new(),
                });
                if del_end > cursor {
                    self.text.splice(cursor..del_end, std::iter::empty());
                }
                if del_start < cursor {
                    self.text.splice(del_start..cursor, std::iter::empty());
                }
                self.sel_start = del_start;
                self.sel_end = del_start;
                change
            }
            FlatImeOp::DeleteSurroundingUtf16 { before, after } => {
                let cursor = self.sel_start.min(self.text.len());
                let before_chars = utf16_units_to_chars(self.text.iter_rev_to(cursor), *before);
                let after_chars = utf16_units_to_chars(self.text.iter_from(cursor), *after);
                let del_start = cursor - before_chars;
                let del_end = cursor + after_chars;
                let change = (del_start < del_end).then_some(FlatImeTextChange {
                    replace_start: del_start,
                    replace_end: del_end,
                    insert: Vec::new(),
                });
                if del_end > cursor {
                    self.text.splice(cursor..del_end, std::iter::empty());
                }
                if del_start < cursor {
                    self.text.splice(del_start..cursor, std::iter::empty());
                }
                self.sel_start = del_start;
                self.sel_end = del_start;
                change
            }
            FlatImeOp::SetComposition { start, end } => {
                self.comp = Some((*start, *end));
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
            Some(change) => Some(change.without_reinserted_boundary_tokens(&initial_text)),
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
    let doc = tr.doc();
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
    doc: &Doc,
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
    let commit_as_is = ops.iter().any(|op| matches!(op, FlatImeOp::CommitAsIs));

    (|| -> Result<(), EditorError> {
        let initial = match FlatImeState::from_editor(editor, &ops) {
            Some(s) => s,
            None => return Ok(()),
        };

        let reduced = initial.clone().reduce_flat_ime_ops(&ops);

        // The cursor-windowed snapshot must reduce identically to one built over
        // the whole document; a mismatch means the window failed to cover an
        // access. Checked in debug across the IME test suite.
        #[cfg(debug_assertions)]
        if let Some(whole) = FlatImeState::from_editor_whole(editor) {
            let whole_reduced = whole.reduce_flat_ime_ops(&ops);
            debug_assert_eq!(
                reduced.text_change, whole_reduced.text_change,
                "windowed IME reduction diverged from whole-document reduction"
            );
        }

        if reduced.text_change.is_none() {
            return Ok(());
        }

        let started_at_gap = editor
            .state
            .selection
            .as_ref()
            .and_then(|s| s.resolve(&editor.state.doc))
            .and_then(|rs| rs.as_gap_cursor())
            .is_some();
        let (initial, reduced) = if initial.text != reduced.state.text && started_at_gap {
            let before_materialize = initial.clone();
            editor.transact(|tr| {
                tr.set_composition(None)?;
                commands::materialize_gap_paragraph(tr)?;
                Ok(())
            })?;
            let initial = match FlatImeState::from_editor_whole(editor) {
                Some(s) => s,
                None => return Ok(()),
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
            return Ok(());
        };
        let del = text_change.deleted_from(&initial.text);

        let del_opens = count_opens(del);
        let del_closes = count_closes(del);
        let ins_opens = count_opens(&text_change.insert);
        let ins_closes = count_closes(&text_change.insert);

        let tokens_increased = ins_opens > del_opens || ins_closes > del_closes;
        if tokens_increased {
            return Ok(());
        }

        if text_change.inserts_token() {
            return Ok(());
        }

        let delta = analyze_delta(
            &editor.state().doc,
            &initial.text,
            text_change.replace_start,
            text_change.replace_end,
            &text_change.insert,
            initial.sel_start,
        );
        let should_insert_after_unit_selection = !delta.ins_text.is_empty()
            && text_change.replace_start == initial.sel_start
            && text_change.replace_end == initial.sel_end
            && editor
                .state()
                .selection
                .as_ref()
                .is_some_and(|selection| selection.is_unit_node_selection(&editor.state().doc));
        let has_text_delta = !del.is_empty() || !text_change.insert.is_empty();
        let has_structural_seams = delta.start_tokens > 0 || delta.end_tokens > 0;

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
                        let deletes_body = delta.replace_start != delta.replace_end;
                        let replacement_modifiers = if !delta.ins_text.is_empty() {
                            let doc = tr.doc();
                            uniform_text_modifiers_in_range(
                                &doc,
                                delta.replace_start,
                                delta.replace_end,
                            )
                        } else {
                            None
                        };

                        if deletes_body {
                            replace_text_range(tr, delta.replace_start, delta.replace_end, "")?;
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

                        if !delta.ins_text.is_empty() {
                            apply_replacement_modifiers(
                                tr,
                                &delta.ins_text,
                                replacement_modifiers.as_deref(),
                            )?;
                            commands::insert_text(tr, &delta.ins_text)?;
                        }
                        composition_text_start = Some(text_change.replace_start);
                    } else {
                        replace_text_range(
                            tr,
                            delta.replace_start,
                            delta.replace_end,
                            &delta.ins_text,
                        )?;
                    }
                }

                let composition = match (result.comp, composition_text_start) {
                    (Some((start, end)), Some(text_start)) => {
                        let invalid = || EditorError::General {
                            msg: "invariant violated: flat IME composition remap failed".into(),
                        };
                        let inserted_len = delta.ins_text.char_count();
                        let relative_start = start.checked_sub(text_start).ok_or_else(invalid)?;
                        let relative_end = end.checked_sub(text_start).ok_or_else(invalid)?;
                        if relative_start > relative_end || relative_end > inserted_len {
                            return Err(invalid());
                        }

                        let doc = tr.doc();
                        let inserted_start = tr
                            .selection()
                            .and_then(|selection| selection.head.resolve(&doc))
                            .and_then(|head| head.to_flat().checked_sub(inserted_len))
                            .ok_or_else(invalid)?;

                        Some(Composition {
                            start: inserted_start + relative_start,
                            end: inserted_start + relative_end,
                        })
                    }
                    (Some((start, end)), None) => Some(Composition { start, end }),
                    (None, _) => None,
                };
                let composition = composition.filter(|composition| {
                    composition_range_valid(&tr.doc(), composition.start, composition.end)
                });

                if composition.is_some() || tr.composition().is_some() {
                    tr.set_composition(composition)?;
                }

                Ok(())
            })?;
        }

        Ok(())
    })()?;

    if commit_as_is {
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
    use editor_model::Modifier;
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
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
                    paragraph { t1: text("abc") }
                    paragraph { text("def") }
                }
            }
            selection: (t1, 0)
        };
        // O(p1)=0, abc=1..4, C(p1)=4, O(p2)=5, def=6..9, C(p2)=9
        assert!(composition_range_valid(&state.doc, 1, 4));
        assert!(!composition_range_valid(&state.doc, 1, 5)); // crosses C(p1)
        assert!(composition_range_valid(&state.doc, 6, 9));
    }

    #[test]
    fn composition_range_valid_rejects_out_of_range() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("ab") } } }
            selection: (t1, 0)
        };
        // O=0, ab=1..3, C=3; flat_size=4
        assert!(composition_range_valid(&state.doc, 1, 3));
        assert!(!composition_range_valid(&state.doc, 1, 4));
        assert!(!composition_range_valid(&state.doc, 2, 1)); // start > end
    }

    #[test]
    fn composition_range_valid_accepts_empty_range() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        // O=0, hello=1..6, C=6; empty ranges are valid anchors for composition
        assert!(composition_range_valid(&state.doc, 1, 1));
        assert!(composition_range_valid(&state.doc, 4, 4));
        assert!(composition_range_valid(&state.doc, 6, 6));
    }

    #[test]
    fn composition_range_valid_rejects_atom() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("a") image {} text("b") }
                }
            }
            selection: (t1, 0)
        };
        // O=0, a=1, img=2, b=3, C=4
        assert!(composition_range_valid(&state.doc, 1, 2)); // "a" only
        assert!(!composition_range_valid(&state.doc, 1, 3)); // crosses image atom
        assert!(!composition_range_valid(&state.doc, 2, 3)); // starts at image atom
        assert!(composition_range_valid(&state.doc, 3, 4)); // "b" only
    }

    #[test]
    fn composition_range_valid_rejects_open_token() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        // O=0, text=1..6, C=6
        assert!(!composition_range_valid(&state.doc, 0, 2)); // includes Open
        assert!(!composition_range_valid(&state.doc, 5, 7)); // includes Close
        assert!(composition_range_valid(&state.doc, 1, 6)); // text only
    }

    #[test]
    fn composition_range_valid_rejects_nested_tokens() {
        let (state, ..) = state! {
            doc { root { blockquote { paragraph { t1: text("hi") } } } }
            selection: (t1, 0)
        };
        // O(bq)=0, O(p)=1, h=2, i=3, C(p)=4, C(bq)=5
        assert!(composition_range_valid(&state.doc, 2, 4)); // "hi"
        assert!(!composition_range_valid(&state.doc, 1, 4)); // includes Open(p)
        assert!(!composition_range_valid(&state.doc, 2, 5)); // includes Close(p)
    }

    #[test]
    fn set_region_stores_valid_range() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
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
    fn set_region_rejects_cross_block() {
        let (state, ..) = state! {
            doc {
                root {
                    paragraph { t1: text("abc") }
                    paragraph { t2: text("def") }
                }
            }
            selection: (t1, 0)
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
            doc { root { paragraph { t1: text("hello world") } } }
            selection: (t1, 0)
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
                    paragraph { t1: text("abc") }
                    paragraph { text("def") }
                }
            }
            selection: (t1, 0)
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
                    paragraph { t1: text("a") image {} text("b") }
                }
            }
            selection: (t1, 0)
        };
        let mut editor = Editor::new_test(state);
        // Range 1..3 crosses the image atom
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 1, end: 3 }],
        });
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn commit_as_is_clears_composition() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn clear_composition_keeps_composing_text() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn clear_composition_without_composition_is_noop() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ClearComposition],
        });
        assert_eq!(editor.state().composition, None);
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn update_no_composition_inserts_at_cursor() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "X".into() }],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("heXllo") } } }
            selection: (t1, 3)
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
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
            doc { root { paragraph { t1: text("hXYlo") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 2, end: 4 })
        );
    }

    #[test]
    fn update_with_composition_replaces_region() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 4)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetComposition { start: 2, end: 5 }],
        });
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "XYZ".into() }],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hXYZo") } } }
            selection: (t1, 4)
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
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 0)
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
            doc { root { paragraph { t1: text("Xhi") } } }
            selection: (t1, 1)
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 4)
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
            doc { root { paragraph { t1: text("hYo") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn commit_no_composition_inserts_at_cursor() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 2)
        };
        let mut editor = Editor::new_test(state);
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::ReplaceSelection { text: "!".into() },
                FlatImeOp::CommitAsIs,
            ],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hi!") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn update_with_cjk_unicode_text() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("") } } }
            selection: (t1, 0)
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
            doc { root { paragraph { t1: text("안녕") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn update_preserves_existing_composition_modifiers() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("하") } } }
            selection: (t1, 1)
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
                    paragraph {
                        t1: text("하")
                        t2: text("하") [bold]
                    }
                }
            }
            selection: (t2, 1)
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
                        chunks: vec![vec![0x0000, 0xFFFF]],
                    },
                    editor_resource::FontWeight {
                        value: 700,
                        hash: "pretendard_700".into(),
                        chunks: vec![vec![0x0000, 0xFFFF]],
                    },
                ],
            }]);
        let resource = Arc::new(Mutex::new(resource));
        let (state, ..) = state! {
            doc {
                root [font_weight(400), font_family("Pretendard".to_string())] {
                    paragraph { t1: text("하") }
                }
            }
            selection: (t1, 1)
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
                    paragraph {
                        t1: text("하")
                        t2: text("하") [font_weight(700)]
                    }
                }
            }
            selection: (t2, 1)
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
                    paragraph {
                        t1: text("A") [bold]
                        t2: text("ㅎ")
                    }
                }
            }
            selection: (t2, 1)
        };
        state.composition = Some(Composition { start: 2, end: 3 });
        let mut editor = Editor::new_test(state);

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "하".into() }],
        });

        let (expected, ..) = state! {
            doc {
                root {
                    paragraph {
                        t1: text("A") [bold]
                        t2: text("하")
                    }
                }
            }
            selection: (t2, 1)
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
                    paragraph {
                        t1: text("하")
                        t2: text("ㅎ") [italic]
                    }
                }
            }
            selection: (t2, 1)
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
                    paragraph {
                        t1: text("하")
                        t2: text("하") [italic]
                    }
                }
            }
            selection: (t2, 1)
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
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 4)
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
            doc { root { paragraph { t1: text("ho") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn commit_with_cjk_unicode_text() {
        let (state, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 2)
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
            doc { root { paragraph { t1: text("hi안녕") } } }
            selection: (t1, 4)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn commit_stale_composition_falls_back_to_cursor() {
        let (mut state, ..) = state! {
            doc { root { paragraph { t1: text("hi") } } }
            selection: (t1, 1)
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
            doc { root { paragraph { t1: text("hXi") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn retroactive_composition_across_formatting_boundary() {
        // "안"[bold] + "녕" (two text nodes in same paragraph, different modifiers)
        let (state, ..) = state! {
            doc { root { paragraph {
                text("안") [bold]
                t2: text("녕")
            }}}
            selection: (t2, 1)  // cursor after "녕"
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

        assert_eq!(editor.state().doc.flat_text(1..4), "안녕하");
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

    fn paragraph_run_texts_and_styles(editor: &Editor) -> Vec<(String, Option<String>)> {
        let paragraph = editor
            .state()
            .doc
            .root()
            .expect("root exists")
            .children()
            .next()
            .expect("paragraph exists");
        paragraph
            .children()
            .filter_map(|child| match child.node() {
                editor_model::Node::Text(text) => {
                    Some((text.text.to_string(), child.entry().style.get().clone()))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn flat_ime_text_replacement() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 5)
        };
        let editor = apply_flat_ime_ops(s, vec![FlatImeOp::ReplaceSelection { text: "!".into() }]);
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hello!") } } }
            selection: (t1, 6)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_repeated_text_insertion_uses_cursor_position() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("aaaa") } } }
            selection: (t1, 0)
        };
        let editor = apply_flat_ime_ops(s, vec![FlatImeOp::ReplaceSelection { text: "a".into() }]);
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("aaaaa") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_repeated_insert_preserves_style_run_affinity() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("ac") } } }
            selection: (t1, 1)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::Style {
            op: StyleOp::Define {
                style_id: "green".into(),
                name: "Green".into(),
                modifiers: vec![Modifier::TextColor {
                    value: "#00ff00".into(),
                }],
            },
        });
        editor.apply(Message::Style {
            op: StyleOp::ApplyToSelection {
                style_id: "green".into(),
            },
        });

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "f".into() }],
        });
        assert_eq!(
            paragraph_run_texts_and_styles(&editor),
            vec![
                ("a".into(), None),
                ("f".into(), Some("green".into())),
                ("c".into(), None),
            ]
        );
        let current_flat = editor
            .state()
            .selection
            .and_then(|selection| selection.head.resolve(&editor.state().doc))
            .map(|pos| pos.to_flat())
            .expect("selection head resolves to flat");
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection {
                    start: current_flat,
                    end: current_flat,
                },
                FlatImeOp::ReplaceSelection { text: "f".into() },
            ],
        });

        assert_eq!(
            paragraph_run_texts_and_styles(&editor),
            vec![
                ("a".into(), None),
                ("ff".into(), Some("green".into())),
                ("c".into(), None),
            ]
        );
    }

    #[test]
    fn flat_ime_repeated_text_middle_insertion_uses_cursor_position() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("aaaa") } } }
            selection: (t1, 2)
        };
        let editor = apply_flat_ime_ops(s, vec![FlatImeOp::ReplaceSelection { text: "a".into() }]);
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("aaaaa") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_all_with_same_text_places_cursor_after_inserted_text() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("a") } } }
            selection: (t1, 1)
        };
        let editor = apply_flat_ime_ops(
            s,
            vec![
                FlatImeOp::SetSelection { start: 0, end: 3 },
                FlatImeOp::ReplaceSelection { text: "a".into() },
            ],
        );
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("a") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_nested_full_selection_removes_structure() {
        let (s, ..) = state! {
            doc { root { blockquote { paragraph { t1: text("a") } } } }
            selection: (t1, 1)
        };
        let editor = apply_flat_ime_ops(
            s,
            vec![
                FlatImeOp::SetSelection { start: 0, end: 5 },
                FlatImeOp::ReplaceSelection { text: "a".into() },
            ],
        );
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("a") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_disjoint_text_edits_are_ignored() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("abcdef") } } }
            selection: (t1, 0)
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
            doc { root { paragraph { t1: text("abcdef") } } }
            selection: (t1, 0)
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
        let flat_text = editor
            .state()
            .doc
            .flat_text(0..editor.state().doc.flat_size());
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
            doc { root { paragraph { t1: text("ㅁㅁㅁㅁ") } } }
            selection: (t1, 2)
        };
        let editor = apply_flat_ime_ops(s, vec![FlatImeOp::Compose { text: "ㅁ".into() }]);
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("ㅁㅁㅁㅁㅁ") } } }
            selection: (t1, 3)
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
            doc { root { paragraph { t1: text("ㅁㅁㅁㅁ") } } }
            selection: (t1, 2)
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
            doc { root { paragraph { t1: text("ㅁㅁㅁㅁㅁ") } } }
            selection: (t1, 3)
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
            doc { root { paragraph { t1: text("ㅁㅁㅁㅁ") } } }
            selection: (t1, 2)
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
            doc { root { paragraph { t1: text("ㅁㅁㅁㅁㅁ") } } }
            selection: (t1, 3)
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
            doc { root { paragraph { t1: text("a") } } }
            selection: (t1, 1)
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
                paragraph { t1: text("b") }
                paragraph { text("c") }
            } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn flat_ime_replace_mixed_seam_and_table_body_removes_body() {
        let (s, ..) = state! {
            doc { root {
                paragraph { t1: text("aa") }
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
                paragraph { t2: text("cc") }
            } }
            selection: (t2, 0, <) -> (t1, 2, >)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "d".into() }],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("aadcc") } } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn flat_ime_delete_mixed_seam_and_table_body_removes_body() {
        let (s, ..) = state! {
            doc { root {
                paragraph { t1: text("aa") }
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
                paragraph { t2: text("cc") }
            } }
            selection: (t2, 0, <) -> (t1, 2, >)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "".into() }],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("aacc") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn flat_ime_compose_mixed_seam_and_table_body_sets_composition() {
        let (s, ..) = state! {
            doc { root {
                paragraph { t1: text("aa") }
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
                paragraph { t2: text("cc") }
            } }
            selection: (t2, 0, <) -> (t1, 2, >)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("aaㅎcc") } } }
            selection: (t1, 3)
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
                blockquote {
                    blockquote {
                        paragraph { t1: text("aa") }
                    }
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
                paragraph { t2: text("cc") }
            } }
            selection: (t2, 0, <) -> (t1, 2, >)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        let (expected, ..) = state! {
            doc { root {
                blockquote {
                    blockquote {
                        paragraph { t1: text("aaㅎcc") }
                    }
                }
                paragraph {}
            } }
            selection: (t1, 3)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 5, end: 6 })
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
                paragraph { t1: text("ㅎ") }
                paragraph { text("c") }
            } }
            selection: (t1, 1)
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
            doc { root { paragraph { t1: text("ㅎ") } } }
            selection: (t1, 1)
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
                blockquote { paragraph { t1: text("나") } }
                paragraph { text("after") }
            } }
            selection: (t1, 1)
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
        let end = editor.state().doc.flat_size();

        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetComposition { start: 0, end },
                FlatImeOp::Compose { text: "ㅁ".into() },
            ],
        });

        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("ㅁ") } } }
            selection: (t1, 1)
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
            doc { root { paragraph { t1: text("!ㅇ") } } }
            selection: (t1, 2)
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
            doc { root { paragraph { t1: text("!아") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_no_change_is_noop() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::SetSelection { start: 4, end: 4 }],
        });
        assert_eq!(editor.state().doc.flat_text(1..6), "hello");
    }

    #[test]
    fn flat_ime_pua_reinsert_filtered() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("ab") } } }
            selection: (t1, 2)
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
            doc { root { paragraph { t1: text("ab") } } }
            selection: (t1, 2)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_delete_surrounding() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::DeleteSurrounding {
                before: 2,
                after: 0,
            }],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("hlo") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_compose_sets_composition() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("hello") } } }
            selection: (t1, 3)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "X".into() }],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("helXlo") } } }
            selection: (t1, 4)
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
            doc { root { paragraph { t1: text("paragraph1") } paragraph { t2: text("") } } }
            selection: (t2, 0)
        };
        let mut editor = editor_with_resource(s);
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::DeleteSurrounding {
                before: 1,
                after: 0,
            }],
        });
        let state = editor.state();
        let flat = state.doc.flat_text(0..state.doc.flat_size());
        assert!(
            !flat.contains("\u{2028}\u{2029}\u{2029}"),
            "empty paragraph should have been removed"
        );
    }

    #[test]
    fn flat_ime_join_paragraph_backward_cursor_at_end() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("A") } p2: paragraph {} } }
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
            doc { root { paragraph { t1: text("A") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_empty_paragraph_backspace_removes_paragraph() {
        let (s, ..) = state! {
            doc { root { paragraph { t1: text("hello") } p2: paragraph {} } }
            selection: (p2, 0)
        };
        let mut editor = editor_with_resource(s);
        let original_size = editor.state().doc.flat_size();
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection { start: 7, end: 8 },
                FlatImeOp::ReplaceSelection { text: "".into() },
            ],
        });
        let new_size = editor.state().doc.flat_size();
        assert!(
            new_size < original_size,
            "empty paragraph should be removed: new_size={new_size} original={original_size}"
        );
    }

    #[test]
    fn flat_ime_input_context_has_tokens() {
        let (s, ..) = state! {
            doc { root { blockquote { paragraph { t1: text("") } } paragraph {} } }
            selection: (t1, 0)
        };
        let editor = Editor::new_test(s);
        let ctx = editor.ime(100, 100).unwrap();
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
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 1)
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
                    paragraph { t1: text("hello") }
                    paragraph { t2: text("world") }
                }
                paragraph {}
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_bulk_delete_close_open_pair() {
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
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
                    paragraph { t1: text("hello") }
                    paragraph { t2: text("world") }
                }
                paragraph {}
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_bulk_delete_two_boundaries() {
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
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
                blockquote { paragraph { t1: text("helloworld") } }
                paragraph {}
            } }
            selection: (t1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_bulk_delete_single_open_token_through_first_text() {
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
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
                    paragraph { t1: text("hello") }
                    paragraph { t2: text("orld") }
                }
                paragraph {}
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_bulk_delete_close_open_pair_through_first_text() {
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
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
                    paragraph { t1: text("hello") }
                    paragraph { t2: text("orld") }
                }
                paragraph {}
            } }
            selection: (t2, 0)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_bulk_delete_two_boundaries_through_first_text() {
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
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
                blockquote { paragraph { t1: text("helloorld") } }
                paragraph {}
            } }
            selection: (t1, 5)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_single_open_token_through_first_text() {
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
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
                    paragraph { t1: text("hello") }
                    paragraph { t2: text("aorld") }
                }
                paragraph {}
            } }
            selection: (t2, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_close_open_pair_through_first_text() {
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
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
                    paragraph { t1: text("hello") }
                    paragraph { t2: text("aorld") }
                }
                paragraph {}
            } }
            selection: (t2, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_two_boundaries_through_first_text() {
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
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
                blockquote { paragraph { t1: text("helloaorld") } }
                paragraph {}
            } }
            selection: (t1, 6)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_single_open_token_inserts_at_boundary() {
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
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
                    paragraph { t1: text("hello") }
                    paragraph { t2: text("aworld") }
                }
                paragraph {}
            } }
            selection: (t2, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_close_open_pair_inserts_at_boundary() {
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
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
                    paragraph { t1: text("hello") }
                    paragraph { t2: text("aworld") }
                }
                paragraph {}
            } }
            selection: (t2, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_replace_two_boundaries_inserts_at_join() {
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 0)
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
                blockquote { paragraph { t1: text("helloaworld") } }
                paragraph {}
            } }
            selection: (t1, 6)
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
                paragraph { t1: text("b") }
                paragraph { text("c") }
            } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }

    #[test]
    fn flat_ime_bulk_delete_text_across_structure() {
        let (s, ..) = state! {
            doc { root {
                blockquote { paragraph { t1: text("hello") } }
                paragraph { t2: text("world") }
            } }
            selection: (t2, 3)
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
                blockquote { paragraph { t1: text("hld") } }
                paragraph {}
            } }
            selection: (t1, 1)
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
            doc { root { paragraph { t1: text("ㅎ") } image paragraph { text("b") } } }
            selection: (t1, 1)
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
                paragraph { t1: text("ㅎ") }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (t1, 1)
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
            doc { root { paragraph { t1: text("a") } image paragraph { text("b") } } }
            selection: (t1, 1)
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
                paragraph { t1: text("X") }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (t1, 1)
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
            doc { root { paragraph { t1: text("ㅎ") } image paragraph { text("b") } } }
            selection: (t1, 1)
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
                paragraph { t1: text("ㅎ") }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (t1, 1)
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
            doc { root { paragraph { t1: text("안") } image paragraph { text("b") } } }
            selection: (t1, 1)
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
    fn update_with_stale_composition_at_leading_gap_materializes_and_inserts() {
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
            Some(Composition { start: 0, end: 0 }),
            "precondition: SetRegion at gap leaves stale empty composition"
        );
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("ㅎ") } image paragraph { text("b") } } }
            selection: (t1, 1)
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
            doc { root { paragraph { t1: text("안") } image paragraph { text("b") } } }
            selection: (t1, 1)
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
                paragraph { t1: text("X") }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (t1, 1)
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
    fn commit_with_stale_composition_at_leading_gap_materializes_and_inserts() {
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
            Some(Composition { start: 0, end: 0 }),
            "precondition: SetRegion at gap leaves stale empty composition"
        );
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::Compose { text: "안".into() },
                FlatImeOp::CommitAsIs,
            ],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("안") } image paragraph { text("b") } } }
            selection: (t1, 1)
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
            doc { root { paragraph { t1: text("ㅎ") } image paragraph { text("b") } } }
            selection: (t1, 1)
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
                paragraph { t1: text("ㅎ") }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (t1, 1)
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
        // SetComposition reduces snapshot.comp to Some((0,0)); handle_flat_ime
        // applies that via a separate transact at the end since
        // result.comp != initial.comp. Pin this explicitly so a refactor
        // can't silently regress the "state-only Flat op survives the gap
        // gate" property.
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 0, end: 0 })
        );
    }

    #[test]
    fn flat_ime_compose_with_stale_composition_at_leading_gap_materializes_and_composes() {
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
            Some(Composition { start: 0, end: 0 }),
            "precondition: SetRegion at gap leaves stale empty composition"
        );
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㅎ".into() }],
        });
        let (expected, ..) = state! {
            doc { root { paragraph { t1: text("ㅎ") } image paragraph { text("b") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
    }

    #[test]
    fn clear_with_stale_composition_at_leading_gap_preserves_unit() {
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
            Some(Composition { start: 0, end: 0 }),
            "precondition: SetRegion at gap leaves stale empty composition"
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
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 0, end: 0 })
        );
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
            doc { root { paragraph { t1: text("X") [bold] } image paragraph { text("b") } } }
            selection: (t1, 1)
        };
        assert_state_eq!(editor.state(), &expected);
    }
}
