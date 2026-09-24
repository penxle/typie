use std::sync::Arc;

use editor_renderer::backend::cpu::CpuSink;
use editor_renderer::damage::IRect;
use editor_renderer::diff::raster_rect;
use editor_renderer::display_list::Primitive;
use editor_viewport::{PxRect, TILE_SIZE};

pub(super) fn scratch() -> CpuSink {
    CpuSink::new((TILE_SIZE + 2) as u16, (TILE_SIZE + 2) as u16)
}

pub(super) fn is_damaged(bounds: PxRect, damage: &[IRect]) -> bool {
    let raster = with_gutter(bounds);
    damage.iter().any(|rect| rect.intersect(raster).is_some())
}

pub(super) fn rasterize(
    primitives: &[Primitive],
    bounds: PxRect,
    scratch: &mut CpuSink,
) -> Option<Arc<[u8]>> {
    let raster = with_gutter(bounds);
    if !paints(primitives, raster) {
        return None;
    }
    raster_rect(primitives, raster, scratch);
    let stride = raster.width() as usize * 4;
    let mut pixels = vec![0; stride * raster.height() as usize];
    scratch.read_back_rect(
        &mut pixels,
        stride,
        IRect {
            x0: 0,
            y0: 0,
            x1: raster.width(),
            y1: raster.height(),
        },
    );
    Some(Arc::from(pixels))
}

pub(super) fn repaint(
    primitives: &[Primitive],
    bounds: PxRect,
    damage: &[IRect],
    pixels: Option<Arc<[u8]>>,
    scratch: &mut CpuSink,
) -> Option<Arc<[u8]>> {
    let raster = with_gutter(bounds);
    if !paints(primitives, raster) {
        return None;
    }
    let Some(mut pixels) = pixels else {
        return rasterize(primitives, bounds, scratch);
    };
    let Some(dirty) = damage
        .iter()
        .filter_map(|rect| rect.intersect(raster))
        .reduce(IRect::union)
    else {
        return Some(pixels);
    };
    raster_rect(primitives, dirty, scratch);
    let target = Arc::make_mut(&mut pixels);
    let stride = raster.width() as usize * 4;
    let row_bytes = dirty.width() as usize * 4;
    for y in 0..dirty.height() {
        let offset =
            (dirty.y0 - raster.y0 + y) as usize * stride + (dirty.x0 - raster.x0) as usize * 4;
        scratch.read_back_rect(
            &mut target[offset..offset + row_bytes],
            row_bytes,
            IRect {
                x0: 0,
                y0: y,
                x1: dirty.width(),
                y1: y + 1,
            },
        );
    }
    Some(pixels)
}

fn with_gutter(bounds: PxRect) -> IRect {
    IRect {
        x0: bounds.x0 - 1,
        y0: bounds.y0 - 1,
        x1: bounds.x1 + 1,
        y1: bounds.y1 + 1,
    }
}

fn paints(primitives: &[Primitive], raster: IRect) -> bool {
    primitives
        .iter()
        .any(|primitive| primitive.bounds.intersect(raster).is_some())
}

