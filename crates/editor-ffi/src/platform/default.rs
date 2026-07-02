#![allow(dead_code)]

use editor_macros::ffi;
use editor_renderer::backend::cpu::CpuSink;
use editor_renderer::damage::IRect;

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

    pub fn cpu_sink(&mut self) -> &mut CpuSink {
        unreachable!();
    }

    pub fn present_damage(&mut self, _damage: &[IRect]) -> bool {
        unreachable!();
    }

    pub fn resize(&mut self, _width: f64, _height: f64, _scale_factor: f64) -> bool {
        unreachable!()
    }
}
