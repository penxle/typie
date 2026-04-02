use editor_renderer::{RenderBackend, RenderSink};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

use crate::backend::BackendMode;
use crate::error::FfiError;

pub type PlatformHandle = web_sys::HtmlCanvasElement;

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
            BackendMode::Gpu { device } => {
                let surface = device
                    .instance
                    .create_surface(wgpu::SurfaceTarget::Canvas(handle.clone()))
                    .map_err(|e| FfiError::Surface(e.to_string()))?;

                let mut backend = RenderBackend::new_gpu(Arc::clone(device), surface)
                    .map_err(|e| FfiError::Surface(e.to_string()))?;

                backend.resize(pw as u16, ph as u16);

                backend
            }
        };

        handle.set_width(pw);
        handle.set_height(ph);

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
                let mut buf = vec![0u8; (self.width * self.height * 4) as usize];
                sink.flush_to(&mut buf);

                let ctx = self
                    .handle
                    .get_context("2d")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<web_sys::CanvasRenderingContext2d>()
                    .unwrap();

                let clamped = wasm_bindgen::Clamped(&buf[..]);
                let image_data = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
                    clamped,
                    self.width,
                    self.height,
                )
                .unwrap();

                ctx.put_image_data(&image_data, 0.0, 0.0).unwrap();
            }
            RenderBackend::Gpu(sink) => {
                let _ = sink.present();
            }
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        let pw = (width as f64 * scale_factor).round() as u32;
        let ph = (height as f64 * scale_factor).round() as u32;

        self.width = pw;
        self.height = ph;
        self.scale_factor = scale_factor;

        self.handle.set_width(pw);
        self.handle.set_height(ph);

        self.backend.resize(pw as u16, ph as u16);
    }
}
