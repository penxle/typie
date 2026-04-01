pub mod cpu;
pub mod export;
pub mod gpu;

use std::sync::{Arc, Mutex};

use super::backend::cpu::PixelBuf;

pub use gpu::GpuDevice;

#[allow(dead_code)]
pub enum RenderBackend {
    Cpu {
        buf: PixelBuf,
        scratch_buf: PixelBuf,
    },
    Gpu {
        device: Arc<Mutex<GpuDevice>>,
        post_buf: PixelBuf,
    },
}

impl RenderBackend {
    pub fn new_cpu() -> Self {
        let buf = PixelBuf::new(1, 1).unwrap();
        let scratch_buf = PixelBuf::new(1, 1).unwrap();
        Self::Cpu { buf, scratch_buf }
    }

    pub fn new_gpu(device: Arc<Mutex<GpuDevice>>) -> Self {
        let post_buf = PixelBuf::new(1, 1).unwrap();
        Self::Gpu { device, post_buf }
    }
}
