pub fn effective_zoom(zoom: f64) -> f64 {
    if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    }
}

pub(crate) fn effective_scale(scale: f64) -> f64 {
    if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    }
}

pub(crate) fn length_px(length: f64, zoom: f64, scale: f64) -> f64 {
    let pixels = (length * (effective_zoom(zoom) * scale)).round();
    if pixels.is_finite() {
        pixels.max(1.0)
    } else {
        1.0
    }
}

pub fn page_length(length: f64, zoom: f64, scale: f64) -> f64 {
    let scale = effective_scale(scale);
    length_px(length, zoom, scale) / scale
}

pub(crate) fn stack_pages(
    origin: f64,
    lengths: impl IntoIterator<Item = f64>,
    gap: f64,
) -> Vec<(f64, f64)> {
    let mut spans: Vec<(f64, f64)> = Vec::new();
    let mut top = origin;
    for length in lengths {
        if !spans.is_empty() {
            top += gap;
        }
        let bottom = top + length;
        spans.push((top, bottom));
        top = bottom;
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_length_rounds_half_away_from_zero_with_one_pixel_minimum() {
        assert_eq!(page_length(223.0, 0.75, 2.0), 167.5);
        assert_eq!(page_length(0.0, 1.0, 3.0), 1.0 / 3.0);
        assert_eq!(page_length(0.2, 1.0, 1.0), 1.0);
        assert_eq!(page_length(f64::NAN, 1.0, 2.0), 0.5);
        assert_eq!(page_length(100.0, f64::NAN, 1.0), 100.0);
        assert_eq!(page_length(100.0, 0.0, 1.0), 100.0);
    }

    #[test]
    fn stacks_rounded_page_lengths_from_the_origin() {
        let length = page_length(223.0, 0.75, 2.0);
        assert_eq!(
            stack_pages(10.0, [length; 3], 9.0),
            vec![(10.0, 177.5), (186.5, 354.0), (363.0, 530.5)],
        );
        assert!(stack_pages(10.0, [], 9.0).is_empty());
    }

    #[test]
    fn effective_zoom_falls_back_to_unit() {
        assert_eq!(effective_zoom(1.5), 1.5);
        assert_eq!(effective_zoom(0.0), 1.0);
        assert_eq!(effective_zoom(-2.0), 1.0);
        assert_eq!(effective_zoom(f64::INFINITY), 1.0);
    }
}
