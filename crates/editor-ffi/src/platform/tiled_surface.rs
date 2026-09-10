use editor_renderer::{backend::cpu::CpuSink, damage::IRect, display_list::DisplayList};
use std::sync::Arc;

use crate::error::FfiError;

/// Pixels never change after publication. Unchanged tiles share their allocation across frames.
#[derive(Clone)]
pub struct RenderedTile {
    pub bounds: [i32; 4],
    pub pixels: Arc<[u8]>,
    pub version: u64,
}

pub const TILE_SIZE: i32 = 512;
// Bound one page's requested tile set before allocating any pixel storage.
const MAX_TILES: usize = 128;

pub struct TiledSurface {
    pub width: u32,
    pub height: u32,
    scale_factor: f64,
    requested: Option<Vec<IRect>>,
    pub tiles: Vec<RenderedTile>,
    scratch: CpuSink,
    needs_render: bool,
}

fn pixel_size(width: f64, height: f64, scale: f64) -> Result<(u32, u32), FfiError> {
    let w = (width * scale).round();
    let h = (height * scale).round();
    if !scale.is_finite()
        || scale <= 0.0
        || !w.is_finite()
        || !h.is_finite()
        || w < 1.0
        || h < 1.0
        || w > (i32::MAX - 1) as f64
        || h > (i32::MAX - 1) as f64
    {
        return Err(FfiError::Surface("invalid surface dimensions".into()));
    }
    Ok((w as u32, h as u32))
}

