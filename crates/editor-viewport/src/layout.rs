use std::ops::Range;

use crate::rounding::{effective_scale, effective_zoom, length_px, stack_pages};

const CONTINUOUS_TOP_SPACER: f64 = 40.0;
const PAGINATED_PAGE_GAP: f64 = 24.0;
const CONTINUOUS_BASE_BOTTOM_SPACE: f64 = 20.0;
const SCROLL_PAST_END_FRACTION: f64 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LayoutMode {
    Paginated {
        page_width: f64,
        page_height: f64,
        margin_top: f64,
        margin_bottom: f64,
        margin_left: f64,
        margin_right: f64,
    },
    Continuous {
        max_width: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageExtent {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub width: f64,
    pub height: f64,
    pub occlusion_top: f64,
    pub occlusion_bottom: f64,
    pub device_scale: f64,
}

impl Viewport {
    pub fn visible_top(&self) -> f64 {
        self.occlusion_top
    }

    pub fn visible_bottom(&self) -> f64 {
        (self.height - self.occlusion_bottom)
            .max(0.0)
            .max(self.visible_top())
    }

    pub fn bottom_occlusion(&self) -> f64 {
        (self.height - self.visible_bottom()).max(0.0)
    }

    pub fn visible_height(&self) -> f64 {
        (self.visible_bottom() - self.visible_top()).max(0.0)
    }

    fn sanitized(self) -> Self {
        Self {
            width: finite_or_zero(self.width),
            height: finite_or_zero(self.height),
            occlusion_top: finite_or_zero(self.occlusion_top),
            occlusion_bottom: finite_or_zero(self.occlusion_bottom),
            device_scale: effective_scale(self.device_scale),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Rect {
    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    pub fn height(&self) -> f64 {
        self.bottom - self.top
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PlacementInput<'a> {
    pub mode: &'a LayoutMode,
    pub pages: &'a [PageExtent],
    pub header_height: f64,
    pub viewport: Viewport,
    pub zoom: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContentPlacement {
    pub viewport: Viewport,
    pub zoom: f64,
    pub header_height: f64,
    pub top_spacer: f64,
    pub pages_top: f64,
    pub pages_height: f64,
    pub column_width: f64,
    pub content_width: f64,
    pub minimum_body_height: f64,
    pub bottom_padding: f64,
    pub content_height: f64,
    pub pages: Vec<Rect>,
}

impl ContentPlacement {
    pub fn new(input: PlacementInput<'_>) -> Self {
        let viewport = input.viewport.sanitized();
        let scale = viewport.device_scale;
        let zoom = effective_zoom(input.zoom);
        let header_height = finite_or_zero(input.header_height);
        let top_spacer = match input.mode {
            LayoutMode::Paginated { .. } => 0.0,
            LayoutMode::Continuous { .. } => CONTINUOUS_TOP_SPACER,
        };
        let pages_top_px = ((header_height + top_spacer) * scale).round();
        let gap_px = gap_px(input.mode, zoom, scale);
        let column_width = column_px(input.mode, input.pages, viewport.width, zoom, scale) / scale;
        let content_width = viewport.width.max(column_width).max(0.0);
        let spans = stack_pages(
            pages_top_px,
            input
                .pages
                .iter()
                .map(|page| length_px(page.height, zoom, scale)),
            gap_px,
        );
        let pages: Vec<Rect> = input
            .pages
            .iter()
            .zip(spans)
            .map(|(page, (top_px, bottom_px))| {
                let width_px = length_px(page.width, zoom, scale);
                let left_px = ((content_width * scale - width_px) / 2.0).max(0.0).round();
                Rect {
                    left: left_px / scale,
                    top: top_px / scale,
                    right: (left_px + width_px) / scale,
                    bottom: bottom_px / scale,
                }
            })
            .collect();
        let pages_top = pages_top_px / scale;
        let pages_bottom = pages.last().map_or(pages_top, |page| page.bottom);
        let minimum_body_height =
            (viewport.height - header_height - viewport.bottom_occlusion()).max(0.0);
        let bottom_padding = match input.mode {
            LayoutMode::Paginated { .. } => viewport.bottom_occlusion(),
            LayoutMode::Continuous { .. } => (viewport.bottom_occlusion()
                + viewport.visible_height() * SCROLL_PAST_END_FRACTION
                - CONTINUOUS_BASE_BOTTOM_SPACE * zoom)
                .max(0.0),
        };
        let content_height =
            (header_height + minimum_body_height).max(pages_bottom + bottom_padding);
        Self {
            viewport,
            zoom,
            header_height,
            top_spacer,
            pages_top,
            pages_height: pages_bottom - pages_top,
            column_width,
            content_width,
            minimum_body_height,
            bottom_padding,
            content_height,
            pages,
        }
    }

    pub fn raster_scale(&self) -> f64 {
        self.zoom * self.viewport.device_scale
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn page_rect(&self, index: usize) -> Option<Rect> {
        self.pages.get(index).copied()
    }

    pub fn pages_bottom(&self) -> f64 {
        self.pages.last().map_or(self.pages_top, |page| page.bottom)
    }

    pub fn minimum_body_bottom(&self) -> f64 {
        self.header_height + self.minimum_body_height
    }

    pub fn pages_in(&self, top: f64, bottom: f64) -> Range<usize> {
        pages_overlapping(&self.pages, top, bottom)
    }

    pub fn page_local_to_content(&self, index: usize, x: f64, y: f64) -> Option<(f64, f64)> {
        let page = self.pages.get(index)?;
        Some((page.left + x * self.zoom, page.top + y * self.zoom))
    }

    pub fn max_scroll_x(&self) -> f64 {
        (self.content_width - self.viewport.width).max(0.0)
    }

    pub fn max_scroll_y(&self) -> f64 {
        (self.content_height - self.viewport.height).max(0.0)
    }

    pub fn clamp_scroll_x(&self, scroll_x: f64) -> f64 {
        clamp_scroll(scroll_x, self.max_scroll_x())
    }

    pub fn clamp_scroll_y(&self, scroll_y: f64) -> f64 {
        clamp_scroll(scroll_y, self.max_scroll_y())
    }
}

pub fn page_gap(mode: &LayoutMode, zoom: f64, scale: f64) -> f64 {
    let scale = effective_scale(scale);
    gap_px(mode, effective_zoom(zoom), scale) / scale
}

pub fn pages_overlapping(pages: &[Rect], top: f64, bottom: f64) -> Range<usize> {
    if top.is_nan() || bottom.is_nan() || top >= bottom {
        return 0..0;
    }
    let start = pages.partition_point(|page| page.bottom <= top);
    let end = pages.partition_point(|page| page.top < bottom);
    start..end.max(start)
}

fn gap_px(mode: &LayoutMode, zoom: f64, scale: f64) -> f64 {
    match mode {
        LayoutMode::Paginated { .. } => (PAGINATED_PAGE_GAP * zoom * scale).round().max(0.0),
        LayoutMode::Continuous { .. } => 0.0,
    }
}

fn column_px(
    mode: &LayoutMode,
    pages: &[PageExtent],
    viewport_width: f64,
    zoom: f64,
    scale: f64,
) -> f64 {
    let widest = pages
        .iter()
        .map(|page| page.width)
        .filter(|width| width.is_finite() && *width > 0.0)
        .reduce(f64::max);
    match *mode {
        LayoutMode::Continuous { .. } => match widest {
            Some(width) => length_px(width, zoom, scale),
            None => (viewport_width * (zoom * scale)).round(),
        },
        LayoutMode::Paginated { page_width, .. } => {
            if page_width.is_finite() && page_width > 0.0 {
                length_px(page_width, zoom, scale)
            } else if let Some(width) = widest {
                length_px(width, zoom, scale)
            } else {
                (viewport_width * scale).round()
            }
        }
    }
}

fn clamp_scroll(scroll: f64, maximum: f64) -> f64 {
    if scroll.is_finite() {
        scroll.clamp(0.0, maximum)
    } else {
        0.0
    }
}

fn finite_or_zero(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const PAGINATED: LayoutMode = LayoutMode::Paginated {
        page_width: 720.0,
        page_height: 960.0,
        margin_top: 72.0,
        margin_bottom: 72.0,
        margin_left: 64.0,
        margin_right: 64.0,
    };

    fn viewport(width: f64, height: f64, occlusion_top: f64, occlusion_bottom: f64) -> Viewport {
        Viewport {
            width,
            height,
            occlusion_top,
            occlusion_bottom,
            device_scale: 1.0,
        }
    }

    fn extents(sizes: &[(f64, f64)]) -> Vec<PageExtent> {
        sizes
            .iter()
            .map(|&(width, height)| PageExtent { width, height })
            .collect()
    }

    fn place(
        mode: LayoutMode,
        pages: &[PageExtent],
        header_height: f64,
        viewport: Viewport,
        zoom: f64,
    ) -> ContentPlacement {
        ContentPlacement::new(PlacementInput {
            mode: &mode,
            pages,
            header_height,
            viewport,
            zoom,
        })
    }

    #[test]
    fn continuous_column_uses_the_engine_page_width() {
        let placement = place(
            LayoutMode::Continuous { max_width: 600.0 },
            &extents(&[(640.0, 800.0)]),
            180.0,
            viewport(720.0, 900.0, 120.0, 100.0),
            1.0,
        );
        assert_eq!(placement.column_width, 640.0);
        assert_eq!(placement.minimum_body_height, 620.0);
        assert_eq!(placement.top_spacer, 40.0);
        assert_eq!(placement.content_width, 720.0);
        assert_eq!(
            placement.page_rect(0),
            Some(Rect {
                left: 40.0,
                top: 220.0,
                right: 680.0,
                bottom: 1020.0,
            }),
        );
    }

    #[test]
    fn continuous_column_follows_the_engine_page_width_at_the_cap() {
        let placement = place(
            LayoutMode::Continuous { max_width: 600.0 },
            &extents(&[(620.0, 800.0)]),
            120.0,
            viewport(620.0, 900.0, 120.0, 0.0),
            1.0,
        );
        assert_eq!(placement.column_width, 620.0);
    }

    #[test]
    fn continuous_column_scales_the_engine_page_width_by_zoom() {
        let placement = place(
            LayoutMode::Continuous { max_width: 600.0 },
            &extents(&[(500.0, 800.0)]),
            120.0,
            viewport(500.0, 900.0, 120.0, 0.0),
            1.5,
        );
        assert_eq!(placement.column_width, 750.0);
    }

    #[test]
    fn paginated_column_scales_the_layout_page_width_by_zoom() {
        let placement = place(
            PAGINATED,
            &extents(&[(700.0, 960.0)]),
            120.0,
            viewport(960.0, 900.0, 120.0, 0.0),
            1.25,
        );
        assert_eq!(placement.column_width, 900.0);
    }

    #[test]
    fn minimum_body_height_is_clamped_to_zero() {
        let placement = place(
            PAGINATED,
            &extents(&[(720.0, 960.0)]),
            320.0,
            viewport(720.0, 400.0, 0.0, 120.0),
            1.0,
        );
        assert_eq!(placement.minimum_body_height, 0.0);
    }

    #[test]
    fn short_documents_fill_the_viewport_below_the_header() {
        let mode = LayoutMode::Paginated {
            page_width: 400.0,
            page_height: 88.0,
            margin_top: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
            margin_right: 0.0,
        };
        let short = place(
            mode,
            &extents(&[(400.0, 88.0)]),
            100.0,
            viewport(400.0, 500.0, 0.0, 0.0),
            1.0,
        );
        assert_eq!(short.minimum_body_height, 400.0);
        assert_eq!(short.content_height, 500.0);
        let long = place(
            mode,
            &extents(&[(400.0, 420.0)]),
            100.0,
            viewport(400.0, 500.0, 0.0, 0.0),
            1.0,
        );
        assert_eq!(long.content_height, 520.0);
    }

    #[test]
    fn empty_documents_keep_the_viewport_minimum() {
        let placement = place(
            LayoutMode::Continuous { max_width: 600.0 },
            &[],
            72.0,
            viewport(360.0, 640.0, 72.0, 0.0),
            1.0,
        );
        assert_eq!(placement.page_count(), 0);
        assert_eq!(placement.column_width, 360.0);
        assert_eq!(placement.minimum_body_height, 568.0);
        assert_eq!(placement.top_spacer, 40.0);
        assert_eq!(placement.pages_height, 0.0);
        assert_eq!(placement.pages_bottom(), 112.0);
        assert_eq!(placement.content_height, 640.0);
    }

    #[test]
    fn page_gap_rounds_to_device_pixels() {
        assert_eq!(page_gap(&PAGINATED, 1.0, 3.0), 24.0);
        assert_eq!(page_gap(&PAGINATED, 0.49, 3.0), 35.0 / 3.0);
        assert_eq!(page_gap(&PAGINATED, f64::NAN, 2.0), 24.0);
        assert_eq!(
            page_gap(&LayoutMode::Continuous { max_width: 600.0 }, 1.0, 3.0),
            0.0
        );
    }

    #[test]
    fn paginated_pages_stack_below_the_header_with_rounded_gaps() {
        let pages = extents(&[(720.0, 1000.0); 3]);
        let unit = place(
            PAGINATED,
            &pages,
            200.0,
            Viewport {
                device_scale: 3.0,
                ..viewport(720.0, 800.0, 0.0, 0.0)
            },
            1.0,
        );
        let tops: Vec<f64> = unit.pages.iter().map(|page| page.top).collect();
        assert_eq!(tops, vec![200.0, 1224.0, 2248.0]);
        assert_eq!(unit.pages_top, 200.0);
        assert_eq!(unit.pages_height, 3048.0);
        assert_eq!(unit.content_height, 3248.0);

        let zoomed = place(
            PAGINATED,
            &pages,
            200.0,
            Viewport {
                device_scale: 3.0,
                ..viewport(720.0, 800.0, 0.0, 0.0)
            },
            0.49,
        );
        let tops: Vec<f64> = zoomed.pages.iter().map(|page| page.top).collect();
        assert_eq!(tops, vec![200.0, 2105.0 / 3.0, 3610.0 / 3.0]);
        assert_eq!(zoomed.pages[0].bottom, 2070.0 / 3.0);
        assert_eq!(zoomed.raster_scale(), 0.49 * 3.0);
    }

    #[test]
    fn pages_top_snaps_the_header_to_device_pixels() {
        let placement = place(
            LayoutMode::Continuous { max_width: 600.0 },
            &extents(&[(640.0, 800.0)]),
            100.4,
            Viewport {
                device_scale: 2.0,
                ..viewport(640.0, 900.0, 0.0, 0.0)
            },
            1.0,
        );
        assert_eq!(placement.pages_top, 140.5);
        assert_eq!(placement.pages[0].top, 140.5);
    }

    #[test]
    fn pages_top_rounds_halves_away_from_zero_and_treats_invalid_scales_as_one() {
        let pages_top = |header_height: f64, device_scale: f64| {
            place(
                PAGINATED,
                &extents(&[(720.0, 960.0)]),
                header_height,
                Viewport {
                    device_scale,
                    ..viewport(720.0, 900.0, 0.0, 0.0)
                },
                1.0,
            )
            .pages_top
        };
        assert_eq!(pages_top(0.25, 2.0), 0.5);
        assert_eq!(pages_top(-0.25, 2.0), -0.5);
        assert_eq!(pages_top(10.2, 3.0), 31.0 / 3.0);
        assert_eq!(pages_top(1.3, 0.0), 1.0);
        assert_eq!(pages_top(1.3, f64::NAN), 1.0);
    }

    #[test]
    fn pages_are_centered_on_device_pixels() {
        let placement = place(
            LayoutMode::Continuous { max_width: 600.0 },
            &extents(&[(641.0, 800.0)]),
            0.0,
            viewport(1000.0, 900.0, 0.0, 0.0),
            1.0,
        );
        assert_eq!(placement.pages[0].left, 180.0);
        assert_eq!(placement.pages[0].right, 821.0);
    }

    #[test]
    fn bottom_padding_reserves_the_occlusion_and_scroll_past_end() {
        let area = viewport(400.0, 900.0, 100.0, 50.0);
        let continuous = place(
            LayoutMode::Continuous { max_width: 600.0 },
            &extents(&[(400.0, 100.0)]),
            0.0,
            area,
            1.5,
        );
        assert_eq!(continuous.bottom_padding, 395.0);
        let paginated = place(PAGINATED, &extents(&[(720.0, 960.0)]), 0.0, area, 1.0);
        assert_eq!(paginated.bottom_padding, 50.0);
    }

    #[test]
    fn bottom_occlusion_never_exceeds_the_area_below_the_top_occlusion() {
        let area = viewport(400.0, 300.0, 200.0, 250.0);
        assert_eq!(area.visible_bottom(), 200.0);
        assert_eq!(area.bottom_occlusion(), 100.0);
        assert_eq!(area.visible_height(), 0.0);
        let normal = viewport(400.0, 900.0, 120.0, 100.0);
        assert_eq!(normal.bottom_occlusion(), 100.0);
        assert_eq!(normal.visible_height(), 680.0);
    }

    #[test]
    fn local_points_map_to_content_with_the_page_origin() {
        let pages = extents(&[(400.0, 600.0), (400.0, 800.0), (400.0, 500.0)]);
        let mode = LayoutMode::Continuous { max_width: 360.0 };
        let unit = place(mode, &pages, 0.0, viewport(400.0, 900.0, 0.0, 0.0), 1.0);
        assert_eq!(
            unit.page_local_to_content(1, 100.0, 50.0),
            Some((100.0, 690.0))
        );
        let zoomed = place(mode, &pages, 0.0, viewport(400.0, 900.0, 0.0, 0.0), 2.0);
        assert_eq!(
            zoomed.page_local_to_content(1, 100.0, 50.0),
            Some((200.0, 1340.0))
        );
        assert_eq!(unit.page_local_to_content(5, 0.0, 0.0), None);
        assert_eq!(unit.page_rect(5), None);
    }

    #[test]
    fn pages_in_returns_half_open_overlaps() {
        let placement = place(
            LayoutMode::Continuous { max_width: 360.0 },
            &extents(&[(400.0, 100.0); 3]),
            0.0,
            viewport(400.0, 900.0, 0.0, 0.0),
            1.0,
        );
        assert_eq!(placement.pages_in(0.0, 40.0), 0..0);
        assert_eq!(placement.pages_in(0.0, 41.0), 0..1);
        assert_eq!(placement.pages_in(139.0, 141.0), 0..2);
        assert_eq!(placement.pages_in(140.0, 240.0), 1..2);
        assert_eq!(placement.pages_in(340.0, 400.0), 3..3);
        assert_eq!(placement.pages_in(f64::NAN, 400.0), 0..0);
        assert_eq!(placement.pages_in(200.0, 100.0), 0..0);
    }

    #[test]
    fn scroll_is_clamped_to_the_content() {
        let placement = place(
            LayoutMode::Continuous { max_width: 600.0 },
            &extents(&[(600.0, 1000.0)]),
            0.0,
            viewport(600.0, 400.0, 0.0, 0.0),
            1.0,
        );
        assert_eq!(placement.content_height, 1220.0);
        assert_eq!(placement.max_scroll_y(), 820.0);
        assert_eq!(placement.clamp_scroll_y(90_000.0), 820.0);
        assert_eq!(placement.clamp_scroll_y(-5.0), 0.0);
        assert_eq!(placement.clamp_scroll_y(f64::NAN), 0.0);
        assert_eq!(placement.max_scroll_x(), 0.0);
        assert_eq!(placement.clamp_scroll_x(30.0), 0.0);
    }

    fn mode_strategy() -> impl Strategy<Value = LayoutMode> {
        prop_oneof![
            (100.0..1200.0f64).prop_map(|max_width| LayoutMode::Continuous { max_width }),
            (
                200.0..1200.0f64,
                200.0..2000.0f64,
                0.0..120.0f64,
                0.0..120.0f64
            )
                .prop_map(|(page_width, page_height, vertical, horizontal)| {
                    LayoutMode::Paginated {
                        page_width,
                        page_height,
                        margin_top: vertical,
                        margin_bottom: vertical,
                        margin_left: horizontal,
                        margin_right: horizontal,
                    }
                }),
        ]
    }

    fn scenario() -> impl Strategy<Value = (LayoutMode, Vec<PageExtent>, f64, Viewport, f64)> {
        (
            mode_strategy(),
            proptest::collection::vec(0.0..3000.0f64, 0..40),
            0.0..400.0f64,
            (
                200.0..1400.0f64,
                300.0..1400.0f64,
                0.0..150.0f64,
                0.0..400.0f64,
                proptest::sample::select(vec![1.0, 2.0, 3.0, 2.625, 3.5]),
            ),
            0.1..3.0f64,
        )
            .prop_map(
                |(mode, heights, header_height, (width, height, top, bottom, scale), zoom)| {
                    let page_width = match mode {
                        LayoutMode::Continuous { max_width } => {
                            max_width.min((width - 40.0).max(0.0)) + 40.0
                        }
                        LayoutMode::Paginated { page_width, .. } => page_width,
                    };
                    let pages = heights
                        .into_iter()
                        .map(|height| PageExtent {
                            width: page_width,
                            height,
                        })
                        .collect();
                    let viewport = Viewport {
                        width,
                        height,
                        occlusion_top: top,
                        occlusion_bottom: bottom,
                        device_scale: scale,
                    };
                    (mode, pages, header_height, viewport, zoom)
                },
            )
    }

    fn on_pixel(value: f64, scale: f64) -> bool {
        let pixels = value * scale;
        (pixels - pixels.round()).abs() < 1e-6
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

        #[test]
        fn page_tops_increase_on_device_pixels(
            (mode, pages, header_height, viewport, zoom) in scenario(),
        ) {
            let placement = place(mode, &pages, header_height, viewport, zoom);
            let scale = viewport.device_scale;
            let gap = page_gap(&mode, zoom, scale);
            prop_assert_eq!(placement.page_count(), pages.len());
            prop_assert!(on_pixel(placement.pages_top, scale));
            for (index, page) in placement.pages.iter().enumerate() {
                prop_assert!(on_pixel(page.top, scale));
                prop_assert!(on_pixel(page.bottom, scale));
                prop_assert!(on_pixel(page.left, scale));
                prop_assert!(on_pixel(page.right, scale));
                prop_assert!(page.bottom - page.top >= 1.0 / scale - 1e-9);
                if index == 0 {
                    prop_assert_eq!(page.top, placement.pages_top);
                } else {
                    let previous = placement.pages[index - 1];
                    prop_assert!(page.top > previous.top);
                    prop_assert!((page.top - previous.bottom - gap).abs() < 1e-6);
                }
            }
        }

        #[test]
        fn content_covers_the_viewport_minimum_and_every_page(
            (mode, pages, header_height, viewport, zoom) in scenario(),
        ) {
            let placement = place(mode, &pages, header_height, viewport, zoom);
            prop_assert!(placement.minimum_body_height >= 0.0);
            prop_assert!(placement.bottom_padding >= 0.0);
            prop_assert!(
                placement.content_height >= viewport.height - viewport.bottom_occlusion() - 1e-9
            );
            prop_assert!(
                placement.content_height
                    >= placement.pages_bottom() + placement.bottom_padding - 1e-9
            );
            prop_assert!(placement.content_width >= viewport.width);
            for page in &placement.pages {
                prop_assert!(page.left >= 0.0);
                prop_assert!(page.right <= placement.content_width + 1e-9);
            }
        }

        #[test]
        fn pages_in_matches_a_linear_scan(
            (mode, pages, header_height, viewport, zoom) in scenario(),
            from in -0.1..1.1f64,
            to in -0.1..1.1f64,
        ) {
            let placement = place(mode, &pages, header_height, viewport, zoom);
            let top = from * placement.content_height;
            let bottom = to * placement.content_height;
            let expected: Vec<usize> = placement
                .pages
                .iter()
                .enumerate()
                .filter(|(_, page)| top < bottom && page.top < bottom && page.bottom > top)
                .map(|(index, _)| index)
                .collect();
            let actual: Vec<usize> = placement.pages_in(top, bottom).collect();
            prop_assert_eq!(actual, expected);
        }
    }
}
