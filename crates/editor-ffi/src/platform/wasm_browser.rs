use editor_renderer::RenderBackend;
use editor_renderer::backend::cpu::{CpuSink, unpremultiply};
use editor_renderer::damage::IRect;
use wasm_bindgen::prelude::*;

use super::gl_surface;
use crate::error::FfiError;

pub type PlatformHandle = web_sys::HtmlCanvasElement;

pub struct CpuPageSurface {
    backend: Option<RenderBackend>,
    handle: PlatformHandle,
    width: u32,
    height: u32,
    scale_factor: f64,
    oversized: bool,
}

impl CpuPageSurface {
    pub fn new(
        handle: PlatformHandle,
        width: f64,
        height: f64,
        scale_factor: f64,
    ) -> Result<Self, FfiError> {
        let raw_w = (width * scale_factor).round() as u32;
        let raw_h = (height * scale_factor).round() as u32;
        let pw = gl_surface::clamp_dim_u16(raw_w);
        let ph = gl_surface::clamp_dim_u16(raw_h);
        let w = u32::from(pw);
        let h = u32::from(ph);
        if raw_w != w || raw_h != h {
            web_sys::console::warn_1(
                &format!("[cpu-surface] page {raw_w}x{raw_h} clamped to {w}x{h}").into(),
            );
        }

        handle.set_width(w);
        handle.set_height(h);

        let (backend, oversized) = Self::alloc_backend(pw, ph, w, h);

        Ok(Self {
            backend,
            handle,
            width: w,
            height: h,
            scale_factor,
            oversized,
        })
    }

    fn alloc_backend(pw: u16, ph: u16, w: u32, h: u32) -> (Option<RenderBackend>, bool) {
        if !Self::budget_gate("page", w, h) {
            return (None, true);
        }
        match RenderBackend::try_new_cpu(pw, ph) {
            Some(backend) => (Some(backend), false),
            None => {
                Self::warn_alloc_failed("page", w, h);
                (None, true)
            }
        }
    }

    // `new`/`resize` 공통: 예산 초과 시 콘솔 경고 후 false를 돌려준다(oversized로 강등).
    fn budget_gate(context: &str, w: u32, h: u32) -> bool {
        let within = gl_surface::cpu_surface_within_budget(w, h);
        if !within {
            web_sys::console::warn_1(
                &format!("[cpu-surface] {context} {w}x{h} exceeds byte budget; surface oversized")
                    .into(),
            );
        }
        within
    }

    // `new`/`resize` 공통: 할당 실패 시 콘솔 경고.
    fn warn_alloc_failed(context: &str, w: u32, h: u32) {
        web_sys::console::warn_1(
            &format!("[cpu-surface] {context} {w}x{h} allocation failed; surface oversized").into(),
        );
    }

    pub fn scale_factor(&self) -> f64 {
        self.scale_factor
    }

    pub fn is_oversized(&self) -> bool {
        self.oversized
    }

    pub fn cpu_sink(&mut self) -> Option<&mut CpuSink> {
        self.backend.as_mut().map(RenderBackend::cpu_sink)
    }

    pub fn apply_damage(
        &mut self,
        dl: &editor_renderer::display_list::DisplayList,
        damage: &[IRect],
    ) -> bool {
        if self.oversized {
            return false;
        }
        let bounds = IRect {
            x0: 0,
            y0: 0,
            x1: self.width as i32,
            y1: self.height as i32,
        };
        let clamped: Vec<IRect> = damage.iter().filter_map(|&r| r.intersect(bounds)).collect();

        let Some(sink) = self.cpu_sink() else {
            return false;
        };
        for &r in &clamped {
            sink.clear_rect(r);
            sink.set_clip(Some(r));
            editor_renderer::diff::replay(dl, r, sink);
        }
        sink.set_clip(None);
        self.present_damage(&clamped)
    }

    pub fn present_damage(&mut self, damage: &[IRect]) -> bool {
        if self.oversized {
            return false;
        }
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

            if !self.present_rect_via_put_image_data(&ctx, r) {
                return false;
            }
        }

