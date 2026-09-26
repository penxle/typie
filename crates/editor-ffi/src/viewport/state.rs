use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use editor_core::Revision;
use editor_renderer::backend::cpu::CpuSink;
use editor_renderer::damage::IRect;
use editor_renderer::display_list::DisplayList;
use editor_viewport::{
    ContentPlacement, FillClass, FillTask, LayoutMode, PageExtent, PxRect, Rect, Viewport, ZoomKey,
    initial_zoom, order_fill_tasks, row_center, zoom_key,
};

use super::raster;
use super::{FrameDebug, FrameLayout, FrameTile, FrameTileRef, ViewportRequest};

pub(super) struct Counter(u64);

impl Counter {
    pub(super) fn next(&mut self) -> u64 {
        let value = self.0;
        self.0 += 1;
        value
    }
}

pub(super) struct Tile {
    pub(super) pixels: Option<Arc<[u8]>>,
    pub(super) version: u64,
}

pub(super) struct PageRaster {
    pub(super) scale: f64,
    pub(super) signature: Option<u64>,
    pub(super) display_list: Option<DisplayList>,
    pub(super) tiles: BTreeMap<PxRect, Tile>,
    pub(super) stale: BTreeSet<PxRect>,
}

impl PageRaster {
    pub(super) fn at(
        pages: &mut BTreeMap<usize, PageRaster>,
        index: usize,
        scale: f64,
    ) -> &mut PageRaster {
        let raster = pages.entry(index).or_insert_with(|| PageRaster::new(scale));
        if raster.scale != scale {
            *raster = PageRaster::new(scale);
        }
        raster
    }

    fn new(scale: f64) -> Self {
        Self {
            scale,
            signature: None,
            display_list: None,
            tiles: BTreeMap::new(),
            stale: BTreeSet::new(),
        }
    }

    pub(super) fn is_current(&self, signature: u64) -> bool {
        self.signature == Some(signature)
    }

    pub(super) fn retain(&mut self, keep: &BTreeSet<PxRect>) {
        self.tiles.retain(|bounds, _| keep.contains(bounds));
        self.stale.retain(|bounds| keep.contains(bounds));
    }

    pub(super) fn invalidate(&mut self, damage: &[IRect], keep: &BTreeSet<PxRect>) -> Vec<PxRect> {
        let damaged: Vec<PxRect> = self
            .tiles
            .keys()
            .copied()
            .filter(|bounds| raster::is_damaged(*bounds, damage))
            .collect();
        let mut kept = Vec::new();
        for bounds in damaged {
            if keep.contains(&bounds) {
                kept.push(bounds);
            } else {
                self.tiles.remove(&bounds);
                self.stale.insert(bounds);
            }
        }
        kept
    }
}

pub(super) struct PagePlan {
    pub(super) size: (i32, i32),
    pub(super) visible: BTreeSet<PxRect>,
    pub(super) margin: BTreeSet<PxRect>,
    pub(super) destination: BTreeSet<PxRect>,
}

impl PagePlan {
    pub(super) fn required(&self) -> BTreeSet<PxRect> {
        self.visible
            .iter()
            .chain(&self.margin)
            .chain(&self.destination)
            .copied()
            .collect()
    }
}

pub(super) struct FramePlan {
    pub(super) request: ViewportRequest,
    pub(super) revision: Revision,
    pub(super) layout: FrameLayout,
    pub(super) view: Rect,
    pub(super) destination: Option<Rect>,
    pub(super) pages: BTreeMap<usize, PagePlan>,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct PlacementKey {
    pub(super) revision: Revision,
    pub(super) mode: LayoutMode,
    pub(super) zoom: f64,
    pub(super) viewport: Viewport,
    pub(super) header_height: f64,
}

pub(super) struct PlacementCache {
    pub(super) key: PlacementKey,
    pub(super) extents: Vec<PageExtent>,
    pub(super) placement: ContentPlacement,
}

pub(super) struct ViewportState {
    pub(super) pages: BTreeMap<usize, PageRaster>,
    pub(super) emitted: BTreeMap<(usize, PxRect), u64>,
    pub(super) active: BTreeSet<usize>,
    pub(super) placement: Option<PlacementCache>,
    pub(super) zoom: Option<(ZoomKey, f64)>,
    pub(super) plan: Option<FramePlan>,
    pub(super) frame_ids: Counter,
    pub(super) versions: Counter,
    pub(super) presented: Option<u64>,
    pub(super) scratch: CpuSink,
}

impl ViewportState {
    pub(super) fn new() -> Self {
        Self {
            pages: BTreeMap::new(),
            emitted: BTreeMap::new(),
            active: BTreeSet::new(),
            placement: None,
            zoom: None,
            plan: None,
            frame_ids: Counter(1),
            versions: Counter(1),
            presented: None,
            scratch: raster::scratch(),
        }
    }

