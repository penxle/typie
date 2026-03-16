use crate::global::TextBrush;
use crate::model::Style;
use parley::style::*;
use std::borrow::Cow;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrutMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub font_size: f32,
}

pub fn measure_strut(
    lcx: &mut parley::LayoutContext<TextBrush>,
    fcx: &mut parley::FontContext,
    family: &str,
    weight: u16,
    font_size_px: f32,
    line_height: f32,
) -> StrutMetrics {
    measure_strut_inner(
        lcx,
        fcx,
        family,
        weight,
        font_size_px,
        line_height,
        None,
        0,
        |_builder, _style, _range, _font_size| {},
    )
}

pub fn measure_strut_with_styles<ApplyStyle>(
    lcx: &mut parley::LayoutContext<TextBrush>,
    fcx: &mut parley::FontContext,
    family: &str,
    weight: u16,
    font_size_px: f32,
    line_height: f32,
    extra_styles: &[Style],
    extra_style_default_font_size: u32,
    apply_style: ApplyStyle,
) -> StrutMetrics
where
    ApplyStyle: for<'a> FnMut(&mut parley::RangedBuilder<'a, TextBrush>, &Style, Range<usize>, u32),
{
    measure_strut_inner(
        lcx,
        fcx,
        family,
        weight,
        font_size_px,
        line_height,
        Some(extra_styles),
        extra_style_default_font_size,
        apply_style,
    )
}

fn measure_strut_inner<ApplyStyle>(
    lcx: &mut parley::LayoutContext<TextBrush>,
    fcx: &mut parley::FontContext,
    family: &str,
    weight: u16,
    font_size_px: f32,
    line_height: f32,
    extra_styles: Option<&[Style]>,
    extra_style_default_font_size: u32,
    mut apply_style: ApplyStyle,
) -> StrutMetrics
where
    ApplyStyle: for<'a> FnMut(&mut parley::RangedBuilder<'a, TextBrush>, &Style, Range<usize>, u32),
{
    let mut dummy_builder = lcx.ranged_builder(fcx, "\u{200B}", 1.0, false);
    dummy_builder.push_default(StyleProperty::FontFamily(FontFamily::Single(
        FontFamilyName::Named(family.to_string().into()),
    )));
    dummy_builder.push_default(StyleProperty::FontWeight(FontWeight::new(weight as f32)));
    dummy_builder.push_default(StyleProperty::FontSize(font_size_px));
    dummy_builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(
        line_height,
    )));
    dummy_builder.push_default(StyleProperty::FontFeatures(FontFeatures::Source(
        Cow::Owned("\"ss05\" 1, \"cv12\" 1, \"ss18\" 1".to_string()),
    )));

    if let Some(styles) = extra_styles {
        let range = 0.."\u{200B}".len();
        let font_size = styles
            .iter()
            .find_map(|style| {
                if let Style::FontSize(font_size) = style {
                    Some(font_size.size)
                } else {
                    None
                }
            })
            .unwrap_or(extra_style_default_font_size);
        for style in styles {
            apply_style(&mut dummy_builder, style, range.clone(), font_size);
        }
    }

    let mut dummy_layout = dummy_builder.build("\u{200B}");
    dummy_layout.break_all_lines(None);
    let dummy_line = dummy_layout.lines().next().unwrap();
    let dummy_metrics = dummy_line.metrics();
    let dummy_font_size = dummy_line
        .items()
        .find_map(|item| match item {
            parley::PositionedLayoutItem::GlyphRun(glyph_run) => Some(glyph_run.run().font_size()),
            _ => None,
        })
        .unwrap_or(font_size_px);

    StrutMetrics {
        ascent: dummy_metrics.ascent,
        descent: dummy_metrics.descent,
        font_size: dummy_font_size,
    }
}
