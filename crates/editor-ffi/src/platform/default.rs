use editor_macros::ffi;
use editor_renderer::RenderSink;

use crate::error::FfiError;

#[ffi]
pub type PlatformHandle = u64;

pub struct SurfaceHandle;

impl SurfaceHandle {
    pub fn new(
        _handle: PlatformHandle,
        _width: f64,
        _height: f64,
        _scale_factor: f64,
    ) -> Result<Self, FfiError> {
        unreachable!();
    }

    pub fn scale_factor(&self) -> f64 {
        unreachable!();
    }

    pub fn sink(&mut self) -> &mut dyn RenderSink {
        unreachable!();
    }

    pub fn present(&mut self) {
        unreachable!();
    }

    pub fn resize(&mut self, _width: f64, _height: f64, _scale_factor: f64) {
        unreachable!();
    }
}
