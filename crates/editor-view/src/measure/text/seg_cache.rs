use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Range;

use editor_crdt::Dot;
use editor_model::Alignment;
use hashbrown::HashMap;

use super::inline::TextRun;
use super::measure::MeasuredLine;
use super::resolve::ResolvedTextStyle;

/// Per-paragraph cache of measured hard-break segments. Hard-break segments never
/// reflow across each other, so editing one segment leaves every other segment's
/// shaped output identical except for a positional offset shift (later segments move
/// when an earlier one changes length). Lines are cached with segment-relative
/// offsets and rebased to the segment's current absolute start on reuse, so a
/// keystroke inside a huge multi-line paragraph re-shapes only the one edited
/// segment instead of all of them.
#[derive(Default)]
pub(crate) struct SegmentCache {
    entries: HashMap<(Dot, usize), Cached>,
}

struct Cached {
    hash: u64,
    /// Lines with offsets relative to the segment's start (start == 0).
    lines: Vec<MeasuredLine>,
}

impl SegmentCache {
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }

    /// Cached lines for `(para, index)` rebased to `seg_start`, if the content hash
    /// matches; `None` on miss.
    pub(crate) fn get(
        &self,
        para: Dot,
        index: usize,
        hash: u64,
        seg_start: usize,
    ) -> Option<Vec<MeasuredLine>> {
        let c = self.entries.get(&(para, index))?;
        if c.hash != hash {
            return None;
        }
        Some(
            c.lines
                .iter()
                .map(|l| shifted(l, seg_start as isize))
                .collect(),
        )
    }

    /// Store the freshly measured (absolute-offset) lines under `(para, index)`,
    /// normalized to segment-relative offsets.
    pub(crate) fn put(
        &mut self,
        para: Dot,
        index: usize,
        hash: u64,
        abs_lines: &[MeasuredLine],
        seg_start: usize,
    ) {
        let lines = abs_lines
            .iter()
            .map(|l| shifted(l, -(seg_start as isize)))
            .collect();
        self.entries.insert((para, index), Cached { hash, lines });
    }

    /// Drop `para`'s entries at indices `>= keep` (segments removed since last measure).
    pub(crate) fn prune(&mut self, para: Dot, keep: usize) {
        self.entries.retain(|(p, i), _| *p != para || *i < keep);
    }
}

fn shifted(line: &MeasuredLine, delta: isize) -> MeasuredLine {
    let sh = |x: usize| (x as isize + delta) as usize;
    let mut l = line.clone();
    if let Some(r) = &mut l.offset_range {
        *r = sh(r.start)..sh(r.end);
    }
    for g in &mut l.glyph_runs {
        g.offset_range = sh(g.offset_range.start)..sh(g.offset_range.end);
    }
    for t in &mut l.tab_gaps {
        t.offset_index = sh(t.offset_index);
    }
    l
}

fn hash_style<H: Hasher>(s: &ResolvedTextStyle, h: &mut H) {
    s.font_family.hash(h);
    s.font_weight.hash(h);
    s.font_size.to_bits().hash(h);
    s.letter_spacing.to_bits().hash(h);
    s.line_height.to_bits().hash(h);
}

/// Content hash capturing everything a segment's measured output depends on: its
/// text, the width/alignment/indent, the paragraph base style, and every inline
/// run's byte range (relative), font style, and effective modifiers (so a color or
/// decoration change — same glyphs, different output — still misses).
pub(crate) fn segment_hash(
    seg_text: &str,
    seg_off: &Range<usize>,
    runs: &[TextRun],
    width: f32,
    align: Alignment,
    indent: f32,
    base_style: &ResolvedTextStyle,
) -> u64 {
    let mut h = DefaultHasher::new();
    seg_text.hash(&mut h);
    width.to_bits().hash(&mut h);
    (align as u8).hash(&mut h);
    indent.to_bits().hash(&mut h);
    hash_style(base_style, &mut h);
    for r in runs
        .iter()
        .filter(|r| seg_off.start <= r.offset_range.start && r.offset_range.end <= seg_off.end)
    {
        (r.offset_range.start - seg_off.start).hash(&mut h);
        (r.offset_range.end - seg_off.start).hash(&mut h);
        hash_style(&r.style, &mut h);
        for (k, v) in r.effective.iter() {
            k.hash(&mut h);
            v.hash(&mut h);
        }
        h.write_u8(0xff);
    }
    h.finish()
}
