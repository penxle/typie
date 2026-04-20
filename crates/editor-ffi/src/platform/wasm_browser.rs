use editor_renderer::{RenderBackend, RenderSink};
use wasm_bindgen::prelude::*;

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
        handle: PlatformHandle,
        width: f64,
        height: f64,
        scale_factor: f64,
    ) -> Result<Self, FfiError> {
        let pw = (width * scale_factor).round() as u32;
        let ph = (height * scale_factor).round() as u32;

        let backend = RenderBackend::new_cpu(pw as u16, ph as u16);

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
                let buf_size = (self.width * self.height * 4) as usize;
                let mut buf = vec![0u8; buf_size];
                sink.flush_to(&mut buf);

                // putImageData expects straight alpha
                for chunk in buf.chunks_exact_mut(4) {
                    let a = chunk[3] as u32;
                    if a > 0 && a < 255 {
                        chunk[0] = ((chunk[0] as u32 * 255 + a / 2) / a).min(255) as u8;
                        chunk[1] = ((chunk[1] as u32 * 255 + a / 2) / a).min(255) as u8;
                        chunk[2] = ((chunk[2] as u32 * 255 + a / 2) / a).min(255) as u8;
                    }
                }

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
        }
    }

    pub fn resize(&mut self, width: f64, height: f64, scale_factor: f64) {
        let pw = (width * scale_factor).round() as u32;
        let ph = (height * scale_factor).round() as u32;

        self.width = pw;
        self.height = ph;
        self.scale_factor = scale_factor;

        self.handle.set_width(pw);
        self.handle.set_height(ph);

        self.backend.resize(pw as u16, ph as u16);
    }
}
