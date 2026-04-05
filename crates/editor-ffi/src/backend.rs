use editor_renderer::{BackendKind, GpuDevice};
use std::sync::Arc;

#[derive(Clone)]
pub enum BackendMode {
    Cpu,
    Gpu { device: Arc<GpuDevice> },
}

impl BackendMode {
    pub fn kind(&self) -> BackendKind {
        match self {
            Self::Cpu => BackendKind::Cpu,
            Self::Gpu { .. } => BackendKind::Gpu,
        }
    }
}