    pub(super) fn zoom(&mut self, mode: &LayoutMode, viewport_width: f64) -> f64 {
        let key = zoom_key(mode);
        match self.zoom {
            Some((current, zoom)) if current == key => zoom,
            _ => {
                let zoom = initial_zoom(mode, viewport_width);
                if viewport_width.is_finite() && viewport_width > 0.0 {
                    self.zoom = Some((key, zoom));
                }
                zoom
            }
        }
    }

    pub(super) fn take_changes(
        &mut self,
        signatures: &BTreeMap<usize, u64>,
    ) -> (Vec<FrameTile>, Vec<Arc<[u8]>>) {
        let mut shown = BTreeMap::new();
        for (&index, page) in self.plan.iter().flat_map(|plan| &plan.pages) {
            let Some(raster) = self.pages.get(&index) else {
                continue;
            };
            if !signatures
                .get(&index)
                .is_some_and(|signature| raster.is_current(*signature))
            {
                continue;
            }
            let required = page.required();
            let incomplete: BTreeSet<i32> = required
                .iter()
                .filter(|bounds| !raster.tiles.contains_key(bounds))
                .map(|bounds| bounds.y0)
                .collect();
            for bounds in required
                .iter()
                .filter(|bounds| !incomplete.contains(&bounds.y0))
            {
                let tile = &raster.tiles[bounds];
                if let Some(pixels) = &tile.pixels {
                    shown.insert((index, *bounds), (tile.version, pixels));
                }
            }
        }
        let mut tiles: Vec<FrameTile> = self
            .emitted
            .keys()
            .filter(|key| !shown.contains_key(key))
            .map(|&(page, bounds)| FrameTile::Drop {
                page: page as u32,
                bounds: bounds.into(),
            })
            .collect();
        let mut pixels = Vec::new();
        for (&(page, bounds), &(version, tile)) in &shown {
            if self.emitted.get(&(page, bounds)) == Some(&version) {
                continue;
            }
            tiles.push(FrameTile::Set {
                page: page as u32,
                bounds: bounds.into(),
                version,
                pixels: pixels.len() as u32,
            });
            pixels.push(Arc::clone(tile));
        }
        self.emitted = shown
            .into_iter()
            .map(|(key, (version, _))| (key, version))
            .collect();
        (tiles, pixels)
    }

