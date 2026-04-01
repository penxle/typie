use editor_renderer::{RenderBackend, RenderSink};

use crate::backend::BackendMode;
use crate::error::FfiError;

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
        let backend = match mode {
            BackendMode::Cpu => RenderBackend::new_cpu(width as u16, height as u16),
            BackendMode::Gpu { .. } => {
                return Err(FfiError::Surface(
                    "Android GPU surface not yet supported".into(),
                ));
            }
        };

        Ok(Self {
            backend,
            handle,
            width,
            height,
            scale_factor,
        })
    }

    pub fn sink(&mut self) -> &mut dyn RenderSink {
        self.backend.sink()
    }

    pub fn present(&mut self) {
        // Android present: todo -- requires ANativeWindow integration
        todo!("android present")
    }

    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        self.width = width;
        self.height = height;
        self.scale_factor = scale_factor;

        self.backend.resize(width as u16, height as u16);
    }
}
