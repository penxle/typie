use editor_macros::ffi;
use editor_state::{Position, ResolvedPosition, ResolvedPositionFlatExt, Selection};
use serde::{Deserialize, Serialize};

use crate::{Editor, EditorError, Revision};

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImeRange {
    pub start: usize,
    pub end: usize,
}

#[ffi]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ime {
    pub text: String,
    pub window_start: usize,
    pub selection: ImeRange,
    pub composing: Option<ImeRange>,
}

#[derive(Clone, Copy)]
pub(crate) struct ImeWindowAnchor {
    start: usize,
    last_sel_start: usize,
}

impl Editor {
    pub(crate) fn set_composition_target_ranges(&mut self, ranges: Vec<ImeRange>) {
        let mut targets: Vec<ImeRange> = Vec::new();
        if self.focused
            && let Some(composition) = self.state.composition
        {
            let mut ranges: Vec<_> = ranges
                .into_iter()
                .filter_map(|range| {
                    let start = range.start.max(composition.start);
                    let end = range.end.min(composition.end);
                    (start < end).then_some(ImeRange { start, end })
                })
                .collect();
            ranges.sort_unstable_by_key(|range| range.start);
            for range in ranges {
                if let Some(previous) = targets.last_mut()
                    && range.start <= previous.end
                {
                    previous.end = previous.end.max(range.end);
                } else {
                    targets.push(range);
                }
            }
            if targets.len() == 1
                && targets[0].start == composition.start
                && targets[0].end == composition.end
            {
                targets.clear();
            }
        }
        if self.composition_target_ranges != targets {
            self.composition_target_ranges = targets;
            self.invalidate_render();
        }
    }

    pub(crate) fn composition_target_rects(&self) -> Vec<editor_view::PageRect> {
        let doc = self.state.view();
        self.composition_target_ranges
            .iter()
            .flat_map(|range| {
                let Some(from) = ResolvedPosition::from_flat(&doc, range.start) else {
                    return Vec::new();
                };
                let Some(to) = ResolvedPosition::from_flat(&doc, range.end) else {
                    return Vec::new();
                };
                let selection = Selection::new(Position::from(&from), Position::from(&to));
                selection.resolve(&doc).map_or_else(Vec::new, |selection| {
                    self.view
                        .selection_text_rects(&selection)
                        .iter()
                        .map(|rect| rect.without_meta())
                        .collect()
                })
            })
            .collect()
    }