    pub(super) fn fill_tasks(&self, signatures: &BTreeMap<usize, u64>) -> Vec<FillTask> {
        let (Some(plan), Some(cache)) = (&self.plan, &self.placement) else {
            return Vec::new();
        };
        let pages = &cache.placement.pages;
        let scale = plan.request.device_scale;
        let decelerating = plan.destination.is_some();
        let direction = plan
            .destination
            .map_or(0.0, |destination| destination.top - plan.view.top);
        let middle = (plan.view.top + plan.view.bottom) / 2.0;
        let mut verify_visible = Vec::new();
        let mut visible_rows = Vec::new();
        let mut nearby = Vec::new();
        let mut arriving = Vec::new();
        let mut verify_other = Vec::new();
        for (&index, page) in &plan.pages {
            let Some(rect) = pages.get(index).copied() else {
                continue;
            };
            let raster = self.pages.get(&index);
            let held =
                |bounds: &PxRect| raster.is_some_and(|raster| raster.tiles.contains_key(bounds));
            let current = raster.is_some_and(|raster| {
                signatures
                    .get(&index)
                    .is_some_and(|signature| raster.is_current(*signature))
            });
            let rows: BTreeSet<i32> = page.visible.iter().map(|bounds| bounds.y0).collect();
            let ahead: BTreeSet<PxRect> = page
                .margin
                .iter()
                .filter(|bounds| {
                    !rows.contains(&bounds.y0)
                        && (row_center(rect, **bounds, scale) - middle) * direction > 0.0
                })
                .copied()
                .collect();
            let arrival: BTreeSet<PxRect> = page
                .destination
                .iter()
                .filter(|bounds| !rows.contains(&bounds.y0) && !ahead.contains(bounds))
                .copied()
                .collect();
            let task = |class, bounds| FillTask {
                class,
                page: index,
                bounds,
            };
            if !current && raster.is_some_and(|raster| !raster.tiles.is_empty()) {
                let verify = task(
                    FillClass::Verify,
                    PxRect {
                        x0: 0,
                        y0: 0,
                        x1: page.size.0,
                        y1: page.size.1,
                    },
                );
                if !rows.is_empty() {
                    verify_visible.push(verify);
                } else if !ahead.is_empty() {
                    nearby.push(verify);
                } else if !arrival.is_empty() && direction != 0.0 {
                    arriving.push(verify);
                } else {
                    verify_other.push(verify);
                }
            }
            visible_rows.extend(
                page.required()
                    .into_iter()
                    .filter(|bounds| rows.contains(&bounds.y0) && !held(bounds))
                    .map(|bounds| task(FillClass::VisibleMissing, bounds)),
            );
            if decelerating {
                nearby.extend(
                    ahead
                        .iter()
                        .filter(|bounds| !held(bounds))
                        .map(|bounds| task(FillClass::MarginMissing, *bounds)),
                );
                arriving.extend(
                    arrival
                        .iter()
                        .filter(|bounds| !held(bounds))
                        .map(|bounds| task(FillClass::Destination, *bounds)),
                );
            } else {
                nearby.extend(
                    page.margin
                        .iter()
                        .filter(|bounds| !rows.contains(&bounds.y0) && !held(bounds))
                        .map(|bounds| task(FillClass::MarginMissing, *bounds)),
                );
            }
        }
        verify_visible.sort_by_key(|task| task.page);
        visible_rows.sort_by_key(|task| (task.page, task.bounds));
        if direction == 0.0 {
            order_fill_tasks(&mut nearby, pages, scale, plan.view);
            order_fill_tasks(&mut arriving, pages, scale, plan.view);
        } else {
            let reach = |task: &FillTask| {
                let rect = pages[task.page];
                let position = match task.class {
                    FillClass::Verify if direction > 0.0 => rect.top,
                    FillClass::Verify => rect.bottom,
                    _ => row_center(rect, task.bounds, scale),
                };
                position * direction.signum()
            };
            let arrival_order = |first: &FillTask, second: &FillTask| {
                reach(first)
                    .total_cmp(&reach(second))
                    .then_with(|| first.class.cmp(&second.class))
                    .then_with(|| first.page.cmp(&second.page))
                    .then_with(|| first.bounds.cmp(&second.bounds))
            };
            nearby.sort_by(arrival_order);
            arriving.sort_by(arrival_order);
        }
        order_fill_tasks(&mut verify_other, pages, scale, plan.view);
        verify_visible
            .into_iter()
            .chain(visible_rows)
            .chain(nearby)
            .chain(arriving)
            .chain(verify_other)
            .collect()
    }

