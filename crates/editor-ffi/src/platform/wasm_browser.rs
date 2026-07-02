use editor_renderer::RenderBackend;
use editor_renderer::backend::cpu::CpuSink;
use editor_renderer::damage::IRect;
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

    pub fn cpu_sink(&mut self) -> &mut CpuSink {
        self.backend.cpu_sink()
    }

    pub fn present_damage(&mut self, damage: &[IRect]) -> bool {
        if damage.is_empty() {
            return true;
        }

        let ctx = self
            .handle
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        for &r in damage {
            let w = r.width() as u32;
            let h = r.height() as u32;
            if w == 0 || h == 0 {
                continue;
            }

            let mut buf = vec![0u8; (w * h * 4) as usize];
            self.backend
                .cpu_sink()
                .read_back_rect(&mut buf, (w * 4) as usize, r);

            let clamped = wasm_bindgen::Clamped(&buf[..]);
            let image_data =
                web_sys::ImageData::new_with_u8_clamped_array_and_sh(clamped, w, h).unwrap();

            ctx.put_image_data(&image_data, r.x0 as f64, r.y0 as f64)
                .unwrap();
        }

        true
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
