use crate::layout::ContentPlacement;

pub fn current_page(
    placement: &ContentPlacement,
    scroll_y: f64,
    viewport_height: f64,
) -> Option<usize> {
    if placement.pages.is_empty() {
        return None;
    }
    let top = scroll_y.max(0.0);
    let bottom = top + viewport_height.max(0.0);
    let mut current = 0;
    let mut longest = 0.0;
    for index in placement.pages_in(top, bottom) {
        let page = placement.pages[index];
        let length = (page.bottom.min(bottom) - page.top.max(top)).max(0.0);
        if length > longest {
            longest = length;
            current = index;
        }
    }
    Some(current)
}

pub fn percent(scroll_y: f64, content_height: f64, viewport_height: f64) -> u32 {
    let max_scroll = (content_height - viewport_height).max(0.0);
    if max_scroll <= 0.0 {
        return 0;
    }
    let ratio = scroll_y / max_scroll;
    if ratio.is_nan() {
        return 0;
    }
    (ratio.clamp(0.0, 1.0) * 100.0).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{LayoutMode, PageExtent, PlacementInput, Viewport};
    use proptest::prelude::*;

    fn place(mode: LayoutMode, heights: &[f64], header_height: f64) -> ContentPlacement {
        let pages: Vec<PageExtent> = heights
            .iter()
            .map(|&height| PageExtent {
                width: 720.0,
                height,
            })
            .collect();
        ContentPlacement::new(PlacementInput {
            mode: &mode,
            pages: &pages,
            header_height,
            viewport: Viewport {
                width: 720.0,
                height: 800.0,
                occlusion_top: 0.0,
                occlusion_bottom: 0.0,
                device_scale: 1.0,
            },
            zoom: 1.0,
        })
    }

    fn paginated() -> LayoutMode {
        LayoutMode::Paginated {
            page_width: 720.0,
            page_height: 1000.0,
            margin_top: 72.0,
            margin_bottom: 72.0,
            margin_left: 64.0,
            margin_right: 64.0,
        }
    }

    #[test]
    fn continuous_percent_is_the_clamped_scroll_ratio() {
        assert_eq!(percent(100.0, 300.0, 100.0), 50);
        assert_eq!(percent(1.0, 300.0, 100.0), 1);
        assert_eq!(percent(-40.0, 300.0, 100.0), 0);
        assert_eq!(percent(250.0, 300.0, 100.0), 100);
        assert_eq!(percent(10.0, 100.0, 100.0), 0);
        assert_eq!(percent(f64::NAN, 300.0, 100.0), 0);
    }

    #[test]
    fn current_page_counts_the_header_in_content_coordinates() {
        let placement = place(paginated(), &[1000.0; 3], 200.0);
        assert_eq!(current_page(&placement, 700.0, 800.0), Some(0));
        assert_eq!(current_page(&placement, 1000.0, 800.0), Some(1));
    }

    #[test]
    fn current_page_prefers_the_earlier_page_on_ties() {
        let placement = place(
            LayoutMode::Continuous { max_width: 680.0 },
            &[100.0, 100.0],
            0.0,
        );
        assert_eq!(current_page(&placement, 90.0, 100.0), Some(0));
    }

    #[test]
    fn current_page_starts_at_the_content_top_while_bouncing() {
        let placement = place(
            LayoutMode::Continuous { max_width: 680.0 },
            &[100.0, 1000.0],
            0.0,
        );
        assert_eq!(current_page(&placement, -100.0, 300.0), Some(1));
        let tall_header = place(paginated(), &[1000.0; 3], 1000.0);
        assert_eq!(current_page(&tall_header, 0.0, 800.0), Some(0));
    }

    #[test]
    fn current_page_is_absent_without_pages() {
        let placement = place(paginated(), &[], 200.0);
        assert_eq!(current_page(&placement, 0.0, 800.0), None);
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

        #[test]
        fn current_page_matches_a_linear_scan(
            heights in proptest::collection::vec(1.0..2000.0f64, 1..30),
            header_height in 0.0..600.0f64,
            continuous in any::<bool>(),
            scroll_y in -300.0..40_000.0f64,
            viewport_height in 0.0..1200.0f64,
        ) {
            let mode = if continuous {
                LayoutMode::Continuous { max_width: 680.0 }
            } else {
                paginated()
            };
            let placement = place(mode, &heights, header_height);
            let top = scroll_y.max(0.0);
            let bottom = top + viewport_height;
            let mut expected = 0;
            let mut longest = 0.0;
            for (index, page) in placement.pages.iter().enumerate() {
                let length = (page.bottom.min(bottom) - page.top.max(top)).max(0.0);
                if length > longest {
                    longest = length;
                    expected = index;
                }
            }
            prop_assert_eq!(current_page(&placement, scroll_y, viewport_height), Some(expected));
        }

        #[test]
        fn percent_is_bounded_and_monotone(
            first in -500.0..5000.0f64,
            second in -500.0..5000.0f64,
            content_height in 0.0..5000.0f64,
            viewport_height in 0.0..1200.0f64,
        ) {
            let (low, high) = if first <= second { (first, second) } else { (second, first) };
            let low_percent = percent(low, content_height, viewport_height);
            let high_percent = percent(high, content_height, viewport_height);
            prop_assert!(high_percent <= 100);
            prop_assert!(low_percent <= high_percent);
        }
    }
}