    pub(super) fn debug(&self, signatures: &BTreeMap<usize, u64>) -> FrameDebug {
        let mut debug = FrameDebug {
            pending: Vec::new(),
            invalidated: Vec::new(),
        };
        let Some(plan) = &self.plan else {
            return debug;
        };
        for (&index, page) in &plan.pages {
            let Some(raster) = self.pages.get(&index) else {
                continue;
            };
            let reference = |bounds: &PxRect| FrameTileRef {
                page: index as u32,
                bounds: (*bounds).into(),
            };
            if !signatures
                .get(&index)
                .is_some_and(|signature| raster.is_current(*signature))
            {
                debug.invalidated.extend(
                    raster
                        .tiles
                        .iter()
                        .filter(|(_, tile)| tile.pixels.is_some())
                        .map(|(bounds, _)| reference(bounds)),
                );
            }
            for bounds in &page.required() {
                if raster.tiles.contains_key(bounds) {
                    continue;
                }
                if raster.stale.contains(bounds) {
                    debug.invalidated.push(reference(bounds));
                } else {
                    debug.pending.push(reference(bounds));
                }
            }
        }
        debug
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_viewport::PlacementInput;

    fn tile(x0: i32, y0: i32) -> PxRect {
        PxRect {
            x0,
            y0,
            x1: x0 + 512,
            y1: y0 + 512,
        }
    }

    fn pixels(value: u8) -> Option<Arc<[u8]>> {
        Some(Arc::from(vec![value; 4]))
    }

    fn paginated(page_width: f64) -> LayoutMode {
        LayoutMode::Paginated {
            page_width,
            page_height: 1024.0,
            margin_top: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
            margin_right: 0.0,
        }
    }

    fn page_plan(visible: &[PxRect], margin: &[PxRect], destination: &[PxRect]) -> PagePlan {
        PagePlan {
            size: (1024, 1024),
            visible: visible.iter().copied().collect(),
            margin: margin.iter().copied().collect(),
            destination: destination.iter().copied().collect(),
        }
    }

    fn planned<const N: usize>(
        destination: Option<Rect>,
        pages: [(usize, PagePlan); N],
    ) -> ViewportState {
        let mode = paginated(1024.0);
        let extents = [PageExtent {
            width: 1024.0,
            height: 1024.0,
        }; 3];
        let viewport = Viewport {
            width: 1024.0,
            height: 1024.0,
            occlusion_top: 0.0,
            occlusion_bottom: 0.0,
            device_scale: 1.0,
        };
        let request = ViewportRequest {
            scroll_x: 0.0,
            scroll_y: 0.0,
            width: 1024.0,
            height: 1024.0,
            occlusion_top: 0.0,
            occlusion_bottom: 0.0,
            device_scale: 1.0,
            header_height: 0.0,
            time_ms: 0.0,
            debug: false,
            destination: destination.map(|destination| destination.top),
        };
        let placement = ContentPlacement::new(PlacementInput {
            mode: &mode,
            pages: &extents,
            header_height: 0.0,
            viewport,
            zoom: 1.0,
        });
        let mut state = ViewportState::new();
        state.placement = Some(PlacementCache {
            key: PlacementKey {
                revision: Revision::INITIAL,
                mode,
                zoom: 1.0,
                viewport,
                header_height: 0.0,
            },
            extents: extents.to_vec(),
            placement,
        });
        state.plan = Some(FramePlan {
            request,
            revision: Revision::INITIAL,
            layout: FrameLayout::Continuous,
            view: Rect {
                left: 0.0,
                top: 0.0,
                right: 1024.0,
                bottom: 1024.0,
            },
            destination,
            pages: BTreeMap::from(pages),
        });
        state
    }

    fn classes<'a>(
        tasks: impl IntoIterator<Item = &'a FillTask>,
    ) -> Vec<(FillClass, usize, PxRect)> {
        tasks
            .into_iter()
            .map(|task| (task.class, task.page, task.bounds))
            .collect()
    }

    fn painted(version: u64) -> Tile {
        Tile {
            pixels: pixels(1),
            version,
        }
    }

    #[test]
    fn changes_set_new_versions_and_drop_tiles_that_stop_showing() {
        let row = [tile(0, 0), tile(512, 0)];
        let mut state = planned(None, [(0, page_plan(&row, &row, &[]))]);
        let raster = PageRaster::at(&mut state.pages, 0, 1.0);
        raster.signature = Some(7);
        raster.tiles.insert(
            tile(0, 0),
            Tile {
                pixels: pixels(1),
                version: 1,
            },
        );
        raster.tiles.insert(
            tile(512, 0),
            Tile {
                pixels: None,
                version: 2,
            },
        );
        let signatures = BTreeMap::from([(0, 7)]);

        let (tiles, sent) = state.take_changes(&signatures);
        assert_eq!(
            tiles,
            vec![FrameTile::Set {
                page: 0,
                bounds: tile(0, 0).into(),
                version: 1,
                pixels: 0,
            }]
        );
        assert_eq!(sent.len(), 1);
        assert!(state.take_changes(&signatures).0.is_empty());

        state
            .pages
            .get_mut(&0)
            .unwrap()
            .tiles
            .get_mut(&tile(0, 0))
            .unwrap()
            .version = 3;
        assert_eq!(
            state.take_changes(&signatures).0,
            vec![FrameTile::Set {
                page: 0,
                bounds: tile(0, 0).into(),
                version: 3,
                pixels: 0,
            }]
        );

        state.pages.get_mut(&0).unwrap().signature = Some(8);
        let drop = vec![FrameTile::Drop {
            page: 0,
            bounds: tile(0, 0).into(),
        }];
        assert_eq!(state.take_changes(&signatures).0, drop);

        state.pages.get_mut(&0).unwrap().signature = Some(7);
        assert_eq!(state.take_changes(&signatures).0.len(), 1);
        state.pages.clear();
        assert_eq!(state.take_changes(&signatures).0, drop);
        assert!(state.emitted.is_empty());
    }