impl TiledSurface {
    pub fn new(width: f64, height: f64, scale_factor: f64) -> Result<Self, FfiError> {
        let (width, height) = pixel_size(width, height, scale_factor)?;
        Ok(Self {
            width,
            height,
            scale_factor,
            requested: None,
            tiles: Vec::new(),
            scratch: CpuSink::new((TILE_SIZE + 2) as u16, (TILE_SIZE + 2) as u16),
            needs_render: true,
        })
    }

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }
    pub fn needs_render(&self) -> bool {
        self.needs_render
    }

    pub fn configure_tiles(&mut self, bounds: &[i32]) -> Result<(), FfiError> {
        if !bounds.len().is_multiple_of(4) || bounds.len() / 4 > MAX_TILES {
            return Err(FfiError::Surface(
                "tile count exceeds surface budget".into(),
            ));
        }
        let mut requested = Vec::with_capacity(bounds.len() / 4);
        for r in bounds.chunks_exact(4) {
            if r[0] < 0
                || r[1] < 0
                || r[2] <= r[0]
                || r[3] <= r[1]
                || i64::from(r[2]) - i64::from(r[0]) > i64::from(TILE_SIZE)
                || i64::from(r[3]) - i64::from(r[1]) > i64::from(TILE_SIZE)
            {
                return Err(FfiError::Surface("invalid tile bounds".into()));
            }
            let rect = IRect {
                x0: r[0],
                y0: r[1],
                x1: r[2],
                y1: r[3],
            };
            if requested.contains(&rect) {
                return Err(FfiError::Surface("duplicate tile".into()));
            }
            requested.push(rect);
        }
        if self.requested.as_ref() != Some(&requested) {
            self.requested = Some(requested);
            self.needs_render = true;
        }
        Ok(())
    }

    fn requested_tiles(&self) -> Option<Vec<IRect>> {
        let full = IRect {
            x0: 0,
            y0: 0,
            x1: self.width as i32,
            y1: self.height as i32,
        };
        if let Some(requested) = &self.requested {
            return Some(requested.iter().filter_map(|r| r.intersect(full)).collect());
        }
        let columns = self.width.div_ceil(TILE_SIZE as u32);
        let rows = self.height.div_ceil(TILE_SIZE as u32);
        if u64::from(columns) * u64::from(rows) > MAX_TILES as u64 {
            return None;
        }
        Some(
            (0..rows)
                .flat_map(|row| {
                    (0..columns).map(move |col| IRect {
                        x0: col as i32 * TILE_SIZE,
                        y0: row as i32 * TILE_SIZE,
                        x1: ((col + 1) as i64 * i64::from(TILE_SIZE)).min(i64::from(self.width))
                            as i32,
                        y1: ((row + 1) as i64 * i64::from(TILE_SIZE)).min(i64::from(self.height))
                            as i32,
                    })
                })
                .collect(),
        )
    }

    pub fn apply_damage(&mut self, dl: &DisplayList, damage: &[IRect], version: u64) -> bool {
        let Some(requested) = self.requested_tiles() else {
            return false;
        };
        let mut previous = std::mem::take(&mut self.tiles);
        let mut tiles = Vec::with_capacity(requested.len());
        for rect in requested {
            let bounds = [rect.x0, rect.y0, rect.x1, rect.y1];
            let raster = IRect {
                x0: rect.x0 - 1,
                y0: rect.y0 - 1,
                x1: rect.x1 + 1,
                y1: rect.y1 + 1,
            };
            if let Some(index) = previous.iter().position(|tile| tile.bounds == bounds) {
                let mut tile = previous.swap_remove(index);
                if let Some(dirty) = damage
                    .iter()
                    .filter_map(|r| r.intersect(raster))
                    .reduce(IRect::union)
                {
                    editor_renderer::diff::raster_rect(dl, dirty, &mut self.scratch);
                    // Native readers may still hold the previous frame. Copy only
                    // when shared; on Web the tile allocation is usually unique.
                    let pixels = Arc::make_mut(&mut tile.pixels);
                    let stride = raster.width() as usize * 4;
                    let row_bytes = dirty.width() as usize * 4;
                    for y in 0..dirty.height() {
                        let offset = (dirty.y0 - raster.y0 + y) as usize * stride
                            + (dirty.x0 - raster.x0) as usize * 4;
                        self.scratch.read_back_rect(
                            &mut pixels[offset..offset + row_bytes],
                            row_bytes,
                            IRect {
                                x0: 0,
                                y0: y,
                                x1: dirty.width(),
                                y1: y + 1,
                            },
                        );
                    }
                    tile.version = version;
                }
                tiles.push(tile);
                continue;
            }
            editor_renderer::diff::raster_rect(dl, raster, &mut self.scratch);
            let mut pixels = vec![0; raster.width() as usize * raster.height() as usize * 4];
            self.scratch.read_back_rect(
                &mut pixels,
                raster.width() as usize * 4,
                IRect {
                    x0: 0,
                    y0: 0,
                    x1: raster.width(),
                    y1: raster.height(),
                },
            );
            tiles.push(RenderedTile {
                bounds,
                pixels: Arc::from(pixels),
                version,
            });
        }
        self.tiles = tiles;
        self.needs_render = false;
        true
    }

    pub fn resize(&mut self, width: f64, height: f64, scale_factor: f64) -> bool {
        let Ok((width, height)) = pixel_size(width, height, scale_factor) else {
            return false;
        };
        if self.width == width && self.height == height && self.scale_factor == scale_factor {
            return false;
        }
        self.width = width;
        self.height = height;
        self.scale_factor = scale_factor;
        self.tiles.clear();
        self.needs_render = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_common::{Color, Rect};
    use editor_renderer::{display_list::DisplayListRecorder, sink::RenderSink, types::Transform};

    #[test]
    fn huge_page_only_rasters_requested_tiles_and_reuses_unchanged_pixels() {
        let mut surface = TiledSurface::new(100_000.0, 200_000.0, 2.0).unwrap();
        surface.configure_tiles(&[512, 1024, 1024, 1536]).unwrap();
        let full = IRect {
            x0: 0,
            y0: 0,
            x1: 200_000,
            y1: 400_000,
        };
        let mut recorder = DisplayListRecorder::new(full);
        recorder.fill_rect(
            Rect::from_xywh(520.0, 1030.0, 10.0, 10.0),
            Color::new(255, 0, 0, 255),
            Transform::IDENTITY,
        );
        let dl = recorder.into_list();
        assert!(surface.apply_damage(&dl, &[full], 1));
        assert_eq!(surface.tiles.len(), 1);
        assert_eq!(surface.tiles[0].pixels.len(), 514 * 514 * 4);
        let original = surface.tiles[0].pixels.clone();
        assert!(original[(7 * 514 + 9) * 4] >= 250);
        surface
            .configure_tiles(&[512, 1024, 1024, 1536, 1024, 1024, 1536, 1536])
            .unwrap();
        assert!(surface.needs_render());
        assert!(surface.apply_damage(&dl, &[], 2));
        assert!(Arc::ptr_eq(&original, &surface.tiles[0].pixels));
        assert_eq!(surface.tiles[0].version, 1);
        assert_eq!(surface.tiles[1].version, 2);
        assert!(surface.tiles[1].pixels.iter().all(|p| *p == 0));
        assert!(surface.apply_damage(&DisplayList::default(), &[full], 3));
        assert!(!Arc::ptr_eq(&original, &surface.tiles[0].pixels));
        assert!(surface.tiles[0].pixels.iter().all(|p| *p == 0));
        assert!(original[(7 * 514 + 9) * 4] >= 250);
    }

    #[test]
    fn gutters_match_full_raster_and_resize_discards_old_pixels() {
        let bounds = IRect {
            x0: 0,
            y0: 0,
            x1: 1100,
            y1: 700,
        };
        let mut recorder = DisplayListRecorder::new(bounds);
        recorder.fill_rect(
            Rect::from_xywh(0.0, 0.0, 1100.0, 700.0),
            Color::new(255, 255, 255, 255),
            Transform::IDENTITY,
        );
        recorder.fill_rect(
            Rect::from_xywh(500.5, 490.25, 50.0, 75.5),
            Color::new(25, 110, 240, 180),
            Transform::IDENTITY,
        );
        let dl = recorder.into_list();
        let mut full = CpuSink::new(1100, 700);
        editor_renderer::diff::raster_rect(&dl, bounds, &mut full);
        let mut expected = vec![0; 1100 * 700 * 4];
        full.read_back_rect(&mut expected, 1100 * 4, bounds);
        let mut surface = TiledSurface::new(1100.0, 700.0, 1.0).unwrap();
        assert!(surface.apply_damage(&dl, &[bounds], 1));
        for tile in &surface.tiles {
            let [left, top, right, bottom] = tile.bounds;
            let stride = (right - left + 2) as usize * 4;
            // Include the gutter where it overlaps the page: filtering samples it.
            for y in (top - 1).max(0)..(bottom + 1).min(700) {
                for x in (left - 1).max(0)..(right + 1).min(1100) {
                    let actual = (y - top + 1) as usize * stride + (x - left + 1) as usize * 4;
                    let want = (y as usize * 1100 + x as usize) * 4;
                    assert_eq!(
                        &tile.pixels[actual..actual + 4],
                        &expected[want..want + 4],
                        "pixel ({x}, {y})"
                    );
                }
            }
        }
        assert!(!surface.resize(1100.0, 700.0, 1.0));
        assert!(surface.resize(400.0, 300.0, 2.0));
        assert_eq!(surface.scale_factor(), 2.0);
        assert!(surface.tiles.is_empty());
        assert!(surface.apply_damage(&DisplayList::default(), &[], 2));
        assert_eq!(surface.tiles.len(), 4);
        assert!(
            surface
                .tiles
                .iter()
                .all(|tile| tile.pixels.iter().all(|pixel| *pixel == 0))
        );
    }

    #[test]
    fn partial_damage_matches_full_raster_without_changing_pinned_pixels() {
        let full = IRect {
            x0: 0,
            y0: 0,
            x1: 1100,
            y1: 700,
        };
        let display_list = |x| {
            let mut recorder = DisplayListRecorder::new(full);
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
        let mut surface = TiledSurface::new(1100.0, 700.0, 1.0).unwrap();
        assert!(surface.apply_damage(&before, &[full], 1));
        let pinned = surface.tiles.clone();
        let original_pixels = pinned
            .iter()
            .map(|tile| tile.pixels.to_vec())
            .collect::<Vec<_>>();
        let damage = editor_renderer::diff::diff(&before, &after, full);
        assert!(surface.apply_damage(&after, &damage, 2));
        let mut expected = TiledSurface::new(1100.0, 700.0, 1.0).unwrap();
        assert!(expected.apply_damage(&after, &[full], 2));
        for (actual, expected) in surface.tiles.iter().zip(&expected.tiles) {
            assert_eq!(actual.pixels, expected.pixels);
        }
        for (pinned, original) in pinned.iter().zip(&original_pixels) {
            assert_eq!(pinned.pixels.as_ref(), original.as_slice());
        }
        // The edit crosses both tile boundaries, but leaves the last column alone.
        assert!(
            surface
                .tiles
                .iter()
                .filter(|tile| tile.bounds[0] == 1024)
                .all(|tile| tile.version == 1)
        );
    }

    #[test]
    fn tile_bounds_and_budget_are_checked_before_allocating() {
        let mut surface = TiledSurface::new(100_000.0, 200_000.0, 2.0).unwrap();
        assert!(surface.configure_tiles(&[0, 0, 513, 512]).is_err());
        assert!(
            surface
                .configure_tiles(&[0, 0, i32::MAX, i32::MAX])
                .is_err()
        );
        assert!(
            surface
                .configure_tiles(&[0, 0, 512, 512, 0, 0, 512, 512])
                .is_err()
        );
        assert!(
            surface
                .configure_tiles(&vec![0; (MAX_TILES + 1) * 4])
                .is_err()
        );
        assert!(!surface.apply_damage(&DisplayList::default(), &[], 1));
    }
}
