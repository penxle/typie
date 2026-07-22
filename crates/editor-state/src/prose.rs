use std::ops::Range;

use editor_crdt::Dot;
use editor_model::{DocView, NodeType};

use crate::{
    Affinity, FlatSegment, Position, ResolvedPosition, ResolvedPositionFlatExt, Selection,
    flat_segments,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProseRun {
    plain_range: Range<usize>,
    flat_start: usize,
}

/// User-visible "prose" projection of a document: the plain text (textblock
/// content joined by blank-line boundaries, hard breaks as `\n`, atoms skipped)
/// together with a mapping from prose char offsets back to flat positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProseText {
    text: String,
    plain_len: usize,
    runs: Vec<ProseRun>,
}

#[derive(Debug, Clone, Copy)]
enum Bias {
    Left,
    Right,
}

impl ProseText {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn to_flat_range(&self, range: Range<usize>) -> Option<Range<usize>> {
        if range.start > range.end {
            return None;
        }
        if range.end > self.plain_len {
            return None;
        }
        if self.runs.is_empty() {
            return None;
        }

        if range.start == range.end {
            let p = self.locate(range.start, Bias::Right)?;
            return Some(p..p);
        }

        let start = self.locate(range.start, Bias::Right)?;
        let end = self.locate(range.end, Bias::Left)?;
        Some(start..end)
    }

    pub fn to_selection(&self, view: &DocView, range: Range<usize>) -> Option<Selection> {
        let collapsed = range.start == range.end;
        let flat = self.to_flat_range(range)?;
        let anchor = ResolvedPosition::from_flat(view, flat.start)?;
        let head = ResolvedPosition::from_flat(view, flat.end)?;

        Some(Selection {
            anchor: Position {
                node: anchor.node(),
                offset: anchor.offset(),
                affinity: Affinity::Downstream,
            },
            head: Position {
                node: head.node(),
                offset: head.offset(),
                affinity: if collapsed {
                    Affinity::Downstream
                } else {
                    Affinity::Upstream
                },
            },
        })
    }

    fn locate(&self, plain_pos: usize, bias: Bias) -> Option<usize> {
        let idx = self
            .runs
            .partition_point(|r| r.plain_range.end <= plain_pos);

        if idx == self.runs.len() {
            let last = self.runs.last()?;
            if plain_pos == last.plain_range.end {
                return Some(last.flat_start + last.plain_range.len());
            }
            return None;
        }

        let run = &self.runs[idx];

        if plain_pos == run.plain_range.start && idx > 0 {
            let prev = &self.runs[idx - 1];
            return Some(match bias {
                Bias::Left => prev.flat_start + prev.plain_range.len(),
                Bias::Right => run.flat_start,
            });
        }

        Some(run.flat_start + (plain_pos - run.plain_range.start))
    }
}

pub fn prose(view: &DocView) -> ProseText {
    build(view, false)
}

pub fn prose_annotated(view: &DocView) -> ProseText {
    build(view, true)
}

fn build(view: &DocView, annotated: bool) -> ProseText {
    let mut state = EmitState {
        annotated,
        ..Default::default()
    };
    let mut flat_offset = 0usize;
    for segment in flat_segments(view) {
        let size = match &segment {
            FlatSegment::Text { leaves, .. } => leaves.len(),
            _ => 1,
        };
        state.handle(view, flat_offset, segment);
        flat_offset += size;
    }
    state.finish()
}

#[derive(Default)]
struct EmitState {
    text: String,
    runs: Vec<ProseRun>,
    plain_len: usize,
    pending_boundary: bool,
    block_emitted_stack: Vec<bool>,
    last_text_end_flat: usize,
    annotated: bool,
    pending_empty_blocks: usize,
}

