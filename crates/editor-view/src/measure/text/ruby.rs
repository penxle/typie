pub(crate) const RUBY_FONT_SIZE_RATIO: f32 = 0.5;
pub(crate) const RUBY_FONT_SIZE_MIN_PX: f32 = 12.0;
pub(crate) const RUBY_GAP: f32 = 2.0;

pub fn ruby_extra_top(baseline: f32, ascent: f32, ruby_annotations: &[RubyAnnotation]) -> f32 {
    if ruby_annotations.is_empty() {
        return 0.0;
    }
    let max_ruby_ascent = ruby_annotations
        .iter()
        .map(|r| r.ascent)
        .fold(0.0, f32::max);
    let max_ruby_descent = ruby_annotations
        .iter()
        .map(|r| r.descent)
        .fold(0.0, f32::max);
    let required_top = max_ruby_ascent + max_ruby_descent + RUBY_GAP;
    let available_top = (baseline - ascent).max(0.0);
    (required_top - available_top).max(0.0)
}

use std::borrow::Cow;

use editor_resource::{Resource, TextBrush};
use parley::style::{
    FontFamily, FontFamilyName, FontFeatures, FontWeight as ParleyFontWeight, LineHeight, TextStyle,
};
use parley::{OverflowWrap, WordBreak};

use crate::glyph_run::{Glyph, RubyAnnotation, RubyGlyphRun, Synthesis};

use super::extract::ExtractedLine;
use super::inline::RubyGroup;
use super::style_run::resolve_cluster_family_weight;
use crate::glyph_run::GlyphRun;

#[derive(Debug)]
struct RubyFontRun {
    byte_range: std::ops::Range<usize>,
    family_id: u16,
    weight: u16,
}

fn resolve_ruby_font_runs(
    text: &str,
    family: &str,
    weight: u16,
    resource: &mut Resource,
) -> Vec<RubyFontRun> {
    let requested_family_id = resource.font_registry.intern(family);
    let segmenters = std::sync::Arc::clone(&resource.segmenters);
    let mut runs: Vec<RubyFontRun> = Vec::new();
    let mut cluster_codepoints: Vec<u32> = Vec::new();
    let mut cluster_start = 0usize;

    for boundary in segmenters
        .grapheme
        .as_borrowed()
        .segment_str(text)
        .filter(|&b| b > 0)
    {
        let cluster = &text[cluster_start..boundary];
        cluster_codepoints.clear();
        cluster_codepoints.extend(cluster.chars().map(|c| c as u32));

        let (family_id, resolved_weight) = resolve_cluster_family_weight(
            &resource.font_registry,
            requested_family_id,
            weight,
            &cluster_codepoints,
        );
        if let Some(last) = runs.last_mut()
            && last.family_id == family_id
            && last.weight == resolved_weight
        {
            last.byte_range.end = boundary;
        } else {
            runs.push(RubyFontRun {
                byte_range: cluster_start..boundary,
                family_id,
                weight: resolved_weight,
            });
        }
        cluster_start = boundary;
    }

    runs
}

