use std::borrow::Cow;

use editor_resource::Resource;
use parley::style::{FontFamily, FontFamilyName, FontWeight, LineHeight, TextStyle};

use super::resolve::ResolvedTextStyle;

pub struct StrutMetrics {
    pub ascent: f32,
    pub descent: f32,
}

pub fn compute_strut(resource: &mut Resource, style: &ResolvedTextStyle) -> StrutMetrics {
    let text = " ";
    let mut builder =
        resource
            .layout_context
            .style_run_builder(&mut resource.font_context, text, 1.0, true);

    let style = TextStyle {
        font_size: style.font_size,
        font_weight: FontWeight::new(style.font_weight as f32),
        font_family: FontFamily::Single(FontFamilyName::Named(Cow::Owned(
            style.font_family.clone(),
        ))),
        line_height: LineHeight::Absolute(style.font_size),
        ..TextStyle::default()
    };

    let idx = builder.push_style(style);
    builder.push_style_run(idx, 0..text.len());

    let mut layout = builder.build(text);
    layout.break_all_lines(None);

    let line = layout
        .lines()
        .next()
        .expect("strut layout should have one line");

    let run = line
        .runs()
        .next()
        .expect("strut layout should have one run");

    let metrics = run.metrics();

    StrutMetrics {
        ascent: metrics.ascent,
        descent: metrics.descent,
    }
}
