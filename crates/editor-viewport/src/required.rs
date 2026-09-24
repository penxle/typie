use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt;

use crate::layout::{Rect, pages_overlapping};
use crate::rounding::effective_zoom;

pub const TILE_SIZE: i32 = 512;
pub const MAX_TILES: usize = 128;

const PAGE_ACQUIRE_HEIGHTS: f64 = 1.0;
const PAGE_RETAIN_HEIGHTS: f64 = 1.5;
const MARGIN_WIDTHS: f64 = 0.25;
const MARGIN_HEIGHTS: f64 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PxRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Ord for PxRect {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.y0, self.x0, self.y1, self.x1).cmp(&(other.y0, other.x0, other.y1, other.x1))
    }
}

impl PartialOrd for PxRect {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileBudgetExceeded;

impl fmt::Display for TileBudgetExceeded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("tile budget exceeded")
    }
}

impl std::error::Error for TileBudgetExceeded {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileArea {
    Visible,
    Margin,
}

pub fn page_view_set(
    pages: &[Rect],
    top: f64,
    bottom: f64,
    active: &BTreeSet<usize>,
) -> BTreeSet<usize> {
    let height = bottom - top;
    if !top.is_finite() || !bottom.is_finite() || height <= 0.0 {
        return BTreeSet::new();
    }
    let acquire = PAGE_ACQUIRE_HEIGHTS * height;
    let retain = PAGE_RETAIN_HEIGHTS * height;
    pages_with_height(pages, top - acquire, bottom + acquire)
        .chain(
            pages_with_height(pages, top - retain, bottom + retain)
                .filter(|index| active.contains(index)),
        )
        .collect()
}

pub fn pages_with_height(
    pages: &[Rect],
    top: f64,
    bottom: f64,
) -> impl Iterator<Item = usize> + '_ {
    pages_overlapping(pages, top, bottom).filter(|&index| {
        let page = pages[index];
        page.top.is_finite() && page.bottom.is_finite() && page.bottom > page.top
    })
}

pub fn margin_view(view: Rect) -> Rect {
    let horizontal = view.width() * MARGIN_WIDTHS;
    let vertical = view.height() * MARGIN_HEIGHTS;
    Rect {
        left: view.left - horizontal,
        top: view.top - vertical,
        right: view.right + horizontal,
        bottom: view.bottom + vertical,
    }
}

pub fn approach_view(view: Rect, destination_top: f64) -> Rect {
    let height = view.height();
    if !destination_top.is_finite() || !height.is_finite() || height <= 0.0 {
        return view;
    }
    let (top, bottom) = if destination_top >= view.top {
        (
            view.top.max(destination_top - height),
            destination_top + height,
        )
    } else {
        (
            destination_top,
            view.top.min(destination_top + height) + height,
        )
    };
    Rect {
        left: view.left,
        top,
        right: view.right,
        bottom,
    }
}

pub fn tile_region(page: Rect, view: Rect, zoom: f64, area: TileArea) -> Rect {
    let zoom = effective_zoom(zoom);
    let region = match area {
        TileArea::Visible => view,
        TileArea::Margin => margin_view(view),
    };
    Rect {
        left: (region.left - page.left) / zoom,
        top: (region.top - page.top) / zoom,
        right: (region.right - page.left) / zoom,
        bottom: (region.bottom - page.top) / zoom,
    }
}

pub fn full_width(region: Rect, page_width: f64) -> Rect {
    Rect {
        left: 0.0,
        right: page_width,
        ..region
    }
}

pub fn raster_size(width: f64, height: f64, scale: f64) -> Option<(i32, i32)> {
    if !width.is_finite() || !height.is_finite() || !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    Some((pixel_extent(width * scale), pixel_extent(height * scale)))
}

