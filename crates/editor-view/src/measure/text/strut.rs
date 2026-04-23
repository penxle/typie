use std::borrow::Cow;

use editor_resource::{PLACEHOLDER_FAMILY_NAME, PLACEHOLDER_WEIGHT, Resolution, Resource};
use parley::style::{FontFamily, FontFamilyName, FontWeight, LineHeight, TextStyle};

use super::resolve::ResolvedTextStyle;

pub struct StrutMetrics {
    pub ascent: f32,
    pub descent: f32,
}

pub fn compute_strut(resource: &mut Resource, style: &ResolvedTextStyle) -> Option<StrutMetrics> {
    let requested_family_id = resource.font_registry.intern(&style.font_family);
    let placeholder_id = resource.font_registry.placeholder_family_id()?;

    let (family_id, weight) =
        match resource
            .font_registry
            .resolve(requested_family_id, style.font_weight, ' ' as u32)
        {
            Resolution::Ready(target) => (target.family_id, target.weight),
            Resolution::Pending {
                target,
                needs_base: false,
            } => (target.family_id, target.weight),
            Resolution::Pending {
                needs_base: true, ..
            }
            | Resolution::Missing => (placeholder_id, PLACEHOLDER_WEIGHT),
        };

    let family_name = resource
        .font_registry
        .family_name_opt(family_id)
        .unwrap_or(PLACEHOLDER_FAMILY_NAME)
        .to_owned();

    strut_for_family(resource, style.font_size, weight, &family_name)
}

fn strut_for_family(
    resource: &mut Resource,
    font_size: f32,
    weight: u16,
    family_name: &str,
) -> Option<StrutMetrics> {
    let text = " ";
    let mut builder =
        resource
            .layout_context
            .style_run_builder(&mut resource.font_context, text, 1.0, true);

    let parley_style = TextStyle {
        font_size,
        font_weight: FontWeight::new(weight as f32),
        font_family: FontFamily::Single(FontFamilyName::Named(Cow::Owned(family_name.to_string()))),
        line_height: LineHeight::Absolute(font_size),
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
