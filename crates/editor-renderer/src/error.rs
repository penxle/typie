#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("no suitable GPU adapter found: {0}")]
    NoAdapter(#[from] wgpu::RequestAdapterError),

    #[error("failed to request device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),

    #[error("failed to create renderer: {0}")]
    CreateRenderer(#[from] vello::Error),

    #[error("surface error: {0}")]
    Surface(#[from] wgpu::SurfaceError),

    #[error("GPU does not support required capabilities")]
    UnsupportedGpu,
}