pub(crate) fn build_ruby_annotations(
    line: &ExtractedLine,
    line_width: f32,
    groups: &[RubyGroup],
    group_offsets: &mut [usize],
    resource: &mut Resource,
) -> Vec<RubyAnnotation> {
    use editor_common::StrExt;

    if groups.is_empty() || line.glyph_runs.is_empty() {
        return Vec::new();
    }

    struct Pending {
        font_size: f32,
        synthesis: Synthesis,
        color: String,
        ascent: f32,
        descent: f32,
        glyph_runs_relative: Vec<RubyGlyphRun>,
        x: f32,
        width: f32,
    }
    let mut pending: Vec<Pending> = Vec::new();

    let group_of_run = |r: &GlyphRun| -> Option<(usize, &RubyGroup)> {
        groups.iter().enumerate().find(|(_, g)| {
            g.offset_range.start <= r.offset_range.start && r.offset_range.end <= g.offset_range.end
        })
    };

    let mut i = 0;
    while i < line.glyph_runs.len() {
        let run = &line.glyph_runs[i];
        let Some((g_idx, group)) = group_of_run(run) else {
            i += 1;
            continue;
        };

        let mut j = i + 1;
        while j < line.glyph_runs.len() {
            let next = &line.glyph_runs[j];
            if matches!(group_of_run(next), Some((idx, _)) if idx == g_idx) {
                j += 1;
            } else {
                break;
            }
        }

        let slice = &line.glyph_runs[i..j];
        let base_min_x = slice.iter().map(|r| r.x).fold(f32::INFINITY, f32::min);
        let base_max_x = slice
            .iter()
            .map(|r| r.x + r.width)
            .fold(f32::NEG_INFINITY, f32::max);
        let base_width = (base_max_x - base_min_x).max(0.0);

        let line_chars: usize = slice.iter().map(|r| r.text.as_str().char_count()).sum();
        let ruby_chars: Vec<char> = group.text.chars().collect();
        let ruby_len = ruby_chars.len();

        let start_chars = group_offsets[g_idx];
        let end_chars = start_chars + line_chars;
        group_offsets[g_idx] = end_chars;

        let total = group.total_base_chars.max(1);
        let start_ratio = (start_chars as f32) / (total as f32);
        let end_ratio = (end_chars as f32) / (total as f32);
        let ruby_start = (start_ratio * ruby_len as f32).round() as usize;
        let ruby_end = ((end_ratio * ruby_len as f32).round() as usize).min(ruby_len);
        if ruby_start >= ruby_end {
            i = j;
            continue;
        }
        let ruby_slice: String = ruby_chars[ruby_start..ruby_end].iter().collect();

        let first = &slice[0];
        let ruby_font_size = (first.font_size * RUBY_FONT_SIZE_RATIO).max(RUBY_FONT_SIZE_MIN_PX);

        let font_runs = resolve_ruby_font_runs(
            &ruby_slice,
            &group.requested_font_family,
            group.requested_font_weight,
            resource,
        );
        let family_names = font_runs
            .iter()
            .map(|run| {
                resource
                    .font_registry
                    .family_name_opt(run.family_id)
                    .unwrap_or_default()
                    .to_owned()
            })
            .collect::<Vec<_>>();

        let resource = &mut *resource;
        let mut builder = resource.layout_context.style_run_builder(
            &mut resource.font_context,
            &ruby_slice,
            1.0,
            true,
        );
        for (run_index, (font_run, family_name)) in font_runs.iter().zip(&family_names).enumerate()
        {
            let ruby_style = TextStyle {
                font_family: FontFamily::Single(FontFamilyName::Named(Cow::Borrowed(family_name))),
                font_size: ruby_font_size,
                font_weight: ParleyFontWeight::new(font_run.weight as f32),
                line_height: LineHeight::FontSizeRelative(1.0),
                brush: TextBrush { run_index },
                font_features: FontFeatures::Source(Cow::Borrowed(
                    "\"ss05\" 1, \"cv12\" 1, \"ss18\" 1",
                )),
                word_break: WordBreak::BreakAll,
                overflow_wrap: OverflowWrap::Anywhere,
                ..TextStyle::default()
            };
            let style_index = builder.push_style(ruby_style);
            builder.push_style_run(style_index, font_run.byte_range.clone());
        }

        let mut layout = builder.build(&ruby_slice);
        layout.break_all_lines(None);

        let Some(ruby_line) = layout.lines().next() else {
            i = j;
            continue;
        };
        let m = ruby_line.metrics();
        let ruby_width = m.advance;
        let ruby_ascent = m.ascent;
        let ruby_descent = m.descent;

        let ruby_x = (base_min_x + (base_width - ruby_width) / 2.0)
            .clamp(0.0, (line_width - ruby_width).max(0.0));

        let mut glyph_runs_relative = Vec::new();
        for item in ruby_line.items() {
            if let parley::PositionedLayoutItem::GlyphRun(gr) = item {
                let font_run = &font_runs[gr.style().brush.run_index];
                let run_x = gr.offset();
                let mut adv = 0.0;
                let glyphs = gr
                    .glyphs()
                    .map(|g| {
                        let gx = adv + g.x;
                        adv += g.advance;
                        Glyph {
                            id: g.id,
                            x: run_x + gx,
                            y: g.y,
                        }
                    })
                    .collect();
                glyph_runs_relative.push(RubyGlyphRun {
                    family_id: font_run.family_id,
                    weight: font_run.weight,
                    glyphs,
                });
            }
        }

        pending.push(Pending {
            font_size: ruby_font_size,
            synthesis: first.synthesis,
            color: first.color.clone(),
            ascent: ruby_ascent,
            descent: ruby_descent,
            glyph_runs_relative,
            x: ruby_x,
            width: ruby_width,
        });

        i = j;
    }

    if pending.is_empty() {
        return Vec::new();
    }

    let max_descent = pending.iter().map(|p| p.descent).fold(0.0, f32::max);
    let baseline_y = (line.baseline - line.ascent) - RUBY_GAP - max_descent;

    pending
        .into_iter()
        .map(|p| {
            let glyph_runs = p
                .glyph_runs_relative
                .into_iter()
                .map(|mut run| {
                    for glyph in &mut run.glyphs {
                        glyph.x += p.x;
                        glyph.y += baseline_y;
                    }
                    run
                })
                .collect();
            RubyAnnotation {
                font_size: p.font_size,
                synthesis: p.synthesis,
                color: p.color,
                ascent: p.ascent,
                descent: p.descent,
                glyph_runs,
                x: p.x,
                baseline_y,
                width: p.width,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use editor_resource::{
        FontFamily, FontFamilySource, FontManifest, FontWeight, Resource, compress_zstd,
    };

    use super::*;
    use crate::glyph_run::{GraphemeSpan, TextDecoration};

    fn make_run(
        offset_range: Range<usize>,
        text: &str,
        x: f32,
        width: f32,
        font_size: f32,
    ) -> GlyphRun {
        GlyphRun {
            family_id: 0,
            weight: 400,
            font_size,
            synthesis: Synthesis::default(),
            color: "text.black".to_string(),
            background_color: None,
            glyphs: vec![],
            decoration: TextDecoration::default(),
            offset_range,
            link: None,
            text: text.to_string(),
            x,
            width,
            graphemes: vec![GraphemeSpan {
                advance: width,
                codepoints: text.chars().count() as u8,
            }],
            cursor_ascent: 0.0,
            cursor_descent: 0.0,
        }
    }

    fn make_line(runs: Vec<GlyphRun>) -> ExtractedLine {
        ExtractedLine {
            height: 16.0,
            baseline: 13.0,
            ascent: 13.0,
            descent: 3.0,
            glyph_runs: runs,
            tab_gaps_raw: vec![],
            is_phantom: false,
            content_edge_x: None,
        }
    }

    fn group(text: &str, offset_range: Range<usize>, total_base_chars: usize) -> RubyGroup {
        RubyGroup {
            text: text.to_string(),
            offset_range,
            total_base_chars,
            requested_font_family: editor_model::DEFAULT_FONT_FAMILY.to_string(),
            requested_font_weight: editor_model::DEFAULT_FONT_WEIGHT,
        }
    }

    fn fallback_resource() -> Resource {
        const TEST_FONT: &[u8] = include_bytes!("../../../assets/test-font.ttf");

        let mut resource = Resource::new_test();
        resource.set_fonts(vec![
            FontFamily {
                name: "Primary".to_string(),
                source: FontFamilySource::Default,
                weights: vec![FontWeight {
                    value: 400,
                    hash: "primary".to_string(),
                }],
            },
            FontFamily {
                name: "Fallback".to_string(),
                source: FontFamilySource::Fallback,
                weights: vec![FontWeight {
                    value: 700,
                    hash: "fallback".to_string(),
                }],
            },
        ]);
        let primary_id = resource.font_registry.intern_id("Primary").unwrap();
        resource.font_registry.set_manifest(
            primary_id,
            400,
            FontManifest::from_coverages(&[vec!['A' as u32, 'A' as u32, 'C' as u32, 'C' as u32]]),
        );
        let fallback_id = resource.font_registry.intern_id("Fallback").unwrap();
        resource.font_registry.set_manifest(
            fallback_id,
            700,
            FontManifest::from_coverages(&[vec!['B' as u32, 'B' as u32]]),
        );

        let font = compress_zstd(TEST_FONT);
        let empty_chunk = 0u32.to_be_bytes();
        for (family, weight) in [("Primary", 400), ("Fallback", 700)] {
            resource.add_font_base(family, weight, &font).unwrap();
            resource
                .add_font_chunk(family, weight, 0, &empty_chunk)
                .unwrap();
        }
        resource
    }

    #[test]
    fn ruby_resolves_each_character_from_group_first_requested_font() {
        let mut resource = fallback_resource();
        let fallback = resource.font_registry.intern_id("Fallback").unwrap();
        let primary = resource.font_registry.intern_id("Primary").unwrap();
        let mut base_run = make_run(0..1, "B", 0.0, 32.0, 16.0);
        base_run.family_id = fallback;
        base_run.weight = 700;
        let line = make_line(vec![base_run]);
        let groups = vec![RubyGroup {
            text: "ABC".to_string(),
            offset_range: 0..1,
            total_base_chars: 1,
            requested_font_family: "Primary".to_string(),
            requested_font_weight: 400,
        }];
        let mut offsets = vec![0];

        let annotations =
            build_ruby_annotations(&line, 200.0, &groups, &mut offsets, &mut resource);

        assert_eq!(annotations.len(), 1);
        assert_eq!(
            annotations[0]
                .glyph_runs
                .iter()
                .map(|run| (run.family_id, run.weight))
                .collect::<Vec<_>>(),
            vec![(primary, 400), (fallback, 700), (primary, 400)]
        );
    }

    #[test]
    fn ruby_grapheme_cluster_stays_in_one_font_run() {
        // "A" + U+0301: 결합 부호를 어느 폰트도 커버하지 못해도 cluster는
        // base 문자 'A'의 폰트(Primary)로 통째로 배정되어야 한다.
        let mut resource = fallback_resource();
        let primary = resource.font_registry.intern_id("Primary").unwrap();

        let runs = resolve_ruby_font_runs("A\u{0301}", "Primary", 400, &mut resource);

        assert_eq!(
            runs.iter()
                .map(|r| (r.byte_range.clone(), r.family_id, r.weight))
                .collect::<Vec<_>>(),
            vec![(0..3, primary, 400)],
            "cluster가 폰트 경계로 쪼개지면 안 된다"
        );
    }

    #[test]
    fn single_ruby_centers_over_base() {
        let mut res = Resource::new_test();
        let line = make_line(vec![make_run(0..2, "AB", 20.0, 20.0, 16.0)]);
        let groups = vec![group("ru", 0..2, 2)];
        let mut offsets = vec![0usize; 1];
        let out = build_ruby_annotations(&line, 200.0, &groups, &mut offsets, &mut res);
        assert_eq!(out.len(), 1);
        let ann = &out[0];
        // exact centering formula (codex hardening #3): base_min_x=20, base_width=20, line_width=200.
        let expected_x = (20.0 + (20.0 - ann.width) / 2.0).clamp(0.0, (200.0 - ann.width).max(0.0));
        assert!(
            (ann.x - expected_x).abs() < 0.01,
            "ruby_x must equal the exact centering/clamp formula"
        );
        assert!(ann.font_size < 16.0); // RATIO applied → smaller than base 16px
    }

    #[test]
    fn no_groups_or_empty_line_no_annotations() {
        let mut res = Resource::new_test();
        let line = make_line(vec![make_run(0..1, "a", 0.0, 10.0, 16.0)]);
        let mut offsets: Vec<usize> = vec![];
        assert!(build_ruby_annotations(&line, 100.0, &[], &mut offsets, &mut res).is_empty());
        let empty = make_line(vec![]);
        let groups = vec![group("x", 0..1, 1)];
        let mut offsets = vec![0usize; 1];
        assert!(build_ruby_annotations(&empty, 100.0, &groups, &mut offsets, &mut res).is_empty());
    }

    #[test]
    fn run_outside_any_group_is_skipped() {
        // glyph run at offset 5..6 is not contained in the group's 0..2 → not annotated,
        // and a skipped run must NOT advance group_offsets (codex hardening #2).
        let mut res = Resource::new_test();
        let line = make_line(vec![make_run(5..6, "a", 0.0, 10.0, 16.0)]);
        let groups = vec![group("x", 0..2, 2)];
        let mut offsets = vec![0usize; 1];
        assert!(build_ruby_annotations(&line, 100.0, &groups, &mut offsets, &mut res).is_empty());
        assert_eq!(
            offsets[0], 0,
            "a skipped (non-contained) run must not consume base chars"
        );
    }

    #[test]
    fn partial_overlap_run_is_not_grouped() {
        // A run that overlaps a group on ONE side only (run 1..3 vs group 0..2) must NOT match —
        // the lookup is CONTAINMENT (run ⊆ group), not overlap. (codex hardening #1: distinguishes
        // the containment predicate from a looser overlap predicate that would false-pass.)
        let mut res = Resource::new_test();
        let line = make_line(vec![make_run(1..3, "ab", 0.0, 20.0, 16.0)]);
        let groups = vec![group("x", 0..2, 2)];
        let mut offsets = vec![0usize; 1];
        let out = build_ruby_annotations(&line, 100.0, &groups, &mut offsets, &mut res);
        assert!(
            out.is_empty(),
            "run 1..3 is not ⊆ group 0..2 → must not be annotated"
        );
        assert_eq!(offsets[0], 0);
    }

    #[test]
    fn consecutive_runs_same_group_merge_to_one_annotation() {
        // two glyph runs both ⊆ the single group's 0..2 → ONE annotation spanning both.
        let mut res = Resource::new_test();
        let line = make_line(vec![
            make_run(0..1, "A", 0.0, 16.0, 16.0),
            make_run(1..2, "B", 16.0, 16.0, 16.0),
        ]);
        let groups = vec![group("ru", 0..2, 2)];
        let mut offsets = vec![0usize; 1];
        let out = build_ruby_annotations(&line, 200.0, &groups, &mut offsets, &mut res);
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn two_groups_two_annotations() {
        let mut res = Resource::new_test();
        let line = make_line(vec![
            make_run(0..1, "A", 0.0, 16.0, 16.0),
            make_run(1..2, "B", 16.0, 16.0, 16.0),
        ]);
        let groups = vec![group("a", 0..1, 1), group("b", 1..2, 1)];
        let mut offsets = vec![0usize; 2];
        let out = build_ruby_annotations(&line, 200.0, &groups, &mut offsets, &mut res);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn ruby_clamps_to_line_edges() {
        let mut res = Resource::new_test();
        // base at left edge, ruby much wider → clamped to x >= 0
        let left = make_line(vec![make_run(0..1, "A", 0.0, 4.0, 16.0)]);
        let groups = vec![group("very_long_ruby", 0..1, 1)];
        let mut offsets = vec![0usize; 1];
        let out = build_ruby_annotations(&left, 200.0, &groups, &mut offsets, &mut res);
        assert_eq!(out.len(), 1);
        assert!(out[0].x >= 0.0);
        // base near right edge → clamped so x + width <= line_width
        let right = make_line(vec![make_run(0..1, "A", 196.0, 4.0, 16.0)]);
        let mut offsets = vec![0usize; 1];
        let out = build_ruby_annotations(&right, 200.0, &groups, &mut offsets, &mut res);
        assert!(out[0].x + out[0].width <= 200.0 + 0.01);
    }

    #[test]
    fn wrap_distribution_accumulates_across_calls() {
        // group spans 4 base chars; two lines each cover 2 chars (two calls share group_offsets).
        let mut res = Resource::new_test();
        let groups = vec![group("abcd", 0..4, 4)];
        let mut offsets = vec![0usize; 1];
        let line_a = make_line(vec![make_run(0..2, "AB", 0.0, 32.0, 16.0)]);
        let out_a = build_ruby_annotations(&line_a, 200.0, &groups, &mut offsets, &mut res);
        assert_eq!(offsets[0], 2, "first line consumes 2 base chars");
        assert_eq!(out_a.len(), 1);
        let line_b = make_line(vec![make_run(2..4, "CD", 0.0, 32.0, 16.0)]);
        let out_b = build_ruby_annotations(&line_b, 200.0, &groups, &mut offsets, &mut res);
        assert_eq!(offsets[0], 4, "second line consumes 2 more (total 4)");
        assert_eq!(out_b.len(), 1);
    }

    #[test]
    fn ruby_font_size_floor_and_ratio() {
        let mut res = Resource::new_test();
        // base 16 * 0.5 = 8 < 12 floor → clamps to RUBY_FONT_SIZE_MIN_PX (12)
        let line = make_line(vec![make_run(0..1, "A", 0.0, 16.0, 16.0)]);
        let groups = vec![group("x", 0..1, 1)];
        let mut offsets = vec![0usize; 1];
        let out = build_ruby_annotations(&line, 200.0, &groups, &mut offsets, &mut res);
        assert!((out[0].font_size - RUBY_FONT_SIZE_MIN_PX).abs() < 0.01);
        // base 32 * 0.5 = 16 > 12 floor → ratio wins (16)
        let line = make_line(vec![make_run(0..1, "A", 0.0, 32.0, 32.0)]);
        let mut offsets = vec![0usize; 1];
        let out = build_ruby_annotations(&line, 200.0, &groups, &mut offsets, &mut res);
        assert!((out[0].font_size - 16.0).abs() < 0.01);
    }

    #[test]
    fn two_annotations_share_baseline() {
        let mut res = Resource::new_test();
        let line = make_line(vec![
            make_run(0..1, "A", 0.0, 16.0, 16.0),
            make_run(1..2, "B", 16.0, 16.0, 24.0), // different font size
        ]);
        let groups = vec![group("x", 0..1, 1), group("y", 1..2, 1)];
        let mut offsets = vec![0usize; 2];
        let out = build_ruby_annotations(&line, 200.0, &groups, &mut offsets, &mut res);
        assert_eq!(out.len(), 2);
        assert!((out[0].baseline_y - out[1].baseline_y).abs() < 0.01);
    }

    #[test]
    fn ruby_descent_bottom_exactly_ruby_gap_above_base_ascent_top() {
        // pins baseline_y = (line.baseline - line.ascent) - RUBY_GAP - max_descent.
        let mut res = Resource::new_test();
        // make_line defaults baseline=13, ascent=13 → base_ascent_top = 0.
        let line = make_line(vec![make_run(0..1, "A", 0.0, 16.0, 16.0)]);
        let groups = vec![group("x", 0..1, 1)];
        let mut offsets = vec![0usize; 1];
        let out = build_ruby_annotations(&line, 200.0, &groups, &mut offsets, &mut res);
        assert_eq!(out.len(), 1);
        let ann = &out[0];
        let base_ascent_top = line.baseline - line.ascent;
        let ruby_descent_bottom = ann.baseline_y + ann.descent;
        let gap = base_ascent_top - ruby_descent_bottom;
        assert!(
            (gap - RUBY_GAP).abs() < 0.01,
            "gap {gap} must equal RUBY_GAP {RUBY_GAP}"
        );
    }
}
