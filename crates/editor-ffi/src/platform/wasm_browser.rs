use editor_renderer::{RenderBackend, RenderSink};
use wasm_bindgen::prelude::*;

use crate::error::FfiError;

pub type PlatformHandle = Vec<web_sys::HtmlCanvasElement>;

pub struct SurfaceHandle {
    scratch: RenderBackend,
    canvases: [web_sys::HtmlCanvasElement; 4],
    allocated: [bool; 4],
    width: u32,
    height: u32,
    scale_factor: f64,
}

impl SurfaceHandle {
    pub fn new(
        canvases: PlatformHandle,
        width: f64,
        height: f64,
        scale_factor: f64,
    ) -> Result<Self, FfiError> {
        let pw = (width * scale_factor).round() as u32;
        let ph = (height * scale_factor).round() as u32;

        let scratch = RenderBackend::new_cpu(pw as u16, ph as u16);

        let canvases: [web_sys::HtmlCanvasElement; 4] = canvases
            .try_into()
            .map_err(|_| FfiError::Surface("expected 4 canvases".into()))?;

        for canvas in &canvases {
            canvas.set_width(0);
            canvas.set_height(0);
        }

        Ok(Self {
            scratch,
            canvases,
            allocated: [false; 4],
            width: pw,
            height: ph,
            scale_factor,
        })
    }

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    pub fn scratch_sink(&mut self) -> &mut dyn RenderSink {
        self.scratch.sink()
    }

    pub fn take_touched(&mut self) -> bool {
        self.scratch.take_touched()
    }

    pub fn ensure_canvas(&mut self, i: usize) {
        if !self.allocated[i] {
            self.canvases[i].set_width(self.width);
            self.canvases[i].set_height(self.height);
            self.allocated[i] = true;
        }
    }

    pub fn release_canvas(&mut self, i: usize) {
        if self.allocated[i] {
            self.canvases[i].set_width(0);
            self.canvases[i].set_height(0);
            self.allocated[i] = false;
        }
    }

    pub fn present_layer(&mut self, i: usize) {
        match &mut self.scratch {
            RenderBackend::Cpu(sink) => {
                let buf_size = (self.width * self.height * 4) as usize;
                let mut buf = vec![0u8; buf_size];
                sink.flush_to(&mut buf);

                let ctx = self.canvases[i]
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

        self.scratch.resize(pw as u16, ph as u16);

        self.allocated = [false; 4];
        for canvas in &self.canvases {
            canvas.set_width(0);
            canvas.set_height(0);
        }
    }
}
