use editor_renderer::GpuDevice;
use std::sync::Arc;

#[derive(Clone)]
pub enum BackendMode {
    Cpu,
    Gpu { device: Arc<GpuDevice> },
}

impl BackendMode {
    pub fn kind(&self) -> editor_renderer::BackendKind {
        match self {
            Self::Cpu => editor_renderer::BackendKind::Cpu,
            Self::Gpu { .. } => editor_renderer::BackendKind::Gpu,
        }
    }
}