    pub fn ime(
        &mut self,
        before_limit: usize,
        after_limit: usize,
    ) -> Result<Option<Ime>, EditorError> {
        let state = &self.state;
        let doc = state.view();
        let doc_size = editor_state::flat_size(&doc);

        let Some(sel) = state.selection else {
            return Ok(None);
        };
        let anchor_flat = sel
            .anchor
            .resolve(&doc)
            .ok_or_else(|| EditorError::General {
                msg: "invariant violated: state.selection.anchor must resolve against state.doc"
                    .into(),
            })?
            .to_flat();
        let head_flat = sel
            .head
            .resolve(&doc)
            .ok_or_else(|| EditorError::General {
                msg: "invariant violated: state.selection.head must resolve against state.doc"
                    .into(),
            })?
            .to_flat();
        let (sel_start, sel_end) = (anchor_flat.min(head_flat), anchor_flat.max(head_flat));
        // Text input materializes a gap before replaying IME ops, so the host
        // must receive the gap's real offset instead of an adjacent text offset.
        let collapsed_insertable =
            sel_start == sel_end && is_flat_offset_insertable(&doc, sel_start);
        let at_gap_cursor = !collapsed_insertable
            && sel.is_collapsed()
            && editor_state::gap_cursor_at(&sel.head, &doc).is_some();
        // ime() runs on every cursor move; the collapsed caret is the dominant
        // case, so avoid walking the same offset twice.
        let (sel_start, sel_end) = if collapsed_insertable || at_gap_cursor {
            (sel_start, sel_end)
        } else if sel_start == sel_end {
            let s = nearest_insertable_flat(&doc, doc_size, sel_start);
            (s, s)
        } else {
            let s = nearest_insertable_flat(&doc, doc_size, sel_start);
            let e = nearest_insertable_flat(&doc, doc_size, sel_end);
            (s.min(e), s.max(e))
        };

        // Keep the window start anchored across keyboard edits: IMEs predict the
        // window's next content by applying their own edits to it (iOS resets
        // Hangul composition whenever the pulled value diverges from that
        // prediction), and a window re-centered around the selection diverges on
        // every length-changing edit. Re-anchoring is itself such a divergence,
        // so outside the [limit, 2×limit] before-context band it is deferred
        // while a composition may be active — visible composition state, or a
        // selection delta shaped like a composition edit ({-1, 0, +1}; Hangul
        // composition is invisible to the editor) — and happens at moments that
        // already diverge for the IME (jumps, Enter, rewrites). Two violations
        // re-anchor unconditionally: the window no longer contains the cursor,
        // and the hard cap on retained before-context. A live composition is
        // still included below even when the cursor moves beyond that cap.
        let natural_start = sel_start.saturating_sub(before_limit);
        let window_start = match self.ime_window_anchor {
            Some(anchor) => {
                let kept = anchor.start;
                let within_band =
                    kept <= natural_start && sel_start - kept <= before_limit.saturating_mul(2);
                if within_band {
                    kept
                } else {
                    let must_re_anchor =
                        kept > sel_start || sel_start - kept > before_limit.saturating_mul(16);
                    let composition_may_be_active = state.composition.is_some()
                        || sel_start.abs_diff(anchor.last_sel_start) <= 1;
                    if !must_re_anchor && composition_may_be_active {
                        kept
                    } else {
                        natural_start
                    }
                }
            }
            None => natural_start,
        };
        // A native cursor move can leave composition behind until the IME ends
        // it. Keep both ranges in the window so the host can report real offsets.
        let window_start = state
            .composition
            .map_or(window_start, |c| window_start.min(c.start));
        // Keep trailing context relative to the whole composition, not its
        // internal caret. Otherwise moving inside marked text changes the
        // window's text and looks like an external edit to the IME.
        let context_end = state.composition.map_or(sel_end, |c| sel_end.max(c.end));
        let window_end = context_end.saturating_add(after_limit).min(doc_size);

        let text = editor_state::flat_text(&doc, window_start..window_end);
        let composing = state.composition.map(|c| ImeRange {
            start: c.start,
            end: c.end,
        });
        self.ime_window_anchor = Some(ImeWindowAnchor {
            start: window_start,
            last_sel_start: sel_start,
        });

        Ok(Some(Ime {
            text,
            window_start,
            selection: ImeRange {
                start: sel_start,
                end: sel_end,
            },
            composing,
        }))
    }

    /// First visual line of a flat character range, or the caret for an empty range.
    /// The revision must match the layout whose text offsets the caller is using.
    pub fn first_rect_for_range(
        &self,
        revision: Revision,
        start: usize,
        end: usize,
    ) -> Option<editor_view::PageRect> {
        if revision != self.revision() || start > end {
            return None;
        }
        let state = self.state();
        let doc = state.view();
        let from = Position::from(&ResolvedPosition::from_flat(&doc, start)?);
        let to = Position::from(&ResolvedPosition::from_flat(&doc, end)?);
        if start == end {
            // A flat offset alone cannot distinguish the two sides of a soft wrap.
            let from = state
                .selection
                .map(|selection| selection.head)
                .filter(|head| head.resolve(&doc).is_some_and(|p| p.to_flat() == start))
                .unwrap_or(from);
            let cursor = self.view().cursor_metrics(state, &from)?;
            return Some(editor_view::PageRect::new(cursor.page_idx, cursor.caret));
        }
        let selection = Selection::new(from, to).resolve(&doc)?;
        self.view()
            .selection_rects(&selection)
            .first()
            .map(|rect| rect.without_meta())
    }
}

fn is_flat_offset_insertable(doc: &editor_model::DocView, offset: usize) -> bool {
    ResolvedPosition::from_flat(doc, offset)
        .and_then(|rp| doc.node(rp.node()))
        .is_some_and(|n| n.spec().is_textblock())
}

