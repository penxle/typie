use fontique::{Attributes, FontStyle, FontWeight, FontWidth, QueryFamily, QueryStatus};
use skrifa::instance::{LocationRef, Size as SkrifaSize};
use skrifa::{FontRef, MetadataProvider};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrutMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub font_size: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct StrutRequest<'a> {
    pub family: &'a str,
    pub weight: u16,
    pub font_size_px: f32,
    pub style: FontStyle,
}

pub fn measure_strut(fcx: &mut parley::FontContext, request: StrutRequest<'_>) -> StrutMetrics {
    measure_font_metrics(fcx, request).unwrap_or(StrutMetrics {
        ascent: 0.0,
        descent: 0.0,
        font_size: request.font_size_px,
    })
}

fn measure_font_metrics(
    fcx: &mut parley::FontContext,
    request: StrutRequest<'_>,
) -> Option<StrutMetrics> {
    let mut query = fcx.collection.query(&mut fcx.source_cache);
    query.set_families([QueryFamily::Named(request.family)]);
    query.set_attributes(Attributes::new(
        FontWidth::NORMAL,
        request.style,
        FontWeight::new(request.weight as f32),
    ));

    let mut measured = None;
    query.matches_with(|font| {
        measured = font_metrics_from_query(font, request.font_size_px);
        if measured.is_some() {
            QueryStatus::Stop
        } else {
            QueryStatus::Continue
        }
    });

    measured
}

fn font_metrics_from_query(font: &fontique::QueryFont, font_size_px: f32) -> Option<StrutMetrics> {
    let font_ref = FontRef::from_index(font.blob.as_ref(), font.index).ok()?;
    let location = font_ref
        .axes()
        .location(font.synthesis.variation_settings().iter().copied());
    let metrics = skrifa::metrics::Metrics::new(
        &font_ref,
        SkrifaSize::new(font_size_px),
        LocationRef::new(location.coords()),
    );

    Some(StrutMetrics {
        ascent: metrics.ascent.max(0.0),
        descent: (-metrics.descent).max(0.0),
        font_size: font_size_px,
    })
}
