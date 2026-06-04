use editor_model::{Modifier, Node, NodeId, NodeRef};
use editor_resource::{Resource, TextBrush};
use parley::style::{
    FontFamily, FontFamilyName, FontFeatures, FontWeight as ParleyFontWeight, LineHeight, TextStyle,
};
use parley::{OverflowWrap, WordBreak};
use smallvec::SmallVec;
use std::borrow::Cow;

use crate::glyph_run::{Glyph, RubyAnnotation, Synthesis};
use crate::measure::text::extract::ExtractedLine;

pub(crate) const RUBY_FONT_SIZE_RATIO: f32 = 0.5;
// 9pt readability floor, expressed in px (9 * 96 / 72).
pub(crate) const RUBY_FONT_SIZE_MIN_PX: f32 = 12.0;
pub(crate) const RUBY_GAP: f32 = 2.0;

/// Space ruby reserves above the base text, inflating the line's `ascent` and
/// `baseline` (see `measure_inline_text`); 0 without ruby. Subtract from
/// `ascent` for the base-text ascent so backgrounds/selection skip the ruby.
///
/// Takes the already-inflated `baseline`/`ascent`: their difference is
/// invariant under the inflation, so this recovers what the measure pass added.
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RubyGroup {
    pub text: String,
    pub node_ids: SmallVec<[NodeId; 2]>,
    pub total_base_chars: usize,
}

pub(crate) fn identify_ruby_groups(paragraph: &NodeRef<'_>) -> Vec<RubyGroup> {
    let mut groups: Vec<RubyGroup> = Vec::new();
    let mut current: Option<RubyGroup> = None;

    let flush = |current: &mut Option<RubyGroup>, groups: &mut Vec<RubyGroup>| {
        if let Some(g) = current.take() {
            // drop groups with no ruby text or no base chars — nothing to render.
            if !g.text.is_empty() && g.total_base_chars > 0 {
                groups.push(g);
            }
        }
    };

    for child in paragraph.children() {
        let Node::Text(text_node) = child.node() else {
            flush(&mut current, &mut groups);
            continue;
        };

        let ruby_text: Option<&str> = child.modifiers().find_map(|m| match m {
            Modifier::Ruby { text } => Some(text.as_str()),
            _ => None,
        });

        let Some(ruby_text) = ruby_text else {
            flush(&mut current, &mut groups);
            continue;
        };

        if ruby_text.is_empty() {
            flush(&mut current, &mut groups);
            continue;
        }

        let chars = text_node.text.len();

        match current.as_mut() {
            Some(g) if g.text == ruby_text => {
                g.node_ids.push(child.id());
                g.total_base_chars += chars;
            }
            _ => {
                flush(&mut current, &mut groups);
                current = Some(RubyGroup {
                    text: ruby_text.to_owned(),
                    node_ids: smallvec::smallvec![child.id()],
                    total_base_chars: chars,
                });
            }
        }
    }

    flush(&mut current, &mut groups);
    groups
}