fn nearest_insertable_flat(doc: &editor_model::DocView, total: usize, offset: usize) -> usize {
    if is_flat_offset_insertable(doc, offset) {
        return offset;
    }
    for radius in 1..=total {
        if let Some(before) = offset.checked_sub(radius)
            && is_flat_offset_insertable(doc, before)
        {
            return before;
        }
        let after = offset + radius;
        if after <= total && is_flat_offset_insertable(doc, after) {
            return after;
        }
    }
    offset
}

#[cfg(test)]
pub(crate) fn nearest_insertable_flat_probe(
    doc: &editor_model::DocView,
    total: usize,
    offset: usize,
) -> usize {
    nearest_insertable_flat(doc, total, offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FlatImeOp, Message, SelectionOp};
    use editor_macros::state;
    use editor_state::Composition;

    #[test]
    fn input_context_without_selection_returns_none() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: none
        };
        let mut editor = Editor::new_test(state);
        assert!(editor.ime(usize::MAX, usize::MAX).unwrap().is_none());
    }

    #[test]
    fn input_context_full_window_returns_whole_doc() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello") } } }
            selection: (p1, 2)
        };
        let mut editor = Editor::new_test(state);
        let ctx = editor.ime(usize::MAX, usize::MAX).unwrap().unwrap();
        // flat: O(p)=0, "hello"=1..6, C(p)=6 → flat_size=7
        // (p1,2) → flat 3; window covers full doc [0,7)
        assert_eq!(ctx.text, "\u{2028}hello\u{2029}");
        assert_eq!(ctx.window_start, 0);
        assert_eq!(ctx.selection.start, 3);
        assert_eq!(ctx.selection.end, 3);
        assert!(ctx.composing.is_none());
    }

    #[test]
    fn input_context_limited_window_clamps() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 6)
        };
        let mut editor = Editor::new_test(state);
        let ctx = editor.ime(3, 3).unwrap().unwrap();
        // flat: O(p)=0, "hello world"=1..12, C(p)=12 → flat_size=13
        // (p1,6) → flat 7; window [7-3, 7+3) = [4, 10) → "lo wor"
        assert_eq!(ctx.window_start, 4);
        assert_eq!(ctx.text, "lo wor");
        assert_eq!(ctx.selection.start, 7);
        assert_eq!(ctx.selection.end, 7);
    }

    #[test]
    fn input_context_with_non_collapsed_selection() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 2) -> (p1, 8)
        };
        let mut editor = Editor::new_test(state);
        let ctx = editor.ime(usize::MAX, usize::MAX).unwrap().unwrap();
        // flat: O(p)=0, "hello world"=1..12, C(p)=12 → flat_size=13
        // (p1,2)→flat 3, (p1,8)→flat 9; window covers full doc [0,13)
        assert_eq!(ctx.text, "\u{2028}hello world\u{2029}");
        assert_eq!(ctx.selection.start, 3);
        assert_eq!(ctx.selection.end, 9);
    }

    #[test]
    fn input_context_preserves_leading_gap_cursor_offset() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(state);

        let ctx = editor.ime(usize::MAX, usize::MAX).unwrap().unwrap();

        assert_eq!(ctx.selection.start, 0);
        assert_eq!(ctx.selection.end, 0);
    }

    #[test]
    fn input_context_preserves_between_monolithic_gap_cursor_offset() {
        let (state, ..) = state! {
            doc { r: root {
                fold { fold_title { text("A") } fold_content { paragraph { text("x") } } }
                fold { fold_title { text("B") } fold_content { paragraph { text("y") } } }
                paragraph {}
            } }
            selection: (r, 1)
        };
        let mut editor = Editor::new_test(state);

        let ctx = editor.ime(usize::MAX, usize::MAX).unwrap().unwrap();

        assert_eq!(ctx.selection.start, 10);
        assert_eq!(ctx.selection.end, 10);
    }

    #[test]
    fn web_text_input_from_reported_leading_gap_materializes_and_inserts() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(state);
        let ctx = editor.ime(usize::MAX, usize::MAX).unwrap().unwrap();

        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetSelection {
                    start: ctx.selection.start,
                    end: ctx.selection.end,
                },
                FlatImeOp::ReplaceSelection { text: "a".into() },
            ],
        });

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("a") } image paragraph { text("b") } } }
            selection: (p1, 1)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
        assert_eq!(editor.state().composition, None);
    }

    #[test]
    fn web_composition_from_reported_leading_gap_materializes_and_composes() {
        let (state, ..) = state! {
            doc { r: root { image paragraph { text("b") } } }
            selection: (r, 0, <)
        };
        let mut editor = Editor::new_test(state);
        let ctx = editor.ime(usize::MAX, usize::MAX).unwrap().unwrap();

        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::SetComposition {
                    start: ctx.selection.start,
                    end: ctx.selection.end,
                },
                FlatImeOp::Compose { text: "ㅎ".into() },
            ],
        });

        let (expected, ..) = state! {
            doc { root { p1: paragraph { text("ㅎ") } image paragraph { text("b") } } }
            selection: (p1, 1)
        };
        editor_state::assert_state_eq!(editor.state(), &expected);
        assert_eq!(
            editor.state().composition,
            Some(Composition { start: 1, end: 2 })
        );
    }

    #[test]
    fn ime_keeps_composition_visible_after_cursor_moves_until_ime_finishes_it() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("abcdefghijklmnopqrstuvwxyz") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        editor.ime(1, 1).unwrap().unwrap();
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "한".into() }],
        });
        let before = editor.ime(1, 1).unwrap().unwrap();
        editor.view.layout(&editor.state);

        // Android moves the cursor first, then lets the IME finish its preedit.
        // The new cursor is beyond the window's normal re-anchoring hard cap.
        editor.apply(Message::Selection {
            op: SelectionOp::SetAt {
                page: 0,
                x: 9999.0,
                y: 5.0,
            },
        });
        let moved = editor.ime(1, 1).unwrap().unwrap();
        assert_eq!(moved.selection, ImeRange { start: 28, end: 28 });
        assert_eq!(moved.composing, before.composing);
        assert!(moved.window_start <= 1, "{moved:?}");
        assert!(
            moved.text.contains("한abcdefghijklmnopqrstuvwxyz"),
            "{moved:?}"
        );

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::CommitAsIs],
        });
        assert!(editor.state().composition.is_none());
        assert_eq!(
            editor.ime(1, 1).unwrap().unwrap().selection,
            moved.selection
        );
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "ㄱ".into() }],
        });
        let after = editor.ime(64, 64).unwrap().unwrap();
        let doc = editor.state().view();
        assert_eq!(
            editor_state::flat_text(&doc, 0..editor_state::flat_size(&doc)),
            "\u{2028}한abcdefghijklmnopqrstuvwxyzㄱ\u{2029}"
        );
        assert_eq!(after.composing, Some(ImeRange { start: 28, end: 29 }));
        assert_eq!(editor.state().selection.unwrap().head.node, p1);
    }

    #[test]
    fn ime_window_text_stays_unchanged_when_moving_within_composition() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 6)
        };
        let mut editor = Editor::new_test(state);
        editor.ime(3, 3).unwrap().unwrap();
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose {
                text: "にほんご".into(),
            }],
        });
        let before = editor.ime(3, 3).unwrap().unwrap();
        assert_eq!(before.text, "lo にほんごwor");

        // UIKit resends the same marked text with a different relative caret.
        for caret in [10, 9, 10, 11] {
            editor.apply(Message::TextInput {
                ops: vec![
                    FlatImeOp::Compose {
                        text: "にほんご".into(),
                    },
                    FlatImeOp::SetSelection {
                        start: caret,
                        end: caret,
                    },
                ],
            });
            let after = editor.ime(3, 3).unwrap().unwrap();
            assert_eq!(after.text, before.text, "caret at {caret}");
            assert_eq!(after.window_start, before.window_start);
            assert_eq!(
                after.selection,
                ImeRange {
                    start: caret,
                    end: caret
                }
            );
            assert_eq!(after.composing, before.composing);
        }
    }

    #[test]
    fn ime_window_follows_composition_replacements_with_an_internal_caret() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 6)
        };
        let mut editor = Editor::new_test(state);
        editor.ime(3, 3).unwrap().unwrap();

        // Growing and shrinking the preedit must change only the preedit, even
        // when its caret is not at the end. Flat offsets count Unicode scalars.
        for (text, caret, end, expected) in [
            ("にほんご", 8, 11, "lo にほんごwor"),
            ("語😀", 8, 9, "lo 語😀wor"),
            ("にほんごx", 9, 12, "lo にほんごxwor"),
        ] {
            editor.apply(Message::TextInput {
                ops: vec![
                    FlatImeOp::Compose { text: text.into() },
                    FlatImeOp::SetSelection {
                        start: caret,
                        end: caret,
                    },
                ],
            });
            let ctx = editor.ime(3, 3).unwrap().unwrap();
            assert_eq!(ctx.window_start, 4);
            assert_eq!(ctx.text, expected);
            assert_eq!(
                ctx.selection,
                ImeRange {
                    start: caret,
                    end: caret
                }
            );
            assert_eq!(ctx.composing, Some(ImeRange { start: 7, end }));
        }
    }

    #[test]
    fn ime_window_includes_composition_without_trailing_context() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 6)
        };
        let mut editor = Editor::new_test(state);
        editor.ime(3, 0).unwrap().unwrap();
        editor.apply(Message::TextInput {
            ops: vec![
                FlatImeOp::Compose {
                    text: "abcd".into(),
                },
                FlatImeOp::SetSelection { start: 7, end: 7 },
            ],
        });

        let ctx = editor.ime(3, 0).unwrap().unwrap();
        assert_eq!(ctx.window_start, 4);
        assert_eq!(ctx.text, "lo abcd");
        assert_eq!(ctx.selection, ImeRange { start: 7, end: 7 });
        assert_eq!(ctx.composing, Some(ImeRange { start: 7, end: 11 }));
    }

    #[test]
    fn ime_window_start_stays_anchored_while_typing() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 6)
        };
        let mut editor = Editor::new_test(state);
        // flat: O(p)=0, "hello world"=1..12, C(p)=12; (p1,6) → flat 7
        let before = editor.ime(3, 3).unwrap().unwrap();
        assert_eq!(before.window_start, 4);
        assert_eq!(before.text, "lo wor");

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "X".into() }],
        });

        // The window start must not re-center: the pulled window must equal the
        // previous window with the edit applied verbatim (the IME's prediction).
        let after = editor.ime(3, 3).unwrap().unwrap();
        assert_eq!(after.window_start, 4);
        let cursor = before.selection.start - before.window_start;
        let mut predicted = before.text.clone();
        predicted.insert(
            predicted
                .char_indices()
                .nth(cursor)
                .map_or(predicted.len(), |(i, _)| i),
            'X',
        );
        assert_eq!(after.text, predicted);
        assert_eq!(after.selection.start, before.selection.start + 1);
    }

    #[test]
    fn ime_window_defers_re_anchor_past_soft_cap_while_typing() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 6)
        };
        let mut editor = Editor::new_test(state);
        assert_eq!(editor.ime(3, 3).unwrap().unwrap().window_start, 4);

        // Contiguous typing (+1 deltas) may be mid-composition, so crossing the
        // soft cap (2×limit) must not re-anchor: the window keeps matching the
        // IME's prediction of it.
        for i in 0..6 {
            editor.apply(Message::TextInput {
                ops: vec![FlatImeOp::ReplaceSelection { text: "X".into() }],
            });
            let ctx = editor.ime(3, 3).unwrap().unwrap();
            assert_eq!(ctx.window_start, 4, "kept through insert #{}", i + 1);
        }
        assert_eq!(editor.ime(3, 3).unwrap().unwrap().selection.start, 13);
    }

    #[test]
    fn ime_window_re_anchors_on_jump_after_soft_cap() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 6)
        };
        let mut editor = Editor::new_test(state);
        assert_eq!(editor.ime(3, 3).unwrap().unwrap().window_start, 4);

        // Push the before-context past the soft cap with contiguous typing;
        // production pulls the window every tick, so pull per keystroke.
        for _ in 0..4 {
            editor.apply(Message::TextInput {
                ops: vec![FlatImeOp::ReplaceSelection { text: "X".into() }],
            });
            assert_eq!(editor.ime(3, 3).unwrap().unwrap().window_start, 4);
        }

        // A selection jump landing outside the band already diverges from the
        // IME's prediction (it resets composition anyway), so the deferred
        // re-anchor happens here. "hello XXXXworld": (p1,14) → flat 15.
        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: Selection::collapsed(Position::new(p1, 14)),
            },
        });
        let ctx = editor.ime(3, 3).unwrap().unwrap();
        assert_eq!(ctx.selection.start, 15);
        assert_eq!(ctx.window_start, 12);
    }

    #[test]
    fn ime_window_defers_backward_re_anchor_while_composing() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 6)
        };
        let mut editor = Editor::new_test(state);
        // Fresh anchor at the band floor: before-context is exactly the limit.
        assert_eq!(editor.ime(3, 3).unwrap().unwrap().window_start, 4);

        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::Compose { text: "X".into() }],
        });
        assert_eq!(editor.ime(3, 3).unwrap().unwrap().window_start, 4);

        // A single batch deleting two chars before the composition moves the
        // selection by -2 (not a contiguous delta), dropping the before-context
        // below the limit — but the active composition defers the re-anchor.
        editor.apply(Message::TextInput {
            ops: vec![
                // Surrounding deletes count from the selection, so move to
                // the composition start before deleting the preceding text.
                FlatImeOp::SetSelection { start: 7, end: 7 },
                FlatImeOp::DeleteSurrounding {
                    before: 1,
                    after: 0,
                },
                FlatImeOp::DeleteSurrounding {
                    before: 1,
                    after: 0,
                },
                FlatImeOp::SetSelection { start: 6, end: 6 },
            ],
        });
        let ctx = editor.ime(3, 3).unwrap().unwrap();
        assert!(editor.state().composition.is_some());
        assert_eq!(ctx.selection.start, 6);
        assert_eq!(ctx.window_start, 4);
    }

    #[test]
    fn ime_window_hard_cap_forces_re_anchor_during_contiguous_typing() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 6)
        };
        let mut editor = Editor::new_test(state);
        assert_eq!(editor.ime(3, 3).unwrap().unwrap().window_start, 4);

        // Contiguous typing defers up to the hard cap (16×limit = 48): kept
        // while sel - anchor ≤ 48, i.e. through sel = 52 (45 inserts from 7).
        for _ in 0..45 {
            editor.apply(Message::TextInput {
                ops: vec![FlatImeOp::ReplaceSelection { text: "X".into() }],
            });
            assert_eq!(editor.ime(3, 3).unwrap().unwrap().window_start, 4);
        }
        // The 46th insert pushes the context to 49 > 48 and forces the bounded
        // re-anchor even though the delta still looks like typing.
        editor.apply(Message::TextInput {
            ops: vec![FlatImeOp::ReplaceSelection { text: "X".into() }],
        });
        let ctx = editor.ime(3, 3).unwrap().unwrap();
        assert_eq!(ctx.selection.start, 53);
        assert_eq!(ctx.window_start, 50);
    }

    #[test]
    fn ime_window_re_anchors_on_backward_selection_jump() {
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("hello world") } } }
            selection: (p1, 6)
        };
        let mut editor = Editor::new_test(state);
        assert_eq!(editor.ime(3, 3).unwrap().unwrap().window_start, 4);

        editor.apply(Message::Selection {
            op: SelectionOp::Set {
                selection: Selection::collapsed(Position::new(p1, 1)),
            },
        });

        // flat 2; the anchor (4) no longer covers the requested before-context.
        let ctx = editor.ime(3, 3).unwrap().unwrap();
        assert_eq!(ctx.selection.start, 2);
        assert_eq!(ctx.window_start, 0);
    }

    #[test]
    fn input_context_empty_blockquote_has_tokens() {
        let (state, _p1) = state! {
            doc { root { blockquote { p1: paragraph { text("") } } paragraph {} } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        let ctx = editor.ime(100, 100).unwrap().unwrap();
        assert!(
            !ctx.text.is_empty(),
            "IME buffer must not be empty for empty blockquote"
        );

        let cursor_in_window = ctx.selection.start - ctx.window_start;
        assert!(cursor_in_window > 0, "cursor should be after Open tokens");
    }

    #[test]
    fn ime_geometry_preserves_the_current_caret_side_of_a_soft_wrap() {
        use editor_state::{Affinity, Position, ResolvedPositionFlatExt, Selection};
        let (state, p1) = state! {
            doc { root { p1: paragraph { text("abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz") } } }
            selection: (p1, 0)
        };
        let mut editor = Editor::new_test(state);
        for offset in 1..104 {
            let upstream = Position {
                node: p1,
                offset,
                affinity: Affinity::Upstream,
            };
            let downstream = Position {
                affinity: Affinity::Downstream,
                ..upstream
            };
            let up = editor
                .view()
                .cursor_metrics(editor.state(), &upstream)
                .unwrap();
            let down = editor
                .view()
                .cursor_metrics(editor.state(), &downstream)
                .unwrap();
            if up.caret.y == down.caret.y {
                continue;
            }
            editor.apply(Message::Selection {
                op: crate::SelectionOp::Set {
                    selection: Selection::collapsed(upstream),
                },
            });
            let doc = editor.state().view();
            let flat = upstream.resolve(&doc).unwrap().to_flat();
            let actual = editor
                .first_rect_for_range(editor.revision(), flat, flat)
                .unwrap();
            assert_eq!(actual.rect, up.caret);
            return;
        }
        panic!("fixture must contain a soft wrap");
    }

    #[test]
    fn ime_geometry_uses_requested_range_instead_of_current_caret() {
        let (state, _p1) = state! {
            doc { root { p1: paragraph { text("にほんご") } } }
            selection: (p1, 4)
        };
        let mut editor = Editor::new_test(state);
        let revision = editor.revision();
        let range = editor.first_rect_for_range(revision, 1, 4).unwrap();
        let start = editor.first_rect_for_range(revision, 1, 1).unwrap();
        let end = editor.first_rect_for_range(revision, 4, 4).unwrap();
        assert_eq!(range.rect.x, start.rect.x);
        assert_eq!(range.rect.right(), end.rect.x);
        assert!(range.rect.height > 1.0);
        assert!(start.rect.x < end.rect.x);
        editor.apply(Message::Selection {
            op: crate::SelectionOp::SetFlat { start: 2, end: 2 },
        });
        assert_eq!(
            editor.first_rect_for_range(editor.revision(), 1, 4),
            Some(range)
        );
        assert!(editor.first_rect_for_range(revision, 1, 4).is_none());
    }

    #[test]
    fn ime_geometry_returns_only_first_line_and_rejects_invalid_offsets() {
        let (state, _p1, _p2) = state! {
            doc { root {
                p1: paragraph { text("hello") }
                p2: paragraph { text("world") }
            } }
            selection: (p2, 3)
        };
        let editor = Editor::new_test(state);
        let revision = editor.revision();
        let first_line = editor.first_rect_for_range(revision, 2, 6).unwrap();
        let spanning = editor.first_rect_for_range(revision, 2, 10).unwrap();
        assert_eq!(spanning.page_idx, first_line.page_idx);
        assert_eq!(spanning.rect.x, first_line.rect.x);
        assert_eq!(spanning.rect.y, first_line.rect.y);
        assert_eq!(spanning.rect.height, first_line.rect.height);
        // A selection spanning a paragraph also includes its paragraph-break mark.
        assert!(spanning.rect.width >= first_line.rect.width);
        assert!(editor.first_rect_for_range(revision, 6, 2).is_none());
        assert!(
            editor
                .first_rect_for_range(revision, 2, usize::MAX)
                .is_none()
        );
        assert!(
            editor
                .first_rect_for_range(Revision { value: u64::MAX }, 2, 6)
                .is_none()
        );
    }
}
