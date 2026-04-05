use editor_macros::ffi;
use editor_renderer::{RenderBackend, RenderSink};
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_metal::{MTLOrigin, MTLRegion, MTLSize};
use std::ffi::c_void;
use std::sync::Arc;

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

        let layer_ptr = handle as *mut AnyObject;
        if !layer_ptr.is_null() {
            unsafe {
                let ca = objc2::runtime::AnyClass::get(c"CATransaction").unwrap();
                let _: () = msg_send![ca, begin];
                let _: () = msg_send![ca, setDisableActions: true];

                let drawable_size = objc2_core_foundation::CGSize {
                    width: pw as f64,
                    height: ph as f64,
                };
                let frame = objc2_core_foundation::CGRect {
                    origin: objc2_core_foundation::CGPoint { x: 0.0, y: 0.0 },
                    size: objc2_core_foundation::CGSize {
                        width: width as f64,
                        height: height as f64,
                    },
                };
                let _: () = msg_send![&*layer_ptr, setFrame: frame];
                let _: () = msg_send![&*layer_ptr, setDrawableSize: drawable_size];
                let _: () = msg_send![&*layer_ptr, setContentsScale: scale_factor];
                let _: () = msg_send![&*layer_ptr, setOpaque: false];

                let _: () = msg_send![ca, commit];
            }
        }

        let backend = match mode {
            BackendMode::Cpu => {
                if !layer_ptr.is_null() {
                    unsafe {
                        let _: () = msg_send![&*layer_ptr, setFramebufferOnly: false];
                    }
                }
                RenderBackend::new_cpu(pw as u16, ph as u16)
            }
            BackendMode::Gpu { device } => {
                let layer_ptr_void = handle as *mut c_void;
                if layer_ptr_void.is_null() {
                    return Err(FfiError::Surface("null CAMetalLayer handle".into()));
                }

                let surface = unsafe {
                    device
                        .instance
                        .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(
                            layer_ptr_void,
                        ))
                        .map_err(|e| FfiError::Surface(e.to_string()))?
                };

                match RenderBackend::new_gpu(Arc::clone(device), surface) {
                    Ok(mut backend) => {
                        backend.resize(pw as u16, ph as u16);
                        backend
                    }
                    Err(_) => {
                        if !layer_ptr.is_null() {
                            unsafe {
                                let _: () = msg_send![&*layer_ptr, setFramebufferOnly: false];
                            }
                        }
                        RenderBackend::new_cpu(pw as u16, ph as u16)
                    }
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
        match &mut self.backend {
            RenderBackend::Cpu(sink) => {
                let mut buf = vec![0u8; (self.width * self.height * 4) as usize];
                sink.flush_to(&mut buf);

                // CAMetalLayer only supports BGRA8Unorm
                for pixel in buf.chunks_exact_mut(4) {
                    pixel.swap(0, 2);
                }

                unsafe {
                    let layer = &*(self.handle as *const AnyObject);

                    let drawable: Option<Retained<AnyObject>> = msg_send![layer, nextDrawable];
                    let Some(drawable) = drawable else { return };

                    let texture: *const AnyObject = msg_send![&*drawable, texture];
                    if texture.is_null() {
                        return;
                    }

                    let region = MTLRegion {
                        origin: MTLOrigin { x: 0, y: 0, z: 0 },
                        size: MTLSize {
                            width: self.width as usize,
                            height: self.height as usize,
                            depth: 1,
                        },
                    };
                    let bytes_per_row = self.width as usize * 4;

                    let _: () = msg_send![
                        &*texture,
                        replaceRegion: region,
                        mipmapLevel: 0usize,
                        withBytes: buf.as_ptr() as *const c_void,
                        bytesPerRow: bytes_per_row
                    ];

                    let _: () = msg_send![&*drawable, present];
                }
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

        let layer_ptr = self.handle as *mut AnyObject;
        if !layer_ptr.is_null() {
            unsafe {
                let ca = objc2::runtime::AnyClass::get(c"CATransaction").unwrap();
                let _: () = msg_send![ca, begin];
                let _: () = msg_send![ca, setDisableActions: true];

                let drawable_size = objc2_core_foundation::CGSize {
                    width: pw as f64,
                    height: ph as f64,
                };
                let frame = objc2_core_foundation::CGRect {
                    origin: objc2_core_foundation::CGPoint { x: 0.0, y: 0.0 },
                    size: objc2_core_foundation::CGSize {
                        width: width as f64,
                        height: height as f64,
                    },
                };
                let _: () = msg_send![&*layer_ptr, setFrame: frame];
                let _: () = msg_send![&*layer_ptr, setDrawableSize: drawable_size];
                let _: () = msg_send![&*layer_ptr, setContentsScale: scale_factor];

                let _: () = msg_send![ca, commit];
            }
        }

        self.backend.resize(pw as u16, ph as u16);
    }
}