impl EmitState {
    fn handle(&mut self, view: &DocView, flat_offset: usize, segment: FlatSegment) {
        match segment {
            FlatSegment::Text { leaves, .. } => {
                let text: String = leaves
                    .iter()
                    .filter_map(|&d| view.leaf(d).and_then(|l| l.as_char()))
                    .collect();
                self.emit_text(flat_offset, &text);
            }
            FlatSegment::Break { .. } => self.emit_break(flat_offset),
            FlatSegment::Atom { leaf } => {
                if self.annotated
                    && let Some(lv) = view.leaf(leaf)
                    && lv.node_type() == NodeType::HorizontalRule
                {
                    self.emit_divider(flat_offset);
                }
            }
            FlatSegment::Open { block } => self.handle_open(view, block),
            FlatSegment::Close { block } => self.handle_close(view, block),
        }
    }

    fn handle_open(&mut self, view: &DocView, block: Dot) {
        if let Some(nv) = view.node(block)
            && nv.spec().is_textblock()
        {
            self.block_emitted_stack.push(false);
        }
    }

    fn handle_close(&mut self, view: &DocView, block: Dot) {
        if let Some(nv) = view.node(block)
            && nv.spec().is_textblock()
            && let Some(emitted) = self.block_emitted_stack.pop()
        {
            if emitted {
                self.pending_boundary = true;
            } else if self.annotated {
                self.pending_empty_blocks += 1;
            }
        }
    }

    fn emit_text(&mut self, flat_offset: usize, text: &str) {
        let n = text.chars().count();
        if n == 0 {
            return;
        }
        self.flush_pending_boundary(flat_offset);
        self.flush_pending_empties(flat_offset);
        let p = self.plain_len;
        self.runs.push(ProseRun {
            plain_range: p..p + n,
            flat_start: flat_offset,
        });
        self.text.push_str(text);
        self.plain_len += n;
        self.last_text_end_flat = flat_offset + n;
        self.mark_block_emitted();
    }

    fn emit_break(&mut self, flat_offset: usize) {
        self.flush_pending_boundary(flat_offset);
        self.flush_pending_empties(flat_offset);
        let p = self.plain_len;
        self.runs.push(ProseRun {
            plain_range: p..p + 1,
            flat_start: flat_offset,
        });
        self.text.push('\n');
        self.plain_len += 1;
        self.last_text_end_flat = flat_offset + 1;
        self.mark_block_emitted();
    }

    fn flush_pending_boundary(&mut self, current_flat: usize) {
        if !self.pending_boundary {
            return;
        }
        let p = self.plain_len;
        self.runs.push(ProseRun {
            plain_range: p..p + 1,
            flat_start: self.last_text_end_flat,
        });
        debug_assert!(current_flat >= 1);
        self.runs.push(ProseRun {
            plain_range: (p + 1)..(p + 2),
            flat_start: current_flat - 1,
        });
        self.text.push_str("\n\n");
        self.plain_len += 2;
        self.pending_boundary = false;
    }

    fn push_synthetic(&mut self, ch: char, current_flat: usize) {
        if ch == '\n' {
            if self.text.is_empty() {
                return; // 문서 시작의 선행 개행 억제
            }
            let trailing = self.text.chars().rev().take_while(|&c| c == '\n').count();
            if trailing >= 4 {
                return; // 클램프: 연속 개행 최대 4 (= 빈 줄 3)
            }
        }
        let p = self.plain_len;
        let flat = if self.text.ends_with('\n') {
            current_flat.saturating_sub(1)
        } else {
            self.last_text_end_flat
        };
        self.runs.push(ProseRun {
            plain_range: p..p + 1,
            flat_start: flat,
        });
        self.text.push(ch);
        self.plain_len += 1;
    }

    fn flush_pending_empties(&mut self, current_flat: usize) {
        for _ in 0..self.pending_empty_blocks {
            self.push_synthetic('\n', current_flat);
        }
        self.pending_empty_blocks = 0;
    }

    fn emit_divider(&mut self, flat_offset: usize) {
        self.flush_pending_boundary(flat_offset);
        self.flush_pending_empties(flat_offset);
        for ch in "***".chars() {
            self.push_synthetic(ch, flat_offset);
        }
        self.pending_boundary = true;
        self.last_text_end_flat = flat_offset + 1;
    }

