use std::borrow::Cow;

use editor_resource::{PLACEHOLDER_FAMILY_NAME, Resource};
use parley::style::{FontFamily, FontFamilyName, FontWeight, LineHeight, TextStyle};

use super::resolve::ResolvedTextStyle;

pub struct StrutMetrics {
    pub ascent: f32,
    pub descent: f32,
}

pub fn compute_strut(resource: &mut Resource, style: &ResolvedTextStyle) -> Option<StrutMetrics> {
    if let Some(m) = strut_for_family(resource, style, &style.font_family) {
        return Some(m);
    }
    strut_for_family(resource, style, PLACEHOLDER_FAMILY_NAME)
}

fn strut_for_family(
    resource: &mut Resource,
    style: &ResolvedTextStyle,
    family_name: &str,
) -> Option<StrutMetrics> {
    let text = " ";
    let mut builder =
        resource
            .layout_context
            .style_run_builder(&mut resource.font_context, text, 1.0, true);

    let parley_style = TextStyle {
        font_size: style.font_size,
        font_weight: FontWeight::new(style.font_weight as f32),
        font_family: FontFamily::Single(FontFamilyName::Named(Cow::Owned(family_name.to_string()))),
        line_height: LineHeight::Absolute(style.font_size),
        ..TextStyle::default()
    };

    let idx = builder.push_style(parley_style);
    builder.push_style_run(idx, 0..text.len());

    let mut layout = builder.build(text);
    layout.break_all_lines(None);

    let line = layout.lines().next()?;
    let run = line.runs().next()?;
    let metrics = run.metrics();

    Some(StrutMetrics {
        ascent: metrics.ascent,
        descent: metrics.descent,
    })
}

#[cfg(test)]
mod tests {
    use editor_resource::Resource;

    use super::super::resolve::ResolvedTextStyle;
    use super::*;

    #[test]
    fn falls_back_to_placeholder_when_user_family_unregistered() {
        let mut resource = Resource::new_test();
        let style = ResolvedTextStyle {
            font_family: "UnregisteredFamily".into(),
            font_weight: 400,
            font_size: 16.0,
            letter_spacing: 0.0,
            line_height: 1.0,
        };

        let strut = compute_strut(&mut resource, &style);
        assert!(strut.is_some(), "strut should fall back to placeholder");
        let strut = strut.unwrap();
        assert!(strut.ascent > 0.0 && strut.descent > 0.0);
    }
}
