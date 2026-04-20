use editor_macros::ffi;
use editor_renderer::{RenderBackend, RenderSink};
use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_core_foundation::CGSize;
use objc2_metal::{MTLOrigin, MTLRegion, MTLSize};
use std::ffi::c_void;

use crate::error::FfiError;

#[ffi]
pub type PlatformHandle = u64;

// Read the layer's drawableSize directly so Rust and Kotlin agree on pixel dims.
// Returns None before the view has been laid out (drawableSize is 0).
fn drawable_size(handle: PlatformHandle) -> Option<(u32, u32)> {
    if handle == 0 {
        return None;
    }
    unsafe {
        let layer = &*(handle as *const AnyObject);
        let size: CGSize = msg_send![layer, drawableSize];
        if size.width > 0.0 && size.height > 0.0 {
            Some((size.width as u32, size.height as u32))
        } else {
            None
        }
    }
}

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
        let (pw, ph) = drawable_size(handle).unwrap_or_else(|| {
            (
                (width as f64 * scale_factor).round() as u32,
                (height as f64 * scale_factor).round() as u32,
            )
        });

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
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, scale_factor: f64) {
        let (pw, ph) = drawable_size(self.handle).unwrap_or_else(|| {
            (
                (width as f64 * scale_factor).round() as u32,
                (height as f64 * scale_factor).round() as u32,
            )
        });

        self.width = pw;
        self.height = ph;
        self.scale_factor = scale_factor;

        self.backend.resize(pw as u16, ph as u16);
    }
}