pub(crate) fn build_ruby_annotations_for_line(
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
        family_id: u16,
        weight: u16,
        font_size: f32,
        synthesis: Synthesis,
        color: String,
        ascent: f32,
        descent: f32,
        glyphs_relative: Vec<Glyph>, // y relative to ruby baseline, x relative to ruby start.
        x: f32,
        width: f32,
    }
    let mut pending: Vec<Pending> = Vec::new();

    let group_of_node = |node_id: editor_model::NodeId| -> Option<(usize, &RubyGroup)> {
        groups
            .iter()
            .enumerate()
            .find(|(_, g)| g.node_ids.contains(&node_id))
    };

    let mut i = 0;
    while i < line.glyph_runs.len() {
        let run = &line.glyph_runs[i];
        let Some((g_idx, group)) = group_of_node(run.node_id) else {
            i += 1;
            continue;
        };

        let mut j = i + 1;
        while j < line.glyph_runs.len() {
            let next = &line.glyph_runs[j];
            if matches!(group_of_node(next.node_id), Some((idx, _)) if idx == g_idx) {
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

        // same builder pattern as layout.rs / list_item.rs.
        let family_name = resource
            .font_registry
            .family_name_opt(first.family_id)
            .unwrap_or_default()
            .to_owned();

        let ruby_style = TextStyle {
            font_family: FontFamily::Single(FontFamilyName::Named(Cow::Owned(family_name))),
            font_size: ruby_font_size,
            font_weight: ParleyFontWeight::new(first.weight as f32),
            line_height: LineHeight::FontSizeRelative(1.0),
            brush: TextBrush {
                node_id: first.node_id,
            },
            font_features: FontFeatures::Source(Cow::Borrowed(
                "\"ss05\" 1, \"cv12\" 1, \"ss18\" 1",
            )),
            word_break: WordBreak::BreakAll,
            overflow_wrap: OverflowWrap::Anywhere,
            ..TextStyle::default()
        };

        let resource = &mut *resource;
        let mut builder = resource.layout_context.style_run_builder(
            &mut resource.font_context,
            &ruby_slice,
            1.0,
            true,
        );
        let idx = builder.push_style(ruby_style);
        builder.push_style_run(idx, 0..ruby_slice.len());

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

        let mut glyphs_relative = Vec::new();
        for item in ruby_line.items() {
            if let parley::PositionedLayoutItem::GlyphRun(gr) = item {
                let run_x = gr.offset();
                let mut adv = 0.0;
                for g in gr.glyphs() {
                    let gx = adv + g.x;
                    adv += g.advance;
                    glyphs_relative.push(Glyph {
                        id: g.id,
                        x: run_x + gx,
                        y: g.y,
                    });
                }
            }
        }

        pending.push(Pending {
            family_id: first.family_id,
            weight: first.weight,
            font_size: ruby_font_size,
            synthesis: first.synthesis,
            color: first.color.clone(),
            ascent: ruby_ascent,
            descent: ruby_descent,
            glyphs_relative,
            x: ruby_x,
            width: ruby_width,
        });

        i = j;
    }

    if pending.is_empty() {
        return Vec::new();
    }

    // Line-local coordinates (line top = 0). Base ascent top = line.baseline - line.ascent.
    // Ruby descent bottom sits RUBY_GAP above that; ruby baseline = descent bottom - max_ruby_descent.
    // The extra_top correction is applied by measure_segment, so we compute against the
    // pre-correction line metrics here.
    let max_descent = pending.iter().map(|p| p.descent).fold(0.0, f32::max);
    let baseline_y = (line.baseline - line.ascent) - RUBY_GAP - max_descent;

    pending
        .into_iter()
        .map(|p| {
            let glyphs: Vec<Glyph> = p
                .glyphs_relative
                .into_iter()
                .map(|g| Glyph {
                    id: g.id,
                    x: p.x + g.x,
                    y: baseline_y + g.y,
                })
                .collect();
            RubyAnnotation {
                family_id: p.family_id,
                weight: p.weight,
                font_size: p.font_size,
                synthesis: p.synthesis,
                color: p.color,
                ascent: p.ascent,
                descent: p.descent,
                glyphs,
                x: p.x,
                baseline_y,
                width: p.width,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_macros::doc;

    #[test]
    fn empty_paragraph_returns_no_groups() {
        let (d, p1) = doc! { root { p1: paragraph } };
        let groups = identify_ruby_groups(&d.node(p1).unwrap());
        assert!(groups.is_empty());
    }

    #[test]
    fn paragraph_without_ruby_returns_no_groups() {
        let (d, p1) = doc! { root { p1: paragraph { text("abc") } } };
        let groups = identify_ruby_groups(&d.node(p1).unwrap());
        assert!(groups.is_empty());
    }

    #[test]
    fn single_ruby_text_node_one_group() {
        let (d, p1) = doc! {
            root { p1: paragraph { text("한자") [ruby(text: "한자".to_string())] } }
        };
        let groups = identify_ruby_groups(&d.node(p1).unwrap());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].text, "한자");
        assert_eq!(groups[0].total_base_chars, 2);
        assert_eq!(groups[0].node_ids.len(), 1);
    }

    #[test]
    fn adjacent_same_ruby_merges() {
        let (d, p1) = doc! {
            root {
                p1: paragraph {
                    text("굵게") [font_weight(700), ruby(text: "루비".to_string())]
                    text("보통")  [ruby(text: "루비".to_string())]
                }
            }
        };
        let groups = identify_ruby_groups(&d.node(p1).unwrap());
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].text, "루비");
        assert_eq!(groups[0].total_base_chars, 4);
        assert_eq!(groups[0].node_ids.len(), 2);
    }

    #[test]
    fn adjacent_different_ruby_splits() {
        let (d, p1) = doc! {
            root {
                p1: paragraph {
                    text("A") [ruby(text: "a".to_string())]
                    text("B") [ruby(text: "b".to_string())]
                }
            }
        };
        let groups = identify_ruby_groups(&d.node(p1).unwrap());
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn non_ruby_text_between_breaks_group() {
        let (d, p1) = doc! {
            root {
                p1: paragraph {
                    text("A") [ruby(text: "x".to_string())]
                    text("plain")
                    text("B") [ruby(text: "x".to_string())]
                }
            }
        };
        let groups = identify_ruby_groups(&d.node(p1).unwrap());
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn hard_break_between_breaks_group() {
        let (d, p1) = doc! {
            root {
                p1: paragraph {
                    text("A") [ruby(text: "x".to_string())]
                    hard_break
                    text("B") [ruby(text: "x".to_string())]
                }
            }
        };
        let groups = identify_ruby_groups(&d.node(p1).unwrap());
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn empty_ruby_text_does_not_form_group() {
        let (d, p1) = doc! {
            root {
                p1: paragraph {
                    text("A") [ruby(text: "".to_string())]
                }
            }
        };
        let groups = identify_ruby_groups(&d.node(p1).unwrap());
        assert!(groups.is_empty());
    }

    #[test]
    fn zero_base_char_group_is_dropped() {
        let (d, p1) = doc! {
            root {
                p1: paragraph {
                    text("") [ruby(text: "ruby".to_string())]
                }
            }
        };
        let groups = identify_ruby_groups(&d.node(p1).unwrap());
        assert!(
            groups.is_empty(),
            "group with zero base chars must be dropped even if ruby modifier is present"
        );
    }
}

#[cfg(test)]
mod build_tests {
    use super::*;
    use crate::glyph_run::{GlyphRun, GraphemeSpan, TextDecoration};
    use crate::measure::text::extract::ExtractedLine;
    use editor_model::NodeId;
    use editor_resource::Resource;

    fn make_run(node_id: NodeId, text: &str, x: f32, width: f32, font_size: f32) -> GlyphRun {
        GlyphRun {
            family_id: 0,
            weight: 400,
            font_size,
            synthesis: Synthesis::default(),
            color: "text.black".to_string(),
            background_color: None,
            glyphs: vec![],
            decoration: TextDecoration::default(),
            node_id,
            offset: 0,
            text: text.to_string(),
            x,
            width,
            graphemes: vec![GraphemeSpan {
                advance: width,
                codepoints: text.chars().count() as u8,
            }],
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

    #[test]
    fn no_groups_no_annotations() {
        let mut res = Resource::new_test();
        let n = NodeId::new();
        let line = make_line(vec![make_run(n, "abc", 0.0, 30.0, 16.0)]);
        let mut offsets = vec![0usize; 0];
        let out = build_ruby_annotations_for_line(&line, 100.0, &[], &mut offsets, &mut res);
        assert!(out.is_empty());
    }

    #[test]
    fn single_ruby_centers_over_base() {
        let mut res = Resource::new_test();
        let n = NodeId::new();
        let group = RubyGroup {
            text: "ru".to_string(),
            node_ids: smallvec::smallvec![n],
            total_base_chars: 2,
        };
        let line = make_line(vec![make_run(n, "AB", 20.0, 20.0, 16.0)]);
        let mut offsets = vec![0usize; 1];
        let out = build_ruby_annotations_for_line(&line, 200.0, &[group], &mut offsets, &mut res);
        assert_eq!(out.len(), 1);
        let ann = &out[0];
        // center: 20 + (20 - ruby_width)/2. ruby_width > 0 so ann.x lands inside the base span.
        assert!(ann.x >= 20.0 - ann.width);
        assert!(ann.x + ann.width <= 40.0 + ann.width);
        assert!(ann.font_size < 16.0); // RUBY_FONT_SIZE_RATIO applied
    }

    #[test]
    fn ruby_clamps_to_line_left_edge() {
        let mut res = Resource::new_test();
        let n = NodeId::new();
        let group = RubyGroup {
            text: "very_long_ruby".to_string(),
            node_ids: smallvec::smallvec![n],
            total_base_chars: 1,
        };
        // base sits at the left edge (x=0, width=4); ruby is much wider than the base.
        let line = make_line(vec![make_run(n, "A", 0.0, 4.0, 16.0)]);
        let mut offsets = vec![0usize; 1];
        let out = build_ruby_annotations_for_line(&line, 200.0, &[group], &mut offsets, &mut res);
        assert_eq!(out.len(), 1);
        assert!(out[0].x >= 0.0);
    }

    #[test]
    fn ruby_clamps_to_line_right_edge() {
        let mut res = Resource::new_test();
        let n = NodeId::new();
        let group = RubyGroup {
            text: "very_long_ruby".to_string(),
            node_ids: smallvec::smallvec![n],
            total_base_chars: 1,
        };
        let line = make_line(vec![make_run(n, "A", 196.0, 4.0, 16.0)]);
        let mut offsets = vec![0usize; 1];
        let out = build_ruby_annotations_for_line(&line, 200.0, &[group], &mut offsets, &mut res);
        assert_eq!(out.len(), 1);
        assert!(out[0].x + out[0].width <= 200.0 + 0.01);
    }

    #[test]
    fn two_groups_two_annotations() {
        let mut res = Resource::new_test();
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        let groups = vec![
            RubyGroup {
                text: "a".to_string(),
                node_ids: smallvec::smallvec![n1],
                total_base_chars: 1,
            },
            RubyGroup {
                text: "b".to_string(),
                node_ids: smallvec::smallvec![n2],
                total_base_chars: 1,
            },
        ];
        let line = make_line(vec![
            make_run(n1, "A", 0.0, 16.0, 16.0),
            make_run(n2, "B", 16.0, 16.0, 16.0),
        ]);
        let mut offsets = vec![0usize; groups.len()];
        let out = build_ruby_annotations_for_line(&line, 200.0, &groups, &mut offsets, &mut res);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn wrap_distribution_partial_share() {
        let mut res = Resource::new_test();
        let n = NodeId::new();
        // The group spans 4 chars total; this line occupies only 2 of them.
        let group = RubyGroup {
            text: "abcd".to_string(),
            node_ids: smallvec::smallvec![n],
            total_base_chars: 4,
        };
        let line = make_line(vec![make_run(n, "AB", 0.0, 32.0, 16.0)]);
        let mut offsets = vec![0usize; 1];
        let out = build_ruby_annotations_for_line(&line, 200.0, &[group], &mut offsets, &mut res);
        assert_eq!(out.len(), 1);
        // Exact slice correctness is verified by integration tests; here we only check non-emptiness.
        assert!(out[0].width > 0.0);
    }

    #[test]
    fn shared_baseline_across_two_annotations() {
        let mut res = Resource::new_test();
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        let groups = vec![
            RubyGroup {
                text: "x".to_string(),
                node_ids: smallvec::smallvec![n1],
                total_base_chars: 1,
            },
            RubyGroup {
                text: "y".to_string(),
                node_ids: smallvec::smallvec![n2],
                total_base_chars: 1,
            },
        ];
        let line = make_line(vec![
            make_run(n1, "A", 0.0, 16.0, 16.0),
            // larger font_size on the second run means different ruby_font_size, making baseline alignment meaningful to verify.
            make_run(n2, "B", 16.0, 16.0, 24.0),
        ]);
        let mut offsets = vec![0usize; groups.len()];
        let out = build_ruby_annotations_for_line(&line, 200.0, &groups, &mut offsets, &mut res);
        assert_eq!(out.len(), 2);
        assert!((out[0].baseline_y - out[1].baseline_y).abs() < 0.01);
    }

    #[test]
    fn group_offsets_accumulates_across_calls() {
        let mut res = Resource::new_test();
        let n = NodeId::new();
        let group = RubyGroup {
            text: "abcd".to_string(),
            node_ids: smallvec::smallvec![n],
            total_base_chars: 4,
        };
        let line_a = make_line(vec![make_run(n, "AB", 0.0, 32.0, 16.0)]);
        let line_b = make_line(vec![make_run(n, "CD", 0.0, 32.0, 16.0)]);

        let mut offsets = vec![0usize];
        let out_a = build_ruby_annotations_for_line(
            &line_a,
            200.0,
            std::slice::from_ref(&group),
            &mut offsets,
            &mut res,
        );
        assert_eq!(
            offsets[0], 2,
            "accumulated offset after first line consumes 2 chars must be 2"
        );
        assert_eq!(out_a.len(), 1);

        let out_b =
            build_ruby_annotations_for_line(&line_b, 200.0, &[group], &mut offsets, &mut res);
        assert_eq!(
            offsets[0], 4,
            "accumulated offset after second line consumes 2 more chars must be 4"
        );
        assert_eq!(out_b.len(), 1);
    }

    #[test]
    fn ruby_descent_bottom_exactly_ruby_gap_above_base_ascent_top() {
        let mut res = Resource::new_test();
        let n = NodeId::new();
        let group = RubyGroup {
            text: "x".to_string(),
            node_ids: smallvec::smallvec![n],
            total_base_chars: 1,
        };
        // make_line defaults: baseline=13.0, ascent=13.0, so base_ascent_top = 0.0.
        let line = make_line(vec![make_run(n, "A", 0.0, 16.0, 16.0)]);
        let mut offsets = vec![0usize];
        let out = build_ruby_annotations_for_line(&line, 200.0, &[group], &mut offsets, &mut res);
        assert_eq!(out.len(), 1);
        let ann = &out[0];

        let base_ascent_top = line.baseline - line.ascent;
        let ruby_descent_bottom = ann.baseline_y + ann.descent;
        let gap = base_ascent_top - ruby_descent_bottom;

        assert!(
            (gap - RUBY_GAP).abs() < 0.01,
            "gap between ruby descent bottom and base ascent top must equal RUBY_GAP (gap={}, RUBY_GAP={})",
            gap,
            RUBY_GAP
        );
    }

    #[test]
    fn ruby_font_size_respects_minimum_floor() {
        let mut res = Resource::new_test();
        let n = NodeId::new();
        let group = RubyGroup {
            text: "x".to_string(),
            node_ids: smallvec::smallvec![n],
            total_base_chars: 1,
        };
        // base 16px * 0.5 = 8px would fall below the 12px floor.
        let line = make_line(vec![make_run(n, "A", 0.0, 16.0, 16.0)]);
        let mut offsets = vec![0usize];
        let out = build_ruby_annotations_for_line(&line, 200.0, &[group], &mut offsets, &mut res);
        assert_eq!(out.len(), 1);
        assert!(
            (out[0].font_size - RUBY_FONT_SIZE_MIN_PX).abs() < 0.01,
            "ruby font_size must clamp to RUBY_FONT_SIZE_MIN_PX when base * ratio is smaller (got {})",
            out[0].font_size
        );
    }

    #[test]
    fn ruby_font_size_uses_ratio_when_above_floor() {
        let mut res = Resource::new_test();
        let n = NodeId::new();
        let group = RubyGroup {
            text: "x".to_string(),
            node_ids: smallvec::smallvec![n],
            total_base_chars: 1,
        };
        // base 32px * 0.5 = 16px stays above the 12px floor; ratio wins.
        let line = make_line(vec![make_run(n, "A", 0.0, 32.0, 32.0)]);
        let mut offsets = vec![0usize];
        let out = build_ruby_annotations_for_line(&line, 200.0, &[group], &mut offsets, &mut res);
        assert_eq!(out.len(), 1);
        assert!(
            (out[0].font_size - 16.0).abs() < 0.01,
            "ruby font_size must follow ratio when above floor (got {})",
            out[0].font_size
        );
    }
}