    #[test]
    fn page_rasters_reset_when_the_raster_scale_changes() {
        let mut pages = BTreeMap::new();
        let raster = PageRaster::at(&mut pages, 3, 2.0);
        raster.signature = Some(1);
        raster.tiles.insert(
            tile(0, 0),
            Tile {
                pixels: pixels(1),
                version: 1,
            },
        );
        raster.stale.insert(tile(0, 512));
        assert_eq!(PageRaster::at(&mut pages, 3, 2.0).tiles.len(), 1);
        let reset = PageRaster::at(&mut pages, 3, 3.0);
        assert!(reset.tiles.is_empty() && reset.stale.is_empty());
        assert_eq!(reset.signature, None);
        assert_eq!(reset.scale, 3.0);
    }

    #[test]
    fn invalidated_tiles_become_stale_and_retain_forgets_them() {
        let mut pages = BTreeMap::new();
        let raster = PageRaster::at(&mut pages, 0, 1.0);
        for bounds in [tile(0, 0), tile(0, 512)] {
            raster.tiles.insert(
                bounds,
                Tile {
                    pixels: pixels(1),
                    version: 1,
                },
            );
        }
        let damage = [IRect {
            x0: 10,
            y0: 600,
            x1: 20,
            y1: 610,
        }];
        assert!(
            raster
                .invalidate(&damage, &BTreeSet::from([tile(0, 0)]))
                .is_empty()
        );
        assert_eq!(
            raster.tiles.keys().copied().collect::<Vec<_>>(),
            vec![tile(0, 0)]
        );
        assert_eq!(
            raster.stale.iter().copied().collect::<Vec<_>>(),
            vec![tile(0, 512)]
        );
        raster.retain(&BTreeSet::from([tile(0, 0)]));
        assert!(raster.stale.is_empty());
        assert_eq!(raster.tiles.len(), 1);

        raster.tiles.insert(
            tile(0, 512),
            Tile {
                pixels: pixels(1),
                version: 2,
            },
        );
        assert_eq!(
            raster.invalidate(&damage, &BTreeSet::from([tile(0, 512)])),
            vec![tile(0, 512)]
        );
        assert_eq!(raster.tiles.len(), 2);
        assert!(raster.stale.is_empty());
    }

    #[test]
    fn zoom_is_recomputed_only_when_the_layout_key_changes() {
        let mut state = ViewportState::new();
        assert_eq!(state.zoom(&paginated(800.0), 400.0), 0.5);
        assert_eq!(state.zoom(&paginated(800.0), 200.0), 0.5);
        assert_eq!(state.zoom(&paginated(1000.0), 200.0), 0.2);
        assert_eq!(
            state.zoom(&LayoutMode::Continuous { max_width: 600.0 }, 200.0),
            1.0
        );
    }

    #[test]
    fn fill_tasks_order_visible_rows_then_margin_then_hidden_page_verification() {
        let mut state = planned(
            None,
            [
                (
                    0,
                    page_plan(
                        &[tile(0, 0), tile(512, 512)],
                        &[tile(0, 0), tile(512, 512), tile(0, 512)],
                        &[],
                    ),
                ),
                (1, page_plan(&[], &[tile(0, 0), tile(0, 512)], &[])),
            ],
        );
        let current = PageRaster::at(&mut state.pages, 0, 1.0);
        current.signature = Some(1);
        current.tiles.insert(
            tile(512, 512),
            Tile {
                pixels: pixels(1),
                version: 1,
            },
        );
        let unverified = PageRaster::at(&mut state.pages, 1, 1.0);
        unverified.signature = Some(1);
        unverified.tiles.insert(
            tile(0, 0),
            Tile {
                pixels: pixels(1),
                version: 2,
            },
        );
        let signatures = BTreeMap::from([(0, 1), (1, 2)]);

        let tasks = state.fill_tasks(&signatures);
        assert_eq!(
            tasks
                .iter()
                .map(|task| (task.class, task.page, task.bounds))
                .collect::<Vec<_>>(),
            vec![
                (FillClass::VisibleMissing, 0, tile(0, 0)),
                (FillClass::VisibleMissing, 0, tile(0, 512)),
                (FillClass::MarginMissing, 1, tile(0, 512)),
                (
                    FillClass::Verify,
                    1,
                    PxRect {
                        x0: 0,
                        y0: 0,
                        x1: 1024,
                        y1: 1024,
                    }
                ),
            ]
        );
    }

