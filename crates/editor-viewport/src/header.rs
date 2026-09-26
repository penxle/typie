use crate::layout::LayoutMode;

const HEADER_CONTENT_INSET: f64 = 20.0;
const HEADER_MIN_CONTENT_WIDTH: f64 = 320.0;
const HEADER_MIN_SCALE: f64 = 0.75;
const HEADER_VIEWPORT_GAP: f64 = 20.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeaderTrack {
    pub track_width: f64,
    pub content_width: f64,
    pub track_left: f64,
    pub content_left: f64,
    pub content_right: f64,
    pub field_width: f64,
    pub max_scroll_offset: f64,
}

impl HeaderTrack {
    pub fn field_screen_left(&self, scroll_x: f64) -> f64 {
        let scroll =
            if scroll_x.is_finite() { scroll_x } else { 0.0 }.clamp(0.0, self.max_scroll_offset);
        let field_left = (scroll + HEADER_VIEWPORT_GAP).clamp(
            self.content_left,
            self.content_left.max(self.content_right - self.field_width),
        );
        field_left - scroll
    }
}

pub fn header_track(
    mode: &LayoutMode,
    viewport_width: f64,
    body_track_width: f64,
    zoom: f64,
) -> Option<HeaderTrack> {
    let valid = |value: f64| value.is_finite() && value > 0.0;
    if !valid(viewport_width) || !valid(body_track_width) || !valid(zoom) {
        return None;
    }
    let (inset_left, inset_right) = match *mode {
        LayoutMode::Paginated {
            margin_left,
            margin_right,
            ..
        } => (margin_left, margin_right),
        LayoutMode::Continuous { .. } => (HEADER_CONTENT_INSET, HEADER_CONTENT_INSET),
    };
    let scaled_inset = |inset: f64| {
        let scaled = inset * zoom;
        if scaled.is_finite() {
            scaled.max(0.0)
        } else {
            0.0
        }
    };
    let left_inset = scaled_inset(inset_left);
    let right_inset = scaled_inset(inset_right);
    let readable_min =
        (HEADER_MIN_CONTENT_WIDTH * zoom).max(HEADER_MIN_CONTENT_WIDTH * HEADER_MIN_SCALE);
    let capacity = (viewport_width - left_inset - right_inset).max(0.0);
    let track_width = body_track_width.max(left_inset + right_inset + readable_min.min(capacity));
    let content_width = viewport_width.max(track_width);
    let track_left = (content_width - track_width) / 2.0;
    let content_left = track_left + left_inset;
    let content_right = content_left.max(track_left + track_width - right_inset);
    let field_width =
        (content_right - content_left).min((viewport_width - HEADER_VIEWPORT_GAP * 2.0).max(0.0));
    Some(HeaderTrack {
        track_width,
        content_width,
        track_left,
        content_left,
        content_right,
        field_width,
        max_scroll_offset: (content_width - viewport_width).max(0.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn paginated(margin_left: f64, margin_right: f64) -> LayoutMode {
        LayoutMode::Paginated {
            page_width: 720.0,
            page_height: 960.0,
            margin_top: 72.0,
            margin_bottom: 72.0,
            margin_left,
            margin_right,
        }
    }

    #[test]
    fn continuous_header_aligns_with_the_body_inset() {
        let track = header_track(
            &LayoutMode::Continuous { max_width: 600.0 },
            720.0,
            640.0,
            1.0,
        )
        .unwrap();
        assert_eq!(track.field_width, 600.0);
        assert_eq!(track.field_screen_left(0.0), 60.0);
    }

    #[test]
    fn paginated_header_keeps_a_readable_width_at_low_zoom() {
        let track = header_track(&paginated(64.0, 64.0), 320.0, 180.0, 0.25).unwrap();
        assert_eq!(track.field_width, 240.0);
        assert_eq!(track.field_screen_left(0.0), 40.0);
    }

    #[test]
    fn paginated_header_follows_horizontal_scroll_within_the_content_bounds() {
        let track = header_track(&paginated(64.0, 64.0), 320.0, 1440.0, 2.0).unwrap();
        assert_eq!(track.field_width, 280.0);
        assert_eq!(track.max_scroll_offset, 1120.0);
        assert_eq!(track.field_screen_left(0.0), 128.0);
        assert_eq!(track.field_screen_left(300.0), 20.0);
        assert_eq!(track.field_screen_left(1120.0), -88.0);
        let asymmetric = header_track(&paginated(64.0, 96.0), 320.0, 1440.0, 2.0).unwrap();
        assert_eq!(asymmetric.field_screen_left(1120.0), -152.0);
    }

    #[test]
    fn header_track_requires_a_positive_viewport() {
        assert_eq!(header_track(&paginated(64.0, 64.0), 0.0, 720.0, 1.0), None);
        assert_eq!(header_track(&paginated(64.0, 64.0), 320.0, 0.0, 1.0), None);
        assert_eq!(
            header_track(&paginated(64.0, 64.0), 320.0, 720.0, f64::NAN),
            None
        );
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

        #[test]
        fn header_field_stays_inside_the_track_and_the_viewport(
            viewport_width in 1.0..1400.0f64,
            body_track_width in 1.0..3000.0f64,
            zoom in 0.05..3.0f64,
            margin_left in 0.0..150.0f64,
            margin_right in 0.0..150.0f64,
            scroll_x in -200.0..4000.0f64,
        ) {
            let track = header_track(&paginated(margin_left, margin_right), viewport_width, body_track_width, zoom).unwrap();
            prop_assert!(track.track_width >= body_track_width);
            prop_assert!(track.field_width >= 0.0);
            prop_assert!(track.field_width <= track.content_right - track.content_left + 1e-9);
            prop_assert!(track.field_width <= (viewport_width - 40.0).max(0.0) + 1e-9);
            let scroll = scroll_x.clamp(0.0, track.max_scroll_offset);
            let left = track.field_screen_left(scroll_x) + scroll;
            prop_assert!(left >= track.content_left - 1e-9);
            prop_assert!(left <= track.content_left.max(track.content_right - track.field_width) + 1e-9);
        }
    }
}
