use editor_model::Alignment;
use editor_resource::{Resource, TextBrush};
use parley::style::{
    FontFamily, FontFamilyName, FontFeatures, FontVariations, FontWeight, LineHeight, TextStyle,
};
use parley::{
    Alignment as ParleyAlignment, AlignmentOptions, IndentOptions, InlineBox, InlineBoxKind,
    Layout, OverflowWrap, WordBreak,
};
use std::borrow::Cow;

use super::style_run::StyleRun;

pub fn build_layout(
    text: &str,
    style_runs: &[StyleRun],
    align: Alignment,
    indent: f32,
    width: f32,
    resource: &mut Resource,
    tabs: &[(super::text_run::TabMark, f32)],
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
                node_id: style_run.node_id,
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
