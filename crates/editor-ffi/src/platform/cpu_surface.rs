#[cfg(not(test))]
use editor_macros::ffi;
use editor_renderer::{damage::IRect, display_list::DisplayList};

use super::{
    render_buffer::{RenderBuffer, RenderedFrame},
    tiled_surface::TiledSurface,
};
use crate::{editor::FrameKey, error::FfiError};

#[cfg_attr(not(test), ffi)]
pub type PlatformHandle = u64;

pub struct SurfaceHandle {
    handle: PlatformHandle,
    raster: TiledSurface,
}

impl SurfaceHandle {
    pub fn new(
        handle: PlatformHandle,
        width: f64,
        height: f64,
        scale_factor: f64,
    ) -> Result<Self, FfiError> {
        Ok(Self {
            handle,
            raster: TiledSurface::new(width, height, scale_factor)?,
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
        self.raster.resize(width, height, scale_factor)
    }

    pub fn apply_damage(
        &mut self,
        dl: &DisplayList,
        damage: &[IRect],
        editor_revision: u64,
        frame_key: FrameKey,
    ) -> bool {
        if !self.raster.apply_damage(dl, damage, frame_key.value) {
            return false;
        }
        self.publish_frame(editor_revision, frame_key)
    }

    pub fn publish_frame(&self, editor_revision: u64, frame_key: FrameKey) -> bool {
        unsafe {
            (&*(self.handle as *const RenderBuffer)).publish(RenderedFrame {
                width: self.raster.width,
                height: self.raster.height,
                editor_revision,
                frame_key: frame_key.value,
                tiles: self.raster.tiles.clone(),
            })
        }
    }
}