#[cfg(test)]
#[track_caller]
pub(super) fn assert_same_pixels(
    actual: Option<&[u8]>,
    expected: Option<&[u8]>,
    context: std::fmt::Arguments<'_>,
) {
    let (actual, expected) = match (actual, expected) {
        (Some(actual), Some(expected)) => (actual, expected),
        (None, None) => return,
        (actual, expected) => panic!(
            "{context}: painted bytes {:?} vs expected {:?}",
            actual.map(<[u8]>::len),
            expected.map(<[u8]>::len)
        ),
    };
    if actual == expected {
        return;
    }
    let index = actual
        .iter()
        .zip(expected)
        .position(|(left, right)| left != right)
        .unwrap_or(actual.len().min(expected.len()));
    let around = |bytes: &[u8]| {
        let start = index.saturating_sub(4).min(bytes.len());
        bytes[start..(index + 4).min(bytes.len())].to_vec()
    };
    panic!(
        "{context}: lengths {} vs {}, first difference at byte {index}: {:?} vs {:?}",
        actual.len(),
        expected.len(),
        around(actual),
        around(expected)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_common::{Color, Rect};
    use editor_renderer::display_list::{DisplayList, DisplayListRecorder};
    use editor_renderer::sink::RenderSink;
    use editor_renderer::types::Transform;

    fn page(width: i32, height: i32) -> IRect {
        IRect {
            x0: 0,
            y0: 0,
            x1: width,
            y1: height,
        }
    }

    fn grid(width: i32, height: i32) -> Vec<PxRect> {
        let mut tiles = Vec::new();
        for y0 in (0..height).step_by(TILE_SIZE as usize) {
            for x0 in (0..width).step_by(TILE_SIZE as usize) {
                tiles.push(PxRect {
                    x0,
                    y0,
                    x1: (x0 + TILE_SIZE).min(width),
                    y1: (y0 + TILE_SIZE).min(height),
                });
            }
        }
        tiles
    }

    fn full_raster(list: &DisplayList, bounds: IRect) -> Vec<u8> {
        let mut sink = CpuSink::new(bounds.width() as u16, bounds.height() as u16);
        raster_rect(&list.primitives, bounds, &mut sink);
        let mut pixels = vec![0; bounds.width() as usize * bounds.height() as usize * 4];
        sink.read_back_rect(&mut pixels, bounds.width() as usize * 4, bounds);
        pixels
    }

    fn assert_matches_page(tile: &[u8], bounds: PxRect, page_pixels: &[u8], page: IRect) {
        let stride = (bounds.x1 - bounds.x0 + 2) as usize * 4;
        assert_eq!(tile.len(), stride * (bounds.y1 - bounds.y0 + 2) as usize);
        for y in (bounds.y0 - 1).max(0)..(bounds.y1 + 1).min(page.y1) {
            for x in (bounds.x0 - 1).max(0)..(bounds.x1 + 1).min(page.x1) {
                let actual =
                    (y - bounds.y0 + 1) as usize * stride + (x - bounds.x0 + 1) as usize * 4;
                let expected = (y as usize * page.x1 as usize + x as usize) * 4;
                assert_eq!(
                    &tile[actual..actual + 4],
                    &page_pixels[expected..expected + 4],
                    "pixel ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn blank_tiles_have_no_pixels_and_painted_tiles_keep_the_gutter() {
        let bounds = page(2048, 512);
        let mut recorder = DisplayListRecorder::new(bounds);
        recorder.fill_rect(
            Rect::from_xywh(512.25, 10.25, 8.5, 20.5),
            Color::new(80, 120, 160, 127),
            Transform::IDENTITY,
        );
        recorder.fill_rect(
            Rect::from_xywh(1540.0, 10.0, 20.0, 20.0),
            Color::new(160, 120, 80, 255),
            Transform::IDENTITY,
        );
        let list = recorder.into_list();
        let expected = full_raster(&list, bounds);
        let mut sink = scratch();
        let tiles: Vec<_> = grid(2048, 512)
            .into_iter()
            .map(|tile| (tile, rasterize(&list.primitives, tile, &mut sink)))
            .collect();
        assert_eq!(
            tiles
                .iter()
                .map(|(tile, pixels)| (tile.x0, pixels.is_some()))
                .collect::<Vec<_>>(),
            vec![(0, true), (512, true), (1024, false), (1536, true)]
        );
        for (tile, pixels) in &tiles {
            if let Some(pixels) = pixels {
                assert_matches_page(pixels, *tile, &expected, bounds);
            }
        }
    }

    #[test]
    fn repaint_matches_a_full_raster_and_leaves_shared_pixels_intact() {
        let bounds = page(1100, 700);
        let display_list = |x| {
            let mut recorder = DisplayListRecorder::new(bounds);
            recorder.fill_rect(
                Rect::from_xywh(0.0, 0.0, 1100.0, 700.0),
                Color::new(30, 60, 90, 180),
                Transform::IDENTITY,
            );
            recorder.fill_rect(
                Rect::from_xywh(x, 500.5, 20.0, 20.0),
                Color::new(255, 0, 0, 120),
                Transform::IDENTITY,
            );
            recorder.into_list()
        };
        let before = display_list(500.5);
        let after = display_list(510.5);
        let mut sink = scratch();
        let tiles: Vec<_> = grid(1100, 700)
            .into_iter()
            .map(|tile| {
                (
                    tile,
                    rasterize(&before.primitives, tile, &mut sink).unwrap(),
                )
            })
            .collect();
        let shared: Vec<_> = tiles.iter().map(|(_, pixels)| Arc::clone(pixels)).collect();
        let originals: Vec<Vec<u8>> = tiles.iter().map(|(_, pixels)| pixels.to_vec()).collect();
        let damage = editor_renderer::diff::diff(&before, &after, bounds);
        let expected = full_raster(&after, bounds);
        for (tile, pixels) in tiles {
            if tile.x0 == 1024 {
                assert!(!is_damaged(tile, &damage));
                continue;
            }
            assert!(is_damaged(tile, &damage));
            let repainted =
                repaint(&after.primitives, tile, &damage, Some(pixels), &mut sink).unwrap();
            assert_matches_page(&repainted, tile, &expected, bounds);
        }
        for (tile, (pixels, original)) in grid(1100, 700)
            .into_iter()
            .zip(shared.iter().zip(&originals))
        {
            assert_same_pixels(
                Some(pixels),
                Some(original),
                format_args!("shared tile {tile:?}"),
            );
        }
    }

    #[test]
    fn repaint_clears_erased_tiles_and_fills_blank_ones() {
        let bounds = page(512, 512);
        let tile = grid(512, 512)[0];
        let mut recorder = DisplayListRecorder::new(bounds);
        recorder.fill_rect(
            Rect::from_xywh(10.0, 10.0, 30.0, 30.0),
            Color::new(0, 0, 0, 255),
            Transform::IDENTITY,
        );
        let painted = recorder.into_list();
        let erased = DisplayListRecorder::new(bounds).into_list();
        let mut sink = scratch();
        let pixels = rasterize(&painted.primitives, tile, &mut sink);
        assert!(pixels.is_some());
        assert!(repaint(&erased.primitives, tile, &[bounds], pixels, &mut sink).is_none());
        let filled = repaint(&painted.primitives, tile, &[bounds], None, &mut sink).unwrap();
        assert_matches_page(&filled, tile, &full_raster(&painted, bounds), bounds);
    }
}
