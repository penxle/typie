use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

use editor_core::TickResult;
use editor_renderer::backend::cpu::CpuSink;
use editor_renderer::damage::IRect;
use editor_renderer::diff::diff;
use editor_viewport::{
    ContentPlacement, FillClass, FillTask, LayoutMode, PageExtent, PlacementInput, PxRect, Rect,
    TileArea, Viewport, approach_view, continuous_engine_width, current_page, full_width,
    margin_view, page_view_set, pages_with_height, percent, raster_size, required_tiles,
    tile_region,
};

use super::raster;
use super::state::{
    Counter, FramePlan, PagePlan, PageRaster, PlacementCache, PlacementKey, Tile, ViewportState,
};
use super::{
    EditorFrame, FrameBody, FrameGeometry, FrameLayout, FramePage, FramePosition, ViewportRequest,
};
use crate::prelude::*;

pub(super) fn frame(
    core: &mut editor_core::Editor,
    state: &mut ViewportState,
    request: ViewportRequest,
) -> EditorResult<EditorFrame> {
    let (mode, _) = document_layout(core);
    let zoom = state.zoom(&mode, request.width);
    if let LayoutMode::Continuous { .. } = mode
        && request.width.is_finite()
        && request.width > 0.0
    {
        let width = continuous_engine_width(request.width, zoom) as f32;
        if core.view().viewport().width != width {
            core.enqueue_request(vec![editor_core::Message::System {
                event: editor_core::SystemEvent::Resize {
                    width,
                    height: request.height as f32,
                    scale_factor: request.device_scale,
                },
            }])?;
        }
    }
    let tick = core.tick()?;
    let revision = core.revision();
    let (mode, layout) = document_layout(core);
    let zoom = state.zoom(&mode, request.width);
    let viewport = Viewport {
        width: request.width,
        height: request.height,
        occlusion_top: request.occlusion_top,
        occlusion_bottom: request.occlusion_bottom,
        device_scale: request.device_scale,
    };
    let key = PlacementKey {
        revision,
        mode,
        zoom,
        viewport,
        header_height: request.header_height,
    };
    let first = state.plan.is_none();
    let edit = state
        .plan
        .as_ref()
        .is_none_or(|plan| plan.revision != revision);
    let cache = match state.placement.take() {
        Some(cache) if cache.key == key => cache,
        _ => placement_cache(core, key),
    };
    let cache = &*state.placement.insert(cache);
    let placement = &cache.placement;
    let top = placement.clamp_scroll_y(request.scroll_y);
    let left = placement.clamp_scroll_x(request.scroll_x);
    let view = Rect {
        left,
        top,
        right: left + request.width,
        bottom: top + request.height,
    };
    let destination = request
        .destination
        .filter(|destination| destination.is_finite())
        .map(|destination| {
            let top = placement.clamp_scroll_y(destination);
            Rect {
                left: view.left,
                top,
                right: view.right,
                bottom: top + view.height(),
            }
        });
    let prefill =
        destination.map(|destination| (approach_view(view, destination.top), destination));
    let mut view_set = page_view_set(&placement.pages, top, view.bottom, &state.active);
    if let Some((approach, destination)) = prefill {
        let margin = margin_view(destination);
        view_set.extend(pages_with_height(
            &placement.pages,
            approach.top.min(margin.top),
            approach.bottom.max(margin.bottom),
        ));
    }
    state.pages.retain(|index, _| view_set.contains(index));
    let scale = placement.raster_scale();
    let mut pages = BTreeMap::new();
    let mut signatures = BTreeMap::new();
    for &index in &view_set {
        let extent = cache.extents[index];
        let Some(size) = raster_size(extent.width, extent.height, scale) else {
            continue;
        };
        let rect = placement.pages[index];
        let local = |region, area| tile_region(rect, region, placement.zoom, area);
        let rows = |region, area| {
            tiles_in(
                index,
                extent,
                scale,
                full_width(local(region, area), extent.width),
            )
        };
        let visible = tiles_in(index, extent, scale, local(view, TileArea::Visible));
        let mut margin = rows(view, TileArea::Margin);
        margin.extend(visible.iter().copied());
        let mut prefilled = BTreeSet::new();
        if let Some((approach, destination)) = prefill {
            prefilled.extend(rows(approach, TileArea::Visible));
            prefilled.extend(rows(destination, TileArea::Margin));
        }
        let plan = PagePlan {
            size,
            visible,
            margin,
            destination: prefilled,
        };
        let required = plan.required();
        let signature = core.page_render_signature(index as u32);
        signatures.insert(index, signature);
        let raster = PageRaster::at(&mut state.pages, index, scale);
        raster.retain(&required);
        if edit && !plan.visible.is_empty() {
            let fresh = raster.display_list.is_none();
            let Some(damage) = refresh(
                core,
                raster,
                index,
                signature,
                &plan.visible,
                &mut state.scratch,
                &mut state.versions,
            ) else {
                continue;
            };
            let shown: Vec<PxRect> = state
                .emitted
                .keys()
                .filter(|(page, _)| *page == index)
                .map(|&(_, bounds)| bounds)
                .collect();
            let damage = if fresh && shown.is_empty() {
                Vec::new()
            } else {
                damage
            };
            let rows: BTreeSet<i32> = plan
                .visible
                .iter()
                .filter(|&&bounds| {
                    first
                        || raster::is_damaged(bounds, &damage)
                        || shown.iter().any(|held| overlaps(*held, bounds))
                })
                .map(|bounds| bounds.y0)
                .collect();
            for &bounds in &required {
                if rows.contains(&bounds.y0) && !raster.tiles.contains_key(&bounds) {
                    rasterize_tile(raster, bounds, &mut state.scratch, &mut state.versions);
                }
            }
        }
        pages.insert(index, plan);
    }
    state.active = view_set;
    state.plan = Some(FramePlan {
        request,
        revision,
        layout,
        view,
        destination,
        pages,
    });
    Ok(assemble(state, &signatures, tick))
}

