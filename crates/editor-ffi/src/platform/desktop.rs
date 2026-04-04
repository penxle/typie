use editor_macros::ffi;
use editor_renderer::{RenderBackend, RenderSink};

use crate::backend::BackendMode;
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
        mode: &BackendMode,
        handle: PlatformHandle,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> Result<Self, FfiError> {
        let pw = (width as f64 * scale_factor).round() as u32;
        let ph = (height as f64 * scale_factor).round() as u32;

        let backend = match mode {
            BackendMode::Cpu => RenderBackend::new_cpu(pw as u16, ph as u16),
            BackendMode::Gpu { .. } => {
                return Err(FfiError::Surface(
                    "desktop GPU surface not yet supported".into(),
                ));
            }
        };

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
        let _ = self.handle;
        // todo -- requires raw_window_handle integration
        todo!("desktop present")
    }

    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        let pw = (width as f64 * scale_factor).round() as u32;
        let ph = (height as f64 * scale_factor).round() as u32;

        self.width = pw;
        self.height = ph;
        self.scale_factor = scale_factor;

        self.backend.resize(pw as u16, ph as u16);
    }
}