    #[test]
    fn changes_show_a_row_only_when_every_required_tile_of_it_is_rasterized() {
        let set = |bounds: PxRect, version: u64, pixels: u32| FrameTile::Set {
            page: 0,
            bounds: bounds.into(),
            version,
            pixels,
        };
        let drop = |bounds: PxRect| FrameTile::Drop {
            page: 0,
            bounds: bounds.into(),
        };
        let mut state = planned(
            None,
            [(
                0,
                page_plan(
                    &[tile(0, 0), tile(512, 0)],
                    &[tile(0, 0), tile(512, 0), tile(0, 512), tile(512, 512)],
                    &[tile(0, 1024), tile(512, 1024)],
                ),
            )],
        );
        let raster = PageRaster::at(&mut state.pages, 0, 1.0);
        raster.signature = Some(7);
        raster.tiles.insert(tile(0, 0), painted(1));
        raster.tiles.insert(tile(0, 512), painted(2));
        raster.tiles.insert(
            tile(512, 512),
            Tile {
                pixels: None,
                version: 3,
            },
        );
        raster.tiles.insert(tile(0, 1024), painted(4));
        let signatures = BTreeMap::from([(0, 7)]);

        let (tiles, sent) = state.take_changes(&signatures);
        assert_eq!(tiles, vec![set(tile(0, 512), 2, 0)]);
        assert_eq!(sent.len(), 1);

        let raster = state.pages.get_mut(&0).unwrap();
        raster.tiles.insert(tile(512, 0), painted(5));
        raster.tiles.insert(tile(512, 1024), painted(6));
        let (tiles, _) = state.take_changes(&signatures);
        assert_eq!(
            tiles,
            vec![
                set(tile(0, 0), 1, 0),
                set(tile(512, 0), 5, 1),
                set(tile(0, 1024), 4, 2),
                set(tile(512, 1024), 6, 3),
            ]
        );

        state.pages.get_mut(&0).unwrap().tiles.remove(&tile(512, 0));
        assert_eq!(
            state.take_changes(&signatures).0,
            vec![drop(tile(0, 0)), drop(tile(512, 0))]
        );
        state
            .pages
            .get_mut(&0)
            .unwrap()
            .tiles
            .insert(tile(512, 0), painted(8));
        assert_eq!(
            state.take_changes(&signatures).0,
            vec![set(tile(0, 0), 1, 0), set(tile(512, 0), 8, 1),]
        );
    }

    #[test]
    fn deceleration_fills_visible_rows_then_rows_ahead_then_destination_rows_in_arrival_order() {
        let destination = Rect {
            left: 0.0,
            top: 1560.0,
            right: 1024.0,
            bottom: 2584.0,
        };
        let whole = PxRect {
            x0: 0,
            y0: 0,
            x1: 1024,
            y1: 1024,
        };
        let pages = || {
            [
                (
                    0,
                    page_plan(
                        &[tile(0, 0)],
                        &[tile(0, 0), tile(0, 512), tile(512, 512)],
                        &[tile(0, 512), tile(512, 512)],
                    ),
                ),
                (
                    1,
                    page_plan(
                        &[tile(0, 0)],
                        &[tile(0, 0)],
                        &[tile(0, 0), tile(512, 0), tile(0, 512), tile(512, 512)],
                    ),
                ),
                (2, page_plan(&[], &[tile(0, 0)], &[])),
            ]
        };
        let mut state = planned(Some(destination), pages());
        for page in [1, 2] {
            let unverified = PageRaster::at(&mut state.pages, page, 1.0);
            unverified.signature = Some(1);
            unverified.tiles.insert(tile(0, 0), painted(1));
        }
        let signatures = BTreeMap::from([(0, 1), (1, 2), (2, 2)]);

        let tasks = state.fill_tasks(&signatures);
        assert_eq!(
            classes(&tasks),
            vec![
                (FillClass::Verify, 1, whole),
                (FillClass::VisibleMissing, 0, tile(0, 0)),
                (FillClass::VisibleMissing, 1, tile(512, 0)),
                (FillClass::MarginMissing, 0, tile(0, 512)),
                (FillClass::MarginMissing, 0, tile(512, 512)),
                (FillClass::Verify, 2, whole),
                (FillClass::Destination, 1, tile(0, 512)),
                (FillClass::Destination, 1, tile(512, 512)),
            ]
        );

        let mut normal = planned(
            None,
            [
                (
                    0,
                    page_plan(&[tile(0, 0)], &[tile(0, 0), tile(0, 512)], &[]),
                ),
                (1, page_plan(&[tile(0, 0)], &[tile(0, 0)], &[])),
                (2, page_plan(&[], &[tile(0, 0)], &[])),
            ],
        );
        normal.pages = std::mem::take(&mut state.pages);
        assert_eq!(
            classes(&normal.fill_tasks(&signatures)),
            vec![
                (FillClass::Verify, 1, whole),
                (FillClass::VisibleMissing, 0, tile(0, 0)),
                (FillClass::MarginMissing, 0, tile(0, 512)),
                (FillClass::Verify, 2, whole),
            ]
        );
    }

