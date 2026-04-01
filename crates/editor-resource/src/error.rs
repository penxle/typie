#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("decompression failed: {0}")]
    Decompression(String),

    #[error("invalid font data: {0}")]
    InvalidFont(String),
}
