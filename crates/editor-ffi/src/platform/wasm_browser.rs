use editor_renderer::{
    backend::cpu::unpremultiply,
    damage::IRect,
    display_list::{DisplayList, Primitive},
};
use wasm_bindgen::prelude::*;

use super::tiled_surface::{RenderedTile, TiledSurface};
use crate::{editor::FrameKey, error::FfiError};

pub type PlatformHandle = web_sys::HtmlElement;

struct CanvasTile {
    bounds: [i32; 4],
    version: u64,
    element: web_sys::HtmlCanvasElement,
    context: web_sys::CanvasRenderingContext2d,
    pending: Option<(IRect, Vec<u8>)>,
}

struct SurfaceLayer {
    handle: PlatformHandle,
    container: web_sys::HtmlElement,
    raster: TiledSurface,
    tiles: Vec<CanvasTile>,
    prepared_frame: Option<u64>,
    tiles_changed: bool,
    mounted: bool,
}

impl SurfaceLayer {
    pub fn new(
        handle: PlatformHandle,
        width: f64,
        height: f64,
        scale_factor: f64,
        layer: Option<&str>,
    ) -> Result<Self, FfiError> {
        let container = handle
            .owner_document()
            .ok_or_else(|| FfiError::Surface("surface has no document".into()))?
            .create_element("div")
            .and_then(|element| {
                element
                    .dyn_into::<web_sys::HtmlElement>()
                    .map_err(Into::into)
            })
            .map_err(|_| FfiError::Surface("could not create tile container".into()))?;
        container
            .set_attribute("style", "position:absolute;left:0;top:0;")
            .map_err(|_| FfiError::Surface("could not style tile container".into()))?;
        if let Some(layer) = layer {
            container
                .set_attribute("data-surface-layer", layer)
                .map_err(|_| FfiError::Surface("could not name surface layer".into()))?;
        }
        Ok(Self {
            handle,
            container,
            raster: TiledSurface::new(width, height, scale_factor)?,
            tiles: Vec::new(),
            prepared_frame: None,
            tiles_changed: true,
            mounted: false,
        })
    }

    pub fn scale_factor(&self) -> f64 {
        self.raster.scale_factor()
    }
    pub fn needs_render(&self) -> bool {
        self.raster.needs_render()
    }
    pub fn configure_tiles(&mut self, bounds: &[i32]) -> Result<(), FfiError> {
        self.raster.configure_tiles(bounds)
    }
    pub fn resize(&mut self, width: f64, height: f64, scale_factor: f64) -> bool {
        if !self.raster.resize(width, height, scale_factor) {
            return false;
        }
        self.prepared_frame = None;
        self.tiles.clear();
        self.tiles_changed = true;
        true
    }

    pub fn apply_damage(
        &mut self,
        primitives: &[Primitive],
        damage: &[IRect],
        _revision: u64,
        frame_key: FrameKey,
    ) -> bool {
        if !self
            .raster
            .apply_damage(primitives, damage, frame_key.value)
        {
            return false;
        }
        self.tiles_changed |= self.tiles.iter().map(|tile| tile.bounds).ne(self
            .raster
            .tiles
            .iter()
            .map(|tile| tile.bounds));
        let mut previous = std::mem::take(&mut self.tiles);
        let mut tiles = Vec::with_capacity(self.raster.tiles.len());
        for tile in &self.raster.tiles {
            let [left, top, right, bottom] = tile.bounds;
            let raster = IRect {
                x0: left - 1,
                y0: top - 1,
                x1: right + 1,
                y1: bottom + 1,
            };
            let (mut canvas, dirty) =
                if let Some(index) = previous.iter().position(|t| t.bounds == tile.bounds) {
                    let existing = previous.swap_remove(index);
                    if existing.version == tile.version {
                        tiles.push(existing);
                        continue;
                    }
                    let dirty = damage
                        .iter()
                        .filter_map(|r| r.intersect(raster))
                        .reduce(IRect::union)
                        .unwrap_or(raster);
                    // Accumulate every change since the canvas was last presented,
                    // including revisions prepared while another page was pending.
                    let dirty = existing
                        .pending
                        .as_ref()
                        .map_or(dirty, |(pending, _)| dirty.union(*pending));
                    (existing, dirty)
                } else {
                    let Ok(canvas) = self.create_tile(tile) else {
                        return false;
                    };
                    (canvas, raster)
                };
            let stride = raster.width() as usize * 4;
            let row_bytes = dirty.width() as usize * 4;
            let mut pixels = Vec::with_capacity(row_bytes * dirty.height() as usize);
            for y in dirty.y0..dirty.y1 {
                let offset =
                    (y - raster.y0) as usize * stride + (dirty.x0 - raster.x0) as usize * 4;
                pixels.extend_from_slice(&tile.pixels[offset..offset + row_bytes]);
            }
            unpremultiply(&mut pixels);
            canvas.version = tile.version;
            canvas.pending = Some((dirty, pixels));
            tiles.push(canvas);
        }
        // Keep visible pixels intact until the matching snapshot is presented.
        self.tiles = tiles;
        self.prepared_frame = Some(frame_key.value);
        true
    }

