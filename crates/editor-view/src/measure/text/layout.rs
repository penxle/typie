use editor_model::TextAlign;
use editor_resource::{Resource, TextBrush};
use parley::style::{
    FontFamily, FontFamilyName, FontFeatures, FontVariations, FontWeight, LineHeight, TextStyle,
};
use parley::{Alignment, AlignmentOptions, IndentOptions, Layout};
use std::borrow::Cow;

use super::style_run::StyleRun;

pub fn build_layout(
    text: &str,
    style_runs: &[StyleRun],
    align: TextAlign,
    indent: f32,
    width: f32,
    resource: &mut Resource,
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
                .resolve_opt(sr.family)
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
            font_features: FontFeatures::empty(),
            ..TextStyle::default()
        };

        let idx = builder.push_style(style);
        builder.push_style_run(idx, style_run.byte_range.clone());
    }

    let mut layout = builder.build(text);

    if indent > 0.0 {
        layout.set_text_indent(indent, IndentOptions::default());
    }

    layout.break_all_lines(Some(width));

    let alignment = match align {
        TextAlign::Left => Alignment::Start,
        TextAlign::Center => Alignment::Center,
        TextAlign::Right => Alignment::End,
        TextAlign::Justify => Alignment::Justify,
    };
    layout.align(Some(width), alignment, AlignmentOptions::default());

    layout
}