pub fn required_tiles(
    width: f64,
    height: f64,
    scale: f64,
    regions: &[Rect],
) -> Result<Vec<PxRect>, TileBudgetExceeded> {
    let Some((pixel_width, pixel_height)) = raster_size(width, height, scale) else {
        return Ok(Vec::new());
    };
    let mut tiles = BTreeSet::new();
    for region in regions {
        if ![region.left, region.top, region.right, region.bottom]
            .iter()
            .all(|edge| edge.is_finite())
        {
            continue;
        }
        let left = pixel_edge((region.left * scale).floor(), pixel_width);
        let top = pixel_edge((region.top * scale).floor(), pixel_height);
        let right = pixel_edge((region.right * scale).ceil(), pixel_width);
        let bottom = pixel_edge((region.bottom * scale).ceil(), pixel_height);
        if right <= left || bottom <= top {
            continue;
        }
        let columns = left / TILE_SIZE..=(right - 1) / TILE_SIZE;
        let rows = top / TILE_SIZE..=(bottom - 1) / TILE_SIZE;
        let count = (i64::from(columns.end() - columns.start()) + 1)
            * (i64::from(rows.end() - rows.start()) + 1);
        if count > MAX_TILES as i64 {
            return Err(TileBudgetExceeded);
        }
        for row in rows {
            for column in columns.clone() {
                let x0 = column * TILE_SIZE;
                let y0 = row * TILE_SIZE;
                tiles.insert(PxRect {
                    x0,
                    y0,
                    x1: x0.saturating_add(TILE_SIZE).min(pixel_width),
                    y1: y0.saturating_add(TILE_SIZE).min(pixel_height),
                });
                if tiles.len() > MAX_TILES {
                    return Err(TileBudgetExceeded);
                }
            }
        }
    }
    Ok(tiles.into_iter().collect())
}

fn pixel_extent(value: f64) -> i32 {
    value.round().clamp(1.0, f64::from(i32::MAX)) as i32
}

