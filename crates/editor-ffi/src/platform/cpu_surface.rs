use editor_macros::ffi;
use editor_renderer::{RenderBackend, RenderSink};

use super::render_buffer::RenderBuffer;
use crate::error::FfiError;

#[ffi]
pub type PlatformHandle = u64;

pub struct SurfaceHandle {
    backend: RenderBackend,
    handle: PlatformHandle,
    width: u32,
    height: u32,
    scale_factor: f64,
}

impl SurfaceHandle {
    pub fn new(
        handle: PlatformHandle,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> Result<Self, FfiError> {
        let pw = (width as f64 * scale_factor).round() as u32;
        let ph = (height as f64 * scale_factor).round() as u32;

        if handle != 0 {
            unsafe {
                (*(handle as *const RenderBuffer)).resize(pw, ph);
            }
        }

        let backend = RenderBackend::new_cpu(pw as u16, ph as u16);

        Ok(Self {
            backend,
            handle,
            width: pw,
            height: ph,
            scale_factor,
        })
    }

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    pub fn sink(&mut self) -> &mut dyn RenderSink {
        self.backend.sink()
    }

    pub fn present(&mut self) {
        match &mut self.backend {
            RenderBackend::Cpu(sink) => {
                if self.handle == 0 {
                    return;
                }
                unsafe {
                    (*(self.handle as *const RenderBuffer)).commit(|data| sink.flush_to(data));
                }
            }
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        let pw = (width as f64 * scale_factor).round() as u32;
        let ph = (height as f64 * scale_factor).round() as u32;

        self.width = pw;
        self.height = ph;
        self.scale_factor = scale_factor;

        if self.handle != 0 {
            unsafe {
                (*(self.handle as *const RenderBuffer)).resize(pw, ph);
            }
        }

        self.backend.resize(pw as u16, ph as u16);
    }
}