    fn create_tile(&self, tile: &RenderedTile) -> Result<CanvasTile, JsValue> {
        let document = self
            .handle
            .owner_document()
            .ok_or_else(|| JsValue::from_str("surface has no document"))?;
        let canvas = document
            .create_element("canvas")?
            .dyn_into::<web_sys::HtmlCanvasElement>()?;
        let [left, top, right, bottom] = tile.bounds;
        let width = (right - left) as u32;
        let height = (bottom - top) as u32;
        canvas.set_width(width);
        canvas.set_height(height);
        canvas.set_attribute("data-tile-x", &left.to_string())?;
        canvas.set_attribute("data-tile-y", &top.to_string())?;
        let scale = 1.0 / self.raster.scale_factor();
        // Map the intrinsic canvas pixels directly to page coordinates. Scaling
        // a clipped wrapper can resample glyphs, while fractional CSS bounds are
        // rounded independently and can misalign adjacent tiles.
        canvas.set_attribute(
            "style",
            &format!(
                "position:absolute;left:0;top:0;width:{width}px;height:{height}px;image-rendering:pixelated;transform-origin:0 0;transform:matrix({scale},0,0,{scale},{},{});",
                f64::from(left) * scale,
                f64::from(top) * scale,
            ),
        )?;
        let ctx = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("2d context unavailable"))?
            .dyn_into::<web_sys::CanvasRenderingContext2d>()?;
        Ok(CanvasTile {
            bounds: tile.bounds,
            version: tile.version,
            element: canvas,
            context: ctx,
            pending: None,
        })
    }

    pub fn present(&mut self, frame_key: u64) -> bool {
        if self.prepared_frame != Some(frame_key) {
            return false;
        }
        for tile in &mut self.tiles {
            if let Some((dirty, pixels)) = tile.pending.take() {
                // ImageData borrows WASM memory, so create and upload it while
                // its owned pixel buffer is alive, without retaining the JS view.
                let Ok(data) = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
                    wasm_bindgen::Clamped(&pixels),
                    dirty.width() as u32,
                    dirty.height() as u32,
                ) else {
                    return false;
                };
                // The raster gutter is outside this canvas and is clipped by
                // putImageData, without a CSS clip boundary.
                if tile
                    .context
                    .put_image_data(
                        &data,
                        f64::from(dirty.x0 - tile.bounds[0]),
                        f64::from(dirty.y0 - tile.bounds[1]),
                    )
                    .is_err()
                {
                    return false;
                }
            }
        }
        if self.tiles_changed {
            let nodes = js_sys::Array::new();
            for tile in &self.tiles {
                nodes.push(&tile.element);
            }
            self.container.replace_children_with_node(&nodes);
            self.tiles_changed = false;
        }
        if !self.mounted {
            let _ = self.handle.append_child(&self.container);
            self.mounted = true;
        }
        true
    }
}

/// Viewer backgrounds and foregrounds share one frame proof and tile request.
/// The DOM selection paints between them; editable surfaces keep one layer.
pub struct SurfaceHandle {
    foreground: SurfaceLayer,
    background: Option<SurfaceLayer>,
}

impl SurfaceHandle {
    pub fn new(
        handle: PlatformHandle,
        width: f64,
        height: f64,
        scale_factor: f64,
    ) -> Result<Self, FfiError> {
        let split = handle.has_attribute("data-separate-background");
        let background = if split {
            Some(SurfaceLayer::new(
                handle.clone(),
                width,
                height,
                scale_factor,
                Some("background"),
            )?)
        } else {
            None
        };
        let foreground = SurfaceLayer::new(
            handle,
            width,
            height,
            scale_factor,
            split.then_some("foreground"),
        )?;
        Ok(Self {
            foreground,
            background,
        })
    }
    pub fn scale_factor(&self) -> f64 {
        self.foreground.scale_factor()
    }
    pub fn needs_render(&self) -> bool {
        self.foreground.needs_render()
            || self
                .background
                .as_ref()
                .is_some_and(SurfaceLayer::needs_render)
    }
    pub fn configure_tiles(&mut self, bounds: &[i32]) -> Result<(), FfiError> {
        self.foreground.configure_tiles(bounds)?;
        if let Some(background) = &mut self.background {
            background.configure_tiles(bounds)?;
        }
        Ok(())
    }
    pub fn resize(&mut self, width: f64, height: f64, scale_factor: f64) -> bool {
        let changed = self.foreground.resize(width, height, scale_factor);
        if let Some(background) = &mut self.background {
            background.resize(width, height, scale_factor);
        }
        changed
    }
    pub fn apply_damage(
        &mut self,
        dl: &DisplayList,
        damage: &[IRect],
        revision: u64,
        frame_key: FrameKey,
    ) -> bool {
        let primitives = if let Some(background) = &mut self.background {
            if !background.apply_damage(
                &dl.primitives[..dl.foreground_start],
                damage,
                revision,
                frame_key,
            ) {
                return false;
            }
            &dl.primitives[dl.foreground_start..]
        } else {
            &dl.primitives
        };
        self.foreground
            .apply_damage(primitives, damage, revision, frame_key)
    }
    pub fn present(&mut self, frame_key: u64) -> bool {
        if self.foreground.prepared_frame != Some(frame_key)
            || self
                .background
                .as_ref()
                .is_some_and(|b| b.prepared_frame != Some(frame_key))
        {
            return false;
        }
        if let Some(background) = &mut self.background
            && !background.present(frame_key)
        {
            return false;
        }
        self.foreground.present(frame_key)
    }
}
