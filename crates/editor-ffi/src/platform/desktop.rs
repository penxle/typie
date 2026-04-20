use editor_macros::ffi;
use editor_renderer::{RenderBackend, RenderSink};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::FfiError;

struct PixelBuffer {
    data: Vec<u8>,
    width: u32,
    height: u32,
    dirty: AtomicBool,
}

#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_compose_DesktopSurfaceBridge_allocatePixelBuffer(
    _env: *mut c_void,
    _class: *mut c_void,
    width: i32,
    height: i32,
) -> i64 {
    let size = (width as usize) * (height as usize) * 4;
    let buf = Box::new(PixelBuffer {
        data: vec![0u8; size],
        width: width as u32,
        height: height as u32,
        dirty: AtomicBool::new(false),
    });
    Box::into_raw(buf) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_compose_DesktopSurfaceBridge_freePixelBuffer(
    _env: *mut c_void,
    _class: *mut c_void,
    ptr: i64,
) {
    if ptr != 0 {
        unsafe {
            let _ = Box::from_raw(ptr as *mut PixelBuffer);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_compose_DesktopSurfaceBridge_getDataPointer(
    _env: *mut c_void,
    _class: *mut c_void,
    ptr: i64,
) -> i64 {
    if ptr == 0 {
        return 0;
    }
    unsafe {
        let buf = &*(ptr as *const PixelBuffer);
        buf.data.as_ptr() as i64
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_compose_DesktopSurfaceBridge_getPixelWidth(
    _env: *mut c_void,
    _class: *mut c_void,
    ptr: i64,
) -> i32 {
    if ptr == 0 {
        return 0;
    }
    unsafe { (*(ptr as *const PixelBuffer)).width as i32 }
}

#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_compose_DesktopSurfaceBridge_getPixelHeight(
    _env: *mut c_void,
    _class: *mut c_void,
    ptr: i64,
) -> i32 {
    if ptr == 0 {
        return 0;
    }
    unsafe { (*(ptr as *const PixelBuffer)).height as i32 }
}

#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_compose_DesktopSurfaceBridge_checkAndClearDirty(
    _env: *mut c_void,
    _class: *mut c_void,
    ptr: i64,
) -> bool {
    if ptr == 0 {
        return false;
    }
    unsafe {
        (*(ptr as *const PixelBuffer))
            .dirty
            .swap(false, Ordering::AcqRel)
    }
}

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
                let buf = &mut *(handle as *mut PixelBuffer);
                let size = (pw as usize) * (ph as usize) * 4;
                buf.data.resize(size, 0);
                buf.width = pw;
                buf.height = ph;
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
                    let buf = &mut *(self.handle as *mut PixelBuffer);
                    sink.flush_to(&mut buf.data);
                    buf.dirty.store(true, Ordering::Release);
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
                let buf = &mut *(self.handle as *mut PixelBuffer);
                let size = (pw as usize) * (ph as usize) * 4;
                buf.data.resize(size, 0);
                buf.width = pw;
                buf.height = ph;
            }
        }

        self.backend.resize(pw as u16, ph as u16);
    }
}