fn pixel_edge(value: f64, limit: i32) -> i32 {
    value.clamp(0.0, f64::from(limit)) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{ContentPlacement, LayoutMode, PageExtent, PlacementInput, Viewport};
    use proptest::prelude::*;

    fn spans(boundaries: &[f64]) -> Vec<Rect> {
        boundaries
            .windows(2)
            .map(|pair| Rect {
                left: 0.0,
                top: pair[0],
                right: 100.0,
                bottom: pair[1],
            })
            .collect()
    }

    fn uniform(count: usize) -> Vec<Rect> {
        let boundaries: Vec<f64> = (0..=count).map(|index| index as f64 * 100.0).collect();
        spans(&boundaries)
    }

    fn set(items: &[usize]) -> BTreeSet<usize> {
        items.iter().copied().collect()
    }

    fn edges(left: f64, top: f64, right: f64, bottom: f64) -> Rect {
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }

    fn px(x0: i32, y0: i32, x1: i32, y1: i32) -> PxRect {
        PxRect { x0, y0, x1, y1 }
    }

    #[test]
    fn empty_documents_have_no_page_views() {
        assert_eq!(page_view_set(&[], 0.0, 100.0, &set(&[0])), set(&[]));
    }

    #[test]
    fn invalid_viewports_and_empty_pages_are_ignored() {
        let pages = spans(&[0.0, 100.0, 200.0]);
        assert_eq!(page_view_set(&pages, f64::NAN, 100.0, &set(&[1])), set(&[]));
        assert_eq!(page_view_set(&pages, 300.0, 200.0, &set(&[1])), set(&[]));
        let with_empty_page = spans(&[0.0, 100.0, 100.0, 200.0]);
        assert_eq!(
            page_view_set(&with_empty_page, 0.0, 100.0, &set(&[1])),
            set(&[0, 2])
        );
    }

    #[test]
    fn pages_within_one_viewport_height_are_acquired() {
        let pages = spans(&[0.0, 100.0, 200.0, 300.0, 400.0, 500.0]);
        assert_eq!(
            page_view_set(&pages, 200.0, 300.0, &set(&[])),
            set(&[1, 2, 3])
        );
    }

    #[test]
    fn active_pages_are_retained_within_one_and_a_half_heights() {
        let pages = spans(&[0.0, 50.0, 100.0, 200.0, 300.0, 400.0, 450.0, 500.0]);
        assert_eq!(
            page_view_set(&pages, 200.0, 300.0, &set(&[0, 1, 5, 6])),
            set(&[1, 2, 3, 4, 5])
        );
    }

    #[test]
    fn out_of_range_active_pages_are_ignored() {
        let pages = spans(&[0.0, 100.0, 200.0, 300.0, 400.0]);
        assert_eq!(
            page_view_set(&pages, 0.0, 100.0, &set(&[2, 3, 100])),
            set(&[0, 1, 2])
        );
    }

    #[test]
    fn appended_pages_join_the_page_views() {
        assert_eq!(
            page_view_set(&spans(&[0.0, 100.0]), 100.0, 200.0, &set(&[0])),
            set(&[0])
        );
        assert_eq!(
            page_view_set(&uniform(4), 100.0, 200.0, &set(&[0])),
            set(&[0, 1, 2])
        );
    }

    #[test]
    fn removed_pages_leave_the_page_views() {
        assert_eq!(
            page_view_set(&spans(&[0.0, 100.0]), 100.0, 200.0, &set(&[0, 1, 2, 3])),
            set(&[0])
        );
    }

    #[test]
    fn long_documents_keep_the_page_views_bounded() {
        assert_eq!(
            page_view_set(&uniform(100), 5000.0, 5100.0, &set(&[])),
            set(&[49, 50, 51])
        );
    }

    #[test]
    fn pages_with_height_skip_empty_and_unbounded_pages() {
        let mut pages = spans(&[0.0, 100.0, 100.0, 200.0]);
        pages.push(edges(0.0, 200.0, 100.0, f64::INFINITY));
        let within =
            |top: f64, bottom: f64| pages_with_height(&pages, top, bottom).collect::<Vec<_>>();
        assert_eq!(within(50.0, 150.0), vec![0, 2]);
        assert_eq!(within(100.0, 150.0), vec![2]);
        assert_eq!(within(0.0, 300.0), vec![0, 2]);
        assert_eq!(within(150.0, 150.0), Vec::<usize>::new());
        assert_eq!(within(f64::NAN, 150.0), Vec::<usize>::new());
    }

    #[test]
    fn offscreen_pages_are_not_implicit_demand() {
        let required = page_view_set(&uniform(10), 0.0, 100.0, &set(&[]));
        assert_eq!(required, set(&[0, 1]));
        assert!(!required.contains(&8));
    }

    #[test]
    fn shortened_documents_clamp_the_scroll_before_planning() {
        let placement = ContentPlacement::new(PlacementInput {
            mode: &LayoutMode::Continuous { max_width: 600.0 },
            pages: &[PageExtent {
                width: 600.0,
                height: 1000.0,
            }],
            header_height: 0.0,
            viewport: Viewport {
                width: 600.0,
                height: 400.0,
                occlusion_top: 0.0,
                occlusion_bottom: 0.0,
                device_scale: 1.0,
            },
            zoom: 1.0,
        });
        let top = placement.clamp_scroll_y(90_000.0);
        assert_eq!(top, placement.max_scroll_y());
        assert_eq!(
            page_view_set(&placement.pages, top, top + 400.0, &set(&[])),
            set(&[0])
        );
    }

    #[test]
    fn tile_regions_are_page_local_points() {
        let page = edges(10.0, 1000.0, 410.0, 2000.0);
        let view = edges(0.0, 1200.0, 400.0, 2000.0);
        assert_eq!(
            tile_region(page, view, 2.0, TileArea::Margin),
            edges(-55.0, -100.0, 245.0, 700.0)
        );
        assert_eq!(
            tile_region(page, view, 2.0, TileArea::Visible),
            edges(-5.0, 100.0, 195.0, 500.0)
        );
    }

    #[test]
    fn margin_view_widens_by_a_quarter_width_and_a_half_height() {
        let page = edges(10.0, 1000.0, 410.0, 2000.0);
        let view = edges(0.0, 1200.0, 400.0, 2000.0);
        assert_eq!(margin_view(view), edges(-100.0, 800.0, 500.0, 2400.0));
        assert_eq!(
            tile_region(page, view, 2.0, TileArea::Margin),
            tile_region(page, margin_view(view), 2.0, TileArea::Visible)
        );
    }

    #[test]
    fn approach_view_moving_down_ends_one_screen_past_the_destination() {
        let view = edges(10.0, 1000.0, 400.0, 1800.0);
        assert_eq!(
            approach_view(view, 5000.0),
            edges(10.0, 4200.0, 400.0, 5800.0)
        );
        assert_eq!(
            approach_view(view, 1800.0),
            edges(10.0, 1000.0, 400.0, 2600.0)
        );
        assert_eq!(
            approach_view(view, 1300.0),
            edges(10.0, 1000.0, 400.0, 2100.0)
        );
        assert_eq!(approach_view(view, 1000.0), view);
    }

    #[test]
    fn approach_view_moving_up_starts_at_the_destination() {
        let view = edges(10.0, 1000.0, 400.0, 1800.0);
        assert_eq!(
            approach_view(view, -3000.0),
            edges(10.0, -3000.0, 400.0, -1400.0)
        );
        assert_eq!(
            approach_view(view, 200.0),
            edges(10.0, 200.0, 400.0, 1800.0)
        );
        assert_eq!(
            approach_view(view, 700.0),
            edges(10.0, 700.0, 400.0, 1800.0)
        );
    }

    #[test]
    fn approach_view_without_a_usable_destination_or_height_is_the_view() {
        let view = edges(10.0, 1000.0, 400.0, 1800.0);
        assert_eq!(approach_view(view, f64::NAN), view);
        assert_eq!(approach_view(view, f64::INFINITY), view);
        assert_eq!(approach_view(view, f64::NEG_INFINITY), view);
        let flat = edges(0.0, 500.0, 400.0, 500.0);
        assert_eq!(approach_view(flat, 3000.0), flat);
        let inverted = edges(0.0, 500.0, 400.0, 100.0);
        assert_eq!(approach_view(inverted, 3000.0), inverted);
        let unbounded = edges(0.0, 500.0, 400.0, f64::INFINITY);
        assert_eq!(approach_view(unbounded, 3000.0), unbounded);
    }

    #[test]
    fn full_width_regions_keep_their_rows_and_span_every_column() {
        let region = tile_region(
            edges(0.0, 0.0, 800.0, 1200.0),
            edges(20.0, 300.0, 420.0, 900.0),
            1.0,
            TileArea::Margin,
        );
        assert_eq!(region, edges(-80.0, 0.0, 520.0, 1200.0));
        assert_eq!(full_width(region, 800.0), edges(0.0, 0.0, 800.0, 1200.0));
        let cells = |region: Rect| {
            required_tiles(800.0, 1200.0, 2.0, &[region])
                .unwrap()
                .into_iter()
                .map(|tile| (tile.y0, tile.x0))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            cells(region),
            [0, 512, 1024, 1536, 2048]
                .into_iter()
                .flat_map(|y0| [(y0, 0), (y0, 512), (y0, 1024)])
                .collect::<Vec<_>>()
        );
        assert_eq!(
            cells(full_width(region, 800.0)),
            [0, 512, 1024, 1536, 2048]
                .into_iter()
                .flat_map(|y0| [(y0, 0), (y0, 512), (y0, 1024), (y0, 1536)])
                .collect::<Vec<_>>()
        );
        let band = edges(-10.0, 700.0, 30.0, 760.0);
        assert_eq!(full_width(band, 800.0), edges(0.0, 700.0, 800.0, 760.0));
    }

    #[test]
    fn raster_size_rounds_half_away_from_zero_with_one_pixel_minimum() {
        assert_eq!(raster_size(512.5, 1.0, 1.0), Some((513, 1)));
        assert_eq!(raster_size(0.1, 0.1, 1.0), Some((1, 1)));
        assert_eq!(raster_size(200.0, 300.0, 1.59375), Some((319, 478)));
        assert_eq!(raster_size(100.0, 100.0, 0.0), None);
        assert_eq!(raster_size(f64::NAN, 100.0, 1.0), None);
    }

    #[test]
    fn tile_count_is_bounded_by_the_viewport_across_zoom() {
        let density = 3.1875;
        let counts: Vec<usize> = [0.5, 1.0, 2.0]
            .into_iter()
            .map(|zoom| {
                let region = edges(0.0, 1200.0 / zoom, 510.0 / zoom, 2700.0 / zoom);
                required_tiles(10_000.0, 200_000.0, density * zoom, &[region])
                    .unwrap()
                    .len()
            })
            .collect();
        assert_eq!(counts, vec![40, 40, 40]);
        assert!(counts.iter().all(|count| *count < 48));
    }

    #[test]
    fn overlapping_regions_share_tiles_on_a_clamped_grid() {
        let current = edges(100.0, 500.0, 700.0, 1000.0);
        let target = edges(100.0, 900.0, 700.0, 1400.0);
        let first = required_tiles(800.0, 200_000.0, 1.0, &[current]).unwrap();
        let prepared = required_tiles(800.0, 200_000.0, 1.0, &[current, target, current]).unwrap();
        assert_eq!(first.len(), 4);
        assert!(first.iter().all(|tile| prepared.contains(tile)));
        assert_eq!(prepared.len(), 6);
        assert!(prepared.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            required_tiles(800.0, 600.0, 1.0, &[edges(512.0, 512.0, 999.0, 999.0)]).unwrap(),
            vec![px(512, 512, 800, 600)]
        );
        assert_eq!(
            required_tiles(512.5, 1.0, 1.0, &[edges(0.0, 0.0, 512.5, 1.0)]).unwrap(),
            vec![px(0, 0, 512, 1), px(512, 0, 513, 1)]
        );
        assert_eq!(
            required_tiles(
                100_000.0,
                200_000.0,
                2.0,
                &[edges(0.0, 0.0, 100_000.0, 200_000.0)]
            ),
            Err(TileBudgetExceeded)
        );
    }

    #[test]
    fn tiles_are_ordered_by_top_then_left() {
        let tiles =
            required_tiles(1200.0, 1200.0, 1.0, &[edges(0.0, 0.0, 1200.0, 1200.0)]).unwrap();
        assert_eq!(
            tiles,
            vec![
                px(0, 0, 512, 512),
                px(512, 0, 1024, 512),
                px(1024, 0, 1200, 512),
                px(0, 512, 512, 1024),
                px(512, 512, 1024, 1024),
                px(1024, 512, 1200, 1024),
                px(0, 1024, 512, 1200),
                px(512, 1024, 1024, 1200),
                px(1024, 1024, 1200, 1200),
            ]
        );
        assert!(
            required_tiles(100.0, 100.0, 1.0, &[edges(f64::NAN, 0.0, 10.0, 10.0)])
                .unwrap()
                .is_empty()
        );
        assert!(
            required_tiles(100.0, 100.0, 1.0, &[edges(200.0, 0.0, 300.0, 10.0)])
                .unwrap()
                .is_empty()
        );
    }

    fn region_strategy() -> impl Strategy<Value = Rect> {
        (
            -2000.0..25_000.0f64,
            -2000.0..25_000.0f64,
            0.0..1500.0f64,
            0.0..1500.0f64,
        )
            .prop_map(|(left, top, width, height)| edges(left, top, left + width, top + height))
    }

    fn placement_strategy() -> impl Strategy<Value = (ContentPlacement, Vec<PageExtent>)> {
        (
            prop_oneof![
                (300.0..900.0f64).prop_map(|max_width| LayoutMode::Continuous { max_width }),
                (300.0..900.0f64, 400.0..1400.0f64).prop_map(|(page_width, page_height)| {
                    LayoutMode::Paginated {
                        page_width,
                        page_height,
                        margin_top: 72.0,
                        margin_bottom: 72.0,
                        margin_left: 64.0,
                        margin_right: 64.0,
                    }
                }),
            ],
            proptest::collection::vec(1.0..2000.0f64, 1..40),
            0.0..300.0f64,
            (
                320.0..932.0f64,
                320.0..932.0f64,
                proptest::sample::select(vec![2.0, 3.0]),
            ),
            0.2..2.5f64,
        )
            .prop_map(
                |(mode, heights, header_height, (width, height, scale), zoom)| {
                    let page_width = match mode {
                        LayoutMode::Continuous { max_width } => {
                            max_width.min((width - 40.0).max(0.0)) + 40.0
                        }
                        LayoutMode::Paginated { page_width, .. } => page_width,
                    };
                    let pages: Vec<PageExtent> = heights
                        .into_iter()
                        .map(|height| PageExtent {
                            width: page_width,
                            height,
                        })
                        .collect();
                    let placement = ContentPlacement::new(PlacementInput {
                        mode: &mode,
                        pages: &pages,
                        header_height,
                        viewport: Viewport {
                            width,
                            height,
                            occlusion_top: 0.0,
                            occlusion_bottom: 0.0,
                            device_scale: scale,
                        },
                        zoom,
                    });
                    (placement, pages)
                },
            )
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

        #[test]
        fn tiles_stay_inside_the_raster_on_the_grid(
            width in 1.0..20_000.0f64,
            height in 1.0..200_000.0f64,
            scale in 0.5..3.0f64,
            regions in proptest::collection::vec(region_strategy(), 0..4),
        ) {
            let (pixel_width, pixel_height) = raster_size(width, height, scale).unwrap();
            if let Ok(tiles) = required_tiles(width, height, scale, &regions) {
                prop_assert!(tiles.len() <= MAX_TILES);
                prop_assert!(tiles.windows(2).all(|pair| pair[0] < pair[1]));
                for tile in &tiles {
                    prop_assert_eq!(tile.x0 % TILE_SIZE, 0);
                    prop_assert_eq!(tile.y0 % TILE_SIZE, 0);
                    prop_assert!(tile.x0 < tile.x1 && tile.x1 <= pixel_width);
                    prop_assert!(tile.y0 < tile.y1 && tile.y1 <= pixel_height);
                    prop_assert!(tile.x1 - tile.x0 <= TILE_SIZE);
                    prop_assert!(tile.y1 - tile.y0 <= TILE_SIZE);
                }
            }
        }

        #[test]
        fn visible_tiles_are_a_subset_of_margin_tiles(
            (placement, pages) in placement_strategy(),
            from in 0.0..1.0f64,
            across in 0.0..1.0f64,
        ) {
            let top = placement.clamp_scroll_y(from * placement.max_scroll_y());
            let left = placement.clamp_scroll_x(across * placement.max_scroll_x());
            let view = edges(
                left,
                top,
                left + placement.viewport.width,
                top + placement.viewport.height,
            );
            let page_views = page_view_set(&placement.pages, view.top, view.bottom, &BTreeSet::new());
            for index in placement.pages_in(view.top, view.bottom) {
                prop_assert!(page_views.contains(&index));
            }
            let raster_scale = placement.raster_scale();
            for index in page_views {
                let page = placement.pages[index];
                let extent = pages[index];
                let visible = required_tiles(
                    extent.width,
                    extent.height,
                    raster_scale,
                    &[tile_region(page, view, placement.zoom, TileArea::Visible)],
                )
                .unwrap();
                let margin = required_tiles(
                    extent.width,
                    extent.height,
                    raster_scale,
                    &[tile_region(page, view, placement.zoom, TileArea::Margin)],
                )
                .unwrap();
                prop_assert!(visible.iter().all(|tile| margin.contains(tile)));
            }
        }

        #[test]
        fn approach_view_covers_every_screen_near_the_destination_on_the_way(
            left in -1000i32..1000,
            width in 1i32..2000,
            start in -800_000i32..800_000,
            height in 1i32..32_000,
            travel in -64_000i32..64_000,
        ) {
            let s = f64::from(start) / 8.0;
            let h = f64::from(height) / 8.0;
            let t = s + f64::from(travel) / 8.0;
            let view = edges(f64::from(left), s, f64::from(left + width), s + h);
            let approach = approach_view(view, t);
            prop_assert_eq!((approach.left, approach.right), (view.left, view.right));
            let covers = |top: f64| approach.top <= top && top + h <= approach.bottom;
            prop_assert!(covers(t));
            for step in 0..=8 {
                let u = s + (t - s) * f64::from(step) / 8.0;
                if (u - t).abs() < h {
                    prop_assert!(covers(u), "u = {}", u);
                }
            }
            prop_assert!(approach.top >= s.min(t));
            prop_assert!(approach.bottom <= s.max(t) + h);
            prop_assert!(approach.bottom - approach.top <= 2.0 * h);
        }

        #[test]
        fn raster_size_matches_the_displayed_page_pixels(
            (placement, pages) in placement_strategy(),
        ) {
            let scale = placement.viewport.device_scale;
            for (page, extent) in placement.pages.iter().zip(&pages) {
                let displayed = (
                    (page.width() * scale).round() as i32,
                    (page.height() * scale).round() as i32,
                );
                prop_assert_eq!(
                    raster_size(extent.width, extent.height, placement.raster_scale()),
                    Some(displayed)
                );
            }
        }

        #[test]
        fn page_views_match_a_linear_scan(
            heights in proptest::collection::vec(prop_oneof![Just(0.0), 0.0..300.0f64], 0..30),
            gap in 0.0..40.0f64,
            top in -200.0..8000.0f64,
            height in -50.0..600.0f64,
            active in proptest::collection::btree_set(0usize..40, 0..10),
        ) {
            let mut pages = Vec::new();
            let mut cursor = 0.0;
            for length in heights {
                pages.push(edges(0.0, cursor, 100.0, cursor + length));
                cursor += length + gap;
            }
            let bottom = top + height;
            let overlaps = |page: &Rect, distance: f64| {
                page.bottom > page.top
                    && page.top < bottom + distance
                    && page.bottom > top - distance
            };
            let expected: BTreeSet<usize> = if height > 0.0 {
                pages
                    .iter()
                    .enumerate()
                    .filter(|(index, page)| {
                        overlaps(page, height) || (active.contains(index) && overlaps(page, height * 1.5))
                    })
                    .map(|(index, _)| index)
                    .collect()
            } else {
                BTreeSet::new()
            };
            prop_assert_eq!(page_view_set(&pages, top, bottom, &active), expected);
            let within: Vec<usize> = if height > 0.0 {
                pages
                    .iter()
                    .enumerate()
                    .filter(|(_, page)| overlaps(page, 0.0))
                    .map(|(index, _)| index)
                    .collect()
            } else {
                Vec::new()
            };
            prop_assert_eq!(pages_with_height(&pages, top, bottom).collect::<Vec<_>>(), within);
        }
    }
}