    fn mark_block_emitted(&mut self) {
        if let Some(top) = self.block_emitted_stack.last_mut() {
            *top = true;
        }
    }

    fn finish(self) -> ProseText {
        ProseText {
            text: self.text,
            plain_len: self.plain_len,
            runs: self.runs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ResolvedPositionFlatExt;
    use editor_macros::state;

    #[test]
    fn maps_multibyte_prose_range_to_semantic_selection() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("한😀글") } } }
            selection: (p1, 0)
        };
        let view = state.view();
        let prose = prose(&view);

        let selection = prose.to_selection(&view, 0..2).expect("mapped selection");
        let resolved = selection.resolve(&view).expect("resolved selection");
        let flat = resolved.from().to_flat()..resolved.to().to_flat();

        assert_eq!(crate::flat_text(&view, flat), "한😀");
    }

    #[test]
    fn annotated_emits_divider_marker_between_paragraphs() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("가나") } hr: horizontal_rule p2: paragraph { text("다라") } } }
            selection: (p1, 0)
        };
        let view = state.view();
        assert_eq!(prose_annotated(&view).text(), "가나\n\n***\n\n다라");
        assert_eq!(prose(&view).text(), "가나\n\n다라");
    }

    #[test]
    fn annotated_preserves_empty_paragraphs_with_clamp() {
        // 빈 문단 1개 → 빈 줄 2개(개행 3), 2개 → 빈 줄 3개(개행 4), 3개 이상 → 클램프(개행 4 유지)
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("가") } e1: paragraph p2: paragraph { text("나") } } }
            selection: (p1, 0)
        };
        let view = state.view();
        assert_eq!(prose_annotated(&view).text(), "가\n\n\n나");
        assert_eq!(prose(&view).text(), "가\n\n나");
    }

    #[test]
    fn annotated_clamps_consecutive_blank_lines() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("가") } e1: paragraph e2: paragraph e3: paragraph e4: paragraph p2: paragraph { text("나") } } }
            selection: (p1, 0)
        };
        let view = state.view();
        assert_eq!(prose_annotated(&view).text(), "가\n\n\n\n나");
    }

    #[test]
    fn annotated_divider_at_document_start_has_no_leading_newlines() {
        let (state, ..) = state! {
            doc { root { hr: horizontal_rule p1: paragraph { text("가") } } }
            selection: (p1, 0)
        };
        let view = state.view();
        assert_eq!(prose_annotated(&view).text(), "***\n\n가");
    }

    #[test]
    fn annotated_range_over_marker_clamps_to_adjacent_text() {
        let (state, ..) = state! {
            doc { root { p1: paragraph { text("가나") } hr: horizontal_rule p2: paragraph { text("다라") } } }
            selection: (p1, 0)
        };
        let view = state.view();
        let prose = prose_annotated(&view);
        // "가나\n\n***\n\n다라"에서 "나"~"다"를 걸치는 range(1..10)가 유효 선택으로 클램프되는지
        let selection = prose.to_selection(&view, 1..10).expect("mapped selection");
        let resolved = selection.resolve(&view).expect("resolved");
        assert!(resolved.from().to_flat() < resolved.to().to_flat());
    }

    proptest::proptest! {
        #[test]
        fn annotated_ranges_never_panic_and_plain_is_subsequence(
            start in 0usize..64, len in 0usize..64,
        ) {
            let (state, ..) = state! {
                doc { root { p1: paragraph { text("가나다") } hr: horizontal_rule e1: paragraph p2: paragraph { text("라마바") } } }
                selection: (p1, 0)
            };
            let view = state.view();
            let annotated = prose_annotated(&view);
            let plain = prose(&view);

            // ① 임의 range에서 panic 없이 Some(유효 flat) 또는 None
            let end = (start + len).min(annotated.text().chars().count());
            let s = start.min(end);
            let _ = annotated.to_flat_range(s..end);

            // ② plain 텍스트는 annotated 텍스트의 부분수열
            let mut it = annotated.text().chars();
            proptest::prop_assert!(plain.text().chars().all(|c| it.any(|a| a == c)));
        }
    }
}
