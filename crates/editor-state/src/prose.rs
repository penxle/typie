use std::ops::Range;

use editor_crdt::Dot;
use editor_model::DocView;

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
    let mut state = EmitState::default();
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
            FlatSegment::Atom { .. } => {}
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
            && emitted
        {
            self.pending_boundary = true;
        }
    }

    fn emit_text(&mut self, flat_offset: usize, text: &str) {
        let n = text.chars().count();
        if n == 0 {
            return;
        }
        self.flush_pending_boundary(flat_offset);
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
}
