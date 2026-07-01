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
/// run's byte range (relative), font style, effective modifiers, and own modifiers
/// (so a color or decoration change — same glyphs, different output — still misses).
///
/// `own_modifiers` (value + `from_style` provenance) is hashed alongside `effective`
/// because the rendered link/color/decoration in the cached `MeasuredLine` are
/// resolved from `own_modifiers` via `resolve_link`/`resolve_colors`/`resolve_decoration`
/// (which branch on `from_style`), a distinction `effective` merges away — so two
/// runs with identical `effective` but different provenance must still miss.
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
        h.write_u8(0xfe);
        for (k, o) in r.own_modifiers.iter() {
            k.hash(&mut h);
            o.value.hash(&mut h);
            o.from_style.hash(&mut h);
        }
        h.write_u8(0xff);
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use editor_model::{Modifier, ModifierType, OwnModifier};

    use super::*;

    fn style() -> ResolvedTextStyle {
        ResolvedTextStyle {
            font_family: "sans".to_string(),
            font_weight: 400,
            font_size: 16.0,
            letter_spacing: 0.0,
            line_height: 1.6,
        }
    }

    // Two runs identical in every input the pre-fix hash covered (text, offsets,
    // font style, effective) but differing only in own-modifier provenance: a Link
    // supplied via a named style (from_style: true) resolves to no link/underline
    // and default color, while the same href set directly (from_style: false)
    // resolves to a clickable, underlined LINK_COLOR run. The cached MeasuredLine
    // carries that resolved output, so their segment hashes must differ.
    #[test]
    fn segment_hash_distinguishes_own_modifier_provenance() {
        let effective: BTreeMap<ModifierType, Modifier> = [(
            ModifierType::Link,
            Modifier::Link {
                href: "https://example.com".to_string(),
            },
        )]
        .into_iter()
        .collect();

        let mk_own = |from_style: bool| -> BTreeMap<ModifierType, OwnModifier> {
            [(
                ModifierType::Link,
                OwnModifier {
                    value: Modifier::Link {
                        href: "https://example.com".to_string(),
                    },
                    from_style,
                },
            )]
            .into_iter()
            .collect()
        };

        let hash_for = |own: &BTreeMap<ModifierType, OwnModifier>| {
            let runs = vec![TextRun {
                byte_range: 0..1,
                offset_range: 0..1,
                own_modifiers: own,
                effective: &effective,
                style: style(),
            }];
            segment_hash("a", &(0..1), &runs, 100.0, Alignment::Left, 0.0, &style())
        };

        assert_ne!(
            hash_for(&mk_own(true)),
            hash_for(&mk_own(false)),
            "runs differing only in own-modifier from_style must hash differently"
        );
    }
}