pub(super) fn fill(
    core: &mut editor_core::Editor,
    state: &mut ViewportState,
    budget_ms: f64,
) -> Option<EditorFrame> {
    let (Some(plan), Some(cache)) = (&state.plan, &state.placement) else {
        return None;
    };
    if core.revision() != plan.revision {
        return None;
    }
    let scale = cache.placement.raster_scale();
    let signatures: BTreeMap<usize, u64> = plan
        .pages
        .keys()
        .map(|&index| (index, core.page_render_signature(index as u32)))
        .collect();
    let budget = Duration::try_from_secs_f64(budget_ms / 1000.0).unwrap_or(if budget_ms > 0.0 {
        Duration::MAX
    } else {
        Duration::ZERO
    });
    let start = Instant::now();
    let mut performed = false;
    while let Some(task) = state.fill_tasks(&signatures).first().copied() {
        if !perform(core, state, task, signatures[&task.page], scale) {
            break;
        }
        performed = true;
        if start.elapsed() >= budget {
            break;
        }
    }
    performed.then(|| assemble(state, &signatures, None))
}

fn perform(
    core: &mut editor_core::Editor,
    state: &mut ViewportState,
    task: FillTask,
    signature: u64,
    scale: f64,
) -> bool {
    let raster = PageRaster::at(&mut state.pages, task.page, scale);
    if refresh(
        core,
        raster,
        task.page,
        signature,
        &BTreeSet::new(),
        &mut state.scratch,
        &mut state.versions,
    )
    .is_none()
    {
        return false;
    }
    if task.class != FillClass::Verify && !raster.tiles.contains_key(&task.bounds) {
        rasterize_tile(raster, task.bounds, &mut state.scratch, &mut state.versions);
    }
    true
}

fn refresh(
    core: &mut editor_core::Editor,
    raster: &mut PageRaster,
    index: usize,
    signature: u64,
    visible: &BTreeSet<PxRect>,
    scratch: &mut CpuSink,
    versions: &mut Counter,
) -> Option<Vec<IRect>> {
    if raster.is_current(signature) {
        return Some(Vec::new());
    }
    let (list, bounds) = core.build_display_list(index as u32, raster.scale as f32)?;
    let damage = match &raster.display_list {
        Some(previous) => diff(previous, &list, bounds),
        None => vec![bounds],
    };
    for bounds in raster.invalidate(&damage, visible) {
        let tile = raster.tiles.get_mut(&bounds).expect("held tile");
        tile.pixels = raster::repaint(
            &list.primitives,
            bounds,
            &damage,
            tile.pixels.take(),
            scratch,
        );
        tile.version = versions.next();
    }
    raster.display_list = Some(list);
    raster.signature = Some(signature);
    Some(damage)
}

