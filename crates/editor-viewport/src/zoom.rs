use crate::layout::LayoutMode;
use crate::rounding::effective_zoom;

const CONTINUOUS_HORIZONTAL_MARGIN: f64 = 20.0;
const MIN_DOCUMENT_DISPLAY_WIDTH: f64 = 100.0;
const MIN_ZOOM_FLOOR: f64 = 0.01;
const MAX_DOCUMENT_ZOOM: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomBounds {
    pub min: f64,
    pub max: f64,
}

impl ZoomBounds {
    pub fn clamp(&self, zoom: f64) -> f64 {
        if zoom.is_finite() {
            zoom.clamp(self.min, self.max)
        } else {
            self.min
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomKey {
    pub paginated: bool,
    pub width: f64,
}

pub fn document_zoom_width(mode: &LayoutMode) -> f64 {
    let width = match *mode {
        LayoutMode::Paginated { page_width, .. } => page_width,
        LayoutMode::Continuous { max_width } => max_width + CONTINUOUS_HORIZONTAL_MARGIN * 2.0,
    };
    if width.is_finite() && width > 0.0 {
        width
    } else {
        1.0
    }
}

pub fn zoom_bounds(mode: &LayoutMode) -> ZoomBounds {
    let min = (MIN_DOCUMENT_DISPLAY_WIDTH / document_zoom_width(mode)).max(MIN_ZOOM_FLOOR);
    ZoomBounds {
        min,
        max: MAX_DOCUMENT_ZOOM.max(min),
    }
}

pub fn initial_zoom(mode: &LayoutMode, viewport_width: f64) -> f64 {
    let bounds = zoom_bounds(mode);
    match mode {
        LayoutMode::Paginated { .. } => {
            let width = document_zoom_width(mode);
            let viewport_width = if viewport_width.is_finite() && viewport_width > 0.0 {
                viewport_width
            } else {
                width
            };
            bounds.clamp(viewport_width / width).min(1.0)
        }
        LayoutMode::Continuous { .. } => bounds.clamp(1.0),
    }
}

pub fn zoom_key(mode: &LayoutMode) -> ZoomKey {
    ZoomKey {
        paginated: matches!(mode, LayoutMode::Paginated { .. }),
        width: document_zoom_width(mode),
    }
}

pub fn continuous_engine_width(viewport_width: f64, zoom: f64) -> f64 {
    let width = if viewport_width.is_finite() && viewport_width > 0.0 {
        viewport_width
    } else {
        1.0
    };
    width / effective_zoom(zoom).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGINATED: LayoutMode = LayoutMode::Paginated {
        page_width: 720.0,
        page_height: 960.0,
        margin_top: 72.0,
        margin_bottom: 72.0,
        margin_left: 64.0,
        margin_right: 64.0,
    };
    const CONTINUOUS: LayoutMode = LayoutMode::Continuous { max_width: 600.0 };

    #[test]
    fn document_zoom_width_adds_the_continuous_margins() {
        assert_eq!(document_zoom_width(&CONTINUOUS), 640.0);
        assert_eq!(document_zoom_width(&PAGINATED), 720.0);
        assert_eq!(
            document_zoom_width(&LayoutMode::Continuous {
                max_width: f64::NAN
            }),
            1.0
        );
        assert_eq!(
            document_zoom_width(&LayoutMode::Paginated {
                page_width: 0.0,
                page_height: 960.0,
                margin_top: 0.0,
                margin_bottom: 0.0,
                margin_left: 0.0,
                margin_right: 0.0,
            }),
            1.0
        );
    }

    #[test]
    fn zoom_bounds_follow_the_document_zoom_width() {
        assert_eq!(
            zoom_bounds(&CONTINUOUS),
            ZoomBounds {
                min: 0.15625,
                max: 2.0
            }
        );
        assert_eq!(
            zoom_bounds(&LayoutMode::Continuous { max_width: 10.0 }),
            ZoomBounds { min: 2.0, max: 2.0 }
        );
        assert_eq!(zoom_bounds(&CONTINUOUS).clamp(f64::NAN), 0.15625);
        assert_eq!(zoom_bounds(&CONTINUOUS).clamp(3.0), 2.0);
    }

    #[test]
    fn paginated_initial_zoom_fits_the_viewport_width() {
        assert_eq!(initial_zoom(&PAGINATED, 360.0), 0.5);
        assert_eq!(initial_zoom(&PAGINATED, 960.0), 1.0);
        assert_eq!(initial_zoom(&PAGINATED, 0.0), 1.0);
        assert_eq!(initial_zoom(&PAGINATED, 50.0), 100.0 / 720.0);
    }

    #[test]
    fn continuous_initial_zoom_is_unit_within_bounds() {
        assert_eq!(initial_zoom(&CONTINUOUS, 960.0), 1.0);
        assert_eq!(initial_zoom(&CONTINUOUS, 200.0), 1.0);
        assert_eq!(
            initial_zoom(&LayoutMode::Continuous { max_width: 30.0 }, 390.0),
            100.0 / 70.0
        );
    }

    #[test]
    fn zoom_key_changes_only_with_the_layout_kind_and_width() {
        assert_eq!(zoom_key(&CONTINUOUS), zoom_key(&CONTINUOUS));
        assert_ne!(
            zoom_key(&CONTINUOUS),
            zoom_key(&LayoutMode::Continuous { max_width: 610.0 })
        );
        let paginated_640 = LayoutMode::Paginated {
            page_width: 640.0,
            page_height: 960.0,
            margin_top: 72.0,
            margin_bottom: 72.0,
            margin_left: 64.0,
            margin_right: 64.0,
        };
        assert_eq!(
            document_zoom_width(&paginated_640),
            document_zoom_width(&CONTINUOUS)
        );
        assert_ne!(zoom_key(&paginated_640), zoom_key(&CONTINUOUS));
    }

    #[test]
    fn continuous_engine_width_widens_below_unit_zoom() {
        assert_eq!(continuous_engine_width(500.0, 1.5), 500.0);
        assert_eq!(continuous_engine_width(500.0, 0.8), 625.0);
        assert_eq!(continuous_engine_width(500.0, f64::NAN), 500.0);
        assert_eq!(continuous_engine_width(0.0, 1.0), 1.0);
    }
}
