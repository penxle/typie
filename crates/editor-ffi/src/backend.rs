use editor_renderer::GpuDevice;
use std::sync::Arc;

#[derive(Clone)]
pub enum BackendMode {
    Cpu,
    Gpu { device: Arc<GpuDevice> },
}