    #[test]
    fn resting_deceleration_verifies_hidden_pages_last_like_the_normal_order() {
        let whole = PxRect {
            x0: 0,
            y0: 0,
            x1: 1024,
            y1: 1024,
        };
        let row = [tile(0, 0), tile(512, 0)];
        let pages = |prefill: bool| {
            let destination = |tiles: &[PxRect]| if prefill { tiles.to_vec() } else { Vec::new() };
            [
                (0, page_plan(&row, &row, &destination(&row))),
                (
                    1,
                    page_plan(&[], &[tile(0, 0)], &destination(&[tile(0, 0)])),
                ),
                (
                    2,
                    page_plan(&[], &[tile(0, 0)], &destination(&[tile(0, 0)])),
                ),
            ]
        };
        let resting = Rect {
            left: 0.0,
            top: 0.0,
            right: 1024.0,
            bottom: 1024.0,
        };
        let mut state = planned(Some(resting), pages(true));
        let stale = PageRaster::at(&mut state.pages, 1, 1.0);
        stale.signature = Some(1);
        stale.tiles.insert(tile(0, 0), painted(1));
        let signatures = BTreeMap::from([(1, 2)]);
        assert_eq!(
            classes(&state.fill_tasks(&signatures)),
            vec![
                (FillClass::VisibleMissing, 0, tile(0, 0)),
                (FillClass::VisibleMissing, 0, tile(512, 0)),
                (FillClass::Destination, 2, tile(0, 0)),
                (FillClass::Verify, 1, whole),
            ]
        );

        let mut normal = planned(None, pages(false));
        normal.pages = std::mem::take(&mut state.pages);
        assert_eq!(
            classes(&normal.fill_tasks(&signatures)),
            vec![
                (FillClass::VisibleMissing, 0, tile(0, 0)),
                (FillClass::VisibleMissing, 0, tile(512, 0)),
                (FillClass::MarginMissing, 2, tile(0, 0)),
                (FillClass::Verify, 1, whole),
            ]
        );
    }

    fn ahead_order(
        view: Rect,
        destination: f64,
        pages: [(usize, PagePlan); 3],
    ) -> Vec<(FillClass, usize, PxRect)> {
        let mut state = planned(
            Some(Rect {
                left: 0.0,
                top: destination,
                right: 1024.0,
                bottom: destination + 1024.0,
            }),
            pages,
        );
        state.plan.as_mut().unwrap().view = view;
        let stale = PageRaster::at(&mut state.pages, 1, 1.0);
        stale.signature = Some(1);
        for bounds in [tile(0, 0), tile(512, 0)] {
            stale.tiles.insert(bounds, painted(1));
        }
        classes(&state.fill_tasks(&BTreeMap::from([(1, 2)])))
    }

    const WHOLE: PxRect = PxRect {
        x0: 0,
        y0: 0,
        x1: 1024,
        y1: 1024,
    };

