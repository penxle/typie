#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("decompression failed: {0}")]
    Decompression(String),

    #[error("failed to create ICU blob provider: {0}")]
    IcuProvider(String),

    #[error("failed to create segmenter: {0}")]
    IcuSegmenter(String),

    #[error("unknown font: {0}")]
    UnknownFont(String),

    #[error("unknown subset: {0}")]
    UnknownSubset(String),

    #[error("invalid TTF: {0}")]
    InvalidTtf(String),

    #[error("invalid font data: {0}")]
    InvalidFont(String),
}
