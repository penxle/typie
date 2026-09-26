use crate::layout::Rect;
use crate::required::PxRect;
use crate::rounding::effective_scale;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FillClass {
    VisibleMissing,
    Verify,
    Destination,
    MarginMissing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FillTask {
    pub class: FillClass,
    pub page: usize,
    pub bounds: PxRect,
}

pub fn order_fill_tasks(tasks: &mut [FillTask], pages: &[Rect], device_scale: f64, view: Rect) {
    tasks.sort_by(|first, second| {
        first
            .class
            .cmp(&second.class)
            .then_with(|| {
                row_distance(first, pages, device_scale, view).total_cmp(&row_distance(
                    second,
                    pages,
                    device_scale,
                    view,
                ))
            })
            .then_with(|| first.page.cmp(&second.page))
            .then_with(|| first.bounds.cmp(&second.bounds))
    });
}

pub fn row_center(page: Rect, bounds: PxRect, device_scale: f64) -> f64 {
    page.top + (f64::from(bounds.y0) + f64::from(bounds.y1)) / (2.0 * effective_scale(device_scale))
}

fn row_distance(task: &FillTask, pages: &[Rect], device_scale: f64, anchor: Rect) -> f64 {
    let Some(page) = pages.get(task.page) else {
        return f64::INFINITY;
    };
    (row_center(*page, task.bounds, device_scale) - (anchor.top + anchor.bottom) / 2.0).abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::BTreeSet;

    fn edges(left: f64, top: f64, right: f64, bottom: f64) -> Rect {
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }

    fn task(class: FillClass, page: usize, x0: i32, y0: i32, x1: i32, y1: i32) -> FillTask {
        FillTask {
            class,
            page,
            bounds: PxRect { x0, y0, x1, y1 },
        }
    }

    #[test]
    fn fill_orders_by_class_then_distance_to_the_viewport_center() {
        let pages = [edges(0.0, 0.0, 400.0, 2000.0)];
        let view = edges(0.0, 900.0, 400.0, 1300.0);
        let far_margin = task(FillClass::MarginMissing, 0, 0, 512, 800, 1024);
        let near_margin = task(FillClass::MarginMissing, 0, 0, 2560, 800, 3072);
        let near_visible = task(FillClass::VisibleMissing, 0, 0, 2048, 800, 2560);
        let far_visible = task(FillClass::VisibleMissing, 0, 0, 1024, 800, 1536);
        let verify = task(FillClass::Verify, 0, 0, 0, 800, 4000);
        let mut tasks = vec![far_margin, verify, far_visible, near_margin, near_visible];
        order_fill_tasks(&mut tasks, &pages, 2.0, view);
        assert_eq!(
            tasks,
            vec![near_visible, far_visible, verify, near_margin, far_margin]
        );
    }

    #[test]
    fn row_centers_are_content_positions_of_the_tile_row() {
        let page = edges(0.0, 1000.0, 400.0, 2000.0);
        let bounds = PxRect {
            x0: 512,
            y0: 512,
            x1: 800,
            y1: 1024,
        };
        assert_eq!(row_center(page, bounds, 2.0), 1384.0);
        assert_eq!(row_center(page, bounds, 3.0), 1256.0);
        assert_eq!(row_center(page, bounds, 0.0), 1768.0);
        assert_eq!(row_center(page, bounds, f64::NAN), 1768.0);
    }

    #[test]
    fn fill_distance_counts_from_the_page_origin() {
        let pages = [
            edges(0.0, 0.0, 400.0, 2000.0),
            edges(0.0, 2024.0, 400.0, 4024.0),
        ];
        let view = edges(0.0, 1900.0, 400.0, 2300.0);
        let near = task(FillClass::MarginMissing, 1, 0, 512, 800, 1024);
        let far = task(FillClass::MarginMissing, 0, 0, 2048, 800, 2560);
        let mut tasks = vec![far, near];
        order_fill_tasks(&mut tasks, &pages, 2.0, view);
        assert_eq!(tasks, vec![near, far]);
    }

    #[test]
    fn fill_breaks_distance_ties_by_page_then_top_then_left() {
        let pages = [
            edges(0.0, 0.0, 400.0, 100.0),
            edges(0.0, 500.0, 400.0, 600.0),
        ];
        let view = edges(0.0, 100.0, 400.0, 500.0);
        let upper_left = task(FillClass::MarginMissing, 0, 0, 100, 200, 200);
        let upper_right = task(FillClass::MarginMissing, 0, 200, 100, 400, 200);
        let left = task(FillClass::MarginMissing, 0, 0, 250, 100, 350);
        let right = task(FillClass::MarginMissing, 0, 300, 250, 400, 350);
        let lower_left = task(FillClass::MarginMissing, 0, 0, 400, 200, 500);
        let lower_right = task(FillClass::MarginMissing, 0, 200, 400, 400, 500);
        let first_page = task(FillClass::MarginMissing, 0, 0, 0, 400, 100);
        let second_page = task(FillClass::MarginMissing, 1, 0, 0, 400, 100);
        let mut tasks = vec![
            second_page,
            lower_right,
            first_page,
            upper_right,
            lower_left,
            right,
            left,
            upper_left,
        ];
        order_fill_tasks(&mut tasks, &pages, 1.0, view);
        assert_eq!(
            tasks,
            vec![
                left,
                right,
                upper_left,
                upper_right,
                lower_left,
                lower_right,
                first_page,
                second_page,
            ]
        );
    }

    #[test]
    fn fill_completes_the_nearest_row_before_starting_the_next() {
        let pages = [edges(0.0, 0.0, 600.0, 4000.0)];
        let view = edges(0.0, 42.0, 600.0, 886.0);
        let row = |class: FillClass, index: i32| {
            [(0, 512), (512, 1024), (1024, 1200)]
                .map(|(x0, x1)| task(class, 0, x0, index * 512, x1, index * 512 + 512))
        };
        let [near_left, near_middle, near_right] = row(FillClass::MarginMissing, 1);
        let [far_left, far_middle, far_right] = row(FillClass::MarginMissing, 2);
        let mut margin = vec![
            far_middle,
            near_right,
            far_left,
            near_middle,
            far_right,
            near_left,
        ];
        order_fill_tasks(&mut margin, &pages, 2.0, view);
        assert_eq!(
            margin,
            vec![
                near_left,
                near_middle,
                near_right,
                far_left,
                far_middle,
                far_right,
            ]
        );
    }

    #[test]
    fn page_verification_sorts_before_destination_tiles_even_when_farther() {
        assert!(FillClass::Verify < FillClass::Destination);
        let pages = [
            edges(0.0, 0.0, 400.0, 2000.0),
            edges(0.0, 2024.0, 400.0, 4024.0),
        ];
        let view = edges(0.0, 900.0, 400.0, 1300.0);
        let destination = task(FillClass::Destination, 0, 0, 2048, 800, 2560);
        let verify = task(FillClass::Verify, 1, 0, 0, 800, 4000);
        let mut tasks = vec![destination, verify];
        order_fill_tasks(&mut tasks, &pages, 2.0, view);
        assert_eq!(tasks, vec![verify, destination]);
    }

    fn class_strategy() -> impl Strategy<Value = FillClass> {
        prop_oneof![
            Just(FillClass::VisibleMissing),
            Just(FillClass::Verify),
            Just(FillClass::Destination),
            Just(FillClass::MarginMissing),
        ]
    }

    fn tasks_strategy() -> impl Strategy<Value = Vec<FillTask>> {
        (
            proptest::collection::vec(prop_oneof![Just(512), 1i32..=512], 40),
            proptest::collection::vec(
                (class_strategy(), 0usize..4, 0i32..8, 0usize..40, 1i32..=512),
                0..60,
            ),
        )
            .prop_map(|(heights, cells)| {
                cells
                    .into_iter()
                    .map(|(class, page, column, row, width)| {
                        let x0 = column * 512;
                        let y0 = row as i32 * 512;
                        task(class, page, x0, y0, x0 + width, y0 + heights[row])
                    })
                    .collect()
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

        #[test]
        fn fill_order_is_class_then_row_distance_then_page_then_bounds(
            tasks in tasks_strategy(),
            scroll_y in -500.0..40_000.0f64,
            scale in proptest::sample::select(vec![1.0, 2.0, 3.0]),
        ) {
            let pages = [
                edges(0.0, 0.0, 800.0, 1000.0),
                edges(0.0, 1024.0, 800.0, 2024.0),
                edges(0.0, 2048.0, 800.0, 3048.0),
            ];
            let view = edges(0.0, scroll_y, 390.0, scroll_y + 844.0);
            let mut ordered = tasks.clone();
            order_fill_tasks(&mut ordered, &pages, scale, view);
            let mut sorted_input = tasks.clone();
            sorted_input.sort_by_key(|task| (task.class, task.page, task.bounds));
            let mut sorted_output = ordered.clone();
            sorted_output.sort_by_key(|task| (task.class, task.page, task.bounds));
            prop_assert_eq!(sorted_input, sorted_output);
            let distance = |task: &FillTask| {
                pages.get(task.page).map_or(f64::INFINITY, |page| {
                    (page.top + f64::from(task.bounds.y0 + task.bounds.y1) / (2.0 * scale)
                        - (view.top + view.bottom) / 2.0)
                        .abs()
                })
            };
            for pair in ordered.windows(2) {
                prop_assert!(pair[0].class <= pair[1].class);
                if pair[0].class == pair[1].class {
                    let first = distance(&pair[0]);
                    let second = distance(&pair[1]);
                    prop_assert!(first <= second);
                    if first == second {
                        prop_assert!((pair[0].page, pair[0].bounds) <= (pair[1].page, pair[1].bounds));
                    }
                }
            }
            let mut finished = BTreeSet::new();
            let mut current = None;
            for task in &ordered {
                let row = (task.class, task.page, task.bounds.y0, task.bounds.y1);
                if current != Some(row) {
                    prop_assert!(finished.insert(row), "row {:?} resumed", row);
                    current = Some(row);
                }
            }
        }
    }
}
