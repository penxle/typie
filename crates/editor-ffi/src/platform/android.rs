use editor_macros::ffi;
use editor_renderer::{RenderBackend, RenderSink};
use std::ffi::c_void;
use std::ptr;
use std::ptr::NonNull;
use std::sync::Arc;

use crate::backend::BackendMode;
use crate::error::FfiError;

const WINDOW_FORMAT_RGBA_8888: i32 = 1;

#[repr(C)]
struct ANativeWindowBuffer {
    width: i32,
    height: i32,
    stride: i32,
    format: i32,
    bits: *mut c_void,
    reserved: [u32; 6],
}

#[link(name = "android")]
unsafe extern "C" {
    #[link_name = "ANativeWindow_fromSurface"]
    fn native_window_from_surface(env: *mut c_void, surface: *mut c_void) -> *mut c_void;
    #[link_name = "ANativeWindow_release"]
    fn native_window_release(window: *mut c_void);
    #[link_name = "ANativeWindow_setBuffersGeometry"]
    fn native_window_set_buffers_geometry(
        window: *mut c_void,
        width: i32,
        height: i32,
        format: i32,
    ) -> i32;
    #[link_name = "ANativeWindow_lock"]
    fn native_window_lock(
        window: *mut c_void,
        out_buffer: *mut ANativeWindowBuffer,
        in_out_dirty_bounds: *mut c_void,
    ) -> i32;
    #[link_name = "ANativeWindow_unlockAndPost"]
    fn native_window_unlock_and_post(window: *mut c_void) -> i32;
    #[link_name = "ANativeWindow_getWidth"]
    fn native_window_get_width(window: *mut c_void) -> i32;
    #[link_name = "ANativeWindow_getHeight"]
    fn native_window_get_height(window: *mut c_void) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_compose_NativeWindowBridge_fromSurface(
    env: *mut c_void,
    _class: *mut c_void,
    surface: *mut c_void,
) -> i64 {
    unsafe { native_window_from_surface(env, surface) as i64 }
}

#[unsafe(no_mangle)]
pub extern "C" fn Java_co_typie_editor_compose_NativeWindowBridge_release(
    _env: *mut c_void,
    _class: *mut c_void,
    handle: i64,
) {
    if handle != 0 {
        unsafe { native_window_release(handle as *mut c_void) };
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
        mode: &BackendMode,
        handle: PlatformHandle,
        width: u32,
        height: u32,
        scale_factor: f64,
    ) -> Result<Self, FfiError> {
        // format only — 0,0 keeps the surface's natural dimensions
        if handle != 0 {
            unsafe {
                native_window_set_buffers_geometry(
                    handle as *mut c_void,
                    0,
                    0,
                    WINDOW_FORMAT_RGBA_8888,
                );
            }
        }

        // use actual surface dimensions, not derived from scale_factor
        let (pw, ph) = if handle != 0 {
            unsafe {
                let window = handle as *mut c_void;
                let w = native_window_get_width(window);
                let h = native_window_get_height(window);
                if w > 0 && h > 0 {
                    (w as u32, h as u32)
                } else {
                    (
                        (width as f64 * scale_factor).round() as u32,
                        (height as f64 * scale_factor).round() as u32,
                    )
                }
            }
        } else {
            (
                (width as f64 * scale_factor).round() as u32,
                (height as f64 * scale_factor).round() as u32,
            )
        };

        let backend = match mode {
            BackendMode::Cpu => RenderBackend::new_cpu(pw as u16, ph as u16),
            BackendMode::Gpu { device } => {
                let window_ptr = NonNull::new(handle as *mut c_void)
                    .ok_or_else(|| FfiError::Surface("null ANativeWindow handle".into()))?;

                let raw_window_handle = raw_window_handle::RawWindowHandle::AndroidNdk(
                    raw_window_handle::AndroidNdkWindowHandle::new(window_ptr),
                );
                let raw_display_handle = raw_window_handle::RawDisplayHandle::Android(
                    raw_window_handle::AndroidDisplayHandle::new(),
                );

                let surface = unsafe {
                    device
                        .instance
                        .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                            raw_display_handle,
                            raw_window_handle,
                        })
                        .map_err(|e| FfiError::Surface(e.to_string()))?
                };

                match RenderBackend::new_gpu(Arc::clone(device), surface) {
                    Ok(mut backend) => {
                        backend.resize(pw as u16, ph as u16);
                        backend
                    }
                    Err(_) => RenderBackend::new_cpu(pw as u16, ph as u16),
                }
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
        let window = self.handle as *mut c_void;
        if window.is_null() {
            return;
        }

        // lock first to get actual buffer dimensions
        if let RenderBackend::Cpu(_) = &self.backend {
            let (bw, bh, _) = unsafe {
                let mut buffer = std::mem::zeroed::<ANativeWindowBuffer>();
                if native_window_lock(window, &mut buffer, ptr::null_mut()) != 0 {
                    return;
                }
                if buffer.bits.is_null() {
                    native_window_unlock_and_post(window);
                    return;
                }
                let dims = (
                    buffer.width as u32,
                    buffer.height as u32,
                    buffer.stride as u32,
                );
                native_window_unlock_and_post(window);
                dims
            };

            if bw != self.width || bh != self.height {
                self.width = bw;
                self.height = bh;
                self.backend.resize(bw as u16, bh as u16);
            }
        }

        match &mut self.backend {
            RenderBackend::Cpu(sink) => {
                let mut buf = vec![0u8; (self.width * self.height * 4) as usize];
                sink.flush_to(&mut buf);

                unsafe {
                    let mut buffer = std::mem::zeroed::<ANativeWindowBuffer>();
                    if native_window_lock(window, &mut buffer, ptr::null_mut()) != 0 {
                        return;
                    }

                    if buffer.bits.is_null() {
                        native_window_unlock_and_post(window);
                        return;
                    }

                    let dst = buffer.bits as *mut u8;
                    let dst_stride = buffer.stride as u32 * 4;
                    let src_stride = self.width * 4;

                    for y in 0..self.height {
                        ptr::copy_nonoverlapping(
                            buf.as_ptr().add((y * src_stride) as usize),
                            dst.add((y * dst_stride) as usize),
                            src_stride as usize,
                        );
                    }

                    native_window_unlock_and_post(window);
                }
            }
            RenderBackend::Gpu(sink) => {
                let _ = sink.present();
            }
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        let (pw, ph) = {
            let window = self.handle as *mut c_void;
            if !window.is_null() {
                unsafe {
                    let w = native_window_get_width(window);
                    let h = native_window_get_height(window);
                    if w > 0 && h > 0 {
                        (w as u32, h as u32)
                    } else {
                        (
                            (width as f64 * scale_factor).round() as u32,
                            (height as f64 * scale_factor).round() as u32,
                        )
                    }
                }
            } else {
                (
                    (width as f64 * scale_factor).round() as u32,
                    (height as f64 * scale_factor).round() as u32,
                )
            }
        };

        self.width = pw;
        self.height = ph;
        self.scale_factor = scale_factor;

        self.backend.resize(pw as u16, ph as u16);
    }
}
