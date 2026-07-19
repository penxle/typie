use editor_macros::ffi;
use editor_renderer::RenderBackend;
use editor_renderer::backend::cpu::CpuSink;
use editor_renderer::damage::IRect;

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
        width: f64,
        height: f64,
        scale_factor: f64,
    ) -> Result<Self, FfiError> {
        let pw = (width * scale_factor).round() as u32;
        let ph = (height * scale_factor).round() as u32;

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

    pub fn cpu_sink(&mut self) -> &mut CpuSink {
        self.backend.cpu_sink()
    }

    pub fn apply_damage(
        &mut self,
        dl: &editor_renderer::display_list::DisplayList,
        damage: &[IRect],
    ) -> bool {
        let sink = self.cpu_sink();
        for &r in damage {
            sink.clear_rect(r);
            sink.set_clip(Some(r));
            editor_renderer::diff::replay(dl, r, sink);
        }
        sink.set_clip(None);
        self.present_damage(damage)
    }

    pub fn present_damage(&mut self, damage: &[IRect]) -> bool {
        if self.handle == 0 {
            return true;
        }

        let (w, handle) = (self.width, self.handle);
        match &mut self.backend {
            RenderBackend::Cpu(sink) => unsafe {
                (*(handle as *const RenderBuffer)).commit_damage(damage, |data, r| {
                    sink.read_back_rect_absolute(data, w as usize * 4, r);
                })
            },
        }
    }

    pub fn resize(&mut self, width: f64, height: f64, scale_factor: f64) -> bool {
        let pw = (width * scale_factor).round() as u32;
        let ph = (height * scale_factor).round() as u32;

        if self.width == pw && self.height == ph && self.scale_factor == scale_factor {
            return false;
        }

        self.width = pw;
        self.height = ph;
        self.scale_factor = scale_factor;

        if self.handle != 0 {
            unsafe {
                (*(self.handle as *const RenderBuffer)).resize(pw, ph);
            }
        }

        self.backend.resize(pw as u16, ph as u16);
        true
    }
}