fn rasterize_tile(
    raster: &mut PageRaster,
    bounds: PxRect,
    scratch: &mut CpuSink,
    versions: &mut Counter,
) {
    let list = raster
        .display_list
        .as_ref()
        .expect("current page has a list");
    let pixels = raster::rasterize(&list.primitives, bounds, scratch);
    raster.tiles.insert(
        bounds,
        Tile {
            pixels,
            version: versions.next(),
        },
    );
    raster.stale.remove(&bounds);
}

fn overlaps(first: PxRect, second: PxRect) -> bool {
    first.x0 < second.x1 && second.x0 < first.x1 && first.y0 < second.y1 && second.y0 < first.y1
}

fn assemble(
    state: &mut ViewportState,
    signatures: &BTreeMap<usize, u64>,
    tick: Option<TickResult>,
) -> EditorFrame {
    let (tiles, pixels) = state.take_changes(signatures);
    let fill_remaining = !state.fill_tasks(signatures).is_empty();
    let id = state.frame_ids.next();
    let plan = state.plan.as_ref().expect("a frame plan precedes assembly");
    let placement = &state
        .placement
        .as_ref()
        .expect("a placement precedes assembly")
        .placement;
    let request = plan.request;
    let top = plan.view.top;
    let debug = request.debug.then(|| state.debug(signatures));
    EditorFrame {
        geometry: FrameGeometry {
            id,
            revision: plan.revision,
            tick,
            layout: plan.layout,
            zoom: placement.zoom,
            raster_scale: placement.raster_scale(),
            content_width: placement.content_width,
            content_height: placement.content_height,
            page_count: placement.page_count() as u32,
            body: FrameBody {
                pages_top: placement.pages_top,
                top_spacer: placement.top_spacer,
                pages_bottom: placement.pages_bottom(),
                bottom_padding: placement.bottom_padding,
                minimum_body_bottom: placement.minimum_body_bottom(),
            },
            pages: plan
                .pages
                .keys()
                .map(|&index| {
                    let rect = placement.pages[index];
                    FramePage {
                        index: index as u32,
                        x: rect.left,
                        y: rect.top,
                        width: rect.width(),
                        height: rect.height(),
                    }
                })
                .collect(),
            tiles,
            position: current_page(placement, top, request.height).map(|page| FramePosition {
                page: page as u32 + 1,
                pages: placement.page_count() as u32,
                percent: percent(top, placement.content_height, request.height),
            }),
            debug,
            needs_next_frame: false,
            fill_remaining,
        },
        pixels,
    }
}

fn placement_cache(core: &editor_core::Editor, key: PlacementKey) -> PlacementCache {
    let extents: Vec<PageExtent> = core
        .view()
        .pages()
        .iter()
        .map(|page| PageExtent {
            width: f64::from(page.size.width),
            height: f64::from(page.size.height),
        })
        .collect();
    let placement = ContentPlacement::new(PlacementInput {
        mode: &key.mode,
        pages: &extents,
        header_height: key.header_height,
        viewport: key.viewport,
        zoom: key.zoom,
    });
    PlacementCache {
        key,
        extents,
        placement,
    }
}

fn document_layout(core: &editor_core::Editor) -> (LayoutMode, FrameLayout) {
    let mode = crate::root::attrs(&core.state().view())
        .map(|root| root.layout_mode)
        .unwrap_or_default();
    match mode {
        editor_model::LayoutMode::Paginated {
            page_width,
            page_height,
            page_margin_top,
            page_margin_bottom,
            page_margin_left,
            page_margin_right,
        } => (
            LayoutMode::Paginated {
                page_width: f64::from(page_width),
                page_height: f64::from(page_height),
                margin_top: f64::from(page_margin_top),
                margin_bottom: f64::from(page_margin_bottom),
                margin_left: f64::from(page_margin_left),
                margin_right: f64::from(page_margin_right),
            },
            FrameLayout::Paginated {
                margin_top: f64::from(page_margin_top),
                margin_bottom: f64::from(page_margin_bottom),
                margin_left: f64::from(page_margin_left),
                margin_right: f64::from(page_margin_right),
            },
        ),
        editor_model::LayoutMode::Continuous { max_width } => (
            LayoutMode::Continuous {
                max_width: f64::from(max_width),
            },
            FrameLayout::Continuous,
        ),
    }
}

fn tiles_in(index: usize, extent: PageExtent, scale: f64, region: Rect) -> BTreeSet<PxRect> {
    match required_tiles(extent.width, extent.height, scale, &[region]) {
        Ok(tiles) => tiles.into_iter().collect(),
        Err(error) => {
            log::error!("page {index}: {error}");
            BTreeSet::new()
        }
    }
}
