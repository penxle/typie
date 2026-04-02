#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("decompression failed: {0}")]
    Decompression(String),

    #[error("failed to create ICU blob provider: {0}")]
    IcuProvider(String),

    #[error("failed to create segmenter: {0}")]
    IcuSegmenter(String),

    #[error("invalid font data: {0}")]
    InvalidFont(String),

    #[error("invalid manifest data: {0}")]
    InvalidManifest(String),
}