    #[test]
    fn rows_ahead_downward_take_a_stale_page_verification_where_the_screen_reaches_it() {
        let upper = [tile(0, 0), tile(512, 0)];
        let both = [upper, [tile(0, 512), tile(512, 512)]].concat();
        let order = ahead_order(
            Rect {
                left: 0.0,
                top: 0.0,
                right: 1024.0,
                bottom: 1024.0,
            },
            3000.0,
            [
                (0, page_plan(&upper, &both, &[])),
                (1, page_plan(&[], &upper, &[])),
                (2, page_plan(&[], &upper, &[])),
            ],
        );
        assert_eq!(
            order,
            vec![
                (FillClass::VisibleMissing, 0, tile(0, 0)),
                (FillClass::VisibleMissing, 0, tile(512, 0)),
                (FillClass::MarginMissing, 0, tile(0, 512)),
                (FillClass::MarginMissing, 0, tile(512, 512)),
                (FillClass::Verify, 1, WHOLE),
                (FillClass::MarginMissing, 2, tile(0, 0)),
                (FillClass::MarginMissing, 2, tile(512, 0)),
            ]
        );
    }

    #[test]
    fn rows_ahead_upward_take_a_stale_page_verification_where_the_screen_reaches_it() {
        let upper = [tile(0, 0), tile(512, 0)];
        let lower = [tile(0, 512), tile(512, 512)];
        let both = [upper, lower].concat();
        let order = ahead_order(
            Rect {
                left: 0.0,
                top: 2096.0,
                right: 1024.0,
                bottom: 3120.0,
            },
            -500.0,
            [
                (0, page_plan(&[], &lower, &[])),
                (1, page_plan(&[], &upper, &[])),
                (2, page_plan(&lower, &both, &[])),
            ],
        );
        assert_eq!(
            order,
            vec![
                (FillClass::VisibleMissing, 2, tile(0, 512)),
                (FillClass::VisibleMissing, 2, tile(512, 512)),
                (FillClass::MarginMissing, 2, tile(0, 0)),
                (FillClass::MarginMissing, 2, tile(512, 0)),
                (FillClass::Verify, 1, WHOLE),
                (FillClass::MarginMissing, 0, tile(0, 512)),
                (FillClass::MarginMissing, 0, tile(512, 512)),
            ]
        );
    }

    #[test]
    fn deceleration_leads_only_in_the_direction_of_motion() {
        let pages = || {
            [
                (
                    0,
                    page_plan(&[tile(0, 512)], &[tile(0, 0), tile(0, 512)], &[]),
                ),
                (1, page_plan(&[], &[tile(0, 0)], &[])),
            ]
        };
        let at = |top: f64| {
            let state = planned(
                Some(Rect {
                    left: 0.0,
                    top,
                    right: 1024.0,
                    bottom: top + 1024.0,
                }),
                pages(),
            );
            let tasks = state.fill_tasks(&BTreeMap::new());
            classes(&tasks)
        };
        let visible = (FillClass::VisibleMissing, 0, tile(0, 512));
        assert_eq!(
            at(-500.0),
            vec![visible, (FillClass::MarginMissing, 0, tile(0, 0))]
        );
        assert_eq!(
            at(1560.0),
            vec![visible, (FillClass::MarginMissing, 1, tile(0, 0))]
        );
        assert_eq!(at(0.0), vec![visible]);
    }

    #[test]
    fn normal_fills_verify_visible_pages_then_visible_rows_then_margin_then_other_pages() {
        let mut state = planned(
            None,
            [
                (
                    0,
                    page_plan(
                        &[tile(0, 0)],
                        &[tile(0, 0), tile(512, 0), tile(0, 512), tile(512, 512)],
                        &[],
                    ),
                ),
                (1, page_plan(&[], &[tile(0, 0), tile(512, 0)], &[])),
            ],
        );
        for page in [0, 1] {
            let unverified = PageRaster::at(&mut state.pages, page, 1.0);
            unverified.signature = Some(1);
            unverified.tiles.insert(tile(512, 512), painted(1));
        }
        let signatures = BTreeMap::from([(0, 2), (1, 2)]);
        let whole = PxRect {
            x0: 0,
            y0: 0,
            x1: 1024,
            y1: 1024,
        };
        assert_eq!(
            classes(&state.fill_tasks(&signatures)),
            vec![
                (FillClass::Verify, 0, whole),
                (FillClass::VisibleMissing, 0, tile(0, 0)),
                (FillClass::VisibleMissing, 0, tile(512, 0)),
                (FillClass::MarginMissing, 0, tile(0, 512)),
                (FillClass::MarginMissing, 1, tile(0, 0)),
                (FillClass::MarginMissing, 1, tile(512, 0)),
                (FillClass::Verify, 1, whole),
            ]
        );
    }
}
