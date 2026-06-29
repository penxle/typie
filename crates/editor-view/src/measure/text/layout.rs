use std::borrow::Cow;

use editor_model::Alignment;
use editor_resource::{Resource, TextBrush};
use parley::style::{
    FontFamily, FontFamilyName, FontFeatures, FontVariations, FontWeight, LineHeight, TextStyle,
};
use parley::{
    Alignment as ParleyAlignment, AlignmentOptions, IndentOptions, InlineBox, InlineBoxKind,
    Layout, OverflowWrap, WordBreak,
};

use super::inline::TabMark;
use super::style_run::StyleRun;

pub(crate) fn build_layout(
    text: &str,
    style_runs: &[StyleRun],
    align: Alignment,
    indent: f32,
    width: f32,
    resource: &mut Resource,
    tabs: &[(TabMark, f32)],
) -> Layout<TextBrush> {
    let mut builder =
        resource
            .layout_context
            .style_run_builder(&mut resource.font_context, text, 1.0, true);

    let family_names: Vec<String> = style_runs
        .iter()
        .map(|sr| {
            resource
                .font_registry
                .family_name_opt(sr.family)
                .unwrap_or_default()
                .to_owned()
        })
        .collect();
    for (style_run, family_name) in style_runs.iter().zip(&family_names) {
        let style = TextStyle {
            font_family: FontFamily::Single(FontFamilyName::Named(Cow::Borrowed(family_name))),
            font_size: style_run.font_size,
            font_weight: FontWeight::new(style_run.weight as f32),
            letter_spacing: style_run.letter_spacing,
            line_height: LineHeight::FontSizeRelative(style_run.line_height),
            brush: TextBrush {
                run_index: style_run.run_index,
            },
            font_variations: FontVariations::empty(),
            font_features: FontFeatures::Source(Cow::Borrowed(
                "\"ss05\" 1, \"cv12\" 1, \"ss18\" 1",
            )),
            word_break: WordBreak::BreakAll,
            overflow_wrap: OverflowWrap::Anywhere,
            ..TextStyle::default()
        };

        let idx = builder.push_style(style);
        builder.push_style_run(idx, style_run.byte_range.clone());
    }

    for (i, (tab, placeholder)) in tabs.iter().enumerate() {
        builder.push_inline_box(InlineBox {
            id: i as u64,
            kind: InlineBoxKind::InFlow,
            index: tab.byte_offset,
            width: *placeholder,
            height: 0.0,
        });
    }

    let mut layout = builder.build(text);

    if indent > 0.0 {
        layout.set_text_indent(indent, IndentOptions::default());
    }

    layout.break_all_lines(Some(width));

    let alignment = match align {
        Alignment::Left => ParleyAlignment::Start,
        Alignment::Center => ParleyAlignment::Center,
        Alignment::Right => ParleyAlignment::End,
        Alignment::Justify => ParleyAlignment::Justify,
    };
    layout.align(alignment, AlignmentOptions::default());

    layout
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use editor_model::{Modifier, ModifierType, OwnModifier};
    use editor_resource::Resource;

    use super::*;
    use crate::measure::text::resolve::ResolvedTextStyle;
    use crate::measure::text::tab_metric::tab_px;

    fn style_run(
        run_index: usize,
        byte_range: std::ops::Range<usize>,
        family: u16,
        font_size: f32,
    ) -> StyleRun {
        StyleRun {
            run_index,
            byte_range,
            family,
            weight: 400,
            font_size,
            letter_spacing: 0.0,
            line_height: 1.6,
        }
    }

    fn run_indices(layout: &Layout<TextBrush>) -> std::collections::BTreeSet<usize> {
        let mut out = std::collections::BTreeSet::new();
        for line in layout.lines() {
            for item in line.items() {
                if let parley::PositionedLayoutItem::GlyphRun(gr) = item {
                    out.insert(gr.style().brush.run_index);
                }
            }
        }
        out
    }

    fn run_index_by_byte_start(layout: &Layout<TextBrush>) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for line in layout.lines() {
            for item in line.items() {
                if let parley::PositionedLayoutItem::GlyphRun(gr) = item {
                    let run_index = gr.style().brush.run_index;
                    let start = gr
                        .run()
                        .visual_clusters()
                        .map(|c| c.text_range().start)
                        .min();
                    if let Some(start) = start {
                        out.push((start, run_index));
                    }
                }
            }
        }
        out
    }

    #[test]
    fn brush_round_trips_run_index_through_parley() {
        let mut resource = Resource::new_test();
        let fam = resource.font_registry.placeholder_family_id().unwrap();
        let style_runs = vec![style_run(0, 0..1, fam, 16.0), style_run(1, 1..2, fam, 32.0)];
        let layout = build_layout(
            "AB",
            &style_runs,
            Alignment::Left,
            0.0,
            1.0e6,
            &mut resource,
            &[],
        );
        assert_eq!(run_indices(&layout), [0usize, 1].into_iter().collect());
        let map = run_index_by_byte_start(&layout);
        assert_eq!(map.iter().find(|(b, _)| *b == 0).map(|(_, r)| *r), Some(0));
        assert_eq!(map.iter().find(|(b, _)| *b == 1).map(|(_, r)| *r), Some(1));
    }

    #[test]
    fn single_run_one_brush_value() {
        let mut resource = Resource::new_test();
        let fam = resource.font_registry.placeholder_family_id().unwrap();
        let style_runs = vec![style_run(0, 0..5, fam, 16.0)];
        let layout = build_layout(
            "hello",
            &style_runs,
            Alignment::Left,
            0.0,
            1.0e6,
            &mut resource,
            &[],
        );
        assert_eq!(run_indices(&layout), [0usize].into_iter().collect());
    }

    #[test]
    fn tab_produces_in_flow_inline_box() {
        let mut resource = Resource::new_test();
        let fam = resource.font_registry.placeholder_family_id().unwrap();
        let style_runs = vec![style_run(0, 0..2, fam, 16.0)];
        let own: BTreeMap<ModifierType, OwnModifier> = BTreeMap::new();
        let eff: BTreeMap<ModifierType, Modifier> = BTreeMap::new();
        let tab = TabMark {
            offset_index: 1,
            byte_offset: 1,
            own_modifiers: &own,
            effective: &eff,
            style: ResolvedTextStyle {
                font_family: String::new(),
                font_weight: 400,
                font_size: 16.0,
                letter_spacing: 0.0,
                line_height: 1.6,
            },
        };
        let tab_w = tab_px(&tab.style, &mut resource);
        let layout = build_layout(
            "ab",
            &style_runs,
            Alignment::Left,
            0.0,
            1.0e6,
            &mut resource,
            &[(tab, tab_w)],
        );
        let mut found_box = false;
        for line in layout.lines() {
            for item in line.items() {
                if let parley::PositionedLayoutItem::InlineBox(b) = item
                    && b.id == 0
                {
                    found_box = true;
                }
            }
        }
        assert!(found_box, "tab must produce an InlineBox with id 0");
    }

    #[test]
    fn narrow_width_wraps_into_multiple_lines() {
        let mut resource = Resource::new_test();
        let fam = resource.font_registry.placeholder_family_id().unwrap();
        let text = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let style_runs = vec![style_run(0, 0..text.len(), fam, 16.0)];
        let layout = build_layout(
            text,
            &style_runs,
            Alignment::Left,
            0.0,
            20.0,
            &mut resource,
            &[],
        );
        assert!(layout.lines().count() > 1, "narrow width must wrap");
    }

    #[test]
    fn alignment_is_applied() {
        const W: f32 = 1000.0;
        let mut resource = Resource::new_test();
        let fam = resource.font_registry.placeholder_family_id().unwrap();
        let style_runs = vec![style_run(0, 0..2, fam, 16.0)];
        let measure = |align: Alignment, resource: &mut Resource| -> (f32, f32) {
            let layout = build_layout("ab", &style_runs, align, 0.0, W, resource, &[]);
            // layout_max_advance() returns the container width (W), not the text content advance.
            // Use the first line's advance minus trailing whitespace as the actual content width.
            let content_width = layout
                .lines()
                .next()
                .map(|l| {
                    let m = l.metrics();
                    m.advance - m.trailing_whitespace
                })
                .unwrap_or(0.0);
            let first_offset = layout
                .lines()
                .flat_map(|l| l.items())
                .find_map(|it| match it {
                    parley::PositionedLayoutItem::GlyphRun(gr) => Some(gr.offset()),
                    _ => None,
                })
                .unwrap_or(0.0);
            (first_offset, content_width)
        };
        let (left, content_width) = measure(Alignment::Left, &mut resource);
        let (center, _) = measure(Alignment::Center, &mut resource);
        assert!(
            content_width < W,
            "layout width {W} must exceed content {content_width} for centering to have free space"
        );
        assert!(
            center > left,
            "centered text starts right of left-aligned (center={center}, left={left})"
        );
    }

    #[test]
    fn family_resolves_through_registry() {
        let mut resource = Resource::new_test();
        let fam = resource.font_registry.placeholder_family_id().unwrap();
        let style_runs = vec![style_run(0, 0..3, fam, 16.0)];
        let layout = build_layout(
            "xyz",
            &style_runs,
            Alignment::Left,
            0.0,
            1.0e6,
            &mut resource,
            &[],
        );
        let glyph_runs = layout
            .lines()
            .flat_map(|l| l.items())
            .filter(|it| matches!(it, parley::PositionedLayoutItem::GlyphRun(_)))
            .count();
        assert!(
            glyph_runs >= 1,
            "registered family must shape into ≥1 glyph run"
        );
    }
}