        true
    }

    fn present_rect_via_put_image_data(
        &mut self,
        ctx: &web_sys::CanvasRenderingContext2d,
        r: IRect,
    ) -> bool {
        let w = r.width() as u32;
        if w == 0 {
            return true;
        }
        let max_rows =
            gl_surface::max_strip_rows(w, gl_surface::CPU_PRESENT_STRIP_BYTE_BUDGET).max(1) as i32;
        let mut y = r.y0;
        while y < r.y1 {
            let y1 = y.saturating_add(max_rows).min(r.y1);
            let strip = IRect {
                x0: r.x0,
                y0: y,
                x1: r.x1,
                y1,
            };
            if !self.put_image_strip(ctx, strip) {
                return false;
            }
            y = y1;
        }
        true
    }

    fn put_image_strip(&mut self, ctx: &web_sys::CanvasRenderingContext2d, r: IRect) -> bool {
        let w = r.width() as u32;
        let h = r.height() as u32;
        if w == 0 || h == 0 {
            return true;
        }

        let len = w as usize * h as usize * 4;
        let mut buf = Vec::new();
        if buf.try_reserve_exact(len).is_err() {
            return false;
        }
        buf.resize(len, 0u8);

        let Some(sink) = self.cpu_sink() else {
            return false;
        };
        sink.read_back_rect(&mut buf, (w * 4) as usize, r);
        unpremultiply(&mut buf);

        let clamped = wasm_bindgen::Clamped(&buf[..]);
        let Ok(image_data) = web_sys::ImageData::new_with_u8_clamped_array_and_sh(clamped, w, h)
        else {
            return false;
        };

        ctx.put_image_data(&image_data, r.x0 as f64, r.y0 as f64)
            .is_ok()
    }

    pub fn resize(&mut self, width: f64, height: f64, scale_factor: f64) -> bool {
        let raw_w = (width * scale_factor).round() as u32;
        let raw_h = (height * scale_factor).round() as u32;
        let pw = gl_surface::clamp_dim_u16(raw_w);
        let ph = gl_surface::clamp_dim_u16(raw_h);
        let w = u32::from(pw);
        let h = u32::from(ph);

        if self.width == w && self.height == h && self.scale_factor == scale_factor {
            return false;
        }
        if raw_w != w || raw_h != h {
            web_sys::console::warn_1(
                &format!("[cpu-surface] resize {raw_w}x{raw_h} clamped to {w}x{h}").into(),
            );
        }

        self.width = w;
        self.height = h;
        self.scale_factor = scale_factor;

        self.handle.set_width(w);
        self.handle.set_height(h);

        if !Self::budget_gate("resize", w, h) {
            self.backend = None;
            self.oversized = true;
            return true;
        }

        let resized = match self.backend.as_mut() {
            Some(backend) => backend.try_resize(pw, ph),
            None => match RenderBackend::try_new_cpu(pw, ph) {
                Some(backend) => {
                    self.backend = Some(backend);
                    true
                }
                None => false,
            },
        };
        if !resized {
            Self::warn_alloc_failed("resize", w, h);
            self.backend = None;
        }
        self.oversized = !resized;
        true
    }
}

// 스펙 §3.4 예산 장부: 브라우저 한도(≥8) − 프로브/GC 마진 → 8 (구 공유 presenter 제거 완료).
// TS 풀(GL_POOL_BUDGET)이 1차 결정권자, 이 상수는 Rust측 백스톱.
const MAX_GL_CONTEXTS: usize = 8;

thread_local! {
    static GL_CONTEXT_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub enum SurfaceHandle {
    Cpu(CpuPageSurface),
    Gl(gl_surface::GlPageSurface),
}

impl SurfaceHandle {
    pub fn new(
        handle: PlatformHandle,
        width: f64,
        height: f64,
        scale_factor: f64,
    ) -> Result<Self, FfiError> {
        Self::new_with_backend(handle, width, height, scale_factor, "cpu")
    }

    // Backend selection now lives in the TS pool (Task 2); this stays the single
    // chokepoint the FFI goes through, with `GL_CONTEXT_COUNT`/`MAX_GL_CONTEXTS` kept
    // as a defense-in-depth backstop until the pool owns the budget (Plan 4).
    pub fn new_with_backend(
        handle: PlatformHandle,
        width: f64,
        height: f64,
        scale_factor: f64,
        backend: &str,
    ) -> Result<Self, FfiError> {
        if backend == "gl"
            && GL_CONTEXT_COUNT.with(|c| c.get()) < MAX_GL_CONTEXTS
            && let Some(surface) =
                gl_surface::GlPageSurface::new(handle.clone(), width, height, scale_factor)
        {
            GL_CONTEXT_COUNT.with(|c| c.set(c.get() + 1));
            return Ok(Self::Gl(surface));
        }
        Ok(Self::Cpu(CpuPageSurface::new(
            handle,
            width,
            height,
            scale_factor,
        )?))
    }

    pub fn scale_factor(&self) -> f64 {
        match self {
            Self::Cpu(surface) => surface.scale_factor(),
            Self::Gl(surface) => surface.scale_factor(),
        }
    }

    // Full device-pixel bounds of the backing (both variants). The debug readback FFI
    // clamps probe requests to this before reading texture/present/CPU pixels.
    pub fn device_bounds(&self) -> IRect {
        match self {
            Self::Cpu(surface) => IRect {
                x0: 0,
                y0: 0,
                x1: surface.width as i32,
                y1: surface.height as i32,
            },
            Self::Gl(surface) => surface.device_bounds(),
        }
    }

    pub fn apply_damage(
        &mut self,
        dl: &editor_renderer::display_list::DisplayList,
        damage: &[IRect],
    ) -> bool {
        match self {
            Self::Cpu(surface) => surface.apply_damage(dl, damage),
            Self::Gl(surface) => surface.apply_damage(dl, damage),
        }
    }

    pub fn resize(&mut self, width: f64, height: f64, scale_factor: f64) -> bool {
        match self {
            Self::Cpu(surface) => surface.resize(width, height, scale_factor),
            Self::Gl(surface) => surface.resize(width, height, scale_factor),
        }
    }
}

impl Drop for SurfaceHandle {
    fn drop(&mut self) {
        if matches!(self, Self::Gl(_)) {
            GL_CONTEXT_COUNT.with(|c| c.set(c.get().saturating_sub(1)));
        }
    }
}
