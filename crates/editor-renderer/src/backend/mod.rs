pub mod cpu;
pub mod gpu;
pub mod kind;

use std::sync::Arc;

use crate::sink::RenderSink;
use cpu::CpuSink;
use gpu::GpuSink;

pub enum RenderBackend {
    Cpu(CpuSink),
    Gpu(GpuSink),
}

impl RenderBackend {
    pub fn new_cpu(width: u16, height: u16) -> Self {
        Self::Cpu(CpuSink::new(width, height))
    }

    pub fn new_gpu(
        device: Arc<gpu::GpuDevice>,
        surface: wgpu::Surface<'static>,
    ) -> Result<Self, crate::RendererError> {
        Ok(Self::Gpu(GpuSink::new(device, surface)?))
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        match self {
            Self::Cpu(s) => s.resize(width, height),
            Self::Gpu(s) => s.resize(width as u32, height as u32),
        }
    }

    pub fn render(&mut self, f: impl FnOnce(&mut dyn RenderSink)) {
        match self {
            Self::Cpu(s) => f(s),
            Self::Gpu(s) => f(s),
        }
    }

    pub fn sink(&mut self) -> &mut dyn RenderSink {
        match self {
            Self::Cpu(s) => s,
            Self::Gpu(s) => s,
        }
    }

    pub fn kind(&self) -> kind::BackendKind {
        match self {
            Self::Cpu(_) => kind::BackendKind::Cpu,
            Self::Gpu(_) => kind::BackendKind::Gpu,
        }
    }
}
